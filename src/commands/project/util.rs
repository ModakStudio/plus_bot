use serenity::builder::EditInteractionResponse;

use crate::commands::utils::ChannelManager;

// 이미 존재하는 프로젝트 이름인지 확인하고, 존재하면 에러 메시지 전송
pub async fn find_already_exist_project_name(
    channel_manager: &ChannelManager<'_>,
    name: &str,
) -> bool {
    // 1. 키 존재 여부만 빠르게 확인 후 락 자동 해제
    let exists = channel_manager
        .cache_lock
        .read()
        .await
        .project_name_to_id
        .contains_key(name);

    // 2. 락이 풀린 상태에서 안전하게 HTTP 통신 진행
    if exists {
        let _ = channel_manager
            .command
            .edit_response(
                &channel_manager.ctx.http,
                EditInteractionResponse::new()
                    .content(format!("❌ 이미 '{}' 이름의 프로젝트가 존재합니다.", name)),
            )
            .await;
        return true;
    }

    false
}
