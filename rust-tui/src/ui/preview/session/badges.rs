use super::super::common::pad_to_width;
use crate::theme::Theme;
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

pub(crate) fn localized_status_label(
    locale: crate::i18n::Locale,
    state: &crate::model::AgentState,
) -> &'static str {
    match state {
        crate::model::AgentState::Busy => crate::i18n::t(locale, "preview.working"),
        crate::model::AgentState::Waiting => crate::i18n::t(locale, "preview.waiting"),
        crate::model::AgentState::Idle => crate::i18n::t(locale, "preview.idle"),
    }
}

pub(crate) fn preview_badge(
    label: &str,
    fg: ratatui::style::Color,
    bg: ratatui::style::Color,
) -> Span<'static> {
    Span::styled(
        format!(" {} ", label),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )
}

pub(crate) fn preview_agent_badge_colors(
    agent_type: &crate::model::AgentType,
    theme: &Theme,
) -> (ratatui::style::Color, ratatui::style::Color) {
    match agent_type {
        crate::model::AgentType::Codex => (theme.bg, Color::Rgb(88, 166, 255)),
        crate::model::AgentType::Claude => (theme.bg, Color::Rgb(249, 140, 87)),
        crate::model::AgentType::Gemini => (theme.bg, Color::Rgb(180, 140, 255)),
        crate::model::AgentType::Kimi | crate::model::AgentType::OpenCode => {
            (Color::White, Color::Black)
        }
        crate::model::AgentType::Aider => (theme.bg, theme.success),
        crate::model::AgentType::Cursor => (theme.bg, Color::Rgb(180, 140, 255)),
        crate::model::AgentType::Unknown => (theme.fg, theme.comment),
    }
}

pub(crate) fn fixed_label(label: &str, width: usize, theme: &Theme) -> Span<'static> {
    Span::styled(
        format!("{} ", pad_to_width(label, width)),
        Style::default()
            .fg(theme.comment)
            .add_modifier(Modifier::BOLD),
    )
}
