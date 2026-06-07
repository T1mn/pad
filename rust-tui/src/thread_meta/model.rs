#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ThreadMetaKey {
    pub agent_type: String,
    pub thread_id: String,
}

impl ThreadMetaKey {
    pub fn new(agent_type: impl Into<String>, thread_id: impl Into<String>) -> Self {
        Self {
            agent_type: agent_type.into(),
            thread_id: thread_id.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadMeta {
    pub title_override: Option<String>,
    pub generated_title: Option<String>,
    pub generated_turn_count: Option<usize>,
    pub generated_updated_at: Option<i64>,
    pub deleted: bool,
    pub deleted_at: Option<i64>,
    pub note: Option<String>,
    pub pinned: bool,
    pub tags: Vec<String>,
    pub updated_at: i64,
}
