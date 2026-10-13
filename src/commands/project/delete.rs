use tracing::{error, info};

use serenity::all::{ChannelId, GuildId};
use serenity::builder::EditInteractionResponse;
use serenity::model::channel::Channel;
use serenity::prelude::*;

use crate::cache::*;
use crate::commands::utils::*;

pub async fn delete_project(
    channel_manager: &ChannelManager<'_>,
    category_id: ChannelId,
) -> serenity::Result<()> {
    let (ctx, guild_id, command, cache_lock) = (
        channel_manager.ctx,
        channel_manager.guild_id,
        channel_manager.command,
        &channel_manager.cache_lock,
    );

    let project_name = match category_id.to_channel(&ctx.http).await {
        Ok(Channel::Guild(cat)) => cat.name,
        _ => String::new(),
    };

    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new()
                .content("🧹 프로젝트 채널들과 역할을 완전히 삭제합니다..."),
        )
        .await?;

    // 1. 하위 채널 청소
    if let Ok(channels) = guild_id.channels(&ctx.http).await {
        for (id, guild_channel) in &channels {
            if guild_channel.parent_id == Some(category_id) && *id != command.channel_id {
                let _ = guild_channel.id.delete(&ctx.http).await;
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
        let _ = command.channel_id.delete(&ctx.http).await;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // 2. 카테고리 삭제
    let _ = category_id.delete(&ctx.http).await;

    // 3. 동명 역할 찾아서 삭제
    try_delete_role(ctx, guild_id, &project_name).await;

    // 4. Notion 프로젝트 삭제
    let project_id = match BotCache::get_project_id_by_name(&cache_lock, &project_name).await {
        Some(id) => id,
        None => return Ok(()),
    };

    use crate::integration::notion::project::delete_project;
    match delete_project(&project_id).await {
        Ok(_) => {
            info!("Notion 프로젝트 '{}' 삭제 완료", project_name);
        }
        Err(why) => {
            error!(
                "Notion 프로젝트 '{}' 삭제 실패. 에러: {:?}",
                project_name, why
            );
        }
    }

    // 5. 캐시 갱신
    let _ = BotCache::refresh_about_project(&cache_lock).await;

    Ok(())
}

// 역할 이름으로 역할 삭제 시도
async fn try_delete_role(ctx: &Context, guild_id: GuildId, role_name: &str) {
    if role_name.is_empty() {
        return;
    }
    if let Ok(roles) = guild_id.roles(&ctx.http).await {
        if let Some(role) = roles
            .values()
            .find(|r| r.name.eq_ignore_ascii_case(role_name))
        {
            let _ = guild_id.delete_role(&ctx.http, role.id).await;
        }
    }
}
