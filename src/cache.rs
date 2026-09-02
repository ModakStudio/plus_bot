/*
cache.rs
프로젝트를 저장하는 캐쉬 구조 구현
*/

use std::collections::HashMap;
use std::sync::Arc;

use tracing::{error, info};

use serenity::all::UserId;
use serenity::gateway::ShardManager;
use serenity::http::Http;
use serenity::model::id::GuildId;
use serenity::prelude::TypeMapKey;

use tokio::sync::{mpsc, RwLock};

use crate::integration::notion::project::{get_projects, Project};

//봇이 전체적으로 공유할 캐쉬 구조체
pub struct BotCache {
    // 유저 아이디로 관리
    pub all_members: HashMap<UserId, String>,
    pub project_name_to_id: HashMap<String, String>, // 프로젝트 이름 -> 프로젝트 노션 아이디
    pub project_id_mapping: HashMap<String, usize>,  // 프로젝트 노션 아이디 -> project_vec 인덱스
    pub project_vec: Vec<Project>,
}

// 캐시 스레드가 처리할 명령 목록
#[derive(Debug)]
pub enum CacheCommand {
    RefreshAll, // 모든 캐시를 갱신
    UpdateSingleMember {
        // 특정 유저의 이름만 타겟팅해서 즉시 갱신 (추후 확장용)
        user_id: serenity::model::id::UserId,
        display_name: String,
    },
    // AddProjectMembers {
    //     // 특정 프로젝트에 참여중인 유저 목록 추가
    //     project_name: String,
    //     user_ids: HashSet<UserId>,
    // },
    // RemoveProjectMembers {
    //     // 특정 프로젝트에 참여중인 유저 목록 제거
    //     project_name: String,
    //     user_ids: HashSet<UserId>,
    // },
}

pub struct SharedCacheKey;

impl TypeMapKey for SharedCacheKey {
    type Value = Arc<RwLock<BotCache>>;
}

// 💡 봇 전체에서 "캐시 갱신 신호"를 보낼 수 있도록 Sender를 전역 키로 등록합니다.
pub struct CacheNotifyKey;
impl TypeMapKey for CacheNotifyKey {
    type Value = mpsc::Sender<CacheCommand>;
}

pub struct ShardManagerContainer;
impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<ShardManager>;
}

// 쓰레드 구성
pub fn start_cache_thread(
    cache: Arc<RwLock<BotCache>>,
    _http: Arc<Http>,
    _guild_id: GuildId,
) -> mpsc::Sender<CacheCommand> {
    // 버퍼 크기가 32인 비동기 채널 생성(가동신호 수신용)
    let (tx, mut rx) = mpsc::channel::<CacheCommand>(32);

    tokio::spawn(async move {
        info!("백그라운드 동기화 스레드 가동");

        // 봇이 켜졌을떄 한 번 연동
        refresh_cache(&cache).await;

        while let Some(command) = rx.recv().await {
            match command {
                CacheCommand::RefreshAll => {
                    info!("[캐시] 전체 캐시 즉시 동기화 요청 처리 중...");
                    refresh_cache(&cache).await;
                }
                CacheCommand::UpdateSingleMember {
                    user_id,
                    display_name,
                } => {
                    info!("[캐시] {} 님의 단일 캐시 업데이트 중...", display_name);
                    update_single_member(&cache, user_id, display_name).await;
                } // CacheCommand::AddProjectMembers {
                  //     project_name,
                  //     user_ids,
                  // } => {
                  //     info!("[캐시] {} 프로젝트 참여자 목록 추가 중...", project_name);
                  //     add_project_members(&cache, project_name, user_ids).await;
                  // }
                  // CacheCommand::RemoveProjectMembers {
                  //     project_name,
                  //     user_ids,
                  // } => {
                  //     info!("[캐시] {} 프로젝트에서 참여자 제외 중...", project_name);
                  //     remove_project_members(&cache, project_name, user_ids).await;
                  // }
                  // _ => {
                  //     info!("[캐시] 알 수 없는 명령어 수신: {:?}", command);
                  // }
            }
        }
        // 만약 봇이 꺼지거나 tx를 가진 곳이 전부 드롭되면 루프 종료.
        info!("백그라운드 동기화 스레드 종료");
    });

    tx
}

// 캐쉬 갱신 함수
async fn refresh_cache(cache: &Arc<RwLock<BotCache>>) {
    let mut new_cache = BotCache {
        all_members: HashMap::new(),
        project_name_to_id: HashMap::new(),
        project_id_mapping: HashMap::new(),
        project_vec: Vec::new(),
    };

    match get_projects().await {
        Ok(projects) => {
            new_cache.project_vec = projects;
            // 프로젝트 이름과 ID를 키로 사용하여 인덱스를 매핑
            for (index, project) in new_cache.project_vec.iter().enumerate() {
                new_cache
                    .project_name_to_id
                    .insert(project.name.clone(), project.id.clone());
                new_cache
                    .project_id_mapping
                    .insert(project.id.clone(), index);
            }
        }
        Err(e) => {
            error!("프로젝트 목록을 가져오는 데 실패했습니다: {}", e);
        }
    }

    //새로 갱신한 값 덮어쓰기
    {
        let mut lock = cache.write().await;
        *lock = new_cache;
    }
    info!("백그라운드 데이터 갱신 완료");
}

// 단일 유저 캐시 갱신
async fn update_single_member(
    cache: &Arc<RwLock<BotCache>>,
    user_id: UserId,
    display_name: String,
) {
    let mut guard = cache.write().await;
    guard.all_members.insert(user_id, display_name);
}
