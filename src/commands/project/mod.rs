pub mod delete;
pub mod generate;
pub mod register_command;
pub mod rename;
pub mod util;

use serenity::builder::EditInteractionResponse;
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Channel;
use serenity::prelude::*;

use crate::commands::utils::*;

use delete::delete_project;
use generate::generate_project;
use rename::rename_project;
use util::find_already_exist_project_name;

pub async fn run_project_command(
    ctx: &Context,
    command: &CommandInteraction,
) -> serenity::Result<()> {
    command.defer(&ctx.http).await?;

    // 서버 내에서만 명령어 실행 가능하도록 검증
    let Some(guild_id) = command.guild_id else {
        command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .content("❌ 이 명령어는 서버 안에서만 사용 가능합니다."),
            )
            .await?;
        return Ok(());
    };

    // 첫 번째 옵션에서 서브커맨드 추출 [generate, rename, delete]
    let Some(subcommand_option) = command.data.options.first() else {
        command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("❌ 올바른 하위 명령어를 선택해주세요."),
            )
            .await?;
        return Ok(());
    };

    // 공유 캐시 및 알림 채널 불러오기
    let data_read = ctx.data.read().await;
    let cache_lock = data_read
        .get::<crate::cache::SharedCacheKey>()
        .expect("보관함에 캐시가 없습니다.")
        .clone();

    // 필요한 컨텍스트를 구조체로 묶기
    let channel_manager = ChannelManager {
        ctx,
        guild_id,
        command,
        cache_lock,
    };

    // 서브커맨드 이름 매칭 분기
    match subcommand_option.name.as_str() {
        "generate" => {
            // 서브커맨드 하위에 포함된 인자(name) 추출
            let project_name = match get_string_arg(subcommand_option, "name") {
                Some(name) => name,
                None => {
                    command
                        .edit_response(
                            &ctx.http,
                            EditInteractionResponse::new()
                                .content("⚠️ 생성할 프로젝트 이름을 제대로 입력해주세요."),
                        )
                        .await?;
                    return Ok(());
                }
            };

            if find_already_exist_project_name(&channel_manager, &project_name).await {
                return Ok(());
            }

            generate_project(&channel_manager, project_name).await?;
        }

        "rename" => {
            // 서브커맨드 하위에 포함된 인자(new_name) 추출
            let new_name = match get_string_arg(subcommand_option, "new_name") {
                Some(name) => name,
                None => {
                    command
                        .edit_response(
                            &ctx.http,
                            EditInteractionResponse::new()
                                .content("⚠️ 변경할 새 이름을 입력해주세요."),
                        )
                        .await?;
                    return Ok(());
                }
            };

            let error_msg = "❌ 프로젝트 카테고리 내부 채널에서 명령어를 입력해주세요.";
            let Some(category_id) = get_project_category_id(&channel_manager, error_msg).await
            else {
                return Ok(());
            };

            // 프로젝트 PM만 이름 변경 가능하도록 검증
            if is_project_pm(&channel_manager, category_id).await == false {
                command
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new()
                            .content("❌ 프로젝트 PM만 이름 변경이 가능합니다."),
                    )
                    .await?;
                return Ok(());
            }

            let old_name = match category_id.to_channel(&ctx.http).await {
                Ok(Channel::Guild(cat_channel)) => cat_channel.name,
                _ => String::new(),
            };

            if find_already_exist_project_name(&channel_manager, &new_name).await {
                return Ok(());
            }

            rename_project(&channel_manager, category_id, old_name, new_name).await?;
        }

        "delete" => {
            let guild = guild_id.to_partial_guild(&ctx.http).await?;
            let Some(member) = &command.member else {
                return Ok(());
            };

            let error_msg = "❌ 삭제할 프로젝트 카테고리 내부의 채널에서 명령어를 입력해주세요.";
            let Some(category_id) = get_project_category_id(&channel_manager, error_msg).await
            else {
                return Ok(());
            };

            let is_admin = guild.owner_id == command.user.id
                || member.roles.iter().any(|role_id| {
                    guild
                        .roles
                        .get(role_id)
                        .map_or(false, |r| r.permissions.administrator())
                });

            if !is_admin {
                command
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content(
                            "❌ 이 명령어는 서버 관리자(ADMINISTRATOR) 권한이 필요합니다.",
                        ),
                    )
                    .await?;
                return Ok(());
            }

            delete_project(&channel_manager, category_id).await?;
        }
        _ => {}
    }

    Ok(())
}
