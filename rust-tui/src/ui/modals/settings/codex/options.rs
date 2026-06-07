use super::{
    items::{switch_item, value_item, web_search_label},
    version,
};
use crate::app::App;
use crate::i18n::t;
use crate::ui::selection::SelectionItem;

pub(super) fn runtime_items(app: &App) -> Vec<SelectionItem> {
    vec![
        switch_item(
            app,
            "settings.codex_yolo",
            app.config.agent_permissions.codex_auto_full_access,
            "settings.codex_yolo_desc",
        ),
        switch_item(
            app,
            "settings.codex_fast",
            app.config.codex.fast_mode,
            "settings.codex_fast_desc",
        ),
        switch_item(
            app,
            "settings.codex_goals",
            app.config.codex.goals,
            "settings.codex_goals_desc",
        ),
        switch_item(
            app,
            "settings.codex_multi_agent",
            app.config.codex.multi_agent,
            "settings.codex_multi_agent_desc",
        ),
        value_item(
            app,
            "settings.codex_web_search",
            web_search_label(app),
            "settings.codex_web_search_desc",
        ),
    ]
}

pub(super) fn status_line_items(app: &App) -> Vec<SelectionItem> {
    vec![
        switch_item(
            app,
            "settings.codex_status_model",
            app.config.codex.status_line_model_with_reasoning,
            "settings.codex_status_model_desc",
        ),
        switch_item(
            app,
            "settings.codex_status_fast",
            app.config.codex.status_line_fast_mode,
            "settings.codex_status_fast_desc",
        ),
        switch_item(
            app,
            "settings.codex_status_context",
            app.config.codex.status_line_context_remaining,
            "settings.codex_status_context_desc",
        ),
        switch_item(
            app,
            "settings.codex_status_current_dir",
            app.config.codex.status_line_current_dir,
            "settings.codex_status_current_dir_desc",
        ),
    ]
}

pub(super) fn prompt_items(app: &App) -> Vec<SelectionItem> {
    vec![
        switch_item(
            app,
            "settings.codex_jailbreak_prompt_file",
            app.config.codex.jailbreak_prompt_file,
            "settings.codex_jailbreak_prompt_file_desc",
        ),
        switch_item(
            app,
            "settings.codex_index_prompt_file",
            app.config.codex.index_prompt_file,
            "settings.codex_index_prompt_file_desc",
        ),
    ]
}

pub(super) fn preview_items(app: &App) -> Vec<SelectionItem> {
    vec![
        switch_item(
            app,
            "settings.codex_title_summary",
            app.config.codex.title_summary,
            "settings.codex_title_summary_desc",
        ),
        switch_item(
            app,
            "settings.codex_qa_preview",
            app.config.codex.show_qa_preview,
            "settings.codex_qa_preview_desc",
        ),
    ]
}

pub(super) fn cli_items(app: &App) -> Vec<SelectionItem> {
    vec![SelectionItem {
        title: t(app.locale, "settings.codex_cli_version").to_string(),
        value: None,
        subtitle: Some(version::codex_cli_version_summary(app)),
        keyword: Some(format!(
            "{} {}",
            t(app.locale, "settings.codex_cli_version"),
            version::codex_cli_version_summary(app)
        )),
        detail: None,
        disabled: false,
    }]
}
