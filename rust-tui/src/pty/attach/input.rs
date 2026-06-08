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
