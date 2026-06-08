use super::ledger::load_ledger;
use crate::session_continuity::model::{ContinuitySnapshot, SessionContinuityRecord};
use crate::session_continuity::storage::with_continuity_lock;
use crate::session_continuity::utils::clean_text;
use std::fs;

pub(in crate::session_continuity) fn load_snapshot_for(
    session_id: Option<&str>,
    transcript_path: Option<&str>,
) -> Option<ContinuitySnapshot> {
    if let Some(session_id) = session_id.and_then(|value| clean_text(Some(value))) {
        if let Some(snapshot) = load_snapshot_by_session_id(session_id) {
            return Some(snapshot);
        }
    }

    let transcript_path = transcript_path.and_then(|value| clean_text(Some(value)))?;
    with_continuity_lock(|| {
        let ledger = load_ledger();
        ledger
            .sessions
            .into_iter()
            .find(|record| {
                record
                    .transcript_path
                    .as_deref()
                    .map(|path| same_path_str(path, transcript_path))
                    .unwrap_or(false)
            })
            .map(Into::into)
    })
    .flatten()
}

pub(in crate::session_continuity) fn load_record_snapshot(
    session_id: &str,
) -> Option<SessionContinuityRecord> {
    with_continuity_lock(|| {
        let ledger = load_ledger();
        ledger
            .sessions
            .into_iter()
            .find(|record| record.session_id == session_id)
    })
    .flatten()
}

fn load_snapshot_by_session_id(session_id: &str) -> Option<ContinuitySnapshot> {
    let session_id = clean_text(Some(session_id))?;
    load_record_snapshot(session_id).map(Into::into)
}

fn same_path_str(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }

    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}
