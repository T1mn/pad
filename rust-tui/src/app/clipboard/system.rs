use std::io::{self, Write};
use std::process::{Command, Stdio};

pub fn read_text_from_clipboard() -> io::Result<String> {
    let mut candidates: Vec<(&str, Vec<&str>)> = Vec::new();

    if cfg!(target_os = "macos") {
        candidates.push(("pbpaste", Vec::new()));
    }

    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        candidates.push(("wl-paste", vec!["--no-newline"]));
    }

    if std::env::var_os("DISPLAY").is_some() {
        candidates.push(("xclip", vec!["-selection", "clipboard", "-out"]));
        candidates.push(("xsel", vec!["--clipboard", "--output"]));
    }

    if std::env::var_os("TMUX").is_some() {
        candidates.push(("tmux", vec!["save-buffer", "-"]));
    }

    let mut last_error = None;
    for (program, args) in candidates {
        match read_clipboard_command(program, &args) {
            Ok(text) => return Ok(text),
            Err(err) => last_error = Some(err),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "no supported clipboard read command is available",
        )
    }))
}

pub(super) fn copy_text_to_clipboard(text: &str) -> io::Result<()> {
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
            Err(err) => last_error = Some(err),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "no supported clipboard command is available",
        )
    }))
}

fn read_clipboard_command(program: &str, args: &[&str]) -> io::Result<String> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(io::Error::other(format!(
            "{} exited with status {}",
            program, output.status
        )))
    }
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
