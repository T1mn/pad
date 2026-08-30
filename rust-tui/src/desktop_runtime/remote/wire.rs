use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Result sent back through the WSS command receipt cache. Keeping the
/// projected result here guarantees retransmission never executes a command.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct RemoteCommandOutcome {
    pub(crate) ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<RemoteError>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct RemoteError {
    pub(crate) code: String,
    pub(crate) message: String,
}

impl RemoteCommandOutcome {
    pub(crate) fn rejected(code: &str, message: &str) -> Self {
        Self {
            ok: false,
            result: None,
            error: Some(RemoteError {
                code: code.to_string(),
                message: message.to_string(),
            }),
        }
    }
}

pub(crate) fn remote_action_allowed(action: &str) -> bool {
    matches!(
        action,
        "hello"
            | "bootstrap"
            | "list_sidebar"
            | "history"
            | "create_task"
            | "prompt"
            | "abort"
            | "stop"
            | "stop_task"
            | "respond_ui"
            | "set_task"
            | "runtime_snapshot"
    )
}

pub(crate) fn remote_action_mutates(action: &str) -> bool {
    matches!(
        action,
        "create_task" | "prompt" | "abort" | "stop" | "stop_task" | "respond_ui" | "set_task"
    )
}

/// An extra, stricter projection is applied after the Desktop v2 sanitizer.
/// Mobile clients receive conversation state but never machine paths,
/// credentials, provider authentication, environment, or raw diagnostics.
pub(crate) fn project_remote_result(mut value: Value) -> Value {
    redact(&mut value);
    value
}

fn redact(value: &mut Value) {
    match value {
        Value::Object(object) => {
            object.retain(|key, _| !sensitive_key(key));
            for nested in object.values_mut() {
                redact(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                redact(item);
            }
        }
        Value::String(text) if looks_like_private_path(text) => text.clear(),
        _ => {}
    }
}

fn sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "cwd"
            | "path"
            | "primary_root"
            | "additional_roots"
            | "workspace_roots"
            | "session"
            | "session_id"
            | "sessionid"
            | "session_file"
            | "session_dir"
            | "agent_dir"
            | "provider"
            | "default_provider"
            | "authentication"
            | "provider_authentication"
            | "authenticated_providers"
            | "credential_ref"
            | "token"
            | "device_token"
            | "secret"
            | "api_key"
            | "authorization"
            | "password"
            | "env"
            | "environment_variables"
            | "stderr"
            | "raw_stderr"
            | "protected_namespaces"
    ) || lower.ends_with("_path")
        || lower.ends_with("_dir")
        || lower.ends_with("_token")
        || lower.ends_with("_secret")
}

fn looks_like_private_path(value: &str) -> bool {
    let normalized = value.replace('\\', "/").to_ascii_lowercase();
    normalized.contains("/.codex")
        || normalized.contains("/.chatgpt")
        || normalized.contains("/.pi")
        || normalized.contains("/.pad")
        || normalized.contains("/library/application support/pad desktop")
        || normalized.contains("/library/application support/codex")
        || normalized.contains("/library/application support/openai")
        || normalized.contains("/library/containers/com.openai")
        || normalized.contains("/library/group containers/")
}
