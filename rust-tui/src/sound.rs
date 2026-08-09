use crate::paths;
use crate::theme::SoundConfig;
use std::fs;
use std::io;

mod catalog;
mod playback {
    #[cfg(all(any(target_os = "linux", target_os = "macos"), not(test)))]
    use std::io;
    use std::path::Path;
    #[cfg(all(any(target_os = "linux", target_os = "macos"), not(test)))]
    use std::process::{Command, Stdio};

    #[cfg(any(target_os = "linux", target_os = "macos", test))]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(super) struct CommandSpec {
        pub(super) program: String,
        pub(super) args: Vec<String>,
    }

    #[cfg(any(target_os = "linux", test))]
    pub(super) fn linux_command_spec(
        path: &Path,
        has_command: impl Fn(&str) -> bool,
    ) -> Option<CommandSpec> {
        let file = path.to_string_lossy().into_owned();
        if has_command("paplay") {
            return Some(CommandSpec {
                program: "paplay".into(),
                args: vec![file],
            });
        }
        if has_command("pw-play") {
            return Some(CommandSpec {
                program: "pw-play".into(),
                args: vec![file],
            });
        }
        if has_command("aplay") {
            return Some(CommandSpec {
                program: "aplay".into(),
                args: vec!["-q".into(), file],
            });
        }
        if has_command("play") {
            return Some(CommandSpec {
                program: "play".into(),
                args: vec!["-q".into(), file],
            });
        }
        None
    }

    #[cfg(any(target_os = "macos", test))]
    pub(super) fn macos_command_spec(
        path: &Path,
        has_command: impl Fn(&str) -> bool,
    ) -> Option<CommandSpec> {
        if !has_command("afplay") {
            return None;
        }
        Some(CommandSpec {
            program: "afplay".into(),
            args: vec![path.to_string_lossy().into_owned()],
        })
    }

    #[cfg(all(any(target_os = "linux", target_os = "macos"), not(test)))]
    pub(super) fn spawn_audio(program: &str, args: &[String]) -> io::Result<()> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        std::thread::spawn(move || {
            let _ = child.wait();
        });

        Ok(())
    }

    #[cfg(all(any(target_os = "linux", target_os = "macos"), not(test)))]
    pub(super) fn command_exists(program: &str) -> bool {
        let Some(paths) = std::env::var_os("PATH") else {
            return false;
        };

        std::env::split_paths(&paths).any(|dir| executable_exists(&dir.join(program)))
    }

    #[cfg(all(any(target_os = "linux", target_os = "macos"), not(test)))]
    fn executable_exists(path: &Path) -> bool {
        std::fs::metadata(path)
            .map(|meta| {
                if !meta.is_file() {
                    return false;
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    meta.permissions().mode() & 0o111 != 0
                }
                #[cfg(not(unix))]
                {
                    true
                }
            })
            .unwrap_or(false)
    }
}
#[cfg(test)]
mod test_capture {
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
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::theme::SoundConfig;
    use std::path::{Path, PathBuf};

    fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
        crate::test_support::with_temp_home("pad-sound", name, f)
    }

    pub(crate) fn ensure_runtime_assets_writes_all_presets() {
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

    pub(crate) fn normalize_preset_id_falls_back_to_default() {
        assert_eq!(
            normalize_preset_id_or_default("no-such-preset", "glass"),
            "glass"
        );
        assert_eq!(normalize_preset_id("ping"), Some("ping"));
    }

    pub(crate) fn play_event_records_test_playback_when_enabled() {
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

    pub(crate) fn play_event_respects_global_and_event_switches() {
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

    pub(crate) fn linux_command_spec_uses_expected_priority() {
        let path = PathBuf::from("/tmp/glass.wav");
        let spec = playback::linux_command_spec(&path, |cmd| matches!(cmd, "aplay" | "play"))
            .expect("linux spec");
        assert_eq!(spec.program, "aplay");
        assert_eq!(spec.args, vec!["-q", "/tmp/glass.wav"]);

        let spec = playback::linux_command_spec(&path, |cmd| cmd == "paplay").expect("paplay spec");
        assert_eq!(spec.program, "paplay");
        assert_eq!(spec.args, vec!["/tmp/glass.wav"]);
    }

    pub(crate) fn macos_command_spec_uses_local_wav_path() {
        let spec = playback::macos_command_spec(Path::new("/tmp/ping.wav"), |cmd| cmd == "afplay")
            .expect("macos spec");
        assert_eq!(spec.program, "afplay");
        assert_eq!(spec.args, vec!["/tmp/ping.wav"]);
    }
}
mod wav {
    use super::catalog::{Segment, SoundPreset};
    use std::f32::consts::PI;

    const SAMPLE_RATE: u32 = 22_050;
    const WAV_CHANNELS: u16 = 1;
    const WAV_BITS_PER_SAMPLE: u16 = 16;

    pub(super) fn render_wav_bytes(preset: &SoundPreset) -> Vec<u8> {
        let samples = render_samples(preset.segments);
        write_wav(samples.as_slice())
    }

    fn render_samples(segments: &[Segment]) -> Vec<i16> {
        let total_samples = segments
            .iter()
            .map(|segment| match segment {
                Segment::Tone { ms, .. } | Segment::Pause { ms } => ms_to_samples(*ms),
            })
            .sum::<usize>()
            .saturating_add(ms_to_samples(32));
        let mut mixed = vec![0.0f32; total_samples];
        let mut cursor = 0usize;

        for segment in segments {
            match *segment {
                Segment::Pause { ms } => {
                    cursor = cursor.saturating_add(ms_to_samples(ms));
                }
                Segment::Tone { freq_hz, ms, gain } => {
                    let sample_count = ms_to_samples(ms);
                    for offset in 0..sample_count {
                        let index = cursor + offset;
                        if index >= mixed.len() {
                            break;
                        }
                        let t = offset as f32 / SAMPLE_RATE as f32;
                        let envelope = tone_envelope(offset, sample_count);
                        mixed[index] += (2.0 * PI * freq_hz * t).sin() * gain * envelope;
                    }
                    cursor = cursor.saturating_add(sample_count);
                }
            }
        }

        mixed
            .into_iter()
            .map(|sample| {
                let clamped = sample.clamp(-0.98, 0.98);
                (clamped * i16::MAX as f32) as i16
            })
            .collect()
    }

    fn tone_envelope(index: usize, total: usize) -> f32 {
        if total <= 4 {
            return 1.0;
        }

        let fade_len = total.min(ms_to_samples(18) * 2).max(4) / 2;
        if index < fade_len {
            index as f32 / fade_len as f32
        } else if index + fade_len >= total {
            (total.saturating_sub(index)) as f32 / fade_len as f32
        } else {
            1.0
        }
    }

    fn ms_to_samples(ms: u16) -> usize {
        SAMPLE_RATE as usize * ms as usize / 1000
    }

    fn write_wav(samples: &[i16]) -> Vec<u8> {
        let data_len = std::mem::size_of_val(samples) as u32;
        let riff_len = 36 + data_len;
        let byte_rate = SAMPLE_RATE * WAV_CHANNELS as u32 * WAV_BITS_PER_SAMPLE as u32 / 8;
        let block_align = WAV_CHANNELS * WAV_BITS_PER_SAMPLE / 8;
        let mut bytes = Vec::with_capacity(44 + data_len as usize);

        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&riff_len.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&WAV_CHANNELS.to_le_bytes());
        bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&block_align.to_le_bytes());
        bytes.extend_from_slice(&WAV_BITS_PER_SAMPLE.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }

        bytes
    }
}

pub use catalog::{
    normalize_preset_id, normalize_preset_id_or_default, preset, preset_ids, presets, SoundEvent,
};
#[cfg(test)]
pub(crate) use test_capture::{take_test_playbacks, with_test_sound_capture, TestPlayback};

pub fn play_event(config: &SoundConfig, event: SoundEvent) -> io::Result<bool> {
    let event_config = event.config(config);
    if !config.enabled || !event_config.enabled {
        return Ok(false);
    }

    play_internal(Some(event), &event_config.preset)
}

pub fn preview_preset(preset_id: &str) -> io::Result<bool> {
    play_internal(None, preset_id)
}

pub fn ensure_runtime_assets() -> io::Result<()> {
    fs::create_dir_all(paths::sounds_dir())?;

    for preset in presets() {
        let path = paths::sound_file_path(preset.id);
        let desired = wav::render_wav_bytes(preset);
        if fs::read(&path).ok().as_deref() != Some(desired.as_slice()) {
            fs::write(&path, &desired)?;
        }
    }

    Ok(())
}

#[cfg(test)]
fn play_internal(event: Option<SoundEvent>, preset_id: &str) -> io::Result<bool> {
    let Some(preset_id) = normalize_preset_id(preset_id) else {
        return Ok(false);
    };

    if test_capture::should_capture_test_sounds() {
        test_capture::record_test_playback(event, preset_id);
        return Ok(true);
    }

    Ok(false)
}

#[cfg(not(test))]
fn play_internal(_event: Option<SoundEvent>, preset_id: &str) -> io::Result<bool> {
    let Some(preset_id) = normalize_preset_id(preset_id) else {
        return Ok(false);
    };

    if sounds_disabled() {
        return Ok(false);
    }

    let path = paths::sound_file_path(preset_id);
    if !path.exists() {
        ensure_runtime_assets()?;
    }
    let path = paths::sound_file_path(preset_id);
    if !path.exists() {
        return Ok(false);
    }

    #[cfg(target_os = "macos")]
    {
        let Some(spec) = playback::macos_command_spec(&path, playback::command_exists) else {
            return Ok(false);
        };
        playback::spawn_audio(&spec.program, &spec.args)?;
        Ok(true)
    }

    #[cfg(target_os = "linux")]
    {
        let Some(spec) = playback::linux_command_spec(&path, playback::command_exists) else {
            return Ok(false);
        };
        playback::spawn_audio(&spec.program, &spec.args)?;
        Ok(true)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
        Ok(false)
    }
}

#[cfg(not(test))]
fn sounds_disabled() -> bool {
    cfg!(test) || std::env::var_os("PAD_DISABLE_SOUNDS").is_some()
}
