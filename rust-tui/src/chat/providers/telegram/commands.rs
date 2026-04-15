use super::*;

pub(super) async fn handle_update(
    config: &mut Config,
    state: &mut TelegramState,
    update: TelegramUpdate,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    if let Some(callback_query) = update.callback_query {
        return handle_callback_query(config, state, callback_query).await;
    }

    let Some(message) = update.message else {
        return Ok(());
    };

    if message.chat.kind != "private" {
        log_debug!("telegram: ignoring non-private chat {}", message.chat.id);
        return Ok(());
    }

    let chat_id = message.chat.id.to_string();
    let text = message.text.unwrap_or_default();
    log_debug!(
        "telegram: incoming message chat={} msg_id={} text={}",
        chat_id,
        message.message_id,
        truncate_for_log(&text, 200)
    );

    if config.telegram.chat_id.is_empty() {
        if text.starts_with("/start") {
            config.telegram.chat_id = chat_id.clone();
            config.save();
            send_text(
                &config.telegram.bot_token,
                &chat_id,
                tg(locale, "bind.success"),
            )
            .await?;
        } else {
            send_text(
                &config.telegram.bot_token,
                &chat_id,
                tg(locale, "bind.start_required"),
            )
            .await?;
        }
        return Ok(());
    }

    if config.telegram.chat_id != chat_id {
        send_text(
            &config.telegram.bot_token,
            &chat_id,
            tg(locale, "bind.other_chat"),
        )
        .await?;
        return Ok(());
    }

    if text.starts_with('/') {
        handle_command(config, state, &chat_id, &text).await?;
    } else {
        handle_plain_text(config, state, &chat_id, &text).await?;
    }

    Ok(())
}

pub(super) async fn handle_command(
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
        "/help" => {
            send_help_message(config, state, chat_id, HelpPage::Overview).await?;
        }
        "/list" | "/agents" => send_agent_list(config, state, chat_id).await?,
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
        "/padstatus" => {
            send_pad_status_report(config, state, chat_id).await?;
        }
        "/status" => {
            if matches!(arg, "pad" | "bot") {
                send_pad_status_report(config, state, chat_id).await?;
            } else {
                dispatch_codex_slash_command(config, state, chat_id, "/status", arg, 1000).await?;
            }
        }
        "/fast" => {
            dispatch_codex_slash_command(config, state, chat_id, "/fast", arg, 1200).await?;
        }
        "/compact" => {
            dispatch_codex_slash_command(config, state, chat_id, "/compact", arg, 2000).await?;
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

pub(super) async fn send_pad_status_report(
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

pub(super) fn build_pad_status_body(
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
        state
            .pending_requests
            .iter()
            .map(|pending| pending_status_summary_line(locale, pending))
            .collect::<Vec<_>>()
            .join("\n")
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

pub(super) async fn dispatch_codex_slash_command(
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

pub(super) async fn handle_plain_text(
    config: &Config,
    state: &mut TelegramState,
    chat_id: &str,
    text: &str,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    if text.trim().is_empty() {
        return Ok(());
    }

    if !pad_is_online() {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "pad.offline"),
        )
        .await?;
        return Ok(());
    }

    let Some(target) = state.selected_target.clone() else {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "target.none"),
        )
        .await?;
        return Ok(());
    };
    if pending_request_index_by_pane(state, &target.pane_id).is_some() {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "pending.exists"),
        )
        .await?;
        return Ok(());
    }

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

    tmux_dispatch::dispatch_prompt(&panel.pane_id, text).map_err(telegram_error)?;
    invalidate_live_panels();
    let request_id = next_request_id();
    let transcript_path = panel.transcript_path.clone();
    let result_scan_offset = transcript_path.as_deref().map(transcript_len).unwrap_or(0);
    let approval_scan_offset = transcript_path.as_deref().map(transcript_len).unwrap_or(0);
    let sent_at = now_ts();
    let sent_at_ms = now_ms_i64();
    state.pending_requests.push(PendingRequest {
        request_id: request_id.clone(),
        chat_id: chat_id.to_string(),
        pane_id: panel.pane_id.clone(),
        agent_kind: panel.agent_type.to_string(),
        target_label: compact_target_label(panel),
        session_id: panel.agent_session_id.clone(),
        working_dir: panel.working_dir.clone(),
        prompt_text: text.to_string(),
        prompt_hash: format!("{:x}", md5::compute(text.as_bytes())),
        turn_id: None,
        sent_at,
        sent_at_ms,
        accepted_at: None,
        accepted_at_ms: None,
        last_status_at: None,
        draft_id: next_draft_id(),
        phase: "awaiting_submit".to_string(),
        transcript_path,
        result_scan_offset,
        approval_scan_offset,
        approval_call_id: None,
        approval_justification: None,
        completed_text: None,
        completed_source: None,
        delivery_attempts: 0,
        delivery_retry_at: 0,
    });
    save_state(state)?;
    log_debug!(
        "telegram: prompt dispatched request_id={} pane={} chat={}",
        request_id,
        panel.pane_id,
        chat_id
    );
    refresh_pending_feedback(config, state, true);
    Ok(())
}

pub(super) async fn send_help_message(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
    page: HelpPage,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let _: serde_json::Value = send_message(
        &config.telegram.bot_token,
        &help_message_payload(locale, state, telegram_chat_id_value(chat_id), None, page),
    )
    .await?;
    Ok(())
}

pub(super) async fn edit_help_message(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
    message_id: i64,
    page: HelpPage,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let _: serde_json::Value = edit_message(
        &config.telegram.bot_token,
        &help_message_payload(
            locale,
            state,
            telegram_chat_id_value(chat_id),
            Some(message_id),
            page,
        ),
    )
    .await?;
    Ok(())
}

pub(super) async fn send_agent_list(
    config: &Config,
    state: &mut TelegramState,
    chat_id: &str,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let panels = live_panels().map_err(telegram_error)?;
    let snapshot = panels
        .iter()
        .enumerate()
        .map(|(idx, panel)| AgentSnapshotEntry {
            pane_id: panel.pane_id.clone(),
            label: format_agent_line(idx + 1, panel, locale),
        })
        .collect::<Vec<_>>();
    state.agent_snapshot = snapshot.clone();

    if snapshot.is_empty() {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "list.empty"),
        )
        .await?;
        return Ok(());
    }

    let body = snapshot
        .iter()
        .map(|entry| entry.label.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let keyboard = build_agent_keyboard(&panels, locale);
    send_message(
        &config.telegram.bot_token,
        &json!({
            "chat_id": chat_id,
            "text": body,
            "reply_markup": {
                "inline_keyboard": keyboard
            }
        }),
    )
    .await?;
    Ok(())
}

pub(super) fn capture_looks_like_echo_only(capture: &str, slash: &str) -> bool {
    let trimmed = capture.trim();
    trimmed == slash.trim() || trimmed.ends_with(&format!("\n{}", slash.trim()))
}

pub(super) async fn poll_slash_reply(
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
