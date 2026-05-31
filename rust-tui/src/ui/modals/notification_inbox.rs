use super::common::render_modal_surface;
use crate::app::App;
use crate::notification_inbox::{short_time, NotificationEntry};
use crate::ui::layout::popup_area;
use crate::ui::selection::{render::render_selection_surface, SelectionItem, SelectionState};
use ratatui::{layout::Rect, Frame};

pub fn draw_notification_inbox(f: &mut Frame, app: &App) {
    let count = app.notification_inbox.entries.len() as u16;
    let height = (count.max(1) * 2 + 4).clamp(10, 24);
    let area = popup_area(86, height, f.area());
    render_modal_surface(f, area, &app.theme);
    draw_notification_inbox_content(f, app, inner(area));
}

fn draw_notification_inbox_content(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<SelectionItem> = app
        .notification_inbox
        .entries
        .iter()
        .map(notification_item)
        .collect();
    let mut state = SelectionState {
        selected: app.notification_inbox_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    let unread = app.notification_inbox.unread_count();
    let title = format!(
        "Notification Inbox · {} total · {} unread",
        app.notification_inbox.entries.len(),
        unread
    );
    render_selection_surface(
        f,
        area,
        &app.theme,
        &title,
        &items,
        &state,
        Some("j/k move · Enter/m mark read · a mark all · d delete · Esc close"),
    );
}

fn notification_item(entry: &NotificationEntry) -> SelectionItem {
    let state = if entry.read { "read" } else { "unread" };
    SelectionItem {
        title: format!("{} {}", if entry.read { " " } else { "●" }, entry.title),
        value: Some(short_time(entry.ts)),
        subtitle: Some(format!("{} · {} · {}", state, entry.agent_type, entry.body)),
        keyword: Some(format!(
            "{} {} {} {} {:?} {:?}",
            entry.event,
            entry.agent_type,
            entry.title,
            entry.body,
            entry.working_dir,
            entry.session_id
        )),
        detail: None,
        disabled: false,
    }
}

fn inner(area: Rect) -> Rect {
    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}
