use std::collections::HashMap;

use super::{
    EngineId, EngineRuntime, PaneId, TerminalEngineEvent, TerminalError, TerminalSize,
    TerminalSnapshot, TransportId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaneSpec {
    pub id: PaneId,
    pub label: String,
    pub engine_id: EngineId,
    pub transport_id: TransportId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaneMetadata {
    pub id: PaneId,
    pub label: String,
    pub engine_id: EngineId,
    pub transport_id: TransportId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaneFrame {
    pub metadata: PaneMetadata,
    pub terminal: TerminalSnapshot,
}

pub struct PaneRuntime {
    engines: EngineRuntime,
    panes: HashMap<PaneId, PaneMetadata>,
}

impl PaneRuntime {
    pub fn new(engines: EngineRuntime) -> Self {
        Self {
            engines,
            panes: HashMap::new(),
        }
    }

    pub fn open(&mut self, spec: PaneSpec, size: TerminalSize) -> Result<(), TerminalError> {
        if self.panes.contains_key(&spec.id) {
            return Err(TerminalError::new(format!(
                "terminal pane '{}' is already registered",
                spec.id
            )));
        }
        self.engines
            .open(spec.id.clone(), spec.engine_id.clone(), size)?;
        self.panes.insert(
            spec.id.clone(),
            PaneMetadata {
                id: spec.id,
                label: spec.label,
                engine_id: spec.engine_id,
                transport_id: spec.transport_id,
            },
        );
        Ok(())
    }

    pub fn feed_output(&self, pane_id: &PaneId, bytes: Vec<u8>) -> Result<(), TerminalError> {
        self.ensure_pane(pane_id)?;
        self.engines.feed(pane_id, bytes)
    }

    pub fn resize(&self, pane_id: &PaneId, size: TerminalSize) -> Result<(), TerminalError> {
        self.ensure_pane(pane_id)?;
        self.engines.resize(pane_id, size)
    }

    pub fn frame(&self, pane_id: &PaneId) -> Result<PaneFrame, TerminalError> {
        let metadata = self.ensure_pane(pane_id)?.clone();
        let terminal = self.engines.snapshot(pane_id)?;
        Ok(PaneFrame { metadata, terminal })
    }

    pub fn drain_engine_events(
        &self,
        pane_id: &PaneId,
    ) -> Result<Vec<TerminalEngineEvent>, TerminalError> {
        self.ensure_pane(pane_id)?;
        self.engines.drain_events(pane_id)
    }

    pub fn metadata(&self, pane_id: &PaneId) -> Option<&PaneMetadata> {
        self.panes.get(pane_id)
    }

    pub fn set_label(
        &mut self,
        pane_id: &PaneId,
        label: impl Into<String>,
    ) -> Result<(), TerminalError> {
        let metadata = self.panes.get_mut(pane_id).ok_or_else(|| {
            TerminalError::new(format!("terminal pane '{pane_id}' is not registered"))
        })?;
        metadata.label = label.into();
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.panes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.panes.is_empty()
    }

    pub fn close(&mut self, pane_id: &PaneId) -> Result<(), TerminalError> {
        self.ensure_pane(pane_id)?;
        let result = self.engines.close(pane_id);
        // The worker removes the engine before running its destructor. Even
        // when destruction reports a contained panic, metadata must be
        // removed so the pane ID can be opened again.
        self.panes.remove(pane_id);
        result
    }

    fn ensure_pane(&self, pane_id: &PaneId) -> Result<&PaneMetadata, TerminalError> {
        self.panes.get(pane_id).ok_or_else(|| {
            TerminalError::new(format!("terminal pane '{pane_id}' is not registered"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_runtime::{AlacrittyEngineFactory, EngineRegistry, ALACRITTY_ENGINE_ID};

    #[test]
    fn pane_metadata_can_change_without_recreating_engine() {
        let (mut runtime, pane_id) = runtime_with_pane();
        runtime.feed_output(&pane_id, b"ready".to_vec()).unwrap();

        runtime.set_label(&pane_id, "Claude · review").unwrap();
        let frame = runtime.frame(&pane_id).unwrap();

        assert_eq!(frame.metadata.label, "Claude · review");
        assert_eq!(frame.terminal.row_text(0).as_deref(), Some("ready"));
        assert_eq!(runtime.len(), 1);
    }

    #[test]
    fn close_removes_metadata_and_terminal_engine_together() {
        let (mut runtime, pane_id) = runtime_with_pane();

        runtime.close(&pane_id).unwrap();

        assert!(runtime.is_empty());
        assert!(runtime.metadata(&pane_id).is_none());
        assert!(runtime.frame(&pane_id).is_err());
    }

    #[test]
    fn duplicate_pane_keeps_original_metadata() {
        let (mut runtime, pane_id) = runtime_with_pane();
        let duplicate = PaneSpec {
            id: pane_id.clone(),
            label: "replacement".to_string(),
            engine_id: EngineId::new(ALACRITTY_ENGINE_ID),
            transport_id: TransportId::new("native-pty"),
        };

        assert!(runtime.open(duplicate, TerminalSize::new(40, 10)).is_err());
        let metadata = runtime.metadata(&pane_id).unwrap();
        assert_eq!(metadata.label, "Codex");
        assert_eq!(metadata.transport_id.as_str(), "replay");
    }

    #[test]
    fn close_panic_removes_metadata_and_allows_reopen() {
        let mut registry = EngineRegistry::default();
        registry.register(EngineId::new("drop-panic"), DropPanicFactory);
        registry.register(EngineId::new(ALACRITTY_ENGINE_ID), AlacrittyEngineFactory);
        let engines = EngineRuntime::start(1, registry).unwrap();
        let mut runtime = PaneRuntime::new(engines);
        let pane_id = PaneId::new("recoverable");
        runtime
            .open(
                PaneSpec {
                    id: pane_id.clone(),
                    label: "faulty".to_string(),
                    engine_id: EngineId::new("drop-panic"),
                    transport_id: TransportId::new("replay"),
                },
                TerminalSize::new(10, 2),
            )
            .unwrap();

        assert!(runtime.close(&pane_id).is_err());
        assert!(runtime.metadata(&pane_id).is_none());
        runtime
            .open(
                PaneSpec {
                    id: pane_id.clone(),
                    label: "healthy".to_string(),
                    engine_id: EngineId::new(ALACRITTY_ENGINE_ID),
                    transport_id: TransportId::new("replay"),
                },
                TerminalSize::new(10, 2),
            )
            .unwrap();
        assert_eq!(runtime.metadata(&pane_id).unwrap().label, "healthy");
    }

    struct DropPanicFactory;

    impl crate::terminal_runtime::EngineFactory for DropPanicFactory {
        fn create(
            &self,
            size: TerminalSize,
        ) -> Result<Box<dyn crate::terminal_runtime::TerminalEngine>, TerminalError> {
            Ok(Box::new(DropPanicEngine {
                id: EngineId::new("drop-panic"),
                snapshot: TerminalSnapshot::blank(size),
            }))
        }
    }

    struct DropPanicEngine {
        id: EngineId,
        snapshot: TerminalSnapshot,
    }

    impl crate::terminal_runtime::TerminalEngine for DropPanicEngine {
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
    }

    impl Drop for DropPanicEngine {
        fn drop(&mut self) {
            panic!("injected pane close panic");
        }
    }

    fn runtime_with_pane() -> (PaneRuntime, PaneId) {
        let mut registry = EngineRegistry::default();
        registry.register(EngineId::new(ALACRITTY_ENGINE_ID), AlacrittyEngineFactory);
        let engines = EngineRuntime::start(1, registry).unwrap();
        let mut runtime = PaneRuntime::new(engines);
        let pane_id = PaneId::new("pane-1");
        runtime
            .open(
                PaneSpec {
                    id: pane_id.clone(),
                    label: "Codex".to_string(),
                    engine_id: EngineId::new(ALACRITTY_ENGINE_ID),
                    transport_id: TransportId::new("replay"),
                },
                TerminalSize::new(20, 4),
            )
            .unwrap();
        (runtime, pane_id)
    }
}
