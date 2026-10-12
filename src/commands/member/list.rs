use std::fmt::Write;
use tracing::error;

use serenity::all::*;
use serenity::builder::EditInteractionResponse;

use crate::cache::*;
use crate::commands::utils::*;

pub async fn member_list(
    channel_manager: &ChannelManager<'_>,
    project_name: &str,
    included_mems: &Vec<UserId>,
) -> serenity::Result<()> {
    let (ctx, command, cache_lock) = (
        channel_manager.ctx,
        channel_manager.command,
        &channel_manager.cache_lock,
    );

    // 참여 인원 명단을 문자열로 포맷팅
    let mut content = format!("📌 **'{}' 프로젝트 참여 인원 명단**\n", project_name);
    if included_mems.is_empty() {
        content.push_str("현재 등록된 멤버가 없습니다.\n");
    } else {
        for mem in included_mems {
            // content.push_str(&format!("• {}\n", mem)); // 반복문 안에서 format 사용 시 성능 저하 우려
            // write! 매크로는 버퍼 뒤에 바로 문자열을 포매팅해 추가해 성능저하 적음
            match BotCache::get_member_by_user_id(&cache_lock, *mem).await {
                Some(member) => write!(content, "• {}\n", member.name).unwrap(),
                None => error!("❌ 캐시에서 {} 유저를 찾지 못했습니다.", mem),
            };
        }
    }

    command
        .edit_response(&ctx.http, EditInteractionResponse::new().content(content))
        .await?;

    Ok(())
}
