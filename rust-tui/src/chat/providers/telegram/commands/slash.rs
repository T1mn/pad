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

    let Some(target) = state.selected_target.as_ref() else {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "target.none"),
        )
        .await?;
        return Ok(());
    };

    let panels = live_panels().map_err(telegram_error)?;
    let Some(panel) = panels.iter().find(|panel| panel.pane_id == target.pane_id) else {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "pane.stale"),
        )
        .await?;
        return Ok(());
    };

    if !matches!(&panel.agent_type, &AgentType::Codex) {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "target.not_codex"),
        )
        .await?;
        return Ok(());
    }

    if panel.state == AgentState::Busy {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "agent.busy"),
        )
        .await?;
        return Ok(());
    }
    if panel.state == AgentState::Waiting {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "agent.waiting"),
        )
        .await?;
        return Ok(());
    }

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

    let reply = match poll_slash_reply(&panel.pane_id, &slash, &baseline, deadline_ms).await {
        Ok(Some(capture)) => {
            if capture.is_empty() {
                tg_fmt2(locale, "slash.sent", &slash, compact_target_label(panel))
            } else {
                tg_fmt3(
                    locale,
                    "slash.output",
                    &slash,
                    compact_target_label(panel),
                    capture,
                )
            }
        }
        Ok(None) => tg_fmt2(locale, "slash.sent", &slash, compact_target_label(panel)),
        Err(err) => {
            log_debug!(
                "telegram: capture after slash command failed pane={} command={} err={}",
                panel.pane_id,
                slash,
                err
            );
            tg_fmt2(locale, "slash.sent", &slash, compact_target_label(panel))
        }
    };
    send_text(&config.telegram.bot_token, chat_id, &reply).await?;
    Ok(())
}

pub(crate) fn capture_looks_like_echo_only(capture: &str, slash: &str) -> bool {
    let trimmed = capture.trim();
    trimmed == slash.trim() || trimmed.ends_with(&format!("\n{}", slash.trim()))
}

pub(crate) async fn poll_slash_reply(
    pane_id: &str,
    slash: &str,
    baseline: &str,
    deadline_ms: u64,
) -> TelegramResult<Option<String>> {
    let started = Instant::now();
    let deadline = Duration::from_millis(deadline_ms);
    let mut candidate: Option<String> = None;
    let mut stable_hits = 0usize;

    loop {
        let capture = tmux_dispatch::capture_pane_tail(pane_id, 28).map_err(telegram_error)?;
        let capture = summarize_pane_capture(&capture);
        if !capture.is_empty() && capture != baseline {
            if !capture_looks_like_echo_only(&capture, slash) {
                if candidate.as_deref() == Some(capture.as_str()) {
                    stable_hits += 1;
                } else {
                    candidate = Some(capture.clone());
                    stable_hits = 1;
                }
                if stable_hits >= 2 || started.elapsed() >= Duration::from_millis(250) {
                    return Ok(Some(capture));
                }
            } else {
                candidate = Some(capture);
            }
        }

        if started.elapsed() >= deadline {
            break;
        }
        sleep(Duration::from_millis(SLASH_POLL_INTERVAL_MS)).await;
    }

    Ok(candidate.filter(|capture| capture != baseline))
}
