pub mod layout;
pub mod modals;
pub mod panel_list;
pub mod preview;
pub mod status_bar;

use crate::app::App;
use crate::app::state::Mode;
use ratatui::Frame;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};

pub fn draw(f: &mut Frame, app: &mut App) {
    // Apply global background color from theme
    let bg_block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(app.theme.bg));
    f.render_widget(bg_block, f.area());

    let (main_layout, body_layout) = layout::compute_layout(f.area(), app.show_tree);

    if app.show_tree {
        // Tree mode: left column = file tree + agent status bar, right = file preview
        let left_split = layout::split_tree_left(body_layout[0]);
        panel_list::draw_file_tree(f, app, left_split[0]);
        panel_list::draw_agent_status_bar(f, app, left_split[1]);
        preview::draw_file_preview(f, app, body_layout[1]);
    } else {
        // Normal mode: left = agents panel, right = agent preview
        panel_list::draw_panel_list(f, app, body_layout[0]);
        preview::draw_preview(f, app, body_layout[1]);
    }

    status_bar::draw_status_bar(f, app, main_layout[1]);

    if app.settings_open {
        modals::draw_settings_modal(f, app);
    }

    if app.theme_selector_open {
        modals::draw_theme_selector(f, app);
    }

    if let Some(ref launcher) = app.agent_launcher {
        modals::draw_agent_launcher(f, launcher, f.area());
    }

    if app.mode == Mode::DeleteConfirm {
        modals::draw_delete_confirm(f, app, f.area());
    }

    if app.mode == Mode::Help {
        modals::draw_help(f, &app.theme, f.area());
    }
}
