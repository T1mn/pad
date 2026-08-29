#[cfg(test)]
mod compact_tests;
#[cfg(test)]
mod test_suites;

use std::error::Error;
use std::io;

mod app;
mod atomic_file;
mod browser_remote;
mod chat;
mod claude_history;
mod cli;
mod codex_provider_sync;
mod codex_rollout;
mod codex_runtime;
mod codex_state;
mod codex_turn_diff;
mod desktop_runtime;
mod event;
mod fuzzy;
mod gemini_history;
mod grok_history;
mod hook;
mod i18n;
#[macro_use]
mod logger;
mod model;
mod notification_inbox;
mod notify;
mod opencode_history;
mod opencode_text;
mod pad_store;
mod panic_boundary;
mod paths;
mod permission_policy;
#[cfg(test)]
pub(crate) mod permission_policy_tests;
mod pi_runtime;
mod preview_source;
mod relay;
mod runtime_status;
mod session_cache;
mod session_continuity;
mod shell_quote;
mod shutdown;
mod sidebar;
mod socket_api;
mod sound;
mod startup;
mod telegram;
mod terminal;
mod terminal_runtime;
mod terminal_workspace;
#[cfg(test)]
mod test_support;
mod text_match;
mod text_normalize;
mod theme;
mod thread_meta;
mod time;
mod title_summary;
mod tree;
mod ui;

use app::App;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    // The macOS Desktop shell owns this process over stdin/stdout JSONL.
    // Handle it before native PAD startup so no terminal or logger can write
    // to stdout and corrupt the bridge protocol.
    if matches!(args.get(1).map(String::as_str), Some("__internal"))
        && matches!(args.get(2).map(String::as_str), Some("desktop-server"))
    {
        return desktop_runtime::run_server();
    }

    if cli::is_internal_command(&args) {
        paths::ensure_runtime_layout()?;
        logger::init()?;
        return cli::run_internal_command(&args);
    }

    if cli::handle_info_command(&args)? {
        return Ok(());
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async_main(args))
}

async fn async_main(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let telegram_daemon = cli::is_telegram_daemon_command(&args);
    let debug = args.iter().any(|a| a == "--debug" || a == "-d");
    startup::prepare_runtime_environment(telegram_daemon, debug)?;

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

    log_debug!("runtime: PAD native terminal");

    terminal::install_panic_hook();
    let mut terminal = terminal::enter()?;

    let mut app = App::new();
    startup::start_runtime_services(&mut app)?;
    let size = ui::terminal_viewport_size(&mut app, terminal.size()?.into());
    let workspace = match terminal_workspace::load() {
        Ok(workspace) => workspace,
        Err(error) => match terminal_workspace::quarantine_invalid() {
            Ok(Some(recovery_path)) => {
                log_debug!(
                    "terminal_workspace: quarantined invalid workspace at {} after load error: {}",
                    recovery_path.display(),
                    error
                );
                None
            }
            Ok(None) => None,
            Err(quarantine_error) => {
                let _ = terminal::restore(&mut terminal);
                return Err(Box::new(io::Error::new(
                    quarantine_error.kind(),
                    format!(
                        "terminal workspace is invalid ({error}) and could not be preserved before recovery: {quarantine_error}"
                    ),
                )));
            }
        },
    };
    let start_result = match workspace {
        Some(workspace) => app.restore_native_terminal_workspace(workspace, size),
        None => app.start_native_terminal(size),
    };
    if let Err(error) = start_result {
        let _ = terminal::restore(&mut terminal);
        return Err(Box::new(error));
    }

    let res = {
        let run_app = event::run_app(&mut terminal, &mut app);
        tokio::pin!(run_app);

        tokio::select! {
            res = &mut run_app => res,
            signal = shutdown::shutdown_signal() => {
                log_debug!("native runtime shutdown signal={}", signal);
                log_debug!("收到终止信号，开始清理退出");
                Ok(())
            }
        }
    };

    let workspace = app.terminal_workspace_snapshot();
    if let Err(error) = terminal_workspace::save(&workspace) {
        log_debug!("terminal workspace save failed: {}", error);
    }
    if let Err(error) = app.shutdown_native_terminal() {
        log_debug!("native terminal shutdown failed: {}", error);
    }
    terminal::restore(&mut terminal)?;

    if let Err(ref err) = res {
        log_debug!("main.exit result=error err={:?}", err);
        log_debug!("退出错误: {:?}", err);
        println!("{:?}", err);
    } else {
        log_debug!("main.exit result=ok");
        log_debug!("pad 正常退出");
    }

    res?;
    Ok(())
}
