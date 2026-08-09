use super::*;

pub(super) struct ReplyBackpressureTransport {
    pub(super) id: TransportId,
    pub(super) release: Receiver<()>,
    pub(super) observed: SyncSender<Vec<TransportCommand>>,
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

pub(super) struct DelayedFailureTransport {
    pub(super) id: TransportId,
    pub(super) release: Receiver<()>,
}

pub(super) struct DrainPanicFactory;

impl EngineFactory for DrainPanicFactory {
    fn create(&self, size: TerminalSize) -> Result<Box<dyn TerminalEngine>, TerminalError> {
        Ok(Box::new(DrainPanicEngine {
            id: EngineId::new("drain-panic"),
            snapshot: TerminalSnapshot::blank(size),
        }))
    }
}

pub(super) struct DrainPanicEngine {
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

pub(super) fn runtime() -> LivePaneRuntime {
    runtime_with_capacities(8, 8)
}

pub(super) fn runtime_with_capacities(
    command_capacity: usize,
    event_capacity: usize,
) -> LivePaneRuntime {
    let mut registry = EngineRegistry::default();
    registry.register(EngineId::new(ALACRITTY_ENGINE_ID), AlacrittyEngineFactory);
    let engines = EngineRuntime::start(1, registry).unwrap();
    LivePaneRuntime::new(
        PaneRuntime::new(engines),
        TransportRuntime::new(command_capacity, event_capacity).unwrap(),
    )
}

pub(super) fn pane_spec(pane_id: &PaneId, transport_id: &str) -> PaneSpec {
    PaneSpec {
        id: pane_id.clone(),
        label: "Codex".to_string(),
        engine_id: EngineId::new(ALACRITTY_ENGINE_ID),
        transport_id: TransportId::new(transport_id),
    }
}

pub(super) fn open_replay<const N: usize>(
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

pub(super) fn recording_runtime(
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

pub(super) fn open_recording_replay(
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

pub(super) fn pump_until_exit(runtime: &mut LivePaneRuntime, pane_id: &PaneId) {
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

pub(super) fn pump_until_error(runtime: &mut LivePaneRuntime, pane_id: &PaneId) -> TerminalError {
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

pub(super) fn wait_for_transport_completion(runtime: &mut LivePaneRuntime, pane_id: &PaneId) {
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

pub(super) fn successful_exit() -> TransportExit {
    TransportExit {
        code: Some(0),
        signaled: false,
    }
}

pub(super) struct RecordingFactory {
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

pub(super) struct RecordingEngine {
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
