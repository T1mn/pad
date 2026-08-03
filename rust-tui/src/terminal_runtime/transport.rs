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
    /// Confirms that the transport has applied a requested terminal size.
    /// Output emitted before this event belongs to the previous size; output
    /// emitted after it belongs to `size`.
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
mod tests {
    use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
    use std::thread::{self, JoinHandle};

    use super::*;

    const QUEUE_CAPACITY: usize = 1;

    #[test]
    fn output_recording_preserves_chunks_and_exit() {
        let exit = TransportExit {
            code: Some(7),
            signaled: false,
        };
        let replay = ReplayTransport::from_output_chunks(
            TransportId::new("recording"),
            [b"first".to_vec(), Vec::new(), b"third".to_vec()],
            exit,
        );
        assert_eq!(replay.id().as_str(), "recording");

        let (commands, events, worker) = spawn(replay);
        assert_eq!(
            events.recv().unwrap(),
            TransportEvent::Output(b"first".to_vec())
        );
        assert_eq!(events.recv().unwrap(), TransportEvent::Output(Vec::new()));
        assert_eq!(
            events.recv().unwrap(),
            TransportEvent::Output(b"third".to_vec())
        );
        assert_eq!(events.recv().unwrap(), TransportEvent::Exited(exit));

        drop(commands);
        assert_eq!(worker.join().unwrap(), Ok(()));
    }

    #[test]
    fn commands_gate_later_events_in_recorded_order() {
        let size = TerminalSize::new(132, 43);
        let exit = TransportExit {
            code: Some(0),
            signaled: false,
        };
        let replay = ReplayTransport::new(
            TransportId::new("interactive"),
            [
                ReplayStep::output(b"ready"),
                ReplayStep::expect_input(b"help\r"),
                ReplayStep::output(b"help\r\n"),
                ReplayStep::expect_resize(size),
                ReplayStep::resize_applied(size),
                ReplayStep::output(b"resized"),
                ReplayStep::expect_shutdown(),
                ReplayStep::exit(exit),
            ],
        );

        let (commands, events, worker) = spawn(replay);
        assert_eq!(
            events.recv().unwrap(),
            TransportEvent::Output(b"ready".to_vec())
        );
        assert_eq!(events.try_recv(), Err(TryRecvError::Empty));

        commands
            .send(TransportCommand::Input(b"help\r".to_vec()))
            .unwrap();
        assert_eq!(
            events.recv().unwrap(),
            TransportEvent::Output(b"help\r\n".to_vec())
        );
        assert_eq!(events.try_recv(), Err(TryRecvError::Empty));

        commands.send(TransportCommand::Resize(size)).unwrap();
        assert_eq!(events.recv().unwrap(), TransportEvent::ResizeApplied(size));
        assert_eq!(
            events.recv().unwrap(),
            TransportEvent::Output(b"resized".to_vec())
        );
        assert_eq!(events.try_recv(), Err(TryRecvError::Empty));

        commands.send(TransportCommand::Shutdown).unwrap();
        assert_eq!(events.recv().unwrap(), TransportEvent::Exited(exit));
        assert_eq!(worker.join().unwrap(), Ok(()));
    }

    #[test]
    fn mismatched_command_fails_at_the_exact_step() {
        let replay = ReplayTransport::new(
            TransportId::new("mismatch"),
            [
                ReplayStep::output(b"prompt"),
                ReplayStep::expect_resize(TerminalSize::new(80, 24)),
            ],
        );

        let (commands, events, worker) = spawn(replay);
        assert_eq!(
            events.recv().unwrap(),
            TransportEvent::Output(b"prompt".to_vec())
        );
        commands
            .send(TransportCommand::Input(b"unexpected".to_vec()))
            .unwrap();

        let error = worker.join().unwrap().unwrap_err().to_string();
        assert!(error.contains("replay transport 'mismatch' step 2"));
        assert!(error.contains("expected command Resize"));
        assert!(error.contains("received Input"));
    }

    #[test]
    fn disconnected_command_sender_is_reported() {
        let replay = ReplayTransport::new(
            TransportId::new("commands-closed"),
            [ReplayStep::expect_shutdown()],
        );
        let (commands, _events, worker) = spawn(replay);
        drop(commands);

        let error = worker.join().unwrap().unwrap_err().to_string();
        assert_eq!(
            error,
            "replay transport 'commands-closed' step 1: command sender disconnected while replaying"
        );
    }

    #[test]
    fn disconnected_event_receiver_is_reported() {
        let replay = ReplayTransport::new(
            TransportId::new("events-closed"),
            [ReplayStep::output(b"orphaned")],
        );
        let (commands, command_receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
        let (event_sender, events) = mpsc::sync_channel(QUEUE_CAPACITY);
        drop(events);
        let worker = thread::spawn(move || Box::new(replay).run(command_receiver, event_sender));

        let error = worker.join().unwrap().unwrap_err().to_string();
        assert_eq!(
            error,
            "replay transport 'events-closed' step 1: event receiver disconnected while replaying"
        );
        drop(commands);
    }

    #[test]
    fn bounded_queues_apply_backpressure_without_reordering() {
        let exit = TransportExit {
            code: Some(0),
            signaled: false,
        };
        let replay = ReplayTransport::new(
            TransportId::new("backpressure"),
            [
                ReplayStep::output(b"ready"),
                ReplayStep::expect_input(b"accepted"),
                ReplayStep::exit(exit),
            ],
        );
        let (commands, command_receiver) = mpsc::sync_channel(1);
        // A rendezvous channel is bounded with zero buffered slots. The replay
        // cannot consume commands until this event has been received.
        let (event_sender, events) = mpsc::sync_channel(0);
        let worker = thread::spawn(move || Box::new(replay).run(command_receiver, event_sender));

        commands
            .try_send(TransportCommand::Input(b"accepted".to_vec()))
            .unwrap();
        assert_eq!(
            commands.try_send(TransportCommand::Input(b"overflow".to_vec())),
            Err(TrySendError::Full(TransportCommand::Input(
                b"overflow".to_vec()
            )))
        );

        assert_eq!(
            events.recv().unwrap(),
            TransportEvent::Output(b"ready".to_vec())
        );
        assert_eq!(events.recv().unwrap(), TransportEvent::Exited(exit));
        assert_eq!(worker.join().unwrap(), Ok(()));
    }

    #[test]
    fn exit_must_be_the_final_step_and_is_not_partially_replayed() {
        let replay = ReplayTransport::new(
            TransportId::new("invalid-exit"),
            [
                ReplayStep::exit(TransportExit {
                    code: Some(0),
                    signaled: false,
                }),
                ReplayStep::output(b"too late"),
            ],
        );
        let (commands, events, worker) = spawn(replay);

        let error = worker.join().unwrap().unwrap_err().to_string();
        assert_eq!(
            error,
            "replay transport 'invalid-exit' step 1: exit event must be the final replay step"
        );
        assert_eq!(events.try_recv(), Err(TryRecvError::Disconnected));
        drop(commands);
    }

    #[test]
    fn cloned_recording_replays_identically() {
        let replay = ReplayTransport::from_output_chunks(
            TransportId::new("repeatable"),
            [b"a".to_vec(), b"b".to_vec()],
            TransportExit {
                code: None,
                signaled: true,
            },
        );

        assert_eq!(collect_events(replay.clone()), collect_events(replay));
    }

    fn spawn(
        replay: ReplayTransport,
    ) -> (
        SyncSender<TransportCommand>,
        Receiver<TransportEvent>,
        JoinHandle<Result<(), TerminalError>>,
    ) {
        let (command_sender, command_receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
        let (event_sender, event_receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
        let worker = thread::spawn(move || Box::new(replay).run(command_receiver, event_sender));
        (command_sender, event_receiver, worker)
    }

    fn collect_events(replay: ReplayTransport) -> Vec<TransportEvent> {
        let (commands, events, worker) = spawn(replay);
        let mut collected = Vec::new();
        while let Ok(event) = events.recv() {
            collected.push(event);
        }
        drop(commands);
        assert_eq!(worker.join().unwrap(), Ok(()));
        collected
    }
}
