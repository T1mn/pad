#[path = "live_pane_tests/support.rs"]
mod support;

use support::*;

use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::*;
use crate::terminal_runtime::{
    AlacrittyEngineFactory, EngineFactory, EngineId, EngineRegistry, EngineRuntime, ReplayStep,
    ReplayTransport, TerminalEngine, TerminalSnapshot, TransportId, ALACRITTY_ENGINE_ID,
};

const TEST_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) fn replay_output_is_pumped_into_the_terminal_snapshot_in_order() {
    let mut runtime = runtime();
    let pane_id = PaneId::new("output");
    let exit = successful_exit();
    open_replay(
        &mut runtime,
        &pane_id,
        "replay-output",
        [
            ReplayStep::output(b"hel"),
            ReplayStep::output(b"lo"),
            ReplayStep::exit(exit),
        ],
    )
    .unwrap();

    pump_until_exit(&mut runtime, &pane_id);

    let frame = runtime.frame(&pane_id).unwrap();
    assert_eq!(frame.terminal.row_text(0).as_deref(), Some("hello"));
    assert_eq!(runtime.exit(&pane_id), Some(exit));
}

pub(crate) fn parser_replies_are_routed_back_to_transport_in_order() {
    let mut runtime = runtime();
    let pane_id = PaneId::new("query-reply");
    open_replay(
        &mut runtime,
        &pane_id,
        "replay-query-reply",
        [
            ReplayStep::output(b"\x1b[6n"),
            ReplayStep::expect_input(b"\x1b[1;1R"),
            ReplayStep::exit(successful_exit()),
        ],
    )
    .unwrap();

    pump_until_exit(&mut runtime, &pane_id);

    assert!(runtime.drain_host_events(&pane_id).unwrap().is_empty());
}

pub(crate) fn parser_reply_survives_a_full_command_queue() {
    let (release, release_rx) = mpsc::sync_channel(1);
    let (observed_tx, observed) = mpsc::sync_channel(1);
    let transport_id = TransportId::new("reply-backpressure");
    let mut runtime = runtime_with_capacities(1, 8);
    let pane_id = PaneId::new("reply-backpressure");
    runtime
        .open(
            pane_spec(&pane_id, transport_id.as_str()),
            TerminalSize::new(20, 4),
            Box::new(ReplyBackpressureTransport {
                id: transport_id,
                release: release_rx,
                observed: observed_tx,
            }),
        )
        .unwrap();

    runtime.input(&pane_id, b"user-input").unwrap();
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        if runtime.pump(&pane_id).unwrap() == 1 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "query event was not produced while the command queue stayed full"
        );
        std::thread::yield_now();
    }
    release.send(()).unwrap();

    let deadline = Instant::now() + TEST_TIMEOUT;
    let commands = loop {
        runtime.pump(&pane_id).unwrap();
        if let Ok(commands) = observed.try_recv() {
            break commands;
        }
        assert!(Instant::now() < deadline, "parser reply was not retried");
        std::thread::yield_now();
    };
    assert_eq!(
        commands,
        vec![
            TransportCommand::Input(b"user-input".to_vec()),
            TransportCommand::Input(b"\x1b[1;1R".to_vec()),
        ]
    );
    pump_until_exit(&mut runtime, &pane_id);
}

pub(crate) fn host_title_and_bell_events_are_observable() {
    let mut runtime = runtime();
    let pane_id = PaneId::new("host-events");
    open_replay(
        &mut runtime,
        &pane_id,
        "replay-host-events",
        [
            ReplayStep::output(b"\x1b]0;build\x07\x07"),
            ReplayStep::exit(successful_exit()),
        ],
    )
    .unwrap();

    pump_until_exit(&mut runtime, &pane_id);

    assert_eq!(
        runtime.drain_host_events(&pane_id).unwrap(),
        vec![
            TerminalEngineEvent::Title(Some("build".to_string())),
            TerminalEngineEvent::Bell,
        ]
    );
}

pub(crate) fn repeated_title_and_bell_events_are_coalesced() {
    let mut runtime = runtime();
    let pane_id = PaneId::new("coalesced-host-events");
    let mut output = Vec::new();
    for index in 0..200 {
        output.extend_from_slice(format!("\x1b]0;title-{index}\x07\x07").as_bytes());
    }
    open_replay(
        &mut runtime,
        &pane_id,
        "replay-coalesced-host-events",
        [
            ReplayStep::output(output),
            ReplayStep::exit(successful_exit()),
        ],
    )
    .unwrap();

    pump_until_exit(&mut runtime, &pane_id);

    assert_eq!(
        runtime.drain_host_events(&pane_id).unwrap(),
        vec![
            TerminalEngineEvent::Title(Some("title-199".to_string())),
            TerminalEngineEvent::Bell,
        ]
    );
}

pub(crate) fn final_output_is_applied_before_transport_failure_surfaces() {
    let (release, release_rx) = mpsc::sync_channel(1);
    let mut runtime = runtime();
    let pane_id = PaneId::new("final-output-error");
    runtime
        .open(
            pane_spec(&pane_id, "delayed-failure"),
            TerminalSize::new(20, 4),
            Box::new(DelayedFailureTransport {
                id: TransportId::new("delayed-failure"),
                release: release_rx,
            }),
        )
        .unwrap();

    assert_eq!(runtime.pump(&pane_id).unwrap(), 0);
    release.send(()).unwrap();
    let error = pump_until_error(&mut runtime, &pane_id);

    assert_eq!(
        runtime
            .frame(&pane_id)
            .unwrap()
            .terminal
            .row_text(0)
            .as_deref(),
        Some("final-output")
    );
    assert!(error.to_string().contains("injected transport failure"));
}

pub(crate) fn drain_panic_becomes_a_stable_failure_and_rejects_new_commands() {
    let mut registry = EngineRegistry::default();
    registry.register(EngineId::new("drain-panic"), DrainPanicFactory);
    let engines = EngineRuntime::start(1, registry).unwrap();
    let mut runtime = LivePaneRuntime::new(
        PaneRuntime::new(engines),
        TransportRuntime::new(8, 8).unwrap(),
    );
    let pane_id = PaneId::new("drain-panic");
    let transport_id = TransportId::new("drain-panic-replay");
    runtime
        .open(
            PaneSpec {
                id: pane_id.clone(),
                label: "faulty parser".to_string(),
                engine_id: EngineId::new("drain-panic"),
                transport_id: transport_id.clone(),
            },
            TerminalSize::new(20, 4),
            Box::new(ReplayTransport::new(
                transport_id,
                [ReplayStep::output(b"trigger")],
            )),
        )
        .unwrap();

    let error = pump_until_error(&mut runtime, &pane_id);
    assert!(error.to_string().contains("drain events"));
    assert!(error.to_string().contains("drain exploded"));
    assert_eq!(runtime.pump(&pane_id), Err(error.clone()));
    assert_eq!(runtime.input(&pane_id, b"ignored"), Err(error.clone()));
    assert_eq!(
        runtime.resize(&pane_id, TerminalSize::new(30, 6)),
        Err(error)
    );

    // Host metadata may still be updated for an error placeholder, but
    // closing must remove every pane/transport record even though the
    // failed engine was already evicted by its worker.
    runtime.set_label(&pane_id, "failed parser").unwrap();
    assert!(runtime.close(&pane_id).is_err());
    assert!(runtime.metadata(&pane_id).is_none());
    assert!(runtime.pump(&pane_id).is_err());
}

pub(crate) fn input_and_resize_are_forwarded_while_the_engine_is_resized() {
    let mut runtime = runtime();
    let pane_id = PaneId::new("interactive");
    let resized = TerminalSize::new(31, 7);
    open_replay(
        &mut runtime,
        &pane_id,
        "replay-interactive",
        [
            ReplayStep::expect_input(b"status\r"),
            ReplayStep::expect_resize(resized),
            ReplayStep::resize_applied(resized),
            ReplayStep::output(b"resized"),
            ReplayStep::exit(successful_exit()),
        ],
    )
    .unwrap();

    runtime.input(&pane_id, b"status\r").unwrap();
    runtime.resize(&pane_id, resized).unwrap();
    pump_until_exit(&mut runtime, &pane_id);

    let snapshot = runtime.frame(&pane_id).unwrap().terminal;
    assert_eq!(snapshot.size, resized);
    assert_eq!(snapshot.row_text(0).as_deref(), Some("resized"));
}

pub(crate) fn pump_has_a_fixed_event_budget_and_coalesces_output() {
    let event_count = LivePaneRuntime::PUMP_EVENT_BUDGET + 3;
    let (mut runtime, operations) = recording_runtime(1, event_count + 1);
    let pane_id = PaneId::new("budget");
    let mut steps = vec![ReplayStep::output(b"x"); event_count];
    steps.push(ReplayStep::exit(successful_exit()));
    open_recording_replay(&mut runtime, &pane_id, "replay-budget", steps).unwrap();
    wait_for_transport_completion(&mut runtime, &pane_id);

    assert_eq!(
        runtime.pump(&pane_id).unwrap(),
        LivePaneRuntime::PUMP_EVENT_BUDGET
    );
    assert_eq!(
        operations.lock().unwrap().as_slice(),
        [format!(
            "feed:{}",
            "x".repeat(LivePaneRuntime::PUMP_EVENT_BUDGET)
        )]
    );
    assert_eq!(runtime.exit(&pane_id), None);

    assert_eq!(runtime.pump(&pane_id).unwrap(), 4);
    assert_eq!(
        operations.lock().unwrap().as_slice(),
        [
            format!("feed:{}", "x".repeat(LivePaneRuntime::PUMP_EVENT_BUDGET)),
            "feed:xxx".to_string(),
        ]
    );
    assert_eq!(runtime.exit(&pane_id), Some(successful_exit()));
}

pub(crate) fn resize_ack_orders_old_and_new_output_around_engine_resize() {
    let (mut runtime, operations) = recording_runtime(8, 8);
    let pane_id = PaneId::new("resize-order");
    let initial = TerminalSize::new(20, 4);
    let resized = TerminalSize::new(31, 7);
    open_recording_replay(
        &mut runtime,
        &pane_id,
        "replay-resize-order",
        [
            ReplayStep::output(b"old-1"),
            ReplayStep::output(b"old-2"),
            ReplayStep::expect_resize(resized),
            ReplayStep::resize_applied(resized),
            ReplayStep::output(b"new-1"),
            ReplayStep::output(b"new-2"),
            ReplayStep::exit(successful_exit()),
        ],
    )
    .unwrap();

    runtime.resize(&pane_id, resized).unwrap();
    assert_eq!(runtime.frame(&pane_id).unwrap().terminal.size, initial);
    pump_until_exit(&mut runtime, &pane_id);

    assert_eq!(
        operations.lock().unwrap().as_slice(),
        ["feed:old-1old-2", "resize:31x7", "feed:new-1new-2"]
    );
    assert_eq!(runtime.frame(&pane_id).unwrap().terminal.size, resized);
}

pub(crate) fn duplicate_and_out_of_order_resize_acks_fail_deterministically() {
    let first = TerminalSize::new(30, 6);
    let second = TerminalSize::new(40, 8);
    let (mut runtime, _) = recording_runtime(8, 8);
    let pane_id = PaneId::new("out-of-order");
    open_recording_replay(
        &mut runtime,
        &pane_id,
        "replay-out-of-order",
        [
            ReplayStep::expect_resize(first),
            ReplayStep::expect_resize(second),
            ReplayStep::resize_applied(second),
        ],
    )
    .unwrap();
    runtime.resize(&pane_id, first).unwrap();
    runtime.resize(&pane_id, second).unwrap();
    let error = pump_until_error(&mut runtime, &pane_id);
    assert!(error.to_string().contains("out-of-order"));
    assert_eq!(runtime.pump(&pane_id), Err(error));
    assert_eq!(
        runtime.frame(&pane_id).unwrap().terminal.size,
        TerminalSize::new(20, 4)
    );

    let (mut runtime, _) = recording_runtime(8, 8);
    let pane_id = PaneId::new("duplicate");
    open_recording_replay(
        &mut runtime,
        &pane_id,
        "replay-duplicate",
        [
            ReplayStep::expect_resize(first),
            ReplayStep::resize_applied(first),
            ReplayStep::resize_applied(first),
        ],
    )
    .unwrap();
    runtime.resize(&pane_id, first).unwrap();
    let error = pump_until_error(&mut runtime, &pane_id);
    assert!(error.to_string().contains("no pending resize"));
    assert_eq!(runtime.pump(&pane_id), Err(error));
    assert_eq!(runtime.frame(&pane_id).unwrap().terminal.size, first);
}

pub(crate) fn saturated_and_disconnected_command_queues_return_without_blocking() {
    let mut runtime = runtime_with_capacities(1, 1);
    let pane_id = PaneId::new("saturated");
    open_replay(
        &mut runtime,
        &pane_id,
        "replay-saturated",
        [
            ReplayStep::output(b"blocks-next-event"),
            ReplayStep::output(b"blocks-command-consumer"),
            ReplayStep::expect_input(b"queued"),
        ],
    )
    .unwrap();
    runtime.input(&pane_id, b"queued").unwrap();

    assert_eq!(
        runtime.input(&pane_id, b"full").unwrap_err().to_string(),
        "terminal transport 'replay-saturated' command queue is full"
    );
    assert_eq!(
        runtime
            .resize(&pane_id, TerminalSize::new(30, 6))
            .unwrap_err()
            .to_string(),
        "terminal transport 'replay-saturated' command queue is full"
    );
    assert_eq!(
        runtime.frame(&pane_id).unwrap().terminal.size,
        TerminalSize::new(20, 4)
    );
    runtime.close(&pane_id).unwrap();

    let mut runtime = runtime_with_capacities(1, 1);
    let pane_id = PaneId::new("disconnected");
    open_replay(&mut runtime, &pane_id, "replay-disconnected", []).unwrap();
    wait_for_transport_completion(&mut runtime, &pane_id);
    assert_eq!(
        runtime.input(&pane_id, &[]).unwrap_err().to_string(),
        "terminal transport 'replay-disconnected' command channel disconnected"
    );
}

pub(crate) fn exit_is_stored_without_removing_the_pane() {
    let mut runtime = runtime();
    let pane_id = PaneId::new("exit");
    let exit = TransportExit {
        code: None,
        signaled: true,
    };
    open_replay(
        &mut runtime,
        &pane_id,
        "replay-exit",
        [ReplayStep::exit(exit)],
    )
    .unwrap();

    pump_until_exit(&mut runtime, &pane_id);

    assert_eq!(runtime.exit(&pane_id), Some(exit));
    assert!(runtime.frame(&pane_id).is_ok());
}

pub(crate) fn successful_completion_without_exit_event_is_still_observable() {
    let mut runtime = runtime();
    let pane_id = PaneId::new("implicit-exit");
    open_replay(&mut runtime, &pane_id, "replay-implicit-exit", []).unwrap();

    pump_until_exit(&mut runtime, &pane_id);

    assert_eq!(
        runtime.exit(&pane_id),
        Some(TransportExit {
            code: None,
            signaled: false,
        })
    );
}

pub(crate) fn replay_mismatch_surfaces_worker_error_and_keeps_pane_accessible() {
    let mut runtime = runtime();
    let pane_id = PaneId::new("mismatch");
    open_replay(
        &mut runtime,
        &pane_id,
        "replay-mismatch",
        [ReplayStep::expect_input(b"expected")],
    )
    .unwrap();

    runtime.input(&pane_id, b"actual").unwrap();
    let error = pump_until_error(&mut runtime, &pane_id);

    assert!(error.to_string().contains("expected command"));
    assert!(runtime.frame(&pane_id).is_ok());
    runtime.close(&pane_id).unwrap();
}

pub(crate) fn mismatch_duplicate_and_missing_pane_errors_do_not_change_state() {
    let mut runtime = runtime();
    let pane_id = PaneId::new("validation");
    let mismatch = runtime.open(
        pane_spec(&pane_id, "declared"),
        TerminalSize::new(20, 4),
        Box::new(ReplayTransport::new(TransportId::new("actual"), [])),
    );
    assert!(mismatch
        .unwrap_err()
        .to_string()
        .contains("expects transport"));
    assert!(runtime.metadata(&pane_id).is_none());

    open_replay(
        &mut runtime,
        &pane_id,
        "replay-validation",
        [ReplayStep::expect_shutdown()],
    )
    .unwrap();
    let duplicate = runtime.open(
        pane_spec(&pane_id, "replay-validation"),
        TerminalSize::new(20, 4),
        Box::new(ReplayTransport::new(
            TransportId::new("replay-validation"),
            [],
        )),
    );
    assert_eq!(
        duplicate.unwrap_err().to_string(),
        "terminal pane 'validation' is already registered"
    );
    assert_eq!(runtime.metadata(&pane_id).unwrap().label, "Codex");

    let missing = PaneId::new("missing");
    assert_eq!(
        runtime.pump(&missing).unwrap_err().to_string(),
        "terminal pane 'missing' is not registered"
    );
    assert!(runtime.input(&missing, &[]).is_err());
    assert!(runtime.resize(&missing, TerminalSize::new(1, 1)).is_err());
    assert!(runtime.frame(&missing).is_err());
    assert!(runtime.set_label(&missing, "none").is_err());
    assert!(runtime.close(&missing).is_err());
}

pub(crate) fn close_removes_engine_metadata_and_transport_together() {
    let mut runtime = runtime();
    let pane_id = PaneId::new("close");
    open_replay(
        &mut runtime,
        &pane_id,
        "replay-close",
        [ReplayStep::expect_shutdown()],
    )
    .unwrap();
    runtime.set_label(&pane_id, "Renamed").unwrap();

    runtime.close(&pane_id).unwrap();

    assert!(runtime.metadata(&pane_id).is_none());
    assert_eq!(runtime.exit(&pane_id), None);
    assert!(runtime.frame(&pane_id).is_err());
    assert!(runtime.close(&pane_id).is_err());
}
