use serenity::all::*;
use serenity::builder::EditInteractionResponse;

use crate::cache::*;
use crate::commands::utils::*;
use crate::integration::notion::project::Project;

// 헬퍼 함수들 불러오기
use super::util::{extract_target_user_ids, get_target_role_id, handle_empty_processed_users};

pub async fn member_add(
    channel_manager: &ChannelManager<'_>,
    subcommand_option: &CommandDataOption,
    already_members: &Vec<UserId>,
    my_project: &mut Project,
) -> Result<(), serenity::Error> {
    let (ctx, guild_id, command, cache_lock) = (
        channel_manager.ctx,
        channel_manager.guild_id,
        channel_manager.command,
        &channel_manager.cache_lock,
    );

    // 서버 내에서 대응되는 프로젝트 역할 ID 검색
    let role_id = get_target_role_id(channel_manager, &my_project).await?;

    let mut processed_users = Vec::new();
    for user_id in extract_target_user_ids(subcommand_option) {
        // 이미 존재하는 유저는 건너뛰기
        if already_members.contains(&user_id) {
            continue;
        }

        let notion_member = match BotCache::get_member_by_user_id(cache_lock, user_id).await {
            Some(member) => member,
            None => continue,
        };

        if ctx
            .http
            .add_member_role(guild_id, user_id, role_id, None)
            .await
            .is_ok()
        {
            processed_users.push(user_id);
            my_project.participants.push(notion_member);
        }
    }

    // 변동 사항이 없거나 작업 가능한 대상 유저가 없을 때 처리
    if processed_users.is_empty() {
        return handle_empty_processed_users(channel_manager).await;
    }

    let mentions: Vec<String> = processed_users
        .iter()
        .map(|id| format!("<@{}>", id))
        .collect();

    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(format!(
                "✅ {} 님이 '{}' 프로젝트 멤버로 추가되었습니다!",
                mentions.join(", "),
                my_project.name
            )),
        )
        .await?;

    Ok(())
}
