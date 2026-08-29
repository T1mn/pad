//! PAD Desktop presentation, Profile, Project, and Task catalog operations.

use super::*;

impl DesktopRuntime {
    /// Read the PAD-owned presentation document and normalize stale record
    /// references without consulting or mutating any provider-owned state.
    pub(crate) fn desktop_ui_state(&self) -> Result<DesktopUiState, DesktopRuntimeError> {
        let state = self.store.get_desktop_ui_state()?;
        self.normalize_desktop_ui_state(state)
    }

    /// Persist one complete presentation document. Partial/local renderer
    /// state is deliberately not merged here so every write is deterministic.
    pub(crate) fn set_desktop_ui_state(
        &mut self,
        state: DesktopUiState,
    ) -> Result<DesktopUiState, DesktopRuntimeError> {
        state.validate()?;
        let state = self.normalize_desktop_ui_state(state)?;
        self.store.set_desktop_ui_state(&state)?;
        Ok(state)
    }

    fn normalize_desktop_ui_state(
        &self,
        mut state: DesktopUiState,
    ) -> Result<DesktopUiState, DesktopRuntimeError> {
        let profiles = self.store.list_profiles()?;
        if state
            .active_profile_id
            .as_deref()
            .is_none_or(|active| !profiles.iter().any(|profile| profile.id == active))
        {
            state.active_profile_id = profiles.first().map(|profile| profile.id.clone());
        }
        let selected_is_valid = match (
            state.selected_task_id.as_deref(),
            state.active_profile_id.as_deref(),
        ) {
            (Some(task_id), Some(profile_id)) => self
                .store
                .get_task(task_id)?
                .is_some_and(|task| task.profile_id == profile_id),
            _ => false,
        };
        if !selected_is_valid {
            state.selected_task_id = None;
        }
        // Presentation state is shared by the Desktop process, but project
        // identifiers are account-scoped.  Never carry another Profile's
        // collapsed project IDs into the active renderer snapshot.
        let active_project_ids = state
            .active_profile_id
            .as_deref()
            .map(|profile_id| {
                self.store.list_projects(true).map(|projects| {
                    projects
                        .into_iter()
                        .filter(|project| project.profile_id.as_deref() == Some(profile_id))
                        .map(|project| format!("project:{}", project.id))
                        .collect::<HashSet<_>>()
                })
            })
            .transpose()?
            .unwrap_or_default();
        state
            .collapsed_project_ids
            .retain(|id| active_project_ids.contains(id));
        Ok(state)
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
            ensure_profile_private_storage(&profile)?;
            return Ok(profile);
        }

        let id = "default".to_string();
        let fallback = self.data_root.join("v1").join("profiles").join("default");
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
        ensure_profile_private_storage(&profile)?;
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
        let fallback = self
            .data_root
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
        ensure_profile_private_storage(&profile)?;
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

    pub(crate) fn auth_begin(
        &mut self,
        profile_id: &str,
        provider: &str,
        auth_type: &str,
    ) -> Result<AuthSnapshot, AuthError> {
        let profile = self
            .store
            .get_profile(profile_id)
            .map_err(|error| AuthError {
                code: "store_error",
                message: error.to_string(),
            })?
            .ok_or_else(|| AuthError {
                code: "profile_not_found",
                message: format!("Desktop profile '{profile_id}' was not found"),
            })?;
        self.auth.begin(&profile, provider, auth_type)
    }

    pub(crate) fn auth_status(&mut self) -> (AuthSnapshot, bool) {
        self.auth.status()
    }

    pub(crate) fn auth_owner_profile_id(&self) -> Option<&str> {
        self.auth.owner_profile_id()
    }

    pub(crate) fn auth_running_profile_id(&self) -> Option<&str> {
        self.auth.running_profile_id()
    }

    pub(crate) fn auth_respond(
        &mut self,
        attempt_id: &str,
        prompt_id: &str,
        value: serde_json::Value,
        cancelled: bool,
    ) -> Result<AuthSnapshot, AuthError> {
        self.auth.respond(attempt_id, prompt_id, value, cancelled)
    }

    pub(crate) fn auth_cancel(&mut self, attempt_id: &str) -> Result<AuthSnapshot, AuthError> {
        self.auth.cancel(attempt_id)
    }

    pub(crate) fn logout(
        &mut self,
        profile_id: &str,
        provider: &str,
    ) -> Result<AuthSnapshot, AuthError> {
        let profile = self
            .store
            .get_profile(profile_id)
            .map_err(|error| AuthError {
                code: "store_error",
                message: error.to_string(),
            })?
            .ok_or_else(|| AuthError {
                code: "profile_not_found",
                message: format!("Desktop profile '{profile_id}' was not found"),
            })?;
        self.auth.logout(&profile, provider)
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
}
