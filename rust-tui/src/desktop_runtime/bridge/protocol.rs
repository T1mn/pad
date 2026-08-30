use super::{BridgeError, DesktopRequest};
use crate::desktop_runtime::{DesktopRuntime, DesktopRuntimeError};
use crate::permission_policy::{Profile, Project, Task};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub(crate) const CURRENT_PROTOCOL_VERSION: u32 = 2;
pub(crate) const LEGACY_PROTOCOL_VERSION: u32 = 1;
pub(crate) const MAX_DESKTOP_FRAME_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_DESKTOP_REQUEST_ID_BYTES: usize = 256;

pub(crate) const V2_CAPABILITIES: &[&str] = &[
    "safe_renderer_dto",
    "server_events",
    "profiles",
    "active_profile_isolation",
    "projects",
    "tasks",
    "codex_sidebar",
    "pi_rpc",
    "pi_auth_control_plane",
    "full_access_policy",
    "private_store",
    "history",
    "poll_compatibility",
    "terminal",
    "ui_state",
    "remote_gateway_v1",
    "remote_pairing",
    "remote_device_management",
    "model_catalog",
];

pub(crate) fn request_version(request: &DesktopRequest) -> u32 {
    request.protocol_version.unwrap_or(LEGACY_PROTOCOL_VERSION)
}

pub(crate) fn hello_value() -> Value {
    json!({
        "protocol": {
            "current": CURRENT_PROTOCOL_VERSION,
            "supported": [LEGACY_PROTOCOL_VERSION, CURRENT_PROTOCOL_VERSION],
            "minimum_compatible": LEGACY_PROTOCOL_VERSION,
        },
        "server": {
            "name": "pad-desktop",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "capabilities": V2_CAPABILITIES,
        "limits": {
            "max_frame_bytes": MAX_DESKTOP_FRAME_BYTES,
            "max_request_id_bytes": MAX_DESKTOP_REQUEST_ID_BYTES,
        },
    })
}

pub(crate) fn validate_v2_request(
    raw: &Value,
    request: &DesktopRequest,
) -> Result<(), BridgeError> {
    if request_version(request) != CURRENT_PROTOCOL_VERSION {
        return Err(BridgeError::new(
            "unsupported_protocol_version",
            format!(
                "protocol_version must be {} for v2 requests",
                CURRENT_PROTOCOL_VERSION
            ),
        ));
    }
    if request
        .id
        .as_ref()
        .is_some_and(|id| id.len() > MAX_DESKTOP_REQUEST_ID_BYTES)
    {
        return Err(BridgeError::new(
            "invalid_request_id",
            "request id exceeds the protocol limit",
        ));
    }
    let object = raw
        .as_object()
        .ok_or_else(|| BridgeError::new("invalid_request", "request must be a JSON object"))?;
    let action = request.action.as_deref().unwrap_or("");
    let mut allowed = vec!["id", "action", "protocol_version"];
    allowed.extend_from_slice(action_fields(action));
    let illegal: Vec<&str> = object
        .keys()
        .map(String::as_str)
        .filter(|key| !allowed.contains(key))
        .collect();
    if !illegal.is_empty() {
        return Err(BridgeError::new(
            "invalid_fields",
            format!("unsupported fields for {action}: {}", illegal.join(", ")),
        ));
    }
    Ok(())
}

fn action_fields(action: &str) -> &'static [&'static str] {
    match action {
        "hello" | "ping" | "bootstrap" | "list_sidebar" | "get_ui_state" | "shutdown"
        | "remote_status" | "remote_pair_begin" => &[],
        "create_profile" => &[
            "profile_id",
            "name",
            "default_provider",
            "default_model",
            "permission_mode",
            "unattended",
        ],
        "create_project" => &["profile_id", "name", "cwd"],
        "create_task" => &[
            "task_id",
            "project_id",
            "profile_id",
            "title",
            "summary",
            "cwd",
            "environment",
            "permission_mode",
            "unattended",
        ],
        "start_task" | "retry_task" | "poll" | "history" | "get_messages" | "get_state"
        | "abort" | "runtime_snapshot" | "stop" | "stop_task" => &["task_id"],
        "prompt" => &[
            "task_id",
            "prompt",
            "provider",
            "model",
            "model_id",
            "thinking_level",
        ],
        "get_entries" => &["task_id", "since"],
        "set_model" => &["task_id", "provider", "model", "model_id"],
        "set_thinking_level" => &["task_id", "thinking_level"],
        "respond_ui" | "extension_ui_response" => &[
            "task_id",
            "request_id",
            "interaction_id",
            "response_kind",
            "value",
            "cancelled",
        ],
        "provider_status" => &["profile_id"],
        "model_catalog" => &["profile_id", "refresh"],
        "set_task" => &["task_id", "pinned", "archived", "unread"],
        "set_profile" => &[
            "profile_id",
            "default_provider",
            "default_model",
            "permission_mode",
            "unattended",
        ],
        "auth_begin" => &["profile_id", "provider", "auth_type"],
        "auth_status" => &["attempt_id", "profile_id"],
        "auth_respond" => &["attempt_id", "prompt_id", "value", "cancelled"],
        "auth_cancel" => &["attempt_id"],
        "logout" => &["profile_id", "provider"],
        "terminal_open" => &["task_id", "pane_id", "label", "columns", "rows"],
        "terminal_input" => &["pane_id", "data"],
        "terminal_resize" => &["pane_id", "columns", "rows"],
        "terminal_snapshot" | "terminal_close" => &["pane_id"],
        "set_ui_state" => &["state"],
        "remote_set_enabled" => &["enabled"],
        "remote_pair_cancel" => &["pairing_id"],
        "remote_device_revoke" => &["device_id"],
        _ => &[],
    }
}

pub(crate) fn safe_records_value(runtime: &DesktopRuntime) -> Result<Value, BridgeError> {
    let profiles = runtime
        .store()
        .list_profiles()
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?;
    let active_profile_id = active_profile_id(runtime)?;
    let projects = runtime
        .store()
        .list_projects(true)
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?
        .into_iter()
        .filter(|project| {
            active_profile_id
                .as_deref()
                .is_some_and(|active| project.profile_id.as_deref() == Some(active))
        })
        .collect::<Vec<_>>();
    let tasks = runtime
        .store()
        .list_tasks(None, true)
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?
        .into_iter()
        .filter(|task| {
            active_profile_id
                .as_deref()
                .is_some_and(|active| task.profile_id == active)
        })
        .collect::<Vec<_>>();
    let hidden_roots = private_roots(runtime, &profiles);
    Ok(json!({
        "profiles": profiles.iter().map(|profile| safe_profile_value(runtime, profile)).collect::<Vec<_>>(),
        "projects": projects.iter().map(|project| safe_project_value(project, &hidden_roots)).collect::<Vec<_>>(),
        "tasks": tasks.iter().map(|task| safe_task_value(task, &hidden_roots)).collect::<Vec<_>>(),
    }))
}

pub(crate) fn safe_profile_value(runtime: &DesktopRuntime, profile: &Profile) -> Value {
    json!({
        "id": profile.id,
        "name": profile.name,
        "default_provider": profile.default_provider,
        "default_model": profile.default_model,
        "policy": {
            "mode": profile.policy.mode,
            "unattended": profile.policy.unattended,
        },
        "authentication": {
            "status": runtime.provider_authentication_status(profile),
            "authenticated_providers": runtime.authenticated_providers(profile),
        },
        "created_at": profile.created_at,
        "updated_at": profile.updated_at,
    })
}

pub(crate) fn safe_project_value(project: &Project, hidden_roots: &[PathBuf]) -> Value {
    let primary_root = safe_path(&project.primary_root, hidden_roots);
    let additional_roots = project
        .additional_roots
        .iter()
        .map(|root| safe_path(root, hidden_roots))
        .filter(|root| !root.is_empty())
        .collect::<Vec<_>>();
    json!({
        "id": project.id,
        "name": project.name,
        "primary_root": primary_root,
        "additional_roots": additional_roots,
        "profile_id": project.profile_id,
        "pinned": project.pinned,
        "archived": project.archived,
        "created_at": project.created_at,
        "updated_at": project.updated_at,
    })
}

pub(crate) fn safe_task_from_runtime(
    runtime: &DesktopRuntime,
    task_id: &str,
) -> Result<Option<Value>, BridgeError> {
    let profiles = runtime
        .store()
        .list_profiles()
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?;
    let task = runtime
        .store()
        .get_task(task_id)
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?;
    let active_profile_id = active_profile_id(runtime)?;
    Ok(task
        .filter(|task| active_profile_id.as_deref() == Some(task.profile_id.as_str()))
        .map(|task| safe_task_value(&task, &private_roots(runtime, &profiles))))
}

pub(crate) fn active_profile_id(runtime: &DesktopRuntime) -> Result<Option<String>, BridgeError> {
    runtime
        .desktop_ui_state()
        .map(|state| state.active_profile_id)
        .map_err(BridgeError::from)
}

fn safe_task_value(task: &Task, hidden_roots: &[PathBuf]) -> Value {
    json!({
        "id": task.id,
        "project_id": task.project_id,
        "profile_id": task.profile_id,
        "title": task.title,
        "summary": task.summary,
        "cwd": safe_path(&task.cwd, hidden_roots),
        "environment": task.environment,
        "status": task.status,
        "unread": task.unread,
        "pinned": task.pinned,
        "archived": task.archived,
        "policy": {
            "mode": task.policy.mode,
            "unattended": task.policy.unattended,
        },
        "created_at": task.created_at,
        "updated_at": task.updated_at,
    })
}

fn private_roots(runtime: &DesktopRuntime, profiles: &[Profile]) -> Vec<PathBuf> {
    private_roots_with_inputs(
        profiles,
        runtime.data_root().to_path_buf(),
        dirs::home_dir().as_deref(),
        crate::paths::base::protected_codex_home(),
    )
}

fn private_roots_with_inputs(
    profiles: &[Profile],
    pad_data_root: PathBuf,
    home: Option<&Path>,
    codex_home: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    append_private_root(&mut roots, pad_data_root);
    if let Some(home) = home {
        for namespace in crate::permission_policy::default_protected_namespaces(home) {
            append_private_root(&mut roots, namespace.root);
        }
    }
    if let Some(codex_home) = codex_home {
        append_private_root(&mut roots, codex_home);
    }
    for profile in profiles {
        append_private_root(&mut roots, profile.agent_dir.clone());
        append_private_root(&mut roots, profile.session_dir.clone());
    }
    roots
}

fn append_private_root(roots: &mut Vec<PathBuf>, root: PathBuf) {
    if !root.as_os_str().is_empty() && !roots.contains(&root) {
        roots.push(root);
    }
}

#[cfg(test)]
pub(super) fn private_roots_with_inputs_for_test(
    profiles: &[Profile],
    pad_data_root: PathBuf,
    home: Option<&Path>,
    codex_home: Option<PathBuf>,
) -> Vec<PathBuf> {
    private_roots_with_inputs(profiles, pad_data_root, home, codex_home)
}

fn safe_path(path: &Path, hidden_roots: &[PathBuf]) -> String {
    if path.as_os_str().is_empty() || path_is_hidden(path, hidden_roots) {
        return String::new();
    }
    path.to_string_lossy().into_owned()
}

fn path_is_hidden(path: &Path, roots: &[PathBuf]) -> bool {
    if roots
        .iter()
        .any(|root| !root.as_os_str().is_empty() && path.starts_with(root))
    {
        return true;
    }
    let canonical = path.canonicalize().ok();
    roots.iter().any(|root| {
        canonical.as_ref().is_some_and(|path| {
            root.canonicalize()
                .ok()
                .is_some_and(|root| path.starts_with(root))
        })
    })
}

pub(crate) fn event_frame(sequence: u64, kind: &str, payload: Value) -> Value {
    json!({
        "type": "desktop_event",
        "protocol_version": CURRENT_PROTOCOL_VERSION,
        "sequence": sequence,
        "event": {
            "kind": kind,
            "payload": payload,
        },
    })
}

pub(crate) fn sanitize_v2_result(
    runtime: &DesktopRuntime,
    result: &mut Value,
) -> Result<(), BridgeError> {
    let profiles = runtime
        .store()
        .list_profiles()
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?;
    let records = safe_records_value(runtime)?;
    if let Some(object) = result.as_object_mut() {
        if object.contains_key("protocol_version") {
            object.insert(
                "protocol_version".to_string(),
                Value::from(CURRENT_PROTOCOL_VERSION),
            );
            object.insert("protocol".to_string(), hello_value()["protocol"].clone());
        }
        if object.contains_key("capabilities") {
            object.insert("capabilities".to_string(), json!(V2_CAPABILITIES));
        }
        for (key, collection) in [
            ("profile", "profiles"),
            ("project", "projects"),
            ("task", "tasks"),
        ] {
            if let Some(id) = object
                .get(key)
                .and_then(|record| record.get("id"))
                .and_then(Value::as_str)
            {
                if let Some(safe) =
                    records
                        .get(collection)
                        .and_then(Value::as_array)
                        .and_then(|items| {
                            items
                                .iter()
                                .find(|item| item.get("id").and_then(Value::as_str) == Some(id))
                        })
                {
                    object.insert(key.to_string(), safe.clone());
                }
            }
        }
        if object.contains_key("records") {
            object.insert("records".to_string(), records);
        }
        if let Some(profile_id) = object
            .get("profile")
            .and_then(|profile| profile.get("id"))
            .and_then(Value::as_str)
        {
            if let Some(profile) = profiles.iter().find(|profile| profile.id == profile_id) {
                object.insert("profile".to_string(), safe_profile_value(runtime, profile));
            }
        }
    }
    redact_private_data(result, &private_roots(runtime, &profiles));
    Ok(())
}

pub(crate) fn sanitize_v2_error_message(runtime: &DesktopRuntime, message: &str) -> String {
    let profiles = runtime.store().list_profiles().unwrap_or_default();
    redact_sensitive_text(message, &private_roots(runtime, &profiles))
}

fn redact_private_data(value: &mut Value, roots: &[PathBuf]) {
    match value {
        Value::Object(object) => {
            for key in [
                "agent_dir",
                "session_dir",
                "credential_ref",
                "pi_session_id",
                "session_id",
                "sessionId",
                "session_file",
                "auth_path",
                "api_key",
                "token",
                "credential",
                "credentials",
                "authorization",
                "password",
                "secret",
                "access_token",
                "refresh_token",
                "environment_variables",
                "env",
                "protected_namespaces",
                "workspace_roots",
            ] {
                object.remove(key);
            }
            for nested in object.values_mut() {
                redact_private_data(nested, roots);
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_private_data(item, roots);
            }
        }
        Value::String(string) => {
            *string = redact_sensitive_text(string, roots);
        }
        _ => {}
    }
}

fn redact_sensitive_text(value: &str, roots: &[PathBuf]) -> String {
    let mut safe = value.to_string();
    for root in roots {
        let root = root.to_string_lossy();
        if !root.is_empty() {
            safe = redact_path_token(&safe, root.as_ref());
        }
    }
    for marker in [
        "PI_CODING_AGENT_DIR",
        "PI_CODING_AGENT_SESSION_DIR",
        "PI_SESSION_FILE",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "GEMINI_API_KEY",
        "GOOGLE_API_KEY",
        "XAI_API_KEY",
        "OPENROUTER_API_KEY",
        "GITHUB_TOKEN",
        "CODEX_HOME",
    ] {
        safe = redact_assignment(&safe, marker);
    }
    // These namespace fragments are stable product boundaries, not values
    // that depend on the current process environment.  Redact them even when
    // a test/dev data root is active so archived tool output cannot reveal a
    // production PAD, Codex, or ChatGPT storage location.
    for marker in [
        "/Library/Application Support/PAD Desktop",
        "\\Library\\Application Support\\PAD Desktop",
        "/.codex",
        "\\.codex",
        "/.pi",
        "\\.pi",
        "/.pad",
        "\\.pad",
        "/.chatgpt",
        "\\.chatgpt",
        "/Library/Application Support/com.openai.codex",
        "\\Library\\Application Support\\com.openai.codex",
        "/Library/Application Support/Codex",
        "\\Library\\Application Support\\Codex",
        "/Library/Application Support/OpenAI",
        "\\Library\\Application Support\\OpenAI",
        "/Library/Application Support/com.openai.chat",
        "\\Library\\Application Support\\com.openai.chat",
        "/Library/Application Support/com.openai.chatgpt",
        "\\Library\\Application Support\\com.openai.chatgpt",
        "/Library/Application Support/ChatGPT",
        "\\Library\\Application Support\\ChatGPT",
        "/Library/Group Containers/group.com.openai.codex",
        "\\Library\\Group Containers\\group.com.openai.codex",
        "/Library/Group Containers/group.com.openai.chat",
        "\\Library\\Group Containers\\group.com.openai.chat",
        "/Library/Group Containers/group.com.openai.chatgpt",
        "\\Library\\Group Containers\\group.com.openai.chatgpt",
        "/Library/Group Containers/2DC432GLL2.com.openai.codex.notifications",
        "\\Library\\Group Containers\\2DC432GLL2.com.openai.codex.notifications",
        "/Library/Group Containers/2DC432GLL2.com.openai.sky.CUAService",
        "\\Library\\Group Containers\\2DC432GLL2.com.openai.sky.CUAService",
        "/Library/Containers/com.openai.codex",
        "\\Library\\Containers\\com.openai.codex",
        "/Library/Containers/com.openai.chat",
        "\\Library\\Containers\\com.openai.chat",
        "/Library/Containers/com.openai.chatgpt",
        "\\Library\\Containers\\com.openai.chatgpt",
        "/Library/Caches/Codex",
        "\\Library\\Caches\\Codex",
        "/Library/Caches/com.openai.codex",
        "\\Library\\Caches\\com.openai.codex",
        "/Library/Caches/ChatGPT",
        "\\Library\\Caches\\ChatGPT",
        "/Library/Caches/com.openai.chat",
        "\\Library\\Caches\\com.openai.chat",
        "/Library/Caches/com.openai.chatgpt",
        "\\Library\\Caches\\com.openai.chatgpt",
        "/Library/Logs/com.openai.codex",
        "\\Library\\Logs\\com.openai.codex",
        "/Library/Logs/com.openai.chat",
        "\\Library\\Logs\\com.openai.chat",
        "/Library/Logs/com.openai.chatgpt",
        "\\Library\\Logs\\com.openai.chatgpt",
        "/Library/HTTPStorages/com.openai.codex",
        "\\Library\\HTTPStorages\\com.openai.codex",
        "/Library/HTTPStorages/com.openai.chat",
        "\\Library\\HTTPStorages\\com.openai.chat",
        "/Library/HTTPStorages/com.openai.chatgpt",
        "\\Library\\HTTPStorages\\com.openai.chatgpt",
        "/Library/Preferences/com.openai.codex.plist",
        "\\Library\\Preferences\\com.openai.codex.plist",
        "/Library/Preferences/com.openai.chat.plist",
        "\\Library\\Preferences\\com.openai.chat.plist",
        "/Library/Preferences/com.openai.chatgpt.plist",
        "\\Library\\Preferences\\com.openai.chatgpt.plist",
    ] {
        safe = redact_path_token(&safe, marker);
    }
    for json_key in [
        "\"access_token\"",
        "\"refresh_token\"",
        "\"api_key\"",
        "\"credential_ref\"",
        "\"token\"",
        "\"credential\"",
        "\"credentials\"",
        "\"authorization\"",
        "\"password\"",
        "\"secret\"",
        "\"access\"",
        "\"refresh\"",
    ] {
        safe = redact_assignment(&safe, json_key);
    }
    safe
}

fn redact_path_token(value: &str, marker: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut remaining = value;
    while let Some(index) = remaining.find(marker) {
        let prefix = &remaining[..index];
        let token_start = prefix
            .rfind(is_path_boundary)
            .map(|boundary| {
                boundary
                    + prefix[boundary..]
                        .chars()
                        .next()
                        .map(char::len_utf8)
                        .unwrap_or(0)
            })
            .unwrap_or(0);
        result.push_str(&prefix[..token_start]);
        result.push_str("[private]");
        let after_marker = &remaining[index + marker.len()..];
        let token_end = after_marker
            .find(is_path_boundary)
            .unwrap_or(after_marker.len());
        remaining = &after_marker[token_end..];
    }
    result.push_str(remaining);
    result
}

fn is_path_boundary(character: char) -> bool {
    character.is_whitespace()
        || matches!(
            character,
            '"' | '\'' | '`' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | '='
        )
}

fn redact_assignment(value: &str, marker: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut remaining = value;
    while let Some(index) = remaining.find(marker) {
        result.push_str(&remaining[..index]);
        result.push_str("[private]");
        let after_marker = &remaining[index + marker.len()..];
        let trimmed = after_marker.trim_start();
        let skipped_whitespace = after_marker.len() - trimmed.len();
        let Some(delimiter) = trimmed.chars().next() else {
            remaining = "";
            break;
        };
        if !matches!(delimiter, '=' | ':') {
            remaining = after_marker;
            continue;
        }
        let value_start = skipped_whitespace + delimiter.len_utf8();
        let tail = after_marker[value_start..].trim_start();
        let leading = after_marker[value_start..].len() - tail.len();
        let quoted = tail.starts_with('"') || tail.starts_with('\'');
        let quote = tail.chars().next().unwrap_or_default();
        let consumed = if quoted {
            tail[quote.len_utf8()..]
                .find(quote)
                .map(|end| quote.len_utf8() + end + quote.len_utf8())
                .unwrap_or(tail.len())
        } else {
            tail.find(|character: char| {
                character.is_whitespace() || matches!(character, ',' | '}' | ']')
            })
            .unwrap_or(tail.len())
        };
        result.push_str("=[redacted]");
        remaining = &after_marker[value_start + leading + consumed..];
    }
    result.push_str(remaining);
    result
}
