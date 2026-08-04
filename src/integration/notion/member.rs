use reqwest;
use serde_json::Value;

use super::env::*;

pub async fn get_members_id() {
    let client = reqwest::Client::new();

    let database_response = client
        .get(format!(
            "https://api.notion.com/v1/databases/{}",
            get_notion_member_database_id()
        ))
        .header("Authorization", format!("Bearer {}", get_notion_token()))
        .header("Notion-Version", get_notion_version())
        .send()
        .await
        .expect("Notion API 요청에 실패했습니다.");

    let data_source_id = match database_response.status() {
        reqwest::StatusCode::OK => {
            let json: Value = database_response
                .json()
                .await
                .expect("Notion API 응답을 JSON으로 파싱하는 데 실패했습니다.");
            json["data_sources"][0]["id"]
                .as_str()
                .expect("Notion API 응답에서 데이터 소스 ID를 추출하는 데 실패했습니다.")
                .to_string()
        }
        _ => {
            eprintln!(
                "Notion API 요청이 실패했습니다. Status: {}",
                database_response.status()
            );
            return;
        }
    };

    let data_source_response = client
        .post(format!(
            "https://api.notion.com/v1/data_sources/{}/query",
            data_source_id
        ))
        .header("Authorization", format!("Bearer {}", get_notion_token()))
        .header("Notion-Version", get_notion_version())
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("Notion API 요청에 실패했습니다.");

    let response_text = data_source_response
        .text()
        .await
        .expect("Notion API 응답을 텍스트로 읽는 데 실패했습니다.");

    let response_json: Value = serde_json::from_str(&response_text)
        .expect("Notion API 응답을 JSON으로 파싱하는 데 실패했습니다.");

    println!("Notion API 응답: {}", response_json);
}
