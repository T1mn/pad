use std::error::Error;
use std::io::{self, IsTerminal};

mod agent_resume;
mod app;
mod atomic_file;
mod bootstrap;
mod browser_remote;
mod chat;
mod claude_history;
mod cli;
mod codex_provider_sync;
mod codex_rollout;
mod codex_runtime;
mod codex_state;
mod codex_turn_diff;
mod detector;
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
mod pad_sider;
mod panic_boundary;
mod paths;
mod pipe;
mod preview_source;
pub mod pty;
mod relay;
mod runtime_mode;
mod runtime_status;
mod scanner;
mod session;
mod session_cache;
mod session_continuity;
mod shell_quote;
mod shutdown;
mod sidebar;
mod socket_api;
mod sound;
mod startup;
mod system_check;
mod telegram;
mod terminal;
mod terminal_runtime;
#[cfg(test)]
mod test_support;
mod text_match;
mod text_normalize;
mod theme;
mod thread_meta;
mod time;
mod title_summary;
mod tmux_bindings;
mod tmux_capabilities;
mod tmux_dispatch;
mod tree;
mod ui;
mod workspace_recipe;

use app::App;

fn should_restore_tmux_state(app: &App) -> bool {
    tmux_state_needs_restore(
        app.runtime_mode,
        app.same_session_attached,
        !app.saved_tmux_bindings.is_empty(),
        app.saved_tmux_status.is_some(),
    )
}

fn tmux_state_needs_restore(
    runtime_mode: runtime_mode::RuntimeMode,
    same_session_attached: bool,
    has_saved_bindings: bool,
    has_saved_status: bool,
) -> bool {
    runtime_mode.uses_tmux() && (same_session_attached || has_saved_bindings || has_saved_status)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

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
    let runtime_mode = runtime_mode::RuntimeMode::from_args(&args);
    let tmux_env_present = std::env::var_os("TMUX").is_some();
    let tmux_pane_present = std::env::var_os("TMUX_PANE").is_some();
    let already_bootstrapped = std::env::var_os(bootstrap::PAD_BOOTSTRAP_ENV).is_some();
    if runtime_mode.uses_tmux()
        && bootstrap::should_bootstrap_into_tmux(
            &args,
            tmux_env_present,
            tmux_pane_present,
            already_bootstrapped,
            io::stdin().is_terminal(),
            io::stdout().is_terminal(),
        )
    {
        let _ = system_check::ensure_tmux_available()?;
        return bootstrap::bootstrap_into_tmux(&args);
    }
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

    let configure_tmux_focus_events = if runtime_mode.uses_tmux() {
        let report = system_check::ensure_tmux_available()?;
        for line in report.summary_lines() {
            log_debug!("tmux_probe: {}", line);
        }
        report.capabilities.focus_events
    } else {
        log_debug!("runtime: native terminal mode; tmux probe skipped");
        false
    };

    terminal::install_panic_hook();
    let mut terminal = terminal::enter(configure_tmux_focus_events)?;

    let mut app = App::new();
    app.runtime_mode = runtime_mode;
    startup::start_runtime_services(&mut app)?;
    if runtime_mode.uses_tmux() {
        startup::load_initial_panels(&mut app);
    } else {
        let size = ui::terminal_viewport_size(&mut app, terminal.size()?.into());
        if let Err(error) = app.start_native_terminal(size) {
            let _ = terminal::restore(&mut terminal);
            return Err(Box::new(error));
        }
    }

    let res = {
        let run_app = event::run_app(&mut terminal, &mut app);
        tokio::pin!(run_app);

        tokio::select! {
            res = &mut run_app => res,
            signal = shutdown::shutdown_signal() => {
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

    if let Err(error) = app.shutdown_native_terminal() {
        log_debug!("native terminal shutdown failed: {}", error);
    }
    terminal::restore(&mut terminal)?;

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
mod native_runtime_tests {
    use super::{runtime_mode::RuntimeMode, tmux_state_needs_restore};

    #[test]
    fn native_mode_never_runs_tmux_restore_even_with_stale_state() {
        assert!(!tmux_state_needs_restore(
            RuntimeMode::Native,
            true,
            true,
            true
        ));
        assert!(tmux_state_needs_restore(
            RuntimeMode::TmuxCompatibility,
            false,
            true,
            false
        ));
    }
}
