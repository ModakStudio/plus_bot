use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use serenity::all::{
    ChannelId, ChannelType, Context, CreateChannel, CreateEmbed, 
    CreateMessage, EditMessage, GuildId
};

use crate::cache::BotCache;

/// 멤버 목록 업데이트 백그라운드 스레드 래퍼
/// (이제 특정 채널 ID 대신 길드(서버) ID와 카테고리 ID를 받습니다)
pub fn spawn_sync_member_embed_by_category(
    ctx: Context,
    cache_lock: Arc<RwLock<BotCache>>,
    guild_id: GuildId,
    category_id: ChannelId,
) {
    tokio::spawn(async move {
        info!("🔄 카테고리 기반 백그라운드 멤버 임베드 동기화 시작...");

        if let Err(e) = sync_member_embed_internal(&ctx, &cache_lock, guild_id, category_id).await {
            error!("❌ 멤버 임베드 동기화 실패: {:?}", e);
        } else {
            info!("✅ 멤버 임베드 동기화 완료");
        }
    });
}

/// 실제 API 조작 및 로직 처리를 담당하는 내부 비동기 함수
async fn sync_member_embed_internal(
    ctx: &Context,
    cache_lock: &Arc<RwLock<BotCache>>,
    guild_id: GuildId,
    category_id: ChannelId,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    
    // 1. 길드(서버)의 전체 채널 목록 조회
    let channels = guild_id.channels(&ctx.http).await?;
    let target_channel_name = "📜information"; // 원하는 채널 이름으로 설정하세요

    // 2. 카테고리 하위에 있는 목적지 채널 탐색
    let mut target_channel_id = None;
    for (_, channel) in channels.iter() {
        if channel.parent_id == Some(category_id) 
            && channel.kind == ChannelType::Text 
            && channel.name == target_channel_name 
        {
            target_channel_id = Some(channel.id);
            break;
        }
    }

    // 3. 채널이 없으면 카테고리 하위에 새로 생성
    let channel_id = match target_channel_id {
        Some(id) => id,
        None => {
            info!("카테고리 내에 '{}' 채널이 없어 새로 생성합니다.", target_channel_name);
            let new_channel = guild_id.create_channel(
                &ctx.http,
                CreateChannel::new(target_channel_name)
                    .kind(ChannelType::Text)
                    .category(category_id), // 해당 카테고리 소속으로 지정
            ).await?;
            new_channel.id
        }
    };

    // 4. 캐시에서 최신 멤버 데이터 가져오기
    let members = {
        let cache = cache_lock.read().await;
        cache.all_members.clone()
    };

    // 5. 임베드 내용 구성
    let mut embed_description = String::new();
    for member in &members {
        embed_description.push_str(&format!("• {}\n", member.name)); 
    }

    let embed = CreateEmbed::new()
        .title("📋 전체 멤버 목록")
        .description(embed_description)
        .color(0x3498db);

    // 6. 대상 채널의 고정(Pinned) 메시지 목록 조회
    let pinned_messages = channel_id.pins(&ctx.http).await?;
    let bot_user_id = ctx.cache.current_user().id;

    // 7. 봇이 작성한 기존 임베드 찾기
    let existing_message = pinned_messages.iter().find(|m| {
        m.author.id == bot_user_id
            && m.embeds.iter().any(|e| e.title.as_deref() == Some("📋 전체 멤버 목록"))
    });

    // 8. 메시지 수정 또는 새로 작성 후 고정
    match existing_message {
        Some(msg) => {
            channel_id
                .edit_message(&ctx.http, msg.id, EditMessage::new().embed(embed))
                .await?;
        }
        None => {
            let new_msg = channel_id
                .send_message(&ctx.http, CreateMessage::new().embed(embed))
                .await?;
            new_msg.pin(&ctx.http).await?;
        }
    }

    Ok(())
}