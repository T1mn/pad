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
    truncate_chars_with_marker(text, max_chars, "...")
}

pub(super) fn truncate_chars(text: &str, max_chars: usize) -> String {
    let Some(prefix_len) = truncated_prefix_byte_len(text, max_chars, max_chars.saturating_sub(1))
    else {
        return text.to_string();
    };
    let mut shortened = text[..prefix_len].to_string();
    shortened.push('…');
    shortened
}

fn truncate_chars_with_marker(text: &str, max_chars: usize, marker: &str) -> String {
    let Some(prefix_len) = truncated_prefix_byte_len(text, max_chars, max_chars) else {
        return text.to_string();
    };
    let mut truncated = text[..prefix_len].to_string();
    truncated.push_str(marker);
    truncated
}

fn truncated_prefix_byte_len(text: &str, max_chars: usize, keep_chars: usize) -> Option<usize> {
    let mut prefix_len = 0;
    for (idx, (byte_idx, _)) in text.char_indices().enumerate() {
        if idx == keep_chars {
            prefix_len = byte_idx;
        }
        if idx == max_chars {
            return Some(prefix_len);
        }
    }
    None
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
