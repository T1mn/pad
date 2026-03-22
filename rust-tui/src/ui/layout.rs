use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Returns (main_layout, body_layout)
/// main_layout[0] = body, main_layout[1] = status bar
/// body_layout: always 2 columns (left + right)
///   show_tree=false: [agents, preview]
///   show_tree=true:  [tree_area, file_preview]  (tree_area will be split vertically by caller)
pub fn compute_layout(area: Rect, _show_tree: bool) -> (Vec<Rect>, Vec<Rect>) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let body_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_layout[0]);

    (main_layout.to_vec(), body_layout.to_vec())
}

/// Split the left column for tree mode: file tree + agent status bar
pub fn split_tree_left(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Min(0), Constraint::Length(3)])
        .split(area)
        .to_vec()
}

/// Compute a centered popup Rect from content dimensions.
/// Adds 2 for borders. Clamps to terminal bounds with margin.
pub fn popup_area(content_w: u16, content_h: u16, terminal: Rect) -> Rect {
    let w = (content_w + 2).min(terminal.width.saturating_sub(2));
    let h = (content_h + 2).min(terminal.height.saturating_sub(2));
    let x = terminal.x + (terminal.width.saturating_sub(w)) / 2;
    let y = terminal.y + (terminal.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}
