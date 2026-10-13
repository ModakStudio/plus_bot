mod cache;
mod commands; // 봇이 관리할 캐시 모듈 등록
mod integration;
mod refresh_memlist;

use std::env;
use std::sync::Arc;

use serenity::all::Interaction;
use serenity::async_trait;
use serenity::model::event::ResumedEvent;
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::prelude::*;

use tracing::{error, info};

use crate::cache::*;
use crate::refresh_memlist::*;

struct Handler {
    guild_id: GuildId,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);

        let guild_id = self.guild_id;

        // 슬래시 커맨드 등록 목록
        let commands_list = vec![
            commands::help::register_command::register_help_command(),
            commands::project::register_command::register_project_command(),
            commands::member::register_command::register_member_command(),
        ];

        if let Err(why) = guild_id.set_commands(&ctx.http, commands_list).await {
            error!("슬래시 커맨드 등록 실패: {}", why);
        } else {
            info!("슬래시 커맨드 등록 성공!");
        }

        // TypeMap에서 캐시 락 가져오기
        let cache_lock = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<crate::cache::SharedCacheKey>()
                .expect("Expected SharedCacheKey in TypeMap")
                .clone()
        };

        let ctx_clone = ctx.clone();

        tokio::spawn(async move {
            info!("🔄 [초기화] 백그라운드 노션 데이터 동기화 시작...");

            // 1. 봇이 켜졌으므로 캐시가 비어있습니다. 노션에서 최신 데이터를 무조건 1회 긁어옵니다.
            let _ = BotCache::refresh_about_project(&cache_lock).await;
            let _ = BotCache::refresh_about_member(&cache_lock).await;

            // 2. 갱신된 캐시에서 프로젝트 목록 읽기
            let projects = BotCache::get_all_projects(&cache_lock).await;
            info!(
                "✅ 캐시 초기화 완료. 관리 대상 프로젝트 수: {}",
                projects.len()
            );

            // 3. 모든 프로젝트 카테고리의 UI 갱신
            let _ = update_all_project_embeds(&ctx_clone, guild_id, projects).await;

            info!("🎉 [초기화] 모든 카테고리 UI 업데이트 완료!");
        });
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        // 들어온 상호작용이 슬래시 커맨드(Command)일 때만 처리
        if let Interaction::Command(command) = interaction {
            match command.data.name.as_str() {
                "help" => {
                    if let Err(why) =
                        commands::help::run_help_command(&ctx, &command, self.guild_id).await
                    {
                        error!("help 커맨드 실행 오류: {:?}", why);
                    }
                }
                "project" => {
                    if let Err(why) = commands::project::run_project_command(&ctx, &command).await {
                        error!("커맨드 실행 오류: {:?}", why);
                    }
                }
                "member" => {
                    if let Err(why) = commands::member::run_member_command(&ctx, &command).await {
                        error!("커맨드 실행 오류: {:?}", why);
                    }
                }
                _ => info!("알 수 없는 커맨드: {}", command.data.name),
            }
        }
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[tokio::main]
async fn main() {
    // 현재 작업 중인 CWD에 대한 상대 경로로 .env 파일에서 환경 변수 로드
    dotenv::dotenv().expect("Failed to load .env file");

    // 로거 초기화
    tracing_subscriber::fmt::init();

    // env 파일에서 토큰 및 서버 ID 로드
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let guild_id = GuildId::new(
        env::var("SERVER_ID")
            .expect("Expected SERVER_ID in env")
            .parse::<u64>()
            .expect("Expected a server id in the environment"),
    );

    // ⚡ 인텐트 최적화
    // 슬래시 커맨드는 별도 메시지 인텐트가 필요 없으나, guild_member_update 이벤트를 수신하기 위해 GUILD_MEMBERS는 필수입니다.
    let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_MEMBERS;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler { guild_id })
        .await
        .expect("Err creating client");

    // 캐시 저장소 초기 설정
    let shared_cache = Arc::new(RwLock::new(cache::BotCache::new()));

    // 클라이언트 데이터에 캐시 및 캐시 동기화 채널 저장
    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<cache::SharedCacheKey>(shared_cache.clone());
        // data.insert::<CacheNotifyKey>(cache_tx);
    }

    // Ctrl + C 종료 핸들러 스레드 시작
    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
