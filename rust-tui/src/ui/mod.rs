pub(crate) mod codex_sidebar;
pub mod layout;
pub mod layout_rules {
    pub const COPY_TOAST_MIN_WIDTH: u16 = 18;
    pub const COPY_TOAST_MAX_WIDTH: u16 = 32;
    pub const COPY_TOAST_HEIGHT: u16 = 4;
    pub const COPY_TOAST_RIGHT_MARGIN: u16 = 2;
    pub const COPY_TOAST_TOP_MARGIN: u16 = 1;

    pub fn clamp_copy_toast_width(content_width: usize) -> u16 {
        (content_width as u16 + 4).clamp(COPY_TOAST_MIN_WIDTH, COPY_TOAST_MAX_WIDTH)
    }
}
pub mod modals;
pub mod panel_list;
pub mod preview;
pub mod selection;
pub mod status_bar;
pub mod terminal;
pub mod toast {
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
}

use crate::app::state::Mode;
use crate::app::App;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

pub fn terminal_viewport_size(
    app: &mut App,
    area: ratatui::layout::Rect,
) -> crate::terminal_runtime::TerminalSize {
    let preferred_left_width = if app.sidebar.collapsed {
        Some(0)
    } else if app.sidebar.show_tree {
        None
    } else {
        Some(panel_list::preferred_panel_width(app))
    };
    let (_, body_layout) =
        layout::compute_layout(area, app.sidebar.show_tree, preferred_left_width);
    let terminal_area = body_layout[1];
    let placement = terminal::placement(app, terminal_area);
    let focused = app.focused_terminal_pane_id();
    let inner = placement
        .panes
        .iter()
        .find(|pane| focused == Some(pane.pane_id))
        .or_else(|| placement.panes.first())
        .map(|pane| pane.inner)
        .unwrap_or_default();
    crate::terminal_runtime::TerminalSize::new(inner.width, inner.height)
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let frame_started = std::time::Instant::now();
    // Apply global background color from theme
    let bg_block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(app.theme.bg));
    f.render_widget(bg_block, f.area());

    let preferred_left_width = if app.sidebar.collapsed {
        Some(0)
    } else if app.sidebar.show_tree {
        None
    } else {
        Some(panel_list::preferred_panel_width(app))
    };
    let (main_layout, body_layout) =
        layout::compute_layout(f.area(), app.sidebar.show_tree, preferred_left_width);

    let left_started = std::time::Instant::now();
    if app.sidebar.show_tree && body_layout[0].width > 0 {
        // Tree mode: left column = file tree + agent status bar, right = file preview
        let left_split = layout::split_tree_left(body_layout[0]);
        panel_list::draw_file_tree(f, app, left_split[0]);
        panel_list::draw_agent_status_bar(f, app, left_split[1]);
    } else {
        // Normal mode: left = agents panel
        if body_layout[0].width > 0 {
            panel_list::draw_panel_list(f, app, body_layout[0]);
        }
    }
    let left_elapsed = left_started.elapsed();

    let preview_started = std::time::Instant::now();
    if app.sidebar.show_tree {
        preview::draw_file_preview(f, app, body_layout[1]);
    } else {
        preview::draw_preview(f, app, body_layout[1]);
    }
    let preview_elapsed = preview_started.elapsed();

    let status_started = std::time::Instant::now();
    status_bar::draw_status_bar(f, app, main_layout[1]);
    let status_elapsed = status_started.elapsed();

    let body_elapsed = frame_started.elapsed();
    if body_elapsed >= std::time::Duration::from_millis(12) {
        crate::log_debug!(
            "ui.draw.parts: total_ms={} left_ms={} preview_ms={} status_ms={} preview_source={:?} turns={}",
            body_elapsed.as_millis(),
            left_elapsed.as_millis(),
            preview_elapsed.as_millis(),
            status_elapsed.as_millis(),
            app.preview.source,
            app.preview.turns.len()
        );
    }

    if app.settings_open {
        modals::draw_settings_modal(f, app);
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

    if app.mode == Mode::NotificationInbox {
        modals::draw_notification_inbox(f, app);
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

    if !app.settings_open && app.mode == Mode::TelegramSettings {
        modals::draw_telegram_settings_modal(f, app);
    }

    toast::draw_copy_toast(f, app);
}
