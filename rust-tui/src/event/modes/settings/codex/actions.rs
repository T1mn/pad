use crate::app::state::CodexSettingsView;
use crate::app::App;
use crate::relay;

pub(super) fn apply_selected_codex_action(app: &mut App) {
    match app.codex_settings_view {
        CodexSettingsView::Categories => enter_selected_category(app),
        CodexSettingsView::Runtime => apply_runtime_action(app),
        CodexSettingsView::StatusLine => apply_status_line_action(app),
        CodexSettingsView::Prompts => apply_prompt_action(app),
        CodexSettingsView::Preview => apply_preview_action(app),
        CodexSettingsView::Cli => app.trigger_codex_cli_version_check(),
    }
}

fn enter_selected_category(app: &mut App) {
    app.codex_settings_category_selected = app
        .codex_settings_selected
        .min(CodexSettingsView::CATEGORY_COUNT.saturating_sub(1));
    app.codex_settings_view =
        CodexSettingsView::from_category_index(app.codex_settings_category_selected);
    app.codex_settings_selected = 0;
    app.dirty = true;
}

fn apply_runtime_action(app: &mut App) {
    match app.codex_settings_selected {
        0 => {
            app.config.agent_permissions.codex_auto_full_access =
                !app.config.agent_permissions.codex_auto_full_access;
        }
        1 => {
            app.config.codex.fast_mode = !app.config.codex.fast_mode;
        }
        2 => {
            app.config.codex.goals = !app.config.codex.goals;
        }
        3 => {
            app.config.codex.multi_agent = !app.config.codex.multi_agent;
        }
        4 => {
            app.config.codex.web_search = match app.config.codex.web_search.as_str() {
                "cached" => "live".to_string(),
                "live" => "disabled".to_string(),
                "disabled" => "default".to_string(),
                _ => "cached".to_string(),
            };
        }
        _ => return,
    }
    persist_codex_runtime_config(app);
}

fn apply_status_line_action(app: &mut App) {
    match app.codex_settings_selected {
        0 => {
            app.config.codex.status_line_model_with_reasoning =
                !app.config.codex.status_line_model_with_reasoning;
        }
        1 => {
            app.config.codex.status_line_fast_mode = !app.config.codex.status_line_fast_mode;
        }
        2 => {
            app.config.codex.status_line_five_hour_limit =
                !app.config.codex.status_line_five_hour_limit;
        }
        3 => {
            app.config.codex.status_line_weekly_limit = !app.config.codex.status_line_weekly_limit;
        }
        4 => {
            app.config.codex.status_line_context_remaining =
                !app.config.codex.status_line_context_remaining;
        }
        5 => {
            app.config.codex.status_line_current_dir = !app.config.codex.status_line_current_dir;
        }
        _ => return,
    }
    persist_codex_runtime_config(app);
}

fn apply_prompt_action(app: &mut App) {
    match app.codex_settings_selected {
        0 => {
            app.config.codex.jailbreak_prompt_file = !app.config.codex.jailbreak_prompt_file;
        }
        1 => {
            app.config.codex.index_prompt_file = !app.config.codex.index_prompt_file;
        }
        _ => return,
    }
    persist_codex_runtime_config(app);
}

fn apply_preview_action(app: &mut App) {
    match app.codex_settings_selected {
        0 => {
            app.config.codex.title_summary = !app.config.codex.title_summary;
        }
        1 => {
            app.config.codex.show_qa_preview = !app.config.codex.show_qa_preview;
        }
        _ => return,
    }
    persist_codex_runtime_config(app);
}

fn persist_codex_runtime_config(app: &mut App) {
    app.config.save();
    relay::apply_runtime_configs(
        &app.config.agents,
        &app.config.agent_permissions,
        &app.config.codex,
    );
    app.dirty = true;
}
