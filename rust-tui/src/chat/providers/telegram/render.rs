use super::*;

pub(super) fn format_agent_line(
    idx: usize,
    panel: &AgentPanel,
    locale: crate::i18n::Locale,
) -> String {
    let state = agent_state_label(&panel.state, locale);
    format!(
        "{}. [{}] {} ({})",
        idx,
        panel.agent_type,
        panel_display_title(panel),
        state
    )
}

pub(super) fn format_agent_line_for_button(
    panel: &AgentPanel,
    locale: crate::i18n::Locale,
) -> String {
    format!(
        "[{}] {} ({})",
        panel.agent_type,
        panel_display_title(panel),
        agent_state_label(&panel.state, locale)
    )
}

pub(super) fn build_agent_keyboard(
    panels: &[AgentPanel],
    locale: crate::i18n::Locale,
) -> Vec<Vec<serde_json::Value>> {
    panels
        .iter()
        .map(|panel| {
            vec![json!({
                "text": button_label(panel, locale),
                "callback_data": format!("use-pane:{}", panel.pane_id),
            })]
        })
        .collect()
}

pub(super) fn button_label(panel: &AgentPanel, locale: crate::i18n::Locale) -> String {
    let title = panel_display_title(panel);
    let title = truncate_chars(&title, 24);
    format!(
        "{} | {} | {}",
        panel.agent_type,
        title,
        agent_state_label(&panel.state, locale)
    )
}

pub(super) fn agent_state_label(state: &AgentState, locale: crate::i18n::Locale) -> &'static str {
    match state {
        AgentState::Idle if locale_prefers_chinese(locale) => "空闲",
        AgentState::Idle => "idle",
        AgentState::Busy if locale_prefers_chinese(locale) => "忙碌",
        AgentState::Busy => "busy",
        AgentState::Waiting if locale_prefers_chinese(locale) => "等待",
        AgentState::Waiting => "waiting",
    }
}

pub(super) fn truncate_for_log(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let truncated = text.chars().take(max_chars).collect::<String>();
    format!("{}...", truncated)
}

pub(super) fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let shortened = text
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    format!("{}…", shortened)
}
