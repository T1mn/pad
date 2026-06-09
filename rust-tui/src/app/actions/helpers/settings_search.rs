use super::*;
use crate::text_match::contains_ignore_case;

pub(crate) fn settings_item_search_blob(
    locale: Locale,
    id: &str,
    value: &str,
    name_key: &str,
    desc_key: &str,
) -> String {
    let mut blob = String::new();
    push_search_term(&mut blob, id);
    push_normalized_search_term(&mut blob, id, &['_']);
    push_search_term(&mut blob, name_key);
    push_normalized_search_term(&mut blob, name_key, &['.', '_']);
    push_search_term(&mut blob, desc_key);
    push_normalized_search_term(&mut blob, desc_key, &['.', '_']);
    push_search_term(&mut blob, value);
    push_search_term(&mut blob, crate::i18n::t(locale, name_key));
    push_search_term(&mut blob, crate::i18n::t(locale, desc_key));
    push_search_term(&mut blob, crate::i18n::t(Locale::En, name_key));
    push_search_term(&mut blob, crate::i18n::t(Locale::En, desc_key));
    for alias in settings_item_aliases(id) {
        push_search_term(&mut blob, alias);
    }
    blob
}

pub(crate) fn settings_item_matches_search(
    locale: Locale,
    id: &str,
    value: &str,
    name_key: &str,
    desc_key: &str,
    query: &str,
) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }

    if query.split_whitespace().nth(1).is_some() {
        let blob = settings_item_search_blob(locale, id, value, name_key, desc_key);
        return query
            .split_whitespace()
            .all(|token| term_matches(&blob, token));
    }

    term_matches(id, query)
        || normalized_term_matches(id, query, &['_'])
        || term_matches(name_key, query)
        || normalized_term_matches(name_key, query, &['.', '_'])
        || term_matches(desc_key, query)
        || normalized_term_matches(desc_key, query, &['.', '_'])
        || term_matches(value, query)
        || term_matches(crate::i18n::t(locale, name_key), query)
        || term_matches(crate::i18n::t(locale, desc_key), query)
        || term_matches(crate::i18n::t(Locale::En, name_key), query)
        || term_matches(crate::i18n::t(Locale::En, desc_key), query)
        || settings_item_aliases(id)
            .iter()
            .any(|alias| term_matches(alias, query))
}

fn term_matches(term: &str, query: &str) -> bool {
    contains_ignore_case(term, query)
}

fn normalized_term_matches(term: &str, query: &str, separators: &[char]) -> bool {
    let normalized = term.replace(separators, " ");
    term_matches(&normalized, query)
}

fn push_search_term(blob: &mut String, term: &str) {
    if !blob.is_empty() {
        blob.push(' ');
    }
    blob.push_str(term);
}

fn push_normalized_search_term(blob: &mut String, term: &str, separators: &[char]) {
    if !blob.is_empty() {
        blob.push(' ');
    }
    for ch in term.chars() {
        if separators.contains(&ch) {
            blob.push(' ');
        } else {
            blob.push(ch);
        }
    }
}

fn settings_item_aliases(id: &str) -> &'static [&'static str] {
    match id {
        "theme" => &["theme", "color theme", "appearance"],
        "auto_refresh" => &["auto refresh", "refresh", "refresh interval"],
        "codex_settings" => &[
            "codex",
            "codex settings",
            "codex full access",
            "codex permissions",
            "approval policy",
            "sandbox mode",
            "yolo",
            "fast",
            "goal",
            "goals",
            "/goal",
            "service tier",
            "multi agent",
            "subagents",
            "web search",
            "search mode",
            "codex runtime",
            "codex status line",
            "codex status",
            "codex prompts",
            "codex prompt",
            "codex preview",
            "codex cli",
            "codex version",
        ],
        "claude_full_access" => &[
            "claude",
            "claude full access",
            "claude permissions",
            "bypass permissions",
            "sandbox",
        ],
        "sound" => &[
            "sound",
            "audio",
            "notification sound",
            "completion sound",
            "approval sound",
            "timeout sound",
            "failure sound",
            "beep",
            "chime",
        ],
        "relay" => &[
            "relay",
            "provider",
            "model provider",
            "proxy",
            "codex relay",
            "claude relay",
            "gemini relay",
            "opencode relay",
            "codex provider",
            "claude provider",
            "gemini provider",
            "opencode provider",
        ],
        "telegram" => &["telegram", "bot", "telegram bot"],
        "agent_style" => &["agent style", "attach style", "status bar", "zoom"],
        "preview_mode" => &[
            "preview",
            "preview mode",
            "preview source",
            "session preview",
        ],
        "display_mode" => &[
            "display",
            "display mode",
            "display settings",
            "session scope",
        ],
        "trash" => &["trash", "recycle bin", "deleted", "deleted threads"],
        "language" => &["language", "locale"],
        "version" => &["version", "about"],
        _ => &[],
    }
}
