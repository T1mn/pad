use super::layout::padded_inner;
use super::list::render_selection_list_rows;
use crate::theme::Theme;
use crate::ui::selection::{SelectionItem, SelectionState};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render_selection_surface(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    title: &str,
    items: &[SelectionItem],
    state: &SelectionState,
    footer: Option<&str>,
) {
    let inner = padded_inner(area);
    let mut constraints = vec![Constraint::Length(1), Constraint::Min(0)];
    if footer.is_some() {
        constraints.push(Constraint::Length(1));
    }
    let sections = Layout::vertical(constraints).split(inner);
    render_header(f, sections[0], theme, title, state);
    render_selection_list_rows(f, sections[1], theme, items, state);
    if let Some(footer_text) = footer {
        if let Some(footer_area) = sections.get(2) {
            render_footer(f, *footer_area, theme, footer_text);
        }
    }
}

fn render_header(f: &mut Frame, area: Rect, theme: &Theme, title: &str, state: &SelectionState) {
    let header = if state.searching || !state.query.is_empty() {
        if state.searching {
            format!("/{}|", state.query)
        } else {
            format!("/{}", state.query)
        }
    } else {
        title.to_string()
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            header,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))),
        area,
    );
}

fn render_footer(f: &mut Frame, area: Rect, theme: &Theme, footer_text: &str) {
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            footer_text.to_string(),
            Style::default()
                .fg(theme.comment)
                .add_modifier(Modifier::DIM),
        ))),
        area,
    );
}
