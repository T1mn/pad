//! Embedded terminal core.
//!
//! The module is intentionally not connected to the default UI until the M3
//! acceptance gate. Remove this temporary allowance when that wiring lands.
#![allow(dead_code, unused_imports)]

mod alacritty;
mod engine;
mod live_pane;
mod model;
mod pane;
mod transport;
mod transport_runtime;
mod widget;
mod worker;

pub use alacritty::{AlacrittyEngine, AlacrittyEngineFactory, ALACRITTY_ENGINE_ID};
pub use engine::{EngineFactory, EngineId, EngineRegistry, TerminalEngine, TerminalError};
pub use live_pane::LivePaneRuntime;
pub use model::{
    CursorShape, PaneId, TerminalCell, TerminalColor, TerminalCursor, TerminalEngineEvent,
    TerminalMode, TerminalSize, TerminalSnapshot, TextAttributes,
};
pub use pane::{PaneFrame, PaneMetadata, PaneRuntime, PaneSpec};
pub use transport::{
    ReplayStep, ReplayTransport, SessionTransport, TransportCommand, TransportEvent, TransportExit,
    TransportId,
};
pub use transport_runtime::{
    ShutdownSignal, TransportHandle, TransportRuntime, DEFAULT_COMMAND_QUEUE_CAPACITY,
    DEFAULT_EVENT_QUEUE_CAPACITY,
};
pub use widget::TerminalPaneWidget;
pub use worker::EngineRuntime;

#[cfg(test)]
mod stress_tests;
#[cfg(test)]
mod tests;
