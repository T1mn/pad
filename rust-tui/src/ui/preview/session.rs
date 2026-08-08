mod badges {
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
            crate::model::AgentType::Grok => (theme.bg, Color::Rgb(255, 70, 70)),
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
}
mod detail;
mod list;
mod scroll {
    use crate::app::App;

    pub(crate) fn resolve_session_list_scroll(
        app: &mut App,
        selected_range: Option<(usize, usize)>,
        viewport_height: u16,
        total_lines: usize,
    ) -> u16 {
        if viewport_height == 0 {
            return 0;
        }
        let max_scroll = total_lines.saturating_sub(viewport_height as usize);
        let mut scroll = app
            .preview
            .list_scroll
            .min(max_scroll.min(u16::MAX as usize) as u16);

        if app.preview.follow_selection {
            if let Some((start, end)) = selected_range {
                let scroll_usize = scroll as usize;
                let viewport = viewport_height as usize;
                if start < scroll_usize {
                    scroll = start.min(max_scroll).min(u16::MAX as usize) as u16;
                } else if end >= scroll_usize.saturating_add(viewport) {
                    let adjusted = end
                        .saturating_add(1)
                        .saturating_sub(viewport)
                        .min(max_scroll)
                        .min(u16::MAX as usize);
                    scroll = adjusted as u16;
                }
            }
        }

        app.preview.list_scroll = scroll;
        scroll
    }

    pub(crate) fn resolve_preview_scroll_for_line_count(
        app: &mut App,
        total_lines: usize,
        viewport_height: u16,
    ) -> u16 {
        if viewport_height == 0 {
            return 0;
        }
        let max_scroll = total_lines.saturating_sub(viewport_height as usize);
        let max_scroll = max_scroll.min(u16::MAX as usize) as u16;
        let scroll = if app.preview_uses_detail_scroll() {
            app.preview.detail_scroll.min(max_scroll)
        } else if app.preview.follow_bottom {
            max_scroll
        } else {
            app.preview.scroll.min(max_scroll)
        };
        if app.preview_uses_detail_scroll() {
            app.preview.detail_scroll = scroll;
        } else {
            app.preview.scroll = scroll;
        }
        scroll
    }

    pub(crate) fn visible_detail_window(
        total_lines: usize,
        scroll: u16,
        viewport_height: u16,
    ) -> std::ops::Range<usize> {
        let start = scroll as usize;
        let end = start
            .saturating_add(viewport_height as usize)
            .min(total_lines);
        start..end
    }
}
mod text {
    use super::super::common::{char_display_width, truncate_to_width};

    pub(super) fn split_preview_card_lines(
        text: &str,
        width: usize,
        max_lines: usize,
    ) -> Vec<String> {
        let mut remaining = text.trim();
        let mut lines = Vec::with_capacity(max_lines);

        for idx in 0..max_lines {
            if remaining.is_empty() {
                lines.push(String::new());
                continue;
            }

            if idx + 1 == max_lines {
                lines.push(truncate_to_width(remaining, width));
                remaining = "";
                continue;
            }

            let (prefix, rest) = take_prefix_by_width(remaining, width);
            lines.push(prefix.to_string());
            remaining = rest;
        }

        lines
    }

    fn take_prefix_by_width(text: &str, max_width: usize) -> (&str, &str) {
        if max_width == 0 || text.is_empty() {
            return ("", text);
        }

        let mut used = 0usize;
        let mut split_at = text.len();
        for (idx, ch) in text.char_indices() {
            let ch_width = char_display_width(ch);
            if used + ch_width > max_width {
                split_at = idx;
                break;
            }
            used += ch_width;
        }

        if split_at == text.len() {
            return (text, "");
        }

        let prefix = text[..split_at].trim_end();
        let rest = text[split_at..].trim_start();
        (prefix, rest)
    }

    pub(super) fn question_text_for_display(text: &str) -> String {
        strip_turn_prefix(text, &["Q:", "Q：", "Question:", "question:"]).to_string()
    }

    pub(super) fn answer_text_for_display(text: &str) -> String {
        strip_turn_prefix(text, &["A:", "A：", "Answer:", "answer:"]).to_string()
    }

    fn strip_turn_prefix<'a>(text: &'a str, prefixes: &[&str]) -> &'a str {
        let trimmed = text.trim();
        for prefix in prefixes {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                return rest.trim_start();
            }
        }
        trimmed
    }
}

use crate::app::App;
use crate::theme::Theme;
use ratatui::{layout::Rect, Frame};

pub(crate) use badges::{
    fixed_label, localized_status_label, preview_agent_badge_colors, preview_badge,
};
pub use detail::render_session_detail_lines;
pub(super) use list::render_session_gap_line;
pub(crate) use list::{render_session_card, session_list_total_lines, session_turn_index_at_line};
pub(crate) use scroll::{
    resolve_preview_scroll_for_line_count, resolve_session_list_scroll, visible_detail_window,
};

#[cfg(test)]
pub(crate) use list::build_session_list_lines;

pub(crate) const SESSION_ITEM_CONTENT_HEIGHT: usize = 3;
pub(crate) const SESSION_ITEM_GAP_HEIGHT: usize = 1;

pub(crate) fn draw_session_preview(f: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    if app.preview.view == crate::model::PreviewView::SessionDetail {
        if let Some(selected) = app.preview.expanded_turn {
            detail::draw_session_detail(f, app, area, theme, selected);
            return;
        }
    }
    if app.preview.view == crate::model::PreviewView::SessionList
        || app.preview.view == crate::model::PreviewView::SessionDetail
    {
        list::draw_session_list(f, app, area, theme);
    } else if let Some(selected) = app.preview.expanded_turn {
        detail::draw_session_detail(f, app, area, theme, selected);
    } else {
        list::draw_session_list(f, app, area, theme);
    }
}

#[cfg(test)]
#[path = "session/tests.rs"]
mod tests;
