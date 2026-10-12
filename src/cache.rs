/*
cache.rs
프로젝트를 저장하는 캐쉬 구조 구현
*/

use std::collections::HashMap;
use std::sync::Arc;

use tracing::{error, info};

use serenity::all::UserId;
use serenity::gateway::ShardManager;
use serenity::prelude::TypeMapKey;

use tokio::sync::RwLock;

use crate::commands::utils::convert_string_to_discord_id;
use crate::integration::notion::{member::*, project::*};

pub struct SharedCacheKey;

impl TypeMapKey for SharedCacheKey {
    type Value = Arc<RwLock<BotCache>>;
}

pub struct ShardManagerContainer;
impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<ShardManager>;
}

//봇이 전체적으로 공유할 캐쉬 구조체
pub struct BotCache {
    // 유저 아이디로 관리
    pub user_id_to_notion_member: HashMap<UserId, NotionMember>,

    pub project_name_to_id: HashMap<String, String>, // 프로젝트 이름 -> 프로젝트 노션 아이디
    pub project_id_to_project: HashMap<String, Project>, // 프로젝트 노션 아이디 -> 프로젝트 정보
}

impl BotCache {
    pub fn new() -> Self {
        BotCache {
            user_id_to_notion_member: HashMap::new(),

            project_name_to_id: HashMap::new(),
            project_id_to_project: HashMap::new(),
        }
    }

    /// 캐시 미스 시 자동으로 리프레시 후 재조회하는 공통 제네릭 도우미 함수
    async fn get_or_refresh<T, FGet, FRef, Fut>(
        cache_lock: &Arc<RwLock<BotCache>>,
        getter: FGet,
        refresher: FRef,
        key_desc: &str, // 에러 로그용 설명 (예: "유저 ID 1234", "프로젝트 '플러스봇'")
    ) -> Option<T>
    where
        FGet: Fn(&BotCache) -> Option<T>,
        FRef: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
    {
        info!("🔍 캐시 조회 시도: [{}]", key_desc);

        // 1. Read Lock으로 캐시 조회
        {
            let cache = cache_lock.read().await;
            if let Some(val) = getter(&cache) {
                return Some(val);
            }
        }

        // 2. 캐시 미스 시 갱신 실행
        info!(
            "❌ 캐시에서 [{}]를 찾지 못했습니다. 캐시를 갱신합니다.",
            key_desc
        );
        let _ = refresher().await;

        // 3. 갱신 후 재조회
        let cache = cache_lock.read().await;
        let res = getter(&cache);
        if res.is_none() {
            error!("❌ 캐시 갱신 후에도 [{}]를 찾지 못했습니다.", key_desc);
        }
        res
    }

    // 유저 아이디로 멤버 정보를 가져오는 함수
    pub async fn get_member_by_user_id(
        cache_lock: &Arc<RwLock<BotCache>>,
        user_id: UserId,
    ) -> Option<NotionMember> {
        Self::get_or_refresh(
            cache_lock,
            |c| {
                c.user_id_to_notion_member.get(&user_id).cloned()
            },
            || Self::refresh_about_member(cache_lock),
            &format!("유저 ID {}", user_id),
        )
        .await
    }

    // 프로젝트 이름으로 프로젝트 id를 가져오는 함수
    pub async fn get_project_id_by_name(
        cache_lock: &Arc<RwLock<BotCache>>,
        project_name: &str,
    ) -> Option<String> {
        Self::get_or_refresh(
            cache_lock,
            |c| c.project_name_to_id.get(project_name).cloned(),
            || Self::refresh_about_project(cache_lock),
            &format!("프로젝트 이름 '{}'", project_name),
        )
        .await
    }

    // 프로젝트 ID로 프로젝트 정보를 가져오는 함수
    pub async fn get_project_by_id(
        cache_lock: &Arc<RwLock<BotCache>>,
        project_id: &str,
    ) -> Option<Project> {
        Self::get_or_refresh(
            cache_lock,
            |c| {
                c.project_id_to_project.get(project_id).cloned()
            },
            || Self::refresh_about_project(cache_lock),
            &format!("프로젝트 ID '{}'", project_id),
        )
        .await
    }

    // 프로젝트 이름으로 프로젝트 정보를 가져오는 함수
    pub async fn get_project_by_name(
        cache_lock: &Arc<RwLock<BotCache>>,
        project_name: &str,
    ) -> Option<Project> {
        let project_id = Self::get_project_id_by_name(cache_lock, project_name).await?;

        Self::get_project_by_id(cache_lock, &project_id).await
    }

    // 프로젝트 관련 캐시 갱신
    pub async fn refresh_about_project(
        cache_lock: &Arc<RwLock<BotCache>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 노션 API에서 프로젝트 목록 가져오기
        let projects = get_projects().await?;
        let mut cache = cache_lock.write().await;

        // 기존 매핑 초기화
        cache.project_name_to_id.clear();
        cache.project_id_to_project.clear();

        // 모든 프로젝트를 캐시에 저장
        for project in projects.iter() {
            cache
                .project_name_to_id
                .insert(project.name.clone(), project.id.clone());
            cache.project_id_to_project.insert(project.id.clone(), project.clone());

            // println!("{:?} 프로젝트 캐시 갱신 완료", project);
        }

        Ok(())
    }

    // 멤버 관련 캐시 갱신
    pub async fn refresh_about_member(
        cache_lock: &Arc<RwLock<BotCache>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 노션 API에서 멤버 목록 가져오기
        let members = match get_members().await {
            Ok(members) => members,
            Err(e) => {
                error!("멤버 목록을 가져오는 데 실패했습니다: {}", e);
                return Err(e);
            }
        };

        // 캐시 잠금 획득
        let mut cache = cache_lock.write().await;

        // 기존 매핑 초기화
        cache.user_id_to_notion_member.clear();

        // 유저 아이디와 인덱스를 매핑
        for member in members.iter() {
            if let Some(user_id) = convert_string_to_discord_id(&member.discord_id) {
                cache.user_id_to_notion_member.insert(user_id, member.clone());
            } else {
                error!(
                    "멤버 {}의 Discord ID 변환에 실패했습니다: {}",
                    member.name, member.discord_id
                );
            }
        }

        Ok(())
    }

    pub async fn get_all_projects(cache_lock: &Arc<RwLock<BotCache>>) -> Vec<Project> {
        cache_lock.read().await.project_id_to_project.values().cloned().collect()
    }

    // pub async fn get_all_members(cache_lock: &Arc<RwLock<BotCache>>) -> Vec<NotionMember> {
    //     cache_lock.read().await.user_id_to_notion_member.values().cloned().collect()
    // }
}
