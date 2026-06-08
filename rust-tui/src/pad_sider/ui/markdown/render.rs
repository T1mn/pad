use pulldown_cmark::{Options, Parser};
use ratatui::text::Text;

mod lines;
mod state;
mod styles;

pub(super) use state::{ItemPrefix, ListState, Renderer};

pub fn render_markdown(input: &str) -> Text<'static> {
    Renderer::new().render(input)
}

impl Renderer {
    pub(in crate::pad_sider::ui::markdown) fn render(mut self, input: &str) -> Text<'static> {
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
}
