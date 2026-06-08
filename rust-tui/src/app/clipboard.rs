mod system;
mod toast;

use super::App;
use crate::log_debug;
use system::copy_text_to_clipboard;
pub use system::read_text_from_clipboard;

impl App {
    pub fn show_copy_toast(&mut self, label: &str, content: &str) {
        self.show_action_toast(&format!("已复制{}", label), content);
    }

    pub fn show_action_toast(&mut self, title: &str, content: &str) {
        toast::show_action_toast(&mut self.preview.copy_toast, title, content);
        self.dirty = true;
    }

    pub fn expire_copy_toast_if_needed(&mut self) -> bool {
        if toast::copy_toast_expired(&self.preview.copy_toast) {
            self.preview.copy_toast = None;
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn copy_text_with_toast(&mut self, label: &str, text: &str) -> bool {
        let trimmed = text.trim_matches('\n');
        if trimmed.trim().is_empty() {
            return false;
        }

        match copy_text_to_clipboard(trimmed) {
            Ok(()) => {
                self.show_copy_toast(label, trimmed);
                true
            }
            Err(err) => {
                log_debug!("clipboard: copy failed: {}", err);
                false
            }
        }
    }
}

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod tests;
