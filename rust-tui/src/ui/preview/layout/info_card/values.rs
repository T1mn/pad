use super::super::provider::preview_provider_value;
use crate::app::App;
use crate::i18n::Locale;
use crate::sidebar::SidebarThread;

pub(super) struct InfoCardValues {
    pub(super) cache_badge_label: Option<&'static str>,
    pub(super) status_label: &'static str,
    pub(super) branch: Option<String>,
    pub(super) git_text: String,
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
    let branch = thread
        .git_info
        .as_ref()
        .and_then(|git| git.branch.as_deref());

    InfoCardValues {
        cache_badge_label: cache_badge_label(app, thread, locale),
        status_label: super::super::super::session::localized_status_label(locale, &thread.state),
        branch: branch.map(str::to_string),
        git_text: preview_git_value(app, thread),
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

fn preview_git_value(app: &App, thread: &SidebarThread) -> String {
    if let Some(panel) = thread
        .live_pane_id
        .as_deref()
        .and_then(|pane_id| app.panels.iter().find(|panel| panel.pane_id == pane_id))
    {
        return panel.git_display();
    }

    if let Some(git) = thread.git_info.as_ref() {
        let branch = git.branch.as_deref().unwrap_or("?");
        let commit = git.commit.as_deref().unwrap_or("?");
        format!(
            "{}@{}",
            branch,
            super::super::super::common::truncate_to_width(commit, 7)
        )
    } else {
        String::from("—")
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

    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 2 {
        let short = format!(
            "~/.../{}/{}",
            parts[parts.len() - 2],
            parts[parts.len() - 1]
        );
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
