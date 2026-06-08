use crate::app::App;
use crate::theme::Theme;
use ratatui::style::{Modifier, Style};

pub(super) struct RelayEditState<'a> {
    pub(super) editing: bool,
    pub(super) field: usize,
    pub(super) buffer: &'a str,
    theme: &'a Theme,
}

impl<'a> RelayEditState<'a> {
    pub(super) fn from_app(app: &'a App, theme: &'a Theme) -> Self {
        Self {
            editing: app.relay_editing,
            field: app.relay_edit_field,
            buffer: &app.relay_edit_buffer,
            theme,
        }
    }

    pub(super) fn value(&self, idx: usize, value: &str) -> String {
        if self.editing && self.field == idx {
            format!("{}|", self.buffer)
        } else if value.is_empty() {
            "-".to_string()
        } else {
            value.to_string()
        }
    }

    pub(super) fn field_style(&self, idx: usize) -> Style {
        if self.field == idx {
            Style::default()
                .fg(self.theme.highlight_fg)
                .bg(self.theme.highlight_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.theme.fg)
        }
    }
}
