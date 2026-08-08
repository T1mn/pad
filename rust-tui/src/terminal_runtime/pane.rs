use std::collections::HashMap;

use super::{
    EngineId, EngineRuntime, PaneId, TerminalEngineEvent, TerminalError, TerminalScroll,
    TerminalSize, TerminalSnapshot, TransportId,
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

    pub fn scroll(&self, pane_id: &PaneId, scroll: TerminalScroll) -> Result<(), TerminalError> {
        self.ensure_pane(pane_id)?;
        self.engines.scroll(pane_id, scroll)
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
#[path = "pane_tests.rs"]
mod tests;
