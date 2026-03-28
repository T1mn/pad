pub mod layout;
pub mod layout_rules;
pub mod modals;
pub mod panel_list;
pub mod preview;
pub mod selection;
pub mod status_bar;
pub mod toast;

use crate::app::state::Mode;
use crate::app::App;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &mut App) {
    // Apply global background color from theme
    let bg_block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(app.theme.bg));
    f.render_widget(bg_block, f.area());

    let preferred_left_width = if app.sidebar.show_tree {
        None
    } else {
        Some(panel_list::preferred_panel_width(app))
    };
    let (main_layout, body_layout) =
        layout::compute_layout(f.area(), app.sidebar.show_tree, preferred_left_width);

    if app.sidebar.show_tree {
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
    } else if app.theme_selector_open {
        modals::draw_theme_selector(f, app);
    }

    if let Some(ref launcher) = app.sidebar.agent_launcher {
        modals::draw_agent_launcher(f, app, launcher, f.area());
    }

    if app.mode == Mode::DeleteConfirm {
        modals::draw_delete_confirm(f, app, f.area());
    }

    if app.mode == Mode::ThreadActionConfirm {
        modals::draw_thread_action_confirm(f, app, f.area());
    }

    if app.mode == Mode::Help {
        modals::draw_help(f, app, f.area());
    }

    // Render FuzzyPicker modal overlay
    if let Some(ref picker) = app.fuzzy_picker {
        picker.draw(f);
    }

    // Render RelaySettings modal overlay
    if !app.settings_open && app.mode == Mode::RelaySettings {
        modals::draw_relay_settings(f, app);
        // DetailPane is a third-level popup on top of relay settings
        if app.relay_view == crate::app::state::RelayView::DetailPane {
            modals::draw_relay_detail(f, app);
        }
    }

    if !app.settings_open && app.mode == Mode::LanguageSelector {
        modals::draw_language_selector(f, app);
    }

    if !app.settings_open && app.mode == Mode::AgentStyleSettings {
        modals::draw_agent_style_modal(f, app);
    }

    if !app.settings_open && app.mode == Mode::TelegramSettings {
        modals::draw_telegram_settings_modal(f, app);
    }

    toast::draw_copy_toast(f, app);
}
