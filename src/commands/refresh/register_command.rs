use serenity::all::*;

// 슬래시 커맨드 등록 함수
pub fn register_refresh_command() -> CreateCommand {
    CreateCommand::new("refresh")
        .description("봇이 관리하는 캐시를 최신 상태로 갱신합니다.")
        .dm_permission(false)
}
