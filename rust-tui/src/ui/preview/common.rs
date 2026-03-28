use ratatui::style::Color;

pub(crate) fn display_width(s: &str) -> usize {
    s.chars().map(char_display_width).sum()
}

pub(crate) fn char_display_width(c: char) -> usize {
    if c == '\t' {
        return 4;
    }
    if c.is_control() {
        return 0;
    }

    let code = c as u32;
    if matches!(
        code,
        0x1100..=0x115F
            | 0x2329..=0x232A
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1FAFF
            | 0x20000..=0x3FFFD
    ) {
        2
    } else {
        1
    }
}

pub(crate) fn truncate_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(text) <= max_width {
        return text.to_string();
    }

    let ellipsis = "…";
    let ellipsis_width = display_width(ellipsis);
    let target_width = max_width.saturating_sub(ellipsis_width);
    let mut result = String::new();
    let mut used = 0usize;

    for ch in text.chars() {
        let width = char_display_width(ch);
        if used + width > target_width {
            break;
        }
        result.push(ch);
        used += width;
    }

    result.push_str(ellipsis);
    result
}

pub(crate) fn pad_to_width(text: &str, target_width: usize) -> String {
    let width = display_width(text);
    if width >= target_width {
        return text.to_string();
    }

    let mut out = String::from(text);
    out.push_str(&" ".repeat(target_width - width));
    out
}

pub(crate) fn fallback_color(primary: Color, fallback: Color) -> Color {
    match primary {
        Color::Reset => fallback,
        _ => primary,
    }
}

pub(crate) fn blend_color(highlight: Color, base: Color, mix: f32) -> Color {
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
