use serenity::all::*;

// 프로젝트 관련 슬래시 커맨드 등록
pub fn register_project_command() -> CreateCommand {
    CreateCommand::new("project")
        .description("프로젝트 관련 명령어")
        .dm_permission(false)
        .default_member_permissions(Permissions::MANAGE_CHANNELS)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "generate",
                "새 프로젝트를 생성합니다.",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "name", "생성할 프로젝트 이름")
                    .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "rename",
                "프로젝트 이름을 변경합니다.",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "new_name",
                    "변경할 새 프로젝트 이름",
                )
                .required(true),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "delete",
            "프로젝트를 완전히 삭제합니다.",
        ))
}
