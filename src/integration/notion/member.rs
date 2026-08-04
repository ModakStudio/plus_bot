use reqwest;
use serde_json::Value;

use super::env::*;

#[derive(Debug, Clone)]
pub struct AbilityScore {
    pub fe: u8,
    pub be: u8,
    pub system: u8,
    pub android: u8,
    pub ios: u8,
    pub ai: u8,
}

#[derive(Debug, Clone)]
pub struct Memeber {
    pub id: String,
    pub name: String,
    pub github: String,
    pub email: String,
    pub phone_number: String,
    pub ability_score: AbilityScore,
    pub tier: u8,
}

impl From<&Value> for Memeber {
    fn from(value: &Value) -> Self {
        let properties = &value["properties"];
        let ability_score = AbilityScore {
            fe: properties["FE"]["number"].as_u64().unwrap_or(0) as u8,
            be: properties["BE"]["number"].as_u64().unwrap_or(0) as u8,
            system: properties["System"]["number"].as_u64().unwrap_or(0) as u8,
            android: properties["Android"]["number"].as_u64().unwrap_or(0) as u8,
            ios: properties["iOS"]["number"].as_u64().unwrap_or(0) as u8,
            ai: properties["AI"]["number"].as_u64().unwrap_or(0) as u8,
        };

        Memeber {
            id: properties["member"]["people"][0]["id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            name: properties["member"]["people"][0]["name"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            github: properties["github"]["url"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            email: properties["email"]["email"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            phone_number: properties["phone_number"]["phone_number"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            ability_score,
            tier: properties["tier"]["formula"]["number"]
                .as_str()
                .unwrap_or("0")
                .parse::<u8>()
                .unwrap_or(0),
        }
    }
}

pub async fn get_members() -> Result<Vec<Memeber>, Box<dyn std::error::Error>> {
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
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Notion API 요청 실패",
            )));
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

    match data_source_response.status() {
        reqwest::StatusCode::OK => {
            let json: Value = data_source_response
                .json()
                .await
                .expect("Notion API 응답을 JSON으로 파싱하는 데 실패했습니다.");
            let members: Vec<Memeber> = json["results"]
                .as_array()
                .expect("Notion API 응답에서 results 배열을 추출하는 데 실패했습니다.")
                .iter()
                .map(|item| item.into())
                .collect();

            Ok(members)
        }
        _ => {
            eprintln!(
                "Notion API 요청이 실패했습니다. Status: {}",
                data_source_response.status()
            );
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Notion API 요청 실패",
            )))
        }
    }
}
