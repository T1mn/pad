//! Small PAD Desktop control-plane facade.
//!
//! The native TUI keeps its existing lifecycle.  A future Tauri/WebKit host
//! can use this facade to compose the private Store, Codex sidebar snapshot,
//! and one Profile-scoped Pi supervisor per active Task without duplicating
//! policy or process code in the renderer.

use crate::pad_store::{open_default, PadStore, StoreError};
use crate::permission_policy::{PermissionMode, PolicyLayer, Profile, Project, Task, TaskStatus};
use crate::pi_runtime::{
    PiEventReducer, PiPoll, PiRpcSupervisor, PiRuntimeSnapshot, PiSupervisorError,
};
use crate::ui::codex_sidebar::{snapshot as sidebar_snapshot, CodexSidebarSnapshot};
use serde_json::json;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

pub(crate) mod bridge;
pub(crate) use bridge::run_server;
mod helpers;
use helpers::{
    automatic_ui_response, default_desktop_workspace_root, is_unsafe_generated_project_root,
    path_within_root, profile_storage_segment, read_session_messages, task_status, unique_suffix,
    unix_timestamp,
};

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
}

/// One owner for Desktop-side orchestration.  It is deliberately not global;
/// Tauri can keep one instance per window or share it behind its application
/// state while the single-writer rule is enforced by the active-task map.
pub(crate) struct DesktopRuntime {
    store: PadStore,
    active_tasks: BTreeMap<String, ActiveTask>,
    next_generation: u64,
}

impl DesktopRuntime {
    pub(crate) fn open_default() -> Result<Self, DesktopRuntimeError> {
        Ok(Self::from_store(open_default()?))
    }

    pub(crate) fn in_memory() -> Result<Self, DesktopRuntimeError> {
        Ok(Self::from_store(PadStore::in_memory()?))
    }

    fn from_store(store: PadStore) -> Self {
        Self {
            store,
            active_tasks: BTreeMap::new(),
            next_generation: 1,
        }
    }

    pub(crate) fn store(&self) -> &PadStore {
        &self.store
    }

    pub(crate) fn store_mut(&mut self) -> &mut PadStore {
        &mut self.store
    }

    pub(crate) fn ensure_default_profile(&mut self) -> Result<Profile, DesktopRuntimeError> {
        if let Some(mut profile) = self.store.list_profiles()?.into_iter().next() {
            // Profiles created by the initial Desktop preview had no policy;
            // upgrade that one legacy record to the documented Full Access
            // default while retaining any explicit user choice thereafter.
            if profile.policy.mode.is_none() {
                profile.policy.mode = Some(PermissionMode::SystemFull);
                profile.policy.unattended = Some(true);
                if profile.policy.protected_namespaces.is_empty() {
                    profile.policy.protected_namespaces =
                        crate::permission_policy::default_protected_namespaces(
                            &dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/")),
                        );
                }
                profile.updated_at = unix_timestamp();
                self.store.update_profile(&profile)?;
            }
            return Ok(profile);
        }

        let id = "default".to_string();
        let fallback = crate::paths::pad_desktop_data_dir()
            .join("v1")
            .join("profiles")
            .join("default");
        let profile = Profile {
            id,
            name: "Default".to_string(),
            agent_dir: fallback.join("pi-agent"),
            session_dir: fallback.join("pi-sessions"),
            policy: PolicyLayer {
                mode: Some(PermissionMode::SystemFull),
                unattended: Some(true),
                protected_namespaces: crate::permission_policy::default_protected_namespaces(
                    &dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/")),
                ),
                ..PolicyLayer::default()
            },
            created_at: unix_timestamp(),
            updated_at: unix_timestamp(),
            ..Profile::default()
        };
        self.store.insert_profile(&profile)?;
        Ok(profile)
    }

    pub(crate) fn create_profile(
        &mut self,
        mut profile: Profile,
    ) -> Result<Profile, DesktopRuntimeError> {
        if profile.id.trim().is_empty() {
            profile.id = format!("profile-{}", unique_suffix());
        }
        if profile.name.trim().is_empty() {
            profile.name = profile.id.clone();
        }
        let fallback = crate::paths::pad_desktop_data_dir()
            .join("v1")
            .join("profiles")
            .join(profile_storage_segment(&profile.id));
        if profile.agent_dir.as_os_str().is_empty() {
            profile.agent_dir = fallback.join("pi-agent");
        }
        if profile.session_dir.as_os_str().is_empty() {
            profile.session_dir = fallback.join("pi-sessions");
        }
        if profile.created_at == 0 {
            profile.created_at = unix_timestamp();
        }
        profile.updated_at = unix_timestamp();
        self.store.insert_profile(&profile)?;
        Ok(profile)
    }

    /// Update the automation policy for a persisted Profile.
    ///
    /// The Desktop renderer may optimistically update its badge, but the
    /// private PAD store remains the source of truth.  Keeping this mutation
    /// here also ensures policy changes use the same profile boundary as Pi
    /// process startup.
    pub(crate) fn update_profile_policy(
        &mut self,
        profile_id: &str,
        permission_mode: Option<PermissionMode>,
        unattended: Option<bool>,
    ) -> Result<Profile, DesktopRuntimeError> {
        let mut profile = self
            .store
            .get_profile(profile_id)?
            .ok_or_else(|| DesktopRuntimeError::ProfileNotFound(profile_id.to_string()))?;
        if let Some(mode) = permission_mode {
            profile.policy.mode = Some(mode);
        }
        if let Some(value) = unattended {
            profile.policy.unattended = Some(value);
        }
        profile.updated_at = unix_timestamp();
        self.store.update_profile(&profile)?;
        Ok(profile)
    }

    /// Update non-secret provider selection metadata. `credential_ref` is a
    /// keychain reference only; the bridge never accepts or persists a token
    /// value. Keeping this mutation beside policy updates makes profile
    /// switching explicit and keeps each Pi process on one profile boundary.
    pub(crate) fn update_profile_settings(
        &mut self,
        profile_id: &str,
        default_provider: Option<String>,
        default_model: Option<String>,
        credential_ref: Option<String>,
    ) -> Result<Profile, DesktopRuntimeError> {
        let mut profile = self
            .store
            .get_profile(profile_id)?
            .ok_or_else(|| DesktopRuntimeError::ProfileNotFound(profile_id.to_string()))?;
        if let Some(provider) = default_provider {
            profile.default_provider = Some(provider);
        }
        if let Some(model) = default_model {
            profile.default_model = if model.trim().is_empty() || model == "auto" {
                None
            } else {
                Some(model)
            };
        }
        if let Some(reference) = credential_ref {
            profile.credential_ref = Some(reference);
        }
        profile.updated_at = unix_timestamp();
        self.store.update_profile(&profile)?;
        Ok(profile)
    }

    /// Return provider names present in this Profile's private Pi auth file.
    /// Values are intentionally not returned (and no provider-owned path is
    /// read), so the sidebar can show account readiness without exposing a
    /// token or changing Codex/ChatGPT credentials.
    pub(crate) fn authenticated_providers(&self, profile: &Profile) -> Vec<String> {
        helpers::authenticated_providers(profile)
    }

    pub(crate) fn provider_authentication_status(&self, profile: &Profile) -> &'static str {
        helpers::provider_authentication_status(profile)
    }

    pub(crate) fn ensure_default_project(
        &mut self,
        profile_id: &str,
    ) -> Result<Option<Project>, DesktopRuntimeError> {
        if let Some(mut project) = self
            .store
            .list_projects(true)?
            .into_iter()
            .find(|project| project.profile_id.as_deref() == Some(profile_id))
        {
            // Finder launches a macOS app with `/` as its working directory.
            // Older builds therefore created a generated Workspace whose root
            // covered the entire disk. Repair only that generated placeholder;
            // never rewrite a project path the user explicitly selected.
            if project.id == format!("project-{}", profile_storage_segment(profile_id))
                && project.name == "Workspace"
                && is_unsafe_generated_project_root(&project.primary_root)
            {
                project.primary_root = default_desktop_workspace_root();
                project.updated_at = unix_timestamp();
                self.store.update_project(&project)?;
            }
            return Ok(Some(project));
        }
        let root = default_desktop_workspace_root();
        let project = Project {
            id: format!("project-{}", profile_storage_segment(profile_id)),
            name: "Workspace".to_string(),
            primary_root: root,
            profile_id: Some(profile_id.to_string()),
            created_at: unix_timestamp(),
            updated_at: unix_timestamp(),
            ..Project::default()
        };
        self.store.insert_project(&project)?;
        Ok(Some(project))
    }

    pub(crate) fn create_project(
        &mut self,
        profile_id: &str,
        mut name: String,
        primary_root: PathBuf,
    ) -> Result<Project, DesktopRuntimeError> {
        self.store
            .get_profile(profile_id)?
            .ok_or_else(|| DesktopRuntimeError::ProfileNotFound(profile_id.to_string()))?;
        if name.trim().is_empty() {
            name = primary_root
                .file_name()
                .and_then(|value| value.to_str())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("Workspace")
                .to_string();
        }
        let project = Project {
            id: format!("project-{}", unique_suffix()),
            name,
            primary_root,
            profile_id: Some(profile_id.to_string()),
            created_at: unix_timestamp(),
            updated_at: unix_timestamp(),
            ..Project::default()
        };
        self.store.insert_project(&project)?;
        Ok(project)
    }

    pub(crate) fn create_task(&mut self, mut task: Task) -> Result<Task, DesktopRuntimeError> {
        let profile = self
            .store
            .get_profile(&task.profile_id)?
            .ok_or_else(|| DesktopRuntimeError::ProfileNotFound(task.profile_id.clone()))?;
        let task_project = if let Some(project_id) = task.project_id.as_deref() {
            let project = self
                .store
                .get_project(project_id)?
                .ok_or_else(|| DesktopRuntimeError::ProjectNotFound(project_id.to_string()))?;
            if project
                .profile_id
                .as_deref()
                .is_some_and(|profile_id| profile_id != profile.id)
            {
                return Err(DesktopRuntimeError::ProfileMismatch {
                    task_id: task.id.clone(),
                    profile_id: profile.id,
                });
            }
            Some(project)
        } else {
            None
        };
        if task.id.trim().is_empty() {
            task.id = format!("task-{}", unique_suffix());
        }
        if task.title.trim().is_empty() {
            task.title = "New task".to_string();
        }
        if task.cwd.as_os_str().is_empty() {
            task.cwd = task_project
                .as_ref()
                .map(|project| project.primary_root.clone())
                .unwrap_or_else(default_desktop_workspace_root);
        }
        if task.created_at == 0 {
            task.created_at = unix_timestamp();
        }
        task.updated_at = unix_timestamp();
        self.store.insert_task(&task)?;
        Ok(task)
    }

    pub(crate) fn sidebar_snapshot(&self) -> Result<CodexSidebarSnapshot, DesktopRuntimeError> {
        let (profiles, projects, tasks, sections) = self.store.load_sidebar_records()?;
        let mut sidebar = crate::sidebar::CodexSidebarState::default();
        sidebar.replace_data(profiles, projects, tasks, sections);
        Ok(sidebar_snapshot(&sidebar))
    }

    /// Start one Pi process for an existing PAD Task.  The Profile roots are
    /// selected from the Store, never from renderer-supplied environment data.
    pub(crate) fn start_task(
        &mut self,
        task_id: &str,
        command: &str,
    ) -> Result<u64, DesktopRuntimeError> {
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
        self.start_task_with_profile(&task, &profile, command)
    }

    fn start_task_with_profile(
        &mut self,
        task: &Task,
        profile: &Profile,
        command: &str,
    ) -> Result<u64, DesktopRuntimeError> {
        if task.profile_id != profile.id {
            return Err(DesktopRuntimeError::ProfileMismatch {
                task_id: task.id.clone(),
                profile_id: profile.id.clone(),
            });
        }
        let generation = self.next_generation;
        self.next_generation = self.next_generation.saturating_add(1).max(1);
        let supervisor =
            PiRpcSupervisor::spawn_for_profile(command, &task.cwd, generation, profile)?;
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
        let auto_answer = self
            .store
            .get_task(task_id)?
            .and_then(|task| {
                self.store
                    .get_profile(&task.profile_id)
                    .ok()
                    .flatten()
                    .map(|profile| {
                        matches!(
                            profile.policy.mode,
                            Some(PermissionMode::SystemFull | PermissionMode::WorkspaceFull)
                        ) || matches!(
                            task.policy.mode,
                            Some(PermissionMode::SystemFull | PermissionMode::WorkspaceFull)
                        )
                    })
            })
            .unwrap_or(false);
        let (poll, status) = {
            let active = self
                .active_tasks
                .get_mut(task_id)
                .ok_or_else(|| DesktopRuntimeError::TaskNotFound(task_id.to_string()))?;
            let poll = active.supervisor.poll()?;
            if auto_answer {
                for message in &poll.messages {
                    if let Some(response) = automatic_ui_response(&message.value) {
                        active.supervisor.send(response)?;
                    }
                }
            }
            for event in &poll.events {
                active.reducer.apply(event.clone());
            }
            if let Some(exit) = poll.exit_status {
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
            let poll = self.poll_task(task_id)?;
            if let Some(response) = poll.messages.into_iter().find(|message| {
                message.message_type == "response"
                    && message
                        .value
                        .get("command")
                        .and_then(|value| value.as_str())
                        == Some(expected_command)
            }) {
                return Ok(Some(response.value));
            }
            if poll.exit_status.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        Ok(None)
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

    pub(crate) fn set_model(
        &self,
        task_id: &str,
        provider: &str,
        model_id: &str,
    ) -> Result<(), DesktopRuntimeError> {
        let active = self
            .active_tasks
            .get(task_id)
            .ok_or_else(|| DesktopRuntimeError::TaskNotFound(task_id.to_string()))?;
        active.supervisor.send(json!({
            "type": "set_model",
            "provider": provider,
            "modelId": model_id,
        }))?;
        Ok(())
    }

    pub(crate) fn set_thinking_level(
        &self,
        task_id: &str,
        level: &str,
    ) -> Result<(), DesktopRuntimeError> {
        let active = self
            .active_tasks
            .get(task_id)
            .ok_or_else(|| DesktopRuntimeError::TaskNotFound(task_id.to_string()))?;
        active.supervisor.send(json!({
            "type": "set_thinking_level",
            "level": level,
        }))?;
        Ok(())
    }

    pub(crate) fn respond_ui(
        &self,
        task_id: &str,
        request_id: &str,
        response_kind: Option<&str>,
        value: serde_json::Value,
    ) -> Result<(), DesktopRuntimeError> {
        if request_id.trim().is_empty() {
            return Err(DesktopRuntimeError::Pi(PiSupervisorError::InvalidCommand(
                "extension UI response id is empty".to_string(),
            )));
        }
        let active = self
            .active_tasks
            .get(task_id)
            .ok_or_else(|| DesktopRuntimeError::TaskNotFound(task_id.to_string()))?;
        let response = if response_kind == Some("confirm") {
            json!({
                "type": "extension_ui_response",
                "id": request_id,
                "confirmed": value.as_bool().unwrap_or(false),
            })
        } else {
            json!({
                "type": "extension_ui_response",
                "id": request_id,
                "value": value,
            })
        };
        active.supervisor.send(response)?;
        Ok(())
    }

    fn update_task_metadata_from_poll(
        &mut self,
        task_id: &str,
        poll: &PiPoll,
    ) -> Result<(), DesktopRuntimeError> {
        helpers::update_task_metadata_from_poll(&mut self.store, task_id, poll)
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
    pub(crate) fn retry_task(
        &mut self,
        task_id: &str,
        command: &str,
    ) -> Result<u64, DesktopRuntimeError> {
        if let Some(active) = self.active_tasks.remove(task_id) {
            let _ = active.supervisor.shutdown()?;
        }
        if let Some(mut task) = self.store.get_task(task_id)? {
            task.status = TaskStatus::Retrying;
            task.updated_at = unix_timestamp();
            self.store.update_task(&task)?;
        }
        self.start_task(task_id, command)
    }

    pub(crate) fn is_running(&self, task_id: &str) -> bool {
        self.active_tasks.contains_key(task_id)
    }
}

#[cfg(test)]
pub(crate) mod tests;
