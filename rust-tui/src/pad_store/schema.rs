//! SQLite schema and migrations for PAD's private control-plane store.
//!
//! This schema is deliberately independent from the provider stores used by
//! Codex, ChatGPT, Claude, or any other agent.  The only persisted data here
//! is PAD-owned metadata and references to Pi sessions.

use super::{StoreError, StoreResult};
use rusqlite::Connection;

/// Schema version currently understood by this build of PAD.
pub(crate) const CURRENT_SCHEMA_VERSION: i64 = 2;

/// Apply all migrations in one transaction.
pub(crate) fn apply_migrations(connection: &mut Connection) -> StoreResult<()> {
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(StoreError::from)?;

    let version = connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .map_err(StoreError::from)?;
    if version > CURRENT_SCHEMA_VERSION {
        return Err(StoreError::UnsupportedSchemaVersion {
            found: version,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }

    if version < CURRENT_SCHEMA_VERSION {
        let transaction = connection.transaction().map_err(StoreError::from)?;
        if version < 1 {
            transaction
                .execute_batch(MIGRATION_V1)
                .map_err(StoreError::from)?;
        }
        if version < 2 {
            transaction
                .execute_batch(MIGRATION_V2)
                .map_err(StoreError::from)?;
        }
        transaction
            .pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)
            .map_err(StoreError::from)?;
        transaction.commit().map_err(StoreError::from)?;
    }

    // A connection can be handed to us from a caller that has changed the
    // pragma after opening.  Re-assert it after migration as an invariant.
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(StoreError::from)?;
    Ok(())
}

const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    agent_dir TEXT NOT NULL,
    session_dir TEXT NOT NULL,
    credential_ref TEXT,
    default_provider TEXT,
    default_model TEXT,
    policy_json TEXT NOT NULL DEFAULT '{}',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    primary_root TEXT NOT NULL,
    additional_roots_json TEXT NOT NULL DEFAULT '[]',
    profile_id TEXT REFERENCES profiles(id) ON DELETE SET NULL,
    policy_json TEXT NOT NULL DEFAULT '{}',
    pinned INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_projects_profile ON projects(profile_id);
CREATE INDEX IF NOT EXISTS idx_projects_sidebar ON projects(archived, pinned, updated_at DESC);

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE RESTRICT,
    pi_session_id TEXT,
    session_file TEXT,
    title TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL DEFAULT '',
    cwd TEXT NOT NULL,
    environment TEXT NOT NULL DEFAULT 'local',
    status TEXT NOT NULL DEFAULT 'idle',
    leaf_id TEXT,
    unread INTEGER NOT NULL DEFAULT 0 CHECK (unread IN (0, 1)),
    pinned INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    policy_json TEXT NOT NULL DEFAULT '{}',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_id, archived, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_profile ON tasks(profile_id, archived, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_sidebar ON tasks(archived, pinned, updated_at DESC);

CREATE TABLE IF NOT EXISTS sections (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    section_order INTEGER NOT NULL DEFAULT 0,
    collapsed INTEGER NOT NULL DEFAULT 0 CHECK (collapsed IN (0, 1)),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS section_items (
    section_id TEXT NOT NULL REFERENCES sections(id) ON DELETE CASCADE,
    item_kind TEXT NOT NULL CHECK (item_kind IN ('project', 'task')),
    item_id TEXT NOT NULL,
    item_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (section_id, item_kind, item_id),
    UNIQUE (section_id, item_order)
);

CREATE INDEX IF NOT EXISTS idx_section_items_target
    ON section_items(item_kind, item_id);

-- SQLite cannot express a foreign key whose target table depends on a kind
-- column.  These triggers provide the same invariant for the polymorphic
-- section_items API, and make orphan references fail inside the SQL
-- transaction rather than being silently persisted.
CREATE TRIGGER IF NOT EXISTS section_items_require_target_insert
BEFORE INSERT ON section_items
WHEN (NEW.item_kind = 'project' AND NOT EXISTS (
          SELECT 1 FROM projects WHERE id = NEW.item_id
      ))
  OR (NEW.item_kind = 'task' AND NOT EXISTS (
          SELECT 1 FROM tasks WHERE id = NEW.item_id
      ))
BEGIN
    SELECT RAISE(ABORT, 'section item references an unknown target');
END;

CREATE TRIGGER IF NOT EXISTS section_items_require_target_update
BEFORE UPDATE OF item_kind, item_id ON section_items
WHEN (NEW.item_kind = 'project' AND NOT EXISTS (
          SELECT 1 FROM projects WHERE id = NEW.item_id
      ))
  OR (NEW.item_kind = 'task' AND NOT EXISTS (
          SELECT 1 FROM tasks WHERE id = NEW.item_id
      ))
BEGIN
    SELECT RAISE(ABORT, 'section item references an unknown target');
END;

CREATE TRIGGER IF NOT EXISTS projects_remove_section_items
AFTER DELETE ON projects
BEGIN
    DELETE FROM section_items
    WHERE item_kind = 'project' AND item_id = OLD.id;
END;

CREATE TRIGGER IF NOT EXISTS tasks_remove_section_items
AFTER DELETE ON tasks
BEGIN
    DELETE FROM section_items
    WHERE item_kind = 'task' AND item_id = OLD.id;
END;
"#;

const MIGRATION_V2: &str = r#"
-- A singleton PAD-owned document keeps Desktop presentation state separate
-- from profiles, tasks, Pi journals, and every provider-owned database.  The
-- SQL checks are a second boundary behind the strongly typed repository API.
CREATE TABLE IF NOT EXISTS desktop_ui_state (
    singleton_id INTEGER PRIMARY KEY NOT NULL CHECK (singleton_id = 1),
    state_json TEXT NOT NULL
        CHECK (json_valid(state_json))
        CHECK (json_type(state_json) IS 'object')
        CHECK (length(CAST(state_json AS BLOB)) <= 49152)
        CHECK (COALESCE(json_type(state_json, '$.active_profile_id') IN ('null', 'text'), 0))
        CHECK (COALESCE(json_type(state_json, '$.selected_task_id') IN ('null', 'text'), 0))
        CHECK (json_type(state_json, '$.sidebar_width') IS 'integer')
        CHECK (COALESCE(json_extract(state_json, '$.sidebar_width') BETWEEN 240 AND 520, 0))
        CHECK (COALESCE(json_extract(state_json, '$.theme') IN ('light', 'dark', 'system'), 0))
        CHECK (COALESCE(json_type(state_json, '$.right_panel_open') IN ('true', 'false'), 0))
        CHECK (COALESCE(json_type(state_json, '$.bottom_panel_open') IN ('true', 'false'), 0))
        CHECK (COALESCE(json_type(state_json, '$.sidebar_open') IN ('true', 'false'), 0))
        CHECK (json_type(state_json, '$.collapsed_section_ids') IS 'array')
        CHECK (json_array_length(state_json, '$.collapsed_section_ids') <= 256)
        CHECK (json_type(state_json, '$.collapsed_project_ids') IS 'array')
        CHECK (json_array_length(state_json, '$.collapsed_project_ids') <= 256),
    updated_at INTEGER NOT NULL
);
"#;
