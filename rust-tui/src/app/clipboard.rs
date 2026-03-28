use super::{App, CopyToast};
use crate::log_debug;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

impl App {
    pub fn show_copy_toast(&mut self, label: &str, content: &str) {
        self.show_action_toast(&format!("已复制{}", label), content);
    }

    pub fn show_action_toast(&mut self, title: &str, content: &str) {
        self.copy_toast = Some(CopyToast {
            title: title.to_string(),
            content_preview: summarize_copy_preview(content, 24),
            expires_at: Instant::now() + Duration::from_millis(1800),
        });
        self.dirty = true;
    }

    pub fn expire_copy_toast_if_needed(&mut self) -> bool {
        if self
            .copy_toast
            .as_ref()
            .is_some_and(|toast| Instant::now() >= toast.expires_at)
        {
            self.copy_toast = None;
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

fn summarize_copy_preview(text: &str, max_chars: usize) -> String {
    let condensed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if condensed.is_empty() {
        return String::from("-");
    }

    let mut preview = String::new();
    for (idx, ch) in condensed.chars().enumerate() {
        if idx >= max_chars {
            preview.push_str("...");
            return preview;
        }
        preview.push(ch);
    }
    preview
}

fn copy_text_to_clipboard(text: &str) -> io::Result<()> {
    let mut candidates: Vec<(&str, Vec<&str>)> = Vec::new();

    if cfg!(target_os = "macos") {
        candidates.push(("pbcopy", Vec::new()));
    }

    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        candidates.push(("wl-copy", Vec::new()));
    }

    if std::env::var_os("DISPLAY").is_some() {
        candidates.push(("xclip", vec!["-selection", "clipboard"]));
        candidates.push(("xsel", vec!["--clipboard", "--input"]));
    }

    if std::env::var_os("TMUX").is_some() {
        candidates.push(("tmux", vec!["load-buffer", "-w", "-"]));
    }

    let mut last_error = None;
    for (program, args) in candidates {
        match write_clipboard_command(program, &args, text) {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                last_error = Some(err);
            }
            Err(err) => {
                last_error = Some(err);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "no supported clipboard command is available",
        )
    }))
}

fn write_clipboard_command(program: &str, args: &[&str], text: &str) -> io::Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }

    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{} exited with status {}",
            program, status
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::summarize_copy_preview;

    #[test]
    fn copy_preview_summary_truncates_with_ascii_ellipsis() {
        assert_eq!(
            summarize_copy_preview("hello brave new world", 5),
            "hello..."
        );
    }
}
