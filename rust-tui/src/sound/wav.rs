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
