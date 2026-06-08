use super::*;
use crate::theme::SoundConfig;
use std::path::{Path, PathBuf};

fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
    crate::test_support::with_temp_home("pad-sound", name, f)
}

#[test]
fn ensure_runtime_assets_writes_all_presets() {
    with_temp_home("runtime-assets", |_home| {
        ensure_runtime_assets().expect("write sound assets");

        for preset in presets() {
            let path = crate::paths::sound_file_path(preset.id);
            let body = std::fs::read(&path).expect("preset file");
            assert!(body.starts_with(b"RIFF"));
            assert!(body.len() > 44);
        }
    });
}

#[test]
fn normalize_preset_id_falls_back_to_default() {
    assert_eq!(
        normalize_preset_id_or_default("no-such-preset", "glass"),
        "glass"
    );
    assert_eq!(normalize_preset_id("ping"), Some("ping"));
}

#[test]
fn play_event_records_test_playback_when_enabled() {
    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("lock sound test playback");
    with_test_sound_capture(|| {
        let _ = take_test_playbacks();
        let config = SoundConfig::default();

        let played = play_event(&config, SoundEvent::Completion).expect("play sound");
        assert!(played);
        assert_eq!(
            take_test_playbacks(),
            vec![TestPlayback {
                event: Some(SoundEvent::Completion),
                preset: "glass".into(),
            }]
        );
    });
}

#[test]
fn play_event_respects_global_and_event_switches() {
    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("lock sound toggle tests");
    with_test_sound_capture(|| {
        let _ = take_test_playbacks();
        let mut config = SoundConfig {
            enabled: false,
            ..SoundConfig::default()
        };
        assert!(!play_event(&config, SoundEvent::Completion).expect("play sound"));
        assert!(take_test_playbacks().is_empty());

        config.enabled = true;
        config.completion.enabled = false;
        assert!(!play_event(&config, SoundEvent::Completion).expect("play sound"));
        assert!(take_test_playbacks().is_empty());
    });
}

#[test]
fn linux_command_spec_uses_expected_priority() {
    let path = PathBuf::from("/tmp/glass.wav");
    let spec = playback::linux_command_spec(&path, |cmd| matches!(cmd, "aplay" | "play"))
        .expect("linux spec");
    assert_eq!(spec.program, "aplay");
    assert_eq!(spec.args, vec!["-q", "/tmp/glass.wav"]);

    let spec = playback::linux_command_spec(&path, |cmd| cmd == "paplay").expect("paplay spec");
    assert_eq!(spec.program, "paplay");
    assert_eq!(spec.args, vec!["/tmp/glass.wav"]);
}

#[test]
fn macos_command_spec_uses_local_wav_path() {
    let spec = playback::macos_command_spec(Path::new("/tmp/ping.wav"), |cmd| cmd == "afplay")
        .expect("macos spec");
    assert_eq!(spec.program, "afplay");
    assert_eq!(spec.args, vec!["/tmp/ping.wav"]);
}
