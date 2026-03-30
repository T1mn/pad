mod api;
mod help;
mod locale;
mod state;

#[cfg(test)]
use self::api::chunk_text;
use self::api::{
    answer_callback_query, edit_message, fetch_me, get_updates, send_chat_action, send_message,
    send_message_draft, send_text, set_my_commands, telegram_chat_id_value, TelegramCallbackQuery,
    TelegramUpdate,
};
#[cfg(test)]
use self::help::{build_help_keyboard, help_page_html};
use self::help::{help_message_payload, HelpPage};
use self::locale::{locale_prefers_chinese, telegram_locale, tg, tg_fmt, tg_fmt2, tg_fmt3};
use self::state::{
    journal_len, load_state, mark_update_processed, now_ms_i64, now_ts, save_state,
    AgentSnapshotEntry, PendingRequest, SelectedTarget, TelegramState,
};
use crate::chat::approval::{scan_codex_approval_updates, transcript_len, CodexApprovalRequest};
use crate::chat::backend::{
    build_slash_command_text, compact_target_label, invalidate_live_panels, latest_answer_for_pane,
    leaf_name, live_panels, pad_is_online, summarize_pane_capture,
};
use crate::hook::HookEvent;
use crate::log_debug;
use crate::model::{AgentPanel, AgentState, AgentType};
use crate::runtime_status;
use crate::theme::Config;
use crate::tmux_dispatch;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::sleep;

const PENDING_TIMEOUT_SECS: i64 = 2 * 60 * 60;
const JOURNAL_RECOVERY_RETRY_SECS: i64 = 3;
const JOURNAL_RECOVERY_STALL_SECS: i64 = 5;
const SLASH_POLL_INTERVAL_MS: u64 = 90;
static RECENT_HOOK_SIGNATURES: LazyLock<Mutex<Vec<String>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
static DRAFT_FEEDBACK_GATES: LazyLock<Mutex<HashMap<i64, Arc<DraftFeedbackGate>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

struct DraftFeedbackGate {
    latest_seq: AtomicU64,
    send_lock: AsyncMutex<()>,
}

pub async fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    let _status_guard =
        runtime_status::StatusGuard::new(crate::paths::telegram_bot_status_path(), "telegram-bot")?;
    log_debug!("telegram: daemon starting");

    let mut state = load_state().unwrap_or_default();
    if state.journal_position == 0 && state.pending.is_none() {
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
            log_debug!("telegram: disabled in config, exiting");
            return Ok(());
        }
        if config.telegram.bot_token.trim().is_empty() {
            log_debug!("telegram: bot_token empty, exiting");
            return Ok(());
        }

        if config.telegram.bot_token != last_token
            || config.telegram.bot_username.is_empty()
            || config.language != last_language
        {
            match fetch_me(&config.telegram.bot_token).await {
                Ok(me) => {
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
                Err(err) => {
                    log_debug!("telegram: getMe failed: {}", err);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            }
        }

        if let Err(err) = process_pending_timeout(&config, &mut state).await {
            log_debug!("telegram: pending timeout handling failed: {}", err);
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

        match get_updates(&config.telegram.bot_token, state.update_offset).await {
            Ok(updates) => {
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
            Err(err) => {
                log_debug!("telegram: getUpdates failed: {}", err);
                sleep(Duration::from_secs(2)).await;
            }
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
    if !config.telegram.enabled || config.telegram.bot_token.trim().is_empty() {
        return stop_daemon();
    }
    ensure_daemon_running(config)
}

pub fn restart_daemon(config: &Config) -> io::Result<bool> {
    let _ = stop_daemon()?;
    ensure_daemon_running(config)
}

pub fn daemon_is_running() -> bool {
    runtime_status::read_status(&crate::paths::telegram_bot_status_path())
        .map(|status| runtime_status::process_alive(status.pid))
        .unwrap_or(false)
        || daemon_socket_is_active()
}

fn daemon_socket_is_active() -> bool {
    let path = crate::paths::telegram_hook_socket_path();
    path.exists() && StdUnixStream::connect(path).is_ok()
}

pub fn stop_daemon() -> io::Result<bool> {
    let status_path = crate::paths::telegram_bot_status_path();
    let socket_path = crate::paths::telegram_hook_socket_path();
    let mut stopped = false;

    if let Some(status) = runtime_status::read_status(&status_path) {
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

fn start_direct_hook_listener() -> io::Result<()> {
    let socket_path = crate::paths::telegram_hook_socket_path();
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        if daemon_socket_is_active() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "telegram daemon socket already active at {}",
                    socket_path.display()
                ),
            ));
        }
        match std::fs::remove_file(&socket_path) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }

    tokio::spawn(async move {
        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(err) => {
                log_debug!("telegram: direct hook bind failed: {}", err);
                return;
            }
        };
        log_debug!(
            "telegram: direct hook listener on {}",
            socket_path.display()
        );

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    tokio::spawn(async move {
                        if let Err(err) = handle_direct_hook_stream(stream).await {
                            log_debug!("telegram: direct hook stream error: {}", err);
                        }
                    });
                }
                Err(err) => {
                    log_debug!("telegram: direct hook accept error: {}", err);
                    break;
                }
            }
        }
    });
    Ok(())
}

async fn handle_direct_hook_stream(
    stream: UnixStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let reader = TokioBufReader::new(stream);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let event: HookEvent = serde_json::from_str(&line)?;
        process_direct_hook_event(&event).await?;
    }

    Ok(())
}

async fn process_direct_hook_event(
    event: &HookEvent,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load();
    let locale = telegram_locale(&config);
    if !config.telegram.enabled || config.telegram.bot_token.trim().is_empty() {
        return Ok(());
    }

    let Some(pane_id) = event.tmux.pane_id.as_deref() else {
        return Ok(());
    };

    let mut state = load_state().unwrap_or_default();
    let Some(pending_snapshot) = state.pending.as_ref().cloned() else {
        return Ok(());
    };
    if pending_snapshot.pane_id != pane_id {
        return Ok(());
    }
    if !remember_processed_hook_event(&mut state, event) {
        log_debug!(
            "telegram: skipped duplicate direct hook event={} pane={}",
            event.event,
            pane_id
        );
        return Ok(());
    }

    match event.event.as_str() {
        "user_prompt_submit" => {
            let matches_prompt = event
                .prompt
                .as_deref()
                .map(|prompt| {
                    format!("{:x}", md5::compute(prompt.as_bytes())) == pending_snapshot.prompt_hash
                })
                .unwrap_or(true);
            if matches_prompt {
                if let Some(pending) = state.pending.as_mut() {
                    pending.phase = "awaiting_stop".to_string();
                    pending.accepted_at = Some(now_ts());
                    if event.transcript_path.is_some() {
                        pending.transcript_path = event.transcript_path.clone();
                    }
                }
                refresh_pending_feedback(&config, &mut state, true);
                save_state(&state)?;
                log_debug!(
                    "telegram: direct hook advanced request {} to awaiting_stop dispatch_to_submit_ms={}",
                    pending_snapshot.request_id,
                    now_ms_i64().saturating_sub(pending_sent_ms(&pending_snapshot))
                );
            }
        }
        "stop" => {
            let answer = event
                .last_assistant_message
                .clone()
                .filter(|text| !text.trim().is_empty())
                .or_else(|| latest_answer_for_pane(&pending_snapshot.pane_id));
            let request_id = pending_snapshot.request_id.clone();
            let chat_id = pending_snapshot.chat_id.clone();
            finalize_pending_feedback(&config, &pending_snapshot, tg(locale, "phase.completed"));
            state.pending = None;
            save_state(&state)?;
            let result_text = answer.unwrap_or_else(|| tg(locale, "result.missing").to_string());
            send_text(
                &config.telegram.bot_token,
                &chat_id,
                &tg_fmt2(locale, "result.completed", &request_id, result_text),
            )
            .await
            .map_err(|err| io::Error::other(err.to_string()))?;
            log_debug!(
                "telegram: direct hook completed request {} total_ms={} run_ms={}",
                request_id,
                now_ms_i64().saturating_sub(pending_sent_ms(&pending_snapshot)),
                now_ms_i64().saturating_sub(pending_accepted_ms(&pending_snapshot))
            );
        }
        _ => {}
    }

    Ok(())
}

async fn handle_update(
    config: &mut Config,
    state: &mut TelegramState,
    update: TelegramUpdate,
) -> Result<(), Box<dyn std::error::Error>> {
    let locale = telegram_locale(config);
    if let Some(callback_query) = update.callback_query {
        return handle_callback_query(config, state, callback_query).await;
    }

    let Some(message) = update.message else {
        return Ok(());
    };

    if message.chat.kind != "private" {
        log_debug!("telegram: ignoring non-private chat {}", message.chat.id);
        return Ok(());
    }

    let chat_id = message.chat.id.to_string();
    let text = message.text.unwrap_or_default();
    log_debug!(
        "telegram: incoming message chat={} msg_id={} text={}",
        chat_id,
        message.message_id,
        truncate_for_log(&text, 200)
    );

    if config.telegram.chat_id.is_empty() {
        if text.starts_with("/start") {
            config.telegram.chat_id = chat_id.clone();
            config.save();
            send_text(
                &config.telegram.bot_token,
                &chat_id,
                tg(locale, "bind.success"),
            )
            .await?;
        } else {
            send_text(
                &config.telegram.bot_token,
                &chat_id,
                tg(locale, "bind.start_required"),
            )
            .await?;
        }
        return Ok(());
    }

    if config.telegram.chat_id != chat_id {
        send_text(
            &config.telegram.bot_token,
            &chat_id,
            tg(locale, "bind.other_chat"),
        )
        .await?;
        return Ok(());
    }

    if text.starts_with('/') {
        handle_command(config, state, &chat_id, &text).await?;
    } else {
        handle_plain_text(config, state, &chat_id, &text).await?;
    }

    Ok(())
}

async fn handle_command(
    config: &Config,
    state: &mut TelegramState,
    chat_id: &str,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let locale = telegram_locale(config);
    let mut parts = text.trim().splitn(2, ' ');
    let command = parts.next().unwrap_or_default();
    let arg = parts.next().unwrap_or_default().trim();

    match command {
        "/start" => {
            send_text(
                &config.telegram.bot_token,
                chat_id,
                tg(locale, "start.ready"),
            )
            .await?;
        }
        "/help" => {
            send_help_message(config, state, chat_id, HelpPage::Overview).await?;
        }
        "/list" | "/agents" => send_agent_list(config, state, chat_id).await?,
        "/use" => {
            let idx = arg.parse::<usize>().ok().and_then(|n| n.checked_sub(1));
            let Some(idx) = idx else {
                send_text(&config.telegram.bot_token, chat_id, tg(locale, "use.usage")).await?;
                return Ok(());
            };
            let Some(entry) = state.agent_snapshot.get(idx).cloned() else {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    tg(locale, "use.invalid"),
                )
                .await?;
                return Ok(());
            };
            let panels = live_panels()?;
            if !panels.iter().any(|panel| panel.pane_id == entry.pane_id) {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    tg(locale, "pane.stale"),
                )
                .await?;
                return Ok(());
            }
            state.selected_target = Some(SelectedTarget {
                pane_id: entry.pane_id.clone(),
                label: entry.label.clone(),
            });
            send_text(
                &config.telegram.bot_token,
                chat_id,
                &tg_fmt(locale, "target.switched", entry.label),
            )
            .await?;
        }
        "/padstatus" => {
            send_pad_status_report(config, state, chat_id).await?;
        }
        "/status" => {
            if matches!(arg, "pad" | "bot") {
                send_pad_status_report(config, state, chat_id).await?;
            } else {
                dispatch_codex_slash_command(config, state, chat_id, "/status", arg, 1000).await?;
            }
        }
        "/fast" => {
            dispatch_codex_slash_command(config, state, chat_id, "/fast", arg, 1200).await?;
        }
        "/compact" => {
            dispatch_codex_slash_command(config, state, chat_id, "/compact", arg, 2000).await?;
        }
        "/stop" => {
            let Some(target) = state.selected_target.as_ref() else {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    tg(locale, "target.none"),
                )
                .await?;
                return Ok(());
            };
            match tmux_dispatch::send_escape(&target.pane_id) {
                Ok(()) => {
                    send_text(
                        &config.telegram.bot_token,
                        chat_id,
                        &tg_fmt(locale, "stop.sent", &target.label),
                    )
                    .await?;
                }
                Err(err) => {
                    send_text(
                        &config.telegram.bot_token,
                        chat_id,
                        &tg_fmt(locale, "stop.failed", err),
                    )
                    .await?;
                }
            }
        }
        _ => {
            send_text(
                &config.telegram.bot_token,
                chat_id,
                tg(locale, "unknown.command"),
            )
            .await?;
        }
    }

    Ok(())
}

async fn send_pad_status_report(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let locale = telegram_locale(config);
    let pad_status = runtime_status::describe_status(&crate::paths::pad_status_path());
    let target = state
        .selected_target
        .as_ref()
        .map(|target| target.label.clone())
        .unwrap_or_else(|| tg(locale, "status.none").to_string());
    let pending = state
        .pending
        .as_ref()
        .map(|pending| {
            format!(
                "{} ({})",
                pending.request_id,
                phase_label(locale, &pending.phase)
            )
        })
        .unwrap_or_else(|| tg(locale, "status.pending_none").to_string());
    let body = format!(
        "{}: {}\n{}: {}\n{}: {}",
        tg(locale, "status.pad"),
        pad_status,
        tg(locale, "status.target"),
        target,
        tg(locale, "status.pending"),
        pending
    );
    send_text(&config.telegram.bot_token, chat_id, &body).await?;
    Ok(())
}

async fn dispatch_codex_slash_command(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
    command: &str,
    arg: &str,
    deadline_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let locale = telegram_locale(config);
    if !pad_is_online() {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "pad.offline"),
        )
        .await?;
        return Ok(());
    }

    let Some(target) = state.selected_target.as_ref() else {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "target.none"),
        )
        .await?;
        return Ok(());
    };

    let panels = live_panels()?;
    let Some(panel) = panels.iter().find(|panel| panel.pane_id == target.pane_id) else {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "pane.stale"),
        )
        .await?;
        return Ok(());
    };

    if !matches!(&panel.agent_type, &AgentType::Codex) {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "target.not_codex"),
        )
        .await?;
        return Ok(());
    }

    if panel.state == AgentState::Busy {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "agent.busy"),
        )
        .await?;
        return Ok(());
    }
    if panel.state == AgentState::Waiting {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "agent.waiting"),
        )
        .await?;
        return Ok(());
    }

    let slash = build_slash_command_text(command, arg);
    let baseline = tmux_dispatch::capture_pane_tail(&panel.pane_id, 28)
        .map(|capture| summarize_pane_capture(&capture))
        .unwrap_or_default();
    tmux_dispatch::dispatch_prompt(&panel.pane_id, &slash)?;
    invalidate_live_panels();
    log_debug!(
        "telegram: dispatched codex slash command pane={} command={}",
        panel.pane_id,
        slash
    );

    let reply = match poll_slash_reply(&panel.pane_id, &slash, &baseline, deadline_ms).await {
        Ok(Some(capture)) => {
            if capture.is_empty() {
                tg_fmt2(locale, "slash.sent", &slash, compact_target_label(panel))
            } else {
                tg_fmt3(
                    locale,
                    "slash.output",
                    &slash,
                    compact_target_label(panel),
                    capture,
                )
            }
        }
        Ok(None) => tg_fmt2(locale, "slash.sent", &slash, compact_target_label(panel)),
        Err(err) => {
            log_debug!(
                "telegram: capture after slash command failed pane={} command={} err={}",
                panel.pane_id,
                slash,
                err
            );
            tg_fmt2(locale, "slash.sent", &slash, compact_target_label(panel))
        }
    };
    send_text(&config.telegram.bot_token, chat_id, &reply).await?;
    Ok(())
}

async fn handle_plain_text(
    config: &Config,
    state: &mut TelegramState,
    chat_id: &str,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let locale = telegram_locale(config);
    if text.trim().is_empty() {
        return Ok(());
    }
    if state.pending.is_some() {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "pending.exists"),
        )
        .await?;
        return Ok(());
    }

    if !pad_is_online() {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "pad.offline"),
        )
        .await?;
        return Ok(());
    }

    let Some(target) = state.selected_target.clone() else {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "target.none"),
        )
        .await?;
        return Ok(());
    };

    let panels = live_panels()?;
    let Some(panel) = panels.iter().find(|panel| panel.pane_id == target.pane_id) else {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "pane.stale"),
        )
        .await?;
        return Ok(());
    };

    if panel.state == AgentState::Busy {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "agent.busy"),
        )
        .await?;
        return Ok(());
    }
    if panel.state == AgentState::Waiting {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "agent.waiting"),
        )
        .await?;
        return Ok(());
    }

    tmux_dispatch::dispatch_prompt(&panel.pane_id, text)?;
    invalidate_live_panels();
    let request_id = format!("tg-{}", now_ts());
    let transcript_path = panel.transcript_path.clone();
    let approval_scan_offset = transcript_path.as_deref().map(transcript_len).unwrap_or(0);
    let sent_at = now_ts();
    let sent_at_ms = now_ms_i64();
    state.pending = Some(PendingRequest {
        request_id: request_id.clone(),
        chat_id: chat_id.to_string(),
        pane_id: panel.pane_id.clone(),
        agent_kind: panel.agent_type.to_string(),
        target_label: compact_target_label(panel),
        prompt_text: text.to_string(),
        prompt_hash: format!("{:x}", md5::compute(text.as_bytes())),
        sent_at,
        sent_at_ms,
        accepted_at: None,
        accepted_at_ms: None,
        last_status_at: None,
        draft_id: now_ms_i64(),
        phase: "awaiting_submit".to_string(),
        transcript_path,
        approval_scan_offset,
        approval_call_id: None,
        approval_justification: None,
    });
    save_state(state)?;
    log_debug!(
        "telegram: prompt dispatched request_id={} pane={} chat={}",
        request_id,
        panel.pane_id,
        chat_id
    );
    refresh_pending_feedback(config, state, true);
    Ok(())
}

async fn handle_callback_query(
    config: &Config,
    state: &mut TelegramState,
    query: TelegramCallbackQuery,
) -> Result<(), Box<dyn std::error::Error>> {
    let locale = telegram_locale(config);
    let Some(message) = query.message else {
        answer_callback_query(
            &config.telegram.bot_token,
            &query.id,
            Some(tg(locale, "callback.invalid")),
        )
        .await?;
        return Ok(());
    };
    if message.chat.kind != "private" {
        answer_callback_query(
            &config.telegram.bot_token,
            &query.id,
            Some(tg(locale, "callback.private_only")),
        )
        .await?;
        return Ok(());
    }

    let chat_id = message.chat.id.to_string();
    if !config.telegram.chat_id.is_empty() && config.telegram.chat_id != chat_id {
        answer_callback_query(
            &config.telegram.bot_token,
            &query.id,
            Some(tg(locale, "callback.bound_other")),
        )
        .await?;
        return Ok(());
    }

    let Some(data) = query.data.as_deref() else {
        answer_callback_query(
            &config.telegram.bot_token,
            &query.id,
            Some(tg(locale, "callback.no_data")),
        )
        .await?;
        return Ok(());
    };

    if let Some(page) = HelpPage::from_callback(data) {
        edit_help_message(config, state, &chat_id, message.message_id, page).await?;
        answer_callback_query(&config.telegram.bot_token, &query.id, None).await?;
    } else if data == "help:list" {
        send_agent_list(config, state, &chat_id).await?;
        answer_callback_query(&config.telegram.bot_token, &query.id, None).await?;
    } else if data == "help:padstatus" {
        send_pad_status_report(config, state, &chat_id).await?;
        answer_callback_query(&config.telegram.bot_token, &query.id, None).await?;
    } else if let Some(pane_id) = data.strip_prefix("use-pane:") {
        let panels = live_panels()?;
        if let Some(panel) = panels.iter().find(|panel| panel.pane_id == pane_id) {
            let selected = SelectedTarget {
                pane_id: panel.pane_id.clone(),
                label: format_agent_line_for_button(panel, locale),
            };
            state.selected_target = Some(selected.clone());
            answer_callback_query(
                &config.telegram.bot_token,
                &query.id,
                Some(tg(locale, "callback.switched")),
            )
            .await?;
            send_text(
                &config.telegram.bot_token,
                &chat_id,
                &tg_fmt(locale, "target.switched", selected.label),
            )
            .await?;
        } else {
            answer_callback_query(
                &config.telegram.bot_token,
                &query.id,
                Some(tg(locale, "callback.stale")),
            )
            .await?;
        }
    } else if let Some(choice) = data.strip_prefix("approval:") {
        let Some(pending_snapshot) = state.pending.as_ref().cloned() else {
            answer_callback_query(
                &config.telegram.bot_token,
                &query.id,
                Some(tg(locale, "approval.none")),
            )
            .await?;
            return Ok(());
        };
        if pending_snapshot.agent_kind != "codex" || pending_snapshot.approval_call_id.is_none() {
            answer_callback_query(
                &config.telegram.bot_token,
                &query.id,
                Some(tg(locale, "approval.none")),
            )
            .await?;
            return Ok(());
        }
        let key = match choice {
            "y" => "y",
            "a" => "a",
            "n" => "n",
            _ => {
                answer_callback_query(
                    &config.telegram.bot_token,
                    &query.id,
                    Some(tg(locale, "callback.unknown")),
                )
                .await?;
                return Ok(());
            }
        };
        match tmux_dispatch::send_approval_key(&pending_snapshot.pane_id, key) {
            Ok(()) => {
                invalidate_live_panels();
                if let Some(pending) = state.pending.as_mut() {
                    pending.phase = "awaiting_stop".to_string();
                    pending.approval_call_id = None;
                    pending.approval_justification = None;
                    pending.last_status_at = None;
                }
                refresh_pending_feedback(config, state, true);
                answer_callback_query(
                    &config.telegram.bot_token,
                    &query.id,
                    Some(approval_sent_text(locale, choice)),
                )
                .await?;
            }
            Err(err) => {
                answer_callback_query(
                    &config.telegram.bot_token,
                    &query.id,
                    Some(&tg_fmt(locale, "approval.failed", err)),
                )
                .await?;
            }
        }
    } else {
        answer_callback_query(
            &config.telegram.bot_token,
            &query.id,
            Some(tg(locale, "callback.unknown")),
        )
        .await?;
    }

    Ok(())
}

async fn process_pending_timeout(
    config: &Config,
    state: &mut TelegramState,
) -> Result<(), Box<dyn std::error::Error>> {
    let locale = telegram_locale(config);
    let Some(pending) = state.pending.as_ref() else {
        return Ok(());
    };
    if now_ts() - pending.sent_at < PENDING_TIMEOUT_SECS {
        return Ok(());
    }
    let pending_snapshot = pending.clone();
    let chat_id = pending.chat_id.clone();
    let request_id = pending.request_id.clone();
    finalize_pending_feedback(config, &pending_snapshot, tg(locale, "phase.completed"));
    state.pending = None;
    send_text(
        &config.telegram.bot_token,
        &chat_id,
        &tg_fmt(locale, "timeout", request_id),
    )
    .await?;
    Ok(())
}

async fn process_hook_journal(
    config: &Config,
    state: &mut TelegramState,
) -> Result<(), Box<dyn std::error::Error>> {
    sync_state_from_disk(state);
    let Some(_) = state.pending.clone() else {
        state.journal_position = journal_len();
        return Ok(());
    };

    let path = crate::paths::hook_events_path();
    if !path.exists() {
        return Ok(());
    }

    let file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    if state.journal_position > len {
        state.journal_position = len;
    }
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(state.journal_position))?;

    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        state.journal_position += line.len() as u64;
        sync_state_from_disk(state);
        let Some(current_pending) = state.pending.clone() else {
            line.clear();
            break;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }
        match serde_json::from_str::<HookEvent>(trimmed) {
            Ok(event) => {
                if !remember_processed_hook_event(state, &event) {
                    line.clear();
                    continue;
                }
                if event.tmux.pane_id.as_deref() == Some(current_pending.pane_id.as_str())
                    && apply_hook_event_to_pending(config, state, &event).await?
                {
                    line.clear();
                    break;
                }
            }
            Err(err) => {
                log_debug!("telegram: invalid hook journal line: {}", err);
            }
        }
        line.clear();
    }

    Ok(())
}

fn sync_state_from_disk(state: &mut TelegramState) {
    if let Ok(mut latest) = load_state() {
        latest.journal_position = latest.journal_position.max(state.journal_position);
        latest.last_journal_recovery_at = latest
            .last_journal_recovery_at
            .max(state.last_journal_recovery_at);
        *state = latest;
    }
}

fn should_probe_hook_journal(state: &TelegramState) -> bool {
    should_probe_hook_journal_inner(state, daemon_socket_is_active(), now_ts())
}

fn should_probe_hook_journal_inner(
    state: &TelegramState,
    direct_hook_active: bool,
    now: i64,
) -> bool {
    let Some(pending) = state.pending.as_ref() else {
        return false;
    };
    if state.last_journal_recovery_at == 0 {
        return true;
    }
    if !direct_hook_active {
        return now.saturating_sub(state.last_journal_recovery_at) >= 1;
    }
    if now.saturating_sub(state.last_journal_recovery_at) < JOURNAL_RECOVERY_RETRY_SECS {
        return false;
    }
    match pending.phase.as_str() {
        "awaiting_submit" => now.saturating_sub(pending.sent_at) >= JOURNAL_RECOVERY_STALL_SECS,
        "awaiting_stop" | "awaiting_confirm" => {
            now.saturating_sub(pending.accepted_at.unwrap_or(pending.sent_at))
                >= JOURNAL_RECOVERY_STALL_SECS
        }
        _ => false,
    }
}

fn remember_processed_hook_event(state: &mut TelegramState, event: &HookEvent) -> bool {
    let signature = hook_event_signature(event);
    if recent_hook_signature_exists(&signature) {
        return false;
    }
    if state
        .processed_hook_signatures
        .iter()
        .any(|existing| existing == &signature)
    {
        return false;
    }
    state.processed_hook_signatures.push(signature);
    const MAX_PROCESSED_HOOKS: usize = 64;
    if state.processed_hook_signatures.len() > MAX_PROCESSED_HOOKS {
        let drop_count = state.processed_hook_signatures.len() - MAX_PROCESSED_HOOKS;
        state.processed_hook_signatures.drain(0..drop_count);
    }
    remember_recent_hook_signature(
        state
            .processed_hook_signatures
            .last()
            .expect("processed hook signature must exist"),
    );
    true
}

fn hook_event_signature(event: &HookEvent) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        event.event,
        event.tmux.pane_id.as_deref().unwrap_or(""),
        event.timestamp.as_deref().unwrap_or(""),
        event.session_id.as_deref().unwrap_or(""),
        event
            .prompt
            .as_deref()
            .map(|prompt| format!("{:x}", md5::compute(prompt.as_bytes())))
            .unwrap_or_default()
    )
}

fn recent_hook_signature_exists(signature: &str) -> bool {
    RECENT_HOOK_SIGNATURES
        .lock()
        .map(|signatures| signatures.iter().any(|existing| existing == signature))
        .unwrap_or(false)
}

fn remember_recent_hook_signature(signature: &str) {
    if let Ok(mut signatures) = RECENT_HOOK_SIGNATURES.lock() {
        signatures.push(signature.to_string());
        const MAX_RECENT_HOOK_SIGNATURES: usize = 128;
        if signatures.len() > MAX_RECENT_HOOK_SIGNATURES {
            let drop_count = signatures.len() - MAX_RECENT_HOOK_SIGNATURES;
            signatures.drain(0..drop_count);
        }
    }
}

async fn apply_hook_event_to_pending(
    config: &Config,
    state: &mut TelegramState,
    event: &HookEvent,
) -> Result<bool, Box<dyn std::error::Error>> {
    let locale = telegram_locale(config);
    let Some(pending_snapshot) = state.pending.as_ref().cloned() else {
        return Ok(true);
    };
    match event.event.as_str() {
        "user_prompt_submit" => {
            let matches_prompt = event
                .prompt
                .as_deref()
                .map(|prompt| {
                    format!("{:x}", md5::compute(prompt.as_bytes())) == pending_snapshot.prompt_hash
                })
                .unwrap_or(true);
            if matches_prompt {
                if let Some(pending) = state.pending.as_mut() {
                    pending.phase = "awaiting_stop".to_string();
                    pending.accepted_at = Some(now_ts());
                    pending.accepted_at_ms = Some(now_ms_i64());
                    if event.transcript_path.is_some() {
                        pending.transcript_path = event.transcript_path.clone();
                    }
                }
                refresh_pending_feedback(config, state, true);
                log_debug!(
                    "telegram: pending request {} reached awaiting_stop dispatch_to_submit_ms={}",
                    pending_snapshot.request_id,
                    now_ms_i64().saturating_sub(pending_sent_ms(&pending_snapshot))
                );
            }
            Ok(false)
        }
        "stop" => {
            let answer = event
                .last_assistant_message
                .clone()
                .filter(|text| !text.trim().is_empty())
                .or_else(|| latest_answer_for_pane(&pending_snapshot.pane_id));
            let request_id = pending_snapshot.request_id.clone();
            let chat_id = pending_snapshot.chat_id.clone();
            finalize_pending_feedback(config, &pending_snapshot, tg(locale, "phase.completed"));
            state.pending = None;
            let result_text = answer.unwrap_or_else(|| tg(locale, "result.missing").to_string());
            send_text(
                &config.telegram.bot_token,
                &chat_id,
                &tg_fmt2(locale, "result.completed", &request_id, result_text),
            )
            .await?;
            log_debug!(
                "telegram: completed request {} total_ms={} run_ms={}",
                request_id,
                now_ms_i64().saturating_sub(pending_sent_ms(&pending_snapshot)),
                now_ms_i64().saturating_sub(pending_accepted_ms(&pending_snapshot))
            );
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn format_agent_line(idx: usize, panel: &AgentPanel, locale: crate::i18n::Locale) -> String {
    let state = agent_state_label(&panel.state, locale);
    format!(
        "{}. [{}] {} ({})",
        idx,
        panel.agent_type,
        leaf_name(&panel.working_dir),
        state
    )
}

fn format_agent_line_for_button(panel: &AgentPanel, locale: crate::i18n::Locale) -> String {
    format!(
        "[{}] {} ({})",
        panel.agent_type,
        leaf_name(&panel.working_dir),
        agent_state_label(&panel.state, locale)
    )
}

fn build_agent_keyboard(
    panels: &[AgentPanel],
    locale: crate::i18n::Locale,
) -> Vec<Vec<serde_json::Value>> {
    panels
        .iter()
        .map(|panel| {
            vec![json!({
                "text": button_label(panel, locale),
                "callback_data": format!("use-pane:{}", panel.pane_id),
            })]
        })
        .collect()
}

fn button_label(panel: &AgentPanel, locale: crate::i18n::Locale) -> String {
    let leaf = leaf_name(&panel.working_dir);
    let leaf = truncate_chars(&leaf, 24);
    format!(
        "{} | {} | {}",
        panel.agent_type,
        leaf,
        agent_state_label(&panel.state, locale)
    )
}

fn agent_state_label(state: &AgentState, locale: crate::i18n::Locale) -> &'static str {
    match state {
        AgentState::Idle if locale_prefers_chinese(locale) => "空闲",
        AgentState::Idle => "idle",
        AgentState::Busy if locale_prefers_chinese(locale) => "忙碌",
        AgentState::Busy => "busy",
        AgentState::Waiting if locale_prefers_chinese(locale) => "等待",
        AgentState::Waiting => "waiting",
    }
}

fn phase_label(locale: crate::i18n::Locale, phase: &str) -> String {
    match phase {
        "awaiting_submit" => tg(locale, "phase.awaiting_submit").to_string(),
        "awaiting_confirm" => tg(locale, "phase.awaiting_confirm").to_string(),
        "awaiting_stop" => tg(locale, "phase.accepted").to_string(),
        _ => phase.to_string(),
    }
}

fn pending_status_text(locale: crate::i18n::Locale, pending: &PendingRequest, now: i64) -> String {
    if pending.approval_call_id.is_some() {
        let mut lines = vec![
            tg(locale, "phase.awaiting_confirm").to_string(),
            pending.target_label.clone(),
        ];
        if let Some(justification) = pending.approval_justification.as_deref() {
            lines.push(truncate_chars(justification, 220));
        }
        return lines.join("\n");
    }

    let headline = match pending.phase.as_str() {
        "awaiting_submit" => tg(locale, "phase.awaiting_submit").to_string(),
        "awaiting_stop" => match pending.accepted_at {
            Some(accepted_at) if now.saturating_sub(accepted_at) >= 4 => {
                tg_fmt(locale, "phase.working", now.saturating_sub(accepted_at))
            }
            _ => tg(locale, "phase.accepted").to_string(),
        },
        _ => tg(locale, "phase.completed").to_string(),
    };
    format!("{}\n{}", headline, pending.target_label)
}

fn refresh_pending_feedback(config: &Config, state: &mut TelegramState, force: bool) {
    let locale = telegram_locale(config);
    let now = now_ts();
    let Some(pending) = state.pending.as_mut() else {
        return;
    };

    if !force {
        let Some(accepted_at) = pending.accepted_at else {
            return;
        };
        if accepted_at <= 0 {
            return;
        }
        if let Some(last_status_at) = pending.last_status_at {
            if now.saturating_sub(last_status_at) < 4 {
                return;
            }
        }
    }

    spawn_pending_feedback_update(
        config.telegram.bot_token.clone(),
        pending.chat_id.clone(),
        pending.draft_id,
        pending_status_text(locale, pending, now),
        true,
        tg(locale, "typing.action").to_string(),
    );
    pending.last_status_at = Some(now);
}

fn finalize_pending_feedback(config: &Config, pending: &PendingRequest, status: &str) {
    spawn_pending_feedback_update(
        config.telegram.bot_token.clone(),
        pending.chat_id.clone(),
        pending.draft_id,
        format!("{}\n{}", status, pending.target_label),
        false,
        String::new(),
    );
    let draft_id = pending.draft_id;
    tokio::spawn(async move {
        sleep(Duration::from_secs(5)).await;
        clear_draft_feedback_gate(draft_id);
    });
}

async fn process_codex_pending_approval(
    config: &Config,
    state: &mut TelegramState,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(snapshot) = state.pending.as_ref().cloned() else {
        return Ok(());
    };
    if snapshot.agent_kind != "codex" {
        return Ok(());
    }
    if snapshot.accepted_at.is_none() && snapshot.phase != "awaiting_confirm" {
        return Ok(());
    }

    let transcript_path = match snapshot.transcript_path.clone() {
        Some(path) => path,
        None => {
            let Some(path) = live_panels()?
                .into_iter()
                .find(|panel| panel.pane_id == snapshot.pane_id)
                .and_then(|panel| panel.transcript_path)
            else {
                return Ok(());
            };
            if let Some(pending) = state.pending.as_mut() {
                pending.transcript_path = Some(path.clone());
                if pending.approval_scan_offset == 0 {
                    pending.approval_scan_offset = transcript_len(&path).saturating_sub(32 * 1024);
                }
            }
            path
        }
    };

    let previous_call_id = snapshot.approval_call_id.clone();
    let current_request = snapshot
        .approval_call_id
        .clone()
        .zip(snapshot.approval_justification.clone())
        .map(|(call_id, justification)| CodexApprovalRequest {
            call_id,
            justification,
        });
    let scan_result = scan_codex_approval_updates(
        Path::new(&transcript_path),
        snapshot.approval_scan_offset,
        current_request,
    )?;

    let next_request = scan_result.active_request.clone();
    let changed = previous_call_id.as_deref()
        != next_request
            .as_ref()
            .map(|request| request.call_id.as_str());

    if let Some(pending) = state.pending.as_mut() {
        pending.approval_scan_offset = scan_result.next_offset;
        match next_request.as_ref() {
            Some(request) => {
                pending.phase = "awaiting_confirm".to_string();
                pending.approval_call_id = Some(request.call_id.clone());
                pending.approval_justification = Some(request.justification.clone());
                pending.last_status_at = None;
            }
            None => {
                pending.approval_call_id = None;
                pending.approval_justification = None;
                if pending.phase == "awaiting_confirm" {
                    pending.phase = "awaiting_stop".to_string();
                }
                pending.last_status_at = None;
            }
        }
    }

    if !changed {
        return Ok(());
    }

    refresh_pending_feedback(config, state, true);
    if let Some(request) = next_request {
        let pending = state.pending.as_ref().expect("pending must exist");
        send_codex_approval_prompt(config, &pending.chat_id, pending, &request).await?;
        log_debug!(
            "telegram: codex approval detected request={} pane={} call_id={}",
            pending.request_id,
            pending.pane_id,
            request.call_id
        );
    } else if let Some(previous_call_id) = previous_call_id {
        log_debug!(
            "telegram: codex approval cleared pane={} call_id={}",
            snapshot.pane_id,
            previous_call_id
        );
    }

    Ok(())
}

async fn send_codex_approval_prompt(
    config: &Config,
    chat_id: &str,
    pending: &PendingRequest,
    request: &CodexApprovalRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let locale = telegram_locale(config);
    let body = format!(
        "{}\n{}: {}\n\n{}",
        tg(locale, "approval.prompt"),
        tg(locale, "approval.target"),
        pending.target_label,
        request.justification
    );
    send_message(
        &config.telegram.bot_token,
        &json!({
            "chat_id": telegram_chat_id_value(chat_id),
            "text": body,
            "reply_markup": {
                "inline_keyboard": [[
                    {
                        "text": tg(locale, "approval.button.once"),
                        "callback_data": "approval:y"
                    },
                    {
                        "text": tg(locale, "approval.button.always"),
                        "callback_data": "approval:a"
                    }
                ], [
                    {
                        "text": tg(locale, "approval.button.reject"),
                        "callback_data": "approval:n"
                    }
                ]]
            }
        }),
    )
    .await?;
    Ok(())
}

fn approval_sent_text(locale: crate::i18n::Locale, choice: &str) -> &'static str {
    match choice {
        "y" => tg(locale, "approval.sent.once"),
        "a" => tg(locale, "approval.sent.always"),
        "n" => tg(locale, "approval.sent.reject"),
        _ => tg(locale, "callback.unknown"),
    }
}

async fn send_help_message(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
    page: HelpPage,
) -> Result<(), Box<dyn std::error::Error>> {
    let locale = telegram_locale(config);
    let _: serde_json::Value = send_message(
        &config.telegram.bot_token,
        &help_message_payload(locale, state, telegram_chat_id_value(chat_id), None, page),
    )
    .await?;
    Ok(())
}

async fn edit_help_message(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
    message_id: i64,
    page: HelpPage,
) -> Result<(), Box<dyn std::error::Error>> {
    let locale = telegram_locale(config);
    let _: serde_json::Value = edit_message(
        &config.telegram.bot_token,
        &help_message_payload(
            locale,
            state,
            telegram_chat_id_value(chat_id),
            Some(message_id),
            page,
        ),
    )
    .await?;
    Ok(())
}

async fn send_agent_list(
    config: &Config,
    state: &mut TelegramState,
    chat_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let locale = telegram_locale(config);
    let panels = live_panels()?;
    let snapshot = panels
        .iter()
        .enumerate()
        .map(|(idx, panel)| AgentSnapshotEntry {
            pane_id: panel.pane_id.clone(),
            label: format_agent_line(idx + 1, panel, locale),
        })
        .collect::<Vec<_>>();
    state.agent_snapshot = snapshot.clone();

    if snapshot.is_empty() {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "list.empty"),
        )
        .await?;
        return Ok(());
    }

    let body = snapshot
        .iter()
        .map(|entry| entry.label.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let keyboard = build_agent_keyboard(&panels, locale);
    send_message(
        &config.telegram.bot_token,
        &json!({
            "chat_id": chat_id,
            "text": body,
            "reply_markup": {
                "inline_keyboard": keyboard
            }
        }),
    )
    .await?;
    Ok(())
}

fn truncate_for_log(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let truncated = text.chars().take(max_chars).collect::<String>();
    format!("{}...", truncated)
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let shortened = text
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    format!("{}…", shortened)
}

fn pending_sent_ms(pending: &PendingRequest) -> i64 {
    if pending.sent_at_ms > 0 {
        pending.sent_at_ms
    } else {
        pending.sent_at.saturating_mul(1000)
    }
}

fn pending_accepted_ms(pending: &PendingRequest) -> i64 {
    pending.accepted_at_ms.unwrap_or_else(|| {
        pending
            .accepted_at
            .unwrap_or(pending.sent_at)
            .saturating_mul(1000)
    })
}

fn spawn_pending_feedback_update(
    token: String,
    chat_id: String,
    draft_id: i64,
    text: String,
    send_typing: bool,
    typing_action: String,
) {
    let gate = draft_feedback_gate(draft_id);
    let seq = gate.latest_seq.fetch_add(1, Ordering::SeqCst) + 1;
    tokio::spawn(async move {
        let _guard = gate.send_lock.lock().await;
        if gate.latest_seq.load(Ordering::SeqCst) != seq {
            return;
        }
        if send_typing {
            let _ = send_chat_action(&token, &chat_id, &typing_action).await;
        }
        let _ = send_message_draft(&token, &chat_id, draft_id, &text).await;
    });
}

fn draft_feedback_gate(draft_id: i64) -> Arc<DraftFeedbackGate> {
    let mut gates = DRAFT_FEEDBACK_GATES
        .lock()
        .expect("draft feedback gates lock");
    gates
        .entry(draft_id)
        .or_insert_with(|| {
            Arc::new(DraftFeedbackGate {
                latest_seq: AtomicU64::new(0),
                send_lock: AsyncMutex::new(()),
            })
        })
        .clone()
}

fn clear_draft_feedback_gate(draft_id: i64) {
    if let Ok(mut gates) = DRAFT_FEEDBACK_GATES.lock() {
        gates.remove(&draft_id);
    }
}

fn capture_looks_like_echo_only(capture: &str, slash: &str) -> bool {
    let trimmed = capture.trim();
    trimmed == slash.trim() || trimmed.ends_with(&format!("\n{}", slash.trim()))
}

async fn poll_slash_reply(
    pane_id: &str,
    slash: &str,
    baseline: &str,
    deadline_ms: u64,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let started = Instant::now();
    let deadline = Duration::from_millis(deadline_ms);
    let mut candidate: Option<String> = None;
    let mut stable_hits = 0usize;

    loop {
        let capture = tmux_dispatch::capture_pane_tail(pane_id, 28)?;
        let capture = summarize_pane_capture(&capture);
        if !capture.is_empty() && capture != baseline {
            if !capture_looks_like_echo_only(&capture, slash) {
                if candidate.as_deref() == Some(capture.as_str()) {
                    stable_hits += 1;
                } else {
                    candidate = Some(capture.clone());
                    stable_hits = 1;
                }
                if stable_hits >= 2 || started.elapsed() >= Duration::from_millis(250) {
                    return Ok(Some(capture));
                }
            } else {
                candidate = Some(capture);
            }
        }

        if started.elapsed() >= deadline {
            break;
        }
        sleep(Duration::from_millis(SLASH_POLL_INTERVAL_MS)).await;
    }

    Ok(candidate.filter(|capture| capture != baseline))
}

#[cfg(test)]
mod tests {
    use super::{
        build_agent_keyboard, build_help_keyboard, build_slash_command_text, chunk_text,
        help_page_html, mark_update_processed, pending_status_text, remember_processed_hook_event,
        scan_codex_approval_updates, should_probe_hook_journal_inner, summarize_pane_capture,
        CodexApprovalRequest, HelpPage, PendingRequest, SelectedTarget, TelegramState,
    };
    use crate::hook::{HookEvent, HookTmuxInfo};
    use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};
    use std::fs;

    #[test]
    fn chunk_text_splits_long_messages() {
        let chunks = chunk_text("abcdef", 3);
        assert_eq!(chunks, vec!["abc", "def"]);
    }

    #[test]
    fn slash_command_builder_preserves_optional_args() {
        assert_eq!(build_slash_command_text("/status", ""), "/status");
        assert_eq!(build_slash_command_text("/fast", "status"), "/fast status");
    }

    #[test]
    fn summarize_pane_capture_trims_blank_edges_and_keeps_tail() {
        let capture = "\n\none\n\ntwo\nthree\n\n";
        assert_eq!(summarize_pane_capture(capture), "one\n\ntwo\nthree");
    }

    #[test]
    fn agent_keyboard_uses_clickable_use_callbacks() {
        let panel = AgentPanel {
            session: "0".into(),
            window: "zsh".into(),
            window_index: "1".into(),
            pane: "1".into(),
            pane_id: "%42".into(),
            agent_type: AgentType::Codex,
            working_dir: "/tmp/rust-tui".into(),
            is_active: false,
            state: AgentState::Idle,
            state_source: AgentStateSource::Scanner,
            transcript_path: None,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            git_info: None,
            pid: None,
            start_time: None,
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        };
        let keyboard = build_agent_keyboard(&[panel], crate::i18n::Locale::En);
        assert_eq!(keyboard.len(), 1);
        assert_eq!(keyboard[0][0]["callback_data"], "use-pane:%42");
    }

    #[test]
    fn pending_status_moves_from_accepted_to_working() {
        let pending = PendingRequest {
            request_id: "tg-1".into(),
            chat_id: "1".into(),
            pane_id: "%1".into(),
            agent_kind: "codex".into(),
            target_label: "CODEX • rust-tui".into(),
            prompt_text: "hi".into(),
            prompt_hash: "abc".into(),
            sent_at: 100,
            sent_at_ms: 100_000,
            accepted_at: Some(100),
            accepted_at_ms: Some(100_000),
            last_status_at: None,
            draft_id: 123,
            phase: "awaiting_stop".into(),
            transcript_path: None,
            approval_scan_offset: 0,
            approval_call_id: None,
            approval_justification: None,
        };

        let accepted = pending_status_text(crate::i18n::Locale::En, &pending, 102);
        let working = pending_status_text(crate::i18n::Locale::En, &pending, 106);

        assert!(accepted.contains("Submitted"));
        assert!(working.contains("Working"));
        assert!(working.contains("6s"));
    }

    #[test]
    fn pending_status_reports_approval_needed() {
        let pending = PendingRequest {
            request_id: "tg-1".into(),
            chat_id: "1".into(),
            pane_id: "%1".into(),
            agent_kind: "codex".into(),
            target_label: "CODEX • rust-tui".into(),
            prompt_text: "hi".into(),
            prompt_hash: "abc".into(),
            sent_at: 100,
            sent_at_ms: 100_000,
            accepted_at: Some(100),
            accepted_at_ms: Some(100_000),
            last_status_at: None,
            draft_id: 123,
            phase: "awaiting_confirm".into(),
            transcript_path: None,
            approval_scan_offset: 0,
            approval_call_id: Some("call_1".into()),
            approval_justification: Some("Do you want to allow running cargo check?".into()),
        };

        let text = pending_status_text(crate::i18n::Locale::En, &pending, 110);
        assert!(text.contains("Needs approval"));
        assert!(text.contains("cargo check"));
    }

    #[test]
    fn processed_updates_are_deduplicated() {
        let mut state = TelegramState::default();
        assert!(mark_update_processed(&mut state, 10));
        assert_eq!(state.last_processed_update_id, 10);
        assert_eq!(state.update_offset, 11);

        assert!(!mark_update_processed(&mut state, 10));
        assert_eq!(state.last_processed_update_id, 10);
        assert_eq!(state.update_offset, 11);

        assert!(!mark_update_processed(&mut state, 9));
        assert_eq!(state.last_processed_update_id, 10);
        assert_eq!(state.update_offset, 11);

        assert!(mark_update_processed(&mut state, 12));
        assert_eq!(state.last_processed_update_id, 12);
        assert_eq!(state.update_offset, 13);
    }

    #[test]
    fn telegram_state_backfills_missing_last_processed_update_id() {
        let state: TelegramState = serde_json::from_str(
            r#"{
                "update_offset": 42,
                "journal_position": 7,
                "agent_snapshot": [],
                "pending": null
            }"#,
        )
        .unwrap();

        assert_eq!(state.update_offset, 42);
        assert_eq!(state.last_processed_update_id, 0);
        assert_eq!(state.journal_position, 7);
    }

    #[test]
    fn codex_approval_scan_tracks_open_and_resolved_requests() {
        let path =
            std::env::temp_dir().join(format!("pad-codex-approval-{}.jsonl", std::process::id()));
        let body = concat!(
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"call_id\":\"call_old\",\"arguments\":\"{\\\"sandbox_permissions\\\":\\\"require_escalated\\\",\\\"justification\\\":\\\"old\\\"}\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"call_id\":\"call_old\",\"output\":\"ok\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"call_id\":\"call_new\",\"arguments\":\"{\\\"sandbox_permissions\\\":\\\"require_escalated\\\",\\\"justification\\\":\\\"new justification\\\"}\"}}\n"
        );
        fs::write(&path, body).unwrap();

        let result = scan_codex_approval_updates(&path, 0, None).unwrap();
        assert_eq!(
            result.active_request,
            Some(CodexApprovalRequest {
                call_id: "call_new".into(),
                justification: "new justification".into(),
            })
        );

        fs::write(
            &path,
            format!(
                "{}{}",
                body,
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"call_id\":\"call_new\",\"output\":\"done\"}}\n"
            ),
        )
        .unwrap();
        let result =
            scan_codex_approval_updates(&path, result.next_offset, result.active_request).unwrap();
        assert!(result.active_request.is_none());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn help_page_callbacks_parse() {
        assert_eq!(
            HelpPage::from_callback("help:overview"),
            Some(HelpPage::Overview)
        );
        assert_eq!(HelpPage::from_callback("help:codex"), Some(HelpPage::Codex));
        assert_eq!(
            HelpPage::from_callback("help:workflow"),
            Some(HelpPage::Workflow)
        );
        assert_eq!(HelpPage::from_callback("help:list"), None);
    }

    #[test]
    fn help_page_html_includes_target_and_commands() {
        let state = TelegramState {
            selected_target: Some(SelectedTarget {
                pane_id: "%7".into(),
                label: "X rust-tui".into(),
            }),
            ..TelegramState::default()
        };
        let html = help_page_html(crate::i18n::Locale::En, &state, HelpPage::Codex);
        assert!(html.contains("Pad Telegram"));
        assert!(html.contains("X rust-tui"));
        assert!(html.contains("/status"));
        assert!(html.contains("/compact"));
    }

    #[test]
    fn help_keyboard_marks_active_page() {
        let keyboard = build_help_keyboard(crate::i18n::Locale::En, HelpPage::Workflow);
        assert_eq!(keyboard.len(), 2);
        assert_eq!(keyboard[0][2]["callback_data"], "help:workflow");
        assert_eq!(keyboard[1][0]["callback_data"], "help:list");
    }

    #[test]
    fn processed_hook_events_are_deduplicated_across_channels() {
        let event = HookEvent {
            event: "stop".into(),
            session_id: Some("$1".into()),
            transcript_path: None,
            cwd: None,
            prompt: Some("hello".into()),
            last_assistant_message: Some("done".into()),
            timestamp: Some("2026-03-26T03:38:06Z".into()),
            tmux: HookTmuxInfo {
                pane_id: Some("%14".into()),
                session_name: Some("0".into()),
                window_index: Some("2".into()),
                pane_index: Some("1".into()),
                pane_current_path: Some("/tmp".into()),
            },
        };

        let mut state = TelegramState::default();
        assert!(remember_processed_hook_event(&mut state, &event));
        assert!(!remember_processed_hook_event(&mut state, &event));
        assert_eq!(state.processed_hook_signatures.len(), 1);
    }

    #[test]
    fn journal_recovery_runs_immediately_for_pending_on_startup() {
        let state = TelegramState {
            pending: Some(PendingRequest {
                request_id: "tg-1".into(),
                chat_id: "1".into(),
                pane_id: "%1".into(),
                agent_kind: "codex".into(),
                target_label: "CODEX • rust-tui".into(),
                prompt_text: "hi".into(),
                prompt_hash: "abc".into(),
                sent_at: 100,
                sent_at_ms: 100_000,
                accepted_at: None,
                accepted_at_ms: None,
                last_status_at: None,
                draft_id: 123,
                phase: "awaiting_submit".into(),
                transcript_path: None,
                approval_scan_offset: 0,
                approval_call_id: None,
                approval_justification: None,
            }),
            ..TelegramState::default()
        };

        assert!(should_probe_hook_journal_inner(&state, true, 100));
    }

    #[test]
    fn journal_recovery_waits_for_stall_when_direct_hook_is_alive() {
        let state = TelegramState {
            last_journal_recovery_at: 100,
            pending: Some(PendingRequest {
                request_id: "tg-1".into(),
                chat_id: "1".into(),
                pane_id: "%1".into(),
                agent_kind: "codex".into(),
                target_label: "CODEX • rust-tui".into(),
                prompt_text: "hi".into(),
                prompt_hash: "abc".into(),
                sent_at: 101,
                sent_at_ms: 101_000,
                accepted_at: None,
                accepted_at_ms: None,
                last_status_at: None,
                draft_id: 123,
                phase: "awaiting_submit".into(),
                transcript_path: None,
                approval_scan_offset: 0,
                approval_call_id: None,
                approval_justification: None,
            }),
            ..TelegramState::default()
        };

        assert!(!should_probe_hook_journal_inner(&state, true, 103));
        assert!(should_probe_hook_journal_inner(&state, true, 106));
    }
}
