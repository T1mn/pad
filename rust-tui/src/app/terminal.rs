use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use crate::terminal_runtime::{
    AlacrittyEngineFactory, ControllerQueueError, EngineId, EngineRegistry, EngineRuntime,
    LivePaneRuntime, NativePaneRequest, NativePtyCommand, PaneEpoch, PaneFrame, PaneId,
    PaneRuntime, PaneSpec, TerminalController, TerminalError, TerminalFrameReader, TerminalMode,
    TerminalSize, TransportExit, TransportId, TransportRuntime, ALACRITTY_ENGINE_ID,
};

use super::App;

const MAIN_PANE_ID: &str = "native-main";
const MAIN_TRANSPORT_ID: &str = "native:main";
const MAX_PENDING_INPUT_BYTES: usize = 1024 * 1024;

#[derive(Default)]
pub struct TerminalUiState {
    controller: Option<TerminalController>,
    frames: Option<TerminalFrameReader>,
    pane_id: Option<PaneId>,
    epoch: Option<PaneEpoch>,
    revision: u64,
    frame: Option<Arc<PaneFrame>>,
    pending_input: VecDeque<Vec<u8>>,
    pending_input_bytes: usize,
    pending_resize: Option<TerminalSize>,
    last_requested_size: Option<TerminalSize>,
    error: Option<String>,
    exit: Option<TransportExit>,
}

impl TerminalUiState {
    pub fn is_active(&self) -> bool {
        self.controller.is_some() && self.pane_id.is_some() && self.epoch.is_some()
    }

    pub fn frame(&self) -> Option<&Arc<PaneFrame>> {
        self.frame.as_ref()
    }

    pub fn mode(&self) -> TerminalMode {
        self.frame
            .as_ref()
            .map(|frame| frame.terminal.mode)
            .unwrap_or_default()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn exit(&self) -> Option<TransportExit> {
        self.exit
    }
}

impl App {
    pub fn start_native_terminal(&mut self, size: TerminalSize) -> Result<(), TerminalError> {
        if self.terminal.is_active() {
            return Ok(());
        }

        let worker_count = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1)
            .min(4);
        let mut registry = EngineRegistry::default();
        registry.register(EngineId::new(ALACRITTY_ENGINE_ID), AlacrittyEngineFactory);
        let engines = EngineRuntime::start(worker_count, registry)?;
        let runtime = LivePaneRuntime::new(PaneRuntime::new(engines), TransportRuntime::default());
        let controller = TerminalController::start(runtime)?;
        let frames = controller.frames();
        let pane_id = PaneId::new(MAIN_PANE_ID);
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let label = cwd
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .map(|name| format!("Shell · {name}"))
            .unwrap_or_else(|| "Native Shell".to_string());
        let command = NativePtyCommand::default_program()
            .cwd(&cwd)
            .env("PAD_PANE_ID", MAIN_PANE_ID);
        let request = NativePaneRequest {
            spec: PaneSpec {
                id: pane_id.clone(),
                label,
                engine_id: EngineId::new(ALACRITTY_ENGINE_ID),
                transport_id: TransportId::new(MAIN_TRANSPORT_ID),
            },
            size,
            command,
        };
        let epoch = controller.open_native(request).map_err(|error| {
            TerminalError::new(format!("failed to open native terminal: {error}"))
        })?;

        self.terminal = TerminalUiState {
            controller: Some(controller),
            frames: Some(frames),
            pane_id: Some(pane_id),
            epoch: Some(epoch),
            last_requested_size: Some(size),
            ..TerminalUiState::default()
        };
        self.dirty = true;
        Ok(())
    }

    pub fn terminal_is_active(&self) -> bool {
        self.terminal.is_active()
    }

    pub fn terminal_is_focused(&self) -> bool {
        self.terminal.is_active()
            && self.mode == crate::app::state::Mode::Normal
            && self.preview_is_focused()
    }

    pub fn focus_terminal(&mut self) -> bool {
        if !self.terminal.is_active() || self.sidebar.show_tree {
            return false;
        }
        self.preview.focus = crate::app::state::FocusTarget::Preview;
        self.dirty = true;
        true
    }

    pub fn terminal_mode(&self) -> TerminalMode {
        self.terminal.mode()
    }

    pub fn terminal_frame(&self) -> Option<&Arc<PaneFrame>> {
        self.terminal.frame()
    }

    pub fn terminal_error(&self) -> Option<&str> {
        self.terminal.error()
    }

    pub fn terminal_exit(&self) -> Option<TransportExit> {
        self.terminal.exit()
    }

    /// Queues input without blocking the UI. Controller backpressure is
    /// retained locally and retried in exact order during the next pre-draw
    /// cycle.
    pub fn send_terminal_input(&mut self, bytes: Vec<u8>) -> Result<(), TerminalError> {
        if bytes.is_empty() {
            return Ok(());
        }
        if self
            .terminal
            .pending_input_bytes
            .saturating_add(bytes.len())
            > MAX_PENDING_INPUT_BYTES
        {
            let error = TerminalError::new(format!(
                "native terminal input backlog exceeded {MAX_PENDING_INPUT_BYTES} bytes"
            ));
            self.terminal.error = Some(error.to_string());
            self.dirty = true;
            return Err(error);
        }
        self.terminal.pending_input_bytes += bytes.len();
        self.terminal.pending_input.push_back(bytes);
        self.flush_terminal_commands();
        Ok(())
    }

    pub fn resize_native_terminal(&mut self, size: TerminalSize) {
        if !self.terminal.is_active()
            || self.terminal.last_requested_size == Some(size)
            || self.terminal.pending_resize == Some(size)
        {
            return;
        }
        // Resize is latest-state data. Coalescing intermediate viewport sizes
        // avoids making a drag-resize wait behind obsolete geometry.
        self.terminal.pending_resize = Some(size);
        self.flush_terminal_commands();
    }

    pub fn poll_native_terminal(&mut self) {
        self.flush_terminal_commands();
        let (Some(frames), Some(pane_id), Some(epoch)) = (
            self.terminal.frames.as_ref(),
            self.terminal.pane_id.as_ref(),
            self.terminal.epoch,
        ) else {
            return;
        };
        let Some(published) = frames.latest_for_epoch(pane_id, epoch) else {
            return;
        };
        if published.revision == self.terminal.revision {
            return;
        }
        self.terminal.revision = published.revision;
        self.terminal.frame = published.frame.clone();
        self.terminal.error = published.error.as_deref().map(str::to_string);
        self.terminal.exit = published.exit;
        self.dirty = true;
    }

    pub fn shutdown_native_terminal(&mut self) -> Result<(), TerminalError> {
        let controller = self.terminal.controller.take();
        self.terminal.frames = None;
        self.terminal.pane_id = None;
        self.terminal.epoch = None;
        if let Some(controller) = controller {
            controller.shutdown()?;
        }
        Ok(())
    }

    fn flush_terminal_commands(&mut self) {
        let (Some(controller), Some(pane_id), Some(epoch)) = (
            self.terminal.controller.as_ref(),
            self.terminal.pane_id.as_ref(),
            self.terminal.epoch,
        ) else {
            return;
        };

        while let Some(bytes) = self.terminal.pending_input.pop_front() {
            self.terminal.pending_input_bytes -= bytes.len();
            match controller.input(pane_id.clone(), epoch, bytes) {
                Ok(()) => {}
                Err(ControllerQueueError::Full(bytes)) => {
                    self.terminal.pending_input_bytes += bytes.len();
                    self.terminal.pending_input.push_front(bytes);
                    break;
                }
                Err(ControllerQueueError::Disconnected(_)) => {
                    self.terminal.error = Some(
                        "native terminal controller disconnected while sending input".to_string(),
                    );
                    break;
                }
            }
        }

        if let Some(size) = self.terminal.pending_resize.take() {
            match controller.resize(pane_id.clone(), epoch, size) {
                Ok(()) => self.terminal.last_requested_size = Some(size),
                Err(ControllerQueueError::Full(size)) => self.terminal.pending_resize = Some(size),
                Err(ControllerQueueError::Disconnected(_)) => {
                    self.terminal.error =
                        Some("native terminal controller disconnected while resizing".to_string());
                }
            }
        }
    }
}
