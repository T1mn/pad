use super::*;

pub(super) fn approval_callback_data(request_id: &str, choice: &str) -> String {
    format!("approval:{}:{}", request_id, choice)
}

pub(super) fn parse_approval_callback_data(data: &str) -> Option<(&str, &str)> {
    let rest = data.strip_prefix("approval:")?;
    let (request_id, choice) = rest.rsplit_once(':')?;
    if request_id.is_empty() || choice.is_empty() {
        return None;
    }
    Some((request_id, choice))
}

pub(super) fn approval_pending_index(state: &TelegramState, request_id: &str) -> Option<usize> {
    state.pending_requests.iter().position(|pending| {
        pending.request_id == request_id
            && pending.agent_kind == "codex"
            && pending.approval_call_id.is_some()
    })
}

pub(super) async fn handle_callback_query(
    config: &Config,
    state: &mut TelegramState,
    query: TelegramCallbackQuery,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let Some(message) = query.message else {
        answer_callback_query(
            &config.telegram.bot_token,
            &query.id,
            Some(tg(locale, "callback.invalid")),
        )
        .await?;
        return Ok(());
    };
    if message.chat.kind != "private" {
        answer_callback_query(
            &config.telegram.bot_token,
            &query.id,
            Some(tg(locale, "callback.private_only")),
        )
        .await?;
        return Ok(());
    }

    let chat_id = message.chat.id.to_string();
    if !config.telegram.chat_id.is_empty() && config.telegram.chat_id != chat_id {
        answer_callback_query(
            &config.telegram.bot_token,
            &query.id,
            Some(tg(locale, "callback.bound_other")),
        )
        .await?;
        return Ok(());
    }

    let Some(data) = query.data.as_deref() else {
        answer_callback_query(
            &config.telegram.bot_token,
            &query.id,
            Some(tg(locale, "callback.no_data")),
        )
        .await?;
        return Ok(());
    };

    if let Some(page) = HelpPage::from_callback(data) {
        edit_help_message(config, state, &chat_id, message.message_id, page).await?;
        answer_callback_query(&config.telegram.bot_token, &query.id, None).await?;
    } else if data == "help:list" {
        send_agent_list(config, state, &chat_id).await?;
        answer_callback_query(&config.telegram.bot_token, &query.id, None).await?;
    } else if data == "help:padstatus" {
        send_pad_status_report(config, state, &chat_id).await?;
        answer_callback_query(&config.telegram.bot_token, &query.id, None).await?;
    } else if let Some(pane_id) = data.strip_prefix("use-pane:") {
        let panels = live_panels().map_err(telegram_error)?;
        if let Some(panel) = panels.iter().find(|panel| panel.pane_id == pane_id) {
            let selected = SelectedTarget {
                pane_id: panel.pane_id.clone(),
                label: format_agent_line_for_button(panel, locale),
            };
            state.selected_target = Some(selected.clone());
            answer_callback_query(
                &config.telegram.bot_token,
                &query.id,
                Some(tg(locale, "callback.switched")),
            )
            .await?;
            send_text(
                &config.telegram.bot_token,
                &chat_id,
                &tg_fmt(locale, "target.switched", selected.label),
            )
            .await?;
        } else {
            answer_callback_query(
                &config.telegram.bot_token,
                &query.id,
                Some(tg(locale, "callback.stale")),
            )
            .await?;
        }
    } else if data.starts_with("approval:") {
        let Some((request_id, choice)) = parse_approval_callback_data(data) else {
            answer_callback_query(
                &config.telegram.bot_token,
                &query.id,
                Some(tg(locale, "approval.none")),
            )
            .await?;
            return Ok(());
        };
        let Some(pending_index) = approval_pending_index(state, request_id) else {
            answer_callback_query(
                &config.telegram.bot_token,
                &query.id,
                Some(tg(locale, "approval.none")),
            )
            .await?;
            return Ok(());
        };
        let pending_snapshot = state.pending_requests[pending_index].clone();
        let key = match choice {
            "y" => "y",
            "a" => "a",
            "n" => "n",
            _ => {
                answer_callback_query(
                    &config.telegram.bot_token,
                    &query.id,
                    Some(tg(locale, "callback.unknown")),
                )
                .await?;
                return Ok(());
            }
        };
        let approval_send_error = {
            let send_result = tmux_dispatch::send_approval_key(&pending_snapshot.pane_id, key);
            send_result.err().map(|err| err.to_string())
        };
        match approval_send_error {
            None => {
                invalidate_live_panels();
                let pending = &mut state.pending_requests[pending_index];
                pending.phase = "awaiting_stop".to_string();
                pending.approval_call_id = None;
                pending.approval_justification = None;
                pending.last_status_at = None;
                refresh_pending_feedback(config, state, true);
                answer_callback_query(
                    &config.telegram.bot_token,
                    &query.id,
                    Some(approval_sent_text(locale, choice)),
                )
                .await?;
            }
            Some(err_text) => {
                answer_callback_query(
                    &config.telegram.bot_token,
                    &query.id,
                    Some(&tg_fmt(locale, "approval.failed", err_text)),
                )
                .await?;
                play_sound_event(config, crate::sound::SoundEvent::Failure);
            }
        }
    } else {
        answer_callback_query(
            &config.telegram.bot_token,
            &query.id,
            Some(tg(locale, "callback.unknown")),
        )
        .await?;
    }

    Ok(())
}

pub(super) async fn send_codex_approval_prompt(
    config: &Config,
    chat_id: &str,
    pending: &PendingRequest,
    request: &CodexApprovalRequest,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let mut lines = vec![
        tg(locale, "approval.prompt").to_string(),
        format!(
            "{}: {}",
            tg(locale, "approval.target"),
            pending.target_label
        ),
        format!("{}: {}", tg(locale, "meta.request"), pending.request_id),
        format!("{}: {}", tg(locale, "meta.pane"), pending.pane_id),
    ];
    if let Some(session_id) = pending
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "meta.session"), session_id));
    }
    let body = format!("{}\n\n{}", lines.join("\n"), request.justification);
    send_message(
        &config.telegram.bot_token,
        &json!({
            "chat_id": telegram_chat_id_value(chat_id),
            "text": body,
            "reply_markup": {
                "inline_keyboard": [[
                    {
                        "text": tg(locale, "approval.button.once"),
                        "callback_data": approval_callback_data(&pending.request_id, "y")
                    },
                    {
                        "text": tg(locale, "approval.button.always"),
                        "callback_data": approval_callback_data(&pending.request_id, "a")
                    }
                ], [
                    {
                        "text": tg(locale, "approval.button.reject"),
                        "callback_data": approval_callback_data(&pending.request_id, "n")
                    }
                ]]
            }
        }),
    )
    .await?;
    Ok(())
}

pub(super) fn approval_sent_text(locale: crate::i18n::Locale, choice: &str) -> &'static str {
    match choice {
        "y" => tg(locale, "approval.sent.once"),
        "a" => tg(locale, "approval.sent.always"),
        "n" => tg(locale, "approval.sent.reject"),
        _ => tg(locale, "callback.unknown"),
    }
}
