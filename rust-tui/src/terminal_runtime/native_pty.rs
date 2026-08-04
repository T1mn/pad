use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, Child, CommandBuilder, ExitStatus, MasterPty, PtySize};

use super::{
    SessionTransport, TerminalError, TerminalSize, TransportCommand, TransportEvent, TransportExit,
    TransportId,
};

const READ_BUFFER_SIZE: usize = 16 * 1024;
const READ_QUEUE_CAPACITY: usize = 32;
const READER_DRAIN_BUDGET: usize = 8;
const OUTPUT_FORWARD_BUDGET: usize = 8;
const PENDING_OUTPUT_CAPACITY: usize = 16;
const COMMAND_DRAIN_BUDGET: usize = 32;
const WRITE_BUDGET: usize = 64 * 1024;
const MAX_PENDING_INPUT_BYTES: usize = 8 * 1024 * 1024;
const CONTROL_POLL_INTERVAL: Duration = Duration::from_millis(2);
const FINAL_DRAIN_GRACE: Duration = Duration::from_secs(1);
const READER_JOIN_GRACE: Duration = Duration::from_millis(250);
#[cfg(unix)]
const SIGNAL_GRACE: Duration = Duration::from_millis(150);
#[cfg(unix)]
const KILL_GRACE: Duration = Duration::from_secs(1);

/// A command launched directly inside a native pseudo-terminal.
///
/// Arguments are passed as an argv vector; no shell parsing or interpolation
/// occurs unless the caller explicitly launches a shell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativePtyCommand {
    program: Option<OsString>,
    args: Vec<OsString>,
    cwd: Option<PathBuf>,
    env: Vec<(OsString, OsString)>,
    env_remove: Vec<OsString>,
    clear_env: bool,
}

impl NativePtyCommand {
    pub fn new(program: impl AsRef<OsStr>) -> Self {
        Self {
            program: Some(program.as_ref().to_owned()),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            env_remove: Vec::new(),
            clear_env: false,
        }
    }

    /// Uses the platform's default interactive program as selected by
    /// `portable-pty`.
    pub fn default_program() -> Self {
        Self {
            program: None,
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            env_remove: Vec::new(),
            clear_env: false,
        }
    }

    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args
            .extend(args.into_iter().map(|arg| arg.as_ref().to_owned()));
        self
    }

    pub fn cwd(mut self, cwd: impl AsRef<Path>) -> Self {
        self.cwd = Some(cwd.as_ref().to_owned());
        self
    }

    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.env
            .push((key.as_ref().to_owned(), value.as_ref().to_owned()));
        self
    }

    /// Removes an inherited environment variable from the child process.
    ///
    /// Explicit removals win over values previously supplied with [`Self::env`].
    pub fn env_remove(mut self, key: impl AsRef<OsStr>) -> Self {
        self.env_remove.push(key.as_ref().to_owned());
        self
    }

    pub fn clear_env(mut self) -> Self {
        self.clear_env = true;
        self
    }

    fn build(self) -> Result<CommandBuilder, TerminalError> {
        let mut command = match self.program {
            Some(program) => CommandBuilder::new(program),
            None if self.args.is_empty() => CommandBuilder::new_default_prog(),
            None => {
                return Err(TerminalError::new(
                    "native PTY default program cannot receive explicit arguments",
                ));
            }
        };
        if self.clear_env {
            command.env_clear();
        }
        // These describe the emulator implemented by TerminalEngine. User
        // values are applied afterwards and may intentionally override them.
        command.env_remove("TMUX");
        command.env_remove("TMUX_PANE");
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        command.env("TERM_PROGRAM", "pad");
        command.env("PAD_TERMINAL", "native");
        command.args(self.args);
        if let Some(cwd) = self.cwd {
            command.cwd(cwd);
        }
        for (key, value) in self.env {
            command.env(key, value);
        }
        for key in self.env_remove {
            command.env_remove(key);
        }
        Ok(command)
    }
}

/// A production session transport backed by the operating system's PTY.
///
/// This transport launches and owns the child process directly. It does not
/// execute, connect to, or require tmux.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativePtyTransport {
    id: TransportId,
    command: NativePtyCommand,
    initial_size: TerminalSize,
}

impl NativePtyTransport {
    pub fn new(id: TransportId, command: NativePtyCommand, initial_size: TerminalSize) -> Self {
        Self {
            id,
            command,
            initial_size,
        }
    }

    pub fn command(&self) -> &NativePtyCommand {
        &self.command
    }

    pub fn initial_size(&self) -> TerminalSize {
        self.initial_size
    }

    fn error(&self, operation: &str, error: impl std::fmt::Display) -> TerminalError {
        TerminalError::new(format!(
            "native PTY transport '{}' {operation}: {error}",
            self.id.as_str()
        ))
    }
}

impl SessionTransport for NativePtyTransport {
    fn id(&self) -> &TransportId {
        &self.id
    }

    fn run(
        self: Box<Self>,
        commands: Receiver<TransportCommand>,
        events: SyncSender<TransportEvent>,
    ) -> Result<(), TerminalError> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(pty_size(self.initial_size))
            .map_err(|error| self.error("failed to open", error))?;
        let command = self.command.clone().build()?;
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| self.error("failed to spawn child", error))?;
        // From this point onward every early return must reap the process. In
        // particular, cloning the reader and taking the writer are fallible.
        let mut child = ChildGuard::new(child);
        drop(pair.slave);

        let master = pair.master;
        set_master_nonblocking(master.as_ref())
            .map_err(|error| self.error("failed to configure nonblocking I/O", error))?;
        let reader = master
            .try_clone_reader()
            .map_err(|error| self.error("failed to clone reader", error))?;
        let mut writer = Some(
            master
                .take_writer()
                .map_err(|error| self.error("failed to take writer", error))?,
        );
        let (read_sender, read_events) = mpsc::sync_channel(READ_QUEUE_CAPACITY);
        let mut reader_worker = spawn_reader(&self.id, reader, read_sender)
            .map_err(|error| self.error("failed to start output reader", error))?;

        let mut exit_status = None;
        let mut reader_closed: Option<Result<(), String>> = None;
        let mut pending_output = VecDeque::new();
        let mut pending_resize = None;
        let mut pending_input = VecDeque::new();
        let mut pending_input_bytes = 0usize;
        let mut shutdown_requested = false;
        let mut final_deadline = None;
        let mut reader_join_deadline = None;
        let mut exit_publish_deadline = None;
        let mut final_drain_expired = false;

        loop {
            if final_drain_expired {
                pending_output.clear();
                pending_resize = None;
            } else {
                flush_transport_events(&events, &mut pending_resize, &mut pending_output)
                    .map_err(|error| self.error("output forwarding failed", error))?;
                drain_reader_events(&read_events, &mut pending_output, &mut reader_closed);
            }
            if reader_closed.is_some() && exit_status.is_none() {
                // A child that has lost its output side is no longer a usable
                // terminal session. Terminate it instead of silently parking
                // the transport forever.
                shutdown_requested = true;
            }

            drain_commands(
                &commands,
                &mut pending_input,
                &mut pending_input_bytes,
                &mut pending_resize,
                master.as_ref(),
                &mut shutdown_requested,
            )
            .map_err(|error| self.error("command processing failed", error))?;

            if !shutdown_requested {
                if let Some(pty_writer) = writer.as_mut() {
                    write_pending_input(
                        pty_writer.as_mut(),
                        &mut pending_input,
                        &mut pending_input_bytes,
                    )
                    .map_err(|error| self.error("input write failed", error))?;
                }
            }

            if exit_status.is_none() {
                exit_status = child
                    .try_wait()
                    .map_err(|error| self.error("failed to poll child", error))?;
            }

            if shutdown_requested && exit_status.is_none() {
                writer.take();
                pending_input.clear();
                pending_input_bytes = 0;
                exit_status = Some(
                    child
                        .terminate(master.as_ref(), || {
                            drain_reader_events_dropping_overflow(
                                &read_events,
                                &mut pending_output,
                                &mut reader_closed,
                            );
                        })
                        .map_err(|error| self.error("failed to terminate child", error))?,
                );
            }

            if exit_status.is_some() && final_deadline.is_none() {
                writer.take();
                final_deadline = Some(Instant::now() + FINAL_DRAIN_GRACE);
            }

            if let Some(deadline) = final_deadline {
                if reader_closed.is_some() && reader_worker.is_finished() {
                    reader_worker
                        .join_finished()
                        .map_err(|error| self.error("output reader panicked", error))?;
                } else if Instant::now() >= deadline {
                    final_drain_expired = true;
                    reader_worker.cancel();
                    reader_join_deadline.get_or_insert_with(|| Instant::now() + READER_JOIN_GRACE);
                    exit_publish_deadline.get_or_insert_with(|| Instant::now() + READER_JOIN_GRACE);
                }

                if reader_join_deadline.is_some() {
                    if reader_worker.is_finished() {
                        reader_worker
                            .join_finished()
                            .map_err(|error| self.error("output reader panicked", error))?;
                    } else if reader_join_deadline
                        .is_some_and(|join_deadline| Instant::now() >= join_deadline)
                    {
                        return Err(self.error(
                            "output reader cancellation timed out",
                            "reader did not stop before its deadline",
                        ));
                    }
                }

                if reader_worker.is_joined() {
                    drain_reader_events(&read_events, &mut pending_output, &mut reader_closed);
                    if final_drain_expired {
                        pending_output.clear();
                    }
                    flush_transport_events(&events, &mut pending_resize, &mut pending_output)
                        .map_err(|error| self.error("output forwarding failed", error))?;

                    if pending_output.is_empty() && pending_resize.is_none() {
                        let status = exit_status
                            .as_ref()
                            .expect("reader finalization requires child exit");
                        match events.try_send(TransportEvent::Exited(transport_exit(status))) {
                            Ok(()) => {
                                if let Some(Err(error)) = reader_closed.as_ref() {
                                    return Err(self.error("output reader failed", error));
                                }
                                if final_drain_expired {
                                    return Err(self.error(
                                        "final output drain timed out",
                                        "PTY output did not close before its deadline",
                                    ));
                                }
                                return Ok(());
                            }
                            Err(TrySendError::Full(_))
                                if Instant::now() < exit_publish_deadline.unwrap_or(deadline) => {}
                            Err(TrySendError::Full(_)) => {
                                return Err(self.error(
                                    "failed to publish exit",
                                    "event queue remained full until the final deadline",
                                ));
                            }
                            Err(TrySendError::Disconnected(_)) => {
                                return Err(
                                    self.error("failed to publish exit", "receiver disconnected")
                                );
                            }
                        }
                    }
                }
            }

            thread::sleep(CONTROL_POLL_INTERVAL);
        }
    }
}

fn pty_size(size: TerminalSize) -> PtySize {
    PtySize {
        rows: size.rows,
        cols: size.columns,
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn transport_exit(status: &ExitStatus) -> TransportExit {
    let signaled = status.signal().is_some();
    TransportExit {
        code: if signaled {
            None
        } else {
            i32::try_from(status.exit_code()).ok()
        },
        signaled,
    }
}

enum ReaderEvent {
    Output(Vec<u8>),
    Closed(Result<(), String>),
}

struct PendingInput {
    bytes: Vec<u8>,
    offset: usize,
}

struct ReaderWorker {
    handle: Option<JoinHandle<()>>,
    cancelled: Arc<AtomicBool>,
}

impl ReaderWorker {
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    fn is_finished(&self) -> bool {
        self.handle.as_ref().is_none_or(JoinHandle::is_finished)
    }

    fn is_joined(&self) -> bool {
        self.handle.is_none()
    }

    fn join_finished(&mut self) -> Result<(), &'static str> {
        let Some(handle) = self.handle.take() else {
            return Ok(());
        };
        debug_assert!(handle.is_finished());
        handle.join().map_err(|_| "reader thread panicked")
    }
}

impl Drop for ReaderWorker {
    fn drop(&mut self) {
        self.cancel();
        let deadline = Instant::now() + READER_JOIN_GRACE;
        while self
            .handle
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
            && Instant::now() < deadline
        {
            thread::sleep(CONTROL_POLL_INTERVAL);
        }
        if self.is_finished() {
            let _ = self.join_finished();
        }
    }
}

fn spawn_reader(
    id: &TransportId,
    mut reader: Box<dyn Read + Send>,
    events: SyncSender<ReaderEvent>,
) -> io::Result<ReaderWorker> {
    let cancelled = Arc::new(AtomicBool::new(false));
    let reader_cancelled = Arc::clone(&cancelled);
    let handle = thread::Builder::new()
        .name(reader_thread_name(id))
        .spawn(move || {
            let mut buffer = vec![0; READ_BUFFER_SIZE];
            while !reader_cancelled.load(Ordering::Acquire) {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        send_reader_event(&events, ReaderEvent::Closed(Ok(())), &reader_cancelled);
                        return;
                    }
                    Ok(read) => {
                        if !send_reader_event(
                            &events,
                            ReaderEvent::Output(buffer[..read].to_vec()),
                            &reader_cancelled,
                        ) {
                            return;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(CONTROL_POLL_INTERVAL);
                    }
                    Err(error) if pty_read_is_eof(&error) => {
                        send_reader_event(&events, ReaderEvent::Closed(Ok(())), &reader_cancelled);
                        return;
                    }
                    Err(error) => {
                        send_reader_event(
                            &events,
                            ReaderEvent::Closed(Err(error.to_string())),
                            &reader_cancelled,
                        );
                        return;
                    }
                }
            }
        })?;
    Ok(ReaderWorker {
        handle: Some(handle),
        cancelled,
    })
}

fn pty_read_is_eof(error: &io::Error) -> bool {
    #[cfg(unix)]
    {
        error.raw_os_error() == Some(libc::EIO)
    }

    #[cfg(not(unix))]
    {
        let _ = error;
        false
    }
}

fn send_reader_event(
    events: &SyncSender<ReaderEvent>,
    mut event: ReaderEvent,
    cancelled: &AtomicBool,
) -> bool {
    loop {
        if cancelled.load(Ordering::Acquire) {
            return false;
        }
        match events.try_send(event) {
            Ok(()) => return true,
            Err(TrySendError::Full(returned)) => {
                event = returned;
                thread::sleep(CONTROL_POLL_INTERVAL);
            }
            Err(TrySendError::Disconnected(_)) => return false,
        }
    }
}

fn reader_thread_name(id: &TransportId) -> String {
    let suffix: String = id
        .as_str()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .take(24)
        .collect();
    format!("pad-pty-reader-{suffix}")
}

fn drain_reader_events(
    read_events: &Receiver<ReaderEvent>,
    pending_output: &mut VecDeque<Vec<u8>>,
    reader_closed: &mut Option<Result<(), String>>,
) {
    for _ in 0..READER_DRAIN_BUDGET {
        if pending_output.len() >= PENDING_OUTPUT_CAPACITY {
            return;
        }
        match read_events.try_recv() {
            Ok(ReaderEvent::Output(bytes)) => pending_output.push_back(bytes),
            Ok(ReaderEvent::Closed(result)) => {
                *reader_closed = Some(result);
                return;
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => return,
        }
    }
}

/// Keeps the PTY readable while a child is being signalled and reaped.
///
/// A process can be blocked inside a terminal write when its output buffer is
/// full.  If the transport stops draining at that exact point, even a pending
/// fatal signal may not be observed promptly on some kernels.  Preserve the
/// bounded tail that fits in `pending_output`, but discard overflow so forced
/// shutdown can always make progress.
fn drain_reader_events_dropping_overflow(
    read_events: &Receiver<ReaderEvent>,
    pending_output: &mut VecDeque<Vec<u8>>,
    reader_closed: &mut Option<Result<(), String>>,
) {
    for _ in 0..READ_QUEUE_CAPACITY {
        match read_events.try_recv() {
            Ok(ReaderEvent::Output(bytes)) => {
                if pending_output.len() < PENDING_OUTPUT_CAPACITY {
                    pending_output.push_back(bytes);
                }
            }
            Ok(ReaderEvent::Closed(result)) => {
                *reader_closed = Some(result);
                return;
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => return,
        }
    }
}

fn flush_transport_events(
    transport_events: &SyncSender<TransportEvent>,
    pending_resize: &mut Option<TerminalSize>,
    pending_output: &mut VecDeque<Vec<u8>>,
) -> Result<(), &'static str> {
    if let Some(size) = pending_resize.take() {
        match transport_events.try_send(TransportEvent::ResizeApplied(size)) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                *pending_resize = Some(size);
                return Ok(());
            }
            Err(TrySendError::Disconnected(_)) => return Err("receiver disconnected"),
        }
    }

    for _ in 0..OUTPUT_FORWARD_BUDGET {
        let Some(bytes) = pending_output.pop_front() else {
            break;
        };
        match transport_events.try_send(TransportEvent::Output(bytes)) {
            Ok(()) => {}
            Err(TrySendError::Full(TransportEvent::Output(bytes))) => {
                pending_output.push_front(bytes);
                break;
            }
            Err(TrySendError::Disconnected(_)) => return Err("receiver disconnected"),
            Err(TrySendError::Full(_)) => unreachable!("output event changed variant"),
        }
    }
    Ok(())
}

fn drain_commands(
    commands: &Receiver<TransportCommand>,
    pending_input: &mut VecDeque<PendingInput>,
    pending_input_bytes: &mut usize,
    pending_resize: &mut Option<TerminalSize>,
    master: &dyn MasterPty,
    shutdown_requested: &mut bool,
) -> Result<(), String> {
    for _ in 0..COMMAND_DRAIN_BUDGET {
        match commands.try_recv() {
            Ok(TransportCommand::Input(bytes)) if !*shutdown_requested => {
                let new_size = pending_input_bytes
                    .checked_add(bytes.len())
                    .ok_or_else(|| "pending PTY input size overflowed".to_string())?;
                if new_size > MAX_PENDING_INPUT_BYTES {
                    *shutdown_requested = true;
                    pending_input.clear();
                    *pending_input_bytes = 0;
                    return Err(format!(
                        "pending PTY input exceeded {MAX_PENDING_INPUT_BYTES} bytes"
                    ));
                }
                *pending_input_bytes = new_size;
                pending_input.push_back(PendingInput { bytes, offset: 0 });
            }
            Ok(TransportCommand::Input(_)) => {}
            Ok(TransportCommand::Resize(size)) if !*shutdown_requested => {
                master
                    .resize(pty_size(size))
                    .map_err(|error| error.to_string())?;
                *pending_resize = Some(size);
            }
            Ok(TransportCommand::Resize(_)) => {}
            Ok(TransportCommand::Shutdown) => *shutdown_requested = true,
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                *shutdown_requested = true;
                break;
            }
        }
    }
    Ok(())
}

fn write_pending_input(
    writer: &mut dyn Write,
    pending_input: &mut VecDeque<PendingInput>,
    pending_input_bytes: &mut usize,
) -> io::Result<()> {
    let mut budget = WRITE_BUDGET;
    while budget > 0 {
        let Some(input) = pending_input.front_mut() else {
            break;
        };
        if input.offset == input.bytes.len() {
            pending_input.pop_front();
            continue;
        }
        let remaining = &input.bytes[input.offset..];
        let write_len = remaining.len().min(budget);
        match writer.write(&remaining[..write_len]) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(written) => {
                input.offset += written;
                *pending_input_bytes = pending_input_bytes.saturating_sub(written);
                budget -= written;
                if input.offset == input.bytes.len() {
                    pending_input.pop_front();
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[cfg(unix)]
fn set_master_nonblocking(master: &dyn MasterPty) -> io::Result<()> {
    let file_descriptor = master
        .as_raw_fd()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "PTY has no file descriptor"))?;
    let flags = unsafe { libc::fcntl(file_descriptor, libc::F_GETFL) };
    if flags == -1 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { libc::fcntl(file_descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(unix))]
fn set_master_nonblocking(_master: &dyn MasterPty) -> io::Result<()> {
    Ok(())
}

struct ChildGuard {
    child: Box<dyn Child + Send + Sync>,
    reaped: bool,
    #[cfg(unix)]
    child_pid: Option<libc::pid_t>,
}

impl ChildGuard {
    fn new(child: Box<dyn Child + Send + Sync>) -> Self {
        #[cfg(unix)]
        let child_pid = child
            .process_id()
            .and_then(|pid| libc::pid_t::try_from(pid).ok())
            .filter(|pid| is_safe_child_pid(*pid));
        Self {
            child,
            reaped: false,
            #[cfg(unix)]
            child_pid,
        }
    }

    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if self.reaped {
            return Ok(None);
        }
        let status = self.child.try_wait()?;
        if status.is_some() {
            self.reaped = true;
        }
        Ok(status)
    }

    fn terminate(
        &mut self,
        master: &dyn MasterPty,
        mut service_output: impl FnMut(),
    ) -> io::Result<ExitStatus> {
        #[cfg(unix)]
        {
            let mut last_signal_error = match self.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) => None,
                Err(error) => Some(error),
            };
            for signal in [libc::SIGHUP, libc::SIGTERM, libc::SIGKILL] {
                // Foreground process groups can change when a shell starts a
                // job, so resolve them for every escalation stage. Always
                // signal the owned child PID as well; that prevents a shell
                // which moved out of the foreground group from escaping.
                if let Some(process_group) = safe_process_group(master) {
                    if let Err(error) = signal_process_group(process_group, signal) {
                        last_signal_error = Some(error);
                    }
                }
                if let Some(child_pid) = self.child_pid {
                    if let Err(error) = signal_process(child_pid, signal) {
                        last_signal_error = Some(error);
                    }
                }
                if signal == libc::SIGKILL {
                    if let Err(error) = self.child.kill() {
                        last_signal_error = Some(error);
                    }
                }
                let grace = if signal == libc::SIGKILL {
                    KILL_GRACE
                } else {
                    SIGNAL_GRACE
                };
                match self.wait_until_with(Instant::now() + grace, &mut service_output) {
                    Ok(Some(status)) => return Ok(status),
                    Ok(None) => {}
                    Err(error) => last_signal_error = Some(error),
                }
            }
            let detail = last_signal_error.map_or_else(
                || "child survived SIGHUP, SIGTERM, and SIGKILL deadlines".to_string(),
                |error| format!("child was not reaped; last signal error: {error}"),
            );
            Err(io::Error::new(io::ErrorKind::TimedOut, detail))
        }

        #[cfg(not(unix))]
        {
            match self.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) | Err(_) => {}
            }
            let kill_error = self.child.kill().err();
            match self.wait_until(Instant::now() + Duration::from_millis(500)) {
                Ok(Some(status)) => Ok(status),
                Ok(None) | Err(_) => Err(kill_error.unwrap_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::TimedOut,
                        "child was not reaped before the kill deadline",
                    )
                })),
            }
        }
    }

    fn wait_until(&mut self, deadline: Instant) -> io::Result<Option<ExitStatus>> {
        self.wait_until_with(deadline, || {})
    }

    fn wait_until_with(
        &mut self,
        deadline: Instant,
        mut service: impl FnMut(),
    ) -> io::Result<Option<ExitStatus>> {
        loop {
            service();
            if let Some(status) = self.try_wait()? {
                return Ok(Some(status));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.reaped {
            return;
        }

        #[cfg(unix)]
        {
            // Setup can fail before the controller owns all PTY handles. The
            // captured PID is the only trustworthy target in that path.
            for signal in [libc::SIGHUP, libc::SIGTERM, libc::SIGKILL] {
                if let Some(child_pid) = self.child_pid {
                    let _ = signal_process(child_pid, signal);
                } else if signal == libc::SIGKILL {
                    let _ = self.child.kill();
                }
                if self
                    .wait_until(Instant::now() + SIGNAL_GRACE)
                    .ok()
                    .flatten()
                    .is_some()
                {
                    return;
                }
            }
        }

        #[cfg(not(unix))]
        {
            let _ = self.child.kill();
            let _ = self.wait_until(Instant::now() + Duration::from_millis(500));
        }
    }
}

#[cfg(unix)]
fn is_safe_child_pid(pid: libc::pid_t) -> bool {
    pid > 1 && pid != unsafe { libc::getpid() }
}

#[cfg(unix)]
fn safe_process_group(master: &dyn MasterPty) -> Option<libc::pid_t> {
    let process_group = master.process_group_leader()?;
    let own_process_group = unsafe { libc::getpgrp() };
    (process_group > 1 && process_group != own_process_group).then_some(process_group)
}

#[cfg(unix)]
fn signal_process_group(process_group: libc::pid_t, signal: libc::c_int) -> io::Result<()> {
    let result = unsafe { libc::kill(-process_group, signal) };
    if result == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(error)
    }
}

#[cfg(unix)]
fn signal_process(pid: libc::pid_t, signal: libc::c_int) -> io::Result<()> {
    let result = unsafe { libc::kill(pid, signal) };
    if result == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use super::*;
    use crate::terminal_runtime::TransportRuntime;

    const TEST_TIMEOUT: Duration = Duration::from_secs(5);

    #[cfg(unix)]
    #[test]
    fn native_pty_runs_without_tmux_and_preserves_io_resize_env_and_exit() {
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

    fn wait_for_output(
        handle: &super::super::TransportHandle,
        output: &mut Vec<u8>,
        needle: &[u8],
    ) {
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
                Ok(TransportEvent::ResizeApplied(_)) | Err(TryRecvError::Empty) => {
                    thread::yield_now()
                }
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
}
