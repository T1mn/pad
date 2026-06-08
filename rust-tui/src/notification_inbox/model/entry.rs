use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationEntry {
    pub id: String,
    pub ts: i64,
    pub event: String,
    pub agent_type: String,
    pub title: String,
    pub body: String,
    pub working_dir: Option<String>,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub pane_id: Option<String>,
    pub read: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationDraft {
    pub event: String,
    pub agent_type: String,
    pub title: String,
    pub body: String,
    pub working_dir: Option<String>,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub pane_id: Option<String>,
}

impl NotificationEntry {
    pub fn from_draft(draft: NotificationDraft, ts: i64) -> Self {
        Self {
            id: new_entry_id(ts),
            ts,
            event: draft.event,
            agent_type: draft.agent_type,
            title: draft.title,
            body: draft.body,
            working_dir: draft.working_dir,
            session_id: draft.session_id,
            transcript_path: draft.transcript_path,
            pane_id: draft.pane_id,
            read: false,
        }
    }
}

fn new_entry_id(ts: i64) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("n-{ts}-{nanos}-{}", std::process::id())
}
