use std::io::{self, Write};
use std::process::{Command, Stdio};

pub fn read_text_from_clipboard() -> io::Result<String> {
    let mut last_error = None;

    if cfg!(target_os = "macos") {
        if let Some(text) =
            remember_clipboard_result(read_clipboard_command("pbpaste", &[]), &mut last_error)
        {
            return Ok(text);
        }
    }

    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        if let Some(text) = remember_clipboard_result(
            read_clipboard_command("wl-paste", &["--no-newline"]),
            &mut last_error,
        ) {
            return Ok(text);
        }
    }

    if std::env::var_os("DISPLAY").is_some() {
        if let Some(text) = remember_clipboard_result(
            read_clipboard_command("xclip", &["-selection", "clipboard", "-out"]),
            &mut last_error,
        ) {
            return Ok(text);
        }
        if let Some(text) = remember_clipboard_result(
            read_clipboard_command("xsel", &["--clipboard", "--output"]),
            &mut last_error,
        ) {
            return Ok(text);
        }
    }

    if std::env::var_os("TMUX").is_some() {
        if let Some(text) = remember_clipboard_result(
            read_clipboard_command("tmux", &["save-buffer", "-"]),
            &mut last_error,
        ) {
            return Ok(text);
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
    let mut last_error = None;

    if cfg!(target_os = "macos")
        && remember_clipboard_result(
            write_clipboard_command("pbcopy", &[], text),
            &mut last_error,
        )
        .is_some()
    {
        return Ok(());
    }

    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        && remember_clipboard_result(
            write_clipboard_command("wl-copy", &[], text),
            &mut last_error,
        )
        .is_some()
    {
        return Ok(());
    }

    if std::env::var_os("DISPLAY").is_some() {
        if remember_clipboard_result(
            write_clipboard_command("xclip", &["-selection", "clipboard"], text),
            &mut last_error,
        )
        .is_some()
        {
            return Ok(());
        }
        if remember_clipboard_result(
            write_clipboard_command("xsel", &["--clipboard", "--input"], text),
            &mut last_error,
        )
        .is_some()
        {
            return Ok(());
        }
    }

    if std::env::var_os("TMUX").is_some()
        && remember_clipboard_result(
            write_clipboard_command("tmux", &["load-buffer", "-w", "-"], text),
            &mut last_error,
        )
        .is_some()
    {
        return Ok(());
    }

    Err(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "no supported clipboard command is available",
        )
    }))
}

fn remember_clipboard_result<T>(
    result: io::Result<T>,
    last_error: &mut Option<io::Error>,
) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(err) => {
            *last_error = Some(err);
            None
        }
    }
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
