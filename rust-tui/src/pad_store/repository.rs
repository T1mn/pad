//! CRUD repository for the PAD-owned SQLite store.

use super::{
    json_decode, json_encode, open_database, open_memory_database, path_to_string, string_to_path,
    Profile, Project, Section, SectionItem, StoreError, StoreResult, Task,
};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Repository for PAD's private control-plane metadata.
pub(crate) struct PadStore {
    connection: Connection,
    db_path: Option<PathBuf>,
}

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
    pub(crate) fn load_sidebar_records(
        &self,
    ) -> StoreResult<(Vec<Profile>, Vec<Project>, Vec<Task>, Vec<Section>)> {
        Ok((
            self.list_profiles()?,
            self.list_projects(true)?,
            self.list_tasks(None, true)?,
            self.list_sections()?,
        ))
    }

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

fn task_select_sql(suffix: &str) -> String {
    format!(
        "SELECT id, project_id, profile_id, pi_session_id, session_file,
                title, summary, cwd, environment, status, leaf_id,
                unread, pinned, archived, policy_json, created_at, updated_at
         FROM tasks {suffix}"
    )
}

fn insert_section_row(transaction: &Transaction<'_>, section: &Section) -> StoreResult<()> {
    transaction.execute(
        "INSERT INTO sections (
            id, name, section_order, collapsed, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            section.id,
            section.name,
            i64::from(section.order),
            bool_value(section.collapsed),
            timestamp(section.created_at),
            timestamp(section.updated_at),
        ],
    )?;
    Ok(())
}

fn insert_section_items(
    transaction: &Transaction<'_>,
    section_id: &str,
    items: &[SectionItem],
) -> StoreResult<()> {
    for (item_order, item) in items.iter().enumerate() {
        insert_section_item(transaction, section_id, item, item_order as i64)?;
    }
    Ok(())
}

fn insert_section_item(
    transaction: &Transaction<'_>,
    section_id: &str,
    item: &SectionItem,
    item_order: i64,
) -> StoreResult<()> {
    let (kind, id) = section_item_parts(item);
    transaction.execute(
        "INSERT INTO section_items (section_id, item_kind, item_id, item_order)
         VALUES (?1, ?2, ?3, ?4)",
        params![section_id, kind, id, item_order],
    )?;
    Ok(())
}

fn ensure_section_exists(connection: &Connection, id: &str) -> StoreResult<()> {
    let exists: Option<String> = connection
        .query_row("SELECT id FROM sections WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .optional()?;
    if exists.is_none() {
        return Err(StoreError::NotFound {
            kind: "section",
            id: id.to_string(),
        });
    }
    Ok(())
}

fn section_item_parts(item: &SectionItem) -> (&'static str, &str) {
    match item {
        SectionItem::Project(id) => ("project", id.as_str()),
        SectionItem::Task(id) => ("task", id.as_str()),
    }
}

fn section_item_from_parts(kind: &str, id: String) -> SectionItem {
    if kind == "project" {
        SectionItem::Project(id)
    } else {
        SectionItem::Task(id)
    }
}

fn map_profile(row: &Row<'_>) -> rusqlite::Result<Profile> {
    Ok(Profile {
        id: row.get(0)?,
        name: row.get(1)?,
        agent_dir: string_to_path(row.get(2)?),
        session_dir: string_to_path(row.get(3)?),
        credential_ref: row.get(4)?,
        default_provider: row.get(5)?,
        default_model: row.get(6)?,
        policy: decode_json_row(row.get(7)?, 7)?,
        created_at: decode_u64_row(row.get(8)?, 8)?,
        updated_at: decode_u64_row(row.get(9)?, 9)?,
    })
}

fn map_project(row: &Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        primary_root: string_to_path(row.get(2)?),
        additional_roots: decode_json_row(row.get(3)?, 3)?,
        profile_id: row.get(4)?,
        policy: decode_json_row(row.get(5)?, 5)?,
        pinned: row.get::<_, i64>(6)? != 0,
        archived: row.get::<_, i64>(7)? != 0,
        created_at: decode_u64_row(row.get(8)?, 8)?,
        updated_at: decode_u64_row(row.get(9)?, 9)?,
    })
}

fn map_task(row: &Row<'_>) -> rusqlite::Result<Task> {
    let session_file: Option<String> = row.get(4)?;
    Ok(Task {
        id: row.get(0)?,
        project_id: row.get(1)?,
        profile_id: row.get(2)?,
        pi_session_id: row.get(3)?,
        session_file: session_file.map(string_to_path),
        title: row.get(5)?,
        summary: row.get(6)?,
        cwd: string_to_path(row.get(7)?),
        environment: decode_enum_row(row.get(8)?, 8)?,
        status: decode_enum_row(row.get(9)?, 9)?,
        leaf_id: row.get(10)?,
        unread: row.get::<_, i64>(11)? != 0,
        pinned: row.get::<_, i64>(12)? != 0,
        archived: row.get::<_, i64>(13)? != 0,
        policy: decode_json_row(row.get(14)?, 14)?,
        created_at: decode_u64_row(row.get(15)?, 15)?,
        updated_at: decode_u64_row(row.get(16)?, 16)?,
    })
}

fn map_section(row: &Row<'_>) -> rusqlite::Result<Section> {
    Ok(Section {
        id: row.get(0)?,
        name: row.get(1)?,
        order: u32::try_from(row.get::<_, i64>(2)?)
            .map_err(|error| sqlite_conversion_error(2, error))?,
        collapsed: row.get::<_, i64>(3)? != 0,
        items: Vec::new(),
        created_at: decode_u64_row(row.get(4)?, 4)?,
        updated_at: decode_u64_row(row.get(5)?, 5)?,
    })
}

fn decode_json_row<T: DeserializeOwned>(value: String, index: usize) -> rusqlite::Result<T> {
    json_decode(&value).map_err(|error| sqlite_conversion_error(index, error))
}

fn decode_enum_row<T: DeserializeOwned>(value: String, index: usize) -> rusqlite::Result<T> {
    serde_json::from_value(Value::String(value))
        .map_err(|error| sqlite_conversion_error(index, error))
}

fn decode_u64_row(value: i64, index: usize) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|error| sqlite_conversion_error(index, error))
}

fn sqlite_conversion_error<E>(index: usize, error: E) -> rusqlite::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, Box::new(error))
}

fn enum_encode<T: Serialize>(value: &T) -> StoreResult<String> {
    let value = serde_json::to_value(value)?;
    value.as_str().map(ToOwned::to_owned).ok_or_else(|| {
        StoreError::Serialization(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "expected a string enum",
        )))
    })
}

fn bool_value(value: bool) -> i64 {
    i64::from(value)
}

fn timestamp(value: u64) -> i64 {
    if value == 0 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_secs()).ok())
            .unwrap_or(0)
    } else {
        i64::try_from(value).unwrap_or(i64::MAX)
    }
}

fn require_changed(changed: usize, kind: &'static str, id: &str) -> StoreResult<()> {
    if changed == 0 {
        Err(StoreError::NotFound {
            kind,
            id: id.to_string(),
        })
    } else {
        Ok(())
    }
}
