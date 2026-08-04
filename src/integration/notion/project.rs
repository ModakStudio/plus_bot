use reqwest;
use serde_json::Value;

use super::env::*;

pub async fn get_projects() -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let database_response = client
        .get(&format!(
            "https://api.notion.com/v1/databases/{}",
            get_notion_project_database_id()
        ))
        .header("Authorization", format!("Bearer {}", get_notion_token()))
        .header("Notion-Version", get_notion_version())
        .send()
        .await?;

    let data_source_id = match database_response.status() {
        reqwest::StatusCode::OK => {
            let json: Value = database_response.json().await?;
            json["data_sources"][0]["id"]
                .as_str()
                .ok_or("Notion API 응답에서 데이터 소스 ID를 추출하는 데 실패했습니다.")?
                .to_string()
        }
        _ => {
            eprintln!(
                "Notion API 요청이 실패했습니다. Status: {}",
                database_response.status()
            );
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Notion API 요청 실패",
            )));
        }
    };

    let data_source_response = client
        .post(&format!(
            "https://api.notion.com/v1/data_sources/{}/query",
            data_source_id
        ))
        .header("Authorization", format!("Bearer {}", get_notion_token()))
        .header("Notion-Version", get_notion_version())
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let data_source_response_body: Value = data_source_response.json().await?;
    let data_source_response_body_pretty_json =
        serde_json::to_string_pretty(&data_source_response_body)
            .expect("Failed to serialize response body as JSON");
    println!("Response:\n{}", data_source_response_body_pretty_json);

    Ok(Vec::new())
}
