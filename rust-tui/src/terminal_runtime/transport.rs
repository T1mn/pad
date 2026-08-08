use std::sync::mpsc::{Receiver, SyncSender};

use super::{TerminalError, TerminalSize};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TransportId(String);

impl TransportId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransportCommand {
    Input(Vec<u8>),
    Resize(TerminalSize),
    Shutdown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransportEvent {
    Output(Vec<u8>),
    /// Confirms that the transport has applied a requested terminal size at
    /// the kernel/session boundary. Events remain FIFO, but a real PTY reader
    /// cannot prove when concurrently buffered bytes were physically emitted.
    ResizeApplied(TerminalSize),
    Exited(TransportExit),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransportExit {
    pub code: Option<i32>,
    pub signaled: bool,
}

/// One deterministic interaction in a [`ReplayTransport`] script.
///
/// `Emit` steps travel from the transport to its caller. `Expect` steps block
/// until the caller sends the exact command recorded in the script. Keeping
/// both directions in one ordered sequence makes recordings independent of
/// thread scheduling and suitable for reproducing parser and lifecycle bugs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayStep {
    Emit(TransportEvent),
    Expect(TransportCommand),
}

impl ReplayStep {
    pub fn output(bytes: impl Into<Vec<u8>>) -> Self {
        Self::Emit(TransportEvent::Output(bytes.into()))
    }

    pub fn expect_input(bytes: impl Into<Vec<u8>>) -> Self {
        Self::Expect(TransportCommand::Input(bytes.into()))
    }

    pub fn expect_resize(size: TerminalSize) -> Self {
        Self::Expect(TransportCommand::Resize(size))
    }

    pub fn resize_applied(size: TerminalSize) -> Self {
        Self::Emit(TransportEvent::ResizeApplied(size))
    }

    pub fn expect_shutdown() -> Self {
        Self::Expect(TransportCommand::Shutdown)
    }

    pub fn exit(exit: TransportExit) -> Self {
        Self::Emit(TransportEvent::Exited(exit))
    }
}

/// A deterministic transport backed by an ordered recording.
///
/// Replay never sleeps and does not attach timestamps to a recording. It
/// advances only after an event has entered the caller's bounded event queue
/// or an expected command has arrived, preserving both chunk boundaries and
/// backpressure. The same value can be cloned and run repeatedly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayTransport {
    id: TransportId,
    steps: Vec<ReplayStep>,
}

impl ReplayTransport {
    pub fn new(id: TransportId, steps: impl IntoIterator<Item = ReplayStep>) -> Self {
        Self {
            id,
            steps: steps.into_iter().collect(),
        }
    }

    /// Builds a simple output-only recording followed by an exit event.
    pub fn from_output_chunks<I, B>(id: TransportId, chunks: I, exit: TransportExit) -> Self
    where
        I: IntoIterator<Item = B>,
        B: Into<Vec<u8>>,
    {
        let mut steps: Vec<_> = chunks.into_iter().map(ReplayStep::output).collect();
        steps.push(ReplayStep::exit(exit));
        Self { id, steps }
    }

    pub fn steps(&self) -> &[ReplayStep] {
        &self.steps
    }

    fn validate(&self) -> Result<(), TerminalError> {
        if let Some(index) = self
            .steps
            .iter()
            .position(|step| matches!(step, ReplayStep::Emit(TransportEvent::Exited(_))))
        {
            if index + 1 != self.steps.len() {
                return Err(self.error_at(index, "exit event must be the final replay step"));
            }
        }
        Ok(())
    }

    fn error_at(&self, index: usize, message: impl std::fmt::Display) -> TerminalError {
        TerminalError::new(format!(
            "replay transport '{}' step {}: {message}",
            self.id.as_str(),
            index + 1
        ))
    }
}

/// Blocking transport contract. Implementations own their I/O loop and are
/// hosted by Tokio's blocking pool or a dedicated thread; the UI never calls
/// `run` directly.
pub trait SessionTransport: Send + 'static {
    fn id(&self) -> &TransportId;
    fn run(
        self: Box<Self>,
        commands: Receiver<TransportCommand>,
        events: SyncSender<TransportEvent>,
    ) -> Result<(), TerminalError>;
}

impl SessionTransport for ReplayTransport {
    fn id(&self) -> &TransportId {
        &self.id
    }

    fn run(
        self: Box<Self>,
        commands: Receiver<TransportCommand>,
        events: SyncSender<TransportEvent>,
    ) -> Result<(), TerminalError> {
        self.validate()?;

        for (index, step) in self.steps.iter().enumerate() {
            match step {
                ReplayStep::Emit(event) => events.send(event.clone()).map_err(|_| {
                    self.error_at(index, "event receiver disconnected while replaying")
                })?,
                ReplayStep::Expect(expected) => {
                    let actual = commands.recv().map_err(|_| {
                        self.error_at(index, "command sender disconnected while replaying")
                    })?;
                    if actual != *expected {
                        return Err(self.error_at(
                            index,
                            format_args!("expected command {expected:?}, received {actual:?}"),
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "transport_tests.rs"]
mod tests;
