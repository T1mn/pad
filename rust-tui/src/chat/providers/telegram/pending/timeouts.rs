use super::*;

pub(crate) async fn process_pending_timeout(
    config: &Config,
    state: &mut TelegramState,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let now = now_ts();
    let timed_out = state
        .pending_requests
        .iter()
        .filter(|pending| now.saturating_sub(pending.sent_at) >= PENDING_TIMEOUT_SECS)
        .cloned()
        .collect::<Vec<_>>();

    for pending in timed_out {
        remove_pending_request(state, &pending.request_id);
        finalize_pending_feedback(config, &pending, tg(locale, "phase.completed"));
        send_text(
            &config.telegram.bot_token,
            &pending.chat_id,
            &tg_fmt(locale, "timeout", &pending.request_id),
        )
        .await?;
        play_sound_event(config, crate::sound::SoundEvent::Timeout);
    }

    Ok(())
}
