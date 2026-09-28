use serenity::all::CreateCommand;

pub fn register_help_command() -> CreateCommand {
    CreateCommand::new("help")
        .description("봇의 모든 명령어와 사용법을 안내합니다.")
        .dm_permission(false)
}
