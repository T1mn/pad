use super::super::super::*;
use super::super::approval::{
    approval_pending_index, approval_sent_text, parse_approval_callback_data,
};

pub(super) async fn handle_approval_callback(
    config: &Config,
    state: &mut TelegramState,
    query_id: &str,
    data: &str,
    locale: crate::i18n::Locale,
) -> TelegramResult<()> {
    let Some((request_id, choice)) = parse_approval_callback_data(data) else {
        answer_callback_query(
            &config.telegram.bot_token,
            query_id,
            Some(tg(locale, "approval.none")),
        )
        .await?;
        return Ok(());
    };
    let Some(pending_index) = approval_pending_index(state, request_id) else {
        answer_callback_query(
            &config.telegram.bot_token,
            query_id,
            Some(tg(locale, "approval.none")),
        )
        .await?;
        return Ok(());
    };
    let pending_snapshot = state.pending_requests[pending_index].clone();
    let key = match choice {
        "y" => "y",
        "a" => "a",
        "n" => "n",
        _ => {
            answer_callback_query(
                &config.telegram.bot_token,
                query_id,
                Some(tg(locale, "callback.unknown")),
            )
            .await?;
            return Ok(());
        }
    };
    let approval_send_error = {
        let send_result = tmux_dispatch::send_approval_key(&pending_snapshot.pane_id, key);
        send_result.err().map(|err| err.to_string())
    };
    match approval_send_error {
        None => {
            invalidate_live_panels();
            let pending = &mut state.pending_requests[pending_index];
            pending.phase = "awaiting_stop".to_string();
            pending.approval_call_id = None;
            pending.approval_justification = None;
            pending.last_status_at = None;
            refresh_pending_feedback(config, state, true);
            answer_callback_query(
                &config.telegram.bot_token,
                query_id,
                Some(approval_sent_text(locale, choice)),
            )
            .await?;
        }
        Some(err_text) => {
            answer_callback_query(
                &config.telegram.bot_token,
                query_id,
                Some(&tg_fmt(locale, "approval.failed", err_text)),
            )
            .await?;
            play_sound_event(config, crate::sound::SoundEvent::Failure);
        }
    }
    Ok(())
}
