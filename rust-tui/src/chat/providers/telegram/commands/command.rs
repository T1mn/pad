#[path = "command/reset.rs"]
mod reset;
#[path = "command/restart_cmd.rs"]
mod restart_cmd;
#[path = "command/stop.rs"]
mod stop;
#[path = "command/use_target.rs"]
mod use_target;

use super::*;

pub(crate) async fn handle_command(
    config: &Config,
    state: &mut TelegramState,
    chat_id: &str,
    text: &str,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let mut parts = text.trim().splitn(2, ' ');
    let command = parts.next().unwrap_or_default();
    let arg = parts.next().unwrap_or_default().trim();

    match command {
        "/start" => {
            send_text(
                &config.telegram.bot_token,
                chat_id,
                tg(locale, "start.ready"),
            )
            .await?;
        }
        "/help" => super::send_help_message(config, state, chat_id, HelpPage::Overview).await?,
        "/list" | "/agents" => super::send_agent_list(config, state, chat_id).await?,
        "/use" => use_target::handle_use_command(config, state, chat_id, arg).await?,
        "/padstatus" => super::send_pad_status_report(config, state, chat_id).await?,
        "/history" => super::send_recent_history(config, state, chat_id).await?,
        "/diag" => super::send_session_diag(config, state, chat_id, arg).await?,
        "/restart" => restart_cmd::handle_restart_command(config, chat_id).await?,
        "/status" => {
            if matches!(arg, "pad" | "bot") {
                super::send_pad_status_report(config, state, chat_id).await?;
            } else {
                super::dispatch_codex_slash_command(config, state, chat_id, "/status", arg, 1000)
                    .await?;
            }
        }
        "/fast" => {
            super::dispatch_codex_slash_command(config, state, chat_id, "/fast", arg, 1200).await?;
        }
        "/compact" => {
            super::dispatch_codex_slash_command(config, state, chat_id, "/compact", arg, 2000)
                .await?;
        }
        "/stop" => stop::handle_stop_command(config, state, chat_id).await?,
        "/reset" => reset::handle_reset_command(config, state, chat_id).await?,
        _ => {
            send_text(
                &config.telegram.bot_token,
                chat_id,
                tg(locale, "unknown.command"),
            )
            .await?;
        }
    }

    Ok(())
}
