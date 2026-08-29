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
                default: string_field(object, "default"),
            }),
            Some("editor") => Some(Self::Editor {
                id: id?,
                title: string_field(object, "title"),
                default: string_field(object, "default"),
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
                "value": value,
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PiApprovalAction {
    Ask,
    Auto(PiApprovalResponse),
}

pub(crate) fn classify_pi_approval(
    request: &PiApprovalRequest,
    policy: &RuntimePermissionPolicy,
) -> PiApprovalAction {
    match classify_approval(&request.to_policy_request(), policy) {
        ApprovalDecision::Allow | ApprovalDecision::Ask => PiApprovalAction::Ask,
        ApprovalDecision::AutoAnswer(answer) => match (request, answer) {
            (PiApprovalRequest::Confirm { id, .. }, AutoAnswer::Confirm) => {
                PiApprovalAction::Auto(PiApprovalResponse::Confirm {
                    id: id.clone(),
                    value: true,
                })
            }
            (PiApprovalRequest::Select { id, .. }, AutoAnswer::SelectDefault(index)) => {
                PiApprovalAction::Auto(PiApprovalResponse::Select {
                    id: id.clone(),
                    index,
                })
            }
            (
                PiApprovalRequest::Input { id, .. } | PiApprovalRequest::Editor { id, .. },
                AutoAnswer::InputDefault(value),
            ) => PiApprovalAction::Auto(PiApprovalResponse::Input {
                id: id.clone(),
                value,
            }),
            _ => PiApprovalAction::Ask,
        },
    }
}

/// Extract a conservative filesystem target from a Pi tool execution payload.
/// A missing/ambiguous path remains `None`, which intentionally leads to a
/// normal approval rather than granting an automatic answer.
pub(crate) fn tool_target_path(value: &Value) -> Option<PathBuf> {
    let object = value.as_object()?;
    for key in [
        "path",
        "filePath",
        "file_path",
        "cwd",
        "workdir",
        "directory",
    ] {
        if let Some(path) = object.get(key).and_then(Value::as_str) {
            let path = path.trim();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

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
            "type": "extension_ui_request", "method": "confirm", "id": "c1"
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
    }

    pub(crate) fn unattended_uses_input_default_and_select_default() {
        let input = PiApprovalRequest::parse(&json!({
            "method": "input", "id": "i1", "default": "yes"
        }))
        .unwrap();
        assert_eq!(
            classify_pi_approval(&input, &policy(PermissionMode::Unattended)),
            PiApprovalAction::Auto(PiApprovalResponse::Input {
                id: "i1".into(),
                value: "yes".into(),
            })
        );

        let select = PiApprovalRequest::parse(&json!({
            "method": "select", "id": "s1", "options": ["a", "b"], "defaultIndex": 1
        }))
        .unwrap();
        assert_eq!(
            classify_pi_approval(&select, &policy(PermissionMode::FullAccess)),
            PiApprovalAction::Auto(PiApprovalResponse::Select {
                id: "s1".into(),
                index: 1,
            })
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
    }
}
