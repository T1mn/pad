use super::{
    items::{on_off, web_search_label},
    version,
};
use crate::app::state::CodexSettingsView;
use crate::app::App;
use crate::i18n::t;
use crate::ui::selection::SelectionItem;

pub(super) fn category_items(app: &App) -> Vec<SelectionItem> {
    (0..CodexSettingsView::CATEGORY_COUNT)
        .map(|index| SelectionItem {
            title: format!("› {}", category_title(app, index)),
            value: None,
            subtitle: Some(category_summary(app, index)),
            keyword: Some(format!(
                "{} {}",
                category_title(app, index),
                category_summary(app, index)
            )),
            detail: None,
            disabled: false,
        })
        .collect()
}

pub(super) fn category_title(app: &App, index: usize) -> String {
    let key = match index {
        0 => "settings.codex_category_runtime",
        1 => "settings.codex_category_status_line",
        2 => "settings.codex_category_prompts",
        3 => "settings.codex_category_preview",
        _ => "settings.codex_category_cli",
    };
    t(app.locale, key).to_string()
}

fn category_summary(app: &App, index: usize) -> String {
    match index {
        0 => runtime_summary(app),
        1 => status_line_summary(app),
        2 => prompt_summary(app),
        3 => preview_summary(app),
        _ => version::codex_cli_version_summary(app),
    }
}

fn runtime_summary(app: &App) -> String {
    format!(
        "YOLO {} · Fast {} · Goal {} · MA {} · Web {}",
        on_off(app, app.config.agent_permissions.codex_auto_full_access),
        on_off(app, app.config.codex.fast_mode),
        on_off(app, app.config.codex.goals),
        on_off(app, app.config.codex.multi_agent),
        web_search_label(app)
    )
}

fn status_line_summary(app: &App) -> String {
    format!(
        "{}/3 · Model {} · Context {} · Dir {}",
        app.config.codex.status_line_items().len(),
        on_off(app, app.config.codex.status_line_model_with_reasoning),
        on_off(app, app.config.codex.status_line_context_remaining),
        on_off(app, app.config.codex.status_line_current_dir)
    )
}

fn prompt_summary(app: &App) -> String {
    format!(
        "Jailbreak {} · Index {}",
        on_off(app, app.config.codex.jailbreak_prompt_file),
        on_off(app, app.config.codex.index_prompt_file)
    )
}

fn preview_summary(app: &App) -> String {
    format!(
        "Summary {} · Q/A {}",
        on_off(app, app.config.codex.title_summary),
        on_off(app, app.config.codex.show_qa_preview)
    )
}
