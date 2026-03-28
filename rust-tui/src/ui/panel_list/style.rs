use crate::model::AgentType;
use crate::theme::Theme;
use ratatui::style::{Color, Modifier, Style};

pub(crate) fn sidebar_card_bg(is_selected: bool, theme: &Theme) -> Color {
    if is_selected {
        blend_color(theme.border_focused, theme.highlight_bg, 0.14)
    } else {
        blend_color(theme.border, theme.bg, 0.18)
    }
}

pub(crate) fn sidebar_folder_fg(is_selected: bool, theme: &Theme) -> Color {
    if is_selected {
        theme.highlight_fg
    } else {
        blend_color(theme.fg, theme.comment, 0.28)
    }
}

pub(crate) fn sidebar_thread_fg(is_selected: bool, theme: &Theme) -> Color {
    if is_selected {
        theme.highlight_fg
    } else {
        blend_color(theme.fg, theme.comment, 0.14)
    }
}

pub(crate) fn sidebar_subtitle_fg(is_selected: bool, theme: &Theme) -> Color {
    if is_selected {
        blend_color(theme.highlight_fg, theme.comment, 0.28)
    } else {
        blend_color(theme.fg, theme.comment, 0.08)
    }
}

pub(crate) fn badge_color(agent_type: AgentType, theme: &Theme) -> Color {
    match agent_type {
        AgentType::Claude => Color::Rgb(249, 140, 87),
        AgentType::Codex => Color::Rgb(88, 166, 255),
        AgentType::Kimi => Color::Rgb(80, 200, 120),
        AgentType::Gemini => Color::Rgb(110, 168, 254),
        AgentType::OpenCode => Color::Rgb(250, 173, 20),
        AgentType::Aider => Color::Rgb(163, 190, 140),
        AgentType::Cursor => Color::Rgb(180, 140, 255),
        AgentType::Unknown => theme.comment,
    }
}

pub(crate) fn maybe_bold(style: Style, enabled: bool) -> Style {
    if enabled {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

pub(crate) fn blend_color(highlight: Color, base: Color, mix: f32) -> Color {
    let mix = mix.clamp(0.0, 1.0);
    match (to_rgb(highlight), to_rgb(base)) {
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

fn to_rgb(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Black => Some((0, 0, 0)),
        Color::Red => Some((170, 0, 0)),
        Color::Green => Some((0, 170, 0)),
        Color::Yellow => Some((170, 85, 0)),
        Color::Blue => Some((0, 0, 170)),
        Color::Magenta => Some((170, 0, 170)),
        Color::Cyan => Some((0, 170, 170)),
        Color::Gray => Some((170, 170, 170)),
        Color::DarkGray => Some((85, 85, 85)),
        Color::LightRed => Some((255, 85, 85)),
        Color::LightGreen => Some((85, 255, 85)),
        Color::LightYellow => Some((255, 255, 85)),
        Color::LightBlue => Some((85, 85, 255)),
        Color::LightMagenta => Some((255, 85, 255)),
        Color::LightCyan => Some((85, 255, 255)),
        Color::White => Some((255, 255, 255)),
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Indexed(value) => Some((value, value, value)),
        Color::Reset => None,
    }
}
