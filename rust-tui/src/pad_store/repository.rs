//! CRUD repository for the PAD-owned SQLite store.

use super::{
    json_decode, json_encode, open_database, open_memory_database, path_to_string, string_to_path,
    Profile, Project, Section, SectionItem, StoreError, StoreResult, Task,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

mod support;
use support::*;

pub(crate) const MIN_DESKTOP_SIDEBAR_WIDTH: u16 = 240;
pub(crate) const MAX_DESKTOP_SIDEBAR_WIDTH: u16 = 520;
const MAX_DESKTOP_UI_STATE_BYTES: usize = 48 * 1024;
const MAX_COLLAPSED_IDS_PER_GROUP: usize = 256;
const MAX_UI_STATE_ID_BYTES: usize = 128;

type SidebarRecords = (Vec<Profile>, Vec<Project>, Vec<Task>, Vec<Section>);

/// A bounded Desktop sidebar width. Invalid raw values never reach persisted
/// state through the repository API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct SidebarWidth(u16);

impl SidebarWidth {
    pub(crate) fn new(value: u16) -> StoreResult<Self> {
        if !(MIN_DESKTOP_SIDEBAR_WIDTH..=MAX_DESKTOP_SIDEBAR_WIDTH).contains(&value) {
            return Err(invalid_desktop_ui_state(format!(
                "sidebar_width must be between {MIN_DESKTOP_SIDEBAR_WIDTH} and \
                 {MAX_DESKTOP_SIDEBAR_WIDTH}, got {value}"
            )));
        }
        Ok(Self(value))
    }

    pub(crate) fn get(self) -> u16 {
        self.0
    }
}

impl Default for SidebarWidth {
    fn default() -> Self {
        Self(275)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DesktopTheme {
    Light,
    Dark,
    #[default]
    System,
}

/// Canonical navigation filter for the Codex-style task hierarchy.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DesktopSidebarView {
    #[default]
    All,
    Pinned,
    Archive,
}

/// PAD-owned presentation state. This shape deliberately has no filesystem,
/// provider-session, token, or credential field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesktopUiState {
    pub(crate) active_profile_id: Option<String>,
    pub(crate) selected_task_id: Option<String>,
    pub(crate) collapsed_section_ids: Vec<String>,
    pub(crate) collapsed_project_ids: Vec<String>,
    pub(crate) sidebar_width: SidebarWidth,
    #[serde(default)]
    pub(crate) sidebar_view: DesktopSidebarView,
    pub(crate) theme: DesktopTheme,
    pub(crate) right_panel_open: bool,
    pub(crate) bottom_panel_open: bool,
    pub(crate) sidebar_open: bool,
}

impl Default for DesktopUiState {
    fn default() -> Self {
        Self {
            active_profile_id: None,
            selected_task_id: None,
            collapsed_section_ids: Vec::new(),
            collapsed_project_ids: Vec::new(),
            sidebar_width: SidebarWidth::default(),
            sidebar_view: DesktopSidebarView::All,
            theme: DesktopTheme::System,
            right_panel_open: false,
            bottom_panel_open: false,
            sidebar_open: true,
        }
    }
}

impl DesktopUiState {
    pub(crate) fn validate(&self) -> StoreResult<()> {
        validate_optional_ui_state_id("active_profile_id", self.active_profile_id.as_deref())?;
        validate_optional_ui_state_id("selected_task_id", self.selected_task_id.as_deref())?;
        validate_collapsed_ids("collapsed_section_ids", &self.collapsed_section_ids)?;
        validate_collapsed_ids("collapsed_project_ids", &self.collapsed_project_ids)?;
        SidebarWidth::new(self.sidebar_width.get())?;
        Ok(())
    }
}

/// Repository for PAD's private control-plane metadata.
pub(crate) struct PadStore {
    connection: Connection,
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "the resolved database path is exposed to store isolation tests"
        )
    )]
    db_path: Option<PathBuf>,
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the repository keeps a complete CRUD surface while Desktop initially consumes a focused subset"
    )
)]
impl PadStore {
    pub(crate) fn open(path: impl AsRef<Path>) -> StoreResult<Self> {
        let (connection, db_path) = open_database(path)?;
        Ok(Self {
            connection,
            db_path: Some(db_path),
        })
    }

    /// In-memory constructor used by tests and ephemeral previews.
    pub(crate) fn in_memory() -> StoreResult<Self> {
        Ok(Self {
            connection: open_memory_database()?,
            db_path: None,
        })
    }

    pub(crate) fn db_path(&self) -> Option<&Path> {
        self.db_path.as_deref()
    }

    pub(crate) fn connection(&self) -> &Connection {
        &self.connection
    }

    // ---------------------------------------------------------------------
    // Desktop presentation state
    // ---------------------------------------------------------------------

    /// Return persisted Desktop presentation state, or safe defaults before
    /// the first write. Corrupt or out-of-bounds JSON is an explicit store
    /// serialization error rather than being silently reset.
    pub(crate) fn get_desktop_ui_state(&self) -> StoreResult<DesktopUiState> {
        let encoded = self
            .connection
            .query_row(
                "SELECT state_json FROM desktop_ui_state WHERE singleton_id = 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(encoded) = encoded else {
            return Ok(DesktopUiState::default());
        };
        if encoded.len() > MAX_DESKTOP_UI_STATE_BYTES {
            return Err(invalid_desktop_ui_state(format!(
                "state_json exceeds the {MAX_DESKTOP_UI_STATE_BYTES}-byte limit"
            )));
        }

        let state: DesktopUiState = serde_json::from_str(&encoded).map_err(|error| {
            invalid_desktop_ui_state(format!("stored state_json is corrupt: {error}"))
        })?;
        state.validate()?;
        Ok(state)
    }

    /// Atomically replace the singleton Desktop presentation document.
    pub(crate) fn set_desktop_ui_state(&mut self, state: &DesktopUiState) -> StoreResult<()> {
        state.validate()?;
        let encoded = json_encode(state)?;
        if encoded.len() > MAX_DESKTOP_UI_STATE_BYTES {
            return Err(invalid_desktop_ui_state(format!(
                "encoded state_json exceeds the {MAX_DESKTOP_UI_STATE_BYTES}-byte limit"
            )));
        }

        self.connection.execute(
            "INSERT INTO desktop_ui_state(singleton_id, state_json, updated_at)
             VALUES (1, ?1, ?2)
             ON CONFLICT(singleton_id) DO UPDATE SET
                 state_json = excluded.state_json,
                 updated_at = excluded.updated_at",
            params![encoded, timestamp(0)],
        )?;
        Ok(())
    }

    // ---------------------------------------------------------------------
    // Profiles
    // ---------------------------------------------------------------------

    pub(crate) fn insert_profile(&mut self, profile: &Profile) -> StoreResult<()> {
        self.connection.execute(
            "INSERT INTO profiles (
                id, name, agent_dir, session_dir, credential_ref,
                default_provider, default_model, policy_json, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                profile.id,
                profile.name,
                path_to_string(&profile.agent_dir),
                path_to_string(&profile.session_dir),
                profile.credential_ref,
                profile.default_provider,
                profile.default_model,
                json_encode(&profile.policy)?,
                timestamp(profile.created_at),
                timestamp(profile.updated_at),
            ],
        )?;
        Ok(())
    }

    pub(crate) fn get_profile(&self, id: &str) -> StoreResult<Option<Profile>> {
        self.connection
            .query_row(
                "SELECT id, name, agent_dir, session_dir, credential_ref,
                        default_provider, default_model, policy_json,
                        created_at, updated_at
                 FROM profiles WHERE id = ?1",
                [id],
                map_profile,
            )
            .optional()
            .map_err(StoreError::from)
    }

    pub(crate) fn list_profiles(&self) -> StoreResult<Vec<Profile>> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, agent_dir, session_dir, credential_ref,
                    default_provider, default_model, policy_json,
                    created_at, updated_at
             FROM profiles ORDER BY name COLLATE NOCASE, id",
        )?;
        let rows = statement.query_map([], map_profile)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub(crate) fn update_profile(&mut self, profile: &Profile) -> StoreResult<()> {
        let changed = self.connection.execute(
            "UPDATE profiles SET
                name = ?2,
                agent_dir = ?3,
                session_dir = ?4,
                credential_ref = ?5,
                default_provider = ?6,
                default_model = ?7,
                policy_json = ?8,
                updated_at = ?9
             WHERE id = ?1",
            params![
                profile.id,
                profile.name,
                path_to_string(&profile.agent_dir),
                path_to_string(&profile.session_dir),
                profile.credential_ref,
                profile.default_provider,
                profile.default_model,
                json_encode(&profile.policy)?,
                timestamp(profile.updated_at),
            ],
        )?;
        require_changed(changed, "profile", &profile.id)
    }

    #[allow(
        dead_code,
        reason = "profile deletion is reserved for the account-management UI and remains intentionally explicit"
    )]
    pub(crate) fn delete_profile(&mut self, id: &str) -> StoreResult<()> {
        let changed = self
            .connection
            .execute("DELETE FROM profiles WHERE id = ?1", [id])?;
        require_changed(changed, "profile", id)
    }

    // ---------------------------------------------------------------------
    // Projects
    // ---------------------------------------------------------------------

    pub(crate) fn insert_project(&mut self, project: &Project) -> StoreResult<()> {
        self.connection.execute(
            "INSERT INTO projects (
                id, name, primary_root, additional_roots_json, profile_id,
                policy_json, pinned, archived, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                project.id,
                project.name,
                path_to_string(&project.primary_root),
                json_encode(&project.additional_roots)?,
                project.profile_id,
                json_encode(&project.policy)?,
                bool_value(project.pinned),
                bool_value(project.archived),
                timestamp(project.created_at),
                timestamp(project.updated_at),
            ],
        )?;
        Ok(())
    }

    pub(crate) fn get_project(&self, id: &str) -> StoreResult<Option<Project>> {
        self.connection
            .query_row(
                "SELECT id, name, primary_root, additional_roots_json, profile_id,
                        policy_json, pinned, archived, created_at, updated_at
                 FROM projects WHERE id = ?1",
                [id],
                map_project,
            )
            .optional()
            .map_err(StoreError::from)
    }

    pub(crate) fn list_projects(&self, include_archived: bool) -> StoreResult<Vec<Project>> {
        let sql = if include_archived {
            "SELECT id, name, primary_root, additional_roots_json, profile_id,
                    policy_json, pinned, archived, created_at, updated_at
             FROM projects ORDER BY archived, pinned DESC, updated_at DESC, id"
        } else {
            "SELECT id, name, primary_root, additional_roots_json, profile_id,
                    policy_json, pinned, archived, created_at, updated_at
             FROM projects WHERE archived = 0
             ORDER BY pinned DESC, updated_at DESC, id"
        };
        let mut statement = self.connection.prepare(sql)?;
        let rows = statement.query_map([], map_project)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub(crate) fn update_project(&mut self, project: &Project) -> StoreResult<()> {
        let changed = self.connection.execute(
            "UPDATE projects SET
                name = ?2,
                primary_root = ?3,
                additional_roots_json = ?4,
                profile_id = ?5,
                policy_json = ?6,
                pinned = ?7,
                archived = ?8,
                updated_at = ?9
             WHERE id = ?1",
            params![
                project.id,
                project.name,
                path_to_string(&project.primary_root),
                json_encode(&project.additional_roots)?,
                project.profile_id,
                json_encode(&project.policy)?,
                bool_value(project.pinned),
                bool_value(project.archived),
                timestamp(project.updated_at),
            ],
        )?;
        require_changed(changed, "project", &project.id)
    }

    pub(crate) fn delete_project(&mut self, id: &str) -> StoreResult<()> {
        let changed = self
            .connection
            .execute("DELETE FROM projects WHERE id = ?1", [id])?;
        require_changed(changed, "project", id)
    }

    // ---------------------------------------------------------------------
    // Tasks
    // ---------------------------------------------------------------------

    pub(crate) fn insert_task(&mut self, task: &Task) -> StoreResult<()> {
        self.connection.execute(
            "INSERT INTO tasks (
                id, project_id, profile_id, pi_session_id, session_file,
                title, summary, cwd, environment, status, leaf_id,
                unread, pinned, archived, policy_json, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                       ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                task.id,
                task.project_id,
                task.profile_id,
                task.pi_session_id,
                task.session_file.as_deref().map(path_to_string),
                task.title,
                task.summary,
                path_to_string(&task.cwd),
                enum_encode(&task.environment)?,
                enum_encode(&task.status)?,
                task.leaf_id,
                bool_value(task.unread),
                bool_value(task.pinned),
                bool_value(task.archived),
                json_encode(&task.policy)?,
                timestamp(task.created_at),
                timestamp(task.updated_at),
            ],
        )?;
        Ok(())
    }

    pub(crate) fn get_task(&self, id: &str) -> StoreResult<Option<Task>> {
        let sql = task_select_sql("WHERE id = ?1");
        self.connection
            .query_row(&sql, [id], map_task)
            .optional()
            .map_err(StoreError::from)
    }

    pub(crate) fn list_tasks(
        &self,
        project_id: Option<&str>,
        include_archived: bool,
    ) -> StoreResult<Vec<Task>> {
        // Keep the project filter in SQL instead of filtering a full read
        // model, so archived task lists stay cheap.
        let filter = match (project_id.is_some(), include_archived) {
            (true, false) => "WHERE project_id = ?1 AND archived = 0",
            (true, true) => "WHERE project_id = ?1",
            (false, false) => "WHERE archived = 0",
            (false, true) => "",
        };
        let sql = format!(
            "{} {} ORDER BY pinned DESC, updated_at DESC, id",
            task_select_sql(""),
            filter
        );

        let mut statement = self.connection.prepare(&sql)?;
        let rows = match project_id {
            Some(project_id) => statement.query_map([project_id], map_task)?,
            None => statement.query_map([], map_task)?,
        };
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    #[allow(
        dead_code,
        reason = "profile-scoped task listing is retained for account-switch and diagnostics consumers"
    )]
    pub(crate) fn list_tasks_for_profile(
        &self,
        profile_id: &str,
        include_archived: bool,
    ) -> StoreResult<Vec<Task>> {
        let filter = if include_archived {
            "WHERE profile_id = ?1"
        } else {
            "WHERE profile_id = ?1 AND archived = 0"
        };
        let sql = format!(
            "{} {} ORDER BY pinned DESC, updated_at DESC, id",
            task_select_sql(""),
            filter
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map([profile_id], map_task)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub(crate) fn update_task(&mut self, task: &Task) -> StoreResult<()> {
        let changed = self.connection.execute(
            "UPDATE tasks SET
                project_id = ?2,
                profile_id = ?3,
                pi_session_id = ?4,
                session_file = ?5,
                title = ?6,
                summary = ?7,
                cwd = ?8,
                environment = ?9,
                status = ?10,
                leaf_id = ?11,
                unread = ?12,
                pinned = ?13,
                archived = ?14,
                policy_json = ?15,
                updated_at = ?16
             WHERE id = ?1",
            params![
                task.id,
                task.project_id,
                task.profile_id,
                task.pi_session_id,
                task.session_file.as_deref().map(path_to_string),
                task.title,
                task.summary,
                path_to_string(&task.cwd),
                enum_encode(&task.environment)?,
                enum_encode(&task.status)?,
                task.leaf_id,
                bool_value(task.unread),
                bool_value(task.pinned),
                bool_value(task.archived),
                json_encode(&task.policy)?,
                timestamp(task.updated_at),
            ],
        )?;
        require_changed(changed, "task", &task.id)
    }

    pub(crate) fn delete_task(&mut self, id: &str) -> StoreResult<()> {
        let changed = self
            .connection
            .execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        require_changed(changed, "task", id)
    }

    // ---------------------------------------------------------------------
    // Sections and polymorphic section items
    // ---------------------------------------------------------------------

    pub(crate) fn insert_section(&mut self, section: &Section) -> StoreResult<()> {
        let transaction = self.connection.transaction()?;
        insert_section_row(&transaction, section)?;
        insert_section_items(&transaction, &section.id, &section.items)?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn get_section(&self, id: &str) -> StoreResult<Option<Section>> {
        let section = self
            .connection
            .query_row(
                "SELECT id, name, section_order, collapsed, created_at, updated_at
                 FROM sections WHERE id = ?1",
                [id],
                map_section,
            )
            .optional()?;
        section
            .map(|mut section| {
                section.items = self.read_section_items(id)?;
                Ok(section)
            })
            .transpose()
    }

    pub(crate) fn list_sections(&self) -> StoreResult<Vec<Section>> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, section_order, collapsed, created_at, updated_at
             FROM sections ORDER BY section_order, name COLLATE NOCASE, id",
        )?;
        let rows = statement.query_map([], map_section)?;
        let mut sections = rows.collect::<Result<Vec<_>, _>>()?;
        for section in &mut sections {
            section.items = self.read_section_items(&section.id)?;
        }
        Ok(sections)
    }

    /// Load the complete Desktop sidebar read model in one explicit snapshot.
    /// The caller can replace its in-memory projection atomically after this
    /// returns; Pi session contents are intentionally not read here.
    pub(crate) fn load_sidebar_records(&self) -> StoreResult<SidebarRecords> {
        Ok((
            self.list_profiles()?,
            self.list_projects(true)?,
            self.list_tasks(None, true)?,
            self.list_sections()?,
        ))
    }

    #[allow(
        dead_code,
        reason = "section editing is reserved for the renderer organization workflow"
    )]
    pub(crate) fn update_section(&mut self, section: &Section) -> StoreResult<()> {
        let transaction = self.connection.transaction()?;
        let changed = transaction.execute(
            "UPDATE sections SET name = ?2, section_order = ?3, collapsed = ?4,
                                  updated_at = ?5
             WHERE id = ?1",
            params![
                section.id,
                section.name,
                i64::from(section.order),
                bool_value(section.collapsed),
                timestamp(section.updated_at),
            ],
        )?;
        if changed == 0 {
            return Err(StoreError::NotFound {
                kind: "section",
                id: section.id.clone(),
            });
        }
        transaction.execute(
            "DELETE FROM section_items WHERE section_id = ?1",
            [&section.id],
        )?;
        insert_section_items(&transaction, &section.id, &section.items)?;
        transaction.commit()?;
        Ok(())
    }

    #[allow(
        dead_code,
        reason = "section deletion is reserved for the renderer organization workflow"
    )]
    pub(crate) fn delete_section(&mut self, id: &str) -> StoreResult<()> {
        let changed = self
            .connection
            .execute("DELETE FROM sections WHERE id = ?1", [id])?;
        require_changed(changed, "section", id)
    }

    pub(crate) fn replace_section_items(
        &mut self,
        section_id: &str,
        items: &[SectionItem],
    ) -> StoreResult<()> {
        let transaction = self.connection.transaction()?;
        ensure_section_exists(&transaction, section_id)?;
        transaction.execute(
            "DELETE FROM section_items WHERE section_id = ?1",
            [section_id],
        )?;
        insert_section_items(&transaction, section_id, items)?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn add_section_item(
        &mut self,
        section_id: &str,
        item: &SectionItem,
    ) -> StoreResult<()> {
        let transaction = self.connection.transaction()?;
        ensure_section_exists(&transaction, section_id)?;
        let item_order: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(item_order), -1) + 1
             FROM section_items WHERE section_id = ?1",
            [section_id],
            |row| row.get(0),
        )?;
        insert_section_item(&transaction, section_id, item, item_order)?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn remove_section_item(
        &mut self,
        section_id: &str,
        item: &SectionItem,
    ) -> StoreResult<()> {
        let transaction = self.connection.transaction()?;
        ensure_section_exists(&transaction, section_id)?;
        let (kind, id) = section_item_parts(item);
        let changed = transaction.execute(
            "DELETE FROM section_items
             WHERE section_id = ?1 AND item_kind = ?2 AND item_id = ?3",
            params![section_id, kind, id],
        )?;
        require_changed(changed, "section item", id)?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn section_items(&self, section_id: &str) -> StoreResult<Vec<SectionItem>> {
        ensure_section_exists(&self.connection, section_id)?;
        self.read_section_items(section_id)
    }

    fn read_section_items(&self, section_id: &str) -> StoreResult<Vec<SectionItem>> {
        let mut statement = self.connection.prepare(
            "SELECT item_kind, item_id FROM section_items
             WHERE section_id = ?1 ORDER BY item_order, item_kind, item_id",
        )?;
        let rows = statement.query_map([section_id], |row| {
            let kind: String = row.get(0)?;
            let id: String = row.get(1)?;
            Ok(section_item_from_parts(&kind, id))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }
}
