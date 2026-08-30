//! Small, synchronous supervisor for a Pi `--mode rpc` child process.
//!
//! This module deliberately does not own an async runtime.  Its public methods
//! are guarded by a mutex so a cloned handle can be used by the UI thread and
//! a worker thread without sharing a `Child` unsafely.  Pi's stdout is the
//! JSONL transport; stderr is drained by a separate thread and is never fed to
//! the JSONL codec.

use super::{encode_command, JsonlCodec, JsonlError, PiEvent, PiMessage};
use serde_json::Value;
use std::fmt;
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const STDERR_BUFFER_LIMIT: usize = 512 * 1024;
const SHUTDOWN_GRACE: Duration = Duration::from_millis(150);
const POLL_CHUNK_BYTES: usize = 32 * 1024;

mod validation;
use validation::{
    has_rpc_mode, is_env_program, is_environment_assignment, is_pi_program, shell_words,
    validate_pi_session_args, validate_runtime_root,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PiExitStatus {
    pub(crate) code: Option<i32>,
    pub(crate) signal: Option<i32>,
    pub(crate) killed: bool,
}

impl PiExitStatus {
    pub(crate) fn success(self) -> bool {
        !self.killed && self.signal.is_none() && self.code == Some(0)
    }
}

impl fmt::Display for PiExitStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.killed {
            return formatter.write_str("Pi RPC process was killed");
        }
        if let Some(signal) = self.signal {
            return write!(formatter, "Pi RPC process exited on signal {signal}");
        }
        write!(
            formatter,
            "Pi RPC process exited with status {}",
            self.code
                .map_or_else(|| "unknown".to_string(), |code| code.to_string())
        )
    }
}

#[derive(Debug)]
pub(crate) enum PiSupervisorError {
    Io(io::Error),
    Jsonl(JsonlError),
    InvalidCommand(String),
    ProcessExited(PiExitStatus),
    Poisoned,
}

impl fmt::Display for PiSupervisorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "Pi RPC I/O error: {error}"),
            Self::Jsonl(error) => write!(formatter, "Pi RPC JSONL error: {error}"),
            Self::InvalidCommand(error) => write!(formatter, "invalid Pi RPC command: {error}"),
            Self::ProcessExited(status) => status.fmt(formatter),
            Self::Poisoned => formatter.write_str("Pi RPC supervisor lock is poisoned"),
        }
    }
}

impl std::error::Error for PiSupervisorError {}

impl From<io::Error> for PiSupervisorError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<JsonlError> for PiSupervisorError {
    fn from(error: JsonlError) -> Self {
        Self::Jsonl(error)
    }
}

/// Output collected by one non-blocking `poll` call.
#[derive(Debug, Default)]
pub(crate) struct PiPoll {
    /// All syntactically valid Pi messages, including messages that are not
    /// runtime events.
    pub(crate) messages: Vec<PiMessage>,
    /// Event-shaped messages accepted for this supervisor generation.
    pub(crate) events: Vec<PiEvent>,
    /// stderr bytes, kept separate from stdout JSONL.
    pub(crate) stderr: Vec<u8>,
    /// Malformed frames and other recoverable stream diagnostics.  A bad
    /// frame does not poison later valid frames.
    pub(crate) diagnostics: Vec<String>,
    /// Number of messages carrying a different generation token.
    pub(crate) dropped_stale: usize,
    /// Set once the child has exited and its exit status is known.
    pub(crate) exit_status: Option<PiExitStatus>,
    /// Runtime state changed without a wire frame (for example, an expired
    /// extension UI request).  The sole bridge pump must still fan out an
    /// authoritative snapshot in this case.
    pub(crate) state_changed: bool,
}

impl PiPoll {
    #[allow(
        dead_code,
        reason = "compatibility helper retained for callers that consume one supervisor poll atomically"
    )]
    pub(crate) fn is_empty(&self) -> bool {
        self.messages.is_empty()
            && self.events.is_empty()
            && self.stderr.is_empty()
            && self.diagnostics.is_empty()
            && self.exit_status.is_none()
            && !self.state_changed
    }
}

#[derive(Clone)]
pub(crate) struct PiSupervisor {
    inner: Arc<Mutex<Inner>>,
}

/// Names used by the desktop host while the public integration surface is
/// being assembled.  Keep the implementation type intentionally small.
pub(crate) type PiRpcSupervisor = PiSupervisor;
#[allow(
    dead_code,
    reason = "compatibility name retained while Desktop callers migrate to PiPoll"
)]
pub(crate) type PiSupervisorMessage = PiPoll;

struct Inner {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: ChildStdout,
    codec: JsonlCodec,
    generation: u64,
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "the generation root is exposed to isolation tests through PiSupervisor::root"
        )
    )]
    root: PathBuf,
    stdout_closed: bool,
    exit_status: Option<PiExitStatus>,
    stderr: Arc<Mutex<Vec<u8>>>,
    stderr_thread: Option<JoinHandle<()>>,
}

impl PiSupervisor {
    /// Spawn a command without invoking a shell.  The small shell-word parser
    /// accepts the command produced by `build_pi_rpc_command`, strips its
    /// stale PI env assignments, and applies a generation-specific private
    /// directory after parsing.  This prevents an older command string from
    /// defeating isolation.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "legacy native launch remains available for compatibility and isolation tests"
        )
    )]
    pub(crate) fn spawn(
        command: &str,
        cwd: impl AsRef<Path>,
        generation: u64,
    ) -> Result<Self, PiSupervisorError> {
        let agent_dir = crate::paths::pad_home_dir().join("pi-agent");
        let session_dir = agent_dir.join("sessions");
        Self::spawn_with_roots(command, cwd, generation, agent_dir, session_dir, true)
    }

    /// Spawn Pi for a Desktop Profile.  The profile's roots are kept separate
    /// from the legacy native-terminal root and from every other profile.
    /// Unlike the native compatibility path, these roots remain stable across
    /// process generations so an existing Pi session can be reopened.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "free-form profile launch is retained for compatibility tests; Desktop uses the fixed launcher"
        )
    )]
    pub(crate) fn spawn_for_profile(
        command: &str,
        cwd: impl AsRef<Path>,
        generation: u64,
        profile: &crate::permission_policy::Profile,
    ) -> Result<Self, PiSupervisorError> {
        let (agent_dir, session_dir) = super::profile_pi_roots(profile);
        Self::spawn_with_roots(command, cwd, generation, agent_dir, session_dir, false)
    }

    /// Spawn the fixed Pi executable selected by the Rust Desktop host.
    /// Unlike `spawn_for_profile`, this API accepts no free-form command or
    /// environment assignments, so a renderer request cannot launch another
    /// binary or smuggle additional process arguments.
    pub(crate) fn spawn_desktop_for_profile(
        program: &Path,
        cwd: impl AsRef<Path>,
        generation: u64,
        profile: &crate::permission_policy::Profile,
        session_file: Option<&Path>,
        provider: Option<&str>,
        model: Option<&str>,
        thinking_level: Option<&str>,
    ) -> Result<Self, PiSupervisorError> {
        let (agent_dir, session_dir) = super::profile_pi_roots(profile);
        validate_runtime_root(&agent_dir, "agent")?;
        validate_runtime_root(&session_dir, "session")?;
        crate::paths::base::ensure_private_dir(&agent_dir)?;
        let extension_path = agent_dir.join("pad-fast-mode.ts");
        crate::atomic_file::write_private(&extension_path, super::PAD_FAST_MODE_EXTENSION)?;
        let mut command = format!(
            "{} --mode rpc",
            crate::shell_quote::single_quote(&program.to_string_lossy())
        );
        command.push_str(" --extension ");
        command.push_str(&crate::shell_quote::single_quote(
            &extension_path.to_string_lossy(),
        ));
        if let Some(session_file) = session_file {
            command.push_str(" --session ");
            command.push_str(&crate::shell_quote::single_quote(
                &session_file.to_string_lossy(),
            ));
        }
        if let Some(provider) = provider {
            command.push_str(" --provider ");
            command.push_str(&crate::shell_quote::single_quote(provider));
        }
        if let Some(model) = model {
            command.push_str(" --model ");
            command.push_str(&crate::shell_quote::single_quote(model));
        }
        if let Some(thinking_level) = thinking_level {
            command.push_str(" --thinking ");
            command.push_str(&crate::shell_quote::single_quote(thinking_level));
        }
        Self::spawn_with_roots(&command, cwd, generation, agent_dir, session_dir, false)
    }

    fn spawn_with_roots(
        command: &str,
        cwd: impl AsRef<Path>,
        generation: u64,
        agent_dir: PathBuf,
        session_dir: PathBuf,
        generation_scope_roots: bool,
    ) -> Result<Self, PiSupervisorError> {
        validate_runtime_root(&agent_dir, "agent")?;
        validate_runtime_root(&session_dir, "session")?;
        let mut words = shell_words(command)?;
        if words.is_empty() {
            return Err(PiSupervisorError::InvalidCommand(
                "command is empty".to_string(),
            ));
        }

        let mut environment = Vec::new();
        if words.first().is_some_and(|word| is_env_program(word)) {
            words.remove(0);
            while words
                .first()
                .is_some_and(|word| is_environment_assignment(word))
            {
                let assignment = words.remove(0);
                let Some((key, value)) = assignment.split_once('=') else {
                    unreachable!("checked by is_environment_assignment");
                };
                if key != "PI_CODING_AGENT_DIR" && key != "PI_CODING_AGENT_SESSION_DIR" {
                    environment.push((key.to_string(), value.to_string()));
                }
            }
        }
        let program = words
            .first()
            .cloned()
            .ok_or_else(|| PiSupervisorError::InvalidCommand("program is missing".to_string()))?;
        let mut args = words.into_iter().skip(1).collect::<Vec<_>>();
        if is_pi_program(&program) && !has_rpc_mode(&args) {
            args.push("--mode".to_string());
            args.push("rpc".to_string());
        }

        let root = if generation_scope_roots {
            agent_dir.join("rpc").join(generation.to_string())
        } else {
            agent_dir
        };
        let session_dir = if generation_scope_roots {
            session_dir.join("rpc").join(generation.to_string())
        } else {
            session_dir
        };
        crate::paths::base::ensure_private_dir(&root)?;
        crate::paths::base::ensure_private_dir(&session_dir)?;
        crate::paths::base::harden_private_tree(&root)?;
        crate::paths::base::harden_private_tree(&session_dir)?;
        if is_pi_program(&program) {
            validate_pi_session_args(&args, &session_dir, cwd.as_ref())?;
        }

        let mut process = Command::new(&program);
        process
            .args(&args)
            .current_dir(cwd.as_ref())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            // SAFETY: `umask` is async-signal-safe and touches only the child
            // process state between fork and exec.  It ensures Pi creates new
            // auth/session files as 0600 and directories as 0700.
            unsafe {
                process.pre_exec(|| {
                    libc::umask(0o077);
                    Ok(())
                });
            }
        }
        for (key, value) in environment {
            process.env(key, value);
        }
        // These are intentionally applied last.  In particular, a command
        // made by an older build_pi_rpc_command cannot redirect this child to
        // ~/.codex or to a standalone ~/.pi directory.
        process
            .env("PI_CODING_AGENT_DIR", &root)
            .env("PI_CODING_AGENT_SESSION_DIR", &session_dir)
            .env(
                "PAD_PI_FAST_MODE_FILE",
                root.join(super::PAD_FAST_MODE_FILE),
            )
            // Pi's bundled Bun runtime may transpile provider modules on the
            // first request. Keep those caches Profile-private; writing below
            // Contents/Resources would invalidate the app signature.
            .env("BUN_INSTALL_CACHE_DIR", root.join("bun-cache"))
            .env(
                "BUN_RUNTIME_TRANSPILER_CACHE_PATH",
                root.join("bun-transpiler-cache"),
            );

        let mut child = process.spawn()?;
        let stdin = child.stdin.take();
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("Pi RPC stdout was not piped"))?;
        set_nonblocking(&stdout)?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("Pi RPC stderr was not piped"))?;
        let stderr_buffer = Arc::new(Mutex::new(Vec::new()));
        let stderr_sink = Arc::clone(&stderr_buffer);
        let stderr_thread = thread::Builder::new()
            .name("pad-pi-rpc-stderr".to_string())
            .spawn(move || drain_stderr(stderr, stderr_sink))?;

        Ok(Self {
            inner: Arc::new(Mutex::new(Inner {
                child,
                stdin,
                stdout,
                codec: JsonlCodec::default(),
                generation,
                root,
                stdout_closed: false,
                exit_status: None,
                stderr: stderr_buffer,
                stderr_thread: Some(stderr_thread),
            })),
        })
    }

    #[allow(
        dead_code,
        reason = "generation inspection is part of the supervisor compatibility API"
    )]
    pub(crate) fn generation(&self) -> Result<u64, PiSupervisorError> {
        Ok(self.lock()?.generation)
    }

    /// Return whether the child has exited without blocking.  Desktop uses
    /// this to allow a failed Pi task to be started again from the same Task.
    pub(crate) fn has_exited(&self) -> Result<bool, PiSupervisorError> {
        let mut inner = self.lock()?;
        Ok(inner.refresh_exit()?.is_some())
    }

    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "the private root accessor is retained for isolation tests"
        )
    )]
    pub(crate) fn root(&self) -> Result<PathBuf, PiSupervisorError> {
        Ok(self.lock()?.root.clone())
    }

    pub(crate) fn send(&self, command: Value) -> Result<(), PiSupervisorError> {
        let line = encode_command(&command)?;
        let mut inner = self.lock()?;
        if let Some(status) = inner.refresh_exit()? {
            return Err(PiSupervisorError::ProcessExited(status));
        }
        let Some(stdin) = inner.stdin.as_mut() else {
            let status = inner.exit_status.unwrap_or(PiExitStatus {
                code: None,
                signal: None,
                killed: false,
            });
            return Err(PiSupervisorError::ProcessExited(status));
        };
        stdin.write_all(&line)?;
        stdin.flush()?;
        Ok(())
    }

    /// Read whatever stdout/stderr is available right now.  No operation in
    /// this method waits for a future frame or process exit.
    pub(crate) fn poll(&self) -> Result<PiPoll, PiSupervisorError> {
        let mut inner = self.lock()?;
        let mut poll = PiPoll::default();
        inner.read_stdout(&mut poll)?;
        if let Some(status) = inner.refresh_exit()? {
            poll.exit_status = Some(status);
        }
        poll.stderr = take_stderr(&inner.stderr)?;
        Ok(poll)
    }

    #[allow(
        dead_code,
        reason = "compatibility alias retained for hosts that still call read_available"
    )]
    pub(crate) fn read_available(&self) -> Result<PiPoll, PiSupervisorError> {
        self.poll()
    }

    /// Ask Pi to abort, close stdin, then enforce a short deadline.  The
    /// deadline is important for shutdown paths invoked while the UI is
    /// closing; a broken or unresponsive provider must not hang PAD.
    pub(crate) fn shutdown(&self) -> Result<PiExitStatus, PiSupervisorError> {
        let (status, stderr_thread) = {
            let mut inner = self.lock()?;
            if let Some(status) = inner.exit_status {
                (status, inner.stderr_thread.take())
            } else {
                let abort = encode_command(&serde_json::json!({ "type": "abort" }))?;
                if let Some(stdin) = inner.stdin.as_mut() {
                    let _ = stdin.write_all(&abort);
                    let _ = stdin.flush();
                }
                inner.stdin.take();
                let deadline = Instant::now() + SHUTDOWN_GRACE;
                loop {
                    if let Some(status) = inner.refresh_exit()? {
                        break (status, inner.stderr_thread.take());
                    }
                    if Instant::now() >= deadline {
                        let _ = inner.child.kill();
                        let status = inner.child.wait()?;
                        let status = status_from_exit(status, true);
                        inner.exit_status = Some(status);
                        break (status, inner.stderr_thread.take());
                    }
                    thread::sleep(Duration::from_millis(5));
                }
            }
        };
        join_stderr(stderr_thread);
        Ok(status)
    }

    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "hard termination is exercised by supervisor lifecycle tests; production prefers shutdown"
        )
    )]
    pub(crate) fn kill(&self) -> Result<PiExitStatus, PiSupervisorError> {
        let (status, stderr_thread) = {
            let mut inner = self.lock()?;
            if let Some(status) = inner.exit_status {
                (status, inner.stderr_thread.take())
            } else {
                inner.stdin.take();
                inner.child.kill()?;
                let status = inner.child.wait()?;
                let status = status_from_exit(status, true);
                inner.exit_status = Some(status);
                (status, inner.stderr_thread.take())
            }
        };
        join_stderr(stderr_thread);
        Ok(status)
    }

    fn lock(&self) -> Result<MutexGuard<'_, Inner>, PiSupervisorError> {
        self.inner.lock().map_err(|_| PiSupervisorError::Poisoned)
    }
}

impl Inner {
    fn refresh_exit(&mut self) -> Result<Option<PiExitStatus>, PiSupervisorError> {
        if let Some(status) = self.exit_status {
            return Ok(Some(status));
        }
        let Some(status) = self.child.try_wait()? else {
            return Ok(None);
        };
        let status = status_from_exit(status, false);
        self.exit_status = Some(status);
        Ok(Some(status))
    }

    fn read_stdout(&mut self, poll: &mut PiPoll) -> Result<(), PiSupervisorError> {
        if self.stdout_closed {
            poll.stderr = take_stderr(&self.stderr)?;
            return Ok(());
        }
        let mut buffer = [0_u8; POLL_CHUNK_BYTES];
        loop {
            match self.stdout.read(&mut buffer) {
                Ok(0) => {
                    self.stdout_closed = true;
                    if let Err(error) = self.codec.finish() {
                        poll.diagnostics.push(error.to_string());
                    }
                    break;
                }
                Ok(bytes) => match self.codec.push(&buffer[..bytes]) {
                    Ok(values) => self.accept_values(values, poll),
                    Err(error) => {
                        poll.diagnostics.push(error.to_string());
                        // JsonlCodec removes a malformed complete frame but
                        // intentionally retains the following partial tail.
                        // Flush that tail now so one bad frame cannot hide a
                        // valid event received in the same pipe read.
                        if let Ok(values) = self.codec.push(&[]) {
                            self.accept_values(values, poll);
                        }
                    }
                },
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }

    fn accept_values(&mut self, values: Vec<Value>, poll: &mut PiPoll) {
        for value in values {
            let Some(value_generation) = value.get("generation").and_then(Value::as_u64) else {
                let message = match PiMessage::parse(value) {
                    Ok(message) => message,
                    Err(error) => {
                        poll.diagnostics.push(error.to_string());
                        continue;
                    }
                };
                if let Some(event) = PiEvent::parse(message.value.clone()) {
                    poll.events.push(event);
                }
                poll.messages.push(message);
                continue;
            };
            if value_generation != self.generation {
                poll.dropped_stale += 1;
                continue;
            }
            let message = match PiMessage::parse(value) {
                Ok(message) => message,
                Err(error) => {
                    poll.diagnostics.push(error.to_string());
                    continue;
                }
            };
            if let Some(event) = PiEvent::parse(message.value.clone()) {
                poll.events.push(event);
            }
            poll.messages.push(message);
        }
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        if self.exit_status.is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        self.stdin.take();
        join_stderr(self.stderr_thread.take());
    }
}

fn take_stderr(stderr: &Arc<Mutex<Vec<u8>>>) -> Result<Vec<u8>, PiSupervisorError> {
    let mut stderr = stderr.lock().map_err(|_| PiSupervisorError::Poisoned)?;
    Ok(std::mem::take(&mut *stderr))
}

fn drain_stderr(mut reader: impl Read, sink: Arc<Mutex<Vec<u8>>>) {
    let mut chunk = [0_u8; 16 * 1024];
    loop {
        let bytes = match reader.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(bytes) => bytes,
        };
        if let Ok(mut sink) = sink.lock() {
            let remaining = STDERR_BUFFER_LIMIT.saturating_sub(sink.len());
            sink.extend_from_slice(&chunk[..bytes.min(remaining)]);
        } else {
            break;
        }
    }
}

fn join_stderr(thread: Option<JoinHandle<()>>) {
    if let Some(thread) = thread {
        let _ = thread.join();
    }
}

fn status_from_exit(status: ExitStatus, killed: bool) -> PiExitStatus {
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;
    PiExitStatus {
        code: status.code(),
        #[cfg(unix)]
        signal: status.signal(),
        #[cfg(not(unix))]
        signal: None,
        killed,
    }
}

fn set_nonblocking(stdout: &ChildStdout) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        let fd = stdout.as_raw_fd();
        // SAFETY: fcntl operates on the live pipe descriptor owned by stdout.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: same descriptor; only the O_NONBLOCK bit is added.
        if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests;
