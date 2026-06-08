use super::*;

pub(super) async fn resolve_codex_slash_panel(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
    locale: crate::i18n::Locale,
) -> TelegramResult<Option<AgentPanel>> {
    let Some(target) = state.selected_target.as_ref() else {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "target.none"),
        )
        .await?;
        return Ok(None);
    };

    let panels = live_panels().map_err(telegram_error)?;
    let Some(panel) = panels.iter().find(|panel| panel.pane_id == target.pane_id) else {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "pane.stale"),
        )
        .await?;
        return Ok(None);
    };

    if !matches!(&panel.agent_type, &AgentType::Codex) {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "target.not_codex"),
        )
        .await?;
        return Ok(None);
    }

    if panel.state == AgentState::Busy {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "agent.busy"),
        )
        .await?;
        return Ok(None);
    }
    if panel.state == AgentState::Waiting {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "agent.waiting"),
        )
        .await?;
        return Ok(None);
    }

    Ok(Some(panel.clone()))
}
