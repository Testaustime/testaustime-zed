use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct APIClient {
    client: Client,
    base_url: String,
    api_key: String,
}

#[derive(Serialize, Debug)]
pub struct ActivityUpdate {
    project_name: String,
    language: String,
    editor_name: String,
    hostname: String,
}

impl ActivityUpdate {
    pub fn new(project_name: String, language: String, hostname: String) -> Self {
        Self {
            project_name,
            language,
            editor_name: "Zed".to_string(),
            hostname,
        }
    }
}

#[derive(Deserialize)]
pub struct MeModel {
    pub username: String,
}

impl APIClient {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "https://api.testaustime.fi".to_string()),
            api_key,
        }
    }

    pub async fn heartbeat(&self, data: ActivityUpdate) -> Result<(), reqwest::Error> {
        self.client
            .post(format!("{}/activity/update", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&data)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn flush(&self) -> Result<(), reqwest::Error> {
        self.client
            .post(format!("{}/activity/flush", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn validate_api_key(&self, key: &str) -> Result<MeModel, reqwest::Error> {
        let response = self
            .client
            .get(format!("{}/users/@me", self.base_url))
            .header("Authorization", format!("Bearer {}", key))
            .send()
            .await?
            .error_for_status()?;

        response.json().await
    }
}
