mod poll;
mod target;

use super::*;

pub(crate) async fn dispatch_codex_slash_command(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
    command: &str,
    arg: &str,
    deadline_ms: u64,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    if !pad_is_online() {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "pad.offline"),
        )
        .await?;
        return Ok(());
    }

    let Some(panel) = target::resolve_codex_slash_panel(config, state, chat_id, locale).await?
    else {
        return Ok(());
    };

    let slash = build_slash_command_text(command, arg);
    let baseline = tmux_dispatch::capture_pane_tail(&panel.pane_id, 28)
        .map(|capture| summarize_pane_capture(&capture))
        .unwrap_or_default();
    tmux_dispatch::dispatch_prompt(&panel.pane_id, &slash).map_err(telegram_error)?;
    invalidate_live_panels();
    log_debug!(
        "telegram: dispatched codex slash command pane={} command={}",
        panel.pane_id,
        slash
    );

    let reply = match poll::poll_slash_reply(&panel.pane_id, &slash, &baseline, deadline_ms).await {
        Ok(Some(capture)) => slash_reply_with_capture(locale, &slash, &panel, &capture),
        Ok(None) => slash_sent_reply(locale, &slash, &panel),
        Err(err) => {
            log_debug!(
                "telegram: capture after slash command failed pane={} command={} err={}",
                panel.pane_id,
                slash,
                err
            );
            slash_sent_reply(locale, &slash, &panel)
        }
    };
    send_text(&config.telegram.bot_token, chat_id, &reply).await?;
    Ok(())
}

fn slash_reply_with_capture(
    locale: crate::i18n::Locale,
    slash: &str,
    panel: &AgentPanel,
    capture: &str,
) -> String {
    if capture.is_empty() {
        slash_sent_reply(locale, slash, panel)
    } else {
        tg_fmt3(
            locale,
            "slash.output",
            slash,
            compact_target_label(panel),
            capture,
        )
    }
}

fn slash_sent_reply(locale: crate::i18n::Locale, slash: &str, panel: &AgentPanel) -> String {
    tg_fmt2(locale, "slash.sent", slash, compact_target_label(panel))
}
