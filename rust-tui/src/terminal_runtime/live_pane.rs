use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{TryRecvError, TrySendError};

use super::{
    PaneFrame, PaneId, PaneMetadata, PaneRuntime, PaneSpec, SessionTransport, TerminalEngineEvent,
    TerminalError, TerminalSize, TransportCommand, TransportEvent, TransportExit, TransportHandle,
    TransportRuntime,
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

    fn flush_output(
        &mut self,
        pane_id: &PaneId,
        output: &mut Vec<u8>,
    ) -> Result<(), TerminalError> {
        if output.is_empty() {
            return Ok(());
        }
        if let Err(error) = self.panes.feed_output(pane_id, std::mem::take(output)) {
            self.remember_failure(pane_id, error.clone());
            return Err(error);
        }
        self.collect_engine_events(pane_id)?;
        Ok(())
    }

    fn collect_engine_events(&mut self, pane_id: &PaneId) -> Result<(), TerminalError> {
        let events = match self.panes.drain_engine_events(pane_id) {
            Ok(events) => events,
            Err(error) => {
                self.remember_failure(pane_id, error.clone());
                return Err(error);
            }
        };
        let live = self
            .transports
            .get_mut(pane_id)
            .expect("live transport was checked above");
        for event in events {
            match event {
                TerminalEngineEvent::PtyWrite(bytes) => {
                    if live.pending_pty_write_bytes.saturating_add(bytes.len())
                        > Self::MAX_PENDING_PTY_WRITE_BYTES
                    {
                        let error = TerminalError::new(format!(
                            "terminal pane '{pane_id}' parser reply backlog exceeded {} bytes",
                            Self::MAX_PENDING_PTY_WRITE_BYTES
                        ));
                        live.failure.get_or_insert(error.clone());
                        return Err(error);
                    }
                    live.pending_pty_write_bytes += bytes.len();
                    live.pending_pty_writes.push_back(bytes);
                }
                event => Self::queue_host_event(live, event),
            }
        }
        Ok(())
    }

    fn queue_host_event(live: &mut LiveTransport, event: TerminalEngineEvent) {
        match event {
            TerminalEngineEvent::Title(title) => {
                if let Some(index) = live
                    .host_events
                    .iter()
                    .position(|event| matches!(event, TerminalEngineEvent::Title(_)))
                {
                    live.host_events[index] = TerminalEngineEvent::Title(title);
                } else {
                    live.host_events
                        .push_back(TerminalEngineEvent::Title(title));
                }
            }
            TerminalEngineEvent::Bell => {
                if !live
                    .host_events
                    .iter()
                    .any(|event| matches!(event, TerminalEngineEvent::Bell))
                {
                    live.host_events.push_back(TerminalEngineEvent::Bell);
                }
            }
            TerminalEngineEvent::Exit => {
                if !live
                    .host_events
                    .iter()
                    .any(|event| matches!(event, TerminalEngineEvent::Exit))
                {
                    live.host_events.push_back(TerminalEngineEvent::Exit);
                }
            }
            TerminalEngineEvent::UnsupportedRequest(request) => {
                if live.host_events.iter().any(|event| {
                    matches!(event, TerminalEngineEvent::UnsupportedRequest(existing) if existing == &request)
                }) {
                    return;
                }
                if live.host_events.len() < Self::MAX_PENDING_HOST_EVENTS {
                    live.host_events
                        .push_back(TerminalEngineEvent::UnsupportedRequest(request));
                } else if let Some(index) = live
                    .host_events
                    .iter()
                    .position(|event| matches!(event, TerminalEngineEvent::UnsupportedRequest(_)))
                {
                    live.host_events[index] = TerminalEngineEvent::UnsupportedRequest(
                        "additional terminal requests were coalesced".to_string(),
                    );
                }
            }
            TerminalEngineEvent::PtyWrite(_) => {
                unreachable!("PTY writes are queued before host-event coalescing")
            }
        }
    }

    /// Returns false when bounded command backpressure requires a later pump.
    fn flush_pending_pty_writes(&mut self, pane_id: &PaneId) -> Result<bool, TerminalError> {
        loop {
            let Some(bytes) = self
                .transports
                .get_mut(pane_id)
                .expect("live transport was checked above")
                .pending_pty_writes
                .pop_front()
            else {
                return Ok(true);
            };
            self.transports
                .get_mut(pane_id)
                .expect("live transport was checked above")
                .pending_pty_write_bytes -= bytes.len();
            let result = self
                .transports
                .get(pane_id)
                .expect("live transport was checked above")
                .handle
                .try_send(TransportCommand::Input(bytes));
            match result {
                Ok(()) => {}
                Err(TrySendError::Full(TransportCommand::Input(bytes))) => {
                    let live = self
                        .transports
                        .get_mut(pane_id)
                        .expect("live transport was checked above");
                    live.pending_pty_write_bytes += bytes.len();
                    live.pending_pty_writes.push_front(bytes);
                    return Ok(false);
                }
                Err(TrySendError::Disconnected(TransportCommand::Input(bytes))) => {
                    let live = self
                        .transports
                        .get_mut(pane_id)
                        .expect("live transport was checked above");
                    live.pending_pty_write_bytes += bytes.len();
                    live.pending_pty_writes.push_front(bytes);
                    let error = TerminalError::new(format!(
                        "terminal transport '{}' command channel disconnected while routing parser reply",
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
                Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                    unreachable!("parser replies are always transport input commands")
                }
            }
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
mod tests {
    use std::sync::mpsc::{self, Receiver, SyncSender};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use super::*;
    use crate::terminal_runtime::{
        AlacrittyEngineFactory, EngineFactory, EngineId, EngineRegistry, EngineRuntime, ReplayStep,
        ReplayTransport, TerminalEngine, TerminalSnapshot, TransportId, ALACRITTY_ENGINE_ID,
    };

    const TEST_TIMEOUT: Duration = Duration::from_secs(2);

    #[test]
    fn replay_output_is_pumped_into_the_terminal_snapshot_in_order() {
        let mut runtime = runtime();
        let pane_id = PaneId::new("output");
        let exit = successful_exit();
        open_replay(
            &mut runtime,
            &pane_id,
            "replay-output",
            [
                ReplayStep::output(b"hel"),
                ReplayStep::output(b"lo"),
                ReplayStep::exit(exit),
            ],
        )
        .unwrap();

        pump_until_exit(&mut runtime, &pane_id);

        let frame = runtime.frame(&pane_id).unwrap();
        assert_eq!(frame.terminal.row_text(0).as_deref(), Some("hello"));
        assert_eq!(runtime.exit(&pane_id), Some(exit));
    }

    #[test]
    fn parser_replies_are_routed_back_to_transport_in_order() {
        let mut runtime = runtime();
        let pane_id = PaneId::new("query-reply");
        open_replay(
            &mut runtime,
            &pane_id,
            "replay-query-reply",
            [
                ReplayStep::output(b"\x1b[6n"),
                ReplayStep::expect_input(b"\x1b[1;1R"),
                ReplayStep::exit(successful_exit()),
            ],
        )
        .unwrap();

        pump_until_exit(&mut runtime, &pane_id);

        assert!(runtime.drain_host_events(&pane_id).unwrap().is_empty());
    }

    #[test]
    fn parser_reply_survives_a_full_command_queue() {
        let (release, release_rx) = mpsc::sync_channel(1);
        let (observed_tx, observed) = mpsc::sync_channel(1);
        let transport_id = TransportId::new("reply-backpressure");
        let mut runtime = runtime_with_capacities(1, 8);
        let pane_id = PaneId::new("reply-backpressure");
        runtime
            .open(
                pane_spec(&pane_id, transport_id.as_str()),
                TerminalSize::new(20, 4),
                Box::new(ReplyBackpressureTransport {
                    id: transport_id,
                    release: release_rx,
                    observed: observed_tx,
                }),
            )
            .unwrap();

        runtime.input(&pane_id, b"user-input").unwrap();
        let deadline = Instant::now() + TEST_TIMEOUT;
        loop {
            if runtime.pump(&pane_id).unwrap() == 1 {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "query event was not produced while the command queue stayed full"
            );
            std::thread::yield_now();
        }
        release.send(()).unwrap();

        let deadline = Instant::now() + TEST_TIMEOUT;
        let commands = loop {
            runtime.pump(&pane_id).unwrap();
            if let Ok(commands) = observed.try_recv() {
                break commands;
            }
            assert!(Instant::now() < deadline, "parser reply was not retried");
            std::thread::yield_now();
        };
        assert_eq!(
            commands,
            vec![
                TransportCommand::Input(b"user-input".to_vec()),
                TransportCommand::Input(b"\x1b[1;1R".to_vec()),
            ]
        );
        pump_until_exit(&mut runtime, &pane_id);
    }

    #[test]
    fn host_title_and_bell_events_are_observable() {
        let mut runtime = runtime();
        let pane_id = PaneId::new("host-events");
        open_replay(
            &mut runtime,
            &pane_id,
            "replay-host-events",
            [
                ReplayStep::output(b"\x1b]0;build\x07\x07"),
                ReplayStep::exit(successful_exit()),
            ],
        )
        .unwrap();

        pump_until_exit(&mut runtime, &pane_id);

        assert_eq!(
            runtime.drain_host_events(&pane_id).unwrap(),
            vec![
                TerminalEngineEvent::Title(Some("build".to_string())),
                TerminalEngineEvent::Bell,
            ]
        );
    }

    #[test]
    fn repeated_title_and_bell_events_are_coalesced() {
        let mut runtime = runtime();
        let pane_id = PaneId::new("coalesced-host-events");
        let mut output = Vec::new();
        for index in 0..200 {
            output.extend_from_slice(format!("\x1b]0;title-{index}\x07\x07").as_bytes());
        }
        open_replay(
            &mut runtime,
            &pane_id,
            "replay-coalesced-host-events",
            [
                ReplayStep::output(output),
                ReplayStep::exit(successful_exit()),
            ],
        )
        .unwrap();

        pump_until_exit(&mut runtime, &pane_id);

        assert_eq!(
            runtime.drain_host_events(&pane_id).unwrap(),
            vec![
                TerminalEngineEvent::Title(Some("title-199".to_string())),
                TerminalEngineEvent::Bell,
            ]
        );
    }

    #[test]
    fn final_output_is_applied_before_transport_failure_surfaces() {
        let (release, release_rx) = mpsc::sync_channel(1);
        let mut runtime = runtime();
        let pane_id = PaneId::new("final-output-error");
        runtime
            .open(
                pane_spec(&pane_id, "delayed-failure"),
                TerminalSize::new(20, 4),
                Box::new(DelayedFailureTransport {
                    id: TransportId::new("delayed-failure"),
                    release: release_rx,
                }),
            )
            .unwrap();

        assert_eq!(runtime.pump(&pane_id).unwrap(), 0);
        release.send(()).unwrap();
        let error = pump_until_error(&mut runtime, &pane_id);

        assert_eq!(
            runtime
                .frame(&pane_id)
                .unwrap()
                .terminal
                .row_text(0)
                .as_deref(),
            Some("final-output")
        );
        assert!(error.to_string().contains("injected transport failure"));
    }

    #[test]
    fn drain_panic_becomes_a_stable_failure_and_rejects_new_commands() {
        let mut registry = EngineRegistry::default();
        registry.register(EngineId::new("drain-panic"), DrainPanicFactory);
        let engines = EngineRuntime::start(1, registry).unwrap();
        let mut runtime = LivePaneRuntime::new(
            PaneRuntime::new(engines),
            TransportRuntime::new(8, 8).unwrap(),
        );
        let pane_id = PaneId::new("drain-panic");
        let transport_id = TransportId::new("drain-panic-replay");
        runtime
            .open(
                PaneSpec {
                    id: pane_id.clone(),
                    label: "faulty parser".to_string(),
                    engine_id: EngineId::new("drain-panic"),
                    transport_id: transport_id.clone(),
                },
                TerminalSize::new(20, 4),
                Box::new(ReplayTransport::new(
                    transport_id,
                    [ReplayStep::output(b"trigger")],
                )),
            )
            .unwrap();

        let error = pump_until_error(&mut runtime, &pane_id);
        assert!(error.to_string().contains("drain events"));
        assert!(error.to_string().contains("drain exploded"));
        assert_eq!(runtime.pump(&pane_id), Err(error.clone()));
        assert_eq!(runtime.input(&pane_id, b"ignored"), Err(error.clone()));
        assert_eq!(
            runtime.resize(&pane_id, TerminalSize::new(30, 6)),
            Err(error)
        );

        // Host metadata may still be updated for an error placeholder, but
        // closing must remove every pane/transport record even though the
        // failed engine was already evicted by its worker.
        runtime.set_label(&pane_id, "failed parser").unwrap();
        assert!(runtime.close(&pane_id).is_err());
        assert!(runtime.metadata(&pane_id).is_none());
        assert!(runtime.pump(&pane_id).is_err());
    }

    #[test]
    fn input_and_resize_are_forwarded_while_the_engine_is_resized() {
        let mut runtime = runtime();
        let pane_id = PaneId::new("interactive");
        let resized = TerminalSize::new(31, 7);
        open_replay(
            &mut runtime,
            &pane_id,
            "replay-interactive",
            [
                ReplayStep::expect_input(b"status\r"),
                ReplayStep::expect_resize(resized),
                ReplayStep::resize_applied(resized),
                ReplayStep::output(b"resized"),
                ReplayStep::exit(successful_exit()),
            ],
        )
        .unwrap();

        runtime.input(&pane_id, b"status\r").unwrap();
        runtime.resize(&pane_id, resized).unwrap();
        pump_until_exit(&mut runtime, &pane_id);

        let snapshot = runtime.frame(&pane_id).unwrap().terminal;
        assert_eq!(snapshot.size, resized);
        assert_eq!(snapshot.row_text(0).as_deref(), Some("resized"));
    }

    #[test]
    fn pump_has_a_fixed_event_budget_and_coalesces_output() {
        let event_count = LivePaneRuntime::PUMP_EVENT_BUDGET + 3;
        let (mut runtime, operations) = recording_runtime(1, event_count + 1);
        let pane_id = PaneId::new("budget");
        let mut steps = vec![ReplayStep::output(b"x"); event_count];
        steps.push(ReplayStep::exit(successful_exit()));
        open_recording_replay(&mut runtime, &pane_id, "replay-budget", steps).unwrap();
        wait_for_transport_completion(&mut runtime, &pane_id);

        assert_eq!(
            runtime.pump(&pane_id).unwrap(),
            LivePaneRuntime::PUMP_EVENT_BUDGET
        );
        assert_eq!(
            operations.lock().unwrap().as_slice(),
            [format!(
                "feed:{}",
                "x".repeat(LivePaneRuntime::PUMP_EVENT_BUDGET)
            )]
        );
        assert_eq!(runtime.exit(&pane_id), None);

        assert_eq!(runtime.pump(&pane_id).unwrap(), 4);
        assert_eq!(
            operations.lock().unwrap().as_slice(),
            [
                format!("feed:{}", "x".repeat(LivePaneRuntime::PUMP_EVENT_BUDGET)),
                "feed:xxx".to_string(),
            ]
        );
        assert_eq!(runtime.exit(&pane_id), Some(successful_exit()));
    }

    #[test]
    fn resize_ack_orders_old_and_new_output_around_engine_resize() {
        let (mut runtime, operations) = recording_runtime(8, 8);
        let pane_id = PaneId::new("resize-order");
        let initial = TerminalSize::new(20, 4);
        let resized = TerminalSize::new(31, 7);
        open_recording_replay(
            &mut runtime,
            &pane_id,
            "replay-resize-order",
            [
                ReplayStep::output(b"old-1"),
                ReplayStep::output(b"old-2"),
                ReplayStep::expect_resize(resized),
                ReplayStep::resize_applied(resized),
                ReplayStep::output(b"new-1"),
                ReplayStep::output(b"new-2"),
                ReplayStep::exit(successful_exit()),
            ],
        )
        .unwrap();

        runtime.resize(&pane_id, resized).unwrap();
        assert_eq!(runtime.frame(&pane_id).unwrap().terminal.size, initial);
        pump_until_exit(&mut runtime, &pane_id);

        assert_eq!(
            operations.lock().unwrap().as_slice(),
            ["feed:old-1old-2", "resize:31x7", "feed:new-1new-2"]
        );
        assert_eq!(runtime.frame(&pane_id).unwrap().terminal.size, resized);
    }

    #[test]
    fn duplicate_and_out_of_order_resize_acks_fail_deterministically() {
        let first = TerminalSize::new(30, 6);
        let second = TerminalSize::new(40, 8);
        let (mut runtime, _) = recording_runtime(8, 8);
        let pane_id = PaneId::new("out-of-order");
        open_recording_replay(
            &mut runtime,
            &pane_id,
            "replay-out-of-order",
            [
                ReplayStep::expect_resize(first),
                ReplayStep::expect_resize(second),
                ReplayStep::resize_applied(second),
            ],
        )
        .unwrap();
        runtime.resize(&pane_id, first).unwrap();
        runtime.resize(&pane_id, second).unwrap();
        let error = pump_until_error(&mut runtime, &pane_id);
        assert!(error.to_string().contains("out-of-order"));
        assert_eq!(runtime.pump(&pane_id), Err(error));
        assert_eq!(
            runtime.frame(&pane_id).unwrap().terminal.size,
            TerminalSize::new(20, 4)
        );

        let (mut runtime, _) = recording_runtime(8, 8);
        let pane_id = PaneId::new("duplicate");
        open_recording_replay(
            &mut runtime,
            &pane_id,
            "replay-duplicate",
            [
                ReplayStep::expect_resize(first),
                ReplayStep::resize_applied(first),
                ReplayStep::resize_applied(first),
            ],
        )
        .unwrap();
        runtime.resize(&pane_id, first).unwrap();
        let error = pump_until_error(&mut runtime, &pane_id);
        assert!(error.to_string().contains("no pending resize"));
        assert_eq!(runtime.pump(&pane_id), Err(error));
        assert_eq!(runtime.frame(&pane_id).unwrap().terminal.size, first);
    }

    #[test]
    fn saturated_and_disconnected_command_queues_return_without_blocking() {
        let mut runtime = runtime_with_capacities(1, 1);
        let pane_id = PaneId::new("saturated");
        open_replay(
            &mut runtime,
            &pane_id,
            "replay-saturated",
            [
                ReplayStep::output(b"blocks-next-event"),
                ReplayStep::output(b"blocks-command-consumer"),
                ReplayStep::expect_input(b"queued"),
            ],
        )
        .unwrap();
        runtime.input(&pane_id, b"queued").unwrap();

        assert_eq!(
            runtime.input(&pane_id, b"full").unwrap_err().to_string(),
            "terminal transport 'replay-saturated' command queue is full"
        );
        assert_eq!(
            runtime
                .resize(&pane_id, TerminalSize::new(30, 6))
                .unwrap_err()
                .to_string(),
            "terminal transport 'replay-saturated' command queue is full"
        );
        assert_eq!(
            runtime.frame(&pane_id).unwrap().terminal.size,
            TerminalSize::new(20, 4)
        );
        runtime.close(&pane_id).unwrap();

        let mut runtime = runtime_with_capacities(1, 1);
        let pane_id = PaneId::new("disconnected");
        open_replay(&mut runtime, &pane_id, "replay-disconnected", []).unwrap();
        wait_for_transport_completion(&mut runtime, &pane_id);
        assert_eq!(
            runtime.input(&pane_id, &[]).unwrap_err().to_string(),
            "terminal transport 'replay-disconnected' command channel disconnected"
        );
    }

    #[test]
    fn exit_is_stored_without_removing_the_pane() {
        let mut runtime = runtime();
        let pane_id = PaneId::new("exit");
        let exit = TransportExit {
            code: None,
            signaled: true,
        };
        open_replay(
            &mut runtime,
            &pane_id,
            "replay-exit",
            [ReplayStep::exit(exit)],
        )
        .unwrap();

        pump_until_exit(&mut runtime, &pane_id);

        assert_eq!(runtime.exit(&pane_id), Some(exit));
        assert!(runtime.frame(&pane_id).is_ok());
    }

    #[test]
    fn successful_completion_without_exit_event_is_still_observable() {
        let mut runtime = runtime();
        let pane_id = PaneId::new("implicit-exit");
        open_replay(&mut runtime, &pane_id, "replay-implicit-exit", []).unwrap();

        pump_until_exit(&mut runtime, &pane_id);

        assert_eq!(
            runtime.exit(&pane_id),
            Some(TransportExit {
                code: None,
                signaled: false,
            })
        );
    }

    #[test]
    fn replay_mismatch_surfaces_worker_error_and_keeps_pane_accessible() {
        let mut runtime = runtime();
        let pane_id = PaneId::new("mismatch");
        open_replay(
            &mut runtime,
            &pane_id,
            "replay-mismatch",
            [ReplayStep::expect_input(b"expected")],
        )
        .unwrap();

        runtime.input(&pane_id, b"actual").unwrap();
        let error = pump_until_error(&mut runtime, &pane_id);

        assert!(error.to_string().contains("expected command"));
        assert!(runtime.frame(&pane_id).is_ok());
        runtime.close(&pane_id).unwrap();
    }

    #[test]
    fn mismatch_duplicate_and_missing_pane_errors_do_not_change_state() {
        let mut runtime = runtime();
        let pane_id = PaneId::new("validation");
        let mismatch = runtime.open(
            pane_spec(&pane_id, "declared"),
            TerminalSize::new(20, 4),
            Box::new(ReplayTransport::new(TransportId::new("actual"), [])),
        );
        assert!(mismatch
            .unwrap_err()
            .to_string()
            .contains("expects transport"));
        assert!(runtime.metadata(&pane_id).is_none());

        open_replay(
            &mut runtime,
            &pane_id,
            "replay-validation",
            [ReplayStep::expect_shutdown()],
        )
        .unwrap();
        let duplicate = runtime.open(
            pane_spec(&pane_id, "replay-validation"),
            TerminalSize::new(20, 4),
            Box::new(ReplayTransport::new(
                TransportId::new("replay-validation"),
                [],
            )),
        );
        assert_eq!(
            duplicate.unwrap_err().to_string(),
            "terminal pane 'validation' is already registered"
        );
        assert_eq!(runtime.metadata(&pane_id).unwrap().label, "Codex");

        let missing = PaneId::new("missing");
        assert_eq!(
            runtime.pump(&missing).unwrap_err().to_string(),
            "terminal pane 'missing' is not registered"
        );
        assert!(runtime.input(&missing, &[]).is_err());
        assert!(runtime.resize(&missing, TerminalSize::new(1, 1)).is_err());
        assert!(runtime.frame(&missing).is_err());
        assert!(runtime.set_label(&missing, "none").is_err());
        assert!(runtime.close(&missing).is_err());
    }

    #[test]
    fn close_removes_engine_metadata_and_transport_together() {
        let mut runtime = runtime();
        let pane_id = PaneId::new("close");
        open_replay(
            &mut runtime,
            &pane_id,
            "replay-close",
            [ReplayStep::expect_shutdown()],
        )
        .unwrap();
        runtime.set_label(&pane_id, "Renamed").unwrap();

        runtime.close(&pane_id).unwrap();

        assert!(runtime.metadata(&pane_id).is_none());
        assert_eq!(runtime.exit(&pane_id), None);
        assert!(runtime.frame(&pane_id).is_err());
        assert!(runtime.close(&pane_id).is_err());
    }

    struct ReplyBackpressureTransport {
        id: TransportId,
        release: Receiver<()>,
        observed: SyncSender<Vec<TransportCommand>>,
    }

    impl SessionTransport for ReplyBackpressureTransport {
        fn id(&self) -> &TransportId {
            &self.id
        }

        fn run(
            self: Box<Self>,
            commands: Receiver<TransportCommand>,
            events: SyncSender<TransportEvent>,
        ) -> Result<(), TerminalError> {
            events
                .send(TransportEvent::Output(b"\x1b[6n".to_vec()))
                .map_err(|_| TerminalError::new("query event receiver disconnected"))?;
            self.release
                .recv()
                .map_err(|_| TerminalError::new("reply test was not released"))?;
            let received = vec![
                commands
                    .recv()
                    .map_err(|_| TerminalError::new("missing queued user input"))?,
                commands
                    .recv()
                    .map_err(|_| TerminalError::new("missing parser reply"))?,
            ];
            self.observed
                .send(received)
                .map_err(|_| TerminalError::new("reply observer disconnected"))?;
            events
                .send(TransportEvent::Exited(successful_exit()))
                .map_err(|_| TerminalError::new("exit receiver disconnected"))
        }
    }

    struct DelayedFailureTransport {
        id: TransportId,
        release: Receiver<()>,
    }

    struct DrainPanicFactory;

    impl EngineFactory for DrainPanicFactory {
        fn create(&self, size: TerminalSize) -> Result<Box<dyn TerminalEngine>, TerminalError> {
            Ok(Box::new(DrainPanicEngine {
                id: EngineId::new("drain-panic"),
                snapshot: TerminalSnapshot::blank(size),
            }))
        }
    }

    struct DrainPanicEngine {
        id: EngineId,
        snapshot: TerminalSnapshot,
    }

    impl TerminalEngine for DrainPanicEngine {
        fn id(&self) -> &EngineId {
            &self.id
        }

        fn feed(&mut self, _bytes: &[u8]) -> Result<(), TerminalError> {
            Ok(())
        }

        fn resize(&mut self, size: TerminalSize) -> Result<(), TerminalError> {
            self.snapshot = TerminalSnapshot::blank(size);
            Ok(())
        }

        fn snapshot(&self) -> TerminalSnapshot {
            self.snapshot.clone()
        }

        fn drain_events(&mut self) -> Vec<TerminalEngineEvent> {
            panic!("drain exploded")
        }
    }

    impl SessionTransport for DelayedFailureTransport {
        fn id(&self) -> &TransportId {
            &self.id
        }

        fn run(
            self: Box<Self>,
            _commands: Receiver<TransportCommand>,
            events: SyncSender<TransportEvent>,
        ) -> Result<(), TerminalError> {
            self.release
                .recv()
                .map_err(|_| TerminalError::new("failure test was not released"))?;
            events
                .send(TransportEvent::Output(b"final-output".to_vec()))
                .map_err(|_| TerminalError::new("final output receiver disconnected"))?;
            Err(TerminalError::new("injected transport failure"))
        }
    }

    fn runtime() -> LivePaneRuntime {
        runtime_with_capacities(8, 8)
    }

    fn runtime_with_capacities(command_capacity: usize, event_capacity: usize) -> LivePaneRuntime {
        let mut registry = EngineRegistry::default();
        registry.register(EngineId::new(ALACRITTY_ENGINE_ID), AlacrittyEngineFactory);
        let engines = EngineRuntime::start(1, registry).unwrap();
        LivePaneRuntime::new(
            PaneRuntime::new(engines),
            TransportRuntime::new(command_capacity, event_capacity).unwrap(),
        )
    }

    fn pane_spec(pane_id: &PaneId, transport_id: &str) -> PaneSpec {
        PaneSpec {
            id: pane_id.clone(),
            label: "Codex".to_string(),
            engine_id: EngineId::new(ALACRITTY_ENGINE_ID),
            transport_id: TransportId::new(transport_id),
        }
    }

    fn open_replay<const N: usize>(
        runtime: &mut LivePaneRuntime,
        pane_id: &PaneId,
        transport_id: &str,
        steps: [ReplayStep; N],
    ) -> Result<(), TerminalError> {
        runtime.open(
            pane_spec(pane_id, transport_id),
            TerminalSize::new(20, 4),
            Box::new(ReplayTransport::new(TransportId::new(transport_id), steps)),
        )
    }

    fn recording_runtime(
        command_capacity: usize,
        event_capacity: usize,
    ) -> (LivePaneRuntime, Arc<Mutex<Vec<String>>>) {
        let operations = Arc::new(Mutex::new(Vec::new()));
        let mut registry = EngineRegistry::default();
        registry.register(
            EngineId::new("recording"),
            RecordingFactory {
                operations: operations.clone(),
            },
        );
        let engines = EngineRuntime::start(1, registry).unwrap();
        (
            LivePaneRuntime::new(
                PaneRuntime::new(engines),
                TransportRuntime::new(command_capacity, event_capacity).unwrap(),
            ),
            operations,
        )
    }

    fn open_recording_replay(
        runtime: &mut LivePaneRuntime,
        pane_id: &PaneId,
        transport_id: &str,
        steps: impl IntoIterator<Item = ReplayStep>,
    ) -> Result<(), TerminalError> {
        runtime.open(
            PaneSpec {
                id: pane_id.clone(),
                label: "Recording".to_string(),
                engine_id: EngineId::new("recording"),
                transport_id: TransportId::new(transport_id),
            },
            TerminalSize::new(20, 4),
            Box::new(ReplayTransport::new(TransportId::new(transport_id), steps)),
        )
    }

    fn pump_until_exit(runtime: &mut LivePaneRuntime, pane_id: &PaneId) {
        let deadline = Instant::now() + TEST_TIMEOUT;
        while runtime.exit(pane_id).is_none() {
            runtime.pump(pane_id).unwrap();
            assert!(
                Instant::now() < deadline,
                "replay did not emit exit before timeout"
            );
            std::thread::yield_now();
        }
    }

    fn pump_until_error(runtime: &mut LivePaneRuntime, pane_id: &PaneId) -> TerminalError {
        let deadline = Instant::now() + TEST_TIMEOUT;
        loop {
            if let Err(error) = runtime.pump(pane_id) {
                return error;
            }
            assert!(
                Instant::now() < deadline,
                "replay did not fail before timeout"
            );
            std::thread::yield_now();
        }
    }

    fn wait_for_transport_completion(runtime: &mut LivePaneRuntime, pane_id: &PaneId) {
        let deadline = Instant::now() + TEST_TIMEOUT;
        loop {
            if let Some(completion) = runtime
                .transports
                .get_mut(pane_id)
                .unwrap()
                .handle
                .try_completion()
            {
                completion.unwrap();
                return;
            }
            assert!(
                Instant::now() < deadline,
                "replay did not complete before timeout"
            );
            std::thread::yield_now();
        }
    }

    fn successful_exit() -> TransportExit {
        TransportExit {
            code: Some(0),
            signaled: false,
        }
    }

    struct RecordingFactory {
        operations: Arc<Mutex<Vec<String>>>,
    }

    impl EngineFactory for RecordingFactory {
        fn create(&self, size: TerminalSize) -> Result<Box<dyn TerminalEngine>, TerminalError> {
            Ok(Box::new(RecordingEngine {
                id: EngineId::new("recording"),
                size,
                operations: self.operations.clone(),
            }))
        }
    }

    struct RecordingEngine {
        id: EngineId,
        size: TerminalSize,
        operations: Arc<Mutex<Vec<String>>>,
    }

    impl TerminalEngine for RecordingEngine {
        fn id(&self) -> &EngineId {
            &self.id
        }

        fn feed(&mut self, bytes: &[u8]) -> Result<(), TerminalError> {
            self.operations
                .lock()
                .unwrap()
                .push(format!("feed:{}", String::from_utf8_lossy(bytes)));
            Ok(())
        }

        fn resize(&mut self, size: TerminalSize) -> Result<(), TerminalError> {
            self.size = size;
            self.operations
                .lock()
                .unwrap()
                .push(format!("resize:{}x{}", size.columns, size.rows));
            Ok(())
        }

        fn snapshot(&self) -> TerminalSnapshot {
            TerminalSnapshot::blank(self.size)
        }
    }
}
