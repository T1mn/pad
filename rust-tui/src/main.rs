use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::error::Error;
use std::io;

mod app;
mod event;
mod fuzzy;
#[macro_use]
mod logger;
mod model;
pub mod pty;
mod scanner;
mod session;
mod theme;
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
        println!();
        println!("Options:");
        println!("  -h, --help     显示帮助信息");
        println!("  -V, --version  显示版本号");
        println!("  -d, --debug    调试模式 (日志写入 ~/.config/pad/pad.log)");
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
        println!("pad 0.5.0");
        return Ok(());
    }

    let debug = args.iter().any(|a| a == "--debug" || a == "-d");
    if debug {
        logger::init()?;
        logger::log("pad 启动 (debug mode)");
    }

    // Install panic hook to restore terminal and log panic info
    std::panic::set_hook(Box::new(|info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        let msg = format!("PANIC: {}", info);
        eprintln!("{}", msg);
        logger::log(&msg);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    log_debug!("配置加载: theme={}, auto_refresh={}", app.config.theme, app.config.auto_refresh);

    match scan_panels() {
        Ok(panels) => {
            log_debug!("扫描到 {} 个面板", panels.len());
            app.panels = panels;
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
        DisableMouseCapture
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
