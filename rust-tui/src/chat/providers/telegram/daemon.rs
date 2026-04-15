use super::*;
use tokio::task::JoinHandle;

static EMBEDDED_DAEMON: LazyLock<Mutex<Option<JoinHandle<()>>>> =
    LazyLock::new(|| Mutex::new(None));

pub async fn run_daemon() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    run_daemon_loop(false).await
}

pub fn ensure_embedded_daemon_running() -> io::Result<bool> {
    stop_external_daemon_if_running()?;

    let mut handle_slot = EMBEDDED_DAEMON
        .lock()
        .map_err(|_| io::Error::other("telegram embedded daemon lock poisoned"))?;
    if let Some(handle) = handle_slot.as_ref() {
        if !handle.is_finished() {
            return Ok(false);
        }
    }

    let handle = tokio::spawn(async move {
        if let Err(err) = run_daemon_loop(true).await {
            log_debug!("telegram: embedded daemon exited with error: {}", err);
        }
    });
    *handle_slot = Some(handle);
    Ok(true)
}

async fn run_daemon_loop(embedded: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mode = if embedded {
        "telegram-bot-embedded"
    } else {
        "telegram-bot"
    };
    let _status_guard =
        runtime_status::StatusGuard::new(crate::paths::telegram_bot_status_path(), mode)?;
    log_debug!(
        "telegram: daemon starting mode={}",
        if embedded { "embedded" } else { "standalone" }
    );

    let mut state = load_state().unwrap_or_default();
    if state.journal_position == 0 && state.pending_requests.is_empty() {
        state.journal_position = journal_len();
        save_state(&state)?;
    }
    start_direct_hook_listener()?;

    let mut last_token = String::new();
    let mut last_language = String::new();

    loop {
        let mut config = Config::load();
        if let Ok(latest_state) = load_state() {
            state = latest_state;
        }
        if !config.telegram.enabled {
            if embedded {
                sleep(Duration::from_secs(1)).await;
                continue;
            }
            log_debug!("telegram: disabled in config, exiting");
            return Ok(());
        }
        if config.telegram.bot_token.trim().is_empty() {
            if embedded {
                sleep(Duration::from_secs(1)).await;
                continue;
            }
            log_debug!("telegram: bot_token empty, exiting");
            return Ok(());
        }

        if config.telegram.bot_token != last_token
            || config.telegram.bot_username.is_empty()
            || config.language != last_language
        {
            let auth_result = fetch_me(&config.telegram.bot_token).await;
            let me = match auth_result {
                Ok(me) => Some(me),
                Err(err) => {
                    let err_text = err.to_string();
                    log_debug!("telegram: getMe failed: {}", err_text);
                    None
                }
            };
            let Some(me) = me else {
                sleep(Duration::from_secs(5)).await;
                continue;
            };

            let username = me.username.unwrap_or_default();
            if config.telegram.bot_username != username {
                config.telegram.bot_username = username.clone();
                config.save();
            }
            if let Err(err) =
                set_my_commands(&config.telegram.bot_token, telegram_locale(&config)).await
            {
                log_debug!("telegram: setMyCommands failed: {}", err);
            }
            last_token = config.telegram.bot_token.clone();
            last_language = config.language.clone();
            log_debug!("telegram: authenticated as @{}", username);
        }

        if let Err(err) = process_pending_timeout(&config, &mut state).await {
            log_debug!("telegram: pending timeout handling failed: {}", err);
        }
        let _ = save_state(&state);
        if let Err(err) = process_pending_result_delivery(&config, &mut state).await {
            log_debug!("telegram: pending result delivery failed: {}", err);
        }
        let _ = save_state(&state);
        if should_probe_hook_journal(&state) {
            state.last_journal_recovery_at = now_ts();
            if let Err(err) = process_hook_journal(&config, &mut state).await {
                log_debug!("telegram: hook journal processing failed: {}", err);
            }
            let _ = save_state(&state);
        }
        if let Err(err) = process_codex_pending_approval(&config, &mut state).await {
            log_debug!("telegram: codex approval processing failed: {}", err);
        }
        let _ = save_state(&state);
        refresh_pending_feedback(&config, &mut state, false);
        let _ = save_state(&state);

        let updates_result = get_updates(&config.telegram.bot_token, state.update_offset).await;
        let updates = match updates_result {
            Ok(updates) => Some(updates),
            Err(err) => {
                let err_text = err.to_string();
                log_debug!("telegram: getUpdates failed: {}", err_text);
                None
            }
        };
        let Some(updates) = updates else {
            sleep(Duration::from_secs(2)).await;
            continue;
        };
        for update in updates {
            if let Ok(latest_state) = load_state() {
                state = latest_state;
            }
            if !mark_update_processed(&mut state, update.update_id) {
                log_debug!(
                    "telegram: skipping duplicate/stale update_id={} offset={}",
                    update.update_id,
                    state.update_offset
                );
                let _ = save_state(&state);
                continue;
            }
            let _ = save_state(&state);
            if let Err(err) = handle_update(&mut config, &mut state, update).await {
                log_debug!("telegram: update handling failed: {}", err);
            }
            let _ = save_state(&state);
        }
    }
}

pub fn ensure_daemon_running(config: &Config) -> io::Result<bool> {
    if !config.telegram.enabled || config.telegram.bot_token.trim().is_empty() {
        return Ok(false);
    }
    if daemon_is_running() {
        return Ok(false);
    }

    let exe = std::env::current_exe()?;
    let child = std::process::Command::new(exe)
        .arg("telegram-bot")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    log_debug!("telegram: auto-started daemon pid={}", child.id());
    Ok(true)
}

pub fn sync_daemon(config: &Config) -> io::Result<bool> {
    if crate::chat::backend::pad_is_online() {
        let _ = ensure_embedded_daemon_running()?;
        return Ok(false);
    }
    if !config.telegram.enabled || config.telegram.bot_token.trim().is_empty() {
        return stop_daemon();
    }
    ensure_daemon_running(config)
}

pub fn restart_daemon(config: &Config) -> io::Result<bool> {
    if crate::chat::backend::pad_is_online() {
        let _ = ensure_embedded_daemon_running()?;
        return Ok(false);
    }
    let _ = stop_daemon()?;
    ensure_daemon_running(config)
}

pub fn daemon_is_running() -> bool {
    runtime_status::read_status(&crate::paths::telegram_bot_status_path())
        .map(|status| runtime_status::process_alive(status.pid))
        .unwrap_or(false)
        || daemon_socket_is_active()
}

pub fn stop_daemon() -> io::Result<bool> {
    let status_path = crate::paths::telegram_bot_status_path();
    let socket_path = crate::paths::telegram_hook_socket_path();
    let mut stopped = false;

    if let Some(status) = runtime_status::read_status(&status_path) {
        if status.pid == std::process::id() {
            return Ok(false);
        }
        if runtime_status::process_alive(status.pid) {
            stopped = true;
            #[cfg(unix)]
            unsafe {
                libc::kill(status.pid as i32, libc::SIGTERM);
            }
            for _ in 0..20 {
                if !runtime_status::process_alive(status.pid) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            #[cfg(unix)]
            if runtime_status::process_alive(status.pid) {
                unsafe {
                    libc::kill(status.pid as i32, libc::SIGKILL);
                }
                for _ in 0..10 {
                    if !runtime_status::process_alive(status.pid) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
            if runtime_status::process_alive(status.pid) {
                return Err(io::Error::other(format!(
                    "telegram daemon pid {} did not exit",
                    status.pid
                )));
            }
        }
    }

    match fs::remove_file(&status_path) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    if socket_path.exists() {
        if daemon_socket_is_active() {
            return Err(io::Error::other(
                "telegram direct hook socket is still active",
            ));
        }
        match fs::remove_file(&socket_path) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }

    Ok(stopped)
}

fn stop_external_daemon_if_running() -> io::Result<bool> {
    let status_path = crate::paths::telegram_bot_status_path();
    match runtime_status::read_status(&status_path) {
        Some(status) if status.pid != std::process::id() => stop_daemon(),
        _ => Ok(false),
    }
}
