use crate::app::App;
use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
    layout::Rect,
};

pub fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let elapsed = app.last_refresh.elapsed().as_secs();
    let scan_status = if app.scan_in_progress {
        " [scanning...]"
    } else {
        ""
    };

    let (msg, style) = match app.mode {
        crate::app::state::Mode::Search => (
            format!("SEARCH: {} | Enter: confirm | Esc: cancel", app.search_query),
            Style::default().fg(theme.warning),
        ),
        crate::app::state::Mode::Settings => (
            String::from("j/k: move | Enter: edit | 1-4: jump | Esc: close"),
            Style::default().fg(theme.accent),
        ),
        crate::app::state::Mode::ThemeSelector => (
            String::from("j/k: move | Enter: select | 1-9: jump | Esc: back"),
            Style::default().fg(theme.accent),
        ),
        crate::app::state::Mode::Help => (
            String::from("Press Esc or ? to close help"),
            Style::default().fg(theme.accent),
        ),
        _ => {
            let panel_count = app.filtered_panels().len();
            let mode_indicator = if app.show_tree {
                Span::styled(" TREE ", Style::default().fg(Color::Black).bg(theme.mode_tree_bg))
            } else {
                Span::styled(" NORMAL ", Style::default().fg(Color::Black).bg(theme.mode_normal_bg))
            };

            let base = if app.show_tree {
                "↑/k ↓/j nav | J/K scroll | space expand | t close | ⏎ attach | c create | ? help | q quit"
            } else {
                "↑/k ↓/j nav | t tree | / find | ⏎ attach | c create | ? help | q quit"
            };

            let status = format!(
                " {} | {} panels | {}s ago{}",
                base, panel_count, elapsed, scan_status
            );

            // Build line with mode indicator
            let line = Line::from(vec![mode_indicator, Span::styled(status, Style::default().fg(theme.status_fg))]);
            let status_bar = Paragraph::new(line).alignment(Alignment::Left);
            f.render_widget(status_bar, area);
            return;
        }
    };

    let mode_span = match app.mode {
        crate::app::state::Mode::Search => {
            Span::styled(" SEARCH ", Style::default().fg(Color::Black).bg(theme.mode_search_bg))
        }
        crate::app::state::Mode::Settings => {
            Span::styled(" SETTINGS ", Style::default().fg(Color::Black).bg(theme.accent))
        }
        crate::app::state::Mode::ThemeSelector => {
            Span::styled(" THEME ", Style::default().fg(Color::Black).bg(theme.keyword))
        }
        crate::app::state::Mode::Help => {
            Span::styled(" HELP ", Style::default().fg(Color::Black).bg(theme.accent))
        }
        _ => Span::raw(""),
    };

    let line = Line::from(vec![mode_span, Span::styled(format!(" {}", msg), style)]);
    let status_bar = Paragraph::new(line).alignment(Alignment::Left);
    f.render_widget(status_bar, area);
}
