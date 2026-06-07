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
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            DisableFocusChange,
            DisableBracketedPaste
        );
        let msg = format!("PANIC: {}", info);
        eprintln!("{}", msg);
        crate::logger::log(&msg);
    }));
}

pub fn enter(focus_events_supported: bool) -> Result<TerminalHandle, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableFocusChange,
        EnableBracketedPaste
    )?;

    if focus_events_supported {
        let _ = std::process::Command::new("tmux")
            .args(["set", "-g", "focus-events", "on"])
            .output();
    } else {
        log_debug!("tmux_probe: skip focus-events enable because capability is unavailable");
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
