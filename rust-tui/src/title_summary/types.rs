pub(super) const TITLE_SUMMARY_MODEL: &str = "gpt-5.1-codex-mini";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SummaryWireApi {
    Responses,
    Chat,
}

impl SummaryWireApi {
    pub fn from_config(value: &str) -> Self {
        if value.trim().eq_ignore_ascii_case("chat") {
            Self::Chat
        } else {
            Self::Responses
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TitleSummaryResult {
    pub request_key: String,
    pub session_id: String,
    pub turn_count: usize,
    pub title: Option<String>,
    pub error: Option<String>,
}
