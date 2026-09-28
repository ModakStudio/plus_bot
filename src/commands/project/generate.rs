use tracing::error;

use serenity::all::{GuildId, Permissions};
use serenity::builder::{CreateChannel, EditInteractionResponse, EditRole};
use serenity::model::channel::{ChannelType, PermissionOverwrite, PermissionOverwriteType};
use serenity::model::id::RoleId;

use crate::cache::*;
use crate::commands::utils::*;
use crate::integration::notion::{member::*, project::*};

pub async fn generate_project(
    channel_manager: &ChannelManager<'_>,
    project_name: String,
) -> serenity::Result<()> {
    let (ctx, guild_id, command, cache_lock) = (
        channel_manager.ctx,
        channel_manager.guild_id,
        channel_manager.command,
        &channel_manager.cache_lock,
    );

    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(format!(
                "🏗️ '{}' 프로젝트 생성을 시작합니다. 세팅 중...",
                project_name
            )),
        )
        .await?;

    let Ok(bot_user) = ctx.http.get_current_user().await else {
        command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("❌ 봇 정보를 가져오지 못했습니다."),
            )
            .await?;
        return Ok(());
    };

    let Ok(project_role) = guild_id
        .create_role(&ctx.http, EditRole::new().name(&project_name))
        .await
    else {
        command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("❌ 역할 생성 실패"),
            )
            .await?;
        return Ok(());
    };

    let _ = ctx
        .http
        .add_member_role(guild_id, command.user.id, project_role.id, None)
        .await;

    // 카테고리 및 채널 생성을 위한 권한 묶음 가져오기
    let overwrites =
        build_permission_overwrites(guild_id, project_role.id, bot_user.id, command.user.id);

    let category_builder = CreateChannel::new(&project_name)
        .kind(ChannelType::Category)
        .permissions(vec![
            overwrites.deny_everyone.clone(),
            overwrites.allow_project.clone(),
            overwrites.allow_bot.clone(),
        ]);

    let Ok(category) = guild_id.create_channel(&ctx.http, category_builder).await else {
        command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("❌ 카테고리 생성 실패"),
            )
            .await?;
        return Ok(());
    };

    // 텍스트 채널 생성
    let text_channels = [
        "📜information",
        "🤖bot",
        "🌐dev",
        "🚩issue",
        "✅progress",
        "📢notice",
        "🎡random",
        "🖥️github",
    ];
    for ch_name in text_channels {
        let mut builder = CreateChannel::new(ch_name)
            .kind(ChannelType::Text)
            .category(category.id);

        builder = match ch_name {
            "🖥️github" => builder.permissions(vec![
                overwrites.deny_everyone.clone(),
                overwrites.readonly_project.clone(),
                overwrites.allow_bot.clone(),
            ]),
            "📜information" => builder.permissions(vec![
                overwrites.deny_everyone.clone(),
                overwrites.readonly_project.clone(),
                overwrites.allow_pm.clone(),
                overwrites.allow_bot.clone(),
            ]),
            _ => builder,
        };

        let _ = guild_id.create_channel(&ctx.http, builder).await;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // 음성 채널 생성
    let voice_builder = CreateChannel::new("🎙️ voice chat")
        .kind(ChannelType::Voice)
        .category(category.id);
    let _ = guild_id.create_channel(&ctx.http, voice_builder).await;

    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(format!(
                "🚀 <@{}> 님, 프로젝트 서버 세팅이 완료되었습니다!",
                command.user.id
            )),
        )
        .await?;

    // Notion 프로젝트 생성
    let pm = match BotCache::get_member_by_user_id(&cache_lock, command.user.id).await {
        Some(member) => member,
        None => NotionMember::default(),
    };

    let project = Project {
        name: project_name.clone(),
        pm: pm.clone(),
        category_id: category.id.to_string(),
        ..Default::default()
    };

    if let Err(why) = create_project(&project).await {
        error!(
            "Notion 프로젝트 등록 실패: {}. 에러: {:?}",
            project_name, why
        );
    }

    Ok(())
}

// 프로젝트 카테고리 및 채널 생성 시 필요한 권한 오버라이트를 구조체로 묶어 반환
struct ProjectOverwrites {
    deny_everyone: PermissionOverwrite,
    allow_project: PermissionOverwrite,
    readonly_project: PermissionOverwrite,
    allow_pm: PermissionOverwrite,
    allow_bot: PermissionOverwrite,
}

fn build_permission_overwrites(
    guild_id: GuildId,
    project_role_id: RoleId,
    bot_id: serenity::model::id::UserId,
    pm_id: serenity::model::id::UserId,
) -> ProjectOverwrites {
    ProjectOverwrites {
        deny_everyone: PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
            kind: PermissionOverwriteType::Role(RoleId::new(guild_id.get())),
        },
        allow_project: PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Role(project_role_id),
        },
        readonly_project: PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL,
            deny: Permissions::SEND_MESSAGES,
            kind: PermissionOverwriteType::Role(project_role_id),
        },
        allow_pm: PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(pm_id),
        },
        allow_bot: PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL
                | Permissions::SEND_MESSAGES
                | Permissions::MANAGE_CHANNELS,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(bot_id),
        },
    }
}
