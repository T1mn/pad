use crate::i18n::Locale;
use crate::theme::Theme;
use ratatui::layout::Rect;
use ratatui::{
    style::Color,
    style::Style,
    widgets::{Block, Borders, Clear},
    Frame,
};

pub(super) fn render_modal_surface(f: &mut Frame, area: Rect, theme: &Theme) {
    let overscan_area = Rect {
        x: area.x.saturating_sub(1),
        y: area.y.saturating_sub(1),
        width: area.width.saturating_add(if area.x > 0 { 2 } else { 1 }),
        height: area.height.saturating_add(if area.y > 0 { 2 } else { 1 }),
    };
    f.render_widget(Clear, overscan_area);
    f.render_widget(
        Block::default().style(Style::default().bg(modal_surface_bg(theme))),
        overscan_area,
    );
    let surface = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(modal_border(theme)))
        .style(Style::default().bg(modal_surface_bg(theme)).fg(theme.fg));
    f.render_widget(surface, area);
}

fn modal_surface_bg(theme: &Theme) -> ratatui::style::Color {
    blend_color(theme.highlight_bg, theme.bg, 0.34)
}

fn modal_border(theme: &Theme) -> ratatui::style::Color {
    blend_color(theme.border_focused, theme.border, 0.62)
}

fn blend_color(highlight: Color, base: Color, mix: f32) -> Color {
    let mix = mix.clamp(0.0, 1.0);
    match (rgb_components(highlight), rgb_components(base)) {
        (Some((hr, hg, hb)), Some((br, bg, bb))) => Color::Rgb(
            blend_channel(hr, br, mix),
            blend_channel(hg, bg, mix),
            blend_channel(hb, bb, mix),
        ),
        _ if mix >= 0.5 => highlight,
        _ => base,
    }
}

fn blend_channel(highlight: u8, base: u8, mix: f32) -> u8 {
    let highlight = highlight as f32;
    let base = base as f32;
    (base + (highlight - base) * mix).round().clamp(0.0, 255.0) as u8
}

fn rgb_components(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Black => Some((0, 0, 0)),
        Color::Red => Some((255, 0, 0)),
        Color::Green => Some((0, 128, 0)),
        Color::Yellow => Some((255, 255, 0)),
        Color::Blue => Some((0, 0, 255)),
        Color::Magenta => Some((255, 0, 255)),
        Color::Cyan => Some((0, 255, 255)),
        Color::Gray => Some((128, 128, 128)),
        Color::DarkGray => Some((64, 64, 64)),
        Color::LightRed => Some((255, 102, 102)),
        Color::LightGreen => Some((144, 238, 144)),
        Color::LightYellow => Some((255, 255, 224)),
        Color::LightBlue => Some((173, 216, 230)),
        Color::LightMagenta => Some((255, 153, 255)),
        Color::LightCyan => Some((224, 255, 255)),
        Color::White => Some((255, 255, 255)),
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Indexed(_) | Color::Reset => None,
    }
}

pub(super) fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}

pub(super) fn truncate_modal_line(input: &str, max_chars: usize) -> String {
    let total = input.chars().count();
    if total <= max_chars {
        return input.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let prefix: String = input.chars().take(keep).collect();
    format!("{}...", prefix)
}

pub(super) fn truncate_modal_line_middle(input: &str, max_chars: usize) -> String {
    let total = input.chars().count();
    if total <= max_chars {
        return input.to_string();
    }
    if max_chars <= 3 {
        return "...".to_string();
    }

    let keep = max_chars.saturating_sub(3);
    let front = keep / 2;
    let back = keep.saturating_sub(front);

    let prefix: String = input.chars().take(front).collect();
    let suffix = trailing_chars(input, back);
    format!("{}...{}", prefix, suffix)
}

fn trailing_chars(input: &str, count: usize) -> String {
    if count == 0 {
        return String::new();
    }

    let mut tail = std::collections::VecDeque::with_capacity(count);
    for ch in input.chars() {
        if tail.len() == count {
            tail.pop_front();
        }
        tail.push_back(ch);
    }

    let mut suffix = String::new();
    for ch in tail {
        suffix.push(ch);
    }
    suffix
}

pub(super) fn mask_secret_prefix(value: &str, prefix_len: usize) -> String {
    if value.trim().is_empty() {
        return "-".to_string();
    }
    if value.len() <= prefix_len {
        return value.to_string();
    }
    format!("{}...", &value[..prefix_len])
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod tests;
