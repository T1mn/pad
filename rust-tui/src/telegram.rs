use crate::hook::HookEvent;
use crate::model::{AgentPanel, AgentState};
use crate::runtime_status;
use crate::scanner;
use crate::session_cache;
use crate::theme::Config;
use crate::tmux_dispatch;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::Command;
use tokio::time::sleep;

const TELEGRAM_TIMEOUT_SECS: u64 = 12;
const TELEGRAM_POLL_TIMEOUT_SECS: u64 = 4;
const TELEGRAM_MAX_TEXT_LEN: usize = 3500;
const PENDING_TIMEOUT_SECS: i64 = 2 * 60 * 60;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct TelegramState {
    update_offset: i64,
    journal_position: u64,
    selected_target: Option<SelectedTarget>,
    agent_snapshot: Vec<AgentSnapshotEntry>,
    pending: Option<PendingRequest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SelectedTarget {
    pane_id: String,
    label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AgentSnapshotEntry {
    pane_id: String,
    label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PendingRequest {
    request_id: String,
    chat_id: String,
    pane_id: String,
    #[serde(default)]
    agent_kind: String,
    target_label: String,
    prompt_text: String,
    prompt_hash: String,
    sent_at: i64,
    accepted_at: Option<i64>,
    last_status_at: Option<i64>,
    draft_id: i64,
    phase: String,
    #[serde(default)]
    transcript_path: Option<String>,
    #[serde(default)]
    approval_scan_offset: u64,
    #[serde(default)]
    approval_call_id: Option<String>,
    #[serde(default)]
    approval_justification: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CodexApprovalRequest {
    call_id: String,
    justification: String,
}

#[derive(Clone, Debug, Deserialize)]
struct TelegramEnvelope<T> {
    ok: bool,
    result: T,
    description: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    message: Option<TelegramMessage>,
    callback_query: Option<TelegramCallbackQuery>,
}

#[derive(Clone, Debug, Deserialize)]
struct TelegramMessage {
    message_id: i64,
    chat: TelegramChat,
    text: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct TelegramCallbackQuery {
    id: String,
    message: Option<TelegramMessage>,
    data: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct TelegramChat {
    id: i64,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Clone, Debug, Deserialize)]
struct TelegramMe {
    username: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct TelegramCommandSpec {
    command: &'static str,
    description: String,
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
    start_direct_hook_listener();

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
        if let Err(err) = process_hook_journal(&config, &mut state).await {
            log_debug!("telegram: hook journal processing failed: {}", err);
        }
        let _ = save_state(&state);
        if let Err(err) = process_codex_pending_approval(&config, &mut state).await {
            log_debug!("telegram: codex approval processing failed: {}", err);
        }
        let _ = save_state(&state);
        if let Err(err) = refresh_pending_feedback(&config, &mut state, false).await {
            log_debug!("telegram: pending feedback refresh failed: {}", err);
        }
        let _ = save_state(&state);

        match get_updates(&config.telegram.bot_token, state.update_offset).await {
            Ok(updates) => {
                for update in updates {
                    if let Ok(latest_state) = load_state() {
                        state = latest_state;
                    }
                    state.update_offset = update.update_id + 1;
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
        }
    }

    match fs::remove_file(&status_path) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    match fs::remove_file(&socket_path) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }

    Ok(stopped)
}

fn start_direct_hook_listener() {
    let socket_path = crate::paths::telegram_hook_socket_path();
    if let Some(parent) = socket_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::remove_file(&socket_path);

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
                refresh_pending_feedback(&config, &mut state, true)
                    .await
                    .map_err(|err| io::Error::other(err.to_string()))?;
                save_state(&state)?;
                log_debug!(
                    "telegram: direct hook advanced request {} to awaiting_stop",
                    pending_snapshot.request_id
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
            log_debug!("telegram: direct hook completed request {}", request_id);
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
            send_text(&config.telegram.bot_token, chat_id, tg(locale, "help.body")).await?;
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
        "/status" => {
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
    let request_id = format!("tg-{}", now_ts());
    let transcript_path = panel.transcript_path.clone();
    let approval_scan_offset = transcript_path.as_deref().map(transcript_len).unwrap_or(0);
    state.pending = Some(PendingRequest {
        request_id: request_id.clone(),
        chat_id: chat_id.to_string(),
        pane_id: panel.pane_id.clone(),
        agent_kind: panel.agent_type.to_string(),
        target_label: compact_target_label(panel),
        prompt_text: text.to_string(),
        prompt_hash: format!("{:x}", md5::compute(text.as_bytes())),
        sent_at: now_ts(),
        accepted_at: None,
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
    let _ = send_chat_action(
        &config.telegram.bot_token,
        chat_id,
        tg(locale, "typing.action"),
    )
    .await;
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

    if let Some(pane_id) = data.strip_prefix("use-pane:") {
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
                if let Some(pending) = state.pending.as_mut() {
                    pending.phase = "awaiting_stop".to_string();
                    pending.approval_call_id = None;
                    pending.approval_justification = None;
                    pending.last_status_at = None;
                }
                refresh_pending_feedback(config, state, true).await?;
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
    let chat_id = pending.chat_id.clone();
    let request_id = pending.request_id.clone();
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
    let Some(pending) = state.pending.clone() else {
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
        state.journal_position += line.as_bytes().len() as u64;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }
        match serde_json::from_str::<HookEvent>(trimmed) {
            Ok(event) => {
                if event.tmux.pane_id.as_deref() == Some(pending.pane_id.as_str()) {
                    if apply_hook_event_to_pending(config, state, &event).await? {
                        line.clear();
                        break;
                    }
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
                    if event.transcript_path.is_some() {
                        pending.transcript_path = event.transcript_path.clone();
                    }
                }
                refresh_pending_feedback(config, state, true).await?;
                log_debug!(
                    "telegram: pending request {} reached awaiting_stop",
                    pending_snapshot.request_id
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
            state.pending = None;
            let result_text = answer.unwrap_or_else(|| tg(locale, "result.missing").to_string());
            send_text(
                &config.telegram.bot_token,
                &chat_id,
                &tg_fmt2(locale, "result.completed", &request_id, result_text),
            )
            .await?;
            log_debug!("telegram: completed request {}", request_id);
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn latest_answer_for_pane(pane_id: &str) -> Option<String> {
    let mut panels = scanner::scan_panels().ok()?;
    let _ = session_cache::preload_panels(&mut panels);
    let panel = panels.into_iter().find(|panel| panel.pane_id == pane_id)?;
    panel
        .last_assistant_message
        .filter(|text| !text.trim().is_empty())
        .or_else(|| {
            panel
                .cached_preview_turns
                .first()
                .and_then(|turn| turn.answer.clone())
                .filter(|text| !text.trim().is_empty())
        })
}

fn live_panels() -> Result<Vec<AgentPanel>, Box<dyn std::error::Error>> {
    let mut panels = scanner::scan_panels().map_err(|err| io::Error::other(err.to_string()))?;
    let _ = session_cache::preload_panels(&mut panels);
    Ok(panels)
}

fn format_agent_line(idx: usize, panel: &AgentPanel, locale: crate::i18n::Locale) -> String {
    let state = agent_state_label(&panel.state, locale);
    format!(
        "{}. [{}] {} ({})",
        idx,
        panel.agent_type.to_string(),
        leaf_name(&panel.working_dir),
        state
    )
}

fn format_agent_line_for_button(panel: &AgentPanel, locale: crate::i18n::Locale) -> String {
    format!(
        "[{}] {} ({})",
        panel.agent_type.to_string(),
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
        panel.agent_type.to_string(),
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

fn leaf_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

fn pad_is_online() -> bool {
    runtime_status::read_status(&crate::paths::pad_status_path())
        .map(|status| runtime_status::process_alive(status.pid))
        .unwrap_or(false)
}

fn telegram_locale(config: &Config) -> crate::i18n::Locale {
    crate::i18n::Locale::from_str(&config.language)
}

fn locale_prefers_chinese(locale: crate::i18n::Locale) -> bool {
    matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
    )
}

fn tg<'a>(locale: crate::i18n::Locale, key: &'a str) -> &'a str {
    let zh = locale_prefers_chinese(locale);
    match key {
        "bind.success" if zh => {
            "Pad Telegram 已绑定。先用 /list 查看并点击目标，或用 /use <编号> 选择目标，然后直接发消息。"
        }
        "bind.success" => {
            "Pad Telegram is linked. Use /list to pick a target, or /use <number>, then send plain text."
        }
        "bind.start_required" if zh => "请先发送 /start 以绑定当前聊天。",
        "bind.start_required" => "Send /start first to link this chat.",
        "bind.other_chat" if zh => "这个 bot 已绑定到另一个 Telegram 聊天。",
        "bind.other_chat" => "This bot is already linked to another Telegram chat.",
        "start.ready" if zh => {
            "Pad Telegram 已就绪。用 /list 查看并点击目标，或用 /use <编号> 选择；/status 查看当前状态，/stop 尝试中断当前 agent。"
        }
        "start.ready" => {
            "Pad Telegram is ready. Use /list to pick a target, or /use <number>; /status shows current state and /stop tries to interrupt the agent."
        }
        "help.body" if zh => {
            "/start\n/help\n/list\n/agents\n/use <编号>\n/status\n/stop\n\n选择目标后，直接发送普通文本即可。"
        }
        "help.body" => {
            "/start\n/help\n/list\n/agents\n/use <number>\n/status\n/stop\n\nAfter selecting a target, just send plain text."
        }
        "use.usage" if zh => "用法：/use <编号>。先执行 /agents 获取编号。",
        "use.usage" => "Usage: /use <number>. Run /agents first to get a fresh list.",
        "use.invalid" if zh => "编号无效。请先执行 /agents 获取最新列表。",
        "use.invalid" => "Invalid number. Run /agents first to get the latest list.",
        "pane.stale" if zh => "目标 pane 已失效，请重新执行 /agents。",
        "pane.stale" => "The target pane is no longer available. Run /agents again.",
        "target.none" if zh => "还没有当前目标。请先 /agents 再 /use。",
        "target.none" => "No target selected yet. Use /agents and then /use first.",
        "unknown.command" if zh => "未知命令。发送 /help 查看可用命令。",
        "unknown.command" => "Unknown command. Send /help to see available commands.",
        "pending.exists" if zh => "当前已有待处理请求，请等待上一轮完成。",
        "pending.exists" => "A request is already in progress. Wait for it to finish first.",
        "pad.offline" if zh => "pad 当前未运行，无法派发 prompt。",
        "pad.offline" => "pad is not running, so the prompt can't be dispatched.",
        "agent.busy" if zh => "该 agent 当前正忙，请等待本轮结束后再发送。",
        "agent.busy" => "That agent is busy. Wait for the current turn to finish first.",
        "agent.waiting" if zh => "该 agent 当前正在等待确认，请先处理这条确认请求。",
        "agent.waiting" => "That agent is waiting for confirmation. Resolve it before sending a new prompt.",
        "callback.invalid" if zh => "无效回调",
        "callback.invalid" => "Invalid callback",
        "callback.private_only" if zh => "仅支持私聊",
        "callback.private_only" => "Private chats only",
        "callback.bound_other" if zh => "该 bot 已绑定到其他聊天",
        "callback.bound_other" => "This bot is already linked to another chat",
        "callback.no_data" if zh => "无回调数据",
        "callback.no_data" => "Missing callback data",
        "callback.switched" if zh => "已切换当前目标",
        "callback.switched" => "Target switched",
        "callback.stale" if zh => "目标 pane 已失效，请重新 /list",
        "callback.stale" => "The target pane is gone. Run /list again.",
        "callback.unknown" if zh => "未知操作",
        "callback.unknown" => "Unknown action",
        "approval.none" if zh => "当前没有待处理的 Codex 确认请求",
        "approval.none" => "There is no pending Codex approval request.",
        "approval.failed" if zh => "发送确认失败：{}",
        "approval.failed" => "Failed to send approval input: {}",
        "approval.prompt" if zh => "Codex 需要你确认一条提权命令",
        "approval.prompt" => "Codex needs approval for an escalated command",
        "approval.target" if zh => "目标",
        "approval.target" => "Target",
        "approval.button.once" if zh => "批准一次",
        "approval.button.once" => "Approve once",
        "approval.button.always" if zh => "本次会话总是允许",
        "approval.button.always" => "Always for session",
        "approval.button.reject" if zh => "拒绝",
        "approval.button.reject" => "Reject",
        "approval.sent.once" if zh => "已发送批准一次",
        "approval.sent.once" => "Approve once sent",
        "approval.sent.always" if zh => "已发送本次会话总是允许",
        "approval.sent.always" => "Always for session sent",
        "approval.sent.reject" if zh => "已发送拒绝",
        "approval.sent.reject" => "Reject sent",
        "status.none" if zh => "未选择",
        "status.none" => "none",
        "status.pending_none" if zh => "无",
        "status.pending_none" => "none",
        "status.pad" if zh => "Pad",
        "status.pad" => "Pad",
        "status.target" if zh => "目标",
        "status.target" => "Target",
        "status.pending" if zh => "请求",
        "status.pending" => "Pending",
        "stop.sent" if zh => "已向 {} 发送 Escape。",
        "stop.sent" => "Sent Escape to {}.",
        "stop.failed" if zh => "发送中断失败：{}",
        "stop.failed" => "Failed to send interrupt: {}",
        "target.switched" if zh => "当前目标已切换为：{}",
        "target.switched" => "Current target switched to: {}",
        "list.empty" if zh => "当前没有检测到可用的 agent pane。",
        "list.empty" => "No agent pane is currently available.",
        "timeout" if zh => "请求超时：{}。请回 pad 检查当前 pane 状态。",
        "timeout" => "Request timed out: {}. Check the pane in pad.",
        "result.missing" if zh => "任务结束，但未拿到回答正文，请回 pad 查看详细内容。",
        "result.missing" => "The task finished, but no answer text was captured. Check pad for details.",
        "result.completed" if zh => "完成：{}\n\n{}",
        "result.completed" => "Completed: {}\n\n{}",
        "phase.awaiting_submit" if zh => "等待提交",
        "phase.awaiting_submit" => "Waiting for submit",
        "phase.awaiting_confirm" if zh => "等待确认",
        "phase.awaiting_confirm" => "Needs approval",
        "phase.accepted" if zh => "已受理",
        "phase.accepted" => "Accepted",
        "phase.working" if zh => "进行中 · {}s",
        "phase.working" => "Working · {}s",
        "phase.completed" if zh => "已完成",
        "phase.completed" => "Completed",
        "typing.action" => "typing",
        "command.start" if zh => "绑定当前聊天并显示欢迎信息",
        "command.start" => "Link the current chat and show welcome text",
        "command.help" if zh => "查看可用命令",
        "command.help" => "Show available commands",
        "command.list" if zh => "列出可点击的 agent pane",
        "command.list" => "List clickable agent panes",
        "command.use" if zh => "按编号选择目标 agent",
        "command.use" => "Select the target agent by number",
        "command.status" if zh => "查看当前 pad 和目标状态",
        "command.status" => "Show the current pad and target status",
        "command.stop" if zh => "尝试中断当前 agent",
        "command.stop" => "Try to interrupt the current agent",
        _ => key,
    }
}

fn tg_fmt(locale: crate::i18n::Locale, key: &str, arg: impl std::fmt::Display) -> String {
    tg(locale, key).replacen("{}", &arg.to_string(), 1)
}

fn tg_fmt2(
    locale: crate::i18n::Locale,
    key: &str,
    arg1: impl std::fmt::Display,
    arg2: impl std::fmt::Display,
) -> String {
    tg(locale, key)
        .replacen("{}", &arg1.to_string(), 1)
        .replacen("{}", &arg2.to_string(), 1)
}

fn compact_target_label(panel: &AgentPanel) -> String {
    format!(
        "{} • {}",
        panel.agent_type.to_string().to_uppercase(),
        leaf_name(&panel.working_dir)
    )
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

async fn refresh_pending_feedback(
    config: &Config,
    state: &mut TelegramState,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let locale = telegram_locale(config);
    let now = now_ts();
    let Some(pending) = state.pending.as_mut() else {
        return Ok(());
    };

    if !force {
        let Some(accepted_at) = pending.accepted_at else {
            return Ok(());
        };
        if accepted_at <= 0 {
            return Ok(());
        }
        if let Some(last_status_at) = pending.last_status_at {
            if now.saturating_sub(last_status_at) < 4 {
                return Ok(());
            }
        }
    }

    send_chat_action(
        &config.telegram.bot_token,
        &pending.chat_id,
        tg(locale, "typing.action"),
    )
    .await?;
    send_message_draft(
        &config.telegram.bot_token,
        &pending.chat_id,
        pending.draft_id,
        &pending_status_text(locale, pending, now),
    )
    .await?;
    pending.last_status_at = Some(now);
    Ok(())
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

    refresh_pending_feedback(config, state, true).await?;
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

struct CodexApprovalScanResult {
    active_request: Option<CodexApprovalRequest>,
    next_offset: u64,
}

fn scan_codex_approval_updates(
    path: &Path,
    offset: u64,
    current_request: Option<CodexApprovalRequest>,
) -> io::Result<CodexApprovalScanResult> {
    if !path.exists() {
        return Ok(CodexApprovalScanResult {
            active_request: current_request,
            next_offset: offset,
        });
    }

    let file = File::open(path)?;
    let len = file.metadata()?.len();
    let start = offset.min(len);
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(start))?;

    let mut active_request = current_request;
    let mut next_offset = start;
    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        next_offset += line.as_bytes().len() as u64;
        match codex_approval_line_update(line.trim()) {
            CodexApprovalLineUpdate::Request(request) => active_request = Some(request),
            CodexApprovalLineUpdate::Resolved(call_id) => {
                if active_request
                    .as_ref()
                    .map(|request| request.call_id.as_str())
                    == Some(call_id.as_str())
                {
                    active_request = None;
                }
            }
            CodexApprovalLineUpdate::None => {}
        }
        line.clear();
    }

    Ok(CodexApprovalScanResult {
        active_request,
        next_offset,
    })
}

enum CodexApprovalLineUpdate {
    None,
    Request(CodexApprovalRequest),
    Resolved(String),
}

fn codex_approval_line_update(line: &str) -> CodexApprovalLineUpdate {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return CodexApprovalLineUpdate::None;
    };
    if value.get("type").and_then(Value::as_str) != Some("response_item") {
        return CodexApprovalLineUpdate::None;
    }
    let Some(payload) = value.get("payload") else {
        return CodexApprovalLineUpdate::None;
    };
    match payload.get("type").and_then(Value::as_str) {
        Some("function_call") => extract_codex_approval_request(payload)
            .map(CodexApprovalLineUpdate::Request)
            .unwrap_or(CodexApprovalLineUpdate::None),
        Some("function_call_output") => payload
            .get("call_id")
            .and_then(Value::as_str)
            .map(|call_id| CodexApprovalLineUpdate::Resolved(call_id.to_string()))
            .unwrap_or(CodexApprovalLineUpdate::None),
        _ => CodexApprovalLineUpdate::None,
    }
}

fn extract_codex_approval_request(payload: &Value) -> Option<CodexApprovalRequest> {
    let call_id = payload.get("call_id").and_then(Value::as_str)?.trim();
    if call_id.is_empty() {
        return None;
    }

    let args_value = match payload.get("arguments") {
        Some(Value::String(raw)) => serde_json::from_str::<Value>(raw).ok()?,
        Some(value) => value.clone(),
        None => return None,
    };

    if args_value
        .get("sandbox_permissions")
        .and_then(Value::as_str)
        != Some("require_escalated")
    {
        return None;
    }
    let justification = args_value
        .get("justification")
        .and_then(Value::as_str)?
        .trim();
    if justification.is_empty() {
        return None;
    }

    Some(CodexApprovalRequest {
        call_id: call_id.to_string(),
        justification: justification.to_string(),
    })
}

fn transcript_len(path: &str) -> u64 {
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

async fn fetch_me(token: &str) -> Result<TelegramMe, Box<dyn std::error::Error>> {
    let result = telegram_api::<TelegramMe>(token, "getMe", &json!({}), 15).await?;
    Ok(result)
}

async fn get_updates(
    token: &str,
    offset: i64,
) -> Result<Vec<TelegramUpdate>, Box<dyn std::error::Error>> {
    telegram_api::<Vec<TelegramUpdate>>(
        token,
        "getUpdates",
        &json!({
            "offset": offset,
            "timeout": TELEGRAM_POLL_TIMEOUT_SECS,
            "allowed_updates": ["message", "callback_query"]
        }),
        TELEGRAM_TIMEOUT_SECS,
    )
    .await
}

async fn send_text(
    token: &str,
    chat_id: &str,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let chunks = chunk_text(text, TELEGRAM_MAX_TEXT_LEN);
    let total = chunks.len();
    for (idx, chunk) in chunks.into_iter().enumerate() {
        let body = if total > 1 {
            format!("({}/{})\n{}", idx + 1, total, chunk)
        } else {
            chunk
        };
        let _: serde_json::Value = telegram_api(
            token,
            "sendMessage",
            &json!({
                "chat_id": chat_id,
                "text": body,
            }),
            20,
        )
        .await?;
    }
    Ok(())
}

async fn send_chat_action(
    token: &str,
    chat_id: &str,
    action: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let _: serde_json::Value = telegram_api(
        token,
        "sendChatAction",
        &json!({
            "chat_id": telegram_chat_id_value(chat_id),
            "action": action,
        }),
        10,
    )
    .await?;
    Ok(())
}

async fn send_message_draft(
    token: &str,
    chat_id: &str,
    draft_id: i64,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let _: serde_json::Value = telegram_api(
        token,
        "sendMessageDraft",
        &json!({
            "chat_id": telegram_chat_id_value(chat_id),
            "draft_id": draft_id,
            "text": text,
        }),
        10,
    )
    .await?;
    Ok(())
}

fn telegram_chat_id_value(chat_id: &str) -> serde_json::Value {
    chat_id
        .parse::<i64>()
        .map(serde_json::Value::from)
        .unwrap_or_else(|_| serde_json::Value::from(chat_id.to_string()))
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

async fn answer_callback_query(
    token: &str,
    callback_query_id: &str,
    text: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let payload = match text {
        Some(text) => json!({
            "callback_query_id": callback_query_id,
            "text": text,
        }),
        None => json!({
            "callback_query_id": callback_query_id,
        }),
    };
    let _: serde_json::Value = telegram_api(token, "answerCallbackQuery", &payload, 10).await?;
    Ok(())
}

async fn set_my_commands(
    token: &str,
    locale: crate::i18n::Locale,
) -> Result<(), Box<dyn std::error::Error>> {
    let commands = vec![
        TelegramCommandSpec {
            command: "start",
            description: tg(locale, "command.start").to_string(),
        },
        TelegramCommandSpec {
            command: "help",
            description: tg(locale, "command.help").to_string(),
        },
        TelegramCommandSpec {
            command: "list",
            description: tg(locale, "command.list").to_string(),
        },
        TelegramCommandSpec {
            command: "use",
            description: tg(locale, "command.use").to_string(),
        },
        TelegramCommandSpec {
            command: "status",
            description: tg(locale, "command.status").to_string(),
        },
        TelegramCommandSpec {
            command: "stop",
            description: tg(locale, "command.stop").to_string(),
        },
    ];
    let _: serde_json::Value = telegram_api(
        token,
        "setMyCommands",
        &json!({
            "commands": commands
        }),
        15,
    )
    .await?;
    Ok(())
}

async fn send_message(
    token: &str,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    telegram_api(token, "sendMessage", payload, 20).await
}

async fn telegram_api<T: for<'de> Deserialize<'de>>(
    token: &str,
    method: &str,
    payload: &serde_json::Value,
    timeout_secs: u64,
) -> Result<T, Box<dyn std::error::Error>> {
    let url = format!("https://api.telegram.org/bot{}/{}", token, method);
    let output = Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            &timeout_secs.to_string(),
            "-H",
            "Content-Type: application/json",
            "-X",
            "POST",
            &url,
            "-d",
            &payload.to_string(),
        ])
        .output()
        .await?;

    if !output.status.success() {
        return Err(io::Error::other(format!(
            "curl {} failed: {}",
            method,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
        .into());
    }

    let envelope: TelegramEnvelope<T> = serde_json::from_slice(&output.stdout)?;
    if !envelope.ok {
        return Err(io::Error::other(
            envelope
                .description
                .unwrap_or_else(|| format!("telegram api {} failed", method)),
        )
        .into());
    }
    Ok(envelope.result)
}

fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    if text.chars().count() <= max_chars {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut buf = String::new();
    let mut count = 0usize;
    for ch in text.chars() {
        if count >= max_chars {
            chunks.push(std::mem::take(&mut buf));
            count = 0;
        }
        buf.push(ch);
        count += 1;
    }
    if !buf.is_empty() {
        chunks.push(buf);
    }
    chunks
}

fn load_state() -> io::Result<TelegramState> {
    let path = crate::paths::telegram_state_path();
    match fs::read_to_string(path) {
        Ok(body) => serde_json::from_str(&body)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(TelegramState::default()),
        Err(err) => Err(err),
    }
}

fn save_state(state: &TelegramState) -> io::Result<()> {
    let body = serde_json::to_string_pretty(state)?;
    fs::write(crate::paths::telegram_state_path(), body)
}

fn journal_len() -> u64 {
    fs::metadata(crate::paths::hook_events_path())
        .map(|meta| meta.len())
        .unwrap_or(0)
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn now_ms_i64() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
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

#[cfg(test)]
mod tests {
    use super::{
        build_agent_keyboard, chunk_text, pending_status_text, scan_codex_approval_updates,
        CodexApprovalRequest, PendingRequest,
    };
    use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};
    use std::fs;

    #[test]
    fn chunk_text_splits_long_messages() {
        let chunks = chunk_text("abcdef", 3);
        assert_eq!(chunks, vec!["abc", "def"]);
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
            accepted_at: Some(100),
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

        assert!(accepted.contains("Accepted"));
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
            accepted_at: Some(100),
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
}
