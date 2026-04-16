use super::*;

pub(crate) struct DraftFeedbackGate {
    pub(super) latest_seq: AtomicU64,
    pub(super) send_lock: AsyncMutex<()>,
}

pub(crate) fn refresh_pending_feedback(config: &Config, state: &mut TelegramState, force: bool) {
    let locale = telegram_locale(config);
    let now = now_ts();

    for pending in &mut state.pending_requests {
        if !force {
            let Some(accepted_at) = pending.accepted_at else {
                continue;
            };
            if accepted_at <= 0 {
                continue;
            }
            if let Some(last_status_at) = pending.last_status_at {
                if now.saturating_sub(last_status_at) < 4 {
                    continue;
                }
            }
        }

        spawn_pending_feedback_update(
            config.telegram.bot_token.clone(),
            pending.chat_id.clone(),
            pending.draft_id,
            pending_status_text(locale, pending, now),
            true,
            tg(locale, "typing.action").to_string(),
        );
        pending.last_status_at = Some(now);
    }
}

pub(crate) fn finalize_pending_feedback(config: &Config, pending: &PendingRequest, status: &str) {
    spawn_pending_feedback_update(
        config.telegram.bot_token.clone(),
        pending.chat_id.clone(),
        pending.draft_id,
        format!("{}\n{}", status, pending.target_label),
        false,
        String::new(),
    );
    let draft_id = pending.draft_id;
    tokio::spawn(async move {
        sleep(Duration::from_secs(5)).await;
        clear_draft_feedback_gate(draft_id);
    });
}

fn spawn_pending_feedback_update(
    token: String,
    chat_id: String,
    draft_id: i64,
    text: String,
    send_typing: bool,
    typing_action: String,
) {
    let gate = draft_feedback_gate(draft_id);
    let seq = gate.latest_seq.fetch_add(1, Ordering::SeqCst) + 1;
    tokio::spawn(async move {
        let _guard = gate.send_lock.lock().await;
        if gate.latest_seq.load(Ordering::SeqCst) != seq {
            return;
        }
        if send_typing {
            let _ = send_chat_action(&token, &chat_id, &typing_action).await;
        }
        let _ = send_message_draft(&token, &chat_id, draft_id, &text).await;
    });
}

fn draft_feedback_gate(draft_id: i64) -> Arc<DraftFeedbackGate> {
    let mut gates = DRAFT_FEEDBACK_GATES
        .lock()
        .expect("draft feedback gates lock");
    gates
        .entry(draft_id)
        .or_insert_with(|| {
            Arc::new(DraftFeedbackGate {
                latest_seq: AtomicU64::new(0),
                send_lock: AsyncMutex::new(()),
            })
        })
        .clone()
}

fn clear_draft_feedback_gate(draft_id: i64) {
    if let Ok(mut gates) = DRAFT_FEEDBACK_GATES.lock() {
        gates.remove(&draft_id);
    }
}
