//! Small PAD Desktop control-plane facade.
//!
//! The native TUI keeps its existing lifecycle.  A future Tauri/WebKit host
//! can use this facade to compose the private Store, Codex sidebar snapshot,
//! and one Profile-scoped Pi supervisor per active Task without duplicating
//! policy or process code in the renderer.

use crate::pad_store::{DesktopSidebarView, DesktopUiState, PadStore, StoreError};
use crate::permission_policy::{
    merge_profile_project_task_with_host_defaults, EffectivePolicy, PermissionMode, PolicyLayer,
    Profile, Project, Task, TaskStatus,
};
use crate::pi_runtime::events::PiEventKind;
use crate::pi_runtime::{
    PiApprovalRequest, PiEventReducer, PiPoll, PiRpcSupervisor, PiRuntimeSnapshot,
    PiSupervisorError,
};
use crate::ui::codex_sidebar::{snapshot as sidebar_snapshot, CodexSidebarSnapshot};
use serde_json::json;
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub(crate) mod bridge;
pub(crate) use bridge::run_server;
mod auth;
pub(crate) mod remote;
mod remote_runtime;
use auth::{AuthError, AuthSnapshot, PiAuthCoordinator};
mod catalog;
pub(crate) mod data_root_lock;
pub(crate) mod model_catalog;
use data_root_lock::DesktopDataRootLock;
mod helpers;
mod interactions;
use helpers::{
    automatic_ui_response, default_desktop_workspace_root, is_unsafe_generated_project_root,
    path_within_root, profile_storage_segment, read_session_messages, task_status, unique_suffix,
    unix_timestamp,
};
mod terminal;
use terminal::DesktopTerminalRuntime;

#[derive(Debug)]
pub(crate) enum DesktopRuntimeError {
    Store(StoreError),
    Pi(PiSupervisorError),
    TaskNotFound(String),
    ProfileNotFound(String),
    ProjectNotFound(String),
    ProfileMismatch { task_id: String, profile_id: String },
    InvalidSessionPath { task_id: String, path: PathBuf },
    TaskAlreadyRunning(String),
    DataRootLocked,
}

impl fmt::Display for DesktopRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => error.fmt(formatter),
            Self::Pi(error) => error.fmt(formatter),
            Self::TaskNotFound(id) => write!(formatter, "Desktop task '{id}' was not found"),
            Self::ProfileNotFound(id) => write!(formatter, "Desktop profile '{id}' was not found"),
            Self::ProjectNotFound(id) => write!(formatter, "Desktop project '{id}' was not found"),
            Self::ProfileMismatch {
                task_id,
                profile_id,
            } => write!(
                formatter,
                "task '{task_id}' does not belong to profile '{profile_id}'"
            ),
            Self::InvalidSessionPath { task_id, path } => write!(
                formatter,
                "task '{task_id}' session path is outside its Profile session root: {}",
                path.display()
            ),
            Self::TaskAlreadyRunning(id) => {
                write!(formatter, "Desktop task '{id}' is already running")
            }
            Self::DataRootLocked => {
                formatter.write_str("another PAD desktop-server already owns this data root")
            }
        }
    }
}

impl std::error::Error for DesktopRuntimeError {}

impl From<StoreError> for DesktopRuntimeError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl From<PiSupervisorError> for DesktopRuntimeError {
    fn from(error: PiSupervisorError) -> Self {
        Self::Pi(error)
    }
}

struct ActiveTask {
    supervisor: PiRpcSupervisor,
    reducer: PiEventReducer,
    pending_ui_requests: BTreeMap<String, PendingUiRequest>,
}

struct PendingUiRequest {
    value: serde_json::Value,
    expires_at: Option<Instant>,
}

/// One owner for Desktop-side orchestration.  It is deliberately not global;
/// Tauri can keep one instance per window or share it behind its application
/// state while the single-writer rule is enforced by the active-task map.
pub(crate) struct DesktopRuntime {
    store: PadStore,
    data_root: PathBuf,
    active_tasks: BTreeMap<String, ActiveTask>,
    deferred_polls: BTreeMap<String, PiPoll>,
    auth: PiAuthCoordinator,
    terminal: DesktopTerminalRuntime,
    next_generation: u64,
    pi_program: PathBuf,
    remote_gateway: Option<remote::RemoteGateway>,
    remote_start_error: Option<String>,
    _data_root_lock: Option<DesktopDataRootLock>,
    #[cfg(test)]
    model_catalog_launcher: Option<(PathBuf, PathBuf)>,
}

impl DesktopRuntime {
    pub(crate) fn open_default() -> Result<Self, DesktopRuntimeError> {
        let requested_root = crate::paths::pad_desktop_data_dir();
        let root = crate::paths::base::validate_pad_desktop_data_root(&requested_root)
            .map_err(|error| DesktopRuntimeError::Store(StoreError::Io(error)))?;
        crate::paths::base::ensure_pad_desktop_private_layout(&root)
            .map_err(|error| DesktopRuntimeError::Store(StoreError::Io(error)))?;
        let data_root_lock = DesktopDataRootLock::acquire(&root).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                DesktopRuntimeError::DataRootLocked
            } else {
                DesktopRuntimeError::Store(StoreError::Io(error))
            }
        })?;
        let mut runtime = Self::from_store(PadStore::open(
            root.join("v1").join("store").join("pad.sqlite"),
        )?);
        runtime.data_root = root;
        runtime._data_root_lock = Some(data_root_lock);
        Ok(runtime)
    }

    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "the in-memory runtime is retained for protocol and isolation tests"
        )
    )]
    pub(crate) fn in_memory() -> Result<Self, DesktopRuntimeError> {
        Ok(Self::from_store(PadStore::in_memory()?))
    }

    fn from_store(store: PadStore) -> Self {
        #[cfg(test)]
        let data_root = {
            use std::sync::atomic::{AtomicU64, Ordering};
            static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(1);
            std::env::temp_dir().join(format!(
                "pad-desktop-runtime-test-{}-{}",
                std::process::id(),
                NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed)
            ))
        };
        #[cfg(not(test))]
        let data_root = crate::paths::pad_desktop_data_dir();
        Self {
            store,
            data_root,
            active_tasks: BTreeMap::new(),
            deferred_polls: BTreeMap::new(),
            auth: PiAuthCoordinator::new(),
            terminal: DesktopTerminalRuntime::default(),
            next_generation: 1,
            pi_program: crate::pi_runtime::desktop_pi_program(),
            remote_gateway: None,
            remote_start_error: None,
            _data_root_lock: None,
            #[cfg(test)]
            model_catalog_launcher: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn set_pi_program_for_test(&mut self, program: PathBuf) {
        self.pi_program = program;
    }

    #[cfg(test)]
    pub(crate) fn set_auth_launcher_for_test(&mut self, program: PathBuf, package_root: PathBuf) {
        self.auth.set_launcher_for_test(program, package_root);
    }

    pub(crate) fn store(&self) -> &PadStore {
        &self.store
    }

    pub(crate) fn store_mut(&mut self) -> &mut PadStore {
        &mut self.store
    }

    pub(crate) fn data_root(&self) -> &std::path::Path {
        &self.data_root
    }

    pub(crate) fn sidebar_snapshot(&self) -> Result<CodexSidebarSnapshot, DesktopRuntimeError> {
        let (profiles, mut projects, mut tasks, mut sections) =
            self.store.load_sidebar_records()?;
        let ui_state = self.desktop_ui_state()?;
        // Only the active Profile's project/task hierarchy may reach a
        // renderer snapshot. Profile rows themselves remain available as the
        // deliberately small account-switch surface.
        if let Some(active_profile_id) = ui_state.active_profile_id.as_deref() {
            projects.retain(|project| project.profile_id.as_deref() == Some(active_profile_id));
            tasks.retain(|task| task.profile_id == active_profile_id);
        } else {
            projects.clear();
            tasks.clear();
        }
        let visible_project_ids = projects
            .iter()
            .map(|project| project.id.as_str())
            .collect::<HashSet<_>>();
        let visible_task_ids = tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<HashSet<_>>();
        for section in &mut sections {
            section.items.retain(|item| match item {
                crate::permission_policy::SectionItem::Project(id) => {
                    visible_project_ids.contains(id.as_str())
                }
                crate::permission_policy::SectionItem::Task(id) => {
                    visible_task_ids.contains(id.as_str())
                }
            });
        }
        sections.retain(|section| !section.items.is_empty());
        let section_ids = sections
            .iter()
            .map(|section| section.id.as_str())
            .collect::<HashSet<_>>();
        let project_ids = projects
            .iter()
            .map(|project| project.id.as_str())
            .collect::<HashSet<_>>();
        let mut sidebar = crate::sidebar::CodexSidebarState {
            active_profile_id: ui_state.active_profile_id.clone(),
            view: match ui_state.sidebar_view {
                DesktopSidebarView::All => crate::sidebar::codex::CodexSidebarView::All,
                DesktopSidebarView::Pinned => crate::sidebar::codex::CodexSidebarView::Pinned,
                DesktopSidebarView::Archive => crate::sidebar::codex::CodexSidebarView::Archive,
            },
            collapsed_sections: collapse_record_ids(
                &ui_state.collapsed_section_ids,
                "section:",
                &section_ids,
            ),
            collapsed_projects: collapse_record_ids(
                &ui_state.collapsed_project_ids,
                "project:",
                &project_ids,
            ),
            ..Default::default()
        };
        sidebar.replace_data(profiles, projects, tasks, sections);
        // Collapsing a parent only hides a row; it must not change the active
        // conversation selected in the adjacent content pane.
        sidebar.selected = ui_state
            .selected_task_id
            .map(crate::sidebar::CodexSidebarNode::Task);
        Ok(sidebar_snapshot(&sidebar))
    }

    /// Start one Pi process for an existing PAD Task.  The Profile roots are
    /// selected from the Store, never from renderer-supplied environment data.
    pub(crate) fn start_task(&mut self, task_id: &str) -> Result<u64, DesktopRuntimeError> {
        if let Some(active) = self.active_tasks.get(task_id) {
            if active.supervisor.has_exited()? {
                self.active_tasks.remove(task_id);
            } else {
                return Err(DesktopRuntimeError::TaskAlreadyRunning(task_id.to_string()));
            }
        }
        let task = self
            .store
            .get_task(task_id)?
            .ok_or_else(|| DesktopRuntimeError::TaskNotFound(task_id.to_string()))?;
        let profile = self
            .store
            .get_profile(&task.profile_id)?
            .ok_or_else(|| DesktopRuntimeError::ProfileNotFound(task.profile_id.clone()))?;
        self.start_task_with_profile(&task, &profile)
    }

    fn start_task_with_profile(
        &mut self,
        task: &Task,
        profile: &Profile,
    ) -> Result<u64, DesktopRuntimeError> {
        if task.profile_id != profile.id {
            return Err(DesktopRuntimeError::ProfileMismatch {
                task_id: task.id.clone(),
                profile_id: profile.id.clone(),
            });
        }
        let generation = self.next_generation;
        self.next_generation = self.next_generation.saturating_add(1).max(1);
        let supervisor = PiRpcSupervisor::spawn_desktop_for_profile(
            &self.pi_program,
            &task.cwd,
            generation,
            profile,
        )?;
        // A Task's Pi session is durable metadata, not a command-line option
        // supplied by the renderer. Restore it only after the Profile-scoped
        // supervisor has validated and created its private roots. The startup
        // state request also gives us the canonical session path/id for a new
        // task without parsing Pi's on-disk JSONL journal ourselves.
        if let Some(session_file) = task.session_file.as_ref() {
            let session_root = crate::pi_runtime::profile_pi_roots(profile).1;
            let session_file = if session_file.is_absolute() {
                session_file.clone()
            } else {
                session_root.join(session_file)
            };
            if !path_within_root(&session_file, &session_root) {
                return Err(DesktopRuntimeError::InvalidSessionPath {
                    task_id: task.id.clone(),
                    path: session_file,
                });
            }
            supervisor.send(json!({
                "type": "switch_session",
                "sessionPath": session_file,
            }))?;
        }
        supervisor.send(json!({ "type": "get_state" }))?;
        let mut task = task.clone();
        task.status = TaskStatus::Starting;
        task.updated_at = unix_timestamp();
        self.store.update_task(&task)?;
        self.active_tasks.insert(
            task.id,
            ActiveTask {
                supervisor,
                reducer: PiEventReducer::new(generation),
                pending_ui_requests: BTreeMap::new(),
            },
        );
        Ok(generation)
    }

    pub(crate) fn send_prompt(
        &self,
        task_id: &str,
        prompt: &str,
    ) -> Result<(), DesktopRuntimeError> {
        let active = self
            .active_tasks
            .get(task_id)
            .ok_or_else(|| DesktopRuntimeError::TaskNotFound(task_id.to_string()))?;
        active
            .supervisor
            .send(json!({ "type": "prompt", "message": prompt }))?;
        Ok(())
    }

    /// Drain a task's process without waiting.  Pi events are reduced first;
    /// only the derived Task status is written to SQLite.
    pub(crate) fn poll_task(&mut self, task_id: &str) -> Result<PiPoll, DesktopRuntimeError> {
        if let Some(poll) = self.deferred_polls.remove(task_id) {
            return Ok(poll);
        }
        self.poll_task_fresh(task_id)
    }

    fn poll_task_fresh(&mut self, task_id: &str) -> Result<PiPoll, DesktopRuntimeError> {
        let policy_context = self.task_policy_context(task_id)?;
        let (poll, status) = {
            let active = self
                .active_tasks
                .get_mut(task_id)
                .ok_or_else(|| DesktopRuntimeError::TaskNotFound(task_id.to_string()))?;
            let pending_before = active.pending_ui_requests.len();
            let now = Instant::now();
            active
                .pending_ui_requests
                .retain(|_, pending| pending.expires_at.is_none_or(|deadline| deadline > now));
            let pending_expired = pending_before != active.pending_ui_requests.len();
            if pending_expired && active.pending_ui_requests.is_empty() {
                active.reducer.mark_ui_responded();
            }
            let mut poll = active.supervisor.poll()?;
            poll.state_changed = pending_expired;
            if poll.is_empty() && !pending_expired {
                return Ok(poll);
            }
            let mut automatically_answered = HashSet::new();
            if let Some((policy, cwd)) = policy_context.as_ref() {
                for message in &poll.messages {
                    if let Some(response) = automatic_ui_response(&message.value, policy, cwd) {
                        if let Some(id) = response.get("id").and_then(serde_json::Value::as_str) {
                            automatically_answered.insert(id.to_string());
                        }
                        active.supervisor.send(response)?;
                    }
                }
            }
            if !automatically_answered.is_empty() {
                let is_answered_request = |value: &serde_json::Value| {
                    value.get("type").and_then(serde_json::Value::as_str)
                        == Some("extension_ui_request")
                        && value
                            .get("id")
                            .or_else(|| value.get("requestId"))
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|id| automatically_answered.contains(id))
                };
                poll.messages
                    .retain(|message| !is_answered_request(&message.value));
                poll.events
                    .retain(|event| !is_answered_request(&event.value));
            }
            for event in &poll.events {
                if event.kind == PiEventKind::AgentSettled {
                    active.pending_ui_requests.clear();
                }
                active.reducer.apply(event.clone());
            }
            for message in &poll.messages {
                if message.message_type != "extension_ui_request" {
                    continue;
                }
                let Some(request) = PiApprovalRequest::parse(&message.value) else {
                    continue;
                };
                if matches!(request, PiApprovalRequest::Unknown { .. }) {
                    continue;
                }
                let Some(request_id) = request.id().map(str::to_string) else {
                    continue;
                };
                let expires_at = message
                    .value
                    .get("timeout")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|millis| Instant::now().checked_add(Duration::from_millis(millis)));
                active.pending_ui_requests.insert(
                    request_id,
                    PendingUiRequest {
                        value: message.value.clone(),
                        expires_at,
                    },
                );
            }
            if let Some(exit) = poll.exit_status {
                active.pending_ui_requests.clear();
                let ended_without_settling =
                    active.reducer.snapshot().status != crate::pi_runtime::PiRuntimeStatus::Idle;
                if !exit.success() || ended_without_settling {
                    active.reducer.mark_disconnected();
                }
            }
            (poll, task_status(active.reducer.snapshot().status))
        };
        self.update_task_metadata_from_poll(task_id, &poll)?;
        if let Some(mut task) = self.store.get_task(task_id)? {
            task.status = status;
            task.updated_at = unix_timestamp();
            self.store.update_task(&task)?;
        }
        Ok(poll)
    }

    /// Resolve the one effective policy used by Desktop automatic approval.
    /// A corrupt cross-Profile project reference deliberately produces no
    /// policy, which leaves the request for the user instead of inheriting
    /// another account's permissions.
    fn task_policy_context(
        &self,
        task_id: &str,
    ) -> Result<Option<(EffectivePolicy, PathBuf)>, DesktopRuntimeError> {
        let Some(task) = self.store.get_task(task_id)? else {
            return Ok(None);
        };
        let Some(mut profile) = self.store.get_profile(&task.profile_id)? else {
            return Ok(None);
        };
        let (agent_dir, session_dir) = crate::pi_runtime::profile_pi_roots(&profile);
        profile.agent_dir = agent_dir;
        profile.session_dir = session_dir;
        let project = match task.project_id.as_deref() {
            Some(project_id) => self.store.get_project(project_id)?,
            None => None,
        };
        if project.as_ref().is_some_and(|project| {
            project
                .profile_id
                .as_deref()
                .is_some_and(|profile_id| profile_id != task.profile_id)
        }) {
            return Ok(None);
        }
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let mut mandatory = crate::permission_policy::default_protected_namespaces(&home);
        mandatory.push(crate::permission_policy::ProtectedNamespace::new(
            "pad-desktop-data",
            self.data_root.clone(),
        ));
        mandatory.push(crate::permission_policy::ProtectedNamespace::new(
            "pad-home",
            crate::paths::pad_home_dir(),
        ));
        if let Some(codex_home) = crate::paths::base::protected_codex_home() {
            mandatory.push(crate::permission_policy::ProtectedNamespace::new(
                "codex-home-env",
                codex_home,
            ));
        }
        let policy = merge_profile_project_task_with_host_defaults(
            &profile,
            project.as_ref(),
            Some(&task),
            &mandatory,
        );
        Ok(Some((policy, task.cwd)))
    }

    /// Send one native Pi RPC request and wait only for already-available
    /// output (at most a short bounded window). This keeps the JSONL bridge
    /// responsive while still allowing history/state actions to return their
    /// response in the same IPC round trip in the common case.
    pub(crate) fn request_pi(
        &mut self,
        task_id: &str,
        command: serde_json::Value,
        expected_command: &str,
    ) -> Result<Option<serde_json::Value>, DesktopRuntimeError> {
        let active = self
            .active_tasks
            .get(task_id)
            .ok_or_else(|| DesktopRuntimeError::TaskNotFound(task_id.to_string()))?;
        active.supervisor.send(command)?;

        for _ in 0..40 {
            let mut poll = self.poll_task_fresh(task_id)?;
            if let Some(index) = poll.messages.iter().position(|message| {
                message.message_type == "response"
                    && message
                        .value
                        .get("command")
                        .and_then(|value| value.as_str())
                        == Some(expected_command)
            }) {
                let response = poll.messages.remove(index);
                if let Some(event_index) = poll
                    .events
                    .iter()
                    .position(|event| event.value == response.value)
                {
                    poll.events.remove(event_index);
                }
                self.defer_poll(task_id, poll);
                return Ok(Some(response.value));
            }
            let exited = poll.exit_status.is_some();
            self.defer_poll(task_id, poll);
            if exited {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        Ok(None)
    }

    fn defer_poll(&mut self, task_id: &str, mut poll: PiPoll) {
        if poll.is_empty() {
            return;
        }
        let deferred = self.deferred_polls.entry(task_id.to_string()).or_default();
        deferred.messages.append(&mut poll.messages);
        deferred.events.append(&mut poll.events);
        deferred.stderr.append(&mut poll.stderr);
        deferred.diagnostics.append(&mut poll.diagnostics);
        deferred.dropped_stale = deferred.dropped_stale.saturating_add(poll.dropped_stale);
        deferred.exit_status = poll.exit_status.or(deferred.exit_status);
    }

    pub(crate) fn get_messages(
        &mut self,
        task_id: &str,
    ) -> Result<Option<serde_json::Value>, DesktopRuntimeError> {
        self.request_pi(task_id, json!({ "type": "get_messages" }), "get_messages")
    }

    /// Load a Task transcript after the Desktop host has been restarted.
    ///
    /// A live Pi process remains the authoritative source and is queried via
    /// its native `get_messages` RPC.  For an idle persisted Task there is no
    /// process to query, so read the Profile-scoped JSONL journal directly.
    /// The journal is Pi-owned and this path is strictly read-only; the
    /// Profile root check prevents a stale Store record from escaping its
    /// private namespace.
    pub(crate) fn history(
        &mut self,
        task_id: &str,
    ) -> Result<serde_json::Value, DesktopRuntimeError> {
        if self.active_tasks.contains_key(task_id) {
            return Ok(self.get_messages(task_id)?.unwrap_or_else(|| {
                json!({
                    "type": "response",
                    "command": "get_messages",
                    "success": true,
                    "data": { "messages": [] }
                })
            }));
        }

        let task = self
            .store
            .get_task(task_id)?
            .ok_or_else(|| DesktopRuntimeError::TaskNotFound(task_id.to_string()))?;
        let profile = self
            .store
            .get_profile(&task.profile_id)?
            .ok_or_else(|| DesktopRuntimeError::ProfileNotFound(task.profile_id.clone()))?;
        let session_root = crate::pi_runtime::profile_pi_roots(&profile).1;
        let messages = match task.session_file {
            Some(path) => {
                let path = if path.is_absolute() {
                    path
                } else {
                    session_root.join(path)
                };
                if !path_within_root(&path, &session_root) {
                    return Err(DesktopRuntimeError::InvalidSessionPath {
                        task_id: task.id,
                        path,
                    });
                }
                read_session_messages(&path)
            }
            None => Vec::new(),
        };
        Ok(json!({
            "type": "response",
            "command": "get_messages",
            "success": true,
            "data": { "messages": messages }
        }))
    }

    pub(crate) fn get_state(
        &mut self,
        task_id: &str,
    ) -> Result<Option<serde_json::Value>, DesktopRuntimeError> {
        self.request_pi(task_id, json!({ "type": "get_state" }), "get_state")
    }

    pub(crate) fn get_entries(
        &mut self,
        task_id: &str,
    ) -> Result<Option<serde_json::Value>, DesktopRuntimeError> {
        self.request_pi(task_id, json!({ "type": "get_entries" }), "get_entries")
    }

    pub(crate) fn get_entries_since(
        &mut self,
        task_id: &str,
        since: &str,
    ) -> Result<Option<serde_json::Value>, DesktopRuntimeError> {
        self.request_pi(
            task_id,
            json!({ "type": "get_entries", "since": since }),
            "get_entries",
        )
    }

    pub(crate) fn abort_task(&self, task_id: &str) -> Result<(), DesktopRuntimeError> {
        let active = self
            .active_tasks
            .get(task_id)
            .ok_or_else(|| DesktopRuntimeError::TaskNotFound(task_id.to_string()))?;
        active.supervisor.send(json!({ "type": "abort" }))?;
        Ok(())
    }

    fn update_task_metadata_from_poll(
        &mut self,
        task_id: &str,
        poll: &PiPoll,
    ) -> Result<(), DesktopRuntimeError> {
        helpers::update_task_metadata_from_poll(&mut self.store, task_id, poll)?;
        if let Some(task) = self.store.get_task(task_id)? {
            if let Some(session_file) = task.session_file {
                if let Some(profile) = self.store.get_profile(&task.profile_id)? {
                    let session_root = crate::pi_runtime::profile_pi_roots(&profile).1;
                    let session_file = if session_file.is_absolute() {
                        session_file
                    } else {
                        session_root.join(session_file)
                    };
                    if path_within_root(&session_file, &session_root) {
                        crate::paths::base::harden_private_tree(&session_file)
                            .map_err(|error| DesktopRuntimeError::Store(StoreError::Io(error)))?;
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn runtime_snapshot(
        &self,
        task_id: &str,
    ) -> Result<Option<PiRuntimeSnapshot>, DesktopRuntimeError> {
        Ok(self
            .active_tasks
            .get(task_id)
            .map(|active| active.reducer.snapshot().clone()))
    }

    pub(crate) fn stop_task(&mut self, task_id: &str) -> Result<(), DesktopRuntimeError> {
        self.deferred_polls.remove(task_id);
        let Some(active) = self.active_tasks.remove(task_id) else {
            return Err(DesktopRuntimeError::TaskNotFound(task_id.to_string()));
        };
        let _ = active.supervisor.shutdown()?;
        if let Some(mut task) = self.store.get_task(task_id)? {
            task.status = TaskStatus::Disconnected;
            task.updated_at = unix_timestamp();
            self.store.update_task(&task)?;
        }
        Ok(())
    }

    /// Start a fresh Pi process for a previously stopped/failed Task while
    /// retaining its persisted session identity. A live process is aborted
    /// first, so retry cannot leave two writers attached to one JSONL session.
    pub(crate) fn retry_task(&mut self, task_id: &str) -> Result<u64, DesktopRuntimeError> {
        self.deferred_polls.remove(task_id);
        if let Some(active) = self.active_tasks.remove(task_id) {
            let _ = active.supervisor.shutdown()?;
        }
        if let Some(mut task) = self.store.get_task(task_id)? {
            task.status = TaskStatus::Retrying;
            task.updated_at = unix_timestamp();
            self.store.update_task(&task)?;
        }
        self.start_task(task_id)
    }

    #[allow(
        dead_code,
        reason = "runtime status probe is retained for native host integrations"
    )]
    pub(crate) fn is_running(&self, task_id: &str) -> bool {
        self.active_tasks.contains_key(task_id)
    }
}

fn collapse_record_ids(
    values: &[String],
    renderer_prefix: &str,
    valid_ids: &HashSet<&str>,
) -> HashSet<String> {
    values
        .iter()
        .filter_map(|value| {
            let candidate = value.strip_prefix(renderer_prefix).unwrap_or(value);
            valid_ids.contains(candidate).then(|| candidate.to_string())
        })
        .collect()
}

fn ensure_profile_private_storage(profile: &Profile) -> Result<(), DesktopRuntimeError> {
    let (agent_dir, session_dir) = crate::pi_runtime::profile_pi_roots(profile);
    for directory in [agent_dir, session_dir] {
        crate::paths::base::ensure_private_dir(&directory)
            .map_err(|error| DesktopRuntimeError::Store(StoreError::Io(error)))?;
        crate::paths::base::harden_private_tree(&directory)
            .map_err(|error| DesktopRuntimeError::Store(StoreError::Io(error)))?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests;
