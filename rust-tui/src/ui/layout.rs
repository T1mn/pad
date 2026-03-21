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

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
