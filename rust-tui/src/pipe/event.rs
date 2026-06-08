/// Events emitted by the tmux control pipe
#[derive(Debug)]
pub enum TmuxEvent {
    /// A window was added/removed/changed
    WindowChanged,
    /// A pane mode changed (could indicate state change)
    PaneModeChanged,
    /// Session changed
    SessionChanged,
    /// Output detected on a pane
    OutputDetected,
    /// Pipe disconnected
    Disconnected,
}
