pub mod register_command;

use serenity::all::*;
use serenity::builder::EditInteractionResponse;

use crate::cache::BotCache;
use crate::refresh_proj_embed::update_all_project_embeds;

const COOLDOWN_DURATION: u64 = 60;

pub async fn run_refresh_command(
    ctx: &Context,
    command: &CommandInteraction,
) -> serenity::Result<()> {
    // 타임아웃 방지를 위한 상호작용 defer 처리
    command.defer(&ctx.http).await?;

    // 전역 공유 캐시 가져오기
    let data_read = ctx.data.read().await;
    let cache_lock = data_read
        .get::<crate::cache::SharedCacheKey>()
        .expect("보관함에 캐시가 없습니다.")
        .clone();

    // 마지막 갱신 시간 확인 및 쿨다운 적용
    {
        let cache = cache_lock.read().await;
        if let Some(last_time) = cache.last_refresh_time {
            let elapsed = last_time.elapsed().as_secs(); // 마지막 갱신 이후 경과 시간 계산

            // 쿨다운 시간보다 짧으면 명령어 실행 제한
            if elapsed < COOLDOWN_DURATION {
                let remaining = COOLDOWN_DURATION - elapsed;
                command
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content(format!(
                            "⏳ 캐시 갱신은 {}초마다 가능합니다. 잠시 후 다시 시도해주세요.",
                            remaining
                        )),
                    )
                    .await?;
                return Ok(());
            }
        }
    }

    // 캐시 갱신
    let _ = BotCache::refresh_about_project(&cache_lock).await;
    let _ = BotCache::refresh_about_member(&cache_lock).await;

    let projects;
    {
        // 갱신 후 프로젝트 목록을 가져오기 위해 캐시 읽기
        let mut cache = cache_lock.write().await;

        // 마지막 갱신 시간 업데이트
        cache.last_refresh_time = Some(std::time::Instant::now());

        // 프로젝트 목록 가져오기
        projects = cache.project_id_to_project.values().cloned().collect();
    }

    // 명령어 실행 결과 응답
    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content("✅ 봇 캐시가 최신 상태로 갱신되었습니다."),
        )
        .await?;

    // 프로젝트 임베드 갱신을 비동기적으로 수행
    let ctx_clone = ctx.clone();
    let guild_id = command.guild_id.unwrap().clone();
    if let Err(e) = update_all_project_embeds(&ctx_clone, guild_id, projects).await {
        tracing::error!("프로젝트 임베드 갱신 중 오류 발생: {:?}", e);
    }

    Ok(())
}
