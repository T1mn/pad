use std::sync::{Arc, Barrier};
use std::thread;

use super::{
    AlacrittyEngineFactory, EngineId, EngineRegistry, EngineRuntime, PaneId, TerminalSize,
    ALACRITTY_ENGINE_ID,
};

const PANE_COUNT: usize = 8;
const WRITES_PER_PANE: usize = 250;

#[test]
fn eight_panes_process_output_resize_snapshot_and_close_concurrently() {
    let mut registry = EngineRegistry::default();
    registry.register(EngineId::new(ALACRITTY_ENGINE_ID), AlacrittyEngineFactory);
    let runtime = Arc::new(EngineRuntime::start(4, registry).unwrap());
    let panes: Vec<_> = (0..PANE_COUNT)
        .map(|index| PaneId::new(format!("stress-pane-{index}")))
        .collect();
    for pane_id in &panes {
        runtime
            .open(
                pane_id.clone(),
                EngineId::new(ALACRITTY_ENGINE_ID),
                TerminalSize::new(80, 24),
            )
            .unwrap();
    }

    let start = Arc::new(Barrier::new(PANE_COUNT + 1));
    let workers: Vec<_> = panes
        .iter()
        .enumerate()
        .map(|(pane_index, pane_id)| {
            let runtime = runtime.clone();
            let start = start.clone();
            let pane_id = pane_id.clone();
            thread::spawn(move || {
                start.wait();
                for sequence in 0..WRITES_PER_PANE {
                    if sequence % 50 == 0 {
                        let columns = if sequence % 100 == 0 { 64 } else { 96 };
                        runtime
                            .resize(&pane_id, TerminalSize::new(columns, 24))
                            .unwrap();
                    }
                    runtime
                        .feed(
                            &pane_id,
                            format!("pane-{pane_index}:{sequence}\r\n").into_bytes(),
                        )
                        .unwrap();
                }
                runtime.resize(&pane_id, TerminalSize::new(80, 24)).unwrap();
                runtime
                    .feed(
                        &pane_id,
                        format!("FINAL-PANE-{pane_index}\r\n").into_bytes(),
                    )
                    .unwrap();
            })
        })
        .collect();

    start.wait();
    for worker in workers {
        worker.join().unwrap();
    }

    for (pane_index, pane_id) in panes.iter().enumerate() {
        let snapshot = runtime.snapshot(pane_id).unwrap();
        assert_eq!(snapshot.size, TerminalSize::new(80, 24));
        let expected = format!("FINAL-PANE-{pane_index}");
        assert!(
            (0..snapshot.size.rows)
                .filter_map(|row| snapshot.row_text(row))
                .any(|row| row == expected),
            "final marker missing from {pane_id}"
        );
        runtime.close(pane_id).unwrap();
    }
}
