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
    let l = app.locale;
    let elapsed = app.last_refresh.elapsed().as_secs();
    let scan_status = if app.scan_in_progress {
        format!(" [{}]", crate::i18n::t(l, "status.scanning"))
    } else {
        String::new()
    };

    let (msg, style) = match app.mode {
        crate::app::state::Mode::Search => (
            format!("{}: {} | Enter: {} | Esc: {}",
                crate::i18n::t(l, "status.search"),
                app.search_query,
                crate::i18n::t(l, "status.confirm"),
                crate::i18n::t(l, "status.cancel")),
            Style::default().fg(theme.warning),
        ),
        crate::app::state::Mode::Settings => (
            String::from(crate::i18n::t(l, "status.settings_nav")),
            Style::default().fg(theme.accent),
        ),
        crate::app::state::Mode::ThemeSelector => (
            String::from(crate::i18n::t(l, "status.theme_nav")),
            Style::default().fg(theme.accent),
        ),
        crate::app::state::Mode::Help => (
            String::from(crate::i18n::t(l, "status.help_close")),
            Style::default().fg(theme.accent),
        ),
        crate::app::state::Mode::FilePreview => (
            String::from(crate::i18n::t(l, "status.preview_nav")),
            Style::default().fg(theme.accent),
        ),
        _ => {
            let panel_count = app.filtered_panels().len();
            let mode_indicator = if app.show_tree {
                Span::styled(
                    format!(" {} ", crate::i18n::t(l, "mode.tree")),
                    Style::default().fg(Color::Black).bg(theme.mode_tree_bg),
                )
            } else {
                Span::styled(
                    format!(" {} ", crate::i18n::t(l, "mode.normal")),
                    Style::default().fg(Color::Black).bg(theme.mode_normal_bg),
                )
            };

            let base = if app.show_tree {
                crate::i18n::t(l, "status.tree_nav")
            } else {
                crate::i18n::t(l, "status.normal_nav")
            };

            let status = format!(
                " {} | {} {} | {}s {}{}",
                base,
                panel_count,
                crate::i18n::t(l, "status.panels"),
                elapsed,
                crate::i18n::t(l, "status.ago"),
                scan_status
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
            Span::styled(
                format!(" {} ", crate::i18n::t(l, "mode.search")),
                Style::default().fg(Color::Black).bg(theme.mode_search_bg),
            )
        }
        crate::app::state::Mode::Settings => {
            Span::styled(
                format!(" {} ", crate::i18n::t(l, "mode.settings")),
                Style::default().fg(Color::Black).bg(theme.accent),
            )
        }
        crate::app::state::Mode::ThemeSelector => {
            Span::styled(
                format!(" {} ", crate::i18n::t(l, "mode.theme")),
                Style::default().fg(Color::Black).bg(theme.keyword),
            )
        }
        crate::app::state::Mode::Help => {
            Span::styled(
                format!(" {} ", crate::i18n::t(l, "mode.help")),
                Style::default().fg(Color::Black).bg(theme.accent),
            )
        }
        crate::app::state::Mode::FilePreview => {
            Span::styled(
                format!(" {} ", crate::i18n::t(l, "mode.preview")),
                Style::default().fg(Color::Black).bg(theme.mode_tree_bg),
            )
        }
        _ => Span::raw(""),
    };

    let line = Line::from(vec![mode_span, Span::styled(format!(" {}", msg), style)]);
    let status_bar = Paragraph::new(line).alignment(Alignment::Left);
    f.render_widget(status_bar, area);
}
