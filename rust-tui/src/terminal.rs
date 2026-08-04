use crossterm::{
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::error::Error;
use std::io;

pub type TerminalHandle = Terminal<CrosstermBackend<io::Stdout>>;

pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("PANIC: {}", info);
        crate::logger::log(&msg);
        if panic_requires_terminal_restore() {
            let _ = disable_raw_mode();
            let _ = execute!(
                io::stdout(),
                LeaveAlternateScreen,
                DisableMouseCapture,
                DisableFocusChange,
                DisableBracketedPaste
            );
            eprintln!("{}", msg);
        }
    }));
}

fn panic_requires_terminal_restore() -> bool {
    !crate::panic_boundary::is_isolated()
}

pub fn enter(configure_tmux_focus_events: bool) -> Result<TerminalHandle, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableFocusChange,
        EnableBracketedPaste
    )?;

    if configure_tmux_focus_events {
        let _ = std::process::Command::new("tmux")
            .args(["set", "-g", "focus-events", "on"])
            .output();
    }

    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

pub fn restore(terminal: &mut TerminalHandle) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableFocusChange,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::panic_requires_terminal_restore;

    #[test]
    fn isolated_worker_boundary_keeps_terminal_active() {
        assert!(panic_requires_terminal_restore());
        crate::panic_boundary::catch_isolated_unwind(|| {
            assert!(!panic_requires_terminal_restore());
        })
        .unwrap();
        assert!(panic_requires_terminal_restore());
    }
}
