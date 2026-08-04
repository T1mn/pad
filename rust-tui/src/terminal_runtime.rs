//! Embedded terminal core.
//!
//! Native PTY is connected to the default UI. The remaining public surface is
//! kept for the planned multi-pane and compatibility backends.
#![allow(dead_code)]

mod alacritty;
mod controller;
mod engine;
mod input;
mod live_pane;
mod model;
mod native_pty;
mod pane;
mod transport;
mod transport_runtime;
mod widget;
mod worker;

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
mod stress_tests;
#[cfg(test)]
mod tests;
