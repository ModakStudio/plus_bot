pub mod register_command;

use serenity::all::*;
use serenity::builder::EditInteractionResponse;

use crate::cache::BotCache;

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

    // 캐시 갱신
    let _ = BotCache::refresh_about_project(&cache_lock).await;
    let _ = BotCache::refresh_about_member(&cache_lock).await;

    // 명령어 실행 결과 응답
    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new()
                .content("✅ 봇 캐시가 최신 상태로 갱신되었습니다."),
        )
        .await?;

    Ok(())
}