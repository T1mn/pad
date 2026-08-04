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
    SessionTransport, TerminalError, TerminalSize, TransportExit,
};

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

enum PaneOperation {
    Open {
        epoch: PaneEpoch,
        spec: PaneSpec,
        size: TerminalSize,
        transport: Box<dyn SessionTransport>,
    },
    Input {
        epoch: PaneEpoch,
        bytes: Vec<u8>,
    },
    Resize {
        epoch: PaneEpoch,
        size: TerminalSize,
    },
    SetLabel {
        epoch: PaneEpoch,
        label: String,
    },
    Close {
        epoch: PaneEpoch,
    },
}

impl PaneOperation {
    fn epoch(&self) -> PaneEpoch {
        match self {
            Self::Open { epoch, .. }
            | Self::Input { epoch, .. }
            | Self::Resize { epoch, .. }
            | Self::SetLabel { epoch, .. }
            | Self::Close { epoch } => *epoch,
        }
    }
}

#[derive(Default)]
struct HostPane {
    active_epoch: Option<PaneEpoch>,
    pumpable: bool,
    pending: VecDeque<PaneOperation>,
}

struct ControllerHost {
    runtime: LivePaneRuntime,
    published: Arc<RwLock<HashMap<PaneId, Arc<PublishedPane>>>>,
    panes: HashMap<PaneId, HostPane>,
    round_robin: VecDeque<PaneId>,
    pending_count: usize,
    pending_capacity: usize,
}

impl ControllerHost {
    fn new(
        runtime: LivePaneRuntime,
        published: Arc<RwLock<HashMap<PaneId, Arc<PublishedPane>>>>,
        pending_capacity: usize,
    ) -> Self {
        Self {
            runtime,
            published,
            panes: HashMap::new(),
            round_robin: VecDeque::new(),
            pending_count: 0,
            pending_capacity,
        }
    }

    fn run(
        mut self,
        commands: Receiver<ControllerCommand>,
        stopping: Arc<AtomicBool>,
    ) -> Result<(), TerminalError> {
        while !stopping.load(Ordering::Acquire) {
            if self.pending_count < self.pending_capacity {
                match commands.recv_timeout(HOST_POLL_INTERVAL) {
                    Ok(ControllerCommand::Shutdown) => break,
                    Ok(command) => self.enqueue(command),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            } else {
                // Keep retry latency low without spinning a CPU when a
                // transport deliberately stops consuming commands.
                thread::sleep(HOST_POLL_INTERVAL);
            }
            self.tick();
        }
        self.close_all();
        Ok(())
    }

    fn enqueue(&mut self, command: ControllerCommand) {
        let (pane_id, operation) = match command {
            ControllerCommand::OpenNative { epoch, request } => {
                let pane_id = request.spec.id.clone();
                let transport = NativePtyTransport::new(
                    request.spec.transport_id.clone(),
                    request.command,
                    request.size,
                );
                (
                    pane_id,
                    PaneOperation::Open {
                        epoch,
                        spec: request.spec,
                        size: request.size,
                        transport: Box::new(transport),
                    },
                )
            }
            #[cfg(test)]
            ControllerCommand::OpenTest {
                epoch,
                spec,
                size,
                transport,
            } => (
                spec.id.clone(),
                PaneOperation::Open {
                    epoch,
                    spec,
                    size,
                    transport,
                },
            ),
            ControllerCommand::Input {
                pane_id,
                epoch,
                bytes,
            } => (pane_id, PaneOperation::Input { epoch, bytes }),
            ControllerCommand::Resize {
                pane_id,
                epoch,
                size,
            } => (pane_id, PaneOperation::Resize { epoch, size }),
            ControllerCommand::SetLabel {
                pane_id,
                epoch,
                label,
            } => (pane_id, PaneOperation::SetLabel { epoch, label }),
            ControllerCommand::Close { pane_id, epoch } => {
                (pane_id, PaneOperation::Close { epoch })
            }
            ControllerCommand::Shutdown => return,
        };

        let is_new = !self.panes.contains_key(&pane_id);
        self.panes
            .entry(pane_id.clone())
            .or_default()
            .pending
            .push_back(operation);
        self.pending_count += 1;
        if is_new {
            self.round_robin.push_back(pane_id);
        }
    }

    fn tick(&mut self) {
        let Some(pane_id) = self.round_robin.pop_front() else {
            return;
        };
        let operation = self
            .panes
            .get_mut(&pane_id)
            .and_then(|pane| pane.pending.pop_front());
        if operation.is_some() {
            self.pending_count -= 1;
        }

        if let Some(operation) = operation {
            if let Some(retry) = self.apply_operation(&pane_id, operation) {
                self.panes
                    .get_mut(&pane_id)
                    .expect("controller pane exists while retrying")
                    .pending
                    .push_front(retry);
                self.pending_count += 1;
            }
        }

        self.pump_pane(&pane_id);
        let retain = self
            .panes
            .get(&pane_id)
            .is_some_and(|pane| pane.active_epoch.is_some() || !pane.pending.is_empty());
        if retain {
            self.round_robin.push_back(pane_id);
        } else {
            self.panes.remove(&pane_id);
        }
    }

    /// Returns the operation only when downstream bounded backpressure needs
    /// a retry. Every other failure is published and consumed exactly once.
    fn apply_operation(
        &mut self,
        pane_id: &PaneId,
        operation: PaneOperation,
    ) -> Option<PaneOperation> {
        if matches!(operation, PaneOperation::Open { .. })
            && read_unpoisoned(&self.published)
                .get(pane_id)
                .is_some_and(|published| published.epoch > operation.epoch())
        {
            return None;
        }
        if !matches!(operation, PaneOperation::Open { .. })
            && self.panes.get(pane_id).and_then(|pane| pane.active_epoch) != Some(operation.epoch())
        {
            return None;
        }

        match operation {
            PaneOperation::Open {
                epoch,
                spec,
                size,
                transport,
            } => {
                if self
                    .panes
                    .get(pane_id)
                    .and_then(|pane| pane.active_epoch)
                    .is_some()
                {
                    let _ = self.runtime.close(pane_id);
                }
                self.publish_pending(pane_id, epoch);
                match self.runtime.open(spec, size, transport) {
                    Ok(()) => {
                        let pane = self
                            .panes
                            .get_mut(pane_id)
                            .expect("controller pane exists while opening");
                        pane.active_epoch = Some(epoch);
                        pane.pumpable = true;
                        self.publish_frame(pane_id, epoch, None, None, true);
                    }
                    Err(error) => {
                        let pane = self
                            .panes
                            .get_mut(pane_id)
                            .expect("controller pane exists after failed open");
                        pane.active_epoch = None;
                        pane.pumpable = false;
                        self.publish_error(pane_id, epoch, error, false);
                    }
                }
                None
            }
            PaneOperation::Input { epoch, bytes } => match self.runtime.input(pane_id, &bytes) {
                Ok(()) => None,
                Err(error) if is_command_backpressure(&error) => {
                    Some(PaneOperation::Input { epoch, bytes })
                }
                Err(error) => {
                    self.publish_error(pane_id, epoch, error, true);
                    None
                }
            },
            PaneOperation::Resize { epoch, size } => match self.runtime.resize(pane_id, size) {
                Ok(()) => None,
                Err(error) if is_command_backpressure(&error) => {
                    Some(PaneOperation::Resize { epoch, size })
                }
                Err(error) => {
                    self.publish_error(pane_id, epoch, error, true);
                    None
                }
            },
            PaneOperation::SetLabel { epoch, label } => {
                match self.runtime.set_label(pane_id, label) {
                    Ok(()) => self.publish_frame(pane_id, epoch, None, None, true),
                    Err(error) => self.publish_error(pane_id, epoch, error, true),
                }
                None
            }
            PaneOperation::Close { epoch } => {
                let result = self.runtime.close(pane_id);
                if let Some(pane) = self.panes.get_mut(pane_id) {
                    pane.active_epoch = None;
                    pane.pumpable = false;
                }
                match result {
                    Ok(()) => self.publish_frame(pane_id, epoch, None, None, false),
                    Err(error) => self.publish_error(pane_id, epoch, error, false),
                }
                None
            }
        }
    }

    fn pump_pane(&mut self, pane_id: &PaneId) {
        let Some(epoch) = self
            .panes
            .get(pane_id)
            .and_then(|pane| pane.pumpable.then_some(pane.active_epoch).flatten())
        else {
            return;
        };

        match self.runtime.pump(pane_id) {
            Ok(consumed) => {
                let _ = self.runtime.drain_host_events(pane_id);
                let exit = self.runtime.exit(pane_id);
                if consumed > 0 || exit.is_some() {
                    self.publish_frame(pane_id, epoch, None, exit, true);
                }
                if exit.is_some() {
                    if let Some(pane) = self.panes.get_mut(pane_id) {
                        pane.pumpable = false;
                    }
                }
            }
            Err(error) => {
                self.publish_error(pane_id, epoch, error, true);
                if let Some(pane) = self.panes.get_mut(pane_id) {
                    pane.pumpable = false;
                }
            }
        }
    }

    fn publish_pending(&self, pane_id: &PaneId, epoch: PaneEpoch) {
        let mut panes = write_unpoisoned(&self.published);
        if panes
            .get(pane_id)
            .is_some_and(|published| published.epoch > epoch)
        {
            return;
        }
        let revision = next_revision(panes.get(pane_id));
        panes.insert(
            pane_id.clone(),
            Arc::new(PublishedPane::pending(epoch, revision)),
        );
    }

    fn publish_frame(
        &self,
        pane_id: &PaneId,
        epoch: PaneEpoch,
        mut error: Option<Arc<str>>,
        exit: Option<TransportExit>,
        is_open: bool,
    ) {
        let frame = if is_open {
            match self.runtime.frame(pane_id) {
                Ok(frame) => Some(Arc::new(frame)),
                Err(snapshot_error) => {
                    error.get_or_insert_with(|| Arc::from(snapshot_error.to_string()));
                    None
                }
            }
        } else {
            None
        };
        self.publish(pane_id, epoch, frame, error, exit, is_open);
    }

    fn publish_error(
        &self,
        pane_id: &PaneId,
        epoch: PaneEpoch,
        error: TerminalError,
        is_open: bool,
    ) {
        let frame = if is_open {
            self.runtime.frame(pane_id).ok().map(Arc::new)
        } else {
            None
        };
        self.publish(
            pane_id,
            epoch,
            frame,
            Some(Arc::from(error.to_string())),
            None,
            is_open,
        );
    }

    fn publish(
        &self,
        pane_id: &PaneId,
        epoch: PaneEpoch,
        frame: Option<Arc<PaneFrame>>,
        error: Option<Arc<str>>,
        exit: Option<TransportExit>,
        is_open: bool,
    ) {
        let mut panes = write_unpoisoned(&self.published);
        if panes
            .get(pane_id)
            .is_some_and(|published| published.epoch > epoch)
        {
            return;
        }
        let current = panes
            .get(pane_id)
            .filter(|published| published.epoch == epoch);
        let error = error.or_else(|| current.and_then(|published| published.error.clone()));
        let exit = exit.or_else(|| current.and_then(|published| published.exit));
        let unchanged = current.is_some_and(|published| {
            published.epoch == epoch
                && published.frame == frame
                && published.error == error
                && published.exit == exit
                && published.is_open == is_open
        });
        if unchanged {
            return;
        }
        let revision = next_revision(panes.get(pane_id));
        panes.insert(
            pane_id.clone(),
            Arc::new(PublishedPane {
                epoch,
                revision,
                frame,
                error,
                exit,
                is_open,
            }),
        );
    }

    fn close_all(&mut self) {
        let open: Vec<_> = self
            .panes
            .iter()
            .filter_map(|(pane_id, pane)| pane.active_epoch.map(|epoch| (pane_id.clone(), epoch)))
            .collect();
        for (pane_id, epoch) in open {
            let result = self.runtime.close(&pane_id);
            match result {
                Ok(()) => self.publish_frame(&pane_id, epoch, None, None, false),
                Err(error) => self.publish_error(&pane_id, epoch, error, false),
            }
        }
    }
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
mod tests {
    use std::sync::mpsc::{self, Receiver, SyncSender};
    use std::time::{Duration, Instant};

    use super::*;
    use crate::terminal_runtime::{
        AlacrittyEngineFactory, EngineId, EngineRegistry, EngineRuntime, PaneRuntime, ReplayStep,
        ReplayTransport, SessionTransport, TransportCommand, TransportEvent, TransportId,
        TransportRuntime, ALACRITTY_ENGINE_ID,
    };

    const TIMEOUT: Duration = Duration::from_secs(2);

    #[test]
    fn replay_frames_and_exit_are_published_without_ui_runtime_calls() {
        let controller = controller(8, 8);
        assert_eq!(controller.host_thread_name(), "pad-terminal-controller");
        let reader = controller.frames();
        let pane_id = PaneId::new("published");
        let exit = successful_exit();
        let epoch = open_replay(
            &controller,
            &pane_id,
            "published-replay",
            [ReplayStep::output(b"ready"), ReplayStep::exit(exit)],
        );

        let published = wait_for(&reader, &pane_id, |pane| pane.exit == Some(exit));
        assert_eq!(published.epoch, epoch);
        assert_eq!(
            published
                .frame
                .as_ref()
                .unwrap()
                .terminal
                .row_text(0)
                .as_deref(),
            Some("ready")
        );
        assert!(published.revision >= 2);
        controller.shutdown().unwrap();
    }

    #[test]
    fn transport_failure_is_published_with_the_last_frame() {
        let controller = controller(8, 8);
        let reader = controller.frames();
        let pane_id = PaneId::new("failure");
        let epoch = open_replay(
            &controller,
            &pane_id,
            "failure-replay",
            [
                ReplayStep::output(b"prompt"),
                ReplayStep::expect_input(b"expected"),
            ],
        );
        wait_for(&reader, &pane_id, |pane| {
            pane.frame
                .as_ref()
                .is_some_and(|frame| frame.terminal.row_text(0).as_deref() == Some("prompt"))
        });
        controller
            .input(pane_id.clone(), epoch, b"wrong".to_vec())
            .unwrap();

        let failed = wait_for(&reader, &pane_id, |pane| pane.error.is_some());
        assert!(failed
            .error
            .as_deref()
            .unwrap()
            .contains("expected command"));
        assert_eq!(
            failed
                .frame
                .as_ref()
                .unwrap()
                .terminal
                .row_text(0)
                .as_deref(),
            Some("prompt")
        );
        controller.shutdown().unwrap();
    }

    #[test]
    fn downstream_input_and_resize_backpressure_is_retried_in_order() {
        let controller = controller(16, 1);
        let pane_id = PaneId::new("retry");
        let (release, release_rx) = mpsc::sync_channel(1);
        let (observed_tx, observed) = mpsc::sync_channel(1);
        let size = TerminalSize::new(100, 30);
        let transport_id = TransportId::new("retry-transport");
        let epoch = controller
            .open_test_transport(
                pane_spec(&pane_id, transport_id.as_str()),
                TerminalSize::new(80, 24),
                Box::new(GatedCommandTransport {
                    id: transport_id,
                    release: release_rx,
                    observed: observed_tx,
                }),
            )
            .unwrap();
        wait_for(&controller.frames(), &pane_id, |pane| pane.is_open);

        controller
            .input(pane_id.clone(), epoch, b"first".to_vec())
            .unwrap();
        controller.resize(pane_id.clone(), epoch, size).unwrap();
        release.send(()).unwrap();

        assert_eq!(
            observed.recv_timeout(TIMEOUT).unwrap(),
            vec![
                TransportCommand::Input(b"first".to_vec()),
                TransportCommand::Resize(size),
            ]
        );
        let published = wait_for(&controller.frames(), &pane_id, |pane| {
            pane.frame
                .as_ref()
                .is_some_and(|frame| frame.terminal.size == size)
        });
        assert_eq!(published.epoch, epoch);
        controller.shutdown().unwrap();
    }

    #[test]
    fn controller_queue_backpressure_returns_original_input() {
        let (release, gate) = mpsc::sync_channel(1);
        let controller = TerminalController::start_inner(runtime(8), 1, Some(gate)).unwrap();
        let pane_id = PaneId::new("bounded");
        let epoch = PaneEpoch(99);
        controller
            .input(pane_id.clone(), epoch, b"first".to_vec())
            .unwrap();
        let error = controller
            .input(pane_id, epoch, b"must-retry".to_vec())
            .unwrap_err();
        assert!(error.is_full());
        assert_eq!(error.into_inner(), b"must-retry".to_vec());

        release.send(()).unwrap();
        controller.shutdown().unwrap();
    }

    #[test]
    fn round_robin_keeps_a_quiet_pane_moving_during_noisy_output() {
        let controller = controller(32, 8);
        let reader = controller.frames();
        let noisy_id = PaneId::new("noisy");
        let quiet_id = PaneId::new("quiet");
        let noisy_steps = (0..(LivePaneRuntime::PUMP_EVENT_BUDGET * 4))
            .map(|_| ReplayStep::output(b"x"))
            .chain([ReplayStep::exit(successful_exit())]);
        open_replay(&controller, &noisy_id, "noisy-replay", noisy_steps);
        let quiet_epoch = open_replay(
            &controller,
            &quiet_id,
            "quiet-replay",
            [
                ReplayStep::output(b"quiet-ready"),
                ReplayStep::exit(successful_exit()),
            ],
        );

        let quiet = wait_for(&reader, &quiet_id, |pane| pane.exit.is_some());
        assert_eq!(quiet.epoch, quiet_epoch);
        assert_eq!(
            quiet
                .frame
                .as_ref()
                .unwrap()
                .terminal
                .row_text(0)
                .as_deref(),
            Some("quiet-ready")
        );
        controller.shutdown().unwrap();
    }

    #[test]
    fn stale_epoch_commands_cannot_mutate_a_reopened_pane() {
        let controller = controller(16, 8);
        let reader = controller.frames();
        let pane_id = PaneId::new("reopen");
        let old_epoch = open_replay(
            &controller,
            &pane_id,
            "old-replay",
            [
                ReplayStep::output(b"old"),
                ReplayStep::exit(successful_exit()),
            ],
        );
        wait_for(&reader, &pane_id, |pane| pane.exit.is_some());
        controller.close(pane_id.clone(), old_epoch).unwrap();

        let new_epoch = open_replay(
            &controller,
            &pane_id,
            "new-replay",
            [
                ReplayStep::output(b"new"),
                ReplayStep::exit(successful_exit()),
            ],
        );
        controller
            .set_label(pane_id.clone(), old_epoch, "stale".to_string())
            .unwrap();

        let published = wait_for(&reader, &pane_id, |pane| {
            pane.epoch == new_epoch && pane.exit.is_some()
        });
        assert_eq!(published.epoch, new_epoch);
        assert_eq!(published.frame.as_ref().unwrap().metadata.label, "Codex");
        assert_eq!(
            published
                .frame
                .as_ref()
                .unwrap()
                .terminal
                .row_text(0)
                .as_deref(),
            Some("new")
        );
        controller.shutdown().unwrap();
    }

    #[test]
    fn delayed_older_open_cannot_replace_a_newer_epoch() {
        let controller = controller(16, 8);
        let reader = controller.frames();
        let pane_id = PaneId::new("out-of-order-open");
        let old_epoch = controller.allocate_epoch();
        let new_epoch = controller.allocate_epoch();
        let new_transport_id = "newer-open";
        let old_transport_id = "older-open";

        assert!(controller
            .try_send(ControllerCommand::OpenTest {
                epoch: new_epoch,
                spec: pane_spec(&pane_id, new_transport_id),
                size: TerminalSize::new(80, 24),
                transport: Box::new(ReplayTransport::new(
                    TransportId::new(new_transport_id),
                    [
                        ReplayStep::output(b"newer"),
                        ReplayStep::exit(successful_exit()),
                    ],
                )),
            })
            .is_ok());
        assert!(controller
            .try_send(ControllerCommand::OpenTest {
                epoch: old_epoch,
                spec: pane_spec(&pane_id, old_transport_id),
                size: TerminalSize::new(80, 24),
                transport: Box::new(ReplayTransport::new(
                    TransportId::new(old_transport_id),
                    [
                        ReplayStep::output(b"older"),
                        ReplayStep::exit(successful_exit()),
                    ],
                )),
            })
            .is_ok());
        controller
            .set_label(pane_id.clone(), new_epoch, "new generation".to_string())
            .unwrap();

        let published = wait_for(&reader, &pane_id, |pane| {
            pane.epoch == new_epoch
                && pane
                    .frame
                    .as_ref()
                    .is_some_and(|frame| frame.metadata.label == "new generation")
        });
        assert_eq!(
            published
                .frame
                .as_ref()
                .unwrap()
                .terminal
                .row_text(0)
                .as_deref(),
            Some("newer")
        );
        controller.shutdown().unwrap();
    }

    #[test]
    fn label_and_close_publish_new_revisions() {
        let controller = controller(8, 8);
        let reader = controller.frames();
        let pane_id = PaneId::new("lifecycle");
        let epoch = open_replay(
            &controller,
            &pane_id,
            "lifecycle-replay",
            [ReplayStep::output(b"running")],
        );
        let opened = wait_for(&reader, &pane_id, |pane| pane.is_open);

        controller
            .set_label(pane_id.clone(), epoch, "Claude · review".to_string())
            .unwrap();
        let labeled = wait_for(&reader, &pane_id, |pane| {
            pane.frame
                .as_ref()
                .is_some_and(|frame| frame.metadata.label == "Claude · review")
        });
        assert!(labeled.revision > opened.revision);

        controller.close(pane_id.clone(), epoch).unwrap();
        let closed = wait_for(&reader, &pane_id, |pane| !pane.is_open);
        assert!(closed.frame.is_none());
        assert!(closed.revision > labeled.revision);
        controller.shutdown().unwrap();
    }

    #[test]
    fn drop_is_nonblocking_while_explicit_shutdown_joins() {
        let dropped_controller = controller(8, 8);
        let reader = dropped_controller.frames();
        let pane_id = PaneId::new("drop");
        open_replay(
            &dropped_controller,
            &pane_id,
            "drop-replay",
            [ReplayStep::output(b"active")],
        );
        wait_for(&reader, &pane_id, |pane| pane.is_open);
        let started = Instant::now();
        drop(dropped_controller);
        assert!(started.elapsed() < Duration::from_millis(50));

        let controller = controller(8, 8);
        controller.shutdown().unwrap();
    }

    fn controller(command_capacity: usize, transport_capacity: usize) -> TerminalController {
        TerminalController::start_with_capacity(runtime(transport_capacity), command_capacity)
            .unwrap()
    }

    fn runtime(transport_capacity: usize) -> LivePaneRuntime {
        let mut registry = EngineRegistry::default();
        registry.register(EngineId::new(ALACRITTY_ENGINE_ID), AlacrittyEngineFactory);
        let engines = EngineRuntime::start(2, registry).unwrap();
        LivePaneRuntime::new(
            PaneRuntime::new(engines),
            TransportRuntime::new(transport_capacity, 8).unwrap(),
        )
    }

    fn open_replay(
        controller: &TerminalController,
        pane_id: &PaneId,
        transport_id: &str,
        steps: impl IntoIterator<Item = ReplayStep>,
    ) -> PaneEpoch {
        controller
            .open_test_transport(
                pane_spec(pane_id, transport_id),
                TerminalSize::new(80, 24),
                Box::new(ReplayTransport::new(TransportId::new(transport_id), steps)),
            )
            .unwrap()
    }

    fn pane_spec(pane_id: &PaneId, transport_id: &str) -> PaneSpec {
        PaneSpec {
            id: pane_id.clone(),
            label: "Codex".to_string(),
            engine_id: EngineId::new(ALACRITTY_ENGINE_ID),
            transport_id: TransportId::new(transport_id),
        }
    }

    fn successful_exit() -> TransportExit {
        TransportExit {
            code: Some(0),
            signaled: false,
        }
    }

    fn wait_for(
        reader: &TerminalFrameReader,
        pane_id: &PaneId,
        condition: impl Fn(&PublishedPane) -> bool,
    ) -> Arc<PublishedPane> {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if let Some(pane) = reader.latest(pane_id) {
                if condition(&pane) {
                    return pane;
                }
            }
            assert!(Instant::now() < deadline, "controller state timed out");
            thread::yield_now();
        }
    }

    struct GatedCommandTransport {
        id: TransportId,
        release: Receiver<()>,
        observed: SyncSender<Vec<TransportCommand>>,
    }

    impl SessionTransport for GatedCommandTransport {
        fn id(&self) -> &TransportId {
            &self.id
        }

        fn run(
            self: Box<Self>,
            commands: Receiver<TransportCommand>,
            events: SyncSender<TransportEvent>,
        ) -> Result<(), TerminalError> {
            self.release
                .recv()
                .map_err(|_| TerminalError::new("release disconnected"))?;
            let input = commands
                .recv()
                .map_err(|_| TerminalError::new("input disconnected"))?;
            let resize = commands
                .recv()
                .map_err(|_| TerminalError::new("resize disconnected"))?;
            let size = match resize {
                TransportCommand::Resize(size) => size,
                other => {
                    return Err(TerminalError::new(format!(
                        "expected resize, received {other:?}"
                    )))
                }
            };
            self.observed
                .send(vec![input, TransportCommand::Resize(size)])
                .map_err(|_| TerminalError::new("observer disconnected"))?;
            events
                .send(TransportEvent::ResizeApplied(size))
                .map_err(|_| TerminalError::new("event receiver disconnected"))?;
            events
                .send(TransportEvent::Exited(successful_exit()))
                .map_err(|_| TerminalError::new("event receiver disconnected"))?;
            Ok(())
        }
    }
}
