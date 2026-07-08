use std::ops::Rem;

use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::{prelude::*};
use serenity::prelude::*;

// /member add
// /member remove

// 앞으로 해야할거
// 추후 개발방향: 노션에 연동해서 프로젝트 참여 인원 확인 하기



#[command]
async fn member(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    //let user = &msg.author;

    //명령어가 입력된 채널 정보 가져오기
    let channel = msg.channel_id.to_channel(&ctx.http).await?.guild().unwrap();
    
    //명령어가 카테고리에 속해있는 채널에서 입력된건지 확인
    let mut project_name:String = String::new();
    match channel.parent_id {
        Some(category_id) => {
            let category_channel = category_id.to_channel(&ctx.http).await?.guild().unwrap();
            project_name = category_channel.name.clone();
        }
        None => {
            msg.reply(ctx, "❌ 이 명령어는 프로젝트 내에서만 사용 가능합니다").await?;
            return Ok(());
        },
    };
    
    //인수 파싱
    let subcommand = match args.single::<String>() {
        Ok(cmd) => cmd,
        Err(_) => { //만약 뒤에 아무런 커맨드가 없다면 => 사용법 출력
            // //서버 아이디 획득
            // let guild_id = match msg.guild_id {
            //     Some(id) => id,
            //     None => return Ok(()),
            // };
            let data_read = ctx.data.read().await;
            let cache_lock = data_read
                .get::<crate::cache::SharedCacheKey>()
                .expect("보관함에 캐시가 없습니다.")
                .clone();
            let cache = cache_lock.read().await;

            // 보관된 해쉬맵에서 바로 꺼내쓰기
            let included_set = cache.project_mapping.get(&project_name);

            //인원 출력전 분류 vector
            let mut included_mems = Vec::new(); //참여 인원
            let mut excluded_mems = Vec::new(); //미참여 인원

            //전체 맴버 순회하면서 캐쉬랑 맞춰보고 포함, 배제 구분
            for mem in &cache.all_members {
                if let Some(set) = included_set {
                    if set.contains(mem) {
                        included_mems.push(mem.clone());
                    } else {
                        excluded_mems.push(mem.clone());
                    }
                } else {
                    excluded_mems.push(mem.clone()); // 해당 프로젝트에 아무도 없는 경우
                }
            }

            // 결과 출력
            let mut content = String::from("사용법: `~member <add | remove> [미참여 유저 이름]`\n\n");
            content.push_str("`미참여 인원`\n");
            for mem in excluded_mems { content.push_str(&format!("{}\n", mem)); }
            content.push_str("`참여 인원`\n");
            for mem in included_mems { content.push_str(&format!("{}\n", mem)); }

            msg.reply(ctx, content).await?;
            return Ok(());
        }
    };
    
    match subcommand.as_str() {
        "add" => {
            // add 기능 구현
        },
        "remove" => {
            // remove 기능 구현
        },
        _ => {
            msg.reply(ctx, "❌ 알 수 없는 하위 명령어입니다. (사용 가능: add, remove)").await?;
        }
    };
    
    Ok(())
}