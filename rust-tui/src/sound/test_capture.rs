use super::SoundEvent;
use std::cell::Cell;
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TestPlayback {
    pub event: Option<SoundEvent>,
    pub preset: String,
}

static TEST_PLAYBACKS: LazyLock<Mutex<Vec<TestPlayback>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
thread_local! {
    static TEST_SOUND_CAPTURE: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn record_test_playback(event: Option<SoundEvent>, preset: &str) {
    let mut playbacks = TEST_PLAYBACKS.lock().expect("sound playback lock");
    playbacks.push(TestPlayback {
        event,
        preset: preset.to_string(),
    });
}

pub(crate) fn take_test_playbacks() -> Vec<TestPlayback> {
    let mut playbacks = TEST_PLAYBACKS.lock().expect("sound playback lock");
    std::mem::take(&mut *playbacks)
}

pub(crate) fn with_test_sound_capture<T>(f: impl FnOnce() -> T) -> T {
    TEST_SOUND_CAPTURE.with(|capture| {
        let previous = capture.replace(true);
        let result = f();
        capture.set(previous);
        result
    })
}

pub(super) fn should_capture_test_sounds() -> bool {
    TEST_SOUND_CAPTURE.with(|capture| capture.get())
}
