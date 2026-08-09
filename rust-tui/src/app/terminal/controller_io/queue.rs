use super::*;

impl TerminalUiState {
    pub(in crate::app::terminal) fn queue_input(
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

    pub(in crate::app::terminal) fn queue_scroll(
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

    pub(in crate::app::terminal) fn queue_resize(
        &mut self,
        pane_id: TerminalPaneId,
        size: TerminalSize,
    ) {
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

    pub(in crate::app::terminal) fn queue_label(&mut self, pane_id: TerminalPaneId, label: String) {
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

    pub(in crate::app::terminal) fn queue_close(&mut self, pane_id: TerminalPaneId) {
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
}
