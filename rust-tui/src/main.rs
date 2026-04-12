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
use std::io::{self, IsTerminal};
use std::process::Command;

mod app;
mod chat;
mod claude_history;
mod codex_provider_sync;
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
mod title_summary;
mod tmux_capabilities;
mod tmux_dispatch;
mod tree;
mod ui;

use app::App;
use scanner::scan_panels;

const PAD_BOOTSTRAP_ENV: &str = "PAD_TMUX_BOOTSTRAPPED";
const PAD_DEFAULT_SESSION: &str = "pad";

#[cfg(unix)]
async fn shutdown_signal() -> &'static str {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT handler");
    let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    let mut sighup = signal(SignalKind::hangup()).expect("install SIGHUP handler");
    let mut sigpipe = signal(SignalKind::pipe()).expect("install SIGPIPE handler");

    tokio::select! {
        _ = sigint.recv() => "SIGINT",
        _ = sigterm.recv() => "SIGTERM",
        _ = sighup.recv() => "SIGHUP",
        _ = sigpipe.recv() => "SIGPIPE",
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() -> &'static str {
    let _ = tokio::signal::ctrl_c().await;
    "CTRL_C"
}

fn should_restore_tmux_state(app: &App) -> bool {
    app.same_session_attached
        || !app.saved_tmux_bindings.is_empty()
        || app.saved_tmux_status.is_some()
}

fn is_info_only_command(args: &[String]) -> bool {
    args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--help" | "-h" | "--version" | "-V" | "--tmux-doctor"
        )
    })
}

fn is_telegram_daemon_command(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "telegram-bot")
}

fn should_bootstrap_into_tmux(
    args: &[String],
    tmux_env_present: bool,
    tmux_pane_present: bool,
    already_bootstrapped: bool,
    stdin_is_tty: bool,
    stdout_is_tty: bool,
) -> bool {
    if is_info_only_command(args) || is_telegram_daemon_command(args) {
        return false;
    }
    if already_bootstrapped {
        return false;
    }
    if tmux_env_present && tmux_pane_present {
        return false;
    }
    stdin_is_tty && stdout_is_tty
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

fn bootstrap_command(args: &[String], executable: &std::path::Path) -> String {
    let mut parts = vec![
        "env".to_string(),
        format!("{PAD_BOOTSTRAP_ENV}=1"),
        shell_single_quote(&executable.to_string_lossy()),
    ];
    for arg in args.iter().skip(1) {
        parts.push(shell_single_quote(arg));
    }
    parts.join(" ")
}

fn bootstrap_into_tmux(args: &[String]) -> Result<(), Box<dyn Error>> {
    let executable = std::env::current_exe()?;
    let inner_command = bootstrap_command(args, &executable);
    let mut command = Command::new("tmux");
    command.args([
        "new-session",
        "-A",
        "-s",
        PAD_DEFAULT_SESSION,
        &inner_command,
    ]);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        let err = command.exec();
        Err(Box::new(err))
    }

    #[cfg(not(unix))]
    {
        let status = command.status()?;
        std::process::exit(status.code().unwrap_or(1));
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    if is_info_only_command(&args) && args.iter().any(|a| a == "--help" || a == "-h") {
        println!("PAD - Panel for Agent Development");
        println!();
        println!("Usage: pad [OPTIONS]");
        println!("       pad telegram-bot");
        println!();
        println!("Options:");
        println!("  -h, --help     Show help");
        println!("  -V, --version  Show version");
        println!("  -d, --debug    Enable debug logging (~/.pad/logs/pad.log)");
        println!("      --tmux-doctor  Probe tmux compatibility and print capability details");
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
        println!("  d              Delete pane + hide thread");
        println!("  F1             Settings");
        println!("  q              Quit");
        return Ok(());
    }

    if is_info_only_command(&args) && args.iter().any(|a| a == "--version" || a == "-V") {
        println!("pad {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if is_info_only_command(&args) && args.iter().any(|a| a == "--tmux-doctor") {
        let report = system_check::tmux_doctor()?;
        for line in report.summary_lines() {
            println!("{line}");
        }
        let required = report.missing_required_capabilities();
        if !required.is_empty() {
            println!("required missing: {}", required.join(", "));
        }
        let optional = report.missing_optional_capabilities();
        if !optional.is_empty() {
            println!("optional missing: {}", optional.join(", "));
        }
        return Ok(());
    }

    let telegram_daemon = is_telegram_daemon_command(&args);
    let debug = args.iter().any(|a| a == "--debug" || a == "-d");
    let tmux_env_present = std::env::var_os("TMUX").is_some();
    let tmux_pane_present = std::env::var_os("TMUX_PANE").is_some();
    let already_bootstrapped = std::env::var_os(PAD_BOOTSTRAP_ENV).is_some();
    if should_bootstrap_into_tmux(
        &args,
        tmux_env_present,
        tmux_pane_present,
        already_bootstrapped,
        io::stdin().is_terminal(),
        io::stdout().is_terminal(),
    ) {
        let _ = system_check::ensure_tmux_available()?;
        return bootstrap_into_tmux(&args);
    }
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
    paths::log_runtime_layout_status();

    if telegram_daemon {
        return telegram::run_daemon()
            .await
            .map_err(|err| -> Box<dyn Error> { err });
    }

    let _status_guard = runtime_status::StatusGuard::new(crate::paths::pad_status_path(), "pad")?;
    if hook::hook_socket_is_active() {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "pad hook socket already active at {}",
                crate::paths::hook_socket_path().display()
            ),
        )));
    }

    let tmux_report = system_check::ensure_tmux_available()?;
    for line in tmux_report.summary_lines() {
        log_debug!("tmux_probe: {}", line);
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
    if tmux_report.capabilities.focus_events {
        let _ = std::process::Command::new("tmux")
            .args(["set", "-g", "focus-events", "on"])
            .output();
    } else {
        log_debug!("tmux_probe: skip focus-events enable because capability is unavailable");
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    relay::apply_runtime_configs(
        &app.config.agents,
        &app.config.agent_permissions,
        &app.config.codex,
    );
    if let Err(err) = telegram::ensure_embedded_daemon_running() {
        log_debug!(
            "telegram: embedded daemon start failed during pad startup: {}",
            err
        );
    }
    app.hook_rx = Some(hook::start_hook_listener()?);
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
    app.seed_startup_thread_sort_activity_once();

    let res = {
        let run_app = event::run_app(&mut terminal, &mut app);
        tokio::pin!(run_app);

        tokio::select! {
            res = &mut run_app => res,
            signal = shutdown_signal() => {
                log_debug!(
                    "handoff trace=- stage=main.shutdown_signal signal={}",
                    signal
                );
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
        log_debug!(
            "handoff trace={} stage=main.exit result=error err={:?}",
            app.same_session_trace_id.as_deref().unwrap_or("-"),
            err
        );
        log_debug!("退出错误: {:?}", err);
        println!("{:?}", err);
    } else {
        log_debug!(
            "handoff trace={} stage=main.exit result=ok",
            app.same_session_trace_id.as_deref().unwrap_or("-")
        );
        log_debug!("pad 正常退出");
    }

    res?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn bootstrap_needed_for_interactive_plain_launch() {
        assert!(should_bootstrap_into_tmux(
            &args(&["pad", "--debug"]),
            false,
            false,
            false,
            true,
            true,
        ));
    }

    #[test]
    fn bootstrap_skips_info_and_daemon_commands() {
        assert!(!should_bootstrap_into_tmux(
            &args(&["pad", "--help"]),
            false,
            false,
            false,
            true,
            true,
        ));
        assert!(!should_bootstrap_into_tmux(
            &args(&["pad", "telegram-bot"]),
            false,
            false,
            false,
            true,
            true,
        ));
    }

    #[test]
    fn bootstrap_skips_when_already_inside_tmux_or_reentered() {
        assert!(!should_bootstrap_into_tmux(
            &args(&["pad"]),
            true,
            true,
            false,
            true,
            true,
        ));
        assert!(!should_bootstrap_into_tmux(
            &args(&["pad"]),
            false,
            false,
            true,
            true,
            true,
        ));
    }

    #[test]
    fn bootstrap_skips_without_interactive_terminal() {
        assert!(!should_bootstrap_into_tmux(
            &args(&["pad"]),
            false,
            false,
            false,
            false,
            true,
        ));
    }

    #[test]
    fn bootstrap_command_quotes_executable_and_args() {
        let command = bootstrap_command(
            &args(&["pad", "--debug", "work tree"]),
            std::path::Path::new("/tmp/pad bin"),
        );
        assert_eq!(
            command,
            "env PAD_TMUX_BOOTSTRAPPED=1 '/tmp/pad bin' '--debug' 'work tree'"
        );
    }
}
