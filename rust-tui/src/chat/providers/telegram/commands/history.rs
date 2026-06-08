use super::*;
use std::fmt::Write;

pub(crate) async fn send_recent_history(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
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

    if !history_supported_agent(&panel.agent_type) {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "history.unsupported"),
        )
        .await?;
        return Ok(());
    }

    let turns = recent_history_turns(panel, locale);
    if turns.is_empty() {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "history.empty"),
        )
        .await?;
        return Ok(());
    }

    let body = format_recent_history_message(locale, &compact_target_label(panel), &turns);
    send_text(&config.telegram.bot_token, chat_id, &body).await?;
    Ok(())
}

fn history_supported_agent(agent_type: &AgentType) -> bool {
    matches!(
        agent_type,
        AgentType::Codex | AgentType::Claude | AgentType::Gemini
    )
}

pub(crate) fn recent_history_turns(
    panel: &AgentPanel,
    locale: crate::i18n::Locale,
) -> Vec<crate::model::PreviewTurn> {
    let request = crate::preview_source::PreviewRequest {
        target_key: panel.pane_id.clone(),
        live_pane_id: Some(panel.pane_id.clone()),
        agent_type: panel.agent_type.clone(),
        working_dir: panel.working_dir.clone(),
        state: panel.state.clone(),
        transcript_path: panel.transcript_path.clone(),
        cached_preview_turns: panel.cached_preview_turns.clone(),
        session_cache_state: panel.session_cache_state,
        agent_session_id: panel.agent_session_id.clone(),
        session_origin: Some(crate::model::PreviewSessionOrigin::Pane),
        persist_resolved_session: false,
        known_updated_at: None,
    };

    let update = crate::preview_source::load_preview(&request, "session", locale);
    let turns = if !update.turns.is_empty() {
        update.turns.to_vec()
    } else {
        panel.cached_preview_turns.to_vec()
    };
    turns.into_iter().take(3).collect()
}

pub(crate) fn format_recent_history_message(
    locale: crate::i18n::Locale,
    target_label: &str,
    turns: &[crate::model::PreviewTurn],
) -> String {
    let mut body = String::new();
    body.push_str(tg(locale, "history.title"));
    body.push('\n');
    body.push_str(target_label);

    for (idx, turn) in turns.iter().enumerate() {
        body.push_str("\n\n");
        let _ = writeln!(body, "{}. Q:", idx + 1);
        body.push_str(turn.question.trim());
        body.push_str("\n\nA:\n");
        body.push_str(
            turn.answer
                .as_deref()
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .unwrap_or(tg(locale, "history.answer_missing")),
        );
    }

    body
}
