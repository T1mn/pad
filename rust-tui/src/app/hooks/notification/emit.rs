use crate::log_debug;
use crate::notify::NotificationRequest;

pub(in crate::app::hooks) fn emit_completion_notification(
    config: &crate::theme::Config,
    request: NotificationRequest,
) {
    match crate::notify::notify_completion(&request) {
        Ok(true) => {}
        Ok(false) => {
            log_debug!("notification: skipped (no supported desktop backend)");
        }
        Err(err) => {
            log_debug!("notification: failed to dispatch: {}", err);
        }
    }
    match crate::sound::play_event(&config.sound, crate::sound::SoundEvent::Completion) {
        Ok(true) => {}
        Ok(false) => {}
        Err(err) => {
            log_debug!("sound: failed to play completion sound: {}", err);
        }
    }
}
