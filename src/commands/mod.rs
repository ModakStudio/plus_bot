pub mod help;
pub mod member;
pub mod project;

use serenity::all::*;

// 💡 필요한 컨텍스트를 구조체로 묶습니다.
pub struct ChannelManager<'a> {
    pub ctx: &'a Context,
    pub guild_id: GuildId,
    pub command: &'a CommandInteraction,
    pub cache: tokio::sync::RwLockReadGuard<'a, crate::cache::BotCache>,
    pub tx: tokio::sync::mpsc::Sender<crate::cache::CacheCommand>,
}
