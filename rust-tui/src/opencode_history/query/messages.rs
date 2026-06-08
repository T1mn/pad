use super::super::util::to_io_error;
use crate::opencode_text::{extract_any_part_text, message_role, OpenCodeRole};
use std::collections::{HashMap, HashSet};
use std::io;

#[derive(Default)]
pub(super) struct MessageSummary {
    pub(super) first_user: Option<String>,
    pub(super) last_user: Option<String>,
    pub(super) last_assistant: Option<String>,
}

pub(super) fn load_message_summaries(
    connection: &rusqlite::Connection,
    session_ids: &HashSet<&str>,
) -> io::Result<HashMap<String, MessageSummary>> {
    let mut statement = connection
        .prepare(
            "SELECT m.session_id, m.data, p.data
             FROM message m
             LEFT JOIN part p ON p.message_id = m.id
             ORDER BY m.time_created ASC, p.time_created ASC, p.id ASC",
        )
        .map_err(to_io_error)?;
    let mut rows = statement.query([]).map_err(to_io_error)?;
    let mut summaries: HashMap<String, MessageSummary> = HashMap::new();
    while let Some(row) = rows.next().map_err(to_io_error)? {
        let session_id: String = row.get(0).map_err(to_io_error)?;
        let message_data: String = row.get(1).map_err(to_io_error)?;
        let part_data: Option<String> = row.get(2).map_err(to_io_error)?;
        if session_ids.contains(session_id.as_str()) {
            apply_part(
                &mut summaries,
                session_id,
                &message_data,
                part_data.as_deref(),
            );
        }
    }
    Ok(summaries)
}

pub(super) fn load_message_summaries_for_session(
    connection: &rusqlite::Connection,
    session_id: &str,
) -> io::Result<HashMap<String, MessageSummary>> {
    let mut statement = connection
        .prepare(
            "SELECT m.session_id, m.data, p.data
             FROM message m
             LEFT JOIN part p ON p.message_id = m.id
             WHERE m.session_id = ?1
             ORDER BY m.time_created ASC, p.time_created ASC, p.id ASC",
        )
        .map_err(to_io_error)?;
    let mut rows = statement.query([session_id]).map_err(to_io_error)?;
    let mut summaries: HashMap<String, MessageSummary> = HashMap::new();
    while let Some(row) = rows.next().map_err(to_io_error)? {
        let session_id: String = row.get(0).map_err(to_io_error)?;
        let message_data: String = row.get(1).map_err(to_io_error)?;
        let part_data: Option<String> = row.get(2).map_err(to_io_error)?;
        apply_part(
            &mut summaries,
            session_id,
            &message_data,
            part_data.as_deref(),
        );
    }
    Ok(summaries)
}

fn apply_part(
    summaries: &mut HashMap<String, MessageSummary>,
    session_id: String,
    message_data: &str,
    part_data: Option<&str>,
) {
    let Some(role) = message_role(message_data) else {
        return;
    };
    let text = part_data
        .and_then(extract_any_part_text)
        .unwrap_or_default();
    if text.trim().is_empty() {
        return;
    }
    let summary = summaries.entry(session_id).or_default();
    match role {
        OpenCodeRole::User => {
            if summary.first_user.is_none() {
                summary.first_user = Some(text.clone());
            }
            summary.last_user = Some(text);
        }
        OpenCodeRole::Assistant => summary.last_assistant = Some(text),
    }
}
