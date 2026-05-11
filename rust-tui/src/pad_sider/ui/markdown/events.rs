use super::render::{ItemPrefix, ListState, Renderer};
use super::style::heading_level;
use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

impl Renderer {
    pub(super) fn handle_event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => self.push_text(&text),
            Event::Code(code) => self.push_span(code.into_string(), self.inline_code_style()),
            Event::Html(text) | Event::InlineHtml(text) => {
                self.push_span(text.into_string(), Style::default().fg(Color::DarkGray))
            }
            Event::InlineMath(text) | Event::DisplayMath(text) => {
                self.push_span(text.into_string(), Style::default().fg(Color::Magenta))
            }
            Event::SoftBreak => self.push_text(" "),
            Event::HardBreak => self.flush_line(),
            Event::Rule => self.push_rule(),
            Event::TaskListMarker(done) => self.push_text(if done { "[x] " } else { "[ ] " }),
            Event::FootnoteReference(text) => {
                self.push_span(format!("[^{text}]"), Style::default().fg(Color::Yellow))
            }
        }
    }

    fn start_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => self.start_block(self.item_prefix.is_none()),
            Tag::Heading { level, .. } => {
                self.start_block(true);
                self.heading_level = Some(heading_level(level));
            }
            Tag::BlockQuote(_) => {
                self.start_block(true);
                self.blockquote_depth += 1;
            }
            Tag::CodeBlock(kind) => self.start_code_block(kind),
            Tag::List(start) => self.lists.push(ListState {
                ordered: start.is_some(),
                next_index: start.unwrap_or(1),
            }),
            Tag::Item => self.start_item(),
            Tag::Emphasis => self.emphasis_depth += 1,
            Tag::Strong => self.strong_depth += 1,
            Tag::Strikethrough => self.strike_depth += 1,
            Tag::Link { .. } => self.link_depth += 1,
            _ => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => self.flush_line(),
            TagEnd::Heading(_) => {
                self.flush_line();
                self.heading_level = None;
            }
            TagEnd::BlockQuote(_) => {
                self.flush_line();
                self.blockquote_depth = self.blockquote_depth.saturating_sub(1);
            }
            TagEnd::CodeBlock => {
                self.flush_line();
                self.code_block = false;
                self.code_language = None;
            }
            TagEnd::List(_) => {
                self.flush_line();
                self.lists.pop();
            }
            TagEnd::Item => {
                self.flush_line();
                self.item_prefix = None;
            }
            TagEnd::Emphasis => self.emphasis_depth = self.emphasis_depth.saturating_sub(1),
            TagEnd::Strong => self.strong_depth = self.strong_depth.saturating_sub(1),
            TagEnd::Strikethrough => self.strike_depth = self.strike_depth.saturating_sub(1),
            TagEnd::Link => self.link_depth = self.link_depth.saturating_sub(1),
            _ => {}
        }
    }

    fn start_block(&mut self, _separated: bool) {
        self.flush_line();
    }

    fn start_code_block(&mut self, kind: CodeBlockKind<'_>) {
        self.start_block(true);
        self.code_block = true;
        self.code_language = match kind {
            CodeBlockKind::Fenced(lang) => first_code_language(&lang),
            CodeBlockKind::Indented => None,
        };
    }

    fn start_item(&mut self) {
        self.flush_line();
        let depth = self.lists.len().saturating_sub(1);
        let indent = "  ".repeat(depth);
        let marker = if let Some(list) = self.lists.last_mut() {
            if list.ordered {
                let marker = format!("{}.", list.next_index);
                list.next_index += 1;
                marker
            } else {
                "•".to_string()
            }
        } else {
            "•".to_string()
        };
        self.item_prefix = Some(ItemPrefix {
            first: format!("{indent}{marker} "),
            continuation: format!("{indent}{} ", " ".repeat(marker.chars().count())),
            used_first: false,
        });
    }

    fn push_rule(&mut self) {
        self.start_block(true);
        self.lines.push(Line::from(Span::styled(
            "─".repeat(48),
            Style::default().fg(Color::DarkGray),
        )));
    }
}

fn first_code_language(info: &str) -> Option<String> {
    info.split_whitespace()
        .next()
        .map(|lang| lang.trim_start_matches('.').to_ascii_lowercase())
        .filter(|lang| !lang.is_empty())
}
