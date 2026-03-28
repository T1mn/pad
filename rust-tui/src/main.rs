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
mod telegram;
mod theme;
mod thread_meta;
mod tmux_dispatch;
mod tree;
mod ui;

use app::App;
use scanner::scan_panels;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("pad - Tmux Agent Panel Manager");
        println!();
        println!("Usage: pad [OPTIONS]");
        println!("       pad telegram-bot");
        println!();
        println!("Options:");
        println!("  -h, --help     显示帮助信息");
        println!("  -V, --version  显示版本号");
        println!("  -d, --debug    调试模式 (日志写入 ~/.pad/logs/pad.log)");
        println!();
        println!("快捷键:");
        println!("  j/k or ↑/↓     上下导航");
        println!("  1-9            跳转到面板");
        println!("  Enter          进入面板 (F12 / Ctrl+Q 返回)");
        println!("  t              文件树");
        println!("  Space          展开/折叠目录");
        println!("  /              搜索");
        println!("  ?              帮助");
        println!("  r              刷新");
        println!("  c              创建新会话");
        println!("  d              删除面板");
        println!("  F1             设置");
        println!("  q              退出");
        return Ok(());
    }

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("pad 0.6.0");
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

    let res = event::run_app(&mut terminal, &mut app).await;

    // Clean up any temporary tmux bindings before restoring terminal
    if app.same_session_attached {
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
