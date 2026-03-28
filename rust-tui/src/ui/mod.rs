pub mod layout;
pub mod modals;
pub mod panel_list;
pub mod preview;
pub mod status_bar;

use crate::app::state::Mode;
use crate::app::App;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &mut App) {
    // Apply global background color from theme
    let bg_block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(app.theme.bg));
    f.render_widget(bg_block, f.area());

    let preferred_left_width = if app.show_tree {
        None
    } else {
        Some(panel_list::preferred_panel_width(app))
    };
    let (main_layout, body_layout) =
        layout::compute_layout(f.area(), app.show_tree, preferred_left_width);

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
    if app.mode == Mode::RelaySettings {
        modals::draw_relay_settings(f, app);
        // DetailPane is a third-level popup on top of relay settings
        if app.relay_view == crate::app::state::RelayView::DetailPane {
            modals::draw_relay_detail(f, app);
        }
    }

    if app.mode == Mode::LanguageSelector {
        modals::draw_language_selector(f, app);
    }

    if app.mode == Mode::AgentStyleSettings {
        modals::draw_agent_style_modal(f, app);
    }

    if app.mode == Mode::TelegramSettings {
        modals::draw_telegram_settings_modal(f, app);
    }

    draw_copy_toast(f, app);
}

fn draw_copy_toast(f: &mut Frame, app: &App) {
    let Some(toast) = app.copy_toast.as_ref() else {
        return;
    };

    let title_width = toast.title.chars().count();
    let content_width = toast.content_preview.chars().count();
    let width = (title_width.max(content_width) as u16 + 4).clamp(18, 32);
    let height = 4u16;
    let area = f.area();
    if area.width <= width + 2 || area.height <= height + 2 {
        return;
    }

    let card_area = Rect::new(area.x + area.width - width - 2, area.y + 1, width, height);
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
