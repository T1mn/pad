use std::path::Path;
use std::time::Duration;

use super::*;
use crate::terminal_runtime::TransportRuntime;

const TEST_TIMEOUT: Duration = Duration::from_secs(10);

#[cfg(unix)]
#[test]
fn native_pty_preserves_io_resize_env_and_exit() {
    assert!(Path::new("/bin/sh").is_file());
    let size = TerminalSize::new(31, 9);
    let command = NativePtyCommand::new("/bin/sh")
            .args([
                "-c",
                "stty -echo; printf 'READY\\r\\n'; IFS= read -r value; printf 'INPUT=%s\\r\\n' \"$value\"; printf 'SIZE='; stty size; printf 'ENV=%s\\r\\n' \"$PAD_NATIVE_TEST\"; exit 7",
            ])
            .env("PAD_NATIVE_TEST", "direct-pty");
    let transport = NativePtyTransport::new(
        TransportId::new("native-integration"),
        command,
        TerminalSize::new(20, 4),
    );
    let runtime = TransportRuntime::new(8, 32).unwrap();
    let mut handle = runtime.spawn(Box::new(transport)).unwrap();
    let mut output = Vec::new();

    wait_for_output(&handle, &mut output, b"READY");
    handle.send(TransportCommand::Resize(size)).unwrap();
    wait_for_event(
        &handle,
        |event| matches!(event, TransportEvent::ResizeApplied(applied) if *applied == size),
    );
    handle
        .send(TransportCommand::Input(b"hello-native\r".to_vec()))
        .unwrap();
    let exit = wait_for_exit(&mut handle, &mut output);

    assert_eq!(exit.code, Some(7));
    assert!(!exit.signaled);
    assert!(contains_bytes(&output, b"INPUT=hello-native"), "{output:?}");
    assert!(contains_bytes(&output, b"SIZE=9 31"), "{output:?}");
    assert!(contains_bytes(&output, b"ENV=direct-pty"), "{output:?}");
    handle.recv_completion().unwrap();
}

#[cfg(unix)]
#[test]
fn native_pty_shutdown_terminates_the_owned_child() {
    let command = NativePtyCommand::new("/bin/sh")
        .args(["-c", "printf 'READY\\r\\n'; while :; do sleep 1; done"]);
    let runtime = TransportRuntime::new(8, 8).unwrap();
    let mut handle = runtime
        .spawn(Box::new(NativePtyTransport::new(
            TransportId::new("native-shutdown"),
            command,
            TerminalSize::new(20, 4),
        )))
        .unwrap();
    let mut output = Vec::new();
    wait_for_output(&handle, &mut output, b"READY");

    handle.send(TransportCommand::Shutdown).unwrap();
    let exit = wait_for_exit(&mut handle, &mut output);

    assert!(exit.signaled || exit.code.is_some());
    handle.recv_completion().unwrap();
}

#[cfg(unix)]
#[test]
fn shutdown_escalates_past_ignored_hup_and_term() {
    let command = NativePtyCommand::new("/bin/sh").args([
        "-c",
        "trap '' HUP TERM; printf 'READY\\r\\n'; while :; do sleep 1; done",
    ]);
    let runtime = TransportRuntime::new(4, 4).unwrap();
    let mut handle = runtime
        .spawn(Box::new(NativePtyTransport::new(
            TransportId::new("native-ignore-signals"),
            command,
            TerminalSize::new(20, 4),
        )))
        .unwrap();
    let mut output = Vec::new();
    wait_for_output(&handle, &mut output, b"READY");

    let started = Instant::now();
    handle.send(TransportCommand::Shutdown).unwrap();
    let exit = wait_for_exit(&mut handle, &mut output);

    assert!(exit.signaled, "expected SIGKILL exit, got {exit:?}");
    assert!(started.elapsed() < Duration::from_secs(3));
    handle.recv_completion().unwrap();
}

#[cfg(unix)]
#[test]
fn full_single_slot_output_queue_does_not_starve_shutdown() {
    let command = NativePtyCommand::new("/bin/sh").args([
        "-c",
        "trap '' HUP TERM; printf 'READY\\r\\n'; while :; do printf '0123456789abcdef'; done",
    ]);
    let runtime = TransportRuntime::new(4, 1).unwrap();
    let mut handle = runtime
        .spawn(Box::new(NativePtyTransport::new(
            TransportId::new("native-output-backpressure"),
            command,
            TerminalSize::new(20, 4),
        )))
        .unwrap();
    let mut output = Vec::new();
    wait_for_output(&handle, &mut output, b"READY");
    thread::sleep(Duration::from_millis(30));

    handle.send(TransportCommand::Shutdown).unwrap();
    let exit = wait_for_exit(&mut handle, &mut output);

    assert!(exit.signaled);
    handle.recv_completion().unwrap();
}

#[cfg(unix)]
#[test]
fn blocked_large_input_does_not_starve_shutdown() {
    let command = NativePtyCommand::new("/bin/sh").args([
        "-c",
        "trap '' HUP TERM; stty -echo; printf 'READY\\r\\n'; while :; do sleep 1; done",
    ]);
    let runtime = TransportRuntime::new(4, 2).unwrap();
    let mut handle = runtime
        .spawn(Box::new(NativePtyTransport::new(
            TransportId::new("native-input-backpressure"),
            command,
            TerminalSize::new(20, 4),
        )))
        .unwrap();
    let mut output = Vec::new();
    wait_for_output(&handle, &mut output, b"READY");

    handle
        .send(TransportCommand::Input(vec![b'x'; 2 * 1024 * 1024]))
        .unwrap();
    handle.send(TransportCommand::Shutdown).unwrap();
    let exit = wait_for_exit(&mut handle, &mut output);

    assert!(exit.signaled);
    handle.recv_completion().unwrap();
}

#[cfg(unix)]
#[test]
fn native_pty_preserves_binary_output_and_explicit_env_removal() {
    let command = NativePtyCommand::new("/bin/sh")
            .args([
                "-c",
                "printf '\\200\\377'; if [ \"${PAD_REMOVE_TEST+x}\" = x ]; then printf 'ENV_PRESENT'; else printf 'ENV_REMOVED'; fi",
            ])
            .env("PAD_REMOVE_TEST", "present")
            .env_remove("PAD_REMOVE_TEST");
    let runtime = TransportRuntime::new(2, 8).unwrap();
    let mut handle = runtime
        .spawn(Box::new(NativePtyTransport::new(
            TransportId::new("native-binary-env"),
            command,
            TerminalSize::new(20, 4),
        )))
        .unwrap();
    let mut output = Vec::new();

    let exit = wait_for_exit(&mut handle, &mut output);

    assert_eq!(exit.code, Some(0));
    assert!(contains_bytes(&output, &[0x80, 0xff]), "{output:?}");
    assert!(contains_bytes(&output, b"ENV_REMOVED"), "{output:?}");
    assert!(!contains_bytes(&output, b"ENV_PRESENT"), "{output:?}");
    handle.recv_completion().unwrap();
}

#[test]
fn spawn_failure_is_reported_through_completion() {
    let runtime = TransportRuntime::new(2, 2).unwrap();
    let mut handle = runtime
        .spawn(Box::new(NativePtyTransport::new(
            TransportId::new("native-spawn-failure"),
            NativePtyCommand::new("/pad/definitely/not/a/program"),
            TerminalSize::new(20, 4),
        )))
        .unwrap();

    let error = handle.recv_completion().unwrap_err();

    assert!(
        error.to_string().contains("failed to spawn child"),
        "{error}"
    );
}

#[cfg(unix)]
#[test]
fn dropping_handle_reaps_the_owned_process() {
    let command = NativePtyCommand::new("/bin/sh").args([
        "-c",
        "trap '' HUP TERM; printf 'PID=%s\\r\\n' $$; while :; do sleep 1; done",
    ]);
    let runtime = TransportRuntime::new(2, 4).unwrap();
    let handle = runtime
        .spawn(Box::new(NativePtyTransport::new(
            TransportId::new("native-drop-reap"),
            command,
            TerminalSize::new(20, 4),
        )))
        .unwrap();
    let mut output = Vec::new();
    wait_for_output(&handle, &mut output, b"\r\n");
    let pid = parse_reported_pid(&output);

    drop(handle);

    let deadline = Instant::now() + TEST_TIMEOUT;
    while process_exists(pid) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    if process_exists(pid) {
        let _ = signal_process(pid, libc::SIGKILL);
        panic!("native PTY process {pid} survived handle drop");
    }
}

#[test]
fn default_program_rejects_arguments_without_panicking() {
    let error = NativePtyCommand::default_program()
        .arg("unsupported")
        .build()
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "native PTY default program cannot receive explicit arguments"
    );
}

#[cfg(unix)]
#[test]
fn unix_pty_eio_is_treated_as_end_of_stream() {
    assert!(pty_read_is_eof(&io::Error::from_raw_os_error(libc::EIO)));
    assert!(!pty_read_is_eof(&io::Error::from(
        io::ErrorKind::WouldBlock
    )));
}

fn wait_for_output(handle: &super::super::TransportHandle, output: &mut Vec<u8>, needle: &[u8]) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while Instant::now() < deadline {
        match handle.try_recv() {
            Ok(TransportEvent::Output(bytes)) => {
                output.extend(bytes);
                if contains_bytes(output, needle) {
                    return;
                }
            }
            Ok(_) | Err(TryRecvError::Empty) => thread::yield_now(),
            Err(TryRecvError::Disconnected) => break,
        }
    }
    panic!("timed out waiting for PTY output {needle:?}; output={output:?}");
}

fn wait_for_event(
    handle: &super::super::TransportHandle,
    predicate: impl Fn(&TransportEvent) -> bool,
) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while Instant::now() < deadline {
        match handle.try_recv() {
            Ok(event) if predicate(&event) => return,
            Ok(_) | Err(TryRecvError::Empty) => thread::yield_now(),
            Err(TryRecvError::Disconnected) => break,
        }
    }
    panic!("timed out waiting for PTY event");
}

fn wait_for_exit(
    handle: &mut super::super::TransportHandle,
    output: &mut Vec<u8>,
) -> TransportExit {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while Instant::now() < deadline {
        match handle.try_recv() {
            Ok(TransportEvent::Output(bytes)) => output.extend(bytes),
            Ok(TransportEvent::Exited(exit)) => return exit,
            Ok(TransportEvent::ResizeApplied(_)) | Err(TryRecvError::Empty) => thread::yield_now(),
            Err(TryRecvError::Disconnected) => {
                let completion = handle.recv_completion();
                panic!(
                        "PTY event stream disconnected before exit; completion={completion:?}; output={output:?}"
                    );
            }
        }
    }
    panic!("timed out waiting for PTY exit; output={output:?}");
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[cfg(unix)]
fn parse_reported_pid(output: &[u8]) -> libc::pid_t {
    let marker = b"PID=";
    let start = output
        .windows(marker.len())
        .position(|window| window == marker)
        .map(|index| index + marker.len())
        .expect("PID marker was present");
    let end = output[start..]
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .map(|offset| start + offset)
        .expect("PID was terminated by CRLF");
    std::str::from_utf8(&output[start..end])
        .unwrap()
        .parse()
        .unwrap()
}

#[cfg(unix)]
fn process_exists(pid: libc::pid_t) -> bool {
    if unsafe { libc::kill(pid, 0) } == 0 {
        return true;
    }
    io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
}
