//! Small PAD Desktop control-plane facade.
//!
//! The native TUI keeps its existing lifecycle.  A future Tauri/WebKit host
//! can use this facade to compose the private Store, Codex sidebar snapshot,
//! and one Profile-scoped Pi supervisor per active Task without duplicating
//! policy or process code in the renderer.

use crate::pad_store::{open_default, PadStore, StoreError};
use crate::permission_policy::{PermissionMode, PolicyLayer, Profile, Project, Task, TaskStatus};
use crate::pi_runtime::{
    PiApprovalRequest, PiApprovalResponse, PiEventReducer, PiPoll, PiRpcSupervisor,
    PiRuntimeSnapshot, PiSupervisorError,
};
use crate::ui::codex_sidebar::{snapshot as sidebar_snapshot, CodexSidebarSnapshot};
use serde_json::json;
use std::collections::BTreeMap;
use std::fmt;

pub(crate) mod bridge;
pub(crate) use bridge::run_server;

#[derive(Debug)]
pub(crate) enum DesktopRuntimeError {
    Store(StoreError),
    Pi(PiSupervisorError),
    TaskNotFound(String),
    ProfileNotFound(String),
    ProjectNotFound(String),
    ProfileMismatch { task_id: String, profile_id: String },
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

    pub(crate) fn ensure_default_project(
        &mut self,
        profile_id: &str,
    ) -> Result<Option<Project>, DesktopRuntimeError> {
        if let Some(project) = self
            .store
            .list_projects(true)?
            .into_iter()
            .find(|project| project.profile_id.as_deref() == Some(profile_id))
        {
            return Ok(Some(project));
        }
        let root = std::env::current_dir().map_err(StoreError::Io)?;
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

    pub(crate) fn create_task(&mut self, mut task: Task) -> Result<Task, DesktopRuntimeError> {
        let profile = self
            .store
            .get_profile(&task.profile_id)?
            .ok_or_else(|| DesktopRuntimeError::ProfileNotFound(task.profile_id.clone()))?;
        if let Some(project_id) = task.project_id.as_deref() {
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
        }
        if task.id.trim().is_empty() {
            task.id = format!("task-{}", unique_suffix());
        }
        if task.title.trim().is_empty() {
            task.title = "New task".to_string();
        }
        if task.cwd.as_os_str().is_empty() {
            task.cwd = std::env::current_dir().map_err(StoreError::Io)?;
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
        if let Some(mut task) = self.store.get_task(task_id)? {
            task.status = status;
            task.updated_at = unix_timestamp();
            self.store.update_task(&task)?;
        }
        Ok(poll)
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

    pub(crate) fn is_running(&self, task_id: &str) -> bool {
        self.active_tasks.contains_key(task_id)
    }
}

fn automatic_ui_response(value: &serde_json::Value) -> Option<serde_json::Value> {
    let request = PiApprovalRequest::parse(value)?;
    let response = match request {
        PiApprovalRequest::Confirm { id, title, message } => {
            let text = format!(
                "{} {}",
                title.unwrap_or_default(),
                message.unwrap_or_default()
            )
            .to_ascii_lowercase();
            if text.contains("trust") || text.contains("project") {
                return None;
            }
            PiApprovalResponse::Confirm { id, value: true }
        }
        PiApprovalRequest::Select {
            id, default_index, ..
        } => PiApprovalResponse::Select {
            id,
            index: default_index.unwrap_or(0),
        },
        PiApprovalRequest::Input { id, default, .. }
        | PiApprovalRequest::Editor { id, default, .. } => PiApprovalResponse::Input {
            id,
            value: default.unwrap_or_default(),
        },
        PiApprovalRequest::Unknown { .. } => return None,
    };
    Some(response.to_value())
}

fn task_status(status: crate::pi_runtime::PiRuntimeStatus) -> TaskStatus {
    match status {
        crate::pi_runtime::PiRuntimeStatus::Starting => TaskStatus::Starting,
        crate::pi_runtime::PiRuntimeStatus::Idle => TaskStatus::Idle,
        crate::pi_runtime::PiRuntimeStatus::Running => TaskStatus::Running,
        crate::pi_runtime::PiRuntimeStatus::Streaming => TaskStatus::Streaming,
        crate::pi_runtime::PiRuntimeStatus::ToolRunning => TaskStatus::ToolRunning,
        crate::pi_runtime::PiRuntimeStatus::NeedsApproval => TaskStatus::NeedsApproval,
        crate::pi_runtime::PiRuntimeStatus::NeedsInput => TaskStatus::NeedsInput,
        crate::pi_runtime::PiRuntimeStatus::Compacting => TaskStatus::Compacting,
        crate::pi_runtime::PiRuntimeStatus::Retrying => TaskStatus::Retrying,
        crate::pi_runtime::PiRuntimeStatus::Failed => TaskStatus::Failed,
        crate::pi_runtime::PiRuntimeStatus::Disconnected => TaskStatus::Disconnected,
    }
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn profile_storage_segment(profile_id: &str) -> String {
    crate::pi_runtime::profile_storage_segment(profile_id)
}

fn unique_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    format!("{}-{sequence}", unix_timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permission_policy::{PolicyLayer, Project};
    use std::path::PathBuf;
    use std::thread;
    use std::time::Duration;

    fn profile() -> Profile {
        Profile {
            id: "profile-runtime".to_string(),
            name: "Runtime Profile".to_string(),
            agent_dir: std::env::temp_dir().join("pad-desktop-runtime-agent"),
            session_dir: std::env::temp_dir().join("pad-desktop-runtime-sessions"),
            ..Default::default()
        }
    }

    fn task() -> Task {
        Task {
            id: "task-runtime".to_string(),
            profile_id: "profile-runtime".to_string(),
            cwd: std::env::temp_dir(),
            title: "Runtime task".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn profile_scoped_process_events_update_the_private_task_record() {
        let mut runtime = DesktopRuntime::in_memory().unwrap();
        let profile = profile();
        let project = Project {
            id: "project-runtime".to_string(),
            name: "Runtime project".to_string(),
            primary_root: PathBuf::from("/tmp"),
            profile_id: Some(profile.id.clone()),
            policy: PolicyLayer::default(),
            ..Default::default()
        };
        runtime.store_mut().insert_profile(&profile).unwrap();
        runtime.store_mut().insert_project(&project).unwrap();
        let mut stored_task = task();
        stored_task.project_id = Some(project.id.clone());
        runtime.store_mut().insert_task(&stored_task).unwrap();

        runtime
            .start_task("task-runtime", "/bin/echo '{\"type\":\"agent_settled\"}'")
            .unwrap();
        for _ in 0..20 {
            let _ = runtime.poll_task("task-runtime").unwrap();
            if runtime
                .store()
                .get_task("task-runtime")
                .unwrap()
                .unwrap()
                .status
                == TaskStatus::Idle
            {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            runtime
                .store()
                .get_task("task-runtime")
                .unwrap()
                .unwrap()
                .status,
            TaskStatus::Idle
        );
        assert!(!runtime.is_running("missing"));
        runtime.stop_task("task-runtime").unwrap();
        assert!(!runtime.is_running("task-runtime"));
    }

    #[test]
    fn sidebar_snapshot_is_read_from_the_pad_store_only() {
        let mut runtime = DesktopRuntime::in_memory().unwrap();
        runtime.store_mut().insert_profile(&profile()).unwrap();
        let snapshot = runtime.sidebar_snapshot().unwrap();
        assert_eq!(
            snapshot.active_profile_id.as_deref(),
            Some("profile-runtime")
        );
        assert!(snapshot
            .rows
            .iter()
            .any(|row| row.node
                == crate::sidebar::CodexSidebarNode::Profile("profile-runtime".into())));
    }
}
