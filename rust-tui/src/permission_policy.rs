//! Pure policy and persistence types for the PAD Desktop/Pi runtime.
//!
//! This module deliberately does not read or write configuration files, invoke
//! a process, or display a prompt.  The Desktop host can therefore use it from
//! both the native TUI and the future macOS UI.  Pi's JSONL session remains the
//! source of truth for a conversation; the types below are the small PAD-owned
//! read model and the policy decision boundary around that session.

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// The amount of automation granted to a PAD task.
///
/// `SystemFull` means that PAD will not ask for tool confirmation.  It is still
/// subject to the protected namespace check and to macOS controls such as TCC,
/// Keychain and the user's own process permissions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PermissionMode {
    /// Read-only operations are automatic; risky operations require approval.
    #[default]
    Guarded,
    /// Everything in the declared workspace roots is automatic.
    WorkspaceFull,
    /// All operations are automatic except PAD's protected namespaces.
    SystemFull,
}

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
pub(crate) fn merge_profile_project_task(
    profile: &Profile,
    project: Option<&Project>,
    task: Option<&Task>,
) -> EffectivePolicy {
    let mut policy = merge_policy_layers(
        &profile.policy,
        project.map(|project| &project.policy),
        task.map(|task| &task.policy),
    );

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
        (
            "codex-application-support",
            home.join("Library/Application Support/com.openai.codex"),
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
            "codex-group-container",
            home.join("Library/Group Containers/group.com.openai.codex"),
        ),
        (
            "chatgpt-group-container",
            home.join("Library/Group Containers/group.com.openai.chat"),
        ),
    ]
    .into_iter()
    .map(|(name, root)| ProtectedNamespace::new(name, root))
    .collect()
}

/// Lexically canonicalize a path without touching the filesystem.
///
/// This resolves `.` and `..`, removes duplicate separators through
/// `Path::components`, and makes relative paths relative to `base_dir`.  The
/// host should call `std::fs::canonicalize` first for an existing path when it
/// needs symlink resolution; new files cannot be canonicalized by the OS until
/// their parent exists, so this function is the safe fallback for both cases.
pub(crate) fn canonicalize_policy_path(path: &Path, base_dir: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    };
    let mut result = PathBuf::new();

    for component in joined.components() {
        match component {
            Component::Prefix(prefix) => result.push(prefix.as_os_str()),
            Component::RootDir => result.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                // Do not allow `..` to escape an absolute root.  For a
                // relative base this removes the last normal component.
                let popped = result.pop();
                if !popped && !result.has_root() {
                    result.push(Component::ParentDir.as_os_str());
                }
            }
            Component::Normal(part) => result.push(part),
        }
    }

    if result.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        result
    }
}

/// Return the protected namespace containing `path`, if any.
pub(crate) fn matching_protected_namespace<'a>(
    path: &Path,
    base_dir: &Path,
    namespaces: &'a [ProtectedNamespace],
) -> Option<&'a ProtectedNamespace> {
    let canonical_path = canonicalize_policy_path(path, base_dir);
    namespaces.iter().find(|namespace| {
        let canonical_root = canonicalize_policy_path(&namespace.root, base_dir);
        canonical_path == canonical_root || canonical_path.starts_with(&canonical_root)
    })
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
        .map(|path| canonicalize_policy_path(path, cwd));

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

/// Classify an operation by action and canonical workspace scope.
pub(crate) fn classify_risk(
    policy: &EffectivePolicy,
    operation: &PolicyOperation,
    cwd: &Path,
) -> RiskClass {
    let in_workspace = operation.path.as_deref().is_some_and(|path| {
        let target = canonicalize_policy_path(path, cwd);
        policy.workspace_roots.iter().any(|root| {
            let canonical_root = canonicalize_policy_path(root, cwd);
            target == canonical_root || target.starts_with(&canonical_root)
        })
    });

    match operation.kind {
        OperationKind::Read => {
            if operation.path.is_none() || in_workspace {
                RiskClass::ReadOnly
            } else {
                RiskClass::ExternalRead
            }
        }
        OperationKind::Write => {
            if in_workspace {
                RiskClass::WorkspaceWrite
            } else {
                RiskClass::ExternalWrite
            }
        }
        OperationKind::Execute => {
            if in_workspace {
                RiskClass::WorkspaceExecute
            } else {
                RiskClass::ExternalExecute
            }
        }
        OperationKind::Delete => {
            if in_workspace {
                RiskClass::WorkspaceDestructive
            } else {
                RiskClass::ExternalDestructive
            }
        }
        OperationKind::Network => RiskClass::Network,
        OperationKind::Credential => RiskClass::Credential,
        OperationKind::Install => RiskClass::Install,
        OperationKind::ProcessControl => RiskClass::ProcessControl,
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

/// Minimal Pi session header retained from the append-only JSONL journal.
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
