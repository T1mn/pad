use super::*;

mod context;
mod format;
mod status;

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
