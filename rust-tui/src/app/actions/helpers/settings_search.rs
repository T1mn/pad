use super::*;

pub(crate) fn settings_item_search_blob(
    locale: Locale,
    id: &str,
    value: &str,
    name_key: &str,
    desc_key: &str,
) -> String {
    let mut terms = vec![
        id.to_lowercase(),
        id.replace('_', " ").to_lowercase(),
        name_key.to_lowercase(),
        name_key.replace(['.', '_'], " ").to_lowercase(),
        desc_key.to_lowercase(),
        desc_key.replace(['.', '_'], " ").to_lowercase(),
        value.to_lowercase(),
        crate::i18n::t(locale, name_key).to_lowercase(),
        crate::i18n::t(locale, desc_key).to_lowercase(),
        crate::i18n::t(Locale::En, name_key).to_lowercase(),
        crate::i18n::t(Locale::En, desc_key).to_lowercase(),
    ];
    terms.extend(
        settings_item_aliases(id)
            .iter()
            .map(|alias| alias.to_string()),
    );
    terms.join(" ")
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
        "relay" => &["relay", "provider", "model provider", "proxy"],
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
