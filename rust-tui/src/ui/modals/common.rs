use crate::i18n::Locale;
use crate::theme::Theme;
use ratatui::layout::Rect;
use ratatui::{
    style::Style,
    widgets::{Block, Clear},
    Frame,
};

pub(super) fn render_modal_surface(f: &mut Frame, area: Rect, theme: &Theme) {
    f.render_widget(Clear, area);
    let surface = Block::default().style(Style::default().bg(theme.bg).fg(theme.fg));
    f.render_widget(surface, area);
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
    let suffix: String = input
        .chars()
        .rev()
        .take(back)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{}...{}", prefix, suffix)
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
