mod host;

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::panic_boundary::catch_isolated_unwind;

use super::{
    LivePaneRuntime, NativePtyCommand, NativePtyTransport, PaneFrame, PaneId, PaneSpec,
    SessionTransport, TerminalError, TerminalScroll, TerminalSize, TransportExit,
};
use host::ControllerHost;

pub const DEFAULT_CONTROLLER_COMMAND_CAPACITY: usize = 256;

const HOST_THREAD_NAME: &str = "pad-terminal-controller";
const HOST_POLL_INTERVAL: Duration = Duration::from_millis(1);

/// Generation of one pane process. Commands from older generations are
/// ignored after a pane ID has been reopened.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PaneEpoch(u64);

impl PaneEpoch {
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Immutable state published by the terminal host for UI rendering.
///
/// Cloning this value only clones the frame `Arc`; the UI never talks to the
/// blocking parser or transport runtimes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishedPane {
    pub epoch: PaneEpoch,
    pub revision: u64,
    pub frame: Option<Arc<PaneFrame>>,
    pub error: Option<Arc<str>>,
    pub exit: Option<TransportExit>,
    pub is_open: bool,
}

impl PublishedPane {
    fn pending(epoch: PaneEpoch, revision: u64) -> Self {
        Self {
            epoch,
            revision,
            frame: None,
            error: None,
            exit: None,
            is_open: false,
        }
    }
}

/// Short-lived read access to the latest pane frames.
#[derive(Clone, Default)]
pub struct TerminalFrameReader {
    panes: Arc<RwLock<HashMap<PaneId, Arc<PublishedPane>>>>,
}

impl TerminalFrameReader {
    pub fn latest(&self, pane_id: &PaneId) -> Option<Arc<PublishedPane>> {
        read_unpoisoned(&self.panes).get(pane_id).cloned()
    }

    pub fn latest_for_epoch(
        &self,
        pane_id: &PaneId,
        epoch: PaneEpoch,
    ) -> Option<Arc<PublishedPane>> {
        self.latest(pane_id).filter(|pane| pane.epoch == epoch)
    }

    pub fn pane_ids(&self) -> Vec<PaneId> {
        read_unpoisoned(&self.panes).keys().cloned().collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativePaneRequest {
    pub spec: PaneSpec,
    pub size: TerminalSize,
    pub command: NativePtyCommand,
}

/// A nonblocking controller submission failure. The original value is
/// returned so input and resize requests can be retried without data loss.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControllerQueueError<T> {
    Full(T),
    Disconnected(T),
}

impl<T> ControllerQueueError<T> {
    pub fn into_inner(self) -> T {
        match self {
            Self::Full(value) | Self::Disconnected(value) => value,
        }
    }

    pub fn is_full(&self) -> bool {
        matches!(self, Self::Full(_))
    }
}

impl<T> fmt::Display for ControllerQueueError<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full(_) => formatter.write_str("terminal controller command queue is full"),
            Self::Disconnected(_) => {
                formatter.write_str("terminal controller command queue is disconnected")
            }
        }
    }
}

impl<T: fmt::Debug> std::error::Error for ControllerQueueError<T> {}

/// UI-facing handle for the backend terminal coordinator.
///
/// Every mutating method is nonblocking and returns the original payload when
/// bounded backpressure is reached. `shutdown` is the one explicit joining
/// operation; `Drop` only disconnects and detaches the host thread.
pub struct TerminalController {
    commands: Option<SyncSender<ControllerCommand>>,
    frames: TerminalFrameReader,
    next_epoch: AtomicU64,
    stopping: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<(), TerminalError>>>,
}

impl TerminalController {
    pub fn start(runtime: LivePaneRuntime) -> Result<Self, TerminalError> {
        Self::start_with_capacity(runtime, DEFAULT_CONTROLLER_COMMAND_CAPACITY)
    }

    pub fn start_with_capacity(
        runtime: LivePaneRuntime,
        command_capacity: usize,
    ) -> Result<Self, TerminalError> {
        Self::start_inner(runtime, command_capacity, None)
    }

    fn start_inner(
        runtime: LivePaneRuntime,
        command_capacity: usize,
        #[cfg(test)] startup_gate: Option<Receiver<()>>,
        #[cfg(not(test))] _startup_gate: Option<Receiver<()>>,
    ) -> Result<Self, TerminalError> {
        if command_capacity == 0 {
            return Err(TerminalError::new(
                "terminal controller command queue capacity must be greater than zero",
            ));
        }

        let (commands, receiver) = mpsc::sync_channel(command_capacity);
        let panes = Arc::new(RwLock::new(HashMap::new()));
        let frames = TerminalFrameReader {
            panes: Arc::clone(&panes),
        };
        let stopping = Arc::new(AtomicBool::new(false));
        let worker_stopping = Arc::clone(&stopping);
        let worker = thread::Builder::new()
            .name(HOST_THREAD_NAME.to_string())
            .spawn(move || {
                #[cfg(test)]
                if let Some(gate) = startup_gate {
                    let _ = gate.recv();
                }
                catch_isolated_unwind(|| {
                    ControllerHost::new(runtime, panes, command_capacity)
                        .run(receiver, worker_stopping)
                })
                .unwrap_or_else(|payload| {
                    Err(TerminalError::new(format!(
                        "terminal controller host panicked: {}",
                        panic_message(payload)
                    )))
                })
            })
            .map_err(|error| {
                TerminalError::new(format!("failed to start terminal controller host: {error}"))
            })?;

        Ok(Self {
            commands: Some(commands),
            frames,
            next_epoch: AtomicU64::new(1),
            stopping,
            worker: Some(worker),
        })
    }

    pub fn host_thread_name(&self) -> &'static str {
        HOST_THREAD_NAME
    }

    pub fn frames(&self) -> TerminalFrameReader {
        self.frames.clone()
    }

    // Returning the complete request is intentional: callers must be able to
    // retry a full bounded queue without reconstructing process configuration.
    #[allow(clippy::result_large_err)]
    pub fn open_native(
        &self,
        request: NativePaneRequest,
    ) -> Result<PaneEpoch, ControllerQueueError<NativePaneRequest>> {
        let epoch = self.allocate_epoch();
        let command = ControllerCommand::OpenNative { epoch, request };
        match self.try_send(command) {
            Ok(()) => Ok(epoch),
            Err(TrySendError::Full(ControllerCommand::OpenNative { request, .. })) => {
                Err(ControllerQueueError::Full(request))
            }
            Err(TrySendError::Disconnected(ControllerCommand::OpenNative { request, .. })) => {
                Err(ControllerQueueError::Disconnected(request))
            }
            Err(_) => unreachable!("open_native submitted an OpenNative command"),
        }
    }

    pub fn input(
        &self,
        pane_id: PaneId,
        epoch: PaneEpoch,
        bytes: Vec<u8>,
    ) -> Result<(), ControllerQueueError<Vec<u8>>> {
        match self.try_send(ControllerCommand::Input {
            pane_id,
            epoch,
            bytes,
        }) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(ControllerCommand::Input { bytes, .. })) => {
                Err(ControllerQueueError::Full(bytes))
            }
            Err(TrySendError::Disconnected(ControllerCommand::Input { bytes, .. })) => {
                Err(ControllerQueueError::Disconnected(bytes))
            }
            Err(_) => unreachable!("input submitted an Input command"),
        }
    }

    pub fn resize(
        &self,
        pane_id: PaneId,
        epoch: PaneEpoch,
        size: TerminalSize,
    ) -> Result<(), ControllerQueueError<TerminalSize>> {
        match self.try_send(ControllerCommand::Resize {
            pane_id,
            epoch,
            size,
        }) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(ControllerCommand::Resize { size, .. })) => {
                Err(ControllerQueueError::Full(size))
            }
            Err(TrySendError::Disconnected(ControllerCommand::Resize { size, .. })) => {
                Err(ControllerQueueError::Disconnected(size))
            }
            Err(_) => unreachable!("resize submitted a Resize command"),
        }
    }

    pub fn scroll(
        &self,
        pane_id: PaneId,
        epoch: PaneEpoch,
        scroll: TerminalScroll,
    ) -> Result<(), ControllerQueueError<TerminalScroll>> {
        match self.try_send(ControllerCommand::Scroll {
            pane_id,
            epoch,
            scroll,
        }) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(ControllerCommand::Scroll { scroll, .. })) => {
                Err(ControllerQueueError::Full(scroll))
            }
            Err(TrySendError::Disconnected(ControllerCommand::Scroll { scroll, .. })) => {
                Err(ControllerQueueError::Disconnected(scroll))
            }
            Err(_) => unreachable!("scroll submitted a Scroll command"),
        }
    }

    pub fn set_label(
        &self,
        pane_id: PaneId,
        epoch: PaneEpoch,
        label: String,
    ) -> Result<(), ControllerQueueError<String>> {
        match self.try_send(ControllerCommand::SetLabel {
            pane_id,
            epoch,
            label,
        }) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(ControllerCommand::SetLabel { label, .. })) => {
                Err(ControllerQueueError::Full(label))
            }
            Err(TrySendError::Disconnected(ControllerCommand::SetLabel { label, .. })) => {
                Err(ControllerQueueError::Disconnected(label))
            }
            Err(_) => unreachable!("set_label submitted a SetLabel command"),
        }
    }

    pub fn close(&self, pane_id: PaneId, epoch: PaneEpoch) -> Result<(), ControllerQueueError<()>> {
        match self.try_send(ControllerCommand::Close { pane_id, epoch }) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(ControllerCommand::Close { .. })) => {
                Err(ControllerQueueError::Full(()))
            }
            Err(TrySendError::Disconnected(ControllerCommand::Close { .. })) => {
                Err(ControllerQueueError::Disconnected(()))
            }
            Err(_) => unreachable!("close submitted a Close command"),
        }
    }

    /// Stops and joins the named controller host. This may wait for engine
    /// cleanup and must not be invoked from the UI render path.
    pub fn shutdown(mut self) -> Result<(), TerminalError> {
        self.stopping.store(true, Ordering::Release);
        if let Some(commands) = self.commands.take() {
            let _ = commands.try_send(ControllerCommand::Shutdown);
            drop(commands);
        }
        self.join_worker()
    }

    fn allocate_epoch(&self) -> PaneEpoch {
        PaneEpoch(self.next_epoch.fetch_add(1, Ordering::Relaxed))
    }

    // The internal command remains owned by TrySendError on backpressure; its
    // size is the cost of guaranteeing lossless retry semantics.
    #[allow(clippy::result_large_err)]
    fn try_send(&self, command: ControllerCommand) -> Result<(), TrySendError<ControllerCommand>> {
        match self.commands.as_ref() {
            Some(commands) => commands.try_send(command),
            None => Err(TrySendError::Disconnected(command)),
        }
    }

    fn join_worker(&mut self) -> Result<(), TerminalError> {
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        worker.join().unwrap_or_else(|payload| {
            Err(TerminalError::new(format!(
                "terminal controller host panicked while joining: {}",
                panic_message(payload)
            )))
        })
    }

    #[cfg(test)]
    fn open_test_transport(
        &self,
        spec: PaneSpec,
        size: TerminalSize,
        transport: Box<dyn SessionTransport>,
    ) -> Result<PaneEpoch, TerminalError> {
        let epoch = self.allocate_epoch();
        self.try_send(ControllerCommand::OpenTest {
            epoch,
            spec,
            size,
            transport,
        })
        .map_err(|_| TerminalError::new("test controller command queue rejected open"))?;
        Ok(epoch)
    }
}

impl Drop for TerminalController {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        self.commands.take();
        // Dropping JoinHandle detaches. Cleanup continues on the backend host
        // and never stalls the UI thread.
        self.worker.take();
    }
}

enum ControllerCommand {
    OpenNative {
        epoch: PaneEpoch,
        request: NativePaneRequest,
    },
    #[cfg(test)]
    OpenTest {
        epoch: PaneEpoch,
        spec: PaneSpec,
        size: TerminalSize,
        transport: Box<dyn SessionTransport>,
    },
    Input {
        pane_id: PaneId,
        epoch: PaneEpoch,
        bytes: Vec<u8>,
    },
    Resize {
        pane_id: PaneId,
        epoch: PaneEpoch,
        size: TerminalSize,
    },
    Scroll {
        pane_id: PaneId,
        epoch: PaneEpoch,
        scroll: TerminalScroll,
    },
    SetLabel {
        pane_id: PaneId,
        epoch: PaneEpoch,
        label: String,
    },
    Close {
        pane_id: PaneId,
        epoch: PaneEpoch,
    },
    Shutdown,
}

fn next_revision(current: Option<&Arc<PublishedPane>>) -> u64 {
    current
        .map(|pane| pane.revision.saturating_add(1))
        .unwrap_or(1)
}

fn is_command_backpressure(error: &TerminalError) -> bool {
    error.to_string().contains("command queue is full")
}

fn read_unpoisoned<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write_unpoisoned<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

#[cfg(test)]
#[path = "controller_tests.rs"]
pub(crate) mod tests;
