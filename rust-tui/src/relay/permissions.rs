mod claude {
    use super::super::common::{
        claude_permission_state_path, claude_settings_path, log_file_error, parse_json_object,
        read_json_value, serialize_json_pretty, write_json_value, write_text_file,
    };
    use super::json_helpers::{
        cleanup_empty_json_objects, json_bool_at_path, json_string_at_path, restore_json_bool_path,
        restore_json_string_path, set_json_bool_path, set_json_string_path,
    };
    use serde_json::json;

    pub(super) fn apply_claude_permission_overlay() {
        let path = claude_settings_path();
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let mut obj = parse_json_object(&content);

        capture_claude_permission_state_once(&obj);
        set_json_string_path(
            &mut obj,
            &["permissions", "defaultMode"],
            "bypassPermissions",
        );
        set_json_bool_path(&mut obj, &["sandbox", "enabled"], false);

        if let Err(error) = write_text_file(&path, &serialize_json_pretty(&obj)) {
            log_file_error("write", &path, &error);
        }
    }

    pub(super) fn remove_claude_permission_overlay() {
        let path = claude_settings_path();
        let state_path = claude_permission_state_path();
        if !path.exists() && !state_path.exists() {
            return;
        }
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let mut obj = parse_json_object(&content);
        let state = read_json_value(
            &state_path,
            json!({
                "permissions_default_mode": null,
                "sandbox_enabled": null
            }),
        );

        restore_json_string_path(
            &mut obj,
            &["permissions", "defaultMode"],
            state.get("permissions_default_mode"),
        );
        restore_json_bool_path(
            &mut obj,
            &["sandbox", "enabled"],
            state.get("sandbox_enabled"),
        );
        cleanup_empty_json_objects(&mut obj);

        if let Err(error) = write_text_file(&path, &serialize_json_pretty(&obj)) {
            log_file_error("write", &path, &error);
        }
        let _ = std::fs::remove_file(state_path);
    }

    fn capture_claude_permission_state_once(obj: &serde_json::Value) {
        let path = claude_permission_state_path();
        if path.exists() {
            return;
        }

        let value = json!({
            "permissions_default_mode": json_string_at_path(obj, &["permissions", "defaultMode"]),
            "sandbox_enabled": json_bool_at_path(obj, &["sandbox", "enabled"]),
        });
        if let Err(error) = write_json_value(&path, &value) {
            log_file_error("write", &path, &error);
        }
    }
}

/// Policy primitives shared by the native relay overlays and the Pi RPC
/// adapter.  The existing provider-specific booleans remain the compatibility
/// path; these types add a single, testable decision point for new desktop
/// runtimes without changing the persisted theme schema.
#[allow(
    dead_code,
    reason = "the relay permission model is a compatibility boundary retained while Desktop uses permission_policy"
)]
pub(crate) mod policy {
    use std::fs;
    use std::path::{Component, Path, PathBuf};

    /// The amount of automation PAD may grant to an agent.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub(crate) enum PermissionMode {
        /// Ask PAD for an explicit answer for every approval request.
        #[default]
        Prompt,
        /// Automatically approve deterministic operations inside the project
        /// roots, while asking for out-of-workspace or ambiguous operations.
        Workspace,
        /// Automatically approve deterministic operations outside the project
        /// roots too. Protected namespaces are still never auto-approved.
        FullAccess,
        /// Non-interactive execution. Safe default answers may be supplied to
        /// UI requests, but protected and ambiguous requests still stop.
        Unattended,
    }

    impl PermissionMode {
        pub(crate) fn enables_provider_overlay(self) -> bool {
            matches!(self, Self::FullAccess | Self::Unattended)
        }

        pub(crate) fn is_unattended(self) -> bool {
            self == Self::Unattended
        }
    }

    /// Namespaces that remain outside Full Access. Callers may construct this
    /// with the exact paths that must not be touched; no caller-side string
    /// prefix matching is required.
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    pub(crate) struct ProtectedNamespace {
        roots: Vec<PathBuf>,
    }

    impl ProtectedNamespace {
        pub(crate) fn new(roots: impl IntoIterator<Item = PathBuf>) -> Self {
            let mut normalized = roots
                .into_iter()
                .map(|root| normalize_for_comparison(&root))
                .collect::<Vec<_>>();
            normalized.sort();
            normalized.dedup();
            Self { roots: normalized }
        }

        /// The standard external stores whose state must not be rewritten by
        /// PAD: the user's canonical Codex/Pi homes and common ChatGPT app
        /// containers. PAD's own `~/.pad` namespace is intentionally omitted.
        pub(crate) fn standard() -> Self {
            let Some(home) = dirs::home_dir() else {
                return Self::default();
            };

            Self::new([
                home.join(".codex"),
                home.join(".pi"),
                home.join(".chatgpt"),
                home.join("Library/Application Support/Codex"),
                home.join("Library/Application Support/OpenAI"),
                home.join("Library/Application Support/com.openai.codex"),
                home.join("Library/Application Support/com.openai.chat"),
                home.join("Library/Application Support/com.openai.chatgpt"),
                home.join("Library/Application Support/ChatGPT"),
                home.join("Library/Containers/com.openai.codex"),
                home.join("Library/Containers/com.openai.chat"),
                home.join("Library/Containers/com.openai.chatgpt"),
                home.join("Library/Group Containers/group.com.openai.codex"),
                home.join("Library/Group Containers/group.com.openai.chat"),
                home.join("Library/Group Containers/group.com.openai.chatgpt"),
                home.join("Library/Group Containers/2DC432GLL2.com.openai.codex.notifications"),
                home.join("Library/Group Containers/2DC432GLL2.com.openai.sky.CUAService"),
                home.join("Library/Caches/Codex"),
                home.join("Library/Caches/com.openai.codex"),
                home.join("Library/Caches/ChatGPT"),
                home.join("Library/Caches/com.openai.chat"),
                home.join("Library/Caches/com.openai.chatgpt"),
                home.join("Library/Logs/com.openai.codex"),
                home.join("Library/Logs/com.openai.chat"),
                home.join("Library/Logs/com.openai.chatgpt"),
                home.join("Library/HTTPStorages/com.openai.codex"),
                home.join("Library/HTTPStorages/com.openai.codex.binarycookies"),
                home.join("Library/HTTPStorages/com.openai.chat"),
                home.join("Library/HTTPStorages/com.openai.chat.binarycookies"),
                home.join("Library/HTTPStorages/com.openai.chatgpt"),
                home.join("Library/HTTPStorages/com.openai.chatgpt.binarycookies"),
                home.join("Library/Preferences/com.openai.codex.plist"),
                home.join("Library/Preferences/com.openai.chat.plist"),
                home.join("Library/Preferences/com.openai.chatgpt.plist"),
            ])
        }

        pub(crate) fn roots(&self) -> &[PathBuf] {
            &self.roots
        }

        /// Returns true for the root itself and all descendants. Existing
        /// symlinks are resolved conservatively before comparison, preventing
        /// a symlinked workspace path from escaping this boundary.
        pub(crate) fn contains(&self, path: &Path) -> bool {
            let candidate = resolve_for_comparison(path);
            self.roots.iter().any(|root| {
                let root = resolve_for_comparison(root);
                candidate == root || candidate.starts_with(&root)
            })
        }
    }

    /// A request presented by either a native provider or Pi's extension UI.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct ApprovalRequest {
        pub(crate) operation: ApprovalOperation,
        pub(crate) target: Option<PathBuf>,
        pub(crate) cwd: Option<PathBuf>,
        pub(crate) default_answer: Option<String>,
        pub(crate) default_index: Option<usize>,
        pub(crate) choice_count: usize,
    }

    impl ApprovalRequest {
        pub(crate) fn tool(operation: ApprovalOperation, target: Option<PathBuf>) -> Self {
            Self {
                operation,
                target,
                cwd: None,
                default_answer: None,
                default_index: None,
                choice_count: 0,
            }
        }

        pub(crate) fn ui(
            operation: ApprovalOperation,
            default_answer: Option<String>,
            default_index: Option<usize>,
            choice_count: usize,
        ) -> Self {
            Self {
                operation,
                target: None,
                cwd: None,
                default_answer,
                default_index,
                choice_count,
            }
        }

        pub(crate) fn with_cwd(mut self, cwd: Option<PathBuf>) -> Self {
            self.cwd = cwd;
            self
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) enum ApprovalOperation {
        Read,
        Write,
        Shell,
        Network,
        ProjectTrust,
        Confirm,
        Select,
        Input,
        Editor,
        Unknown,
    }

    /// The result is deliberately small so UI and RPC transports can map it
    /// to their own response envelope without sharing provider-specific types.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) enum ApprovalDecision {
        Allow,
        Ask,
        AutoAnswer(AutoAnswer),
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) enum AutoAnswer {
        Confirm,
        SelectDefault(usize),
        InputDefault(String),
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct RuntimePermissionPolicy {
        pub(crate) mode: PermissionMode,
        pub(crate) workspace_roots: Vec<PathBuf>,
        pub(crate) protected_namespace: ProtectedNamespace,
    }

    impl Default for RuntimePermissionPolicy {
        fn default() -> Self {
            Self {
                mode: PermissionMode::Prompt,
                workspace_roots: Vec::new(),
                protected_namespace: ProtectedNamespace::standard(),
            }
        }
    }

    impl RuntimePermissionPolicy {
        pub(crate) fn new(
            mode: PermissionMode,
            workspace_roots: impl IntoIterator<Item = PathBuf>,
        ) -> Self {
            Self {
                mode,
                workspace_roots: workspace_roots
                    .into_iter()
                    .map(|root| normalize_for_comparison(&root))
                    .collect(),
                protected_namespace: ProtectedNamespace::standard(),
            }
        }

        pub(crate) fn with_protected_namespace(mut self, namespace: ProtectedNamespace) -> Self {
            self.protected_namespace = namespace;
            self
        }

        pub(crate) fn legacy_full_access() -> Self {
            Self::new(PermissionMode::FullAccess, [])
        }

        pub(crate) fn allows_path(&self, path: &Path) -> bool {
            !self.protected_namespace.contains(path)
        }

        pub(crate) fn path_is_in_workspace(&self, path: &Path) -> bool {
            let path = resolve_for_comparison(path);
            self.workspace_roots.iter().any(|root| {
                let root = resolve_for_comparison(root);
                path == root || path.starts_with(&root)
            })
        }

        pub(crate) fn should_auto_apply_provider_overlay(&self) -> bool {
            self.mode.enables_provider_overlay()
        }
    }

    /// Classify an approval without side effects. Protected paths take
    /// precedence over every mode, including Unattended, so the desktop can
    /// surface a deliberate user decision instead of silently crossing the
    /// boundary.
    pub(crate) fn classify_approval(
        request: &ApprovalRequest,
        policy: &RuntimePermissionPolicy,
    ) -> ApprovalDecision {
        if request
            .target
            .as_deref()
            .is_some_and(|target| policy.protected_namespace.contains(target))
        {
            return ApprovalDecision::Ask;
        }

        if request.operation == ApprovalOperation::ProjectTrust {
            return ApprovalDecision::Ask;
        }

        match policy.mode {
            PermissionMode::Prompt => ApprovalDecision::Ask,
            PermissionMode::Workspace => {
                if request
                    .target
                    .as_deref()
                    .is_some_and(|target| !policy.path_is_in_workspace(target))
                {
                    return ApprovalDecision::Ask;
                }
                if request
                    .cwd
                    .as_deref()
                    .is_some_and(|cwd| !policy.path_is_in_workspace(cwd))
                {
                    return ApprovalDecision::Ask;
                }
                classify_deterministic_request(request)
            }
            PermissionMode::FullAccess => classify_deterministic_request(request),
            PermissionMode::Unattended => classify_unattended_request(request),
        }
    }

    fn classify_deterministic_request(request: &ApprovalRequest) -> ApprovalDecision {
        match request.operation {
            ApprovalOperation::Read
            | ApprovalOperation::Write
            | ApprovalOperation::Shell
            | ApprovalOperation::Network => ApprovalDecision::Allow,
            ApprovalOperation::Confirm => ApprovalDecision::AutoAnswer(AutoAnswer::Confirm),
            ApprovalOperation::Select => request
                .default_index
                .filter(|index| *index < request.choice_count)
                .map(|index| ApprovalDecision::AutoAnswer(AutoAnswer::SelectDefault(index)))
                .unwrap_or(ApprovalDecision::Ask),
            ApprovalOperation::Input | ApprovalOperation::Editor | ApprovalOperation::Unknown => {
                ApprovalDecision::Ask
            }
            ApprovalOperation::ProjectTrust => ApprovalDecision::Ask,
        }
    }

    fn classify_unattended_request(request: &ApprovalRequest) -> ApprovalDecision {
        match request.operation {
            ApprovalOperation::Input | ApprovalOperation::Editor => request
                .default_answer
                .as_ref()
                .filter(|answer| !answer.trim().is_empty())
                .map(|answer| {
                    ApprovalDecision::AutoAnswer(AutoAnswer::InputDefault(answer.clone()))
                })
                .unwrap_or(ApprovalDecision::Ask),
            _ => classify_deterministic_request(request),
        }
    }

    fn normalize_for_comparison(path: &Path) -> PathBuf {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("/"))
                .join(path)
        };

        let mut normalized = PathBuf::new();
        for component in absolute.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    normalized.pop();
                }
                Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                    normalized.push(component.as_os_str());
                }
            }
        }
        normalized
    }

    fn resolve_for_comparison(path: &Path) -> PathBuf {
        let normalized = normalize_for_comparison(path);
        let mut existing = normalized.clone();
        let mut suffix = Vec::new();

        while fs::symlink_metadata(&existing).is_err() {
            let Some(name) = existing.file_name().map(ToOwned::to_owned) else {
                break;
            };
            suffix.push(name);
            if !existing.pop() {
                break;
            }
        }

        let mut resolved = fs::canonicalize(&existing).unwrap_or(existing);
        for component in suffix.iter().rev() {
            resolved.push(component);
        }
        normalize_for_comparison(&resolved)
    }
}

mod codex;
mod json_helpers;
mod toml_helpers;

use crate::theme::{AgentConfig, AgentPermissionsConfig, CodexConfig};
use codex::CodexRuntimeOverlay;

pub(super) fn apply_runtime_overlays(
    agents: &[AgentConfig],
    permissions: &AgentPermissionsConfig,
    codex_config: &CodexConfig,
) {
    let has_codex = agents.iter().any(|agent| agent.name == "codex");
    let has_claude = agents.iter().any(|agent| agent.name == "claude");

    if has_codex {
        let status_line_items = codex_config.status_line_items();
        codex::apply_codex_runtime_overlay(CodexRuntimeOverlay {
            yolo_enabled: permissions.codex_auto_full_access,
            fast_enabled: codex_config.fast_mode,
            goals_enabled: codex_config.goals,
            multi_agent_enabled: codex_config.multi_agent,
            web_search_mode: &codex_config.web_search,
            status_line_items: &status_line_items,
            jailbreak_prompt_file_enabled: codex_config.jailbreak_prompt_file,
            index_prompt_file_enabled: codex_config.index_prompt_file,
        });
    } else {
        codex::remove_codex_runtime_overlay(CodexRuntimeOverlay {
            yolo_enabled: false,
            fast_enabled: false,
            goals_enabled: false,
            multi_agent_enabled: false,
            web_search_mode: "default",
            status_line_items: &[],
            jailbreak_prompt_file_enabled: false,
            index_prompt_file_enabled: false,
        });
    }

    if has_claude && permissions.claude_auto_full_access {
        claude::apply_claude_permission_overlay();
    } else if has_claude {
        claude::remove_claude_permission_overlay();
    }
}

/// Apply runtime overlays with the desktop permission policy layered on top
/// of the legacy per-provider switches. The old entry point above remains
/// unchanged for existing Codex/Claude callers.
pub(crate) fn apply_runtime_overlays_with_policy(
    agents: &[AgentConfig],
    permissions: &AgentPermissionsConfig,
    codex_config: &CodexConfig,
    policy: &policy::RuntimePermissionPolicy,
) {
    if !policy.should_auto_apply_provider_overlay() {
        apply_runtime_overlays(agents, permissions, codex_config);
        return;
    }

    let mut effective = permissions.clone();
    // Full Access/Unattended is a global runtime choice. It enables the
    // provider-native bypass overlay, while the separate approval classifier
    // still protects the external namespaces and ambiguous UI questions.
    effective.codex_auto_full_access = true;
    effective.claude_auto_full_access = true;
    apply_runtime_overlays(agents, &effective, codex_config);
}
