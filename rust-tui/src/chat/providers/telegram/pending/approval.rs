mod candidates;
mod notify;
mod scan;
mod transcript;

use super::*;

pub(crate) async fn process_codex_pending_approval(
    config: &Config,
    state: &mut TelegramState,
) -> TelegramResult<()> {
    for request_id in candidates::approval_request_ids(state) {
        process_codex_pending_approval_for_request(config, state, &request_id).await?;
    }
    Ok(())
}

async fn process_codex_pending_approval_for_request(
    config: &Config,
    state: &mut TelegramState,
    request_id: &str,
) -> TelegramResult<()> {
    let Some(snapshot) = candidates::approval_snapshot(state, request_id) else {
        return Ok(());
    };

    let Some(transcript_path) =
        transcript::ensure_approval_transcript_path(state, request_id, &snapshot)?
    else {
        return Ok(());
    };

    let outcome =
        scan::scan_and_apply_approval_state(state, request_id, &snapshot, &transcript_path)?;
    if !outcome.changed {
        return Ok(());
    }

    notify::notify_approval_change(config, state, request_id, &snapshot, outcome).await
}
