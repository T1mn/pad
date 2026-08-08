use super::super::provider::preview_provider_value;
use crate::app::App;
use crate::i18n::Locale;
use crate::sidebar::SidebarThread;

pub(super) struct InfoCardValues {
    pub(super) cache_badge_label: Option<&'static str>,
    pub(super) status_label: &'static str,
    pub(super) session_id: String,
    pub(super) path_text: String,
    pub(super) provider_text: String,
    pub(super) usage_text: String,
    pub(super) location: String,
    pub(super) share_url: String,
    pub(super) summary: String,
}

pub(super) fn build_info_card_values(
    app: &mut App,
    thread: &SidebarThread,
    value_width: usize,
) -> InfoCardValues {
    let locale = app.locale;
    InfoCardValues {
        cache_badge_label: cache_badge_label(app, thread, locale),
        status_label: super::super::super::session::localized_status_label(locale, &thread.state),
        session_id: app
            .preview
            .session_id
            .as_deref()
            .or(thread.session_id.as_deref())
            .unwrap_or("—")
            .to_string(),
        path_text: shortened_thread_path(thread, value_width),
        provider_text: preview_provider_value(app, thread),
        usage_text: preview_usage_value(thread),
        location: thread.live_location.as_deref().unwrap_or("—").to_string(),
        share_url: thread.share_url.as_deref().unwrap_or("—").to_string(),
        summary: if app.config.codex.title_summary {
            thread.generated_title.as_deref().unwrap_or("—").to_string()
        } else {
            "—".to_string()
        },
    }
}

fn cache_badge_label(app: &App, thread: &SidebarThread, locale: Locale) -> Option<&'static str> {
    if app.preview.source == crate::model::PreviewSource::Session
        && app.preview.session_origin != Some(crate::model::PreviewSessionOrigin::App)
        && thread.session_cache_state == Some(crate::model::SessionCacheState::Cached)
    {
        Some(crate::i18n::t(locale, "preview.session_cached"))
    } else {
        None
    }
}

pub(super) fn preview_usage_value(thread: &SidebarThread) -> String {
    match (thread.cost.as_deref(), thread.token_summary.as_deref()) {
        (Some(cost), Some(tokens)) => format!("{cost} · {tokens}"),
        (Some(cost), None) => cost.to_string(),
        (None, Some(tokens)) => tokens.to_string(),
        (None, None) => "—".to_string(),
    }
}

pub(super) fn shortened_thread_path(thread: &SidebarThread, max_len: usize) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let path = if thread.working_dir.starts_with(&home) {
        thread.working_dir.replacen(&home, "~", 1)
    } else {
        thread.working_dir.clone()
    };

    if path.len() <= max_len {
        return path;
    }

    if let Some((parent, leaf)) = trailing_path_segments(&path) {
        let short = format!("~/.../{parent}/{leaf}");
        if short.len() <= max_len {
            return short;
        }
    }

    let start = path
        .char_indices()
        .rev()
        .find(|(i, _)| path.len() - i <= max_len.saturating_sub(3))
        .map(|(i, _)| i)
        .unwrap_or(0);
    format!("...{}", &path[start..])
}

fn trailing_path_segments(path: &str) -> Option<(&str, &str)> {
    let (prefix, leaf) = path.rsplit_once('/')?;
    let parent = prefix.rsplit('/').next()?;
    Some((parent, leaf))
}

#[cfg(test)]
#[path = "values_tests.rs"]
mod tests;
