use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenCodeThreadRef {
    pub session_id: String,
    pub cwd: PathBuf,
    pub updated_at: i64,
    pub db_path: PathBuf,
    pub title: Option<String>,
    pub first_user_message: Option<String>,
    pub last_user_message: Option<String>,
    pub last_assistant_message: Option<String>,
    pub provider_name: Option<String>,
    pub model_name: Option<String>,
    pub archived: bool,
}
