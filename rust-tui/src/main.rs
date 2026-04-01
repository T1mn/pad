use crossterm::{
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::error::Error;
use std::io;

mod app;
mod chat;
mod claude_history;
mod codex_state;
mod detector;
mod event;
mod fuzzy;
mod gemini_history;
mod hook;
mod i18n;
#[macro_use]
mod logger;
mod model;
mod notify;
mod paths;
mod pipe;
mod preview_source;
pub mod pty;
mod relay;
mod runtime_status;
mod scanner;
mod session;
mod session_cache;
mod sidebar;
mod system_check;
mod telegram;
#[cfg(test)]
mod test_support;
mod theme;
mod thread_meta;
mod tmux_dispatch;
mod tree;
mod ui;

use app::App;
use scanner::scan_panels;

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT handler");
    let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    let mut sighup = signal(SignalKind::hangup()).expect("install SIGHUP handler");

    tokio::select! {
        _ = sigint.recv() => {}
        _ = sigterm.recv() => {}
        _ = sighup.recv() => {}
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

fn should_restore_tmux_state(app: &App) -> bool {
    app.same_session_attached
        || !app.saved_tmux_bindings.is_empty()
        || app.saved_tmux_status.is_some()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("PAD - Panel for Agent Development");
        println!();
        println!("Usage: pad [OPTIONS]");
        println!("       pad telegram-bot");
        println!();
        println!("Options:");
        println!("  -h, --help     Show help");
        println!("  -V, --version  Show version");
        println!("  -d, --debug    Enable debug logging (~/.pad/logs/pad.log)");
        println!();
        println!("Key bindings:");
        println!("  j/k or ↑/↓     Move selection");
        println!("  1-9            Jump to visible session");
        println!("  Enter          Attach to panel (F12 / Ctrl+Q to return)");
        println!("  t              Toggle file tree");
        println!("  Space          Expand or collapse directory");
        println!("  /              Open settings");
        println!("  ?              Help");
        println!("  r              Refresh");
        println!("  c              Create session");
        println!("  d              Delete panel");
        println!("  F1             Settings");
        println!("  q              Quit");
        return Ok(());
    }

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("pad {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let telegram_daemon = args.iter().any(|a| a == "telegram-bot");
    let debug = args.iter().any(|a| a == "--debug" || a == "-d");
    paths::ensure_runtime_layout()?;
    if telegram_daemon {
        logger::init_with_path(paths::telegram_bot_log_path())?;
    } else {
        logger::init()?;
    }
    if debug {
        logger::log("pad 启动 (debug mode)");
    } else if telegram_daemon {
        logger::log("telegram-bot 启动");
    } else {
        logger::log("pad 启动");
    }

    if telegram_daemon {
        return telegram::run_daemon().await;
    }

    system_check::ensure_tmux_available()?;

    // Install panic hook to restore terminal and log panic info
    std::panic::set_hook(Box::new(|info| {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            DisableFocusChange,
            DisableBracketedPaste
        );
        let msg = format!("PANIC: {}", info);
        eprintln!("{}", msg);
        logger::log(&msg);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableFocusChange,
        EnableBracketedPaste
    )?;
    // Ensure tmux sends focus events to the terminal
    let _ = std::process::Command::new("tmux")
        .args(["set", "-g", "focus-events", "on"])
        .output();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let _status_guard = runtime_status::StatusGuard::new(crate::paths::pad_status_path(), "pad")?;
    let mut app = App::new();
    relay::apply_runtime_configs(&app.config.agents, &app.config.agent_permissions);
    if let Err(err) = telegram::sync_daemon(&app.config) {
        log_debug!("telegram: daemon sync failed during pad startup: {}", err);
    }
    app.hook_rx = Some(hook::start_hook_listener());
    log_debug!(
        "配置加载: theme={}, auto_refresh={}",
        app.config.theme,
        app.config.auto_refresh
    );

    match scan_panels() {
        Ok(panels) => {
            log_debug!("扫描到 {} 个面板", panels.len());
            app.panels = panels;
            if let Err(err) = session_cache::preload_panels(&mut app.panels) {
                log_debug!("session_cache: preload failed: {}", err);
            }
        }
        Err(e) => {
            log_debug!("扫描失败: {}", e);
        }
    }

    let res = {
        let run_app = event::run_app(&mut terminal, &mut app);
        tokio::pin!(run_app);

        tokio::select! {
            res = &mut run_app => res,
            _ = shutdown_signal() => {
                log_debug!("收到终止信号，开始清理退出");
                Ok(())
            }
        }
    };

    // Clean up any temporary tmux bindings before restoring terminal
    if should_restore_tmux_state(&app) {
        event::restore_tmux_bindings(&mut app);
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableFocusChange,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;

    if let Err(ref err) = res {
        log_debug!("退出错误: {:?}", err);
        println!("{:?}", err);
    } else {
        log_debug!("pad 正常退出");
    }

    res?;
    Ok(())
}
