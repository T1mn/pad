use super::*;

pub(super) fn readable_text_color(primary: Color, current: Color, mix: f32) -> Color {
    blend_theme_color(primary, current, mix)
}

pub(super) fn readable_surface_color(primary: Color, current: Color, mix: f32) -> Color {
    blend_theme_color(primary, current, mix)
}

fn blend_theme_color(target: Color, base: Color, mix: f32) -> Color {
    let mix = mix.clamp(0.0, 1.0);
    match (theme_rgb(target), theme_rgb(base)) {
        (Some((tr, tg, tb)), Some((br, bg, bb))) => Color::Rgb(
            blend_theme_channel(tr, br, mix),
            blend_theme_channel(tg, bg, mix),
            blend_theme_channel(tb, bb, mix),
        ),
        _ if mix >= 0.5 => target,
        _ => base,
    }
}

fn blend_theme_channel(target: u8, base: u8, mix: f32) -> u8 {
    let target = target as f32;
    let base = base as f32;
    (base + (target - base) * mix).round().clamp(0.0, 255.0) as u8
}

fn theme_rgb(color: Color) -> Option<(u8, u8, u8)> {
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
