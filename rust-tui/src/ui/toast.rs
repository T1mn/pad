use crate::app::App;
use crate::ui::layout_rules::{
    clamp_copy_toast_width, COPY_TOAST_HEIGHT, COPY_TOAST_RIGHT_MARGIN, COPY_TOAST_TOP_MARGIN,
};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn draw_copy_toast(f: &mut Frame, app: &App) {
    let Some(toast) = app.preview.copy_toast.as_ref() else {
        return;
    };

    let title_width = toast.title.chars().count();
    let content_width = toast.content_preview.chars().count();
    let width = clamp_copy_toast_width(title_width.max(content_width));
    let area = f.area();
    if area.width <= width + COPY_TOAST_RIGHT_MARGIN
        || area.height <= COPY_TOAST_HEIGHT + COPY_TOAST_TOP_MARGIN + 1
    {
        return;
    }

    let card_area = Rect::new(
        area.x + area.width - width - COPY_TOAST_RIGHT_MARGIN,
        area.y + COPY_TOAST_TOP_MARGIN,
        width,
        COPY_TOAST_HEIGHT,
    );
    let shadow_area = Rect::new(
        card_area.x.saturating_add(1),
        card_area.y.saturating_add(1),
        card_area.width,
        card_area.height,
    );

    let shadow = Block::default().style(Style::default().bg(app.theme.bg));
    f.render_widget(shadow, shadow_area);
    f.render_widget(Clear, card_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.border_focused))
        .style(
            Style::default()
                .bg(app.theme.highlight_bg)
                .fg(app.theme.highlight_fg),
        );
    let inner = block.inner(card_area);
    f.render_widget(block, card_area);

    let content = vec![
        Line::from(Span::styled(
            toast.title.clone(),
            Style::default()
                .fg(app.theme.highlight_fg)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            toast.content_preview.clone(),
            Style::default().fg(app.theme.comment),
        )),
    ];
    let paragraph = Paragraph::new(content).alignment(Alignment::Left).style(
        Style::default()
            .bg(app.theme.highlight_bg)
            .fg(app.theme.highlight_fg),
    );
    f.render_widget(paragraph, inner);
}
