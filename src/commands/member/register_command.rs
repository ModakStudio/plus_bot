use serenity::all::*;

// 슬래시 커맨드 등록 함수
pub fn register_member_command() -> CreateCommand {
    CreateCommand::new("member")
        .description("프로젝트 멤버를 관리하는 명령어입니다.")
        .dm_permission(false)
        // 1. list 서브커맨드 (/member list)
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "list",
            "현재 프로젝트에 참여 중인 인원 목록을 확인합니다.",
        ))
        // 2. add 서브커맨드 (/member add [user1] [user2] [user3])
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "add",
                "프로젝트에 새 멤버를 추가하고 역할을 부여합니다.",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::User, "user1", "추가할 첫 번째 유저")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "user2",
                    "추가할 두 번째 유저 (선택)",
                )
                .required(false),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "user3",
                    "추가할 세 번째 유저 (선택)",
                )
                .required(false),
            ),
        )
        // 3. remove 서브커맨드 (/member remove [user1] [user2] [user3])
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "remove",
                "프로젝트에서 멤버를 내보내고 역할을 회수합니다.",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::User, "user1", "내보낼 첫 번째 유저")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "user2",
                    "내보낼 두 번째 유저 (선택)",
                )
                .required(false),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "user3",
                    "내보낼 세 번째 유저 (선택)",
                )
                .required(false),
            ),
        )
}
