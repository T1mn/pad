//! Persisted Pi session metadata and Codex-style sidebar hierarchy models.

use super::PolicyLayer;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Minimal Pi session header retained from the append-only JSONL journal.
#[allow(
    dead_code,
    reason = "Pi journal DTO is retained for the read-only session index and recovery work"
)]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct PiSessionHeader {
    #[serde(rename = "type", default)]
    pub entry_type: String,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub cwd: PathBuf,
    #[serde(rename = "parentSession", default)]
    pub parent_session: Option<String>,
}

/// PAD's cheap read model for a Pi session.  It is safe to rebuild from Pi's
/// JSONL entries after a crash and therefore must never be the conversation
/// source of truth.
#[allow(
    dead_code,
    reason = "Pi session metadata DTO is retained for read-only history recovery"
)]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct PiSessionMetadata {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub session_file: PathBuf,
    #[serde(default)]
    pub cwd: PathBuf,
    #[serde(default)]
    pub parent_session_id: Option<String>,
    #[serde(default)]
    pub leaf_id: Option<String>,
    #[serde(default)]
    pub session_name: Option<String>,
    #[serde(default)]
    pub message_count: u64,
    #[serde(default)]
    pub entry_count: u64,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Incremental sync cursor for `get_entries(since)` style Pi RPC recovery.
#[allow(
    dead_code,
    reason = "Pi recovery cursor DTO is retained for incremental history synchronization"
)]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct PiSessionCursor {
    #[serde(default)]
    pub last_entry_id: Option<String>,
    #[serde(default)]
    pub leaf_id: Option<String>,
    #[serde(default)]
    pub rpc_sequence: u64,
}

/// Stable identity of one entry in Pi's session tree.
#[allow(
    dead_code,
    reason = "Pi entry identity DTO is retained for append-only journal lineage recovery"
)]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct PiSessionEntryRef {
    pub id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(rename = "type", default)]
    pub entry_type: String,
}

/// Execution state displayed by the sidebar and task header.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskStatus {
    #[default]
    Idle,
    Starting,
    Running,
    Streaming,
    ToolRunning,
    NeedsApproval,
    NeedsInput,
    Compacting,
    Retrying,
    Disconnected,
    Failed,
    Completed,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskEnvironment {
    #[default]
    Local,
    Worktree,
    Remote,
}

/// A credential/profile boundary.  `credential_ref` is a Keychain reference,
/// never the secret itself.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct Profile {
    pub id: String,
    pub name: String,
    pub agent_dir: PathBuf,
    pub session_dir: PathBuf,
    #[serde(default)]
    pub credential_ref: Option<String>,
    #[serde(default)]
    pub default_provider: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub policy: PolicyLayer,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}

/// A Codex-style project: one primary cwd plus optional readable/editable roots.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct Project {
    pub id: String,
    pub name: String,
    pub primary_root: PathBuf,
    #[serde(default)]
    pub additional_roots: Vec<PathBuf>,
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub policy: PolicyLayer,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}

/// A single Pi-backed conversation/task shown in the sidebar.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct Task {
    pub id: String,
    #[serde(default)]
    pub project_id: Option<String>,
    pub profile_id: String,
    #[serde(default)]
    pub pi_session_id: Option<String>,
    #[serde(default)]
    pub session_file: Option<PathBuf>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub summary: String,
    pub cwd: PathBuf,
    #[serde(default)]
    pub environment: TaskEnvironment,
    #[serde(default)]
    pub status: TaskStatus,
    #[serde(default)]
    pub leaf_id: Option<String>,
    #[serde(default)]
    pub unread: bool,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub policy: PolicyLayer,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub(crate) enum SectionItem {
    Project(String),
    Task(String),
}

/// A user-created sidebar section.  Organization changes never alter the
/// Project/Task context or the underlying Pi session.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct Section {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub order: u32,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub items: Vec<SectionItem>,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}
