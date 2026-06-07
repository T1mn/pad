use crate::app::App;
use std::error::Error;

pub fn start_runtime_services(app: &mut App) -> Result<(), Box<dyn Error>> {
    crate::relay::apply_runtime_configs(
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
    if let Err(err) = crate::socket_api::start_api_listener() {
        log_debug!("socket_api: listener not started: {}", err);
    }
    log_debug!(
        "配置加载: theme={}, auto_refresh={}",
        app.config.theme,
        app.config.auto_refresh
    );
    Ok(())
}

pub fn load_initial_panels(app: &mut App) {
    match crate::scanner::scan_panels() {
        Ok(panels) => {
            log_debug!("扫描到 {} 个面板", panels.len());
            app.panels = panels;
            if let Err(err) = crate::session_cache::preload_panels(&mut app.panels) {
                log_debug!("session_cache: preload failed: {}", err);
            }
        }
        Err(e) => {
            log_debug!("扫描失败: {}", e);
        }
    }
    app.seed_startup_thread_sort_activity_once();
}
