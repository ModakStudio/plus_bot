use tracing::{error, info};

use serenity::all::{ChannelId, GuildId};
use serenity::builder::{EditChannel, EditInteractionResponse, EditRole};
use serenity::prelude::*;

use crate::cache::*;
use crate::commands::utils::*;
use crate::integration::notion::project::*;

pub async fn rename_project(
    channel_manager: &ChannelManager<'_>,
    category_id: ChannelId,
    old_name: String,
    new_name: String,
) -> serenity::Result<()> {
    let (ctx, command, cache_lock) = (
        channel_manager.ctx,
        channel_manager.command,
        &channel_manager.cache_lock,
    );

    // Notion 프로젝트 캐시에서 가져오기
    let mut notion_project = match BotCache::get_project_by_name(&cache_lock, &old_name).await {
        Some(project) => project.clone(),
        None => return Ok(()),
    };
    // notion_project.participants.push(notion_project.pm.clone());

    let builder = EditChannel::new().name(&new_name);
    if let Err(why) = category_id.edit(&ctx.http, builder).await {
        command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content(format!("❌ 이름 변경 실패: {:?}", why)),
            )
            .await?;
        return Ok(());
    }

    // Notion 프로젝트 이름 변경
    notion_project.name = new_name.clone();

    let role_renamed = try_rename_role(ctx, channel_manager.guild_id, &old_name, &new_name).await;
    let msg_content = if role_renamed {
        format!(
            "📝 프로젝트 카테고리와 역할 이름이 모두 '{}'으로 변경되었습니다.",
            new_name
        )
    } else {
        format!(
            "📝 프로젝트 이름은 '{}'으로 변경되었으나, 동명의 기존 역할을 찾지 못했습니다.",
            new_name
        )
    };

    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(msg_content),
        )
        .await?;

    // Notion 동기화
    let project_id = notion_project.id.clone();
    match update_project(&project_id, &notion_project).await {
        Ok(_) => {
            info!(
                "Notion 프로젝트 '{}' 정보가 성공적으로 업데이트되었습니다.",
                new_name
            );
        }
        Err(why) => {
            error!(
                "Notion 프로젝트 '{}' 정보 업데이트 실패. 에러: {:?}",
                new_name, why
            );
        }
    }

    Ok(())
}

// 역할 이름 변경 시도, 성공 여부 반환
async fn try_rename_role(ctx: &Context, guild_id: GuildId, old_name: &str, new_name: &str) -> bool {
    if old_name.is_empty() {
        return false;
    }
    let Ok(roles) = guild_id.roles(&ctx.http).await else {
        return false;
    };
    if let Some(role) = roles
        .values()
        .find(|r| r.name.eq_ignore_ascii_case(old_name))
    {
        return guild_id
            .edit_role(&ctx.http, role.id, EditRole::new().name(new_name))
            .await
            .is_ok();
    }
    false
}
