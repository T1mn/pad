use super::*;

mod detect;
mod reply;

pub(super) use detect::{
    detect_pending_rollout_failure_for_request, pending_rollout_failure_check_due,
};
pub(super) use reply::pending_failure_reply_text;

pub(crate) async fn process_pending_rollout_failures(
    config: &Config,
    state: &mut TelegramState,
) -> TelegramResult<()> {
    let now = now_ts();
    let request_ids = state
        .pending_requests
        .iter()
        .filter(|pending| pending_rollout_failure_check_due(pending, now))
        .map(|pending| pending.request_id.clone())
        .collect::<Vec<_>>();

    for request_id in request_ids {
        let resolution = match detect_pending_rollout_failure_for_request(state, &request_id, now) {
            Ok(resolution) => resolution,
            Err(err) => {
                log_debug!(
                    "telegram: rollout failure detection failed request_id={} err={}",
                    request_id,
                    err
                );
                continue;
            }
        };
        let Some(resolution) = resolution else {
            continue;
        };

        let locale = telegram_locale(config);
        finalize_pending_feedback(config, &resolution.pending, tg(locale, "phase.failed"));
        let reply = pending_failure_reply_text(
            locale,
            &resolution.pending,
            &resolution.failure,
            resolution.continuity.as_ref(),
        );
        if let Err(err) = send_text(
            &config.telegram.bot_token,
            &resolution.pending.chat_id,
            &reply,
        )
        .await
        {
            log_debug!(
                "telegram: rollout failure notification failed request_id={} err={}",
                resolution.pending.request_id,
                err
            );
        }
        play_sound_event(config, crate::sound::SoundEvent::Failure);
        log_debug!(
            "telegram: rollout failure released pending request={} pane={} error_info={} message={}",
            resolution.pending.request_id,
            resolution.pending.pane_id,
            resolution
                .failure
                .error_info
                .as_deref()
                .unwrap_or("unknown"),
            truncate_for_log(&resolution.failure.message, 240)
        );
    }

    Ok(())
}
