mod layout {
    use ratatui::layout::{Margin, Rect};

    const SELECTION_HORIZONTAL_PADDING: u16 = 2;
    const SELECTION_VERTICAL_PADDING: u16 = 1;

    pub const fn selection_surface_padding_height() -> u16 {
        SELECTION_VERTICAL_PADDING * 2
    }

    pub fn recommended_list_modal_height(
        item_count: u16,
        row_height: u16,
        header_lines: u16,
        footer_lines: u16,
    ) -> u16 {
        item_count
            .max(1)
            .saturating_mul(row_height)
            .saturating_add(header_lines)
            .saturating_add(footer_lines)
            .saturating_add(selection_surface_padding_height())
    }

    pub(super) fn padded_inner(area: Rect) -> Rect {
        let horizontal = SELECTION_HORIZONTAL_PADDING.min(area.width.saturating_sub(1) / 2);
        let vertical = SELECTION_VERTICAL_PADDING.min(area.height.saturating_sub(1) / 2);
        area.inner(Margin {
            horizontal,
            vertical,
        })
    }
}
mod list;
mod surface {
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

    fn render_header(
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        title: &str,
        state: &SelectionState,
    ) {
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
}

pub use layout::recommended_list_modal_height;
pub use surface::render_selection_surface;
