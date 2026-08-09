pub(crate) mod approval;
mod dispatch {
    mod approval_handler {
        use super::super::super::*;
        use super::super::approval::{
            approval_pending_index, approval_sent_text, parse_approval_callback_data,
        };

        pub(super) async fn handle_approval_callback(
            config: &Config,
            state: &mut TelegramState,
            query_id: &str,
            data: &str,
            locale: crate::i18n::Locale,
        ) -> TelegramResult<()> {
            let Some((request_id, choice)) = parse_approval_callback_data(data) else {
                answer_callback_query(
                    &config.telegram.bot_token,
                    query_id,
                    Some(tg(locale, "approval.none")),
                )
                .await?;
                return Ok(());
            };
            let Some(pending_index) = approval_pending_index(state, request_id) else {
                answer_callback_query(
                    &config.telegram.bot_token,
                    query_id,
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
                        query_id,
                        Some(tg(locale, "callback.unknown")),
                    )
                    .await?;
                    return Ok(());
                }
            };
            let approval_send_error = {
                let send_result =
                    terminal_remote::send_approval_key(&pending_snapshot.pane_id, key);
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
                        query_id,
                        Some(approval_sent_text(locale, choice)),
                    )
                    .await?;
                }
                Some(err_text) => {
                    answer_callback_query(
                        &config.telegram.bot_token,
                        query_id,
                        Some(&tg_fmt(locale, "approval.failed", err_text)),
                    )
                    .await?;
                    play_sound_event(config, crate::sound::SoundEvent::Failure);
                }
            }
            Ok(())
        }
    }
    mod use_pane {
        use super::super::super::*;

        pub(super) async fn handle_use_pane_callback(
            config: &Config,
            state: &mut TelegramState,
            query_id: &str,
            chat_id: &str,
            pane_id: &str,
            locale: crate::i18n::Locale,
        ) -> TelegramResult<()> {
            let panels = live_panels().map_err(telegram_error)?;
            if let Some(panel) = panels.iter().find(|panel| panel.pane_id == pane_id) {
                let selected = SelectedTarget {
                    pane_id: panel.pane_id.clone(),
                    label: format_agent_line_for_button(panel, locale),
                };
                state.selected_target = Some(selected.clone());
                answer_callback_query(
                    &config.telegram.bot_token,
                    query_id,
                    Some(tg(locale, "callback.switched")),
                )
                .await?;
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    &tg_fmt(locale, "target.switched", selected.label),
                )
                .await?;
            } else {
                answer_callback_query(
                    &config.telegram.bot_token,
                    query_id,
                    Some(tg(locale, "callback.stale")),
                )
                .await?;
            }
            Ok(())
        }
    }

    use super::super::*;
    use approval_handler::handle_approval_callback;
    use use_pane::handle_use_pane_callback;

    pub(in crate::chat::providers::telegram) async fn handle_callback_query(
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
            handle_use_pane_callback(config, state, &query.id, &chat_id, pane_id, locale).await?;
        } else if data.starts_with("approval:") {
            handle_approval_callback(config, state, &query.id, data, locale).await?;
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
}

pub(super) use approval::send_codex_approval_prompt;
#[cfg(test)]
pub(super) use approval::{
    approval_callback_data, approval_pending_index, parse_approval_callback_data,
};
pub(super) use dispatch::handle_callback_query;
