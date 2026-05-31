use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ApiRequest {
    pub action: String,
    #[serde(default)]
    pub pane_id: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub ok: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ApiResponse {
    pub fn ok(message: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            ok: true,
            message: message.into(),
            data,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: message.into(),
            data: None,
        }
    }
}
