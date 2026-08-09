mod categories {
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
            "{}/6 · Model {} · Fast {} · 5h {} · Week {} · Context {} · Dir {}",
            app.config.codex.status_line_items().len(),
            on_off(app, app.config.codex.status_line_model_with_reasoning),
            on_off(app, app.config.codex.status_line_fast_mode),
            on_off(app, app.config.codex.status_line_five_hour_limit),
            on_off(app, app.config.codex.status_line_weekly_limit),
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
}
mod items {
    use crate::app::state::CodexSettingsView;
    use crate::app::App;
    use crate::i18n::t;
    use crate::ui::selection::SelectionItem;

    use super::{categories, options};

    pub(super) fn codex_items(app: &App) -> Vec<SelectionItem> {
        match app.codex_settings_view {
            CodexSettingsView::Categories => categories::category_items(app),
            CodexSettingsView::Runtime => options::runtime_items(app),
            CodexSettingsView::StatusLine => options::status_line_items(app),
            CodexSettingsView::Prompts => options::prompt_items(app),
            CodexSettingsView::Preview => options::preview_items(app),
            CodexSettingsView::Cli => options::cli_items(app),
        }
    }

    pub(super) fn codex_title(app: &App) -> String {
        let base = t(app.locale, "settings.codex_settings");
        match app.codex_settings_view {
            CodexSettingsView::Categories => base.to_string(),
            view => format!(
                "{} / {}",
                base,
                categories::category_title(app, view.category_index())
            ),
        }
    }

    pub(super) fn codex_footer(app: &App) -> &'static str {
        match app.codex_settings_view {
            CodexSettingsView::Categories => "j/k move · Enter open · Esc back",
            CodexSettingsView::Cli => "j/k move · Enter check · u update · h back · Esc back",
            _ => "j/k move · Enter/Space toggle or cycle · h back · Esc back",
        }
    }

    pub(super) fn switch_item(
        app: &App,
        name_key: &str,
        enabled: bool,
        desc_key: &str,
    ) -> SelectionItem {
        value_item(
            app,
            name_key,
            if enabled {
                t(app.locale, "settings.on").to_string()
            } else {
                t(app.locale, "settings.off").to_string()
            },
            desc_key,
        )
    }

    pub(super) fn value_item(
        app: &App,
        name_key: &str,
        value: String,
        desc_key: &str,
    ) -> SelectionItem {
        SelectionItem {
            title: t(app.locale, name_key).to_string(),
            value: None,
            subtitle: Some(format!("{}  ·  {}", value, t(app.locale, desc_key))),
            keyword: Some(format!(
                "{} {} {}",
                t(app.locale, name_key),
                value,
                t(app.locale, desc_key)
            )),
            disabled: false,
        }
    }

    pub(super) fn on_off(app: &App, enabled: bool) -> &'static str {
        if enabled {
            t(app.locale, "settings.on")
        } else {
            t(app.locale, "settings.off")
        }
    }

    pub(super) fn web_search_label(app: &App) -> String {
        t(
            app.locale,
            match app.config.codex.web_search.as_str() {
                "cached" => "settings.codex_web_search_cached",
                "live" => "settings.codex_web_search_live",
                "disabled" => "settings.codex_web_search_disabled",
                _ => "settings.codex_web_search_default",
            },
        )
        .to_string()
    }
}
mod options;
mod version {
    use crate::app::App;

    pub(super) fn codex_cli_version_summary(app: &App) -> String {
        let zh = matches!(
            app.locale,
            crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
        );

        if app.codex_cli_check_in_progress {
            return if zh {
                "检查中：本地版本 / 最新版本".to_string()
            } else {
                "Checking local / latest versions".to_string()
            };
        }

        if app.codex_cli_update_in_progress {
            return if zh {
                "升级中：npm install -g @openai/codex@latest".to_string()
            } else {
                "Updating via npm install -g @openai/codex@latest".to_string()
            };
        }

        let Some(info) = app.codex_cli_version_info.as_ref() else {
            return if zh {
                "按 Enter 检查本地 / 最新版本".to_string()
            } else {
                "Press Enter to check local / latest versions".to_string()
            };
        };

        match (
            info.binary_path.as_ref(),
            info.local_version.as_ref(),
            info.latest_version.as_ref(),
        ) {
            (_, Some(local), Some(latest)) if local == latest => {
                if zh {
                    format!("本地 {local} · 已是最新")
                } else {
                    format!("Local {local} · up to date")
                }
            }
            (_, Some(local), Some(latest)) => {
                if zh {
                    format!("本地 {local} · 最新 {latest}")
                } else {
                    format!("Local {local} · latest {latest}")
                }
            }
            (_, Some(local), None) => {
                if zh {
                    format!("本地 {local} · 无法获取最新版本")
                } else {
                    format!("Local {local} · latest unknown")
                }
            }
            (Some(_), None, Some(latest)) => {
                if zh {
                    format!("已检测到 codex · 最新 {latest}")
                } else {
                    format!("Codex found · latest {latest}")
                }
            }
            (None, None, Some(latest)) => {
                if zh {
                    format!("未找到 codex · 最新 {latest}")
                } else {
                    format!("Codex not found · latest {latest}")
                }
            }
            (Some(_), None, None) => {
                if zh {
                    "已检测到 codex · 版本未知".to_string()
                } else {
                    "Codex found · version unknown".to_string()
                }
            }
            (None, None, None) => {
                if zh {
                    "未找到 codex / npm".to_string()
                } else {
                    "Codex / npm not found".to_string()
                }
            }
        }
    }
}

use crate::app::App;
use crate::ui::selection::{render::render_selection_surface, SelectionState};
use ratatui::layout::Rect;
use ratatui::Frame;

pub(super) fn draw_codex_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let items = items::codex_items(app);
    let mut state = SelectionState {
        selected: app.codex_settings_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(
        f,
        area,
        theme,
        &items::codex_title(app),
        &items,
        &state,
        Some(items::codex_footer(app)),
    );
}
