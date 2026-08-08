pub(super) use super::*;

mod approval;
mod failures;
mod feedback;
mod journal;
mod results;
mod status;
mod timeouts {
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
}
mod timing {
    use super::*;

    pub(crate) fn pending_sent_ms(pending: &PendingRequest) -> i64 {
        if pending.sent_at_ms > 0 {
            pending.sent_at_ms
        } else {
            pending.sent_at.saturating_mul(1000)
        }
    }

    pub(crate) fn pending_accepted_ms(pending: &PendingRequest) -> i64 {
        pending.accepted_at_ms.unwrap_or_else(|| {
            pending
                .accepted_at
                .unwrap_or(pending.sent_at)
                .saturating_mul(1000)
        })
    }
}

pub(super) use approval::process_codex_pending_approval;
pub(super) use failures::process_pending_rollout_failures;
pub(super) use feedback::{finalize_pending_feedback, refresh_pending_feedback, DraftFeedbackGate};
pub(super) use journal::process_hook_journal;
#[cfg(test)]
pub(super) use results::completed_reply_text;
pub(super) use results::{deliver_pending_result, process_pending_result_delivery};
pub(super) use status::{
    continuity_detail_lines, pending_status_summary_line, pending_status_text,
};
pub(super) use timeouts::process_pending_timeout;
pub(super) use timing::{pending_accepted_ms, pending_sent_ms};

#[cfg(test)]
mod tests;
