use serenity::all::*;
use serenity::builder::EditInteractionResponse;

use crate::commands::utils::*;
use crate::integration::notion::project::Project;

use super::util::{extract_target_user_ids, get_target_role_id, handle_empty_processed_users};

pub async fn member_remove(
    channel_manager: &ChannelManager<'_>,
    subcommand_option: &CommandDataOption,
    already_members: &Vec<UserId>,
    my_project: &mut Project,
) -> serenity::Result<(), serenity::Error> {
    let (ctx, guild_id, command) = (
        channel_manager.ctx,
        channel_manager.guild_id,
        channel_manager.command,
    );

    // 서버 내에서 대응되는 프로젝트 역할 ID 검색
    let role_id = get_target_role_id(channel_manager, &my_project).await?;

    let mut processed_users = Vec::new();
    for user_id in extract_target_user_ids(subcommand_option) {
        if !already_members.contains(&user_id) {
            continue;
        }

        if ctx
            .http
            .remove_member_role(guild_id, user_id, role_id, None)
            .await
            .is_ok()
        {
            processed_users.push(user_id);

            // Notion 프로젝트 참여자 명단에서 제거
            my_project
                .participants
                .retain(|member| member.discord_id != user_id.to_string());
        }
    }

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
                "🗑️ {} 님이 '{}' 프로젝트 멤버에서 제외되었습니다.",
                mentions.join(", "),
                my_project.name
            )),
        )
        .await?;

    Ok(())
}
