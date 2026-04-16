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
        "/use" => {
            let idx = arg.parse::<usize>().ok().and_then(|n| n.checked_sub(1));
            let Some(idx) = idx else {
                send_text(&config.telegram.bot_token, chat_id, tg(locale, "use.usage")).await?;
                return Ok(());
            };
            let Some(entry) = state.agent_snapshot.get(idx).cloned() else {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    tg(locale, "use.invalid"),
                )
                .await?;
                return Ok(());
            };
            let panels = live_panels().map_err(telegram_error)?;
            if !panels.iter().any(|panel| panel.pane_id == entry.pane_id) {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    tg(locale, "pane.stale"),
                )
                .await?;
                return Ok(());
            }
            state.selected_target = Some(SelectedTarget {
                pane_id: entry.pane_id.clone(),
                label: entry.label.clone(),
            });
            send_text(
                &config.telegram.bot_token,
                chat_id,
                &tg_fmt(locale, "target.switched", entry.label),
            )
            .await?;
        }
        "/padstatus" => super::send_pad_status_report(config, state, chat_id).await?,
        "/history" => super::send_recent_history(config, state, chat_id).await?,
        "/diag" => super::send_session_diag(config, state, chat_id, arg).await?,
        "/restart" => {
            let plan = match restart::current_pad_restart_plan() {
                Ok(plan) => plan,
                Err(err_text) => {
                    send_text(
                        &config.telegram.bot_token,
                        chat_id,
                        &tg_fmt(locale, "restart.failed", err_text),
                    )
                    .await?;
                    play_sound_event(config, crate::sound::SoundEvent::Failure);
                    return Ok(());
                }
            };
            let preparing_key = match plan.target {
                PadRestartTarget::RespawnPane(_) => "restart.preparing",
                PadRestartTarget::NewDetachedSession(_) => "restart.starting",
            };
            send_text(
                &config.telegram.bot_token,
                chat_id,
                tg(locale, preparing_key),
            )
            .await?;
            if let Err(err_text) = restart::execute_pad_restart_plan(&plan) {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    &tg_fmt(locale, "restart.failed", err_text),
                )
                .await?;
                play_sound_event(config, crate::sound::SoundEvent::Failure);
            }
        }
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
        "/stop" => {
            let Some(target) = state.selected_target.as_ref() else {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    tg(locale, "target.none"),
                )
                .await?;
                return Ok(());
            };
            let stop_send_error = {
                let send_result = tmux_dispatch::send_escape(&target.pane_id);
                send_result.err().map(|err| err.to_string())
            };
            match stop_send_error {
                None => {
                    send_text(
                        &config.telegram.bot_token,
                        chat_id,
                        &tg_fmt(locale, "stop.sent", &target.label),
                    )
                    .await?;
                }
                Some(err_text) => {
                    send_text(
                        &config.telegram.bot_token,
                        chat_id,
                        &tg_fmt(locale, "stop.failed", err_text),
                    )
                    .await?;
                    play_sound_event(config, crate::sound::SoundEvent::Failure);
                }
            }
        }
        "/reset" => {
            let Some(target) = state.selected_target.as_ref() else {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    tg(locale, "target.none"),
                )
                .await?;
                return Ok(());
            };
            let target_label = target.label.clone();
            let Some(pending) = remove_selected_target_pending_request(state) else {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    &tg_fmt(locale, "reset.none", target_label),
                )
                .await?;
                return Ok(());
            };
            finalize_pending_feedback(config, &pending, tg(locale, "reset.status"));
            send_text(
                &config.telegram.bot_token,
                chat_id,
                &tg_fmt2(
                    locale,
                    "reset.done",
                    pending.request_id,
                    pending.target_label,
                ),
            )
            .await?;
        }
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
