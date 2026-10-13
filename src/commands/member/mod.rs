pub mod add;
pub mod list;
pub mod register_command;
pub mod remove;
pub mod util;

use tracing::{error, info};

use serenity::all::{ChannelId, CommandInteraction, Context};
use serenity::builder::EditInteractionResponse;

use crate::cache::*;
use crate::commands::utils::*;
use crate::integration::notion::project::update_project;

use crate::refresh_memlist::sync_member_embed_internal;

use add::member_add;
use list::member_list;
use remove::member_remove;
use util::check_pm_or_admin;

// 명령어 실행 함수
pub async fn run_member_command(
    ctx: &Context,
    command: &CommandInteraction,
) -> serenity::Result<()> {
    // 타임아웃 방지를 위한 상호작용 defer 처리
    command.defer(&ctx.http).await?;

    // 명령어가 입력된 채널 정보 가져오기
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

    // 전역 공유 캐시 가져오기
    let data_read = ctx.data.read().await;
    let cache_lock = data_read
        .get::<crate::cache::SharedCacheKey>()
        .expect("보관함에 캐시가 없습니다.")
        .clone();

    let _ = BotCache::refresh_about_project(&cache_lock).await;

    // 필요한 컨텍스트를 구조체로 묶기
    let channel_manager = ChannelManager {
        ctx,
        guild_id,
        command,
        cache_lock,
    };

    // 프로젝트 카테고리 내부 채널에서만 명령어를 실행할 수 있도록 검증
    let error_message = "❌ 이 명령어는 프로젝트 카테고리 내부 채널에서만 사용 가능합니다.";
    let category_id = match get_project_category_id(&channel_manager, error_message).await {
        Some(id) => id,
        None => return Ok(()),
    };

    // 실제 명령어 처리 함수 호출
    handle_member_command(&channel_manager, category_id).await?;

    Ok(())
}

// 명령을 처리하는 실제 함수
async fn handle_member_command(
    channel_manager: &ChannelManager<'_>,
    category_id: ChannelId,
) -> serenity::Result<()> {
    let (ctx, command, cache_lock) = (
        channel_manager.ctx,
        channel_manager.command,
        &channel_manager.cache_lock,
    );

    // 첫 번째 옵션에서 어떤 서브커맨드가 들어왔는지 확인
    let subcommand_option = match command.data.options.first() {
        Some(opt) => opt,
        None => {
            command
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content("⚠️ 올바른 하위 명령어를 선택해주세요."),
                )
                .await?;
            return Ok(());
        }
    };

    // 카테고리 ID를 통해 프로젝트 이름을 가져오기
    let project_name = category_id
        .to_channel(&ctx.http)
        .await?
        .guild()
        .unwrap()
        .name;

    // 프로젝트 인덱스를 통해 프로젝트 정보 가져오기
    let mut my_project = match BotCache::get_project_by_name(cache_lock, &project_name).await {
        Some(project) => project,
        None => return Ok(()),
    };

    // 프로젝트에 참여중인 인원 수집, pm은 가장 앞에 위치
    let pm_discord_id = convert_string_to_discord_id(&my_project.pm.discord_id).unwrap();
    let mut included_mems = vec![pm_discord_id];

    // 프로젝트 참여자 명단을 순회하며 이름을 수집
    for member in &my_project.participants {
        included_mems.push(convert_string_to_discord_id(&member.discord_id).unwrap());
    }

    let subcommand_name = subcommand_option.name.as_str();
    match subcommand_name {
        // --- 1. 참여 인원 조회 (/member list) ---
        "list" => {
            member_list(&channel_manager, &project_name, &included_mems).await?;

            return Ok(());
        }

        // --- 2. 멤버 추가 / 내보내기 공통 로직 (add / remove) ---
        "add" | "remove" => {
            // 권한 체크 (PM 이거나 관리자 권한을 가진 유저인지 판별)
            if check_pm_or_admin(channel_manager, category_id)
                .await
                .is_err()
            {
                return Ok(());
            }

            // 각 서브커맨드별 처리 함수 호출
            let result = match subcommand_name {
                "add" => {
                    member_add(
                        &channel_manager,
                        subcommand_option,
                        &included_mems,
                        &mut my_project,
                    )
                    .await
                }
                "remove" => {
                    member_remove(
                        &channel_manager,
                        subcommand_option,
                        &included_mems,
                        &mut my_project,
                    )
                    .await
                }
                _ => unreachable!(),
            };

            // 프로젝트 정보 업데이트 및 캐시 갱신
            match result {
                Ok(()) => info!("멤버 {} 서브커맨드 처리 완료.", subcommand_name),
                Err(why) => {
                    error!("멤버 {} 중 오류 발생: {:?}", subcommand_name, why);
                    return Ok(());
                }
            }

            if let Err(e) =
                sync_member_embed_internal(&ctx, channel_manager.guild_id, category_id, &my_project)
                    .await
            {
                error!("UI 즉시 업데이트 실패: {:?}", e);
            }

            let action_text = if subcommand_name == "add" {
                "추가"
            } else {
                "삭제"
            };
            command
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content(format!(
                        "✅ 멤버 {}가 성공적으로 반영되었습니다!",
                        action_text
                    )),
                )
                .await?;

            // Notion API를 통해 프로젝트 정보 업데이트
            match update_project(&my_project.id, &my_project).await {
                Ok(_) => info!(
                    "프로젝트 '{}' 정보가 성공적으로 업데이트되었습니다.",
                    my_project.name
                ),
                Err(why) => {
                    error!(
                        "프로젝트 '{}' 정보 업데이트 실패: {:?}",
                        my_project.name, why
                    );

                    return Ok(());
                }
            }

            // 캐시 갱신
            match BotCache::refresh_about_project(cache_lock).await {
                Ok(_) => info!("프로젝트 캐시 갱신 완료."),
                Err(why) => {
                    error!("프로젝트 캐시 갱신 실패: {:?}", why);
                    return Ok(());
                }
            }
        }
        _ => {}
    }

    Ok(())
}
