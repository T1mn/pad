use super::*;

pub(crate) async fn process_pending_result_delivery(
    config: &Config,
    state: &mut TelegramState,
) -> TelegramResult<()> {
    let now = now_ts();
    let request_ids = state
        .pending_requests
        .iter()
        .filter(|pending| pending.phase == "delivering_result" && pending.delivery_retry_at <= now)
        .map(|pending| pending.request_id.clone())
        .collect::<Vec<_>>();

    for request_id in request_ids {
        if let Err(err) =
            deliver_pending_result(config, state, telegram_locale(config), &request_id).await
        {
            log_debug!(
                "telegram: result delivery retry failed request_id={} err={}",
                request_id,
                err
            );
        }
    }

    Ok(())
}

pub(crate) fn completed_reply_text(
    locale: crate::i18n::Locale,
    pending: &PendingRequest,
    result_text: &str,
) -> String {
    let mut reply = String::new();
    push_reply_line(&mut reply, tg(locale, "result.title"));
    push_reply_line(
        &mut reply,
        &format!("{}: {}", tg(locale, "meta.request"), pending.request_id),
    );
    push_reply_line(
        &mut reply,
        &format!("{}: {}", tg(locale, "meta.target"), pending.target_label),
    );
    push_reply_line(
        &mut reply,
        &format!("{}: {}", tg(locale, "meta.pane"), pending.pane_id),
    );
    if let Some(session_id) = pending
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        push_reply_line(
            &mut reply,
            &format!("{}: {}", tg(locale, "meta.session"), session_id),
        );
    }
    if let Some(turn_id) = pending.turn_id.as_deref().filter(|value| !value.is_empty()) {
        push_reply_line(
            &mut reply,
            &format!("{}: {}", tg(locale, "meta.turn"), turn_id),
        );
    }
    if !pending.working_dir.trim().is_empty() {
        push_reply_line(
            &mut reply,
            &format!("{}: {}", tg(locale, "meta.dir"), pending.working_dir),
        );
    }
    reply.push_str("\n\n");
    reply.push_str(result_text);
    reply
}

fn push_reply_line(out: &mut String, line: &str) {
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(line);
}

pub(crate) async fn deliver_pending_result(
    config: &Config,
    state: &mut TelegramState,
    locale: crate::i18n::Locale,
    request_id: &str,
) -> TelegramResult<()> {
    let Some(snapshot) = state
        .pending_requests
        .iter()
        .find(|pending| pending.request_id == request_id)
        .cloned()
    else {
        return Ok(());
    };
    if snapshot.phase != "delivering_result" {
        return Ok(());
    }

    let result_text = snapshot
        .completed_text
        .clone()
        .unwrap_or_else(|| tg(locale, "result.missing").to_string());
    let reply = completed_reply_text(locale, &snapshot, &result_text);
    match send_text(&config.telegram.bot_token, &snapshot.chat_id, &reply).await {
        Ok(()) => {
            finalize_pending_feedback(config, &snapshot, tg(locale, "phase.completed"));
            remove_pending_request(state, request_id);
            Ok(())
        }
        Err(err) => {
            if let Some(index) = pending_request_index_by_id(state, request_id) {
                let pending = &mut state.pending_requests[index];
                pending.delivery_attempts = pending.delivery_attempts.saturating_add(1);
                pending.delivery_retry_at = now_ts().saturating_add(RESULT_DELIVERY_RETRY_SECS);
                pending.last_status_at = None;
            }
            Err(err)
        }
    }
}
