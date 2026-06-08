use super::super::*;

pub(in crate::chat::providers::telegram) fn approval_callback_data(
    request_id: &str,
    choice: &str,
) -> String {
    format!("approval:{}:{}", request_id, choice)
}

pub(in crate::chat::providers::telegram) fn parse_approval_callback_data(
    data: &str,
) -> Option<(&str, &str)> {
    let rest = data.strip_prefix("approval:")?;
    let (request_id, choice) = rest.rsplit_once(':')?;
    if request_id.is_empty() || choice.is_empty() {
        return None;
    }
    Some((request_id, choice))
}

pub(in crate::chat::providers::telegram) fn approval_pending_index(
    state: &TelegramState,
    request_id: &str,
) -> Option<usize> {
    state.pending_requests.iter().position(|pending| {
        pending.request_id == request_id
            && pending.agent_kind == "codex"
            && pending.approval_call_id.is_some()
    })
}

pub(in crate::chat::providers::telegram) async fn send_codex_approval_prompt(
    config: &Config,
    chat_id: &str,
    pending: &PendingRequest,
    request: &CodexApprovalRequest,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let body = approval_prompt_text(locale, pending, request);
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

fn approval_prompt_text(
    locale: crate::i18n::Locale,
    pending: &PendingRequest,
    request: &CodexApprovalRequest,
) -> String {
    let mut body = String::new();
    push_approval_prompt_line(&mut body, tg(locale, "approval.prompt"));
    push_approval_prompt_line(
        &mut body,
        &format!(
            "{}: {}",
            tg(locale, "approval.target"),
            pending.target_label
        ),
    );
    push_approval_prompt_line(
        &mut body,
        &format!("{}: {}", tg(locale, "meta.request"), pending.request_id),
    );
    push_approval_prompt_line(
        &mut body,
        &format!("{}: {}", tg(locale, "meta.pane"), pending.pane_id),
    );
    if let Some(session_id) = pending
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        push_approval_prompt_line(
            &mut body,
            &format!("{}: {}", tg(locale, "meta.session"), session_id),
        );
    }
    body.push_str("\n\n");
    body.push_str(&request.justification);
    body
}

fn push_approval_prompt_line(out: &mut String, line: &str) {
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(line);
}

pub(in crate::chat::providers::telegram) fn approval_sent_text(
    locale: crate::i18n::Locale,
    choice: &str,
) -> &'static str {
    match choice {
        "y" => tg(locale, "approval.sent.once"),
        "a" => tg(locale, "approval.sent.always"),
        "n" => tg(locale, "approval.sent.reject"),
        _ => tg(locale, "callback.unknown"),
    }
}

#[cfg(test)]
#[path = "approval_tests.rs"]
mod tests;
