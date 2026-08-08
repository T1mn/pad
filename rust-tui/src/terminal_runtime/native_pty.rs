mod process;
mod reader_io;

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
#[cfg(all(test, unix))]
use process::signal_process;
use process::ChildGuard;
use reader_io::*;

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

#[cfg(test)]
#[path = "native_pty_tests.rs"]
mod tests;
