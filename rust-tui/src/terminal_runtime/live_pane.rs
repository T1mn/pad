mod events;

use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{TryRecvError, TrySendError};

use super::{
    PaneFrame, PaneId, PaneMetadata, PaneRuntime, PaneSpec, SessionTransport, TerminalEngineEvent,
    TerminalError, TerminalScroll, TerminalSize, TransportCommand, TransportEvent, TransportExit,
    TransportHandle, TransportRuntime,
};

struct LiveTransport {
    handle: TransportHandle,
    exit: Option<TransportExit>,
    pending_resizes: VecDeque<TerminalSize>,
    pending_pty_writes: VecDeque<Vec<u8>>,
    pending_pty_write_bytes: usize,
    host_events: VecDeque<TerminalEngineEvent>,
    failure: Option<TerminalError>,
}

/// Owns live transports and routes their events into the pane engine runtime.
///
/// This is a backend coordinator: engine calls wait for their worker replies,
/// so hosts must run `pump`/`frame` outside the UI thread and publish immutable
/// frames back to the renderer.
pub struct LivePaneRuntime {
    // Keep handles ahead of the engine runtime so normal field destruction
    // disconnects transports before waiting for engine workers to finish.
    transports: HashMap<PaneId, LiveTransport>,
    panes: PaneRuntime,
    transport_runtime: TransportRuntime,
}

impl LivePaneRuntime {
    /// Maximum number of transport events consumed for one pane by one
    /// [`pump`](Self::pump) call. Bounding the work keeps a continuously
    /// readable transport from monopolizing the UI thread.
    pub const PUMP_EVENT_BUDGET: usize = 64;
    pub const MAX_PENDING_PTY_WRITE_BYTES: usize = 1024 * 1024;
    pub const MAX_PENDING_HOST_EVENTS: usize = 128;

    pub fn new(panes: PaneRuntime, transport_runtime: TransportRuntime) -> Self {
        Self {
            transports: HashMap::new(),
            panes,
            transport_runtime,
        }
    }

    pub fn open(
        &mut self,
        spec: PaneSpec,
        size: TerminalSize,
        transport: Box<dyn SessionTransport>,
    ) -> Result<(), TerminalError> {
        if spec.transport_id != *transport.id() {
            return Err(TerminalError::new(format!(
                "terminal pane '{}' expects transport '{}', received '{}'",
                spec.id,
                spec.transport_id.as_str(),
                transport.id().as_str()
            )));
        }
        if self.transports.contains_key(&spec.id) || self.panes.metadata(&spec.id).is_some() {
            return Err(pane_already_registered(&spec.id));
        }

        // Spawn first: if engine creation fails, dropping the unattached
        // handle shuts down and detaches it without adding any pane state.
        let handle = self.transport_runtime.spawn(transport)?;
        let pane_id = spec.id.clone();
        self.panes.open(spec, size)?;
        self.transports.insert(
            pane_id,
            LiveTransport {
                handle,
                exit: None,
                pending_resizes: VecDeque::new(),
                pending_pty_writes: VecDeque::new(),
                pending_pty_write_bytes: 0,
                host_events: VecDeque::new(),
                failure: None,
            },
        );
        Ok(())
    }

    /// Applies up to [`PUMP_EVENT_BUDGET`](Self::PUMP_EVENT_BUDGET) buffered
    /// transport events for one pane without waiting for another event.
    ///
    /// Consecutive output chunks are coalesced into one engine feed while
    /// resize acknowledgements and exit events remain ordering barriers. The
    /// return value counts transport events, not engine operations.
    pub fn pump(&mut self, pane_id: &PaneId) -> Result<usize, TerminalError> {
        if let Some(error) = self.ensure_transport(pane_id)?.failure.clone() {
            return Err(error);
        }
        let _ = self.flush_pending_pty_writes(pane_id)?;
        let mut consumed = 0;
        let mut output = Vec::new();
        let mut event_channel_disconnected = false;

        while consumed < Self::PUMP_EVENT_BUDGET {
            let event = {
                let live = self
                    .transports
                    .get(pane_id)
                    .expect("live transport was checked above");
                live.handle.try_recv()
            };

            match event {
                Ok(TransportEvent::Output(mut bytes)) => {
                    output.append(&mut bytes);
                    consumed += 1;
                }
                Ok(TransportEvent::ResizeApplied(size)) => {
                    self.flush_output(pane_id, &mut output)?;
                    consumed += 1;
                    self.apply_resize(pane_id, size)?;
                    let _ = self.flush_pending_pty_writes(pane_id)?;
                }
                Ok(TransportEvent::Exited(exit)) => {
                    self.flush_output(pane_id, &mut output)?;
                    self.transports
                        .get_mut(pane_id)
                        .expect("live transport was checked above")
                        .exit = Some(exit);
                    consumed += 1;
                    let _ = self.flush_pending_pty_writes(pane_id)?;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    event_channel_disconnected = true;
                    break;
                }
            }
        }
        self.flush_output(pane_id, &mut output)?;
        let _ = self.flush_pending_pty_writes(pane_id)?;

        // Do not surface completion ahead of events that remain buffered after
        // exhausting the budget or a transient Empty observation. The event
        // sender disconnects only after every preceding event is drainable.
        if event_channel_disconnected {
            if let Some(completion) = self
                .transports
                .get_mut(pane_id)
                .expect("live transport was checked above")
                .handle
                .try_completion()
            {
                completion?;
                let live = self
                    .transports
                    .get_mut(pane_id)
                    .expect("live transport was checked above");
                live.exit.get_or_insert(TransportExit {
                    code: None,
                    signaled: false,
                });
            }
        }
        Ok(consumed)
    }

    pub fn input(&self, pane_id: &PaneId, bytes: &[u8]) -> Result<(), TerminalError> {
        self.send_command(pane_id, TransportCommand::Input(bytes.to_vec()))
    }

    pub fn resize(&mut self, pane_id: &PaneId, size: TerminalSize) -> Result<(), TerminalError> {
        self.send_command(pane_id, TransportCommand::Resize(size))?;
        self.transports
            .get_mut(pane_id)
            .expect("live transport was checked while sending resize")
            .pending_resizes
            .push_back(size);
        Ok(())
    }

    /// Moves the parser-owned viewport without involving the PTY transport.
    /// The engine runtime executes this on the pane's worker shard.
    pub fn scroll(&self, pane_id: &PaneId, scroll: TerminalScroll) -> Result<(), TerminalError> {
        self.ensure_transport(pane_id)?;
        self.panes.scroll(pane_id, scroll)
    }

    pub fn frame(&self, pane_id: &PaneId) -> Result<PaneFrame, TerminalError> {
        self.ensure_transport(pane_id)?;
        self.panes.frame(pane_id)
    }

    pub fn metadata(&self, pane_id: &PaneId) -> Option<&PaneMetadata> {
        self.panes.metadata(pane_id)
    }

    pub fn set_label(
        &mut self,
        pane_id: &PaneId,
        label: impl Into<String>,
    ) -> Result<(), TerminalError> {
        self.ensure_transport(pane_id)?;
        self.panes.set_label(pane_id, label)
    }

    pub fn exit(&self, pane_id: &PaneId) -> Option<TransportExit> {
        self.transports.get(pane_id).and_then(|live| live.exit)
    }

    /// Drains title, bell, exit, and explicit unsupported-request events.
    /// Parser-generated PTY replies are routed internally and never exposed
    /// here, so callers cannot accidentally drop protocol bytes.
    pub fn drain_host_events(
        &mut self,
        pane_id: &PaneId,
    ) -> Result<Vec<TerminalEngineEvent>, TerminalError> {
        let live = self
            .transports
            .get_mut(pane_id)
            .ok_or_else(|| pane_not_registered(pane_id))?;
        Ok(live.host_events.drain(..).collect())
    }

    pub fn close(&mut self, pane_id: &PaneId) -> Result<(), TerminalError> {
        self.ensure_transport(pane_id)?;

        // Always tear down the transport record, even if a contained engine
        // panic makes close return an error. The worker has already removed
        // the faulty engine and retaining metadata would make the ID
        // permanently impossible to reopen.
        let pane_result = self.panes.close(pane_id);
        let mut live = self
            .transports
            .remove(pane_id)
            .expect("live transport was checked above");
        let _ = live.handle.shutdown();
        pane_result
    }

    fn send_command(
        &self,
        pane_id: &PaneId,
        command: TransportCommand,
    ) -> Result<(), TerminalError> {
        let live = self.ensure_transport(pane_id)?;
        if let Some(error) = live.failure.clone() {
            return Err(error);
        }
        match live.handle.try_send(command) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(TerminalError::new(format!(
                "terminal transport '{}' command queue is full",
                live.handle.id().as_str()
            ))),
            Err(TrySendError::Disconnected(_)) => Err(TerminalError::new(format!(
                "terminal transport '{}' command channel disconnected",
                live.handle.id().as_str()
            ))),
        }
    }

    fn apply_resize(
        &mut self,
        pane_id: &PaneId,
        applied: TerminalSize,
    ) -> Result<(), TerminalError> {
        let expected = self
            .transports
            .get(pane_id)
            .expect("live transport was checked above")
            .pending_resizes
            .front()
            .copied();
        if expected != Some(applied) {
            let detail = match expected {
                Some(expected) => format!(
                    "out-of-order resize acknowledgement {}x{}; expected {}x{}",
                    applied.columns, applied.rows, expected.columns, expected.rows
                ),
                None => format!(
                    "unexpected resize acknowledgement {}x{} with no pending resize",
                    applied.columns, applied.rows
                ),
            };
            let error = TerminalError::new(format!(
                "terminal transport '{}' {detail}",
                self.transports
                    .get(pane_id)
                    .expect("live transport was checked above")
                    .handle
                    .id()
                    .as_str()
            ));
            self.remember_failure(pane_id, error.clone());
            return Err(error);
        }

        if let Err(error) = self.panes.resize(pane_id, applied) {
            self.remember_failure(pane_id, error.clone());
            return Err(error);
        }
        self.collect_engine_events(pane_id)?;
        self.transports
            .get_mut(pane_id)
            .expect("live transport was checked above")
            .pending_resizes
            .pop_front();
        Ok(())
    }

    fn remember_failure(&mut self, pane_id: &PaneId, error: TerminalError) {
        self.transports
            .get_mut(pane_id)
            .expect("live transport was checked above")
            .failure
            .get_or_insert(error);
    }

    fn ensure_transport(&self, pane_id: &PaneId) -> Result<&LiveTransport, TerminalError> {
        self.transports
            .get(pane_id)
            .ok_or_else(|| pane_not_registered(pane_id))
    }
}

fn pane_already_registered(pane_id: &PaneId) -> TerminalError {
    TerminalError::new(format!("terminal pane '{pane_id}' is already registered"))
}

fn pane_not_registered(pane_id: &PaneId) -> TerminalError {
    TerminalError::new(format!("terminal pane '{pane_id}' is not registered"))
}

#[cfg(test)]
#[path = "live_pane_tests.rs"]
pub(crate) mod tests;
