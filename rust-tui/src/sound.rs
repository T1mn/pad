use crate::paths;
use crate::theme::SoundConfig;
#[cfg(test)]
use std::cell::Cell;
use std::f32::consts::PI;
use std::fs;
use std::io;
#[cfg(test)]
use std::sync::{LazyLock, Mutex};

mod playback;
#[cfg(test)]
mod tests;

const SAMPLE_RATE: u32 = 22_050;
const WAV_CHANNELS: u16 = 1;
const WAV_BITS_PER_SAMPLE: u16 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundEvent {
    Completion,
    Approval,
    Timeout,
    Failure,
}

impl SoundEvent {
    pub fn default_preset_id(self) -> &'static str {
        match self {
            Self::Completion => "glass",
            Self::Approval => "ping",
            Self::Timeout => "warning",
            Self::Failure => "alert",
        }
    }

    fn config(self, config: &SoundConfig) -> &crate::theme::SoundEventConfig {
        match self {
            Self::Completion => &config.completion,
            Self::Approval => &config.approval,
            Self::Timeout => &config.timeout,
            Self::Failure => &config.failure,
        }
    }
}

#[derive(Clone, Copy)]
enum Segment {
    Tone { freq_hz: f32, ms: u16, gain: f32 },
    Pause { ms: u16 },
}

#[derive(Clone, Copy)]
pub struct SoundPreset {
    pub id: &'static str,
    pub name_key: &'static str,
    pub desc_key: &'static str,
    segments: &'static [Segment],
}

const GLASS_SEGMENTS: [Segment; 3] = [
    Segment::Tone {
        freq_hz: 1567.98,
        ms: 90,
        gain: 0.44,
    },
    Segment::Pause { ms: 24 },
    Segment::Tone {
        freq_hz: 2093.00,
        ms: 150,
        gain: 0.30,
    },
];

const PING_SEGMENTS: [Segment; 2] = [
    Segment::Tone {
        freq_hz: 1174.66,
        ms: 120,
        gain: 0.40,
    },
    Segment::Tone {
        freq_hz: 1396.91,
        ms: 90,
        gain: 0.20,
    },
];

const POP_SEGMENTS: [Segment; 3] = [
    Segment::Tone {
        freq_hz: 523.25,
        ms: 70,
        gain: 0.42,
    },
    Segment::Pause { ms: 18 },
    Segment::Tone {
        freq_hz: 783.99,
        ms: 80,
        gain: 0.28,
    },
];

const WARNING_SEGMENTS: [Segment; 3] = [
    Segment::Tone {
        freq_hz: 739.99,
        ms: 130,
        gain: 0.38,
    },
    Segment::Pause { ms: 70 },
    Segment::Tone {
        freq_hz: 587.33,
        ms: 170,
        gain: 0.34,
    },
];

const ALERT_SEGMENTS: [Segment; 5] = [
    Segment::Tone {
        freq_hz: 880.00,
        ms: 90,
        gain: 0.42,
    },
    Segment::Pause { ms: 42 },
    Segment::Tone {
        freq_hz: 880.00,
        ms: 90,
        gain: 0.42,
    },
    Segment::Pause { ms: 42 },
    Segment::Tone {
        freq_hz: 698.46,
        ms: 160,
        gain: 0.38,
    },
];

const PRESETS: [SoundPreset; 5] = [
    SoundPreset {
        id: "glass",
        name_key: "sound.preset.glass",
        desc_key: "sound.preset.glass_desc",
        segments: &GLASS_SEGMENTS,
    },
    SoundPreset {
        id: "ping",
        name_key: "sound.preset.ping",
        desc_key: "sound.preset.ping_desc",
        segments: &PING_SEGMENTS,
    },
    SoundPreset {
        id: "pop",
        name_key: "sound.preset.pop",
        desc_key: "sound.preset.pop_desc",
        segments: &POP_SEGMENTS,
    },
    SoundPreset {
        id: "warning",
        name_key: "sound.preset.warning",
        desc_key: "sound.preset.warning_desc",
        segments: &WARNING_SEGMENTS,
    },
    SoundPreset {
        id: "alert",
        name_key: "sound.preset.alert",
        desc_key: "sound.preset.alert_desc",
        segments: &ALERT_SEGMENTS,
    },
];

const PRESET_IDS: [&str; 5] = ["glass", "ping", "pop", "warning", "alert"];

pub fn presets() -> &'static [SoundPreset] {
    &PRESETS
}

pub fn preset_ids() -> &'static [&'static str] {
    &PRESET_IDS
}

pub fn preset(name: &str) -> Option<&'static SoundPreset> {
    PRESETS.iter().find(|preset| preset.id == name)
}

pub fn normalize_preset_id(value: &str) -> Option<&'static str> {
    preset(value).map(|preset| preset.id)
}

pub fn normalize_preset_id_or_default(value: &str, default: &'static str) -> &'static str {
    normalize_preset_id(value).unwrap_or(default)
}

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
        let desired = render_wav_bytes(preset);
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

    if should_capture_test_sounds() {
        record_test_playback(event, preset_id);
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

fn render_wav_bytes(preset: &SoundPreset) -> Vec<u8> {
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

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TestPlayback {
    pub event: Option<SoundEvent>,
    pub preset: String,
}

#[cfg(test)]
static TEST_PLAYBACKS: LazyLock<Mutex<Vec<TestPlayback>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
#[cfg(test)]
thread_local! {
    static TEST_SOUND_CAPTURE: Cell<bool> = const { Cell::new(false) };
}

#[cfg(test)]
fn record_test_playback(event: Option<SoundEvent>, preset: &str) {
    let mut playbacks = TEST_PLAYBACKS.lock().expect("sound playback lock");
    playbacks.push(TestPlayback {
        event,
        preset: preset.to_string(),
    });
}

#[cfg(test)]
pub(crate) fn take_test_playbacks() -> Vec<TestPlayback> {
    let mut playbacks = TEST_PLAYBACKS.lock().expect("sound playback lock");
    std::mem::take(&mut *playbacks)
}

#[cfg(test)]
pub(crate) fn with_test_sound_capture<T>(f: impl FnOnce() -> T) -> T {
    TEST_SOUND_CAPTURE.with(|capture| {
        let previous = capture.replace(true);
        let result = f();
        capture.set(previous);
        result
    })
}

#[cfg(test)]
fn should_capture_test_sounds() -> bool {
    TEST_SOUND_CAPTURE.with(|capture| capture.get())
}
