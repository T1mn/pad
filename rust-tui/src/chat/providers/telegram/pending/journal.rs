use super::*;

pub(crate) async fn process_hook_journal(
    config: &Config,
    state: &mut TelegramState,
) -> TelegramResult<()> {
    super::super::hooks::sync_state_from_disk_public(state);
    if state.pending_requests.is_empty() {
        state.journal_position = journal_len();
        return Ok(());
    }

    let path = crate::paths::hook_events_path();
    if !path.exists() {
        return Ok(());
    }

    let file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    if state.journal_position > len {
        state.journal_position = len;
    }
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(state.journal_position))?;

    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        state.journal_position += line.len() as u64;
        super::super::hooks::sync_state_from_disk_public(state);
        if state.pending_requests.is_empty() {
            line.clear();
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }
        match serde_json::from_str::<HookEvent>(trimmed) {
            Ok(event) => {
                if !remember_processed_hook_event(state, &event) {
                    line.clear();
                    continue;
                }
                let _ = apply_hook_event_to_pending(config, state, &event).await?;
            }
            Err(err) => {
                log_debug!("telegram: invalid hook journal line: {}", err);
            }
        }
        line.clear();
    }

    Ok(())
}
