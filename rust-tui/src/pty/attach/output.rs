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
