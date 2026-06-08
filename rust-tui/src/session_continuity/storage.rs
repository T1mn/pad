mod diagnostic;
mod ledger;
mod snapshot;

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
