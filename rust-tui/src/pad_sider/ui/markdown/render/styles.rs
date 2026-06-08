use super::state::Renderer;
use crate::pad_sider::ui::markdown::style::{code_block_style, heading_style, inline_code_style};
use ratatui::style::{Color, Modifier, Style};

impl Renderer {
    pub(in crate::pad_sider::ui::markdown) fn current_style(&self) -> Style {
        let mut style = if let Some(level) = self.heading_level {
            heading_style(level)
        } else {
            Style::default().fg(Color::White)
        };
        if self.emphasis_depth > 0 {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if self.strong_depth > 0 {
            style = style.add_modifier(Modifier::BOLD);
        }
        if self.strike_depth > 0 {
            style = style.add_modifier(Modifier::CROSSED_OUT);
        }
        if self.link_depth > 0 {
            style = style.fg(Color::Cyan).add_modifier(Modifier::UNDERLINED);
        }
        style
    }

    pub(in crate::pad_sider::ui::markdown) fn inline_code_style(&self) -> Style {
        inline_code_style()
    }
}

pub(super) fn code_block_style_for(renderer: &Renderer) -> Style {
    code_block_style(renderer.code_language.as_deref())
}
