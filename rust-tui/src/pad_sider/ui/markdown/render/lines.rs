use super::state::Renderer;
use super::styles::code_block_style_for;
use crate::pad_sider::ui::markdown::style::code_block_prefix_style;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

impl Renderer {
    pub(in crate::pad_sider::ui::markdown) fn push_text(&mut self, text: &str) {
        if self.code_block {
            self.push_code_block_text(text);
            return;
        }
        if !text.is_empty() {
            self.push_span(text.to_string(), self.current_style());
        }
    }

    fn push_code_block_text(&mut self, text: &str) {
        for (index, line) in text.split('\n').enumerate() {
            if index > 0 {
                self.flush_line_or_blank();
            }
            if !line.is_empty() {
                self.push_span(line.to_string(), code_block_style_for(self));
            }
        }
    }

    pub(in crate::pad_sider::ui::markdown) fn ensure_prefix(&mut self) {
        if !self.current.is_empty() {
            return;
        }

        let mut prefix = "│ ".repeat(self.blockquote_depth);
        if self.code_block {
            prefix.push_str("  ");
        } else if let Some(item) = self.item_prefix.as_mut() {
            if item.used_first {
                prefix.push_str(&item.continuation);
            } else {
                prefix.push_str(&item.first);
                item.used_first = true;
            }
        }

        if !prefix.is_empty() {
            let style = if self.code_block {
                code_block_prefix_style(self.code_language)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            self.current.push(Span::styled(prefix, style));
        }
    }

    pub(in crate::pad_sider::ui::markdown) fn flush_line(&mut self) {
        if self.current.is_empty() {
            return;
        }
        self.lines
            .push(Line::from(std::mem::take(&mut self.current)));
    }

    pub(in crate::pad_sider::ui::markdown) fn flush_line_or_blank(&mut self) {
        if self.current.is_empty() {
            self.push_blank_line();
        } else {
            self.flush_line();
        }
    }

    fn push_blank_line(&mut self) {
        if self.lines.is_empty()
            || !crate::pad_sider::ui::markdown::style::is_blank_line(self.lines.last())
        {
            self.lines.push(Line::default());
        }
    }
}
