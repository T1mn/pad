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
