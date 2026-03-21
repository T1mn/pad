use crate::log_debug;
use crate::model::AgentPanel;
use std::error::Error;
use std::io::{self, Read, Write};
use std::process::Command;
use std::time::{Duration, Instant};

/// Find detach key in buffer using multiple encoding formats
pub fn find_detach_key(data: &[u8], detach_byte: u8) -> Option<usize> {
    if let Some(pos) = data.iter().position(|&b| b == detach_byte) {
        return Some(pos);
    }

    let key_code = match detach_byte {
        1..=26 => detach_byte + 96,
        28..=31 => detach_byte + 64,
        _ => 0,
    };

    if key_code > 0 {
        let xterm_seq = format!("\x1b[27;5;{}~", key_code);
        if let Some(pos) = data
            .windows(xterm_seq.len())
            .position(|w| w == xterm_seq.as_bytes())
        {
            return Some(pos);
        }

        let kitty_seq = format!("\x1b[{};5u", key_code);
        if let Some(pos) = data
            .windows(kitty_seq.len())
            .position(|w| w == kitty_seq.as_bytes())
        {
            return Some(pos);
        }
    }

    None
}

/// Find F12 key sequence
pub fn find_f12_key(data: &[u8]) -> Option<usize> {
    if let Some(pos) = data
        .windows(5)
        .position(|w| w == &[0x1b, b'[', b'2', b'4', b'~'])
    {
        return Some(pos);
    }

    if data.len() >= 6 {
        if let Some(pos) = data
            .windows(4)
            .position(|w| w == &[0x1b, b'[', b'2', b'4'])
        {
            if data.len() > pos + 4 && data[pos + 4] == b';' {
                for i in (pos + 5)..data.len() {
                    if data[i] == b'~' {
                        return Some(pos);
                    }
                }
            }
        }
    }

    None
}

/// Capture tmux pane content
pub fn capture_pane(pane_id: &str, lines: usize) -> Result<String, Box<dyn Error>> {
    let output = Command::new("tmux")
        .args(["capture-pane", "-p", "-t", pane_id, "-S", &format!("-{}", lines)])
        .output()?;

    if output.status.success() {
        Ok(crate::scanner::strip_ansi(&String::from_utf8_lossy(&output.stdout)))
    } else {
        Err("Failed to capture pane".into())
    }
}

/// Attach to a tmux pane using PTY with creack/pty (Unix-only)
#[cfg(unix)]
pub fn attach_to_pane_pty(panel: &AgentPanel) -> Result<(), Box<dyn Error>> {
    use pty::fork::Fork;
    use std::os::fd::BorrowedFd;
    use std::os::unix::io::AsRawFd;
    use std::os::unix::process::CommandExt;
    use nix::sys::termios;

    #[repr(C)]
    #[derive(Debug)]
    struct Winsize {
        ws_row: u16,
        ws_col: u16,
        ws_xpixel: u16,
        ws_ypixel: u16,
    }

    let target = format!("{}:{}", panel.session, panel.window_index);
    log_debug!("pty: attach target={} pane_id={}", target, panel.pane_id);

    let (cols, rows) = crossterm::terminal::size()
        .map(|(w, h)| (w as u16, h as u16))
        .unwrap_or((80, 24));
    log_debug!("pty: terminal size {}x{}", cols, rows);

    const TERMINAL_STYLE_RESET: &str = "\x1b]8;;\x1b\\\x1b[0m\x1b[24m\x1b[39m\x1b[49m";

    let fork = Fork::from_ptmx()?;

    match fork.is_parent() {
        Ok(mut master) => {
            log_debug!("pty: fork parent, master_fd={}", master.as_raw_fd());
            let master_fd = master.as_raw_fd();
            let winsize = Winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            unsafe {
                libc::ioctl(master_fd, libc::TIOCSWINSZ, &winsize);
            }
            let stdin = io::stdin();
            let stdin_fd = stdin.as_raw_fd();

            let master_borrowed = unsafe { BorrowedFd::borrow_raw(master_fd) };
            let stdin_borrowed = unsafe { BorrowedFd::borrow_raw(stdin_fd) };

            let pty_orig_termios = termios::tcgetattr(master_borrowed)?;
            let mut pty_raw = pty_orig_termios.clone();
            termios::cfmakeraw(&mut pty_raw);
            termios::tcsetattr(master_borrowed, termios::SetArg::TCSAFLUSH, &pty_raw)?;

            let stdin_orig_termios = termios::tcgetattr(stdin_borrowed)?;
            let mut stdin_raw = stdin_orig_termios.clone();
            termios::cfmakeraw(&mut stdin_raw);
            termios::tcsetattr(stdin_borrowed, termios::SetArg::TCSAFLUSH, &stdin_raw)?;

            use std::sync::atomic::{AtomicBool, Ordering};
            use std::sync::Arc;

            let should_exit = Arc::new(AtomicBool::new(false));
            let should_exit_clone = should_exit.clone();

            let master_fd_for_output = unsafe { libc::dup(master_fd) };

            std::thread::spawn(move || {
                let mut pty_buf = [0u8; 1024];
                let mut stdout = io::stdout();

                loop {
                    if should_exit_clone.load(Ordering::Relaxed) {
                        break;
                    }

                    let n = unsafe {
                        libc::read(
                            master_fd_for_output,
                            pty_buf.as_mut_ptr() as *mut libc::c_void,
                            pty_buf.len(),
                        )
                    };

                    if n <= 0 {
                        std::thread::sleep(Duration::from_millis(1));
                        continue;
                    }

                    let n = n as usize;
                    if stdout.write_all(&pty_buf[..n]).is_err() {
                        break;
                    }
                    let _ = stdout.flush();
                }

                unsafe {
                    libc::close(master_fd_for_output);
                }
            });

            let mut stdin = io::stdin();
            let mut buf = [0u8; 256];
            let start_time = Instant::now();
            const CONTROL_SEQ_TIMEOUT: Duration = Duration::from_millis(50);

            loop {
                match stdin.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if start_time.elapsed() < CONTROL_SEQ_TIMEOUT {
                            continue;
                        }

                        let mut detach_idx = None;

                        if let Some(idx) = find_detach_key(&buf[..n], 0x11) {
                            detach_idx = Some(idx);
                        } else if let Some(idx) = find_f12_key(&buf[..n]) {
                            detach_idx = Some(idx);
                        } else if let Some(idx) = find_detach_key(&buf[..n], 0x03) {
                            detach_idx = Some(idx);
                        }

                        if let Some(idx) = detach_idx {
                            if idx > 0 {
                                let _ = master.write_all(&buf[..idx]);
                                let _ = master.flush();
                            }
                            should_exit.store(true, Ordering::Relaxed);
                            break;
                        }

                        if master.write_all(&buf[..n]).is_err() {
                            should_exit.store(true, Ordering::Relaxed);
                            break;
                        }
                        let _ = master.flush();
                    }
                    Err(_) => {
                        should_exit.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            }

            let _ = termios::tcsetattr(
                master_borrowed,
                termios::SetArg::TCSAFLUSH,
                &pty_orig_termios,
            );
            let _ = termios::tcsetattr(
                stdin_borrowed,
                termios::SetArg::TCSAFLUSH,
                &stdin_orig_termios,
            );

            print!("{}", TERMINAL_STYLE_RESET);
            let _ = io::stdout().flush();
            log_debug!("pty: detached, restoring terminal");

            Ok(())
        }
        Err(_) => {
            log_debug!("pty: fork failed, falling back to tmux attach");
            let err = std::process::Command::new("tmux")
                .args(["attach-session", "-t", &target])
                .exec();

            Err(format!("Failed to exec tmux: {}", err).into())
        }
    }
}

/// Non-Unix stub
#[cfg(not(unix))]
pub fn attach_to_pane_pty(_panel: &AgentPanel) -> Result<(), Box<dyn Error>> {
    Err("PTY attach is only supported on Unix systems".into())
}
