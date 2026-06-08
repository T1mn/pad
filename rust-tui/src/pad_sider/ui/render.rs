mod header;
mod nav;

use super::super::app::App;
use super::file_preview;
use super::overlay;
use super::split;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Borders},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    if let Some(preview) = app.preview.as_ref() {
        overlay::draw_preview(frame, app, preview);
        return;
    }

    let (left_area, preview_area) = split::split_columns(frame.area());
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(left_area);

    header::draw_header(frame, app, left[0]);
    nav::draw_nav(frame, app, left[1]);
    file_preview::draw_file_preview(frame, app, preview_area);

    if let Some(search) = app.search.as_ref() {
        overlay::draw_search(frame, search);
    }
    if app.show_help {
        overlay::draw_help(frame);
    }
}

pub(super) fn focus_block(title: &str, focused: bool) -> Block<'static> {
    let mut block = Block::default()
        .title(title.to_string())
        .borders(Borders::ALL);
    if focused {
        block = block.border_style(Style::default().add_modifier(Modifier::BOLD));
    }
    block
}
