use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::{Duration, Instant};

use super::*;
use crate::terminal_runtime::{
    AlacrittyEngineFactory, EngineId, EngineRegistry, EngineRuntime, PaneRuntime, ReplayStep,
    ReplayTransport, SessionTransport, TransportCommand, TransportEvent, TransportId,
    TransportRuntime, ALACRITTY_ENGINE_ID,
};

const TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) fn replay_frames_and_exit_are_published_without_ui_runtime_calls() {
    let controller = controller(8, 8);
    assert_eq!(controller.host_thread_name(), "pad-terminal-controller");
    let reader = controller.frames();
    let pane_id = PaneId::new("published");
    let exit = successful_exit();
    let epoch = open_replay(
        &controller,
        &pane_id,
        "published-replay",
        [ReplayStep::output(b"ready"), ReplayStep::exit(exit)],
    );

    let published = wait_for(&reader, &pane_id, |pane| pane.exit == Some(exit));
    assert_eq!(published.epoch, epoch);
    assert_eq!(
        published
            .frame
            .as_ref()
            .unwrap()
            .terminal
            .row_text(0)
            .as_deref(),
        Some("ready")
    );
    assert!(published.revision >= 2);
    controller.shutdown().unwrap();
}

pub(crate) fn scroll_publishes_a_new_immutable_frame_without_transport_output() {
    let controller = controller(8, 8);
    let reader = controller.frames();
    let pane_id = PaneId::new("scrollback");
    let output = (0..=30)
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\r\n")
        .into_bytes();
    let epoch = open_replay(
        &controller,
        &pane_id,
        "scrollback-replay",
        [
            ReplayStep::output(output),
            ReplayStep::exit(successful_exit()),
        ],
    );
    let bottom = wait_for(&reader, &pane_id, |pane| pane.exit.is_some());
    let bottom_revision = bottom.revision;
    assert_eq!(
        bottom
            .frame
            .as_ref()
            .unwrap()
            .terminal
            .viewport
            .display_offset,
        0
    );

    controller
        .scroll(pane_id.clone(), epoch, TerminalScroll::Lines(3))
        .unwrap();
    let scrolled = wait_for(&reader, &pane_id, |pane| {
        pane.frame
            .as_ref()
            .is_some_and(|frame| frame.terminal.viewport.display_offset == 3)
    });

    assert!(scrolled.revision > bottom_revision);
    assert_eq!(
        scrolled
            .frame
            .as_ref()
            .unwrap()
            .terminal
            .row_text(0)
            .as_deref(),
        Some("4")
    );
    controller.shutdown().unwrap();
}

pub(crate) fn transport_failure_is_published_with_the_last_frame() {
    let controller = controller(8, 8);
    let reader = controller.frames();
    let pane_id = PaneId::new("failure");
    let epoch = open_replay(
        &controller,
        &pane_id,
        "failure-replay",
        [
            ReplayStep::output(b"prompt"),
            ReplayStep::expect_input(b"expected"),
        ],
    );
    wait_for(&reader, &pane_id, |pane| {
        pane.frame
            .as_ref()
            .is_some_and(|frame| frame.terminal.row_text(0).as_deref() == Some("prompt"))
    });
    controller
        .input(pane_id.clone(), epoch, b"wrong".to_vec())
        .unwrap();

    let failed = wait_for(&reader, &pane_id, |pane| pane.error.is_some());
    assert!(failed
        .error
        .as_deref()
        .unwrap()
        .contains("expected command"));
    assert_eq!(
        failed
            .frame
            .as_ref()
            .unwrap()
            .terminal
            .row_text(0)
            .as_deref(),
        Some("prompt")
    );
    controller.shutdown().unwrap();
}

pub(crate) fn downstream_input_and_resize_backpressure_is_retried_in_order() {
    let controller = controller(16, 1);
    let pane_id = PaneId::new("retry");
    let (release, release_rx) = mpsc::sync_channel(1);
    let (observed_tx, observed) = mpsc::sync_channel(1);
    let size = TerminalSize::new(100, 30);
    let transport_id = TransportId::new("retry-transport");
    let epoch = controller
        .open_test_transport(
            pane_spec(&pane_id, transport_id.as_str()),
            TerminalSize::new(80, 24),
            Box::new(GatedCommandTransport {
                id: transport_id,
                release: release_rx,
                observed: observed_tx,
            }),
        )
        .unwrap();
    wait_for(&controller.frames(), &pane_id, |pane| pane.is_open);

    controller
        .input(pane_id.clone(), epoch, b"first".to_vec())
        .unwrap();
    controller.resize(pane_id.clone(), epoch, size).unwrap();
    release.send(()).unwrap();

    assert_eq!(
        observed.recv_timeout(TIMEOUT).unwrap(),
        vec![
            TransportCommand::Input(b"first".to_vec()),
            TransportCommand::Resize(size),
        ]
    );
    let published = wait_for(&controller.frames(), &pane_id, |pane| {
        pane.frame
            .as_ref()
            .is_some_and(|frame| frame.terminal.size == size)
    });
    assert_eq!(published.epoch, epoch);
    controller.shutdown().unwrap();
}

pub(crate) fn controller_queue_backpressure_returns_original_input() {
    let (release, gate) = mpsc::sync_channel(1);
    let controller = TerminalController::start_inner(runtime(8), 1, Some(gate)).unwrap();
    let pane_id = PaneId::new("bounded");
    let epoch = PaneEpoch(99);
    controller
        .input(pane_id.clone(), epoch, b"first".to_vec())
        .unwrap();
    let error = controller
        .input(pane_id, epoch, b"must-retry".to_vec())
        .unwrap_err();
    assert!(error.is_full());
    assert_eq!(error.into_inner(), b"must-retry".to_vec());

    release.send(()).unwrap();
    controller.shutdown().unwrap();
}

pub(crate) fn controller_queue_backpressure_returns_original_scroll() {
    let (release, gate) = mpsc::sync_channel(1);
    let controller = TerminalController::start_inner(runtime(8), 1, Some(gate)).unwrap();
    let pane_id = PaneId::new("bounded-scroll");
    let epoch = PaneEpoch(100);
    controller
        .input(pane_id.clone(), epoch, b"occupy-queue".to_vec())
        .unwrap();
    let scroll = TerminalScroll::Lines(7);
    let error = controller.scroll(pane_id, epoch, scroll).unwrap_err();
    assert!(error.is_full());
    assert_eq!(error.into_inner(), scroll);

    release.send(()).unwrap();
    controller.shutdown().unwrap();
}

pub(crate) fn round_robin_keeps_a_quiet_pane_moving_during_noisy_output() {
    let controller = controller(32, 8);
    let reader = controller.frames();
    let noisy_id = PaneId::new("noisy");
    let quiet_id = PaneId::new("quiet");
    let noisy_steps = (0..(LivePaneRuntime::PUMP_EVENT_BUDGET * 4))
        .map(|_| ReplayStep::output(b"x"))
        .chain([ReplayStep::exit(successful_exit())]);
    open_replay(&controller, &noisy_id, "noisy-replay", noisy_steps);
    let quiet_epoch = open_replay(
        &controller,
        &quiet_id,
        "quiet-replay",
        [
            ReplayStep::output(b"quiet-ready"),
            ReplayStep::exit(successful_exit()),
        ],
    );

    let quiet = wait_for(&reader, &quiet_id, |pane| pane.exit.is_some());
    assert_eq!(quiet.epoch, quiet_epoch);
    assert_eq!(
        quiet
            .frame
            .as_ref()
            .unwrap()
            .terminal
            .row_text(0)
            .as_deref(),
        Some("quiet-ready")
    );
    controller.shutdown().unwrap();
}

pub(crate) fn stale_epoch_commands_cannot_mutate_a_reopened_pane() {
    let controller = controller(16, 8);
    let reader = controller.frames();
    let pane_id = PaneId::new("reopen");
    let old_epoch = open_replay(
        &controller,
        &pane_id,
        "old-replay",
        [
            ReplayStep::output(b"old"),
            ReplayStep::exit(successful_exit()),
        ],
    );
    wait_for(&reader, &pane_id, |pane| pane.exit.is_some());
    controller.close(pane_id.clone(), old_epoch).unwrap();

    let new_epoch = open_replay(
        &controller,
        &pane_id,
        "new-replay",
        [
            ReplayStep::output(b"new"),
            ReplayStep::exit(successful_exit()),
        ],
    );
    controller
        .set_label(pane_id.clone(), old_epoch, "stale".to_string())
        .unwrap();

    let published = wait_for(&reader, &pane_id, |pane| {
        pane.epoch == new_epoch && pane.exit.is_some()
    });
    assert_eq!(published.epoch, new_epoch);
    assert_eq!(published.frame.as_ref().unwrap().metadata.label, "Codex");
    assert_eq!(
        published
            .frame
            .as_ref()
            .unwrap()
            .terminal
            .row_text(0)
            .as_deref(),
        Some("new")
    );
    controller.shutdown().unwrap();
}

pub(crate) fn delayed_older_open_cannot_replace_a_newer_epoch() {
    let controller = controller(16, 8);
    let reader = controller.frames();
    let pane_id = PaneId::new("out-of-order-open");
    let old_epoch = controller.allocate_epoch();
    let new_epoch = controller.allocate_epoch();
    let new_transport_id = "newer-open";
    let old_transport_id = "older-open";

    assert!(controller
        .try_send(ControllerCommand::OpenTest {
            epoch: new_epoch,
            spec: pane_spec(&pane_id, new_transport_id),
            size: TerminalSize::new(80, 24),
            transport: Box::new(ReplayTransport::new(
                TransportId::new(new_transport_id),
                [
                    ReplayStep::output(b"newer"),
                    ReplayStep::exit(successful_exit()),
                ],
            )),
        })
        .is_ok());
    assert!(controller
        .try_send(ControllerCommand::OpenTest {
            epoch: old_epoch,
            spec: pane_spec(&pane_id, old_transport_id),
            size: TerminalSize::new(80, 24),
            transport: Box::new(ReplayTransport::new(
                TransportId::new(old_transport_id),
                [
                    ReplayStep::output(b"older"),
                    ReplayStep::exit(successful_exit()),
                ],
            )),
        })
        .is_ok());
    controller
        .set_label(pane_id.clone(), new_epoch, "new generation".to_string())
        .unwrap();

    let published = wait_for(&reader, &pane_id, |pane| {
        pane.epoch == new_epoch
            && pane
                .frame
                .as_ref()
                .is_some_and(|frame| frame.metadata.label == "new generation")
    });
    assert_eq!(
        published
            .frame
            .as_ref()
            .unwrap()
            .terminal
            .row_text(0)
            .as_deref(),
        Some("newer")
    );
    controller.shutdown().unwrap();
}

pub(crate) fn label_and_close_publish_new_revisions() {
    let controller = controller(8, 8);
    let reader = controller.frames();
    let pane_id = PaneId::new("lifecycle");
    let epoch = open_replay(
        &controller,
        &pane_id,
        "lifecycle-replay",
        [ReplayStep::output(b"running")],
    );
    let opened = wait_for(&reader, &pane_id, |pane| pane.is_open);

    controller
        .set_label(pane_id.clone(), epoch, "Claude · review".to_string())
        .unwrap();
    let labeled = wait_for(&reader, &pane_id, |pane| {
        pane.frame
            .as_ref()
            .is_some_and(|frame| frame.metadata.label == "Claude · review")
    });
    assert!(labeled.revision > opened.revision);

    controller.close(pane_id.clone(), epoch).unwrap();
    let closed = wait_for(&reader, &pane_id, |pane| !pane.is_open);
    assert!(closed.frame.is_none());
    assert!(closed.revision > labeled.revision);
    controller.shutdown().unwrap();
}

pub(crate) fn drop_is_nonblocking_while_explicit_shutdown_joins() {
    let dropped_controller = controller(8, 8);
    let reader = dropped_controller.frames();
    let pane_id = PaneId::new("drop");
    open_replay(
        &dropped_controller,
        &pane_id,
        "drop-replay",
        [ReplayStep::output(b"active")],
    );
    wait_for(&reader, &pane_id, |pane| pane.is_open);
    let started = Instant::now();
    drop(dropped_controller);
    assert!(started.elapsed() < Duration::from_millis(50));

    let controller = controller(8, 8);
    controller.shutdown().unwrap();
}

fn controller(command_capacity: usize, transport_capacity: usize) -> TerminalController {
    TerminalController::start_with_capacity(runtime(transport_capacity), command_capacity).unwrap()
}

fn runtime(transport_capacity: usize) -> LivePaneRuntime {
    let mut registry = EngineRegistry::default();
    registry.register(EngineId::new(ALACRITTY_ENGINE_ID), AlacrittyEngineFactory);
    let engines = EngineRuntime::start(2, registry).unwrap();
    LivePaneRuntime::new(
        PaneRuntime::new(engines),
        TransportRuntime::new(transport_capacity, 8).unwrap(),
    )
}

fn open_replay(
    controller: &TerminalController,
    pane_id: &PaneId,
    transport_id: &str,
    steps: impl IntoIterator<Item = ReplayStep>,
) -> PaneEpoch {
    controller
        .open_test_transport(
            pane_spec(pane_id, transport_id),
            TerminalSize::new(80, 24),
            Box::new(ReplayTransport::new(TransportId::new(transport_id), steps)),
        )
        .unwrap()
}

fn pane_spec(pane_id: &PaneId, transport_id: &str) -> PaneSpec {
    PaneSpec {
        id: pane_id.clone(),
        label: "Codex".to_string(),
        engine_id: EngineId::new(ALACRITTY_ENGINE_ID),
        transport_id: TransportId::new(transport_id),
    }
}

fn successful_exit() -> TransportExit {
    TransportExit {
        code: Some(0),
        signaled: false,
    }
}

fn wait_for(
    reader: &TerminalFrameReader,
    pane_id: &PaneId,
    condition: impl Fn(&PublishedPane) -> bool,
) -> Arc<PublishedPane> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(pane) = reader.latest(pane_id) {
            if condition(&pane) {
                return pane;
            }
        }
        assert!(Instant::now() < deadline, "controller state timed out");
        thread::yield_now();
    }
}

struct GatedCommandTransport {
    id: TransportId,
    release: Receiver<()>,
    observed: SyncSender<Vec<TransportCommand>>,
}

impl SessionTransport for GatedCommandTransport {
    fn id(&self) -> &TransportId {
        &self.id
    }

    fn run(
        self: Box<Self>,
        commands: Receiver<TransportCommand>,
        events: SyncSender<TransportEvent>,
    ) -> Result<(), TerminalError> {
        self.release
            .recv()
            .map_err(|_| TerminalError::new("release disconnected"))?;
        let input = commands
            .recv()
            .map_err(|_| TerminalError::new("input disconnected"))?;
        let resize = commands
            .recv()
            .map_err(|_| TerminalError::new("resize disconnected"))?;
        let size = match resize {
            TransportCommand::Resize(size) => size,
            other => {
                return Err(TerminalError::new(format!(
                    "expected resize, received {other:?}"
                )))
            }
        };
        self.observed
            .send(vec![input, TransportCommand::Resize(size)])
            .map_err(|_| TerminalError::new("observer disconnected"))?;
        events
            .send(TransportEvent::ResizeApplied(size))
            .map_err(|_| TerminalError::new("event receiver disconnected"))?;
        events
            .send(TransportEvent::Exited(successful_exit()))
            .map_err(|_| TerminalError::new("event receiver disconnected"))?;
        Ok(())
    }
}
