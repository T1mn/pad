use std::any::Any;
use std::sync::mpsc::{
    self, Receiver, RecvError, SendError, SyncSender, TryRecvError, TrySendError,
};
use std::thread::{self, JoinHandle};

use crate::panic_boundary::catch_isolated_unwind;

use super::{SessionTransport, TerminalError, TransportCommand, TransportEvent, TransportId};

pub const DEFAULT_COMMAND_QUEUE_CAPACITY: usize = 256;
pub const DEFAULT_EVENT_QUEUE_CAPACITY: usize = 64;

const WORKER_NAME_PREFIX: &str = "pad-terminal-transport";
const MAX_THREAD_ID_CHARS: usize = 32;

/// Configuration and factory for bounded transport worker threads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportRuntime {
    command_capacity: usize,
    event_capacity: usize,
}

impl TransportRuntime {
    pub fn new(command_capacity: usize, event_capacity: usize) -> Result<Self, TerminalError> {
        if command_capacity == 0 {
            return Err(TerminalError::new(
                "terminal transport command queue capacity must be greater than zero",
            ));
        }
        if event_capacity == 0 {
            return Err(TerminalError::new(
                "terminal transport event queue capacity must be greater than zero",
            ));
        }
        Ok(Self {
            command_capacity,
            event_capacity,
        })
    }

    pub fn command_capacity(&self) -> usize {
        self.command_capacity
    }

    pub fn event_capacity(&self) -> usize {
        self.event_capacity
    }

    pub fn spawn(
        &self,
        transport: Box<dyn SessionTransport>,
    ) -> Result<TransportHandle, TerminalError> {
        let id = transport.id().clone();
        let thread_name = worker_thread_name(&id);
        let (commands, command_receiver) = mpsc::sync_channel(self.command_capacity);
        let (event_sender, events) = mpsc::sync_channel(self.event_capacity);
        let (completion_sender, completion) = mpsc::sync_channel(1);

        let worker = thread::Builder::new()
            .name(thread_name.clone())
            .spawn(move || {
                let result =
                    catch_isolated_unwind(|| transport.run(command_receiver, event_sender))
                        .unwrap_or_else(|payload| {
                            Err(TerminalError::new(format!(
                                "terminal transport worker panicked: {}",
                                panic_message(payload)
                            )))
                        });
                let _ = completion_sender.send(result);
            })
            .map_err(|error| {
                TerminalError::new(format!(
                    "failed to start terminal transport '{}': {error}",
                    id.as_str()
                ))
            })?;

        Ok(TransportHandle {
            id,
            thread_name,
            commands: Some(commands),
            events: Some(events),
            completion,
            cached_completion: None,
            worker: Some(worker),
        })
    }
}

impl Default for TransportRuntime {
    fn default() -> Self {
        Self {
            command_capacity: DEFAULT_COMMAND_QUEUE_CAPACITY,
            event_capacity: DEFAULT_EVENT_QUEUE_CAPACITY,
        }
    }
}

/// Result of attempting a nonblocking graceful shutdown request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShutdownSignal {
    Enqueued,
    QueueFull,
    Disconnected,
    AlreadyRequested,
}

/// The UI-facing half of a running [`SessionTransport`].
///
/// Blocking operations are explicit (`send`, `recv`, and `recv_completion`).
/// `shutdown` itself never blocks: it attempts to enqueue `Shutdown`, then
/// closes the command stream. Events remain available so callers can consume a
/// final exit event. Dropping the handle additionally disconnects the event
/// receiver and detaches a worker that has not finished yet.
pub struct TransportHandle {
    id: TransportId,
    thread_name: String,
    commands: Option<SyncSender<TransportCommand>>,
    events: Option<Receiver<TransportEvent>>,
    completion: Receiver<Result<(), TerminalError>>,
    cached_completion: Option<Result<(), TerminalError>>,
    worker: Option<JoinHandle<()>>,
}

impl TransportHandle {
    pub fn id(&self) -> &TransportId {
        &self.id
    }

    pub fn thread_name(&self) -> &str {
        &self.thread_name
    }

    pub fn send(&self, command: TransportCommand) -> Result<(), SendError<TransportCommand>> {
        match &self.commands {
            Some(commands) => commands.send(command),
            None => Err(SendError(command)),
        }
    }

    pub fn try_send(
        &self,
        command: TransportCommand,
    ) -> Result<(), TrySendError<TransportCommand>> {
        match &self.commands {
            Some(commands) => commands.try_send(command),
            None => Err(TrySendError::Disconnected(command)),
        }
    }

    pub fn recv(&self) -> Result<TransportEvent, RecvError> {
        match &self.events {
            Some(events) => events.recv(),
            None => Err(RecvError),
        }
    }

    pub fn try_recv(&self) -> Result<TransportEvent, TryRecvError> {
        match &self.events {
            Some(events) => events.try_recv(),
            None => Err(TryRecvError::Disconnected),
        }
    }

    pub fn drain_events(&self) -> Vec<TransportEvent> {
        self.events
            .as_ref()
            .map(|events| events.try_iter().collect())
            .unwrap_or_default()
    }

    /// Disconnects the event receiver without waiting for the worker.
    ///
    /// This is useful after a caller no longer needs buffered output, and
    /// releases a transport blocked on a full event queue.
    pub fn disconnect_events(&mut self) {
        self.events.take();
        self.reap_finished_worker();
    }

    /// Attempts to request a graceful shutdown and closes the command stream.
    /// This method never waits for queue capacity or worker completion.
    pub fn shutdown(&mut self) -> ShutdownSignal {
        let Some(commands) = self.commands.take() else {
            return ShutdownSignal::AlreadyRequested;
        };
        let signal = match commands.try_send(TransportCommand::Shutdown) {
            Ok(()) => ShutdownSignal::Enqueued,
            Err(TrySendError::Full(_)) => ShutdownSignal::QueueFull,
            Err(TrySendError::Disconnected(_)) => ShutdownSignal::Disconnected,
        };
        drop(commands);
        self.reap_finished_worker();
        signal
    }

    /// Returns the worker result if it is currently available.
    /// Completed results are cached, so observing them is repeatable.
    pub fn try_completion(&mut self) -> Option<Result<(), TerminalError>> {
        if self.cached_completion.is_none() {
            match self.completion.try_recv() {
                Ok(result) => self.cached_completion = Some(result),
                Err(TryRecvError::Empty) => return None,
                Err(TryRecvError::Disconnected) => {
                    self.cached_completion = Some(Err(self.missing_completion_error()))
                }
            }
        }
        self.reap_finished_worker();
        self.cached_completion.clone()
    }

    /// Waits for transport completion.
    ///
    /// Callers must continue receiving events before using this method when a
    /// transport may still produce output; bounded event backpressure is
    /// intentionally preserved.
    pub fn recv_completion(&mut self) -> Result<(), TerminalError> {
        if self.cached_completion.is_none() {
            self.cached_completion = Some(
                self.completion
                    .recv()
                    .unwrap_or_else(|_| Err(self.missing_completion_error())),
            );
        }
        self.reap_finished_worker();
        self.cached_completion
            .clone()
            .expect("transport completion was populated")
    }

    pub fn is_finished(&mut self) -> bool {
        if self.try_completion().is_some() {
            return true;
        }
        self.worker.as_ref().is_none_or(JoinHandle::is_finished)
    }

    fn missing_completion_error(&self) -> TerminalError {
        TerminalError::new(format!(
            "terminal transport '{}' dropped its completion result",
            self.id.as_str()
        ))
    }

    fn reap_finished_worker(&mut self) {
        let finished = self.worker.as_ref().is_some_and(JoinHandle::is_finished);
        if finished {
            let worker = self.worker.take().expect("finished worker was present");
            if let Err(payload) = worker.join() {
                self.cached_completion.get_or_insert_with(|| {
                    Err(TerminalError::new(format!(
                        "terminal transport worker panicked: {}",
                        panic_message(payload)
                    )))
                });
            }
        }
    }
}

impl Drop for TransportHandle {
    fn drop(&mut self) {
        let _ = self.shutdown();
        self.events.take();
        self.reap_finished_worker();
        // Dropping JoinHandle detaches a worker that ignores both disconnected
        // channels. Destructors must not trust arbitrary transports to exit.
        self.worker.take();
    }
}

fn worker_thread_name(id: &TransportId) -> String {
    let suffix: String = id
        .as_str()
        .chars()
        .take(MAX_THREAD_ID_CHARS)
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    let suffix = if suffix.is_empty() {
        "unnamed"
    } else {
        &suffix
    };
    format!("{WORKER_NAME_PREFIX}-{suffix}")
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{self, TryRecvError, TrySendError};
    use std::time::Duration;

    use super::*;
    use crate::terminal_runtime::{ReplayStep, ReplayTransport, TerminalSize, TransportExit};

    const TEST_TIMEOUT: Duration = Duration::from_secs(2);

    #[test]
    fn runtime_rejects_unbounded_configuration() {
        assert!(TransportRuntime::new(0, 1).is_err());
        assert!(TransportRuntime::new(1, 0).is_err());

        let runtime = TransportRuntime::new(3, 5).unwrap();
        assert_eq!(runtime.command_capacity(), 3);
        assert_eq!(runtime.event_capacity(), 5);
    }

    #[test]
    fn replay_preserves_bidirectional_order_and_graceful_shutdown() {
        let size = TerminalSize::new(100, 30);
        let exit = TransportExit {
            code: Some(0),
            signaled: false,
        };
        let replay = ReplayTransport::new(
            TransportId::new("ordered"),
            [
                ReplayStep::output(b"ready"),
                ReplayStep::expect_input(b"status\r"),
                ReplayStep::output(b"working"),
                ReplayStep::expect_resize(size),
                ReplayStep::output(b"resized"),
                ReplayStep::expect_shutdown(),
                ReplayStep::exit(exit),
            ],
        );
        let mut handle = TransportRuntime::new(1, 1)
            .unwrap()
            .spawn(Box::new(replay))
            .unwrap();

        assert_eq!(handle.id().as_str(), "ordered");
        assert_eq!(
            handle.recv().unwrap(),
            TransportEvent::Output(b"ready".to_vec())
        );
        assert_eq!(handle.try_recv(), Err(TryRecvError::Empty));
        handle
            .send(TransportCommand::Input(b"status\r".to_vec()))
            .unwrap();
        assert_eq!(
            handle.recv().unwrap(),
            TransportEvent::Output(b"working".to_vec())
        );
        handle.try_send(TransportCommand::Resize(size)).unwrap();
        assert_eq!(
            handle.recv().unwrap(),
            TransportEvent::Output(b"resized".to_vec())
        );

        assert_eq!(handle.shutdown(), ShutdownSignal::Enqueued);
        assert_eq!(handle.shutdown(), ShutdownSignal::AlreadyRequested);
        assert_eq!(handle.recv().unwrap(), TransportEvent::Exited(exit));
        assert_eq!(handle.recv_completion(), Ok(()));
        assert!(handle.is_finished());
        assert!(matches!(
            handle.try_send(TransportCommand::Shutdown),
            Err(TrySendError::Disconnected(TransportCommand::Shutdown))
        ));
    }

    #[test]
    fn bounded_event_queue_applies_backpressure_and_keeps_order() {
        let (first_sent, first_observed) = mpsc::sync_channel(1);
        let (second_sent, second_observed) = mpsc::sync_channel(1);
        let transport = BackpressureTransport {
            id: TransportId::new("bounded"),
            first_sent,
            second_sent,
        };
        let mut handle = TransportRuntime::new(1, 1)
            .unwrap()
            .spawn(Box::new(transport))
            .unwrap();

        first_observed.recv().unwrap();
        assert_eq!(second_observed.try_recv(), Err(TryRecvError::Empty));
        assert_eq!(handle.try_completion(), None);
        assert_eq!(
            handle.recv().unwrap(),
            TransportEvent::Output(b"one".to_vec())
        );

        second_observed.recv().unwrap();
        assert_eq!(
            handle.recv().unwrap(),
            TransportEvent::Output(b"two".to_vec())
        );
        assert_eq!(handle.recv_completion(), Ok(()));
    }

    #[test]
    fn drain_events_returns_all_buffered_events_in_order() {
        let exit = TransportExit {
            code: None,
            signaled: true,
        };
        let replay = ReplayTransport::from_output_chunks(
            TransportId::new("drain"),
            [b"one".to_vec(), b"two".to_vec()],
            exit,
        );
        let mut handle = TransportRuntime::new(1, 3)
            .unwrap()
            .spawn(Box::new(replay))
            .unwrap();

        assert_eq!(handle.recv_completion(), Ok(()));
        assert_eq!(
            handle.drain_events(),
            vec![
                TransportEvent::Output(b"one".to_vec()),
                TransportEvent::Output(b"two".to_vec()),
                TransportEvent::Exited(exit),
            ]
        );
        assert_eq!(handle.try_recv(), Err(TryRecvError::Disconnected));
    }

    #[test]
    fn worker_name_is_safe_and_visible_inside_transport() {
        let transport = ThreadNameTransport {
            id: TransportId::new("tmux pane/\0unicode-界"),
        };
        let mut handle = TransportRuntime::new(1, 1)
            .unwrap()
            .spawn(Box::new(transport))
            .unwrap();
        let expected = handle.thread_name().as_bytes().to_vec();

        assert_eq!(handle.recv().unwrap(), TransportEvent::Output(expected));
        assert_eq!(handle.recv_completion(), Ok(()));
        assert!(!handle.thread_name().contains('\0'));
        assert!(!handle.thread_name().contains('界'));
    }

    #[test]
    fn worker_error_and_completion_observation_are_repeatable() {
        let mut handle = TransportRuntime::new(1, 1)
            .unwrap()
            .spawn(Box::new(ErrorTransport {
                id: TransportId::new("fault"),
            }))
            .unwrap();

        assert_eq!(
            handle.recv().unwrap(),
            TransportEvent::Output(b"before-error".to_vec())
        );
        let error = handle.recv_completion().unwrap_err();
        assert_eq!(error.to_string(), "injected transport failure");
        assert_eq!(handle.try_completion(), Some(Err(error)));
    }

    #[test]
    fn panic_is_converted_to_a_worker_error() {
        let mut handle = TransportRuntime::new(1, 1)
            .unwrap()
            .spawn(Box::new(PanicTransport {
                id: TransportId::new("panic"),
            }))
            .unwrap();

        let error = handle.recv_completion().unwrap_err().to_string();
        assert!(error.contains("terminal transport worker panicked"));
        assert!(error.contains("injected panic"));
    }

    #[test]
    fn disconnecting_event_receiver_releases_replay_sender() {
        let replay = ReplayTransport::new(
            TransportId::new("event-disconnect"),
            [
                ReplayStep::output(b"first"),
                ReplayStep::expect_input(b"continue"),
                ReplayStep::output(b"blocked"),
            ],
        );
        let mut handle = TransportRuntime::new(1, 1)
            .unwrap()
            .spawn(Box::new(replay))
            .unwrap();

        assert_eq!(
            handle.recv().unwrap(),
            TransportEvent::Output(b"first".to_vec())
        );
        handle.disconnect_events();
        handle
            .send(TransportCommand::Input(b"continue".to_vec()))
            .unwrap();

        let error = handle.recv_completion().unwrap_err().to_string();
        assert!(error.contains("event receiver disconnected while replaying"));
        assert_eq!(handle.try_recv(), Err(TryRecvError::Disconnected));
    }

    #[test]
    fn drop_disconnects_a_full_event_queue_without_waiting() {
        let (first_sent, first_observed) = mpsc::sync_channel(1);
        let (disconnected, disconnect_observed) = mpsc::sync_channel(1);
        let handle = TransportRuntime::new(1, 1)
            .unwrap()
            .spawn(Box::new(DisconnectProbeTransport {
                id: TransportId::new("drop"),
                first_sent,
                disconnected,
            }))
            .unwrap();

        first_observed.recv().unwrap();
        drop(handle);
        disconnect_observed
            .recv_timeout(TEST_TIMEOUT)
            .expect("drop should disconnect a blocked event sender");
    }

    #[test]
    fn drop_disconnects_a_worker_waiting_for_more_commands() {
        let (waiting, waiting_observed) = mpsc::sync_channel(1);
        let (disconnected, disconnect_observed) = mpsc::sync_channel(1);
        let handle = TransportRuntime::new(1, 1)
            .unwrap()
            .spawn(Box::new(CommandDisconnectProbeTransport {
                id: TransportId::new("command-drop"),
                waiting,
                disconnected,
            }))
            .unwrap();

        waiting_observed.recv().unwrap();
        drop(handle);
        disconnect_observed
            .recv_timeout(TEST_TIMEOUT)
            .expect("drop should disconnect a waiting command receiver");
    }

    #[test]
    fn shutdown_does_not_block_when_command_and_event_queues_are_full() {
        let (first_sent, first_observed) = mpsc::sync_channel(1);
        let (disconnected, disconnect_observed) = mpsc::sync_channel(1);
        let mut handle = TransportRuntime::new(1, 1)
            .unwrap()
            .spawn(Box::new(DisconnectProbeTransport {
                id: TransportId::new("full-shutdown"),
                first_sent,
                disconnected,
            }))
            .unwrap();

        first_observed.recv().unwrap();
        handle
            .try_send(TransportCommand::Input(b"queued".to_vec()))
            .unwrap();
        assert_eq!(handle.shutdown(), ShutdownSignal::QueueFull);
        assert!(matches!(
            handle.try_send(TransportCommand::Shutdown),
            Err(TrySendError::Disconnected(TransportCommand::Shutdown))
        ));

        handle.disconnect_events();
        disconnect_observed
            .recv_timeout(TEST_TIMEOUT)
            .expect("event disconnect should release the worker");
        assert_eq!(
            handle.recv_completion().unwrap_err().to_string(),
            "event receiver disconnected"
        );
    }

    struct BackpressureTransport {
        id: TransportId,
        first_sent: SyncSender<()>,
        second_sent: SyncSender<()>,
    }

    impl SessionTransport for BackpressureTransport {
        fn id(&self) -> &TransportId {
            &self.id
        }

        fn run(
            self: Box<Self>,
            _commands: Receiver<TransportCommand>,
            events: SyncSender<TransportEvent>,
        ) -> Result<(), TerminalError> {
            send_output(&events, b"one")?;
            self.first_sent
                .send(())
                .map_err(|_| TerminalError::new("first probe disconnected"))?;
            send_output(&events, b"two")?;
            self.second_sent
                .send(())
                .map_err(|_| TerminalError::new("second probe disconnected"))
        }
    }

    struct ThreadNameTransport {
        id: TransportId,
    }

    impl SessionTransport for ThreadNameTransport {
        fn id(&self) -> &TransportId {
            &self.id
        }

        fn run(
            self: Box<Self>,
            _commands: Receiver<TransportCommand>,
            events: SyncSender<TransportEvent>,
        ) -> Result<(), TerminalError> {
            let name = thread::current()
                .name()
                .ok_or_else(|| TerminalError::new("worker thread was unnamed"))?
                .as_bytes()
                .to_vec();
            events
                .send(TransportEvent::Output(name))
                .map_err(|_| TerminalError::new("event receiver disconnected"))
        }
    }

    struct ErrorTransport {
        id: TransportId,
    }

    impl SessionTransport for ErrorTransport {
        fn id(&self) -> &TransportId {
            &self.id
        }

        fn run(
            self: Box<Self>,
            _commands: Receiver<TransportCommand>,
            events: SyncSender<TransportEvent>,
        ) -> Result<(), TerminalError> {
            send_output(&events, b"before-error")?;
            Err(TerminalError::new("injected transport failure"))
        }
    }

    struct PanicTransport {
        id: TransportId,
    }

    impl SessionTransport for PanicTransport {
        fn id(&self) -> &TransportId {
            &self.id
        }

        fn run(
            self: Box<Self>,
            _commands: Receiver<TransportCommand>,
            _events: SyncSender<TransportEvent>,
        ) -> Result<(), TerminalError> {
            panic!("injected panic");
        }
    }

    struct DisconnectProbeTransport {
        id: TransportId,
        first_sent: SyncSender<()>,
        disconnected: SyncSender<()>,
    }

    struct CommandDisconnectProbeTransport {
        id: TransportId,
        waiting: SyncSender<()>,
        disconnected: SyncSender<()>,
    }

    impl SessionTransport for CommandDisconnectProbeTransport {
        fn id(&self) -> &TransportId {
            &self.id
        }

        fn run(
            self: Box<Self>,
            commands: Receiver<TransportCommand>,
            _events: SyncSender<TransportEvent>,
        ) -> Result<(), TerminalError> {
            self.waiting
                .send(())
                .map_err(|_| TerminalError::new("waiting probe disconnected"))?;
            while commands.recv().is_ok() {}
            self.disconnected
                .send(())
                .map_err(|_| TerminalError::new("command disconnect probe disconnected"))
        }
    }

    impl SessionTransport for DisconnectProbeTransport {
        fn id(&self) -> &TransportId {
            &self.id
        }

        fn run(
            self: Box<Self>,
            _commands: Receiver<TransportCommand>,
            events: SyncSender<TransportEvent>,
        ) -> Result<(), TerminalError> {
            send_output(&events, b"queued")?;
            self.first_sent
                .send(())
                .map_err(|_| TerminalError::new("first probe disconnected"))?;
            let result = send_output(&events, b"blocked");
            if result.is_err() {
                let _ = self.disconnected.send(());
            }
            result
        }
    }

    fn send_output(events: &SyncSender<TransportEvent>, bytes: &[u8]) -> Result<(), TerminalError> {
        events
            .send(TransportEvent::Output(bytes.to_vec()))
            .map_err(|_| TerminalError::new("event receiver disconnected"))
    }
}
