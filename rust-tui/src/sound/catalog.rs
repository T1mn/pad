use crate::theme::{SoundConfig, SoundEventConfig};

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

    pub(super) fn config(self, config: &SoundConfig) -> &SoundEventConfig {
        match self {
            Self::Completion => &config.completion,
            Self::Approval => &config.approval,
            Self::Timeout => &config.timeout,
            Self::Failure => &config.failure,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Segment {
    Tone { freq_hz: f32, ms: u16, gain: f32 },
    Pause { ms: u16 },
}

#[derive(Clone, Copy)]
pub struct SoundPreset {
    pub id: &'static str,
    pub name_key: &'static str,
    pub desc_key: &'static str,
    pub(super) segments: &'static [Segment],
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
