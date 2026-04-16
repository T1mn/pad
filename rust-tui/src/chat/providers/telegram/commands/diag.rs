use super::*;

pub(crate) async fn send_session_diag(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
    arg: &str,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let context = resolve_session_diag_context(state, arg)?;
    let Some(context) = context else {
        let text = if arg.trim().is_empty() {
            tg(locale, "target.none")
        } else {
            tg(locale, "diag.empty")
        };
        send_text(&config.telegram.bot_token, chat_id, text).await?;
        return Ok(());
    };

    let body = format_session_diag_message(locale, &context);
    send_text(&config.telegram.bot_token, chat_id, &body).await?;
    Ok(())
}

pub(crate) async fn send_pad_status_report(
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

fn resolve_session_diag_context(
    state: &TelegramState,
    arg: &str,
) -> TelegramResult<Option<SessionDiagContext>> {
    let panels = live_panels().map_err(telegram_error)?;
    let arg = arg.trim();

    if !arg.is_empty() {
        if let Some(pending) = state
            .pending_requests
            .iter()
            .find(|pending| pending.request_id == arg)
        {
            let panel = panels.iter().find(|panel| panel.pane_id == pending.pane_id);
            let continuity = crate::session_continuity::load_snapshot_for(
                pending.session_id.as_deref(),
                pending
                    .transcript_path
                    .as_deref()
                    .or_else(|| panel.and_then(|panel| panel.transcript_path.as_deref())),
            );
            return Ok(Some(SessionDiagContext {
                target_label: pending.target_label.clone(),
                pane_id: Some(pending.pane_id.clone()),
                request_id: Some(pending.request_id.clone()),
                session_id: pending
                    .session_id
                    .clone()
                    .or_else(|| panel.and_then(|panel| panel.agent_session_id.clone())),
                transcript_path: pending
                    .transcript_path
                    .clone()
                    .or_else(|| panel.and_then(|panel| panel.transcript_path.clone())),
                continuity,
            }));
        }

        if let Some(panel) = panels.iter().find(|panel| panel.pane_id == arg) {
            let continuity = crate::session_continuity::load_snapshot_for(
                panel.agent_session_id.as_deref(),
                panel.transcript_path.as_deref(),
            );
            return Ok(Some(SessionDiagContext {
                target_label: compact_target_label(panel),
                pane_id: Some(panel.pane_id.clone()),
                request_id: state
                    .pending_requests
                    .iter()
                    .find(|pending| pending.pane_id == panel.pane_id)
                    .map(|pending| pending.request_id.clone()),
                session_id: panel.agent_session_id.clone(),
                transcript_path: panel.transcript_path.clone(),
                continuity,
            }));
        }

        let continuity = crate::session_continuity::load_snapshot_for(Some(arg), Some(arg));
        if continuity.is_some() {
            return Ok(Some(SessionDiagContext {
                target_label: arg.to_string(),
                pane_id: None,
                request_id: state
                    .pending_requests
                    .iter()
                    .find(|pending| pending.session_id.as_deref() == Some(arg))
                    .map(|pending| pending.request_id.clone()),
                session_id: Some(arg.to_string()),
                transcript_path: continuity
                    .as_ref()
                    .and_then(|snapshot| snapshot.transcript_path.clone()),
                continuity,
            }));
        }

        return Ok(None);
    }

    let selected = match state.selected_target.as_ref() {
        Some(selected) => selected,
        None => return Ok(None),
    };
    let panel = panels
        .iter()
        .find(|panel| panel.pane_id == selected.pane_id);
    let pending = state
        .pending_requests
        .iter()
        .find(|pending| pending.pane_id == selected.pane_id);
    let session_id = pending
        .and_then(|pending| pending.session_id.clone())
        .or_else(|| panel.and_then(|panel| panel.agent_session_id.clone()));
    let transcript_path = pending
        .and_then(|pending| pending.transcript_path.clone())
        .or_else(|| panel.and_then(|panel| panel.transcript_path.clone()));
    let continuity = crate::session_continuity::load_snapshot_for(
        session_id.as_deref(),
        transcript_path.as_deref(),
    );

    Ok(Some(SessionDiagContext {
        target_label: pending
            .map(|pending| pending.target_label.clone())
            .or_else(|| panel.map(compact_target_label))
            .unwrap_or_else(|| selected.label.clone()),
        pane_id: Some(selected.pane_id.clone()),
        request_id: pending.map(|pending| pending.request_id.clone()),
        session_id,
        transcript_path,
        continuity,
    }))
}

fn format_session_diag_message(
    locale: crate::i18n::Locale,
    context: &SessionDiagContext,
) -> String {
    let mut lines = vec![
        tg(locale, "diag.title").to_string(),
        context.target_label.clone(),
    ];
    if let Some(request_id) = context
        .request_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "meta.request"), request_id));
    }
    if let Some(pane_id) = context.pane_id.as_deref().filter(|value| !value.is_empty()) {
        lines.push(format!("{}: {}", tg(locale, "meta.pane"), pane_id));
    }
    if let Some(session_id) = context
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "meta.session"), session_id));
    }

    if let Some(snapshot) = context.continuity.as_ref() {
        lines.extend(super::pending::continuity_detail_lines(locale, snapshot));
    } else {
        lines.push(tg(locale, "diag.empty").to_string());
        if let Some(path) = context
            .transcript_path
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("{}: {}", tg(locale, "diag.transcript"), path));
        }
    }

    lines.join("\n")
}

pub(crate) fn build_pad_status_body(
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
