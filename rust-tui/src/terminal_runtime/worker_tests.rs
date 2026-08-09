use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use crate::terminal_runtime::EngineFactory;

const TEST_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) fn feed_resize_and_scroll_propagate_engine_and_missing_pane_errors() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let drops = Arc::new(AtomicUsize::new(0));
    let mut registry = EngineRegistry::default();
    registry.register(
        EngineId::new("recording"),
        RecordingFactory::new("recording", events.clone(), drops.clone()),
    );
    let runtime = EngineRuntime::start(1, registry).unwrap();
    let pane_id = PaneId::new("errors");

    assert_eq!(
        runtime
            .feed(&pane_id, b"missing".to_vec())
            .unwrap_err()
            .to_string(),
        "terminal pane 'errors' is not open"
    );
    assert_eq!(
        runtime
            .resize(&pane_id, TerminalSize::new(8, 2))
            .unwrap_err()
            .to_string(),
        "terminal pane 'errors' is not open"
    );
    assert_eq!(
        runtime
            .scroll(&pane_id, TerminalScroll::Lines(2))
            .unwrap_err()
            .to_string(),
        "terminal pane 'errors' is not open"
    );

    runtime
        .open(
            pane_id.clone(),
            EngineId::new("recording"),
            TerminalSize::new(8, 2),
        )
        .unwrap();
    assert_eq!(
        runtime
            .feed(&pane_id, b"feed-error".to_vec())
            .unwrap_err()
            .to_string(),
        "recording feed failed"
    );
    assert_eq!(
        runtime
            .resize(&pane_id, TerminalSize::new(13, 2))
            .unwrap_err()
            .to_string(),
        "recording resize failed"
    );

    // An ordinary engine error does not poison the pane or its shard.
    runtime.feed(&pane_id, b"recovered".to_vec()).unwrap();
    runtime.resize(&pane_id, TerminalSize::new(9, 3)).unwrap();
    runtime.scroll(&pane_id, TerminalScroll::Lines(2)).unwrap();
    assert_eq!(
        events.lock().unwrap().as_slice(),
        ["feed:recovered", "resize:9x3", "scroll:Lines(2)"]
    );

    runtime.close(&pane_id).unwrap();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(
        runtime.close(&pane_id).unwrap_err().to_string(),
        "terminal pane 'errors' is not open"
    );
}

pub(crate) fn bounded_queue_preserves_order_and_close_waits_for_destruction() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let drops = Arc::new(AtomicUsize::new(0));
    let (factory, entered, release) = BlockingFactory::new(events.clone(), drops.clone());
    let mut registry = EngineRegistry::default();
    registry.register(EngineId::new("blocking"), factory);
    let runtime = Arc::new(
        EngineRuntime::start_with_queue_capacity(1, 1, registry).expect("runtime should start"),
    );
    let pane_id = PaneId::new("ordered");
    runtime
        .open(
            pane_id.clone(),
            EngineId::new("blocking"),
            TerminalSize::new(8, 2),
        )
        .unwrap();

    let first_runtime = runtime.clone();
    let first_pane = pane_id.clone();
    let first = thread::spawn(move || first_runtime.feed(&first_pane, b"block".to_vec()));
    entered
        .recv_timeout(TEST_TIMEOUT)
        .expect("first feed should reach the engine");

    let (second_reply, second_result) = mpsc::sync_channel(1);
    let second = EngineCommand::Feed {
        pane_id: pane_id.clone(),
        bytes: b"second".to_vec(),
        reply: second_reply,
    };
    assert!(runtime.senders[0].try_send(second).is_ok());

    let (overflow_reply, _overflow_result) = mpsc::sync_channel(1);
    let overflow = EngineCommand::Feed {
        pane_id: pane_id.clone(),
        bytes: b"overflow".to_vec(),
        reply: overflow_reply,
    };
    assert!(matches!(
        runtime.senders[0].try_send(overflow),
        Err(mpsc::TrySendError::Full(_))
    ));

    let close_runtime = runtime.clone();
    let close_pane = pane_id.clone();
    let close = thread::spawn(move || close_runtime.close(&close_pane));

    release.send(()).unwrap();
    first.join().unwrap().unwrap();
    second_result
        .recv_timeout(TEST_TIMEOUT)
        .expect("queued feed should receive a reply")
        .unwrap();
    close.join().unwrap().unwrap();

    assert_eq!(
        events.lock().unwrap().as_slice(),
        ["feed:block", "feed:second"]
    );
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(
        runtime
            .feed(&pane_id, b"after-close".to_vec())
            .unwrap_err()
            .to_string(),
        "terminal pane 'ordered' is not open"
    );
}

pub(crate) fn separate_shards_make_progress_independently() {
    let blocked_events = Arc::new(Mutex::new(Vec::new()));
    let blocked_drops = Arc::new(AtomicUsize::new(0));
    let (blocking_factory, entered, release) = BlockingFactory::new(blocked_events, blocked_drops);
    let fast_events = Arc::new(Mutex::new(Vec::new()));
    let mut registry = EngineRegistry::default();
    registry.register(EngineId::new("blocking"), blocking_factory);
    registry.register(
        EngineId::new("fast"),
        RecordingFactory::new("fast", fast_events.clone(), Arc::new(AtomicUsize::new(0))),
    );
    let runtime = Arc::new(EngineRuntime::start(2, registry).unwrap());
    let blocked_pane = PaneId::new("blocked");
    let fast_pane = pane_on_other_shard(&runtime, &blocked_pane);
    runtime
        .open(
            blocked_pane.clone(),
            EngineId::new("blocking"),
            TerminalSize::new(8, 2),
        )
        .unwrap();
    runtime
        .open(
            fast_pane.clone(),
            EngineId::new("fast"),
            TerminalSize::new(8, 2),
        )
        .unwrap();

    let blocked_runtime = runtime.clone();
    let blocked = thread::spawn(move || blocked_runtime.feed(&blocked_pane, b"block".to_vec()));
    entered
        .recv_timeout(TEST_TIMEOUT)
        .expect("blocking shard should enter feed");

    let fast_runtime = runtime.clone();
    let (fast_done, fast_result) = mpsc::sync_channel(1);
    let fast = thread::spawn(move || {
        let result = fast_runtime
            .feed(&fast_pane, b"fast".to_vec())
            .map_err(|error| error.to_string());
        let _ = fast_done.send(result);
    });
    let parallel_result = fast_result.recv_timeout(TEST_TIMEOUT);

    // Always unblock the first shard before asserting, so a regression
    // fails cleanly instead of stranding worker threads in the test.
    release.send(()).unwrap();
    blocked.join().unwrap().unwrap();
    fast.join().unwrap();
    parallel_result
        .expect("the other shard should finish while one shard is blocked")
        .unwrap();
    assert_eq!(fast_events.lock().unwrap().as_slice(), ["feed:fast"]);
}

pub(crate) fn shutdown_drains_commands_already_accepted_by_a_full_queue() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let drops = Arc::new(AtomicUsize::new(0));
    let (factory, entered, release) = BlockingFactory::new(events.clone(), drops.clone());
    let mut registry = EngineRegistry::default();
    registry.register(EngineId::new("blocking"), factory);
    let runtime = EngineRuntime::start_with_queue_capacity(1, 1, registry).unwrap();
    let pane_id = PaneId::new("shutdown");
    runtime
        .open(
            pane_id.clone(),
            EngineId::new("blocking"),
            TerminalSize::new(8, 2),
        )
        .unwrap();

    let (first_reply, first_result) = mpsc::sync_channel(1);
    runtime
        .send(
            &pane_id,
            EngineCommand::Feed {
                pane_id: pane_id.clone(),
                bytes: b"block".to_vec(),
                reply: first_reply,
            },
        )
        .unwrap();
    entered.recv_timeout(TEST_TIMEOUT).unwrap();

    let (second_reply, second_result) = mpsc::sync_channel(1);
    assert!(runtime.senders[0]
        .try_send(EngineCommand::Feed {
            pane_id,
            bytes: b"second".to_vec(),
            reply: second_reply,
        })
        .is_ok());

    let (shutdown_done, shutdown_result) = mpsc::sync_channel(1);
    let shutdown = thread::spawn(move || {
        drop(runtime);
        let _ = shutdown_done.send(());
    });
    release.send(()).unwrap();

    first_result.recv_timeout(TEST_TIMEOUT).unwrap().unwrap();
    second_result.recv_timeout(TEST_TIMEOUT).unwrap().unwrap();
    shutdown_result
        .recv_timeout(TEST_TIMEOUT)
        .expect("shutdown should join the drained worker");
    shutdown.join().unwrap();
    assert_eq!(
        events.lock().unwrap().as_slice(),
        ["feed:block", "feed:second"]
    );
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

pub(crate) fn engine_panics_remove_only_the_faulting_pane_and_keep_the_shard_alive() {
    let healthy_feeds = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    let mut registry = EngineRegistry::default();
    for (id, panic_at) in [
        ("healthy", PanicAt::Never),
        ("panic-create", PanicAt::Create),
        ("panic-feed", PanicAt::FeedAndDrop),
        ("panic-resize", PanicAt::Resize),
        ("panic-scroll", PanicAt::Scroll),
        ("panic-snapshot", PanicAt::Snapshot),
        ("panic-drain", PanicAt::DrainEvents),
        ("panic-close", PanicAt::Drop),
    ] {
        registry.register(
            EngineId::new(id),
            PanicFactory {
                id,
                panic_at,
                feeds: healthy_feeds.clone(),
                drops: drops.clone(),
            },
        );
    }
    // One worker makes every pane share a shard, so each successful health
    // check proves that the worker survived the preceding panic.
    let runtime = EngineRuntime::start(1, registry).unwrap();
    let healthy = PaneId::new("healthy-pane");
    runtime
        .open(
            healthy.clone(),
            EngineId::new("healthy"),
            TerminalSize::new(8, 2),
        )
        .unwrap();

    let create_pane = PaneId::new("create-pane");
    let error = runtime
        .open(
            create_pane.clone(),
            EngineId::new("panic-create"),
            TerminalSize::new(8, 2),
        )
        .unwrap_err();
    assert_panic_context(error, &create_pane, "create", "create exploded");
    assert_healthy(&runtime, &healthy);

    let feed_pane = open_panic_pane(&runtime, "feed-pane", "panic-feed");
    let error = runtime.feed(&feed_pane, b"panic".to_vec()).unwrap_err();
    assert_panic_context(error, &feed_pane, "feed", "feed exploded");
    assert_eq!(
        runtime.snapshot(&feed_pane).unwrap_err().to_string(),
        "terminal pane 'feed-pane' is not open"
    );
    assert_healthy(&runtime, &healthy);

    let resize_pane = open_panic_pane(&runtime, "resize-pane", "panic-resize");
    let error = runtime
        .resize(&resize_pane, TerminalSize::new(9, 3))
        .unwrap_err();
    assert_panic_context(error, &resize_pane, "resize", "resize exploded");
    assert_healthy(&runtime, &healthy);

    let scroll_pane = open_panic_pane(&runtime, "scroll-pane", "panic-scroll");
    let error = runtime
        .scroll(&scroll_pane, TerminalScroll::PageUp)
        .unwrap_err();
    assert_panic_context(error, &scroll_pane, "scroll", "scroll exploded");
    assert_healthy(&runtime, &healthy);

    let snapshot_pane = open_panic_pane(&runtime, "snapshot-pane", "panic-snapshot");
    let error = runtime.snapshot(&snapshot_pane).unwrap_err();
    assert_panic_context(error, &snapshot_pane, "snapshot", "snapshot exploded");
    assert_healthy(&runtime, &healthy);

    let drain_pane = open_panic_pane(&runtime, "drain-pane", "panic-drain");
    let error = runtime.drain_events(&drain_pane).unwrap_err();
    assert_panic_context(error, &drain_pane, "drain events", "drain exploded");
    assert_healthy(&runtime, &healthy);

    let close_pane = open_panic_pane(&runtime, "close-pane", "panic-close");
    let error = runtime.close(&close_pane).unwrap_err();
    assert_panic_context(error, &close_pane, "close", "drop exploded");
    assert_eq!(
        runtime.close(&close_pane).unwrap_err().to_string(),
        "terminal pane 'close-pane' is not open"
    );
    assert_healthy(&runtime, &healthy);

    assert_eq!(healthy_feeds.load(Ordering::SeqCst), 7);
}

struct RecordingFactory {
    id: &'static str,
    events: Arc<Mutex<Vec<String>>>,
    drops: Arc<AtomicUsize>,
}

impl RecordingFactory {
    fn new(id: &'static str, events: Arc<Mutex<Vec<String>>>, drops: Arc<AtomicUsize>) -> Self {
        Self { id, events, drops }
    }
}

impl EngineFactory for RecordingFactory {
    fn create(&self, size: TerminalSize) -> Result<Box<dyn TerminalEngine>, TerminalError> {
        Ok(Box::new(RecordingEngine {
            id: EngineId::new(self.id),
            snapshot: TerminalSnapshot::blank(size),
            events: self.events.clone(),
            drops: self.drops.clone(),
            blocker: None,
        }))
    }
}

struct BlockingFactory {
    events: Arc<Mutex<Vec<String>>>,
    drops: Arc<AtomicUsize>,
    entered: SyncSender<()>,
    release: Mutex<Option<Receiver<()>>>,
}

impl BlockingFactory {
    fn new(
        events: Arc<Mutex<Vec<String>>>,
        drops: Arc<AtomicUsize>,
    ) -> (Self, Receiver<()>, SyncSender<()>) {
        let (entered_sender, entered_receiver) = mpsc::sync_channel(1);
        let (release_sender, release_receiver) = mpsc::sync_channel(1);
        (
            Self {
                events,
                drops,
                entered: entered_sender,
                release: Mutex::new(Some(release_receiver)),
            },
            entered_receiver,
            release_sender,
        )
    }
}

impl EngineFactory for BlockingFactory {
    fn create(&self, size: TerminalSize) -> Result<Box<dyn TerminalEngine>, TerminalError> {
        let release = self
            .release
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| TerminalError::new("blocking engine already created"))?;
        Ok(Box::new(RecordingEngine {
            id: EngineId::new("blocking"),
            snapshot: TerminalSnapshot::blank(size),
            events: self.events.clone(),
            drops: self.drops.clone(),
            blocker: Some(Blocker {
                entered: self.entered.clone(),
                release,
            }),
        }))
    }
}

struct Blocker {
    entered: SyncSender<()>,
    release: Receiver<()>,
}

struct RecordingEngine {
    id: EngineId,
    snapshot: TerminalSnapshot,
    events: Arc<Mutex<Vec<String>>>,
    drops: Arc<AtomicUsize>,
    blocker: Option<Blocker>,
}

impl TerminalEngine for RecordingEngine {
    fn id(&self) -> &EngineId {
        &self.id
    }

    fn feed(&mut self, bytes: &[u8]) -> Result<(), TerminalError> {
        let text = String::from_utf8_lossy(bytes);
        if text == "feed-error" {
            return Err(TerminalError::new(format!("{} feed failed", self.id)));
        }
        if text == "block" {
            if let Some(blocker) = &self.blocker {
                blocker
                    .entered
                    .send(())
                    .map_err(|_| TerminalError::new("test gate lost its receiver"))?;
                blocker
                    .release
                    .recv()
                    .map_err(|_| TerminalError::new("test gate was not released"))?;
            }
        }
        self.events.lock().unwrap().push(format!("feed:{text}"));
        Ok(())
    }

    fn resize(&mut self, size: TerminalSize) -> Result<(), TerminalError> {
        if size.columns == 13 {
            return Err(TerminalError::new(format!("{} resize failed", self.id)));
        }
        self.events
            .lock()
            .unwrap()
            .push(format!("resize:{}x{}", size.columns, size.rows));
        self.snapshot = TerminalSnapshot::blank(size);
        Ok(())
    }

    fn scroll(&mut self, scroll: TerminalScroll) -> Result<(), TerminalError> {
        self.events
            .lock()
            .unwrap()
            .push(format!("scroll:{scroll:?}"));
        Ok(())
    }

    fn snapshot(&self) -> TerminalSnapshot {
        self.snapshot.clone()
    }
}

impl Drop for RecordingEngine {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum PanicAt {
    Never,
    Create,
    FeedAndDrop,
    Resize,
    Scroll,
    Snapshot,
    DrainEvents,
    Drop,
}

struct PanicFactory {
    id: &'static str,
    panic_at: PanicAt,
    feeds: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
}

impl EngineFactory for PanicFactory {
    fn create(&self, size: TerminalSize) -> Result<Box<dyn TerminalEngine>, TerminalError> {
        if self.panic_at == PanicAt::Create {
            panic!("create exploded");
        }
        Ok(Box::new(PanicEngine {
            id: EngineId::new(self.id),
            panic_at: self.panic_at,
            snapshot: TerminalSnapshot::blank(size),
            feeds: self.feeds.clone(),
            drops: self.drops.clone(),
        }))
    }
}

struct PanicEngine {
    id: EngineId,
    panic_at: PanicAt,
    snapshot: TerminalSnapshot,
    feeds: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
}

impl TerminalEngine for PanicEngine {
    fn id(&self) -> &EngineId {
        &self.id
    }

    fn feed(&mut self, _bytes: &[u8]) -> Result<(), TerminalError> {
        if self.panic_at == PanicAt::FeedAndDrop {
            panic!("feed exploded");
        }
        self.feeds.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn resize(&mut self, size: TerminalSize) -> Result<(), TerminalError> {
        if self.panic_at == PanicAt::Resize {
            panic!("resize exploded");
        }
        self.snapshot = TerminalSnapshot::blank(size);
        Ok(())
    }

    fn scroll(&mut self, _scroll: TerminalScroll) -> Result<(), TerminalError> {
        if self.panic_at == PanicAt::Scroll {
            panic!("scroll exploded");
        }
        Ok(())
    }

    fn snapshot(&self) -> TerminalSnapshot {
        if self.panic_at == PanicAt::Snapshot {
            panic!("snapshot exploded");
        }
        self.snapshot.clone()
    }

    fn drain_events(&mut self) -> Vec<TerminalEngineEvent> {
        if self.panic_at == PanicAt::DrainEvents {
            panic!("drain exploded");
        }
        Vec::new()
    }
}

impl Drop for PanicEngine {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
        if matches!(self.panic_at, PanicAt::FeedAndDrop | PanicAt::Drop) {
            panic!("drop exploded");
        }
    }
}

fn open_panic_pane(runtime: &EngineRuntime, pane: &str, engine: &str) -> PaneId {
    let pane_id = PaneId::new(pane);
    runtime
        .open(
            pane_id.clone(),
            EngineId::new(engine),
            TerminalSize::new(8, 2),
        )
        .unwrap();
    pane_id
}

fn assert_panic_context(error: TerminalError, pane_id: &PaneId, operation: &str, payload: &str) {
    let message = error.to_string();
    assert!(message.contains(pane_id.as_str()), "{message}");
    assert!(message.contains(operation), "{message}");
    assert!(message.contains(payload), "{message}");
}

fn assert_healthy(runtime: &EngineRuntime, pane_id: &PaneId) {
    runtime.feed(pane_id, b"healthy".to_vec()).unwrap();
    runtime.snapshot(pane_id).unwrap();
}

fn pane_on_other_shard(runtime: &EngineRuntime, first: &PaneId) -> PaneId {
    let first_shard = runtime.shard_index(first);
    (0..100)
        .map(|index| PaneId::new(format!("other-{index}")))
        .find(|pane_id| runtime.shard_index(pane_id) != first_shard)
        .expect("two shards should accept different pane hashes")
}
