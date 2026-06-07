mod body;
mod labels;
mod page;
mod payload;
mod text;

pub(in crate::chat::providers::telegram) use page::HelpPage;
pub(in crate::chat::providers::telegram) use payload::help_message_payload;
#[cfg(test)]
pub(in crate::chat::providers::telegram) use payload::{build_help_keyboard, help_page_html};
