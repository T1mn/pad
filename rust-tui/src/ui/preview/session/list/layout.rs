use super::super::{SESSION_ITEM_CONTENT_HEIGHT, SESSION_ITEM_GAP_HEIGHT};
#[cfg(test)]
use super::card::{render_session_card, render_session_gap_line};
#[cfg(test)]
use crate::theme::Theme;
#[cfg(test)]
use ratatui::text::Line;

#[cfg(test)]
pub(crate) fn build_session_list_lines(
    turns: &[crate::model::PreviewTurn],
    selected_turn: Option<usize>,
    width: usize,
    theme: &Theme,
) -> (Vec<Line<'static>>, Option<(usize, usize)>) {
    let mut lines = Vec::new();
    let mut selected_range = None;

    for (idx, turn) in turns.iter().enumerate() {
        let start = lines.len();
        lines.extend(render_session_card(
            turn,
            selected_turn == Some(idx),
            width,
            theme,
        ));
        let end = lines.len().saturating_sub(1);
        if selected_turn == Some(idx) {
            selected_range = Some((start, end));
        }
        if idx + 1 < turns.len() {
            lines.push(render_session_gap_line(width, theme));
        }
    }

    (lines, selected_range)
}

pub(crate) fn session_list_total_lines(turn_count: usize) -> usize {
    if turn_count == 0 {
        0
    } else {
        turn_count * SESSION_ITEM_CONTENT_HEIGHT + (turn_count - 1) * SESSION_ITEM_GAP_HEIGHT
    }
}

pub(crate) fn session_turn_index_at_line(line: usize, turn_count: usize) -> Option<usize> {
    if line >= session_list_total_lines(turn_count) {
        return None;
    }

    let stride = SESSION_ITEM_CONTENT_HEIGHT + SESSION_ITEM_GAP_HEIGHT;
    let index = line / stride;
    let offset = line % stride;
    if offset < SESSION_ITEM_CONTENT_HEIGHT {
        Some(index)
    } else {
        None
    }
}
