use super::*;

pub(super) async fn handle_callback_query(
    config: &Config,
    state: &mut TelegramState,
    query: TelegramCallbackQuery,
) -> Result<(), Box<dyn std::error::Error>> {
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
        let panels = live_panels()?;
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
    } else if let Some(choice) = data.strip_prefix("approval:") {
        let Some(pending_snapshot) = state.pending.as_ref().cloned() else {
            answer_callback_query(
                &config.telegram.bot_token,
                &query.id,
                Some(tg(locale, "approval.none")),
            )
            .await?;
            return Ok(());
        };
        if pending_snapshot.agent_kind != "codex" || pending_snapshot.approval_call_id.is_none() {
            answer_callback_query(
                &config.telegram.bot_token,
                &query.id,
                Some(tg(locale, "approval.none")),
            )
            .await?;
            return Ok(());
        }
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
        match tmux_dispatch::send_approval_key(&pending_snapshot.pane_id, key) {
            Ok(()) => {
                invalidate_live_panels();
                if let Some(pending) = state.pending.as_mut() {
                    pending.phase = "awaiting_stop".to_string();
                    pending.approval_call_id = None;
                    pending.approval_justification = None;
                    pending.last_status_at = None;
                }
                refresh_pending_feedback(config, state, true);
                answer_callback_query(
                    &config.telegram.bot_token,
                    &query.id,
                    Some(approval_sent_text(locale, choice)),
                )
                .await?;
            }
            Err(err) => {
                answer_callback_query(
                    &config.telegram.bot_token,
                    &query.id,
                    Some(&tg_fmt(locale, "approval.failed", err)),
                )
                .await?;
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
) -> Result<(), Box<dyn std::error::Error>> {
    let locale = telegram_locale(config);
    let body = format!(
        "{}\n{}: {}\n\n{}",
        tg(locale, "approval.prompt"),
        tg(locale, "approval.target"),
        pending.target_label,
        request.justification
    );
    send_message(
        &config.telegram.bot_token,
        &json!({
            "chat_id": telegram_chat_id_value(chat_id),
            "text": body,
            "reply_markup": {
                "inline_keyboard": [[
                    {
                        "text": tg(locale, "approval.button.once"),
                        "callback_data": "approval:y"
                    },
                    {
                        "text": tg(locale, "approval.button.always"),
                        "callback_data": "approval:a"
                    }
                ], [
                    {
                        "text": tg(locale, "approval.button.reject"),
                        "callback_data": "approval:n"
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
