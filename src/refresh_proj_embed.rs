use serenity::all::{
    ChannelId, ChannelType, Context, CreateChannel, CreateEmbed, CreateEmbedFooter, CreateMessage,
    EditMessage, GuildId,
};
use serenity::model::channel::GuildChannel;
use std::collections::HashMap;
use std::sync::Arc;

use tracing::error;

use crate::integration::notion::project::Project;

/// 실제 API 조작 및 로직 처리를 담당하는 내부 비동기 함수
pub async fn sync_member_embed_internal(
    ctx: &Context,
    guild_id: GuildId,
    category_id: ChannelId, // 프로젝트 카테고리 채널 ID
    channels: &HashMap<ChannelId, GuildChannel>,
    project: &Project,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 임베드 내용 구성
    let embed = build_project_embed(project);

    // 대상 채널 탐색 또는 생성
    let target_channel_name = "📜information";

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

    let channel_id = match target_channel_id {
        Some(id) => id,
        None => {
            let new_channel = guild_id
                .create_channel(
                    &ctx.http,
                    CreateChannel::new(target_channel_name)
                        .kind(ChannelType::Text)
                        .category(category_id),
                )
                .await?;
            new_channel.id
        }
    };

    // 4. 메시지 수정 또는 생성
    let pinned_messages = channel_id.pins(&ctx.http).await?;
    let bot_user_id = ctx.cache.current_user().id;

    let existing_message = pinned_messages.iter().find(|m| {
        m.author.id == bot_user_id
            && m.embeds
                .iter()
                .any(|e| e.title.as_deref().unwrap_or("").contains("📁"))
    });

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
            if let Err(why) = new_msg.pin(&ctx.http).await {
                tracing::error!("핀 고정 실패: {:?}", why);
            }
        }
    }

    Ok(())
}

// 전체 프로젝트 임베드 갱신을 비동기적으로 수행하는 함수
pub async fn update_all_project_embeds(
    ctx: &Context,
    guild_id: GuildId,
    projects: Vec<Project>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let channels = Arc::new(guild_id.channels(&ctx.http).await?);
    let mut handles = Vec::with_capacity(projects.len());

    for project in projects {
        let ctx_clone = ctx.clone();
        let channels_ref = Arc::clone(&channels);
        let handle = tokio::spawn(async move {
            let category_id = match project.category_id.parse::<u64>() {
                Ok(id) => ChannelId::new(id),
                Err(_) => {
                    error!(
                        "프로젝트 '{}'의 카테고리 ID '{}'를 파싱할 수 없습니다.",
                        project.name, project.category_id
                    );
                    return;
                }
            };

            // 이전에 만들었던 내부 처리 함수(sync_member_embed_internal)를 직접 호출
            if let Err(e) = sync_member_embed_internal(
                &ctx_clone,
                guild_id,
                category_id,
                &channels_ref,
                &project,
            )
            .await
            {
                error!("❌ 카테고리 {} 갱신 실패: {:?}", category_id, e);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await?;
    }

    Ok(())
}

// 임베드 생성 함수
pub fn build_project_embed(project: &Project) -> CreateEmbed {
    // 1. GitHub 링크 텍스트 포맷팅 (URL이 있으면 마크다운 링크, 없으면 텍스트)
    let github_value = if project.github.trim().is_empty() {
        "미등록".to_string()
    } else {
        format!("[GitHub 바로가기]({})", project.github.trim())
    };

    // 2. 참여 멤버 목록 텍스트 구성
    let participants_value = if project.participants.is_empty() {
        "• 등록된 멤버가 없습니다.".to_string()
    } else {
        project
            .participants
            .iter()
            .filter(|m: &&crate::integration::notion::member::NotionMember| {
                !m.name.trim().is_empty()
            })
            .map(|m| format!("• {}", m.name.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    };

    // 3. 필드 기반 임베드 빌드
    CreateEmbed::new()
        .title(format!("📁 {}", project.name))
        .color(project.status.color())
        .field("", "", false) // 빈 필드로 줄바꿈
        // 1열: 상태, GitHub (inline)
        .field("상태", project.status.display_name(), true)
        .field("GitHub", github_value, true)
        .field("", "", false) // 빈 필드로 줄바꿈
        // 2열: PM (inline)
        .field("PM", &project.pm.name, false)
        .field("", "", false) // 빈 필드로 줄바꿈
        // 전체 너비: 참여 멤버 (inline: false)
        .field("👥 참여 멤버", participants_value, false)
        .footer(CreateEmbedFooter::new("※ 이 임베드는 자동으로 갱신됩니다."))
}
