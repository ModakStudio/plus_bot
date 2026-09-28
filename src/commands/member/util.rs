use serenity::all::*;
use serenity::builder::EditInteractionResponse;
use serenity::model::application::CommandDataOptionValue::{SubCommand, User};

use crate::commands::utils::*;
use crate::integration::notion::project::Project;

// PM 혹은 관리자 권한을 가진 유저인지 판별
pub async fn check_pm_or_admin(
    channel_manager: &ChannelManager<'_>,
    category_id: ChannelId,
) -> serenity::Result<()> {
    let (ctx, command) = (channel_manager.ctx, channel_manager.command);

    // 권한 체크 (PM 이거나 관리자 권한을 가진 유저인지 판별)
    let is_admin = is_server_admin(&channel_manager).await;
    let is_pm = is_project_pm(&channel_manager, category_id).await;
    if !is_pm && !is_admin {
        command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .content("❌ 이 명령어를 사용할 권한이 없습니다. (PM 혹은 관리자만 가능)"),
            )
            .await?;
        return Err(serenity::Error::Other("권한이 없습니다."));
    }

    Ok(())
}

// 프로젝트에 대응되는 역할 ID를 가져오는 헬퍼 함수
pub async fn get_target_role_id(
    channel_manager: &ChannelManager<'_>,
    my_project: &Project,
) -> serenity::Result<RoleId, serenity::Error> {
    let (ctx, guild_id, command) = (
        channel_manager.ctx,
        channel_manager.guild_id,
        channel_manager.command,
    );

    // 서버 내에서 대응되는 프로젝트 역할 ID 검색
    let mut target_role_id = None;
    if let Ok(roles) = guild_id.roles(&ctx.http).await {
        if let Some(role) = roles.values().find(|r| r.name == my_project.name) {
            target_role_id = Some(role.id);
        }
    }

    let role_id = match target_role_id {
        Some(id) => id,
        None => {
            command
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content(format!(
                        "❌ 서버에서 '{}' 프로젝트에 해당하는 역할을 찾을 수 없습니다.",
                        my_project.name
                    )),
                )
                .await?;
            return Err(serenity::Error::Other("역할을 찾을 수 없습니다."));
        }
    };

    Ok(role_id)
}

// 서브커맨드 멘션 옵션에서 대상 유저 ID들을 추출하는 헬퍼 함수
pub fn extract_target_user_ids(subcommand_option: &CommandDataOption) -> Vec<UserId> {
    let mut target_user_ids = Vec::new();

    if let SubCommand(sub_opts) = &subcommand_option.value {
        for opt in sub_opts {
            if let User(user_id) = opt.value {
                if target_user_ids.contains(&user_id) {
                    continue; // 중복된 유저는 건너뛰기
                }

                target_user_ids.push(user_id);
            }
        }
    }

    target_user_ids
}

// 변동 사항이 없거나 작업 가능한 대상 유저가 없을 때 처리하는 헬퍼 함수
pub async fn handle_empty_processed_users(
    channel_manager: &ChannelManager<'_>,
) -> serenity::Result<(), serenity::Error> {
    let (ctx, command) = (channel_manager.ctx, channel_manager.command);

    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new()
                .content("❌ 변동 사항이 없거나 작업 가능한 대상 유저가 없습니다."),
        )
        .await?;

    return Err(serenity::Error::Other(
        "변동 사항이 없거나 작업 가능한 대상 유저가 없습니다.",
    ));
}
