//! SQL row mapping, validation, and section-write helpers.

use super::*;
use rusqlite::{OptionalExtension, Row, Transaction};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn task_select_sql(suffix: &str) -> String {
    format!(
        "SELECT id, project_id, profile_id, pi_session_id, session_file,
                title, summary, cwd, environment, status, leaf_id,
                unread, pinned, archived, policy_json, created_at, updated_at
         FROM tasks {suffix}"
    )
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "section write helper backs the staged Desktop organization CRUD API"
    )
)]
pub(super) fn insert_section_row(
    transaction: &Transaction<'_>,
    section: &Section,
) -> StoreResult<()> {
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

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "section item write helper backs the staged Desktop organization CRUD API"
    )
)]
pub(super) fn insert_section_items(
    transaction: &Transaction<'_>,
    section_id: &str,
    items: &[SectionItem],
) -> StoreResult<()> {
    for (item_order, item) in items.iter().enumerate() {
        insert_section_item(transaction, section_id, item, item_order as i64)?;
    }
    Ok(())
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "section item write helper backs the staged Desktop organization CRUD API"
    )
)]
pub(super) fn insert_section_item(
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

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "section validation helper backs the staged Desktop organization CRUD API"
    )
)]
pub(super) fn ensure_section_exists(connection: &Connection, id: &str) -> StoreResult<()> {
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

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "section item encoding helper backs the staged Desktop organization CRUD API"
    )
)]
pub(super) fn section_item_parts(item: &SectionItem) -> (&'static str, &str) {
    match item {
        SectionItem::Project(id) => ("project", id.as_str()),
        SectionItem::Task(id) => ("task", id.as_str()),
    }
}

pub(super) fn section_item_from_parts(kind: &str, id: String) -> SectionItem {
    if kind == "project" {
        SectionItem::Project(id)
    } else {
        SectionItem::Task(id)
    }
}

pub(super) fn map_profile(row: &Row<'_>) -> rusqlite::Result<Profile> {
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

pub(super) fn map_project(row: &Row<'_>) -> rusqlite::Result<Project> {
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

pub(super) fn map_task(row: &Row<'_>) -> rusqlite::Result<Task> {
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

pub(super) fn map_section(row: &Row<'_>) -> rusqlite::Result<Section> {
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

pub(super) fn decode_json_row<T: DeserializeOwned>(
    value: String,
    index: usize,
) -> rusqlite::Result<T> {
    json_decode(&value).map_err(|error| sqlite_conversion_error(index, error))
}

pub(super) fn decode_enum_row<T: DeserializeOwned>(
    value: String,
    index: usize,
) -> rusqlite::Result<T> {
    serde_json::from_value(Value::String(value))
        .map_err(|error| sqlite_conversion_error(index, error))
}

pub(super) fn decode_u64_row(value: i64, index: usize) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|error| sqlite_conversion_error(index, error))
}

pub(super) fn sqlite_conversion_error<E>(index: usize, error: E) -> rusqlite::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, Box::new(error))
}

pub(super) fn enum_encode<T: Serialize>(value: &T) -> StoreResult<String> {
    let value = serde_json::to_value(value)?;
    value.as_str().map(ToOwned::to_owned).ok_or_else(|| {
        StoreError::Serialization(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "expected a string enum",
        )))
    })
}

pub(super) fn bool_value(value: bool) -> i64 {
    i64::from(value)
}

pub(super) fn timestamp(value: u64) -> i64 {
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

pub(super) fn require_changed(changed: usize, kind: &'static str, id: &str) -> StoreResult<()> {
    if changed == 0 {
        Err(StoreError::NotFound {
            kind,
            id: id.to_string(),
        })
    } else {
        Ok(())
    }
}

pub(super) fn validate_optional_ui_state_id(field: &str, id: Option<&str>) -> StoreResult<()> {
    if let Some(id) = id {
        validate_ui_state_id(field, id)?;
    }
    Ok(())
}

pub(super) fn validate_collapsed_ids(field: &str, ids: &[String]) -> StoreResult<()> {
    if ids.len() > MAX_COLLAPSED_IDS_PER_GROUP {
        return Err(invalid_desktop_ui_state(format!(
            "{field} contains {} entries; maximum is {MAX_COLLAPSED_IDS_PER_GROUP}",
            ids.len()
        )));
    }

    let mut unique = HashSet::with_capacity(ids.len());
    for id in ids {
        validate_ui_state_id(field, id)?;
        if !unique.insert(id.as_str()) {
            return Err(invalid_desktop_ui_state(format!(
                "{field} contains duplicate id '{id}'"
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_ui_state_id(field: &str, id: &str) -> StoreResult<()> {
    if id.is_empty() || id.trim() != id {
        return Err(invalid_desktop_ui_state(format!(
            "{field} must contain a non-empty opaque id without surrounding whitespace"
        )));
    }
    if id.len() > MAX_UI_STATE_ID_BYTES {
        return Err(invalid_desktop_ui_state(format!(
            "{field} id exceeds the {MAX_UI_STATE_ID_BYTES}-byte limit"
        )));
    }
    if id.chars().any(char::is_control) || id.contains('/') || id.contains('\\') {
        return Err(invalid_desktop_ui_state(format!(
            "{field} must contain an opaque id, not a path or control characters"
        )));
    }
    Ok(())
}

pub(super) fn invalid_desktop_ui_state(message: impl Into<String>) -> StoreError {
    StoreError::Serialization(serde_json::Error::io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("invalid PAD Desktop UI state: {}", message.into()),
    )))
}
