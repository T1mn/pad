use crate::sidebar::SidebarThread;
use crate::ui::panel_list::animation::{
    breathing_badge_style, breathing_badge_text, thread_badge_breathes,
};
use crate::ui::panel_list::style::badge_color;
use ratatui::style::Style;

pub(super) struct BadgeSpan {
    pub(super) text: &'static str,
    pub(super) style: Style,
}

pub(super) fn render_badge(
    thread: &SidebarThread,
    theme: &crate::theme::Theme,
    card_bg: ratatui::style::Color,
) -> BadgeSpan {
    let badge_color = badge_color(thread.agent_type.clone(), theme);
    let is_working = thread_badge_breathes(&thread.state);
    BadgeSpan {
        text: if is_working {
            breathing_badge_text()
        } else {
            "• "
        },
        style: if is_working {
            breathing_badge_style(badge_color, card_bg, card_bg)
        } else {
            Style::default().fg(badge_color).bg(card_bg)
        },
    }
}
