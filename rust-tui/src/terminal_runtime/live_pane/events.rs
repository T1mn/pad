use super::*;

impl LivePaneRuntime {
    pub(super) fn flush_output(
        &mut self,
        pane_id: &PaneId,
        output: &mut Vec<u8>,
    ) -> Result<(), TerminalError> {
        if output.is_empty() {
            return Ok(());
        }
        if let Err(error) = self.panes.feed_output(pane_id, std::mem::take(output)) {
            self.remember_failure(pane_id, error.clone());
            return Err(error);
        }
        self.collect_engine_events(pane_id)?;
        Ok(())
    }

    pub(super) fn collect_engine_events(&mut self, pane_id: &PaneId) -> Result<(), TerminalError> {
        let events = match self.panes.drain_engine_events(pane_id) {
            Ok(events) => events,
            Err(error) => {
                self.remember_failure(pane_id, error.clone());
                return Err(error);
            }
        };
        let live = self
            .transports
            .get_mut(pane_id)
            .expect("live transport was checked above");
        for event in events {
            match event {
                TerminalEngineEvent::PtyWrite(bytes) => {
                    if live.pending_pty_write_bytes.saturating_add(bytes.len())
                        > Self::MAX_PENDING_PTY_WRITE_BYTES
                    {
                        let error = TerminalError::new(format!(
                            "terminal pane '{pane_id}' parser reply backlog exceeded {} bytes",
                            Self::MAX_PENDING_PTY_WRITE_BYTES
                        ));
                        live.failure.get_or_insert(error.clone());
                        return Err(error);
                    }
                    live.pending_pty_write_bytes += bytes.len();
                    live.pending_pty_writes.push_back(bytes);
                }
                event => Self::queue_host_event(live, event),
            }
        }
        Ok(())
    }

    fn queue_host_event(live: &mut LiveTransport, event: TerminalEngineEvent) {
        match event {
            TerminalEngineEvent::Title(title) => {
                if let Some(index) = live
                    .host_events
                    .iter()
                    .position(|event| matches!(event, TerminalEngineEvent::Title(_)))
                {
                    live.host_events[index] = TerminalEngineEvent::Title(title);
                } else {
                    live.host_events
                        .push_back(TerminalEngineEvent::Title(title));
                }
            }
            TerminalEngineEvent::Bell => {
                if !live
                    .host_events
                    .iter()
                    .any(|event| matches!(event, TerminalEngineEvent::Bell))
                {
                    live.host_events.push_back(TerminalEngineEvent::Bell);
                }
            }
            TerminalEngineEvent::Exit => {
                if !live
                    .host_events
                    .iter()
                    .any(|event| matches!(event, TerminalEngineEvent::Exit))
                {
                    live.host_events.push_back(TerminalEngineEvent::Exit);
                }
            }
            TerminalEngineEvent::UnsupportedRequest(request) => {
                if live.host_events.iter().any(|event| {
                    matches!(event, TerminalEngineEvent::UnsupportedRequest(existing) if existing == &request)
                }) {
                    return;
                }
                if live.host_events.len() < Self::MAX_PENDING_HOST_EVENTS {
                    live.host_events
                        .push_back(TerminalEngineEvent::UnsupportedRequest(request));
                } else if let Some(index) = live
                    .host_events
                    .iter()
                    .position(|event| matches!(event, TerminalEngineEvent::UnsupportedRequest(_)))
                {
                    live.host_events[index] = TerminalEngineEvent::UnsupportedRequest(
                        "additional terminal requests were coalesced".to_string(),
                    );
                }
            }
            TerminalEngineEvent::PtyWrite(_) => {
                unreachable!("PTY writes are queued before host-event coalescing")
            }
        }
    }

    /// Returns false when bounded command backpressure requires a later pump.
    pub(super) fn flush_pending_pty_writes(
        &mut self,
        pane_id: &PaneId,
    ) -> Result<bool, TerminalError> {
        loop {
            let Some(bytes) = self
                .transports
                .get_mut(pane_id)
                .expect("live transport was checked above")
                .pending_pty_writes
                .pop_front()
            else {
                return Ok(true);
            };
            self.transports
                .get_mut(pane_id)
                .expect("live transport was checked above")
                .pending_pty_write_bytes -= bytes.len();
            let result = self
                .transports
                .get(pane_id)
                .expect("live transport was checked above")
                .handle
                .try_send(TransportCommand::Input(bytes));
            match result {
                Ok(()) => {}
                Err(TrySendError::Full(TransportCommand::Input(bytes))) => {
                    let live = self
                        .transports
                        .get_mut(pane_id)
                        .expect("live transport was checked above");
                    live.pending_pty_write_bytes += bytes.len();
                    live.pending_pty_writes.push_front(bytes);
                    return Ok(false);
                }
                Err(TrySendError::Disconnected(TransportCommand::Input(bytes))) => {
                    let live = self
                        .transports
                        .get_mut(pane_id)
                        .expect("live transport was checked above");
                    live.pending_pty_write_bytes += bytes.len();
                    live.pending_pty_writes.push_front(bytes);
                    let error = TerminalError::new(format!(
                        "terminal transport '{}' command channel disconnected while routing parser reply",
                        self.transports
                            .get(pane_id)
                            .expect("live transport was checked above")
                            .handle
                            .id()
                            .as_str()
                    ));
                    self.remember_failure(pane_id, error.clone());
                    return Err(error);
                }
                Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                    unreachable!("parser replies are always transport input commands")
                }
            }
        }
    }
}
