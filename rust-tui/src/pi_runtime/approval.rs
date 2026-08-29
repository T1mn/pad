use crate::permission_policy::{OperationKind, PolicyOperation};
use crate::relay::permissions::policy::{
    classify_approval, ApprovalDecision, ApprovalOperation, ApprovalRequest, AutoAnswer,
    RuntimePermissionPolicy,
};
use serde_json::{Map, Value};
use std::path::PathBuf;

/// The subset of Pi extension UI requests that can be handled by a desktop
/// transport without embedding Pi's TUI.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PiApprovalRequest {
    Confirm {
        id: String,
        title: Option<String>,
        message: Option<String>,
    },
    Select {
        id: String,
        title: Option<String>,
        options: Vec<String>,
        default_index: Option<usize>,
    },
    Input {
        id: String,
        title: Option<String>,
        default: Option<String>,
    },
    Editor {
        id: String,
        title: Option<String>,
        default: Option<String>,
    },
    Unknown {
        id: Option<String>,
        method: Option<String>,
    },
}

impl PiApprovalRequest {
    pub(crate) fn parse(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        let method = object
            .get("method")
            .or_else(|| object.get("requestType"))
            .and_then(Value::as_str)
            .map(str::to_ascii_lowercase);
        let id = object
            .get("id")
            .or_else(|| object.get("requestId"))
            .and_then(Value::as_str)
            .map(str::to_string);

        match method.as_deref() {
            Some("confirm") => Some(Self::Confirm {
                id: id?,
                title: string_field(object, "title"),
                message: string_field(object, "message").or_else(|| string_field(object, "text")),
            }),
            Some("select") => {
                let options = object
                    .get("options")
                    .and_then(Value::as_array)
                    .map(|options| {
                        options
                            .iter()
                            .filter_map(|option| {
                                option.as_str().map(str::to_string).or_else(|| {
                                    option
                                        .get("label")
                                        .and_then(Value::as_str)
                                        .map(str::to_string)
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                Some(Self::Select {
                    id: id?,
                    title: string_field(object, "title"),
                    options,
                    default_index: object
                        .get("defaultIndex")
                        .or_else(|| object.get("default_index"))
                        .and_then(Value::as_u64)
                        .map(|index| index as usize),
                })
            }
            Some("input") => Some(Self::Input {
                id: id?,
                title: string_field(object, "title"),
                default: string_field(object, "default")
                    .or_else(|| string_field(object, "placeholder")),
            }),
            Some("editor") => Some(Self::Editor {
                id: id?,
                title: string_field(object, "title"),
                default: string_field(object, "default")
                    .or_else(|| string_field(object, "prefill")),
            }),
            _ => Some(Self::Unknown { id, method }),
        }
    }

    pub(crate) fn id(&self) -> Option<&str> {
        match self {
            Self::Confirm { id, .. }
            | Self::Select { id, .. }
            | Self::Input { id, .. }
            | Self::Editor { id, .. } => Some(id),
            Self::Unknown { id, .. } => id.as_deref(),
        }
    }

    /// Return whether this request is an explicit permission gate rather than
    /// a business/workflow question.  Pi extensions use the same `confirm`
    /// primitive for both kinds of dialog, so Full Access must require a
    /// permission-specific phrase before answering it automatically.
    ///
    /// This intentionally errs on the side of asking.  A bare "Continue?",
    /// project-trust dialog, or any select/input/editor request remains
    /// visible to the Desktop UI even when the profile is unattended.
    pub(crate) fn is_explicit_permission_confirmation(&self) -> bool {
        let Self::Confirm { title, message, .. } = self else {
            return false;
        };
        let text = format!(
            "{} {}",
            title.as_deref().unwrap_or_default(),
            message.as_deref().unwrap_or_default()
        )
        .to_ascii_lowercase();
        if text.trim().is_empty()
            || text.contains("trust")
            || text.contains("project")
            || text.contains("信任")
            || text.contains("项目")
        {
            return false;
        }
        [
            "permission",
            "allow",
            "approve",
            "authorize",
            "authorise",
            "permit",
            "consent",
            "approval",
            "grant",
            "access",
            // Pi extensions can localize their dialog copy. Keep the same
            // conservative semantics for Chinese permission prompts.
            "权限",
            "允许",
            "批准",
            "授权",
            "许可",
            "同意",
        ]
        .iter()
        .any(|marker| text.contains(marker))
    }

    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "legacy relay-policy conversion remains test-covered while Desktop uses permission_policy"
        )
    )]
    pub(crate) fn to_policy_request(&self) -> ApprovalRequest {
        match self {
            Self::Confirm { .. } => ApprovalRequest::ui(ApprovalOperation::Confirm, None, None, 0),
            Self::Select {
                options,
                default_index,
                ..
            } => ApprovalRequest::ui(
                ApprovalOperation::Select,
                None,
                *default_index,
                options.len(),
            ),
            Self::Input { default, .. } => {
                ApprovalRequest::ui(ApprovalOperation::Input, default.clone(), None, 0)
            }
            Self::Editor { default, .. } => {
                ApprovalRequest::ui(ApprovalOperation::Editor, default.clone(), None, 0)
            }
            Self::Unknown { .. } => ApprovalRequest::tool(ApprovalOperation::Unknown, None),
        }
    }
}

#[allow(
    dead_code,
    reason = "select and input responses remain part of the Pi extension UI transport contract"
)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PiApprovalResponse {
    Confirm { id: String, value: bool },
    Select { id: String, index: usize },
    Input { id: String, value: String },
}

impl PiApprovalResponse {
    /// Encode the extension UI response shape used by Pi's RPC transport.
    /// Keeping this centralized makes response changes auditable and lets the
    /// caller attach the original request id for correlation.
    pub(crate) fn to_value(&self) -> Value {
        match self {
            Self::Confirm { id, value } => serde_json::json!({
                "type": "extension_ui_response",
                "id": id,
                "confirmed": value,
            }),
            Self::Select { id, index } => serde_json::json!({
                "type": "extension_ui_response",
                "id": id,
                "value": index,
            }),
            Self::Input { id, value } => serde_json::json!({
                "type": "extension_ui_response",
                "id": id,
                "value": value,
            }),
        }
    }
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "legacy relay-policy approval action remains covered by compatibility tests"
    )
)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PiApprovalAction {
    Ask,
    Auto(PiApprovalResponse),
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "legacy relay-policy classifier remains covered while Desktop uses permission_policy"
    )
)]
pub(crate) fn classify_pi_approval(
    request: &PiApprovalRequest,
    policy: &RuntimePermissionPolicy,
) -> PiApprovalAction {
    // `confirm` is overloaded by Pi extensions.  Only a dialog whose copy
    // explicitly describes permission may be auto-confirmed.  Never infer a
    // user's answer to a select/input/editor business prompt from a default.
    if !request.is_explicit_permission_confirmation() {
        return PiApprovalAction::Ask;
    }
    match classify_approval(&request.to_policy_request(), policy) {
        ApprovalDecision::Allow | ApprovalDecision::Ask => PiApprovalAction::Ask,
        ApprovalDecision::AutoAnswer(answer) => match (request, answer) {
            (PiApprovalRequest::Confirm { id, .. }, AutoAnswer::Confirm) => {
                PiApprovalAction::Auto(PiApprovalResponse::Confirm {
                    id: id.clone(),
                    value: true,
                })
            }
            _ => PiApprovalAction::Ask,
        },
    }
}

/// Extract a conservative filesystem target from a Pi tool execution payload.
/// A missing/ambiguous path remains `None`, which intentionally leads to a
/// normal approval rather than granting an automatic answer.
pub(crate) fn tool_target_path(value: &Value) -> Option<PathBuf> {
    for object in structured_objects(value) {
        for key in [
            "path",
            "filePath",
            "file_path",
            "cwd",
            "workdir",
            "directory",
            "target",
        ] {
            if let Some(path) = object.get(key).and_then(Value::as_str) {
                let path = path.trim();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }
    }
    None
}

/// Convert a structured Pi permission request into the shared Desktop policy
/// operation.  This adapter classifies transport fields only; all allow,
/// prompt and deny decisions remain in `permission_policy::evaluate_operation`.
pub(crate) fn pi_policy_operation(value: &Value) -> PolicyOperation {
    let name = structured_string(value, &["toolName", "tool_name", "name", "tool"])
        .unwrap_or_default()
        .to_ascii_lowercase();
    let command = structured_string(value, &["command", "cmd", "detail"]);
    let descriptor = format!(
        "{name} {}",
        command.as_deref().unwrap_or_default().to_ascii_lowercase()
    );
    let kind = if contains_any(
        &descriptor,
        &[
            "credential",
            "keychain",
            "api_key",
            "api-key",
            "auth.json",
            "find-generic-password",
            "add-generic-password",
            "token",
            "login",
        ],
    ) {
        OperationKind::Credential
    } else if contains_any(&name, &["delete", "remove", "unlink", "rmdir"]) || name == "rm" {
        OperationKind::Delete
    } else if contains_any(&name, &["install", "package"]) {
        OperationKind::Install
    } else if contains_any(&name, &["kill", "process", "terminate", "stop"]) {
        OperationKind::ProcessControl
    } else if contains_any(&name, &["http", "network", "fetch", "download", "upload"]) {
        OperationKind::Network
    } else if contains_any(&name, &["read", "grep", "search", "find"]) || name == "ls" {
        OperationKind::Read
    } else if contains_any(&name, &["write", "edit", "patch", "create", "move", "copy"]) {
        OperationKind::Write
    } else {
        // A permission-specific confirm with no tool metadata is treated as
        // execution with an unknown/external scope. System Full may allow it;
        // Workspace Full must receive a structured in-workspace path.
        OperationKind::Execute
    };
    PolicyOperation {
        kind,
        path: tool_target_path(value),
        command,
    }
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "legacy relay-policy tool mapping remains covered by compatibility tests"
    )
)]
pub(crate) fn tool_operation(value: &Value) -> ApprovalOperation {
    let name = value
        .get("toolName")
        .or_else(|| value.get("tool_name"))
        .or_else(|| value.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if name.contains("read") || name == "grep" || name == "find" || name == "ls" {
        ApprovalOperation::Read
    } else if name.contains("write")
        || name.contains("edit")
        || name.contains("patch")
        || name.contains("delete")
    {
        ApprovalOperation::Write
    } else if name.contains("bash")
        || name.contains("shell")
        || name.contains("command")
        || name == "run"
    {
        ApprovalOperation::Shell
    } else if name.contains("http") || name.contains("network") || name.contains("fetch") {
        ApprovalOperation::Network
    } else {
        ApprovalOperation::Unknown
    }
}

fn structured_objects(value: &Value) -> Vec<&Map<String, Value>> {
    let Some(root) = value.as_object() else {
        return Vec::new();
    };
    let mut objects = vec![root];
    for key in ["details", "tool", "request", "payload", "args", "input"] {
        if let Some(object) = root.get(key).and_then(Value::as_object) {
            objects.push(object);
        }
    }
    objects
}

fn structured_string(value: &Value, keys: &[&str]) -> Option<String> {
    for object in structured_objects(value) {
        for key in keys {
            if let Some(value) = object
                .get(*key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn string_field(object: &Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::relay::permissions::policy::{PermissionMode, ProtectedNamespace};
    use serde_json::json;

    fn policy(mode: PermissionMode) -> RuntimePermissionPolicy {
        RuntimePermissionPolicy::new(mode, [PathBuf::from("/workspace")])
            .with_protected_namespace(ProtectedNamespace::new([PathBuf::from("/protected")]))
    }

    pub(crate) fn full_access_auto_answers_confirm_but_not_unknown_ui() {
        let confirm = PiApprovalRequest::parse(&json!({
            "type": "extension_ui_request", "method": "confirm", "id": "c1",
            "title": "Allow tool execution?", "message": "Permit this command?"
        }))
        .unwrap();
        assert_eq!(confirm.id(), Some("c1"));
        assert_eq!(
            classify_pi_approval(&confirm, &policy(PermissionMode::FullAccess)),
            PiApprovalAction::Auto(PiApprovalResponse::Confirm {
                id: "c1".into(),
                value: true,
            })
        );

        let unknown = PiApprovalRequest::parse(&json!({
            "type": "extension_ui_request", "method": "pick_color", "id": "u1"
        }))
        .unwrap();
        assert_eq!(
            classify_pi_approval(&unknown, &policy(PermissionMode::Unattended)),
            PiApprovalAction::Ask
        );

        let generic_confirm = PiApprovalRequest::parse(&json!({
            "type": "extension_ui_request", "method": "confirm", "id": "c2",
            "title": "Continue?", "message": "Proceed with this workflow?"
        }))
        .unwrap();
        assert_eq!(
            classify_pi_approval(&generic_confirm, &policy(PermissionMode::FullAccess)),
            PiApprovalAction::Ask
        );
    }

    pub(crate) fn unattended_uses_input_default_and_select_default() {
        let input = PiApprovalRequest::parse(&json!({
            "method": "input", "id": "i1", "default": "yes"
        }))
        .unwrap();
        assert_eq!(
            classify_pi_approval(&input, &policy(PermissionMode::Unattended)),
            PiApprovalAction::Ask
        );

        let select = PiApprovalRequest::parse(&json!({
            "method": "select", "id": "s1", "options": ["a", "b"], "defaultIndex": 1
        }))
        .unwrap();
        assert_eq!(
            classify_pi_approval(&select, &policy(PermissionMode::FullAccess)),
            PiApprovalAction::Ask
        );
    }

    pub(crate) fn protected_paths_never_get_auto_answers() {
        let request = ApprovalRequest::tool(
            ApprovalOperation::Write,
            Some(PathBuf::from("/protected/session.jsonl")),
        );
        assert_eq!(
            classify_approval(&request, &policy(PermissionMode::Unattended)),
            ApprovalDecision::Ask
        );

        let home = dirs::home_dir().expect("test host has a home directory");
        let standard = RuntimePermissionPolicy::default();
        for relative in [
            "Library/Application Support/Codex/Session Storage/state",
            "Library/Application Support/OpenAI/Codex/session.jsonl",
            "Library/Group Containers/2DC432GLL2.com.openai.codex.notifications/state",
            "Library/Group Containers/2DC432GLL2.com.openai.sky.CUAService/state",
            "Library/Caches/com.openai.codex/index",
            "Library/Preferences/com.openai.codex.plist",
        ] {
            let target = home.join(relative);
            assert!(
                !standard.allows_path(&target),
                "standard policy allowed Codex state: {}",
                target.display()
            );
        }
    }

    pub(crate) fn tool_operation_and_target_are_conservative() {
        let payload = json!({"toolName":"edit", "file_path":"/workspace/a.txt"});
        assert_eq!(tool_operation(&payload), ApprovalOperation::Write);
        assert_eq!(
            tool_target_path(&payload),
            Some(PathBuf::from("/workspace/a.txt"))
        );
        assert_eq!(
            tool_operation(&json!({"toolName":"something"})),
            ApprovalOperation::Unknown
        );

        let nested = json!({
            "method": "confirm",
            "payload": {
                "toolName": "delete_file",
                "path": "/workspace/old.txt",
                "command": "rm old.txt"
            }
        });
        assert_eq!(
            pi_policy_operation(&nested),
            PolicyOperation {
                kind: OperationKind::Delete,
                path: Some(PathBuf::from("/workspace/old.txt")),
                command: Some("rm old.txt".into()),
            }
        );
        assert_eq!(
            pi_policy_operation(&json!({"toolName":"credential_store"})).kind,
            OperationKind::Credential
        );
        assert_eq!(
            pi_policy_operation(&json!({
                "toolName":"bash",
                "command":"security find-generic-password -s provider"
            }))
            .kind,
            OperationKind::Credential
        );
    }
}
