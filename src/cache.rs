/*
cache.rs
프로젝트를 저장하는 캐쉬 구조 구현
*/

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use serenity::prelude::TypeMapKey;
use serenity::http::Http;
use serenity::model::id::GuildId;
use serenity::gateway::ShardManager;

//봇이 전체적으로 공유할 캐쉬 구조체
pub struct BotCache {
    pub all_members: Vec<String>,
    pub project_mapping: HashMap<String, HashSet<String>>,
}

pub struct SharedCacheKey;

impl TypeMapKey for SharedCacheKey {
    type Value = Arc<RwLock<BotCache>>;
}

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<ShardManager>;
}

pub fn start_cache_thread(cache: Arc<RwLock<BotCache>>, http: Arc<Http>, guild_id: GuildId) {
    tokio::spawn(async move {
        println!("백그라운드 동기화 스레드 가동");

        loop {
            if let Ok(members) = guild_id.members(&http, None, None).await {
                if let Ok(server_roles) = guild_id.roles(&http).await {
                    let mut new_cache = BotCache {
                        all_members:Vec::new(),
                        project_mapping: HashMap::new(),
                    };

                    //맴버 별로 순회하면서 해당 프로젝트에 참여중인지 아닌지 확인
                    for member in members {
                        let username = member.user.name.clone();
                        new_cache.all_members.push(username.clone());

                        //맴버가 가진 역할과 프로젝트명 비교
                        for role_id in &member.roles {
                            // 포함된 프로젝트에 매핑
                            if let Some(role) = server_roles.get(role_id) {
                                new_cache.project_mapping
                                    .entry(role.name.clone())
                                    .or_insert_with(HashSet::new)
                                    .insert(username.clone());
                                
                            }
                        }
                    }

                    //새로 갱신한 값 덮어쓰기
                    {
                        let mut lock = cache.write().await;
                        *lock = new_cache;
                    }
                    println!("백그라운드 데이터 갱신 완료");
                }
            }
            //10초 대기
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    });
}