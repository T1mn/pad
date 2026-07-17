use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrokThreadRef {
    pub session_id: String,
    pub cwd: PathBuf,
    pub updated_at: i64,
    pub transcript_path: PathBuf,
    pub title: Option<String>,
    pub model_name: Option<String>,
}
