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
#[path = "transport_runtime_tests.rs"]
pub(crate) mod tests;
