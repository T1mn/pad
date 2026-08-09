//! Embedded terminal core.
//!
//! Native PTY is connected to the default UI. The remaining public surface is
//! kept for the planned multi-pane and compatibility backends.
#![allow(dead_code)]

pub(crate) mod alacritty;
pub(crate) mod controller;
mod engine {
    use std::collections::HashMap;
    use std::error::Error;
    use std::fmt;
    use std::sync::Arc;

    use super::model::TerminalEngineEvent;
    use super::{TerminalScroll, TerminalSize, TerminalSnapshot};

    #[derive(Clone, Debug, Eq, Hash, PartialEq)]
    pub struct EngineId(String);

    impl EngineId {
        pub fn new(value: impl Into<String>) -> Self {
            Self(value.into())
        }

        pub fn as_str(&self) -> &str {
            &self.0
        }
    }

    impl fmt::Display for EngineId {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(&self.0)
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct TerminalError(String);

    impl TerminalError {
        pub fn new(message: impl Into<String>) -> Self {
            Self(message.into())
        }
    }

    impl fmt::Display for TerminalError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(&self.0)
        }
    }

    impl Error for TerminalError {}

    pub trait TerminalEngine: 'static {
        fn id(&self) -> &EngineId;
        fn feed(&mut self, bytes: &[u8]) -> Result<(), TerminalError>;
        fn resize(&mut self, size: TerminalSize) -> Result<(), TerminalError>;
        fn scroll(&mut self, _scroll: TerminalScroll) -> Result<(), TerminalError> {
            Ok(())
        }
        fn snapshot(&self) -> TerminalSnapshot;
        fn drain_events(&mut self) -> Vec<TerminalEngineEvent> {
            Vec::new()
        }
    }

    /// Factories are shared across worker threads, while engines are constructed
    /// inside a worker and deliberately do not need to implement Send or Sync.
    pub trait EngineFactory: Send + Sync + 'static {
        fn create(&self, size: TerminalSize) -> Result<Box<dyn TerminalEngine>, TerminalError>;
    }

    #[derive(Clone, Default)]
    pub struct EngineRegistry {
        factories: HashMap<EngineId, Arc<dyn EngineFactory>>,
    }

    impl EngineRegistry {
        pub fn register(
            &mut self,
            id: EngineId,
            factory: impl EngineFactory,
        ) -> Option<Arc<dyn EngineFactory>> {
            self.factories.insert(id, Arc::new(factory))
        }

        pub fn register_shared(
            &mut self,
            id: EngineId,
            factory: Arc<dyn EngineFactory>,
        ) -> Option<Arc<dyn EngineFactory>> {
            self.factories.insert(id, factory)
        }

        pub(super) fn create(
            &self,
            id: &EngineId,
            size: TerminalSize,
        ) -> Result<Box<dyn TerminalEngine>, TerminalError> {
            let factory = self.factories.get(id).ok_or_else(|| {
                TerminalError::new(format!("terminal engine '{id}' is not registered"))
            })?;
            factory.create(size)
        }
    }
}
pub(crate) mod input;
pub(crate) mod live_pane;
mod model;
pub(crate) mod native_pty;
pub(crate) mod pane;
pub(crate) mod transport;
pub(crate) mod transport_runtime;
pub(crate) mod widget;
pub(crate) mod worker;

#[allow(unused_imports)]
pub use alacritty::{AlacrittyEngine, AlacrittyEngineFactory, ALACRITTY_ENGINE_ID};
#[allow(unused_imports)]
pub use controller::{
    ControllerQueueError, NativePaneRequest, PaneEpoch, PublishedPane, TerminalController,
    TerminalFrameReader, DEFAULT_CONTROLLER_COMMAND_CAPACITY,
};
pub use engine::{EngineFactory, EngineId, EngineRegistry, TerminalEngine, TerminalError};
pub use input::{encode_key_event, encode_mouse_event, encode_paste};
pub use live_pane::LivePaneRuntime;
pub use model::{
    CursorShape, PaneId, TerminalCell, TerminalColor, TerminalCursor, TerminalEngineEvent,
    TerminalMode, TerminalScroll, TerminalSize, TerminalSnapshot, TerminalViewport, TextAttributes,
};
pub use native_pty::{NativePtyCommand, NativePtyTransport};
pub use pane::{PaneFrame, PaneMetadata, PaneRuntime, PaneSpec};
#[allow(unused_imports)]
pub use transport::{
    ReplayStep, ReplayTransport, SessionTransport, TransportCommand, TransportEvent, TransportExit,
    TransportId,
};
#[allow(unused_imports)]
pub use transport_runtime::{
    ShutdownSignal, TransportHandle, TransportRuntime, DEFAULT_COMMAND_QUEUE_CAPACITY,
    DEFAULT_EVENT_QUEUE_CAPACITY,
};
pub use widget::TerminalPaneWidget;
pub use worker::EngineRuntime;

#[cfg(test)]
pub(crate) mod stress_tests {
    use std::sync::{Arc, Barrier};
    use std::thread;

    use super::{
        AlacrittyEngineFactory, EngineId, EngineRegistry, EngineRuntime, PaneId, TerminalSize,
        ALACRITTY_ENGINE_ID,
    };

    const PANE_COUNT: usize = 8;
    const WRITES_PER_PANE: usize = 250;

    pub(crate) fn eight_panes_process_output_resize_snapshot_and_close_concurrently() {
        let mut registry = EngineRegistry::default();
        registry.register(EngineId::new(ALACRITTY_ENGINE_ID), AlacrittyEngineFactory);
        let runtime = Arc::new(EngineRuntime::start(4, registry).unwrap());
        let panes: Vec<_> = (0..PANE_COUNT)
            .map(|index| PaneId::new(format!("stress-pane-{index}")))
            .collect();
        for pane_id in &panes {
            runtime
                .open(
                    pane_id.clone(),
                    EngineId::new(ALACRITTY_ENGINE_ID),
                    TerminalSize::new(80, 24),
                )
                .unwrap();
        }

        let start = Arc::new(Barrier::new(PANE_COUNT + 1));
        let workers: Vec<_> = panes
            .iter()
            .enumerate()
            .map(|(pane_index, pane_id)| {
                let runtime = runtime.clone();
                let start = start.clone();
                let pane_id = pane_id.clone();
                thread::spawn(move || {
                    start.wait();
                    for sequence in 0..WRITES_PER_PANE {
                        if sequence % 50 == 0 {
                            let columns = if sequence % 100 == 0 { 64 } else { 96 };
                            runtime
                                .resize(&pane_id, TerminalSize::new(columns, 24))
                                .unwrap();
                        }
                        runtime
                            .feed(
                                &pane_id,
                                format!("pane-{pane_index}:{sequence}\r\n").into_bytes(),
                            )
                            .unwrap();
                    }
                    runtime.resize(&pane_id, TerminalSize::new(80, 24)).unwrap();
                    runtime
                        .feed(
                            &pane_id,
                            format!("FINAL-PANE-{pane_index}\r\n").into_bytes(),
                        )
                        .unwrap();
                })
            })
            .collect();

        start.wait();
        for worker in workers {
            worker.join().unwrap();
        }

        for (pane_index, pane_id) in panes.iter().enumerate() {
            let snapshot = runtime.snapshot(pane_id).unwrap();
            assert_eq!(snapshot.size, TerminalSize::new(80, 24));
            let expected = format!("FINAL-PANE-{pane_index}");
            assert!(
                (0..snapshot.size.rows)
                    .filter_map(|row| snapshot.row_text(row))
                    .any(|row| row == expected),
                "final marker missing from {pane_id}"
            );
            runtime.close(pane_id).unwrap();
        }
    }
}
#[cfg(test)]
pub(crate) mod tests;
