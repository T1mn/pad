#[cfg(unix)]
mod input {
    #[cfg(unix)]
    pub(super) fn forward_stdin_to_pty<W: std::io::Write>(
        master: &mut W,
        should_exit: &std::sync::atomic::AtomicBool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use super::super::keys::{find_detach_key, find_f12_key};
        use std::io::{self, Read};
        use std::sync::atomic::Ordering;
        use std::time::{Duration, Instant};

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

                    let detach_idx = find_detach_key(&buf[..n], 0x11)
                        .or_else(|| find_f12_key(&buf[..n]))
                        .or_else(|| find_detach_key(&buf[..n], 0x03));

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

        Ok(())
    }
}
#[cfg(unix)]
mod output {
    #[cfg(unix)]
    pub(super) fn spawn_pty_output_forwarder(
        master_fd: i32,
        should_exit: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) {
        use std::io::{self, Write};
        use std::sync::atomic::Ordering;
        use std::time::Duration;

        let master_fd_for_output = unsafe { libc::dup(master_fd) };

        std::thread::spawn(move || {
            let mut pty_buf = [0u8; 1024];
            let mut stdout = io::stdout();

            loop {
                if should_exit.load(Ordering::Relaxed) {
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
    }
}

use crate::model::AgentPanel;
use std::error::Error;

/// Attach to a tmux pane using PTY with creack/pty (Unix-only).
#[cfg(unix)]
pub fn attach_to_pane_pty(panel: &AgentPanel) -> Result<(), Box<dyn Error>> {
    use nix::sys::termios;
    use std::io::{self, Write};
    use std::os::fd::BorrowedFd;
    use std::os::unix::io::AsRawFd;
    use std::os::unix::process::CommandExt;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    use input::forward_stdin_to_pty;
    use output::spawn_pty_output_forwarder;

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

    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    log_debug!("pty: terminal size {}x{}", cols, rows);

    const TERMINAL_STYLE_RESET: &str = "\x1b]8;;\x1b\\\x1b[0m\x1b[24m\x1b[39m\x1b[49m";

    let fork = ::pty::fork::Fork::from_ptmx()?;

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

            let should_exit = Arc::new(AtomicBool::new(false));
            spawn_pty_output_forwarder(master_fd, should_exit.clone());
            forward_stdin_to_pty(&mut master, &should_exit)?;

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

/// Non-Unix stub.
#[cfg(not(unix))]
pub fn attach_to_pane_pty(_panel: &AgentPanel) -> Result<(), Box<dyn Error>> {
    Err("PTY attach is only supported on Unix systems".into())
}
