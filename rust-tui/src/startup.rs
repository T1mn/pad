use crate::app::App;
use std::error::Error;

pub fn prepare_runtime_environment(
    telegram_daemon: bool,
    debug: bool,
) -> Result<(), Box<dyn Error>> {
    crate::paths::ensure_runtime_layout()?;
    if telegram_daemon {
        crate::logger::init_with_path(crate::paths::telegram_bot_log_path())?;
    } else {
        crate::logger::init()?;
    }

    if debug {
        crate::logger::log("pad 启动 (debug mode)");
    } else if telegram_daemon {
        crate::logger::log("telegram-bot 启动");
    } else {
        crate::logger::log("pad 启动");
    }
    crate::paths::log_runtime_layout_status();
    Ok(())
}

pub fn start_runtime_services(app: &mut App) -> Result<(), Box<dyn Error>> {
    // Do not rewrite provider live configs on launch; only apply permission overlays.
    // Relay sync happens when the user edits relay settings, reloads config, or launches an agent.
    crate::relay::apply_runtime_overlays(
        &app.config.agents,
        &app.config.agent_permissions,
        &app.config.codex,
    );
    if let Err(err) = crate::telegram::ensure_embedded_daemon_running() {
        log_debug!(
            "telegram: embedded daemon start failed during pad startup: {}",
            err
        );
    }
    app.hook_rx = Some(crate::hook::start_hook_listener()?);
    match crate::socket_api::start_api_listener() {
        Ok(receiver) => app.api_rx = Some(receiver),
        Err(err) => log_debug!("socket_api: listener not started: {}", err),
    }
    log_debug!(
        "配置加载: theme={}, auto_refresh={}",
        app.config.theme,
        app.config.auto_refresh
    );
    Ok(())
}
