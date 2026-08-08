mod diagnostic {
    use crate::session_continuity::model::ContinuityDiagnosticEvent;
    use crate::session_continuity::storage::with_continuity_lock;
    use std::fs::{self, OpenOptions};
    use std::io::Write;

    pub(in crate::session_continuity) fn append_diagnostic(event: &ContinuityDiagnosticEvent) {
        let _ = with_continuity_lock(|| append_diagnostic_unlocked(event));
    }

    fn append_diagnostic_unlocked(event: &ContinuityDiagnosticEvent) {
        let path = crate::paths::session_continuity_log_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match OpenOptions::new().create(true).append(true).open(path) {
            Ok(mut file) => {
                if let Ok(line) = serde_json::to_string(event) {
                    let _ = writeln!(file, "{}", line);
                }
            }
            Err(err) => {
                crate::log_debug!(
                    "session_continuity: failed to append diagnostic err={}",
                    err
                );
            }
        }
    }
}
mod ledger {
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
}
mod snapshot {
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
}

use super::model::SessionContinuityRecord;
use super::CONTINUITY_IO_LOCK;
use ledger::{load_ledger, save_ledger, upsert_record};
use std::sync::Mutex;

pub(super) use diagnostic::append_diagnostic;
pub(super) use snapshot::{load_record_snapshot, load_snapshot_for};

pub(super) fn mutate_record<F>(
    session_id: &str,
    now: i64,
    mut f: F,
) -> Option<SessionContinuityRecord>
where
    F: FnMut(&mut SessionContinuityRecord),
{
    with_continuity_lock(|| {
        let mut ledger = load_ledger();
        let snapshot = {
            let record = upsert_record(&mut ledger, session_id, now);
            f(record);
            record.clone()
        };
        if let Err(err) = save_ledger(&ledger) {
            crate::log_debug!(
                "session_continuity: failed to save ledger session_id={} err={}",
                session_id,
                err
            );
        }
        snapshot
    })
}

pub(in crate::session_continuity) fn with_continuity_lock<F, T>(f: F) -> Option<T>
where
    F: FnOnce() -> T,
{
    let _guard = CONTINUITY_IO_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .ok()?;
    Some(f())
}
