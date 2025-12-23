use serde::{Deserialize, Serialize};

pub const SETTING_API_KEY: &str = "api_key";
pub const SETTING_API_BASE_URL: &str = "api_base_url";
pub const SETTING_DEBUG_LOGS: &str = "debug_logs";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestaustimeSettings {
    pub api_key: Option<String>,
    pub api_base_url: Option<String>,
    pub debug_logs: Option<bool>,
}

impl TestaustimeSettings {
    pub fn from_json(value: &serde_json::Value) -> Self {
        Self {
            api_key: value
                .get(SETTING_API_KEY)
                .and_then(|v| v.as_str())
                .map(String::from),
            api_base_url: value
                .get(SETTING_API_BASE_URL)
                .and_then(|v| v.as_str())
                .map(String::from),
            debug_logs: value.get(SETTING_DEBUG_LOGS).and_then(|v| v.as_bool()),
        }
    }

    pub fn to_init_options(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();

        if let Some(ref api_key) = self.api_key {
            map.insert(SETTING_API_KEY.to_string(), serde_json::json!(api_key));
        }

        if let Some(ref api_base_url) = self.api_base_url {
            map.insert(
                SETTING_API_BASE_URL.to_string(),
                serde_json::json!(api_base_url),
            );
        }

        if let Some(debug_logs) = self.debug_logs {
            map.insert(
                SETTING_DEBUG_LOGS.to_string(),
                serde_json::json!(debug_logs),
            );
        }

        serde_json::Value::Object(map)
    }
}
