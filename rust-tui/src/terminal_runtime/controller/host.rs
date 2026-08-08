use super::*;

enum PaneOperation {
    Open {
        epoch: PaneEpoch,
        spec: PaneSpec,
        size: TerminalSize,
        transport: Box<dyn SessionTransport>,
    },
    Input {
        epoch: PaneEpoch,
        bytes: Vec<u8>,
    },
    Resize {
        epoch: PaneEpoch,
        size: TerminalSize,
    },
    Scroll {
        epoch: PaneEpoch,
        scroll: TerminalScroll,
    },
    SetLabel {
        epoch: PaneEpoch,
        label: String,
    },
    Close {
        epoch: PaneEpoch,
    },
}

impl PaneOperation {
    fn epoch(&self) -> PaneEpoch {
        match self {
            Self::Open { epoch, .. }
            | Self::Input { epoch, .. }
            | Self::Resize { epoch, .. }
            | Self::Scroll { epoch, .. }
            | Self::SetLabel { epoch, .. }
            | Self::Close { epoch } => *epoch,
        }
    }
}

#[derive(Default)]
struct HostPane {
    active_epoch: Option<PaneEpoch>,
    pumpable: bool,
    pending: VecDeque<PaneOperation>,
}

pub(super) struct ControllerHost {
    runtime: LivePaneRuntime,
    published: Arc<RwLock<HashMap<PaneId, Arc<PublishedPane>>>>,
    panes: HashMap<PaneId, HostPane>,
    round_robin: VecDeque<PaneId>,
    pending_count: usize,
    pending_capacity: usize,
}

impl ControllerHost {
    pub(super) fn new(
        runtime: LivePaneRuntime,
        published: Arc<RwLock<HashMap<PaneId, Arc<PublishedPane>>>>,
        pending_capacity: usize,
    ) -> Self {
        Self {
            runtime,
            published,
            panes: HashMap::new(),
            round_robin: VecDeque::new(),
            pending_count: 0,
            pending_capacity,
        }
    }

    pub(super) fn run(
        mut self,
        commands: Receiver<ControllerCommand>,
        stopping: Arc<AtomicBool>,
    ) -> Result<(), TerminalError> {
        while !stopping.load(Ordering::Acquire) {
            if self.pending_count < self.pending_capacity {
                match commands.recv_timeout(HOST_POLL_INTERVAL) {
                    Ok(ControllerCommand::Shutdown) => break,
                    Ok(command) => self.enqueue(command),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            } else {
                // Keep retry latency low without spinning a CPU when a
                // transport deliberately stops consuming commands.
                thread::sleep(HOST_POLL_INTERVAL);
            }
            self.tick();
        }
        self.close_all();
        Ok(())
    }

    fn enqueue(&mut self, command: ControllerCommand) {
        let (pane_id, operation) = match command {
            ControllerCommand::OpenNative { epoch, request } => {
                let pane_id = request.spec.id.clone();
                let transport = NativePtyTransport::new(
                    request.spec.transport_id.clone(),
                    request.command,
                    request.size,
                );
                (
                    pane_id,
                    PaneOperation::Open {
                        epoch,
                        spec: request.spec,
                        size: request.size,
                        transport: Box::new(transport),
                    },
                )
            }
            #[cfg(test)]
            ControllerCommand::OpenTest {
                epoch,
                spec,
                size,
                transport,
            } => (
                spec.id.clone(),
                PaneOperation::Open {
                    epoch,
                    spec,
                    size,
                    transport,
                },
            ),
            ControllerCommand::Input {
                pane_id,
                epoch,
                bytes,
            } => (pane_id, PaneOperation::Input { epoch, bytes }),
            ControllerCommand::Resize {
                pane_id,
                epoch,
                size,
            } => (pane_id, PaneOperation::Resize { epoch, size }),
            ControllerCommand::Scroll {
                pane_id,
                epoch,
                scroll,
            } => (pane_id, PaneOperation::Scroll { epoch, scroll }),
            ControllerCommand::SetLabel {
                pane_id,
                epoch,
                label,
            } => (pane_id, PaneOperation::SetLabel { epoch, label }),
            ControllerCommand::Close { pane_id, epoch } => {
                (pane_id, PaneOperation::Close { epoch })
            }
            ControllerCommand::Shutdown => return,
        };

        let is_new = !self.panes.contains_key(&pane_id);
        self.panes
            .entry(pane_id.clone())
            .or_default()
            .pending
            .push_back(operation);
        self.pending_count += 1;
        if is_new {
            self.round_robin.push_back(pane_id);
        }
    }

    fn tick(&mut self) {
        let Some(pane_id) = self.round_robin.pop_front() else {
            return;
        };
        let operation = self
            .panes
            .get_mut(&pane_id)
            .and_then(|pane| pane.pending.pop_front());
        if operation.is_some() {
            self.pending_count -= 1;
        }

        if let Some(operation) = operation {
            if let Some(retry) = self.apply_operation(&pane_id, operation) {
                self.panes
                    .get_mut(&pane_id)
                    .expect("controller pane exists while retrying")
                    .pending
                    .push_front(retry);
                self.pending_count += 1;
            }
        }

        self.pump_pane(&pane_id);
        let retain = self
            .panes
            .get(&pane_id)
            .is_some_and(|pane| pane.active_epoch.is_some() || !pane.pending.is_empty());
        if retain {
            self.round_robin.push_back(pane_id);
        } else {
            self.panes.remove(&pane_id);
        }
    }

    /// Returns the operation only when downstream bounded backpressure needs
    /// a retry. Every other failure is published and consumed exactly once.
    fn apply_operation(
        &mut self,
        pane_id: &PaneId,
        operation: PaneOperation,
    ) -> Option<PaneOperation> {
        if matches!(operation, PaneOperation::Open { .. })
            && read_unpoisoned(&self.published)
                .get(pane_id)
                .is_some_and(|published| published.epoch > operation.epoch())
        {
            return None;
        }
        if !matches!(operation, PaneOperation::Open { .. })
            && self.panes.get(pane_id).and_then(|pane| pane.active_epoch) != Some(operation.epoch())
        {
            return None;
        }

        match operation {
            PaneOperation::Open {
                epoch,
                spec,
                size,
                transport,
            } => {
                if self
                    .panes
                    .get(pane_id)
                    .and_then(|pane| pane.active_epoch)
                    .is_some()
                {
                    let _ = self.runtime.close(pane_id);
                }
                self.publish_pending(pane_id, epoch);
                match self.runtime.open(spec, size, transport) {
                    Ok(()) => {
                        let pane = self
                            .panes
                            .get_mut(pane_id)
                            .expect("controller pane exists while opening");
                        pane.active_epoch = Some(epoch);
                        pane.pumpable = true;
                        self.publish_frame(pane_id, epoch, None, None, true);
                    }
                    Err(error) => {
                        let pane = self
                            .panes
                            .get_mut(pane_id)
                            .expect("controller pane exists after failed open");
                        pane.active_epoch = None;
                        pane.pumpable = false;
                        self.publish_error(pane_id, epoch, error, false);
                    }
                }
                None
            }
            PaneOperation::Input { epoch, bytes } => match self.runtime.input(pane_id, &bytes) {
                Ok(()) => None,
                Err(error) if is_command_backpressure(&error) => {
                    Some(PaneOperation::Input { epoch, bytes })
                }
                Err(error) => {
                    self.publish_error(pane_id, epoch, error, true);
                    None
                }
            },
            PaneOperation::Resize { epoch, size } => match self.runtime.resize(pane_id, size) {
                Ok(()) => None,
                Err(error) if is_command_backpressure(&error) => {
                    Some(PaneOperation::Resize { epoch, size })
                }
                Err(error) => {
                    self.publish_error(pane_id, epoch, error, true);
                    None
                }
            },
            PaneOperation::Scroll { epoch, scroll } => {
                match self.runtime.scroll(pane_id, scroll) {
                    Ok(()) => self.publish_frame(pane_id, epoch, None, None, true),
                    Err(error) => self.publish_error(pane_id, epoch, error, true),
                }
                None
            }
            PaneOperation::SetLabel { epoch, label } => {
                match self.runtime.set_label(pane_id, label) {
                    Ok(()) => self.publish_frame(pane_id, epoch, None, None, true),
                    Err(error) => self.publish_error(pane_id, epoch, error, true),
                }
                None
            }
            PaneOperation::Close { epoch } => {
                let result = self.runtime.close(pane_id);
                if let Some(pane) = self.panes.get_mut(pane_id) {
                    pane.active_epoch = None;
                    pane.pumpable = false;
                }
                match result {
                    Ok(()) => self.publish_frame(pane_id, epoch, None, None, false),
                    Err(error) => self.publish_error(pane_id, epoch, error, false),
                }
                None
            }
        }
    }

    fn pump_pane(&mut self, pane_id: &PaneId) {
        let Some(epoch) = self
            .panes
            .get(pane_id)
            .and_then(|pane| pane.pumpable.then_some(pane.active_epoch).flatten())
        else {
            return;
        };

        match self.runtime.pump(pane_id) {
            Ok(consumed) => {
                let _ = self.runtime.drain_host_events(pane_id);
                let exit = self.runtime.exit(pane_id);
                if consumed > 0 || exit.is_some() {
                    self.publish_frame(pane_id, epoch, None, exit, true);
                }
                if exit.is_some() {
                    if let Some(pane) = self.panes.get_mut(pane_id) {
                        pane.pumpable = false;
                    }
                }
            }
            Err(error) => {
                self.publish_error(pane_id, epoch, error, true);
                if let Some(pane) = self.panes.get_mut(pane_id) {
                    pane.pumpable = false;
                }
            }
        }
    }

    fn publish_pending(&self, pane_id: &PaneId, epoch: PaneEpoch) {
        let mut panes = write_unpoisoned(&self.published);
        if panes
            .get(pane_id)
            .is_some_and(|published| published.epoch > epoch)
        {
            return;
        }
        let revision = next_revision(panes.get(pane_id));
        panes.insert(
            pane_id.clone(),
            Arc::new(PublishedPane::pending(epoch, revision)),
        );
    }

    fn publish_frame(
        &self,
        pane_id: &PaneId,
        epoch: PaneEpoch,
        mut error: Option<Arc<str>>,
        exit: Option<TransportExit>,
        is_open: bool,
    ) {
        let frame = if is_open {
            match self.runtime.frame(pane_id) {
                Ok(frame) => Some(Arc::new(frame)),
                Err(snapshot_error) => {
                    error.get_or_insert_with(|| Arc::from(snapshot_error.to_string()));
                    None
                }
            }
        } else {
            None
        };
        self.publish(pane_id, epoch, frame, error, exit, is_open);
    }

    fn publish_error(
        &self,
        pane_id: &PaneId,
        epoch: PaneEpoch,
        error: TerminalError,
        is_open: bool,
    ) {
        let frame = if is_open {
            self.runtime.frame(pane_id).ok().map(Arc::new)
        } else {
            None
        };
        self.publish(
            pane_id,
            epoch,
            frame,
            Some(Arc::from(error.to_string())),
            None,
            is_open,
        );
    }

    fn publish(
        &self,
        pane_id: &PaneId,
        epoch: PaneEpoch,
        frame: Option<Arc<PaneFrame>>,
        error: Option<Arc<str>>,
        exit: Option<TransportExit>,
        is_open: bool,
    ) {
        let mut panes = write_unpoisoned(&self.published);
        if panes
            .get(pane_id)
            .is_some_and(|published| published.epoch > epoch)
        {
            return;
        }
        let current = panes
            .get(pane_id)
            .filter(|published| published.epoch == epoch);
        let error = error.or_else(|| current.and_then(|published| published.error.clone()));
        let exit = exit.or_else(|| current.and_then(|published| published.exit));
        let unchanged = current.is_some_and(|published| {
            published.epoch == epoch
                && published.frame == frame
                && published.error == error
                && published.exit == exit
                && published.is_open == is_open
        });
        if unchanged {
            return;
        }
        let revision = next_revision(panes.get(pane_id));
        panes.insert(
            pane_id.clone(),
            Arc::new(PublishedPane {
                epoch,
                revision,
                frame,
                error,
                exit,
                is_open,
            }),
        );
    }

    fn close_all(&mut self) {
        let open: Vec<_> = self
            .panes
            .iter()
            .filter_map(|(pane_id, pane)| pane.active_epoch.map(|epoch| (pane_id.clone(), epoch)))
            .collect();
        for (pane_id, epoch) in open {
            let result = self.runtime.close(&pane_id);
            match result {
                Ok(()) => self.publish_frame(&pane_id, epoch, None, None, false),
                Err(error) => self.publish_error(&pane_id, epoch, error, false),
            }
        }
    }
}
