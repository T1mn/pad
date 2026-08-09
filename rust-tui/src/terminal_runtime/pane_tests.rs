use super::*;
use crate::terminal_runtime::{AlacrittyEngineFactory, EngineRegistry, ALACRITTY_ENGINE_ID};

pub(crate) fn pane_metadata_can_change_without_recreating_engine() {
    let (mut runtime, pane_id) = runtime_with_pane();
    runtime.feed_output(&pane_id, b"ready".to_vec()).unwrap();

    runtime.set_label(&pane_id, "Claude · review").unwrap();
    let frame = runtime.frame(&pane_id).unwrap();

    assert_eq!(frame.metadata.label, "Claude · review");
    assert_eq!(frame.terminal.row_text(0).as_deref(), Some("ready"));
    assert_eq!(runtime.len(), 1);
}

pub(crate) fn pane_scroll_updates_only_its_immutable_frame_viewport() {
    let (runtime, pane_id) = runtime_with_pane();
    runtime
        .feed_output(&pane_id, b"0\r\n1\r\n2\r\n3\r\n4\r\n5\r\n6".to_vec())
        .unwrap();

    runtime.scroll(&pane_id, TerminalScroll::Lines(2)).unwrap();
    let scrolled = runtime.frame(&pane_id).unwrap();

    assert_eq!(scrolled.terminal.viewport.display_offset, 2);
    assert_eq!(scrolled.terminal.row_text(0).as_deref(), Some("1"));
    assert_eq!(scrolled.terminal.cursor, None);
}

pub(crate) fn close_removes_metadata_and_terminal_engine_together() {
    let (mut runtime, pane_id) = runtime_with_pane();

    runtime.close(&pane_id).unwrap();

    assert!(runtime.is_empty());
    assert!(runtime.metadata(&pane_id).is_none());
    assert!(runtime.frame(&pane_id).is_err());
}

pub(crate) fn duplicate_pane_keeps_original_metadata() {
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

pub(crate) fn close_panic_removes_metadata_and_allows_reopen() {
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
