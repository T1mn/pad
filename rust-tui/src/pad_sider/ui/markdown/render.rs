use super::style::{code_block_prefix_style, code_block_style, heading_style, inline_code_style};
use pulldown_cmark::{Options, Parser};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

pub fn render_markdown(input: &str) -> Text<'static> {
    Renderer::new().render(input)
}

pub(super) struct Renderer {
    pub(super) lines: Vec<Line<'static>>,
    pub(super) current: Vec<Span<'static>>,
    pub(super) blockquote_depth: usize,
    pub(super) lists: Vec<ListState>,
    pub(super) item_prefix: Option<ItemPrefix>,
    pub(super) heading_level: Option<u8>,
    pub(super) link_depth: usize,
    pub(super) emphasis_depth: usize,
    pub(super) strong_depth: usize,
    pub(super) strike_depth: usize,
    pub(super) code_block: bool,
    pub(super) code_language: Option<String>,
}

pub(super) struct ListState {
    pub(super) ordered: bool,
    pub(super) next_index: u64,
}

pub(super) struct ItemPrefix {
    pub(super) first: String,
    pub(super) continuation: String,
    pub(super) used_first: bool,
}

impl Renderer {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            current: Vec::new(),
            blockquote_depth: 0,
            lists: Vec::new(),
            item_prefix: None,
            heading_level: None,
            link_depth: 0,
            emphasis_depth: 0,
            strong_depth: 0,
            strike_depth: 0,
            code_block: false,
            code_language: None,
        }
    }

    fn render(mut self, input: &str) -> Text<'static> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_TASKLISTS);

        for event in Parser::new_ext(input, options) {
            self.handle_event(event);
        }
        self.flush_line();
        Text::from(self.lines)
    }

    pub(super) fn push_text(&mut self, text: &str) {
        if self.code_block {
            for (index, line) in text.split('\n').enumerate() {
                if index > 0 {
                    self.flush_line_or_blank();
                }
                if !line.is_empty() {
                    self.push_span(line.to_string(), self.code_block_style());
                }
            }
            return;
        }
        if !text.is_empty() {
            self.push_span(text.to_string(), self.current_style());
        }
    }

    pub(super) fn push_span(&mut self, text: String, style: Style) {
        self.ensure_prefix();
        self.current.push(Span::styled(text, style));
    }

    fn ensure_prefix(&mut self) {
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
                code_block_prefix_style(self.code_language.as_deref())
            } else {
                Style::default().fg(Color::DarkGray)
            };
            self.current.push(Span::styled(prefix, style));
        }
    }

    pub(super) fn flush_line(&mut self) {
        if self.current.is_empty() {
            return;
        }
        self.lines
            .push(Line::from(std::mem::take(&mut self.current)));
    }

    fn flush_line_or_blank(&mut self) {
        if self.current.is_empty() {
            self.push_blank_line();
        } else {
            self.flush_line();
        }
    }

    fn push_blank_line(&mut self) {
        if self.lines.is_empty() || !super::style::is_blank_line(self.lines.last()) {
            self.lines.push(Line::default());
        }
    }

    fn current_style(&self) -> Style {
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

    pub(super) fn inline_code_style(&self) -> Style {
        inline_code_style()
    }

    fn code_block_style(&self) -> Style {
        code_block_style(self.code_language.as_deref())
    }
}
