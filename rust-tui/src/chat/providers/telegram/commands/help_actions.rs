use super::*;

pub(crate) async fn send_help_message(
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

pub(crate) async fn edit_help_message(
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

pub(crate) async fn send_agent_list(
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

    let body = agent_list_body(&snapshot);
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

fn agent_list_body(snapshot: &[AgentSnapshotEntry]) -> String {
    let mut body = String::new();
    for (idx, entry) in snapshot.iter().enumerate() {
        if idx > 0 {
            body.push('\n');
        }
        body.push_str(&entry.label);
    }
    body
}
