mod body;
mod text;

use crate::app::App;
use body::{mode_span, status_body};
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use text::{display_width, format_status_remainder};

pub fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let background = Style::default().bg(theme.highlight_bg).fg(theme.status_fg);
    let mode_span = mode_span(app);
    let mode_width = display_width(mode_span.content.as_ref());
    let body_width = area.width.saturating_sub(mode_width as u16);
    let body = status_body(app, body_width);

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

#[cfg(test)]
mod tests {
    use super::{display_width, format_status_remainder};
    use crate::app::App;
    use crate::i18n::Locale;
    use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};
    use crate::ui::status_bar::body::compose_status_body;
    use crate::ui::status_bar::text::mode_badge;

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
            cached_preview_turns: Default::default(),
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
