use crate::app::App;
use crate::i18n::t;
use crate::ui::selection::{render::render_selection_surface, SelectionItem, SelectionState};
use ratatui::layout::Rect;
use ratatui::Frame;

pub(super) fn draw_theme_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let items: Vec<SelectionItem> = App::available_themes()
        .iter()
        .map(|(name, desc)| {
            let is_current = *name == app.config.theme;
            SelectionItem {
                title: if is_current {
                    format!("✓ {}", name)
                } else {
                    name.to_string()
                },
                value: None,
                subtitle: Some(if is_current {
                    format!("{}  ·  current", desc)
                } else {
                    (*desc).to_string()
                }),
                keyword: Some(format!("{} {}", name, desc)),
                detail: None,
                disabled: false,
            }
        })
        .collect();
    let mut state = SelectionState {
        selected: app.theme_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(
        f,
        area,
        theme,
        &format!("{} [{}]", t(locale, "settings.theme"), app.theme.name),
        &items,
        &state,
        Some("j/k move · Enter apply · Esc back"),
    );
}

pub(super) fn draw_language_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let current_locale = crate::i18n::Locale::from_str(&app.config.language);
    let items: Vec<SelectionItem> = App::available_locales()
        .iter()
        .map(|entry| {
            let is_current = *entry == current_locale;
            SelectionItem {
                title: if is_current {
                    format!("✓ {}", entry.display_name())
                } else {
                    entry.display_name().to_string()
                },
                value: None,
                subtitle: Some(entry.as_str().to_string()),
                keyword: Some(format!("{} {}", entry.display_name(), entry.as_str())),
                detail: None,
                disabled: false,
            }
        })
        .collect();
    let mut state = SelectionState {
        selected: app.language_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(
        f,
        area,
        theme,
        t(locale, "settings.language"),
        &items,
        &state,
        Some("j/k move · Enter apply · Esc back"),
    );
}

pub(super) fn draw_agent_style_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let style = &app.config.desired_agent_style;

    let zoom_desc = if style.zoom == "auto" {
        "agent_style.desc_zoom_auto"
    } else {
        "agent_style.desc_zoom_keep"
    };
    let status_desc = match style.status.as_str() {
        "show" => "agent_style.desc_status_show",
        "hide" => "agent_style.desc_status_hide",
        _ => "agent_style.desc_status_keep",
    };
    let items: Vec<SelectionItem> = [
        ("agent_style.zoom", style.zoom.as_str(), zoom_desc),
        ("agent_style.status", style.status.as_str(), status_desc),
    ]
    .iter()
    .map(|(name_key, cur_val, desc_key)| {
        let val_display = match *cur_val {
            "auto" => t(locale, "agent_style.zoom_auto"),
            "show" => t(locale, "agent_style.status_show"),
            "hide" => t(locale, "agent_style.status_hide"),
            "keep" => {
                if *name_key == "agent_style.zoom" {
                    t(locale, "agent_style.zoom_keep")
                } else {
                    t(locale, "agent_style.status_keep")
                }
            }
            other => other,
        };
        SelectionItem {
            title: t(locale, name_key).to_string(),
            value: None,
            subtitle: Some(format!("{}  ·  {}", val_display, t(locale, desc_key))),
            keyword: Some(format!(
                "{} {} {}",
                t(locale, name_key),
                val_display,
                t(locale, desc_key)
            )),
            detail: None,
            disabled: false,
        }
    })
    .collect();
    let mut state = SelectionState {
        selected: app.agent_style_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(
        f,
        area,
        theme,
        t(locale, "agent_style.title"),
        &items,
        &state,
        Some("j/k move · Enter/Space toggle · Esc back"),
    );
}

pub(super) fn draw_codex_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let items: Vec<SelectionItem> = [
        (
            "settings.codex_yolo",
            if app.config.agent_permissions.codex_auto_full_access {
                t(locale, "settings.on")
            } else {
                t(locale, "settings.off")
            },
            "settings.codex_yolo_desc",
        ),
        (
            "settings.codex_fast",
            if app.config.codex.fast_mode {
                t(locale, "settings.on")
            } else {
                t(locale, "settings.off")
            },
            "settings.codex_fast_desc",
        ),
        (
            "settings.codex_goals",
            if app.config.codex.goals {
                t(locale, "settings.on")
            } else {
                t(locale, "settings.off")
            },
            "settings.codex_goals_desc",
        ),
        (
            "settings.codex_multi_agent",
            if app.config.codex.multi_agent {
                t(locale, "settings.on")
            } else {
                t(locale, "settings.off")
            },
            "settings.codex_multi_agent_desc",
        ),
        (
            "settings.codex_web_search",
            t(
                locale,
                match app.config.codex.web_search.as_str() {
                    "cached" => "settings.codex_web_search_cached",
                    "live" => "settings.codex_web_search_live",
                    "disabled" => "settings.codex_web_search_disabled",
                    _ => "settings.codex_web_search_default",
                },
            ),
            "settings.codex_web_search_desc",
        ),
        (
            "settings.codex_status_model",
            if app.config.codex.status_line_model_with_reasoning {
                t(locale, "settings.on")
            } else {
                t(locale, "settings.off")
            },
            "settings.codex_status_model_desc",
        ),
        (
            "settings.codex_status_context",
            if app.config.codex.status_line_context_remaining {
                t(locale, "settings.on")
            } else {
                t(locale, "settings.off")
            },
            "settings.codex_status_context_desc",
        ),
        (
            "settings.codex_status_current_dir",
            if app.config.codex.status_line_current_dir {
                t(locale, "settings.on")
            } else {
                t(locale, "settings.off")
            },
            "settings.codex_status_current_dir_desc",
        ),
        (
            "settings.codex_jailbreak_prompt_file",
            if app.config.codex.jailbreak_prompt_file {
                t(locale, "settings.on")
            } else {
                t(locale, "settings.off")
            },
            "settings.codex_jailbreak_prompt_file_desc",
        ),
        (
            "settings.codex_index_prompt_file",
            if app.config.codex.index_prompt_file {
                t(locale, "settings.on")
            } else {
                t(locale, "settings.off")
            },
            "settings.codex_index_prompt_file_desc",
        ),
        (
            "settings.codex_title_summary",
            if app.config.codex.title_summary {
                t(locale, "settings.on")
            } else {
                t(locale, "settings.off")
            },
            "settings.codex_title_summary_desc",
        ),
        (
            "settings.codex_qa_preview",
            if app.config.codex.show_qa_preview {
                t(locale, "settings.on")
            } else {
                t(locale, "settings.off")
            },
            "settings.codex_qa_preview_desc",
        ),
    ]
    .iter()
    .map(|(name_key, value, desc_key)| SelectionItem {
        title: t(locale, name_key).to_string(),
        value: None,
        subtitle: Some(format!("{}  ·  {}", value, t(locale, desc_key))),
        keyword: Some(format!(
            "{} {} {}",
            t(locale, name_key),
            value,
            t(locale, desc_key)
        )),
        detail: None,
        disabled: false,
    })
    .chain(std::iter::once(SelectionItem {
        title: t(locale, "settings.codex_cli_version").to_string(),
        value: None,
        subtitle: Some(codex_cli_version_summary(app)),
        keyword: Some(format!(
            "{} {}",
            t(locale, "settings.codex_cli_version"),
            codex_cli_version_summary(app)
        )),
        detail: None,
        disabled: false,
    }))
    .collect();
    let mut state = SelectionState {
        selected: app.codex_settings_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    let footer = if app.codex_settings_selected == 12 {
        "j/k move · Enter check · u update · Esc back"
    } else {
        "j/k move · Enter/Space toggle or cycle · Esc back"
    };
    render_selection_surface(
        f,
        area,
        theme,
        t(locale, "settings.codex_settings"),
        &items,
        &state,
        Some(footer),
    );
}

fn codex_cli_version_summary(app: &App) -> String {
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

pub(super) fn draw_sound_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let sound = &app.config.sound;
    let preset_name = |preset_id: &str| {
        crate::sound::preset(preset_id)
            .map(|preset| t(locale, preset.name_key).to_string())
            .unwrap_or_else(|| preset_id.to_string())
    };
    let preset_desc = |preset_id: &str| {
        crate::sound::preset(preset_id)
            .map(|preset| t(locale, preset.desc_key).to_string())
            .unwrap_or_else(|| t(locale, "sound.preset_desc").to_string())
    };
    let items: Vec<SelectionItem> = [
        (
            t(locale, "sound.enabled").to_string(),
            if sound.enabled {
                t(locale, "settings.on").to_string()
            } else {
                t(locale, "settings.off").to_string()
            },
            t(locale, "sound.enabled_desc").to_string(),
        ),
        (
            t(locale, "sound.event.completion").to_string(),
            if sound.completion.enabled {
                t(locale, "settings.on").to_string()
            } else {
                t(locale, "settings.off").to_string()
            },
            t(locale, "sound.event.completion_desc").to_string(),
        ),
        (
            t(locale, "sound.preset").to_string(),
            preset_name(&sound.completion.preset),
            preset_desc(&sound.completion.preset),
        ),
        (
            t(locale, "sound.event.approval").to_string(),
            if sound.approval.enabled {
                t(locale, "settings.on").to_string()
            } else {
                t(locale, "settings.off").to_string()
            },
            t(locale, "sound.event.approval_desc").to_string(),
        ),
        (
            t(locale, "sound.preset").to_string(),
            preset_name(&sound.approval.preset),
            preset_desc(&sound.approval.preset),
        ),
        (
            t(locale, "sound.event.timeout").to_string(),
            if sound.timeout.enabled {
                t(locale, "settings.on").to_string()
            } else {
                t(locale, "settings.off").to_string()
            },
            t(locale, "sound.event.timeout_desc").to_string(),
        ),
        (
            t(locale, "sound.preset").to_string(),
            preset_name(&sound.timeout.preset),
            preset_desc(&sound.timeout.preset),
        ),
        (
            t(locale, "sound.event.failure").to_string(),
            if sound.failure.enabled {
                t(locale, "settings.on").to_string()
            } else {
                t(locale, "settings.off").to_string()
            },
            t(locale, "sound.event.failure_desc").to_string(),
        ),
        (
            t(locale, "sound.preset").to_string(),
            preset_name(&sound.failure.preset),
            preset_desc(&sound.failure.preset),
        ),
    ]
    .iter()
    .map(|(name, value, desc)| SelectionItem {
        title: name.clone(),
        value: None,
        subtitle: Some(format!("{value}  ·  {desc}")),
        keyword: Some(format!("{name} {value} {desc}")),
        detail: None,
        disabled: false,
    })
    .collect();

    let mut state = SelectionState {
        selected: app.sound_settings_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(
        f,
        area,
        theme,
        t(locale, "settings.sound"),
        &items,
        &state,
        Some("j/k move · Enter toggle/cycle · Space preview/toggle · Esc back"),
    );
}
