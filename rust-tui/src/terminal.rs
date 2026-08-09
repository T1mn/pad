use crossterm::{
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture,
    },
    execute,
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::error::Error;
use std::io;

pub type TerminalHandle = Terminal<CrosstermBackend<io::Stdout>>;

const KITTY_PAD_ACTIVE: &str = "\x1b]1337;SetUserVar=pad=MQ==\x07";
const KITTY_PAD_INACTIVE: &str = "\x1b]1337;SetUserVar=pad\x07";

fn kitty_pad_marker(active: bool) -> Option<&'static str> {
    std::env::var_os("KITTY_WINDOW_ID").map(|_| {
        if active {
            KITTY_PAD_ACTIVE
        } else {
            KITTY_PAD_INACTIVE
        }
    })
}

pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("PANIC: {}", info);
        crate::logger::log(&msg);
        if panic_requires_terminal_restore() {
            let _ = disable_raw_mode();
            if let Some(marker) = kitty_pad_marker(false) {
                let _ = execute!(io::stdout(), Print(marker));
            }
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

pub fn enter() -> Result<TerminalHandle, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableFocusChange,
        EnableBracketedPaste
    )?;
    if let Some(marker) = kitty_pad_marker(true) {
        execute!(stdout, Print(marker))?;
    }

    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

pub fn restore(terminal: &mut TerminalHandle) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    if let Some(marker) = kitty_pad_marker(false) {
        execute!(terminal.backend_mut(), Print(marker))?;
    }
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
#[path = "terminal_tests.rs"]
pub(crate) mod tests;
