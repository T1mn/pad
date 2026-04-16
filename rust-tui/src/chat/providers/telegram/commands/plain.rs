use super::*;

pub(crate) async fn handle_plain_text(
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
    let failure_scan_offset = result_scan_offset;
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
        failure_scan_offset,
        last_failure_check_at: None,
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
