use crate::session_continuity::model::{ContinuityLedger, SessionContinuityRecord};
use crate::session_continuity::utils::now_ts;
use crate::session_continuity::CONTINUITY_VERSION;
use std::fs;

pub(super) fn load_ledger() -> ContinuityLedger {
    let path = crate::paths::session_continuity_state_path();
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => return default_ledger(),
    };

    serde_json::from_str(&content).unwrap_or_else(|err| {
        crate::log_debug!(
            "session_continuity: failed to parse {}: {}",
            path.display(),
            err
        );
        default_ledger()
    })
}

pub(super) fn save_ledger(ledger: &ContinuityLedger) -> std::io::Result<()> {
    let path = crate::paths::session_continuity_state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = path.with_extension(format!("tmp.{}.{}", std::process::id(), now_ts()));
    fs::write(&tmp_path, serde_json::to_string_pretty(ledger)?)?;
    fs::rename(&tmp_path, &path)
}

pub(super) fn upsert_record<'a>(
    ledger: &'a mut ContinuityLedger,
    session_id: &str,
    now: i64,
) -> &'a mut SessionContinuityRecord {
    ledger.version = CONTINUITY_VERSION;
    if let Some(index) = ledger
        .sessions
        .iter()
        .position(|record| record.session_id == session_id)
    {
        return &mut ledger.sessions[index];
    }

    ledger
        .sessions
        .push(SessionContinuityRecord::new(session_id, now));
    ledger
        .sessions
        .last_mut()
        .expect("session continuity record")
}

fn default_ledger() -> ContinuityLedger {
    ContinuityLedger {
        version: CONTINUITY_VERSION,
        ..ContinuityLedger::default()
    }
}
