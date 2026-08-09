use std::sync::mpsc::{self, TryRecvError, TrySendError};
use std::time::Duration;

use super::*;
use crate::terminal_runtime::{ReplayStep, ReplayTransport, TerminalSize, TransportExit};

const TEST_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) fn runtime_rejects_unbounded_configuration() {
    assert!(TransportRuntime::new(0, 1).is_err());
    assert!(TransportRuntime::new(1, 0).is_err());

    let runtime = TransportRuntime::new(3, 5).unwrap();
    assert_eq!(runtime.command_capacity(), 3);
    assert_eq!(runtime.event_capacity(), 5);
}

pub(crate) fn replay_preserves_bidirectional_order_and_graceful_shutdown() {
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

pub(crate) fn bounded_event_queue_applies_backpressure_and_keeps_order() {
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

pub(crate) fn drain_events_returns_all_buffered_events_in_order() {
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

pub(crate) fn worker_name_is_safe_and_visible_inside_transport() {
    let transport = ThreadNameTransport {
        id: TransportId::new("native pane/\0unicode-界"),
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

pub(crate) fn worker_error_and_completion_observation_are_repeatable() {
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

pub(crate) fn panic_is_converted_to_a_worker_error() {
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

pub(crate) fn disconnecting_event_receiver_releases_replay_sender() {
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

pub(crate) fn drop_disconnects_a_full_event_queue_without_waiting() {
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

pub(crate) fn drop_disconnects_a_worker_waiting_for_more_commands() {
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

pub(crate) fn shutdown_does_not_block_when_command_and_event_queues_are_full() {
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
