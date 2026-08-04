use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;

use crate::terminal_runtime::{
    AlacrittyEngineFactory, ControllerQueueError, EngineId, EngineRegistry, EngineRuntime,
    LivePaneRuntime, NativePaneRequest, NativePtyCommand, PaneEpoch, PaneFrame, PaneId,
    PaneRuntime, PaneSpec, TerminalController, TerminalError, TerminalScroll, TerminalSize,
    TransportExit, TransportId, TransportRuntime, ALACRITTY_ENGINE_ID,
};

use super::model::{TerminalCommandDefinition, TerminalPaneDefinition, TerminalPaneId};
use super::{
    TerminalPaneLifecycle, TerminalUiState, MAX_PENDING_INPUT_BYTES, MAX_PENDING_SCROLL_COMMANDS,
};

pub(super) enum PendingTerminalIo {
    Input(Vec<u8>),
    Scroll(TerminalScroll),
}

pub(super) struct TerminalPaneRuntime {
    pub(super) runtime_id: PaneId,
    pub(super) epoch: Option<PaneEpoch>,
    pub(super) lifecycle: TerminalPaneLifecycle,
    pub(super) revision: u64,
    pub(super) frame: Option<Arc<PaneFrame>>,
    pub(super) pending_open: Option<NativePaneRequest>,
    pub(super) pending_io: VecDeque<PendingTerminalIo>,
    pub(super) pending_input_bytes: usize,
    pub(super) viewport_may_be_scrolled: bool,
    pub(super) viewport_reset_pending: bool,
    pub(super) pending_resize: Option<TerminalSize>,
    pub(super) last_requested_size: Option<TerminalSize>,
    pub(super) pending_label: Option<String>,
    pub(super) pending_close: bool,
    pub(super) error: Option<String>,
    pub(super) exit: Option<TransportExit>,
}

impl TerminalPaneRuntime {
    fn opening(request: NativePaneRequest) -> Self {
        Self {
            runtime_id: request.spec.id.clone(),
            epoch: None,
            lifecycle: TerminalPaneLifecycle::Opening,
            revision: 0,
            frame: None,
            pending_open: Some(request),
            pending_io: VecDeque::new(),
            pending_input_bytes: 0,
            viewport_may_be_scrolled: false,
            viewport_reset_pending: false,
            pending_resize: None,
            last_requested_size: None,
            pending_label: None,
            pending_close: false,
            error: None,
            exit: None,
        }
    }

    pub(super) fn mode(&self) -> crate::terminal_runtime::TerminalMode {
        self.frame
            .as_ref()
            .map(|frame| frame.terminal.mode)
            .unwrap_or_default()
    }

    fn remember_disconnected(&mut self, action: &str) {
        self.error = Some(format!(
            "native terminal controller disconnected while {action}"
        ));
        self.lifecycle = TerminalPaneLifecycle::Failed;
    }
}

impl TerminalUiState {
    pub(super) fn start_controller(&mut self) -> Result<(), TerminalError> {
        if self.controller.is_some() {
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
        self.frames = Some(controller.frames());
        self.controller = Some(controller);
        Ok(())
    }

    pub(super) fn install_pane_runtime(
        &mut self,
        definition: &TerminalPaneDefinition,
        size: TerminalSize,
    ) {
        let request = native_request(definition, size);
        self.panes
            .insert(definition.id, TerminalPaneRuntime::opening(request));
        self.flush_order.push(definition.id);
    }

    pub(super) fn queue_input(
        &mut self,
        pane_id: TerminalPaneId,
        bytes: Vec<u8>,
        return_to_bottom: bool,
    ) -> Result<(), TerminalError> {
        if bytes.is_empty() {
            return Ok(());
        }
        let Some(pane) = self.panes.get_mut(&pane_id) else {
            return Err(TerminalError::new(format!(
                "terminal pane {pane_id} is not running"
            )));
        };
        if pane.pending_close {
            return Err(TerminalError::new(format!(
                "terminal pane {pane_id} is closing"
            )));
        }
        if pane.pending_input_bytes.saturating_add(bytes.len()) > MAX_PENDING_INPUT_BYTES {
            let error = TerminalError::new(format!(
                "native terminal pane {pane_id} input backlog exceeded {MAX_PENDING_INPUT_BYTES} bytes"
            ));
            pane.error = Some(error.to_string());
            return Err(error);
        }
        // Typing and paste always resume the live viewport before the bytes are
        // delivered. Keeping both in one ordered queue prevents a full shared
        // controller queue from reversing the two operations.
        let is_scrolled = pane
            .frame
            .as_ref()
            .is_some_and(|frame| frame.terminal.viewport.display_offset > 0);
        if return_to_bottom
            && (is_scrolled || pane.viewport_may_be_scrolled)
            && !pane.viewport_reset_pending
        {
            // Pending viewport operations are obsolete once the user types.
            // Drop them so the mandatory Bottom remains bounded and follows
            // any scroll commands already accepted by the controller.
            discard_trailing_scrolls(&mut pane.pending_io);
            pane.pending_io
                .push_back(PendingTerminalIo::Scroll(TerminalScroll::Bottom));
            pane.viewport_may_be_scrolled = false;
            pane.viewport_reset_pending = true;
        }
        pane.pending_input_bytes += bytes.len();
        pane.pending_io.push_back(PendingTerminalIo::Input(bytes));
        self.flush_commands();
        Ok(())
    }

    pub(super) fn queue_scroll(
        &mut self,
        pane_id: TerminalPaneId,
        scroll: TerminalScroll,
    ) -> Result<(), TerminalError> {
        let Some(pane) = self.panes.get_mut(&pane_id) else {
            return Err(TerminalError::new(format!(
                "terminal pane {pane_id} is not running"
            )));
        };
        if pane.pending_close {
            return Err(TerminalError::new(format!(
                "terminal pane {pane_id} is closing"
            )));
        }
        if scroll == TerminalScroll::Bottom {
            discard_trailing_scrolls(&mut pane.pending_io);
            let pending_scrolls = pane
                .pending_io
                .iter()
                .filter(|io| matches!(io, PendingTerminalIo::Scroll(_)))
                .count();
            if pending_scrolls >= MAX_PENDING_SCROLL_COMMANDS {
                return Err(TerminalError::new(format!(
                    "native terminal pane {pane_id} scroll backlog exceeded {MAX_PENDING_SCROLL_COMMANDS} commands"
                )));
            }
            pane.pending_io
                .push_back(PendingTerminalIo::Scroll(TerminalScroll::Bottom));
            pane.viewport_may_be_scrolled = false;
            pane.viewport_reset_pending = true;
            self.flush_commands();
            return Ok(());
        }
        if let TerminalScroll::Lines(delta) = scroll {
            if let Some(PendingTerminalIo::Scroll(TerminalScroll::Lines(queued))) =
                pane.pending_io.back_mut()
            {
                *queued = queued.saturating_add(delta);
                let remove = *queued == 0;
                if remove {
                    pane.pending_io.pop_back();
                }
                // Keep this conservative even when two local deltas cancel:
                // an earlier scroll may already be inside the controller.
                pane.viewport_may_be_scrolled = true;
                pane.viewport_reset_pending = false;
                self.flush_commands();
                return Ok(());
            }
        }
        let pending_scrolls = pane
            .pending_io
            .iter()
            .filter(|io| matches!(io, PendingTerminalIo::Scroll(_)))
            .count();
        if pending_scrolls >= MAX_PENDING_SCROLL_COMMANDS {
            return Err(TerminalError::new(format!(
                "native terminal pane {pane_id} scroll backlog exceeded {MAX_PENDING_SCROLL_COMMANDS} commands"
            )));
        }
        pane.viewport_may_be_scrolled = true;
        pane.viewport_reset_pending = false;
        pane.pending_io.push_back(PendingTerminalIo::Scroll(scroll));
        self.flush_commands();
        Ok(())
    }

    pub(super) fn queue_resize(&mut self, pane_id: TerminalPaneId, size: TerminalSize) {
        let Some(pane) = self.panes.get_mut(&pane_id) else {
            return;
        };
        if pane.pending_close
            || pane.last_requested_size == Some(size)
            || pane.pending_resize == Some(size)
        {
            return;
        }
        // Resize is latest-state data. Superseding stale geometry is safe and
        // avoids a drag-resize waiting behind every intermediate viewport.
        pane.pending_resize = Some(size);
        if let Some(request) = pane.pending_open.as_mut() {
            request.size = size;
        }
    }

    pub(super) fn queue_label(&mut self, pane_id: TerminalPaneId, label: String) {
        let Some(pane) = self.panes.get_mut(&pane_id) else {
            return;
        };
        if pane.pending_close {
            return;
        }
        if let Some(request) = pane.pending_open.as_mut() {
            request.spec.label = label.clone();
        }
        pane.pending_label = Some(label);
        self.flush_commands();
    }

    pub(super) fn queue_close(&mut self, pane_id: TerminalPaneId) {
        let Some(pane) = self.panes.get_mut(&pane_id) else {
            return;
        };
        pane.pending_io.clear();
        pane.pending_input_bytes = 0;
        pane.pending_resize = None;
        pane.pending_label = None;
        pane.lifecycle = TerminalPaneLifecycle::Closing;
        if pane.epoch.is_none() {
            // An open command that never entered the controller can be canceled
            // locally without ever starting a child process.
            pane.pending_open = None;
            self.remove_runtime(pane_id);
            return;
        }
        pane.pending_close = true;
        self.flush_commands();
    }

    pub(super) fn poll_frames(&mut self) -> bool {
        self.flush_commands();
        let Some(frames) = self.frames.as_ref() else {
            return false;
        };
        let mut changed = false;
        for pane in self.panes.values_mut() {
            let Some(epoch) = pane.epoch else {
                continue;
            };
            let Some(published) = frames.latest_for_epoch(&pane.runtime_id, epoch) else {
                continue;
            };
            if published.revision == pane.revision {
                continue;
            }
            pane.revision = published.revision;
            pane.frame = published.frame.clone();
            if published
                .frame
                .as_ref()
                .is_some_and(|frame| frame.terminal.viewport.display_offset == 0)
            {
                pane.viewport_reset_pending = false;
            }
            pane.error = published.error.as_deref().map(str::to_string);
            pane.exit = published.exit;
            pane.lifecycle = if published.is_open {
                if published.exit.is_some() {
                    TerminalPaneLifecycle::Exited
                } else {
                    TerminalPaneLifecycle::Running
                }
            } else if published.error.is_some() {
                TerminalPaneLifecycle::Failed
            } else {
                TerminalPaneLifecycle::Opening
            };
            changed = true;
        }
        changed
    }

    pub(super) fn flush_commands(&mut self) {
        if self.controller.is_none() || self.flush_order.is_empty() {
            return;
        }

        loop {
            let order = self.flush_order.clone();
            if order.is_empty() {
                self.flush_cursor = 0;
                return;
            }
            let start = self.flush_cursor % order.len();
            let mut progressed = false;
            for offset in 0..order.len() {
                let index = (start + offset) % order.len();
                let pane_id = order[index];
                match self.flush_one(pane_id) {
                    FlushOutcome::Idle => {}
                    FlushOutcome::Progress => {
                        progressed = true;
                        self.advance_flush_cursor(pane_id);
                    }
                    FlushOutcome::QueueFull => {
                        self.set_flush_cursor(pane_id);
                        return;
                    }
                    FlushOutcome::Remove => {
                        progressed = true;
                        self.remove_runtime(pane_id);
                    }
                }
            }
            if !progressed {
                return;
            }
        }
    }

    fn flush_one(&mut self, pane_id: TerminalPaneId) -> FlushOutcome {
        let Some(controller) = self.controller.as_ref() else {
            return FlushOutcome::Idle;
        };
        let Some(pane) = self.panes.get_mut(&pane_id) else {
            return FlushOutcome::Remove;
        };

        if pane.pending_close {
            let Some(epoch) = pane.epoch else {
                return FlushOutcome::Remove;
            };
            return match controller.close(pane.runtime_id.clone(), epoch) {
                Ok(()) => FlushOutcome::Remove,
                Err(ControllerQueueError::Full(())) => FlushOutcome::QueueFull,
                Err(ControllerQueueError::Disconnected(())) => {
                    pane.remember_disconnected("closing a pane");
                    FlushOutcome::Remove
                }
            };
        }

        if let Some(request) = pane.pending_open.take() {
            return match controller.open_native(request) {
                Ok(epoch) => {
                    pane.epoch = Some(epoch);
                    FlushOutcome::Progress
                }
                Err(ControllerQueueError::Full(request)) => {
                    pane.pending_open = Some(request);
                    FlushOutcome::QueueFull
                }
                Err(ControllerQueueError::Disconnected(_)) => {
                    pane.remember_disconnected("opening a pane");
                    FlushOutcome::Progress
                }
            };
        }

        let Some(epoch) = pane.epoch else {
            return FlushOutcome::Idle;
        };

        if let Some(io) = pane.pending_io.pop_front() {
            return match io {
                PendingTerminalIo::Input(bytes) => {
                    pane.pending_input_bytes -= bytes.len();
                    match controller.input(pane.runtime_id.clone(), epoch, bytes) {
                        Ok(()) => FlushOutcome::Progress,
                        Err(ControllerQueueError::Full(bytes)) => {
                            pane.pending_input_bytes += bytes.len();
                            pane.pending_io.push_front(PendingTerminalIo::Input(bytes));
                            FlushOutcome::QueueFull
                        }
                        Err(ControllerQueueError::Disconnected(_)) => {
                            pane.remember_disconnected("sending input");
                            FlushOutcome::Progress
                        }
                    }
                }
                PendingTerminalIo::Scroll(scroll) => {
                    match controller.scroll(pane.runtime_id.clone(), epoch, scroll) {
                        Ok(()) => FlushOutcome::Progress,
                        Err(ControllerQueueError::Full(scroll)) => {
                            pane.pending_io
                                .push_front(PendingTerminalIo::Scroll(scroll));
                            FlushOutcome::QueueFull
                        }
                        Err(ControllerQueueError::Disconnected(_)) => {
                            pane.remember_disconnected("scrolling");
                            FlushOutcome::Progress
                        }
                    }
                }
            };
        }

        if let Some(label) = pane.pending_label.take() {
            return match controller.set_label(pane.runtime_id.clone(), epoch, label) {
                Ok(()) => FlushOutcome::Progress,
                Err(ControllerQueueError::Full(label)) => {
                    pane.pending_label = Some(label);
                    FlushOutcome::QueueFull
                }
                Err(ControllerQueueError::Disconnected(_)) => {
                    pane.remember_disconnected("renaming a pane");
                    FlushOutcome::Progress
                }
            };
        }

        if let Some(size) = pane.pending_resize.take() {
            return match controller.resize(pane.runtime_id.clone(), epoch, size) {
                Ok(()) => {
                    pane.last_requested_size = Some(size);
                    FlushOutcome::Progress
                }
                Err(ControllerQueueError::Full(size)) => {
                    pane.pending_resize = Some(size);
                    FlushOutcome::QueueFull
                }
                Err(ControllerQueueError::Disconnected(_)) => {
                    pane.remember_disconnected("resizing");
                    FlushOutcome::Progress
                }
            };
        }

        FlushOutcome::Idle
    }

    fn set_flush_cursor(&mut self, pane_id: TerminalPaneId) {
        self.flush_cursor = self
            .flush_order
            .iter()
            .position(|candidate| *candidate == pane_id)
            .unwrap_or(0);
    }

    fn advance_flush_cursor(&mut self, pane_id: TerminalPaneId) {
        if self.flush_order.is_empty() {
            self.flush_cursor = 0;
            return;
        }
        let current = self
            .flush_order
            .iter()
            .position(|candidate| *candidate == pane_id)
            .unwrap_or(0);
        self.flush_cursor = (current + 1) % self.flush_order.len();
    }

    fn remove_runtime(&mut self, pane_id: TerminalPaneId) {
        self.panes.remove(&pane_id);
        self.flush_order.retain(|candidate| *candidate != pane_id);
        if self.flush_order.is_empty() {
            self.flush_cursor = 0;
        } else {
            self.flush_cursor %= self.flush_order.len();
        }
    }
}

fn discard_trailing_scrolls(pending: &mut VecDeque<PendingTerminalIo>) {
    let keep = pending
        .iter()
        .rposition(|io| matches!(io, PendingTerminalIo::Input(_)))
        .map_or(0, |index| index + 1);
    pending.truncate(keep);
}

enum FlushOutcome {
    Idle,
    Progress,
    QueueFull,
    Remove,
}

fn native_request(definition: &TerminalPaneDefinition, size: TerminalSize) -> NativePaneRequest {
    let runtime_id = PaneId::new(format!("native-{}", definition.id.serial()));
    let transport_id = TransportId::new(format!("native:{}", definition.id.serial()));
    NativePaneRequest {
        spec: PaneSpec {
            id: runtime_id,
            label: definition.label.clone(),
            engine_id: EngineId::new(ALACRITTY_ENGINE_ID),
            transport_id,
        },
        size,
        command: native_command(&definition.command, &definition.cwd, definition.id),
    }
}

fn native_command(
    definition: &TerminalCommandDefinition,
    cwd: &Path,
    pane_id: TerminalPaneId,
) -> NativePtyCommand {
    let command = match definition.program.as_deref() {
        Some(program) => NativePtyCommand::new(program).args(&definition.args),
        None => NativePtyCommand::default_program(),
    };
    command
        .cwd(cwd)
        .env("PAD_PANE_ID", format!("native-{}", pane_id.serial()))
}
