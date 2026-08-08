use super::*;

mod context;
mod format;
mod status {
    use super::super::*;

    pub(crate) fn build_pad_status_body(
        locale: crate::i18n::Locale,
        pad_status: &str,
        state: &TelegramState,
    ) -> String {
        let target = state
            .selected_target
            .as_ref()
            .map(|target| target.label.clone())
            .unwrap_or_else(|| tg(locale, "status.none").to_string());
        let pending = if state.pending_requests.is_empty() {
            tg(locale, "status.pending_none").to_string()
        } else {
            let mut lines = String::new();
            for pending in &state.pending_requests {
                if !lines.is_empty() {
                    lines.push('\n');
                }
                lines.push_str(&pending_status_summary_line(locale, pending));
            }
            lines
        };
        format!(
            "{}: {}\n{}: {}\n{}:\n{}",
            tg(locale, "status.pad"),
            pad_status,
            tg(locale, "status.target"),
            target,
            tg(locale, "status.pending"),
            pending
        )
    }
}

use context::resolve_session_diag_context;
use format::format_session_diag_message;
pub(crate) use status::build_pad_status_body;

pub(crate) async fn send_session_diag(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
    arg: &str,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let context = resolve_session_diag_context(state, arg)?;
    let Some(context) = context else {
        let text = if arg.trim().is_empty() {
            tg(locale, "target.none")
        } else {
            tg(locale, "diag.empty")
        };
        send_text(&config.telegram.bot_token, chat_id, text).await?;
        return Ok(());
    };

    let body = format_session_diag_message(locale, &context);
    send_text(&config.telegram.bot_token, chat_id, &body).await?;
    Ok(())
}

pub(crate) async fn send_pad_status_report(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let pad_status = runtime_status::describe_status(&crate::paths::pad_status_path());
    let body = build_pad_status_body(locale, &pad_status, state);
    send_text(&config.telegram.bot_token, chat_id, &body).await?;
    Ok(())
}
