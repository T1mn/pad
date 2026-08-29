//! Pure policy and persistence types for the PAD Desktop/Pi runtime.
//!
//! This module deliberately does not read or write configuration files, invoke
//! a process, or display a prompt.  The Desktop host can therefore use it from
//! both the native TUI and the future macOS UI.  Pi's JSONL session remains the
//! source of truth for a conversation; the types below are the small PAD-owned
//! read model and the policy decision boundary around that session.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

mod model;
mod path;
mod shell;

#[allow(
    unused_imports,
    reason = "Pi recovery DTOs remain part of the permission-policy API before every consumer lands"
)]
pub(crate) use model::{
    PiSessionCursor, PiSessionEntryRef, PiSessionHeader, PiSessionMetadata, Profile, Project,
    Section, SectionItem, Task, TaskEnvironment, TaskStatus,
};
#[cfg(test)]
pub(crate) use path::canonicalize_policy_path;
pub(crate) use path::{canonicalize_existing_prefix, matching_protected_namespace};
use path::{classify_risk, resolve_policy_path};
use shell::{assess_shell_command, ShellCommandAssessment};

/// The amount of automation granted to a PAD task.
///
/// `SystemFull` means that PAD will not ask for tool confirmation when the
/// operation target can be statically verified.  It is still subject to the
/// protected namespace check; dynamically evaluated shell syntax is never
/// automatic, nor are macOS controls such as TCC and Keychain bypassed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PermissionMode {
    /// Read-only operations are automatic; risky operations require approval.
    #[default]
    Guarded,
    /// Everything in the declared workspace roots is automatic.
    WorkspaceFull,
    /// Statically verified operations are automatic outside protected namespaces.
    SystemFull,
}

#[allow(
    dead_code,
    reason = "full-access predicate remains part of the policy API for native and Desktop consumers"
)]
impl PermissionMode {
    pub(crate) const fn is_full_access(self) -> bool {
        matches!(self, Self::WorkspaceFull | Self::SystemFull)
    }
}

/// A broad operation category supplied by the Pi adapter or a Desktop tool.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OperationKind {
    #[default]
    Read,
    Write,
    Execute,
    Delete,
    Network,
    Credential,
    Install,
    ProcessControl,
}

/// A tool operation before it is evaluated against an effective policy.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct PolicyOperation {
    pub kind: OperationKind,
    /// File or directory target.  Relative paths are resolved against the
    /// `cwd` passed to [`evaluate_operation`].
    #[serde(default)]
    pub path: Option<PathBuf>,
    /// Optional human-readable command or target detail for audit records.
    #[serde(default)]
    pub command: Option<String>,
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "operation constructors remain part of the policy API and are exercised by policy tests"
    )
)]
impl PolicyOperation {
    pub(crate) fn new(kind: OperationKind) -> Self {
        Self {
            kind,
            path: None,
            command: None,
        }
    }

    pub(crate) fn at_path(kind: OperationKind, path: impl Into<PathBuf>) -> Self {
        Self {
            kind,
            path: Some(path.into()),
            command: None,
        }
    }
}

/// Risk after the operation target has been scoped to workspace or outside it.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RiskClass {
    ReadOnly,
    WorkspaceWrite,
    WorkspaceExecute,
    WorkspaceDestructive,
    ExternalRead,
    ExternalWrite,
    ExternalExecute,
    ExternalDestructive,
    Network,
    Credential,
    Install,
    ProcessControl,
    Unknown,
    ProtectedNamespace,
}

impl RiskClass {
    pub(crate) const fn is_workspace_scoped(self) -> bool {
        matches!(
            self,
            Self::ReadOnly
                | Self::WorkspaceWrite
                | Self::WorkspaceExecute
                | Self::WorkspaceDestructive
        )
    }

    pub(crate) const fn is_read_only(self) -> bool {
        matches!(self, Self::ReadOnly)
    }
}

/// Result of evaluating one operation.  `Prompt` is only a policy result; the
/// host decides how to render/record the prompt.  In unattended mode prompts
/// become `Deny` so a worker can never block forever waiting for UI input.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub(crate) enum PolicyDecision {
    Allow { risk: RiskClass, reason: String },
    Prompt { risk: RiskClass, reason: String },
    Deny { risk: RiskClass, reason: String },
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "decision predicates remain part of the policy API and are exercised by policy tests"
    )
)]
impl PolicyDecision {
    pub(crate) const fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }

    pub(crate) const fn requires_confirmation(&self) -> bool {
        matches!(self, Self::Prompt { .. })
    }

    pub(crate) const fn is_denied(&self) -> bool {
        matches!(self, Self::Deny { .. })
    }

    pub(crate) const fn risk(&self) -> RiskClass {
        match self {
            Self::Allow { risk, .. } | Self::Prompt { risk, .. } | Self::Deny { risk, .. } => *risk,
        }
    }
}

/// A named path that PAD must never mutate through the Pi tool boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProtectedNamespace {
    pub name: String,
    pub root: PathBuf,
}

impl ProtectedNamespace {
    pub(crate) fn new(name: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            root: root.into(),
        }
    }
}

/// Optional values from one inheritance layer.
///
/// Profile is the base layer, then Project, then Task.  Explicit values in a
/// more specific layer replace the inherited scalar value.  Workspace roots
/// and protected namespaces are additive; a child can add a root but cannot
/// remove a protected namespace inherited from its parent.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct PolicyLayer {
    #[serde(default)]
    pub mode: Option<PermissionMode>,
    #[serde(default)]
    pub workspace_roots: Vec<PathBuf>,
    #[serde(default)]
    pub protected_namespaces: Vec<ProtectedNamespace>,
    #[serde(default)]
    pub unattended: Option<bool>,
}

/// Fully materialized policy used by the runtime.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct EffectivePolicy {
    pub mode: PermissionMode,
    #[serde(default)]
    pub workspace_roots: Vec<PathBuf>,
    #[serde(default)]
    pub protected_namespaces: Vec<ProtectedNamespace>,
    #[serde(default)]
    pub unattended: bool,
}

#[allow(
    dead_code,
    reason = "policy constructors remain part of the hierarchy API for native and Desktop consumers"
)]
impl EffectivePolicy {
    pub(crate) fn guarded() -> Self {
        Self::default()
    }

    pub(crate) fn workspace_full(workspace_roots: Vec<PathBuf>) -> Self {
        Self {
            mode: PermissionMode::WorkspaceFull,
            workspace_roots,
            ..Self::default()
        }
    }

    pub(crate) fn system_full(protected_namespaces: Vec<ProtectedNamespace>) -> Self {
        Self {
            mode: PermissionMode::SystemFull,
            protected_namespaces,
            ..Self::default()
        }
    }
}

/// Merge Profile -> Project -> Task policy layers.
pub(crate) fn merge_policy_layers(
    profile: &PolicyLayer,
    project: Option<&PolicyLayer>,
    task: Option<&PolicyLayer>,
) -> EffectivePolicy {
    let mode = task
        .and_then(|layer| layer.mode)
        .or_else(|| project.and_then(|layer| layer.mode))
        .or(profile.mode)
        .unwrap_or_default();
    let unattended = task
        .and_then(|layer| layer.unattended)
        .or_else(|| project.and_then(|layer| layer.unattended))
        .or(profile.unattended)
        .unwrap_or(false);

    let mut workspace_roots = Vec::new();
    append_unique_paths(&mut workspace_roots, &profile.workspace_roots);
    if let Some(project) = project {
        append_unique_paths(&mut workspace_roots, &project.workspace_roots);
    }
    if let Some(task) = task {
        append_unique_paths(&mut workspace_roots, &task.workspace_roots);
    }

    let mut protected_namespaces = Vec::new();
    append_unique_namespaces(&mut protected_namespaces, &profile.protected_namespaces);
    if let Some(project) = project {
        append_unique_namespaces(&mut protected_namespaces, &project.protected_namespaces);
    }
    if let Some(task) = task {
        append_unique_namespaces(&mut protected_namespaces, &task.protected_namespaces);
    }

    EffectivePolicy {
        mode,
        workspace_roots,
        protected_namespaces,
        unattended,
    }
}

/// Resolve the model hierarchy while adding the project's roots and the task's
/// cwd to the workspace scope.  The optional profile/project/task references
/// are intentionally independent so projectless tasks and profile-only tasks
/// are representable.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the compatibility merge helper remains test-covered; Desktop adds host defaults explicitly"
    )
)]
pub(crate) fn merge_profile_project_task(
    profile: &Profile,
    project: Option<&Project>,
    task: Option<&Task>,
) -> EffectivePolicy {
    merge_profile_project_task_with_host_defaults(profile, project, task, &[])
}

/// Merge the persisted hierarchy while adding host-owned namespaces that no
/// Profile/Project/Task layer is allowed to remove.  Desktop passes
/// [`default_protected_namespaces`] here on every decision so a legacy or
/// partially migrated Profile cannot accidentally make provider state
/// writable through Full Access.
pub(crate) fn merge_profile_project_task_with_host_defaults(
    profile: &Profile,
    project: Option<&Project>,
    task: Option<&Task>,
    host_protected_namespaces: &[ProtectedNamespace],
) -> EffectivePolicy {
    let mut policy = merge_policy_layers(
        &profile.policy,
        project.map(|project| &project.policy),
        task.map(|task| &task.policy),
    );

    append_unique_namespaces(&mut policy.protected_namespaces, host_protected_namespaces);

    if let Some(project) = project {
        append_unique_path(&mut policy.workspace_roots, &project.primary_root);
        append_unique_paths(&mut policy.workspace_roots, &project.additional_roots);
    }
    if let Some(task) = task {
        append_unique_path(&mut policy.workspace_roots, &task.cwd);
    }

    // A Pi process must not be able to mutate its own credentials or session
    // journal as a side effect of a task.  The host may explicitly expose a
    // separate administrative operation for profile management.
    append_unique_namespace(
        &mut policy.protected_namespaces,
        ProtectedNamespace::new("profile-agent-dir", profile.agent_dir.clone()),
    );
    append_unique_namespace(
        &mut policy.protected_namespaces,
        ProtectedNamespace::new("profile-session-dir", profile.session_dir.clone()),
    );

    policy
}

/// Conservative built-in namespaces.  `home` is supplied by the host so this
/// helper remains deterministic and does not inspect the process environment.
pub(crate) fn default_protected_namespaces(home: &Path) -> Vec<ProtectedNamespace> {
    [
        ("codex-home", home.join(".codex")),
        ("pi-home", home.join(".pi")),
        ("legacy-pad-home", home.join(".pad")),
        ("legacy-chatgpt-home", home.join(".chatgpt")),
        (
            "codex-application-support",
            home.join("Library/Application Support/com.openai.codex"),
        ),
        (
            "codex-application-support-current",
            home.join("Library/Application Support/Codex"),
        ),
        (
            "openai-application-support",
            home.join("Library/Application Support/OpenAI"),
        ),
        (
            "chatgpt-application-support",
            home.join("Library/Application Support/com.openai.chat"),
        ),
        (
            "chatgpt-application-support-legacy",
            home.join("Library/Application Support/com.openai.chatgpt"),
        ),
        (
            "chatgpt-application-support-current",
            home.join("Library/Application Support/ChatGPT"),
        ),
        (
            "codex-group-container",
            home.join("Library/Group Containers/group.com.openai.codex"),
        ),
        (
            "chatgpt-group-container",
            home.join("Library/Group Containers/group.com.openai.chat"),
        ),
        (
            "chatgpt-group-container-legacy",
            home.join("Library/Group Containers/group.com.openai.chatgpt"),
        ),
        (
            "codex-notifications-group-container",
            home.join("Library/Group Containers/2DC432GLL2.com.openai.codex.notifications"),
        ),
        (
            "chatgpt-cua-service-group-container",
            home.join("Library/Group Containers/2DC432GLL2.com.openai.sky.CUAService"),
        ),
        (
            "codex-container",
            home.join("Library/Containers/com.openai.codex"),
        ),
        (
            "chatgpt-container",
            home.join("Library/Containers/com.openai.chat"),
        ),
        (
            "chatgpt-container-legacy",
            home.join("Library/Containers/com.openai.chatgpt"),
        ),
        ("codex-cache", home.join("Library/Caches/Codex")),
        (
            "codex-cache-bundle",
            home.join("Library/Caches/com.openai.codex"),
        ),
        ("chatgpt-cache", home.join("Library/Caches/ChatGPT")),
        (
            "chatgpt-cache-bundle",
            home.join("Library/Caches/com.openai.chat"),
        ),
        (
            "chatgpt-cache-bundle-legacy",
            home.join("Library/Caches/com.openai.chatgpt"),
        ),
        ("codex-logs", home.join("Library/Logs/com.openai.codex")),
        ("chatgpt-logs", home.join("Library/Logs/com.openai.chat")),
        (
            "chatgpt-logs-legacy",
            home.join("Library/Logs/com.openai.chatgpt"),
        ),
        (
            "codex-http-storage",
            home.join("Library/HTTPStorages/com.openai.codex"),
        ),
        (
            "codex-http-cookie-storage",
            home.join("Library/HTTPStorages/com.openai.codex.binarycookies"),
        ),
        (
            "chatgpt-http-storage",
            home.join("Library/HTTPStorages/com.openai.chat"),
        ),
        (
            "chatgpt-http-cookie-storage",
            home.join("Library/HTTPStorages/com.openai.chat.binarycookies"),
        ),
        (
            "chatgpt-http-storage-legacy",
            home.join("Library/HTTPStorages/com.openai.chatgpt"),
        ),
        (
            "chatgpt-http-cookie-storage-legacy",
            home.join("Library/HTTPStorages/com.openai.chatgpt.binarycookies"),
        ),
        (
            "codex-preferences",
            home.join("Library/Preferences/com.openai.codex.plist"),
        ),
        (
            "chatgpt-preferences",
            home.join("Library/Preferences/com.openai.chat.plist"),
        ),
        (
            "chatgpt-preferences-legacy",
            home.join("Library/Preferences/com.openai.chatgpt.plist"),
        ),
        (
            "pad-desktop-application-support",
            home.join("Library/Application Support/PAD Desktop"),
        ),
    ]
    .into_iter()
    .map(|(name, root)| ProtectedNamespace::new(name, root))
    .collect()
}

/// Evaluate one Pi/tool operation against an already merged policy.
pub(crate) fn evaluate_operation(
    policy: &EffectivePolicy,
    operation: &PolicyOperation,
    cwd: &Path,
) -> PolicyDecision {
    let canonical_target = operation
        .path
        .as_deref()
        .map(|path| resolve_policy_path(path, cwd));

    if let Some(target) = canonical_target.as_deref() {
        if let Some(namespace) =
            matching_protected_namespace(target, Path::new("/"), &policy.protected_namespaces)
        {
            return PolicyDecision::Deny {
                risk: RiskClass::ProtectedNamespace,
                reason: format!(
                    "target {} is inside protected namespace {}",
                    target.display(),
                    namespace.name
                ),
            };
        }
    }

    // Credential access is administrative, not a task-level Full Access
    // operation.  Provider login/keychain flows use a separate control-plane
    // route and must never be accepted by an unattended Pi prompt.
    if operation.kind == OperationKind::Credential {
        return PolicyDecision::Deny {
            risk: RiskClass::Credential,
            reason: "credential operations require the dedicated profile administration flow"
                .to_string(),
        };
    }

    if let Some(command) = operation.command.as_deref() {
        match assess_shell_command(command, cwd, &policy.protected_namespaces) {
            ShellCommandAssessment::Verified => {}
            ShellCommandAssessment::Protected(namespace) => {
                return PolicyDecision::Deny {
                    risk: RiskClass::ProtectedNamespace,
                    reason: format!(
                        "command resolves inside protected namespace {}",
                        namespace.name
                    ),
                };
            }
            ShellCommandAssessment::Unresolved(reason) => {
                let reason = format!(
                    "shell command cannot be statically verified: {reason}; automatic confirmation is disabled"
                );
                return if policy.unattended {
                    PolicyDecision::Deny {
                        risk: RiskClass::Unknown,
                        reason,
                    }
                } else {
                    PolicyDecision::Prompt {
                        risk: RiskClass::Unknown,
                        reason,
                    }
                };
            }
        }
    }

    let risk = classify_risk(policy, operation, cwd);
    let (decision, reason) = match policy.mode {
        PermissionMode::SystemFull => (true, "system full access"),
        PermissionMode::WorkspaceFull if risk.is_workspace_scoped() => {
            (true, "workspace full access")
        }
        PermissionMode::WorkspaceFull => (false, "operation is outside workspace full access"),
        PermissionMode::Guarded if risk.is_read_only() => (true, "read-only operation"),
        PermissionMode::Guarded => (false, "guarded mode requires confirmation"),
    };

    if decision {
        return PolicyDecision::Allow {
            risk,
            reason: reason.to_string(),
        };
    }

    if policy.unattended {
        PolicyDecision::Deny {
            risk,
            reason: format!("{reason}; unattended execution cannot wait for confirmation"),
        }
    } else {
        PolicyDecision::Prompt {
            risk,
            reason: reason.to_string(),
        }
    }
}

fn append_unique_path(paths: &mut Vec<PathBuf>, path: &Path) {
    if !path.as_os_str().is_empty() && !paths.iter().any(|existing| existing == path) {
        paths.push(path.to_path_buf());
    }
}

fn append_unique_paths(paths: &mut Vec<PathBuf>, incoming: &[PathBuf]) {
    for path in incoming {
        append_unique_path(paths, path);
    }
}

fn append_unique_namespace(
    namespaces: &mut Vec<ProtectedNamespace>,
    namespace: ProtectedNamespace,
) {
    if namespace.root.as_os_str().is_empty()
        || namespaces
            .iter()
            .any(|existing| existing.root == namespace.root)
    {
        return;
    }
    namespaces.push(namespace);
}

fn append_unique_namespaces(
    namespaces: &mut Vec<ProtectedNamespace>,
    incoming: &[ProtectedNamespace],
) {
    for namespace in incoming {
        append_unique_namespace(namespaces, namespace.clone());
    }
}
