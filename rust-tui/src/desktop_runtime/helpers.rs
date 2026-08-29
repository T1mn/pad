use crate::pad_store::PadStore;
use crate::permission_policy::{Profile, TaskStatus};
use crate::pi_runtime::{PiApprovalRequest, PiApprovalResponse, PiPoll, PiRuntimeStatus};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// Extract Pi message payloads from a persisted JSONL session without taking
/// ownership of the journal. A torn final append is ignored here because the
/// live Pi process remains the recovery authority.
pub(super) fn read_session_messages(path: &Path) -> Vec<Value> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|entry| entry.get("type").and_then(Value::as_str) == Some("message"))
        .filter_map(|entry| entry.get("message").cloned())
        .collect()
}

pub(super) fn automatic_ui_response(value: &Value) -> Option<Value> {
    let request = PiApprovalRequest::parse(value)?;
    // Full Access is limited to explicit permission gates. Ordinary confirm,
    // select, input, and editor prompts remain visible to the renderer.
    let raw = serde_json::to_string(value).ok()?.to_ascii_lowercase();
    // Protected provider/PAD credentials and session paths are never automatic.
    if raw.contains(".codex")
        || raw.contains("/.pi/")
        || raw.contains("/com.openai.")
        || raw.contains("auth.json")
        || raw.contains("credential")
        || !request.is_explicit_permission_confirmation()
    {
        return None;
    }
    let PiApprovalRequest::Confirm { id, .. } = request else {
        return None;
    };
    Some(PiApprovalResponse::Confirm { id, value: true }.to_value())
}

pub(super) fn task_status(status: PiRuntimeStatus) -> TaskStatus {
    match status {
        PiRuntimeStatus::Starting => TaskStatus::Starting,
        PiRuntimeStatus::Idle => TaskStatus::Idle,
        PiRuntimeStatus::Running => TaskStatus::Running,
        PiRuntimeStatus::Streaming => TaskStatus::Streaming,
        PiRuntimeStatus::ToolRunning => TaskStatus::ToolRunning,
        PiRuntimeStatus::NeedsApproval => TaskStatus::NeedsApproval,
        PiRuntimeStatus::NeedsInput => TaskStatus::NeedsInput,
        PiRuntimeStatus::Compacting => TaskStatus::Compacting,
        PiRuntimeStatus::Retrying => TaskStatus::Retrying,
        PiRuntimeStatus::Failed => TaskStatus::Failed,
        PiRuntimeStatus::Disconnected => TaskStatus::Disconnected,
    }
}

pub(super) fn update_task_metadata_from_poll(
    store: &mut PadStore,
    task_id: &str,
    poll: &PiPoll,
) -> Result<(), super::DesktopRuntimeError> {
    let Some(mut task) = store.get_task(task_id)? else {
        return Ok(());
    };
    let mut changed = false;
    for message in &poll.messages {
        if message.message_type != "response" {
            continue;
        }
        let Some(data) = message.value.get("data").and_then(Value::as_object) else {
            continue;
        };
        if let Some(session_file) = data
            .get("sessionFile")
            .or_else(|| data.get("session_file"))
            .and_then(Value::as_str)
        {
            let session_file = PathBuf::from(session_file);
            if let Some(profile) = store.get_profile(&task.profile_id)? {
                let root = crate::pi_runtime::profile_pi_roots(&profile).1;
                if path_within_root(&session_file, &root)
                    && task.session_file.as_ref() != Some(&session_file)
                {
                    task.session_file = Some(session_file);
                    changed = true;
                }
            }
        }
        if let Some(session_id) = data
            .get("sessionId")
            .or_else(|| data.get("session_id"))
            .and_then(Value::as_str)
        {
            if task.pi_session_id.as_deref() != Some(session_id) {
                task.pi_session_id = Some(session_id.to_string());
                changed = true;
            }
        }
        if let Some(leaf_id) = data
            .get("leafId")
            .or_else(|| data.get("leaf_id"))
            .and_then(Value::as_str)
        {
            if task.leaf_id.as_deref() != Some(leaf_id) {
                task.leaf_id = Some(leaf_id.to_string());
                changed = true;
            }
        }
    }
    if changed {
        task.updated_at = unix_timestamp();
        store.update_task(&task)?;
    }
    Ok(())
}

pub(super) fn authenticated_providers(profile: &Profile) -> Vec<String> {
    let agent_dir = crate::pi_runtime::profile_pi_roots(profile).0;
    if contains_provider_namespace(&agent_dir) {
        return Vec::new();
    }
    let path = agent_dir.join("auth.json");
    // Refuse auth.json symlink escapes from the private Profile root.
    if !path_within_root(&path, &agent_dir) {
        return Vec::new();
    }
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&content) else {
        return Vec::new();
    };
    let Some(object) = value.as_object() else {
        return Vec::new();
    };
    let mut providers = object
        .iter()
        .filter_map(|(provider, credential)| {
            if provider.starts_with('_') || credential.is_null() {
                return None;
            }
            Some(provider.clone())
        })
        .collect::<Vec<_>>();
    providers.sort_unstable();
    providers.dedup();
    providers
}

pub(super) fn provider_authentication_status(profile: &Profile) -> &'static str {
    let providers = authenticated_providers(profile);
    match profile.default_provider.as_deref() {
        Some(provider) if providers.iter().any(|value| value == provider) => "authenticated",
        Some(_) if !providers.is_empty() => "missing",
        Some(_) => "unknown",
        None if providers.is_empty() => "unknown",
        None => "authenticated",
    }
}

pub(super) fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

pub(super) fn profile_storage_segment(profile_id: &str) -> String {
    crate::pi_runtime::profile_storage_segment(profile_id)
}

pub(super) fn default_desktop_workspace_root() -> PathBuf {
    let current = std::env::current_dir().ok();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    match current {
        Some(path) if !is_unsafe_generated_project_root(&path) => path,
        _ => {
            let documents = home.join("Documents");
            if documents.is_dir() {
                documents
            } else {
                home
            }
        }
    }
}

pub(super) fn is_unsafe_generated_project_root(path: &Path) -> bool {
    path.as_os_str().is_empty() || path.parent().is_none()
}

pub(super) fn path_within_root(path: &Path, root: &Path) -> bool {
    let root = canonicalize_existing_prefix(root).unwrap_or_else(|| lexical_normalize(root));
    let path = canonicalize_existing_prefix(path).unwrap_or_else(|| lexical_normalize(path));
    path == root || path.starts_with(&root)
}

pub(super) fn contains_provider_namespace(path: &Path) -> bool {
    path.components().any(|component| {
        let std::path::Component::Normal(value) = component else {
            return false;
        };
        matches!(
            value.to_string_lossy().to_ascii_lowercase().as_str(),
            ".codex"
                | "codex"
                | ".pi"
                | ".chatgpt"
                | "chatgpt"
                | "com.openai.codex"
                | "com.openai.chatgpt"
                | "com.openai.chat"
                | "openai"
        )
    })
}

fn canonicalize_existing_prefix(path: &Path) -> Option<PathBuf> {
    let mut existing = path.to_path_buf();
    while !existing.exists() {
        if !existing.pop() {
            return None;
        }
    }
    let canonical = fs::canonicalize(&existing).ok()?;
    let remainder = path.strip_prefix(&existing).ok()?;
    Some(canonical.join(remainder))
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => {
                normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR))
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                let _ = normalized.pop();
            }
            std::path::Component::Normal(value) => normalized.push(value),
        }
    }
    normalized
}

pub(super) fn unique_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    format!("{}-{sequence}", unix_timestamp())
}
