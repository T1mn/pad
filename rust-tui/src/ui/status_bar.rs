use crate::app::App;
use crate::model::PreviewSource;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;
    let background = Style::default().bg(theme.highlight_bg).fg(theme.status_fg);

    let mode_span = match app.mode {
        crate::app::state::Mode::Search => {
            mode_badge(crate::i18n::t(l, "mode.search"), theme.mode_search_bg)
        }
        crate::app::state::Mode::Settings => {
            mode_badge(crate::i18n::t(l, "mode.settings"), theme.accent)
        }
        crate::app::state::Mode::TelegramSettings => {
            mode_badge(crate::i18n::t(l, "mode.settings"), theme.accent)
        }
        crate::app::state::Mode::ThemeSelector => {
            mode_badge(crate::i18n::t(l, "mode.theme"), theme.keyword)
        }
        crate::app::state::Mode::Help => mode_badge(crate::i18n::t(l, "mode.help"), theme.accent),
        crate::app::state::Mode::FilePreview => {
            mode_badge(crate::i18n::t(l, "mode.preview"), theme.mode_tree_bg)
        }
        _ if app.show_tree => mode_badge(crate::i18n::t(l, "mode.tree"), theme.mode_tree_bg),
        _ => mode_badge(crate::i18n::t(l, "mode.normal"), theme.mode_normal_bg),
    };
    let mode_width = display_width(mode_span.content.as_ref());
    let body_width = area.width.saturating_sub(mode_width as u16);

    let body = match app.mode {
        crate::app::state::Mode::Search => format!(
            "{}: {}  Enter {}  Esc {}",
            crate::i18n::t(l, "status.search"),
            app.search_query,
            crate::i18n::t(l, "status.confirm"),
            crate::i18n::t(l, "status.cancel")
        ),
        crate::app::state::Mode::Settings => String::from(crate::i18n::t(l, "status.settings_nav")),
        crate::app::state::Mode::TelegramSettings => {
            String::from(crate::i18n::t(l, "status.settings_nav"))
        }
        crate::app::state::Mode::ThemeSelector => {
            String::from(crate::i18n::t(l, "status.theme_nav"))
        }
        crate::app::state::Mode::Help => String::from(crate::i18n::t(l, "status.help_close")),
        crate::app::state::Mode::FilePreview => {
            String::from(crate::i18n::t(l, "status.preview_nav"))
        }
        _ => compose_status_body(app, body_width),
    };

    let line = Line::from(vec![
        mode_span,
        Span::styled(
            format_status_remainder(&body, area.width, mode_width),
            background,
        ),
    ]);
    let status_bar = Paragraph::new(line)
        .style(background)
        .alignment(Alignment::Left);
    f.render_widget(status_bar, area);
}

fn compose_status_body(app: &App, width: u16) -> String {
    let l = app.locale;
    let elapsed = app.last_refresh.elapsed().as_secs();
    let scan_status = if app.scan_in_progress {
        format!(" {}", crate::i18n::t(l, "status.scanning"))
    } else {
        String::new()
    };
    let left = if app.show_tree {
        if let Some(path) = &app.file_preview_path {
            format!("📁 {}", path.display())
        } else {
            crate::i18n::t(l, "tree.explorer").to_string()
        }
    } else {
        format!(
            "{}s {}{}",
            elapsed,
            crate::i18n::t(l, "status.ago"),
            scan_status
        )
    };

    let right_hint = if app.show_tree {
        crate::i18n::t(l, "status.tree_nav")
    } else if app.preview_is_focused() {
        if app.preview_source == PreviewSource::Session && !app.preview_turns.is_empty() {
            crate::i18n::t(l, "status.preview_session_nav")
        } else {
            crate::i18n::t(l, "status.preview_nav")
        }
    } else {
        crate::i18n::t(l, "status.normal_nav_panel")
    };

    let left_text = if app.show_tree {
        format!(
            "{}  {}s {}{}",
            left,
            elapsed,
            crate::i18n::t(l, "status.ago"),
            scan_status
        )
    } else {
        format!(" {}", left)
    };

    format_two_sided(&left_text, right_hint, width as usize)
}

fn format_status_remainder(text: &str, width: u16, occupied: usize) -> String {
    let available = width as usize;
    let remaining = available.saturating_sub(occupied);
    if remaining <= 1 {
        return String::new();
    }
    let content_target = remaining.saturating_sub(1);
    let content = if display_width(text) <= content_target {
        text.to_string()
    } else {
        truncate_from_left_to_width(text, content_target)
    };
    format!(" {}", content)
}

fn format_two_sided(left: &str, right: &str, width: usize) -> String {
    let right_width = display_width(right);
    if right_width + 2 >= width {
        return truncate_to_width(right, width);
    }
    let left_budget = width.saturating_sub(right_width + 3);
    let left_text = truncate_to_width(left, left_budget);
    format!("{}   {}", left_text, right)
}

fn mode_badge(label: &str, bg: ratatui::style::Color) -> Span<'static> {
    Span::styled(
        format!(" {} ", label),
        Style::default().fg(Color::Black).bg(bg),
    )
}

fn truncate_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(text) <= max_width {
        return text.to_string();
    }

    let ellipsis = "…";
    let ellipsis_width = display_width(ellipsis);
    let target_width = max_width.saturating_sub(ellipsis_width);
    let mut result = String::new();
    let mut used = 0usize;

    for ch in text.chars() {
        let width = char_display_width(ch);
        if used + width > target_width {
            break;
        }
        result.push(ch);
        used += width;
    }

    result.push_str(ellipsis);
    result
}

fn truncate_from_left_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(text) <= max_width {
        return text.to_string();
    }

    let ellipsis = "…";
    let ellipsis_width = display_width(ellipsis);
    if max_width <= ellipsis_width {
        return ellipsis.to_string();
    }

    let keep_width = max_width.saturating_sub(ellipsis_width);
    let mut kept = Vec::new();
    let mut used = 0usize;

    for ch in text.chars().rev() {
        let width = char_display_width(ch);
        if used + width > keep_width {
            break;
        }
        kept.push(ch);
        used += width;
    }

    kept.reverse();
    let mut result = String::from(ellipsis);
    result.extend(kept);
    result
}

fn display_width(s: &str) -> usize {
    s.chars().map(char_display_width).sum()
}

fn char_display_width(c: char) -> usize {
    if c == '\t' {
        return 4;
    }
    if c.is_control() {
        return 0;
    }

    let code = c as u32;
    if matches!(
        code,
        0x1100..=0x115F
            | 0x2329..=0x232A
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1FAFF
            | 0x20000..=0x3FFFD
    ) {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::{compose_status_body, display_width, format_status_remainder, mode_badge};
    use crate::app::App;
    use crate::i18n::Locale;
    use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};

    #[test]
    fn normal_status_hides_selected_panel_identity_details() {
        let mut app = App::new();
        app.panels.push(AgentPanel {
            session: "0".into(),
            window: "main".into(),
            window_index: "2".into(),
            pane: "1".into(),
            pane_id: "%1".into(),
            agent_type: AgentType::Codex,
            working_dir: "/tmp/rust-tui".into(),
            is_active: true,
            state: AgentState::Busy,
            state_source: AgentStateSource::Scanner,
            transcript_path: None,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            git_info: None,
            pid: None,
            start_time: None,
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        });
        app.table_state.select(Some(0));

        let body = compose_status_body(&app, 120);
        assert!(!body.contains("0:2.1"));
        assert!(!body.to_lowercase().contains("codex"));
    }

    #[test]
    fn status_remainder_preserves_right_hint_with_mode_badge_width() {
        let mut app = App::new();
        app.locale = Locale::En;
        let badge = mode_badge("NORMAL", app.theme.mode_normal_bg);
        let badge_width = display_width(badge.content.as_ref());
        let body = compose_status_body(&app, 80_u16.saturating_sub((badge_width + 1) as u16));
        let remainder = format_status_remainder(&body, 80, badge_width);
        let right_hint = crate::i18n::t(app.locale, "status.normal_nav_panel");

        assert!(remainder.ends_with(right_hint));
        assert!(badge_width + display_width(&remainder) <= 80);
    }
}
