use crate::model::AgentState;
#[cfg(test)]
use ratatui::style::Modifier;
use ratatui::style::{Color, Style};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

#[cfg(test)]
const SHIMMER_SWEEP_SECONDS: f32 = 2.0;
#[cfg(test)]
const SHIMMER_PADDING: f32 = 10.0;
#[cfg(test)]
const SHIMMER_BAND_HALF_WIDTH: f32 = 5.0;
const BADGE_PULSE_CYCLE_SECONDS: f32 = 1.8;
const BREATHING_MIN_VISIBLE_BLEND: f32 = 0.12;
const BREATHING_BLEND_RANGE: f32 = 0.82;

pub(super) fn thread_badge_breathes(state: &AgentState) -> bool {
    matches!(state, AgentState::Busy)
}

pub(super) fn breathing_badge_style(base_color: Color, surface_bg: Color, bg: Color) -> Style {
    let intensity = breathing_intensity();
    Style::default()
        .fg(super::style::blend_color(
            base_color,
            surface_bg,
            BREATHING_MIN_VISIBLE_BLEND + intensity * BREATHING_BLEND_RANGE,
        ))
        .bg(bg)
}

pub(super) fn breathing_badge_text() -> &'static str {
    "• "
}

#[cfg(test)]
pub(super) fn shimmer_spans(
    text: &str,
    base_color: Color,
    highlight_color: Color,
    bg: Color,
) -> Vec<ratatui::text::Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }

    let total_width = super::metrics::display_width(text) as f32;
    let period = total_width + SHIMMER_PADDING * 2.0;
    let pos = (elapsed_since_start().as_secs_f32() % SHIMMER_SWEEP_SECONDS) / SHIMMER_SWEEP_SECONDS
        * period;
    let true_color = has_true_color();

    let mut spans = Vec::with_capacity(chars.len());
    let mut column = 0.0f32;

    for ch in chars {
        let width = super::metrics::char_display_width(ch).max(1) as f32;
        let center = column + (width * 0.5);
        let dist = ((center + SHIMMER_PADDING) - pos).abs();
        let intensity = if dist <= SHIMMER_BAND_HALF_WIDTH {
            let x = std::f32::consts::PI * (dist / SHIMMER_BAND_HALF_WIDTH);
            0.5 * (1.0 + x.cos())
        } else {
            0.0
        };
        let style = if true_color {
            let mixed = super::style::blend_color(highlight_color, base_color, intensity * 0.9);
            Style::default()
                .fg(mixed)
                .bg(bg)
                .add_modifier(Modifier::BOLD)
        } else {
            fallback_style(base_color, bg, intensity)
        };
        spans.push(ratatui::text::Span::styled(ch.to_string(), style));
        column += width;
    }

    spans
}

fn breathing_intensity() -> f32 {
    let phase = (elapsed_since_start().as_secs_f32() % BADGE_PULSE_CYCLE_SECONDS)
        / BADGE_PULSE_CYCLE_SECONDS;
    0.5 * (1.0 - (phase * std::f32::consts::TAU).cos())
}

fn elapsed_since_start() -> Duration {
    static PROCESS_START: OnceLock<Instant> = OnceLock::new();
    PROCESS_START.get_or_init(Instant::now).elapsed()
}

#[cfg(test)]
fn has_true_color() -> bool {
    static HAS_TRUE_COLOR: OnceLock<bool> = OnceLock::new();
    *HAS_TRUE_COLOR.get_or_init(|| {
        let color_term = std::env::var("COLORTERM")
            .unwrap_or_default()
            .to_lowercase();
        if color_term.contains("truecolor") || color_term.contains("24bit") {
            return true;
        }

        let term = std::env::var("TERM").unwrap_or_default().to_lowercase();
        term.contains("direct") || term.contains("truecolor") || term.contains("kitty")
    })
}

#[cfg(test)]
fn fallback_style(base_color: Color, bg: Color, _intensity: f32) -> Style {
    Style::default().fg(base_color).bg(bg)
}
