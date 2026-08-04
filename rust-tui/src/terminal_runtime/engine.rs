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
