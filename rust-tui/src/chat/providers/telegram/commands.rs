use super::*;

const PAD_DEFAULT_SESSION_NAME: &str = "pad";
const PAD_CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum PadRestartTarget {
    RespawnPane(String),
    NewDetachedSession(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PadRestartPlan {
    pub(super) target: PadRestartTarget,
    pub(super) start_dir: String,
    pub(super) shell_command: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SessionDiagContext {
    target_label: String,
    pane_id: Option<String>,
    request_id: Option<String>,
    session_id: Option<String>,
    transcript_path: Option<String>,
    continuity: Option<crate::session_continuity::ContinuitySnapshot>,
}

pub(super) async fn handle_update(
    config: &mut Config,
    state: &mut TelegramState,
    update: TelegramUpdate,
) -> TelegramResult<()> {
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

pub(super) async fn handle_command(
    config: &Config,
    state: &mut TelegramState,
    chat_id: &str,
    text: &str,
) -> TelegramResult<()> {
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
            let panels = live_panels().map_err(telegram_error)?;
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
        "/history" => {
            send_recent_history(config, state, chat_id).await?;
        }
        "/diag" => {
            send_session_diag(config, state, chat_id, arg).await?;
        }
        "/restart" => {
            let plan = match current_pad_restart_plan() {
                Ok(plan) => plan,
                Err(err_text) => {
                    send_text(
                        &config.telegram.bot_token,
                        chat_id,
                        &tg_fmt(locale, "restart.failed", err_text),
                    )
                    .await?;
                    play_sound_event(config, crate::sound::SoundEvent::Failure);
                    return Ok(());
                }
            };
            let preparing_key = match plan.target {
                PadRestartTarget::RespawnPane(_) => "restart.preparing",
                PadRestartTarget::NewDetachedSession(_) => "restart.starting",
            };
            send_text(
                &config.telegram.bot_token,
                chat_id,
                tg(locale, preparing_key),
            )
            .await?;
            if let Err(err_text) = execute_pad_restart_plan(&plan) {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    &tg_fmt(locale, "restart.failed", err_text),
                )
                .await?;
                play_sound_event(config, crate::sound::SoundEvent::Failure);
            }
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
            let stop_send_error = {
                let send_result = tmux_dispatch::send_escape(&target.pane_id);
                send_result.err().map(|err| err.to_string())
            };
            match stop_send_error {
                None => {
                    send_text(
                        &config.telegram.bot_token,
                        chat_id,
                        &tg_fmt(locale, "stop.sent", &target.label),
                    )
                    .await?;
                }
                Some(err_text) => {
                    send_text(
                        &config.telegram.bot_token,
                        chat_id,
                        &tg_fmt(locale, "stop.failed", err_text),
                    )
                    .await?;
                    play_sound_event(config, crate::sound::SoundEvent::Failure);
                }
            }
        }
        "/reset" => {
            let Some(target) = state.selected_target.as_ref() else {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    tg(locale, "target.none"),
                )
                .await?;
                return Ok(());
            };
            let target_label = target.label.clone();
            let Some(pending) = remove_selected_target_pending_request(state) else {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    &tg_fmt(locale, "reset.none", target_label),
                )
                .await?;
                return Ok(());
            };
            finalize_pending_feedback(config, &pending, tg(locale, "reset.status"));
            send_text(
                &config.telegram.bot_token,
                chat_id,
                &tg_fmt2(
                    locale,
                    "reset.done",
                    pending.request_id,
                    pending.target_label,
                ),
            )
            .await?;
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

pub(super) async fn send_session_diag(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
    arg: &str,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let context = resolve_session_diag_context(state, arg)?;
    let Some(context) = context else {
        let text = if arg.trim().is_empty() {
            tg(locale, "target.none")
        } else {
            tg(locale, "diag.empty")
        };
        send_text(&config.telegram.bot_token, chat_id, text).await?;
        return Ok(());
    };

    let body = format_session_diag_message(locale, &context);
    send_text(&config.telegram.bot_token, chat_id, &body).await?;
    Ok(())
}

pub(super) async fn send_pad_status_report(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let pad_status = runtime_status::describe_status(&crate::paths::pad_status_path());
    let body = build_pad_status_body(locale, &pad_status, state);
    send_text(&config.telegram.bot_token, chat_id, &body).await?;
    Ok(())
}

pub(super) async fn send_recent_history(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let Some(target) = state.selected_target.as_ref() else {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "target.none"),
        )
        .await?;
        return Ok(());
    };

    let panels = live_panels().map_err(telegram_error)?;
    let Some(panel) = panels.iter().find(|panel| panel.pane_id == target.pane_id) else {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "pane.stale"),
        )
        .await?;
        return Ok(());
    };

    if !history_supported_agent(&panel.agent_type) {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "history.unsupported"),
        )
        .await?;
        return Ok(());
    }

    let turns = recent_history_turns(panel, locale);
    if turns.is_empty() {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "history.empty"),
        )
        .await?;
        return Ok(());
    }

    let body = format_recent_history_message(locale, &compact_target_label(panel), &turns);
    send_text(&config.telegram.bot_token, chat_id, &body).await?;
    Ok(())
}

fn resolve_session_diag_context(
    state: &TelegramState,
    arg: &str,
) -> TelegramResult<Option<SessionDiagContext>> {
    let panels = live_panels().map_err(telegram_error)?;
    let arg = arg.trim();

    if !arg.is_empty() {
        if let Some(pending) = state
            .pending_requests
            .iter()
            .find(|pending| pending.request_id == arg)
        {
            let panel = panels.iter().find(|panel| panel.pane_id == pending.pane_id);
            let continuity = crate::session_continuity::load_snapshot_for(
                pending.session_id.as_deref(),
                pending
                    .transcript_path
                    .as_deref()
                    .or_else(|| panel.and_then(|panel| panel.transcript_path.as_deref())),
            );
            return Ok(Some(SessionDiagContext {
                target_label: pending.target_label.clone(),
                pane_id: Some(pending.pane_id.clone()),
                request_id: Some(pending.request_id.clone()),
                session_id: pending
                    .session_id
                    .clone()
                    .or_else(|| panel.and_then(|panel| panel.agent_session_id.clone())),
                transcript_path: pending
                    .transcript_path
                    .clone()
                    .or_else(|| panel.and_then(|panel| panel.transcript_path.clone())),
                continuity,
            }));
        }

        if let Some(panel) = panels.iter().find(|panel| panel.pane_id == arg) {
            let continuity = crate::session_continuity::load_snapshot_for(
                panel.agent_session_id.as_deref(),
                panel.transcript_path.as_deref(),
            );
            return Ok(Some(SessionDiagContext {
                target_label: compact_target_label(panel),
                pane_id: Some(panel.pane_id.clone()),
                request_id: state
                    .pending_requests
                    .iter()
                    .find(|pending| pending.pane_id == panel.pane_id)
                    .map(|pending| pending.request_id.clone()),
                session_id: panel.agent_session_id.clone(),
                transcript_path: panel.transcript_path.clone(),
                continuity,
            }));
        }

        let continuity = crate::session_continuity::load_snapshot_for(Some(arg), Some(arg));
        if continuity.is_some() {
            return Ok(Some(SessionDiagContext {
                target_label: arg.to_string(),
                pane_id: None,
                request_id: state
                    .pending_requests
                    .iter()
                    .find(|pending| pending.session_id.as_deref() == Some(arg))
                    .map(|pending| pending.request_id.clone()),
                session_id: Some(arg.to_string()),
                transcript_path: continuity
                    .as_ref()
                    .and_then(|snapshot| snapshot.transcript_path.clone()),
                continuity,
            }));
        }

        return Ok(None);
    }

    let selected = match state.selected_target.as_ref() {
        Some(selected) => selected,
        None => return Ok(None),
    };
    let panel = panels
        .iter()
        .find(|panel| panel.pane_id == selected.pane_id);
    let pending = state
        .pending_requests
        .iter()
        .find(|pending| pending.pane_id == selected.pane_id);
    let session_id = pending
        .and_then(|pending| pending.session_id.clone())
        .or_else(|| panel.and_then(|panel| panel.agent_session_id.clone()));
    let transcript_path = pending
        .and_then(|pending| pending.transcript_path.clone())
        .or_else(|| panel.and_then(|panel| panel.transcript_path.clone()));
    let continuity = crate::session_continuity::load_snapshot_for(
        session_id.as_deref(),
        transcript_path.as_deref(),
    );

    Ok(Some(SessionDiagContext {
        target_label: pending
            .map(|pending| pending.target_label.clone())
            .or_else(|| panel.map(compact_target_label))
            .unwrap_or_else(|| selected.label.clone()),
        pane_id: Some(selected.pane_id.clone()),
        request_id: pending.map(|pending| pending.request_id.clone()),
        session_id,
        transcript_path,
        continuity,
    }))
}

fn format_session_diag_message(
    locale: crate::i18n::Locale,
    context: &SessionDiagContext,
) -> String {
    let mut lines = vec![
        tg(locale, "diag.title").to_string(),
        context.target_label.clone(),
    ];
    if let Some(request_id) = context
        .request_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "meta.request"), request_id));
    }
    if let Some(pane_id) = context.pane_id.as_deref().filter(|value| !value.is_empty()) {
        lines.push(format!("{}: {}", tg(locale, "meta.pane"), pane_id));
    }
    if let Some(session_id) = context
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "meta.session"), session_id));
    }

    if let Some(snapshot) = context.continuity.as_ref() {
        lines.extend(super::pending::continuity_detail_lines(locale, snapshot));
    } else {
        lines.push(tg(locale, "diag.empty").to_string());
        if let Some(path) = context
            .transcript_path
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("{}: {}", tg(locale, "diag.transcript"), path));
        }
    }

    lines.join("\n")
}

pub(super) fn build_pad_status_body(
    locale: crate::i18n::Locale,
    pad_status: &str,
    state: &TelegramState,
) -> String {
    let target = state
        .selected_target
        .as_ref()
        .map(|target| target.label.clone())
        .unwrap_or_else(|| tg(locale, "status.none").to_string());
    let pending = if state.pending_requests.is_empty() {
        tg(locale, "status.pending_none").to_string()
    } else {
        state
            .pending_requests
            .iter()
            .map(|pending| pending_status_summary_line(locale, pending))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "{}: {}\n{}: {}\n{}:\n{}",
        tg(locale, "status.pad"),
        pad_status,
        tg(locale, "status.target"),
        target,
        tg(locale, "status.pending"),
        pending
    )
}

fn history_supported_agent(agent_type: &AgentType) -> bool {
    matches!(
        agent_type,
        AgentType::Codex | AgentType::Claude | AgentType::Gemini
    )
}

pub(super) fn recent_history_turns(
    panel: &AgentPanel,
    locale: crate::i18n::Locale,
) -> Vec<crate::model::PreviewTurn> {
    let request = crate::preview_source::PreviewRequest {
        target_key: panel.pane_id.clone(),
        live_pane_id: Some(panel.pane_id.clone()),
        agent_type: panel.agent_type.clone(),
        working_dir: panel.working_dir.clone(),
        state: panel.state.clone(),
        transcript_path: panel.transcript_path.clone(),
        cached_preview_turns: panel.cached_preview_turns.clone(),
        session_cache_state: panel.session_cache_state,
        agent_session_id: panel.agent_session_id.clone(),
        session_origin: Some(crate::model::PreviewSessionOrigin::Pane),
        persist_resolved_session: false,
        known_updated_at: None,
    };

    let update = crate::preview_source::load_preview(&request, "session", locale);
    let turns = if !update.turns.is_empty() {
        update.turns.to_vec()
    } else {
        panel.cached_preview_turns.to_vec()
    };
    turns.into_iter().take(3).collect()
}

pub(super) fn format_recent_history_message(
    locale: crate::i18n::Locale,
    target_label: &str,
    turns: &[crate::model::PreviewTurn],
) -> String {
    let mut lines = vec![
        tg(locale, "history.title").to_string(),
        target_label.to_string(),
    ];

    for (idx, turn) in turns.iter().enumerate() {
        lines.push(String::new());
        lines.push(format!("{}. Q:", idx + 1));
        lines.push(turn.question.trim().to_string());
        lines.push(String::new());
        lines.push("A:".to_string());
        lines.push(
            turn.answer
                .as_deref()
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .unwrap_or(tg(locale, "history.answer_missing"))
                .to_string(),
        );
    }

    lines.join("\n")
}

fn current_pad_restart_plan() -> Result<PadRestartPlan, String> {
    let build_dir = std::path::Path::new(PAD_CARGO_MANIFEST_DIR);
    if !build_dir.join("Cargo.toml").exists() {
        return Err(format!(
            "cargo manifest not found in {}",
            build_dir.display()
        ));
    }

    let current_exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let current_args = std::env::args().collect::<Vec<_>>();
    let shell_command = build_pad_restart_shell_command(
        &current_exe,
        &current_args,
        std::env::var("CARGO_TARGET_DIR").ok().as_deref(),
    );
    let target = current_pad_restart_target(&current_exe)?;

    Ok(PadRestartPlan {
        target,
        start_dir: build_dir.to_string_lossy().to_string(),
        shell_command,
    })
}

fn current_pad_restart_target(current_exe: &std::path::Path) -> Result<PadRestartTarget, String> {
    let current_tmux_pane = std::env::var("TMUX_PANE")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let pad_status_pid = runtime_status::read_status(&crate::paths::pad_status_path())
        .filter(|status| runtime_status::process_alive(status.pid))
        .map(|status| status.pid);

    let panes = if current_tmux_pane.is_some() {
        Vec::new()
    } else if tmux_dispatch::session_exists(PAD_DEFAULT_SESSION_NAME)
        .map_err(|err| err.to_string())?
    {
        tmux_dispatch::list_session_panes(PAD_DEFAULT_SESSION_NAME)
            .map_err(|err| err.to_string())?
    } else {
        Vec::new()
    };

    let expected_command = current_exe
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("pad");

    Ok(select_pad_restart_target(
        current_tmux_pane.as_deref(),
        PAD_DEFAULT_SESSION_NAME,
        &panes,
        pad_status_pid,
        expected_command,
    ))
}

pub(super) fn select_pad_restart_target(
    current_tmux_pane: Option<&str>,
    session_name: &str,
    session_panes: &[crate::tmux_dispatch::SessionPaneInfo],
    pad_pid: Option<u32>,
    expected_command: &str,
) -> PadRestartTarget {
    if let Some(pane_id) = current_tmux_pane.filter(|value| !value.trim().is_empty()) {
        return PadRestartTarget::RespawnPane(pane_id.to_string());
    }

    if let Some(pid) = pad_pid {
        if let Some(pane) = session_panes.iter().find(|pane| pane.pid == Some(pid)) {
            return PadRestartTarget::RespawnPane(pane.pane_id.clone());
        }
    }

    if let Some(pane) = session_panes
        .iter()
        .find(|pane| pane.command.trim() == expected_command)
    {
        return PadRestartTarget::RespawnPane(pane.pane_id.clone());
    }

    if let Some(first) = session_panes.first() {
        return PadRestartTarget::RespawnPane(first.pane_id.clone());
    }

    PadRestartTarget::NewDetachedSession(session_name.to_string())
}

pub(super) fn build_pad_restart_shell_command(
    current_exe: &std::path::Path,
    current_args: &[String],
    cargo_target_dir: Option<&str>,
) -> String {
    let mut steps = Vec::new();
    if let Some(cargo_target_dir) = cargo_target_dir.filter(|value| !value.trim().is_empty()) {
        steps.push(format!(
            "export CARGO_TARGET_DIR={}",
            shell_single_quote(cargo_target_dir)
        ));
    }

    let build_cmd = if restart_uses_release_profile(current_exe) {
        "cargo build --release".to_string()
    } else {
        "cargo build".to_string()
    };
    steps.push(build_cmd);

    let mut exec_parts = vec![
        "exec".to_string(),
        shell_single_quote(&current_exe.to_string_lossy()),
    ];
    for arg in pad_restart_args(current_args) {
        exec_parts.push(shell_single_quote(&arg));
    }
    steps.push(exec_parts.join(" "));

    steps.join(" && ")
}

fn execute_pad_restart_plan(plan: &PadRestartPlan) -> Result<(), String> {
    log_debug!(
        "telegram: executing pad restart target={:?} start_dir={} command={}",
        plan.target,
        plan.start_dir,
        plan.shell_command
    );

    match &plan.target {
        PadRestartTarget::RespawnPane(pane_id) => {
            tmux_dispatch::respawn_pane_shell(pane_id, &plan.start_dir, &plan.shell_command)
                .map_err(|err| err.to_string())
        }
        PadRestartTarget::NewDetachedSession(session_name) => {
            tmux_dispatch::new_detached_session_shell(
                session_name,
                &plan.start_dir,
                &plan.shell_command,
            )
            .map_err(|err| err.to_string())
        }
    }
}

fn restart_uses_release_profile(current_exe: &std::path::Path) -> bool {
    current_exe
        .components()
        .any(|component| component.as_os_str() == "release")
}

fn pad_restart_args(current_args: &[String]) -> Vec<String> {
    current_args
        .iter()
        .skip(1)
        .filter(|arg| arg.as_str() != "telegram-bot")
        .cloned()
        .collect()
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

pub(super) async fn dispatch_codex_slash_command(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
    command: &str,
    arg: &str,
    deadline_ms: u64,
) -> TelegramResult<()> {
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

    let panels = live_panels().map_err(telegram_error)?;
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
    tmux_dispatch::dispatch_prompt(&panel.pane_id, &slash).map_err(telegram_error)?;
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

pub(super) async fn handle_plain_text(
    config: &Config,
    state: &mut TelegramState,
    chat_id: &str,
    text: &str,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    if text.trim().is_empty() {
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
    if pending_request_index_by_pane(state, &target.pane_id).is_some() {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "pending.exists"),
        )
        .await?;
        return Ok(());
    }

    let panels = live_panels().map_err(telegram_error)?;
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

    tmux_dispatch::dispatch_prompt(&panel.pane_id, text).map_err(telegram_error)?;
    invalidate_live_panels();
    let request_id = next_request_id();
    let transcript_path = panel.transcript_path.clone();
    let result_scan_offset = transcript_path.as_deref().map(transcript_len).unwrap_or(0);
    let failure_scan_offset = result_scan_offset;
    let approval_scan_offset = transcript_path.as_deref().map(transcript_len).unwrap_or(0);
    let sent_at = now_ts();
    let sent_at_ms = now_ms_i64();
    state.pending_requests.push(PendingRequest {
        request_id: request_id.clone(),
        chat_id: chat_id.to_string(),
        pane_id: panel.pane_id.clone(),
        agent_kind: panel.agent_type.to_string(),
        target_label: compact_target_label(panel),
        session_id: panel.agent_session_id.clone(),
        working_dir: panel.working_dir.clone(),
        prompt_text: text.to_string(),
        prompt_hash: format!("{:x}", md5::compute(text.as_bytes())),
        turn_id: None,
        sent_at,
        sent_at_ms,
        accepted_at: None,
        accepted_at_ms: None,
        last_status_at: None,
        draft_id: next_draft_id(),
        phase: "awaiting_submit".to_string(),
        transcript_path,
        result_scan_offset,
        failure_scan_offset,
        last_failure_check_at: None,
        approval_scan_offset,
        approval_call_id: None,
        approval_justification: None,
        completed_text: None,
        completed_source: None,
        delivery_attempts: 0,
        delivery_retry_at: 0,
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

pub(super) async fn send_help_message(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
    page: HelpPage,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let _: serde_json::Value = send_message(
        &config.telegram.bot_token,
        &help_message_payload(locale, state, telegram_chat_id_value(chat_id), None, page),
    )
    .await?;
    Ok(())
}

pub(super) async fn edit_help_message(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
    message_id: i64,
    page: HelpPage,
) -> TelegramResult<()> {
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

pub(super) async fn send_agent_list(
    config: &Config,
    state: &mut TelegramState,
    chat_id: &str,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let panels = live_panels().map_err(telegram_error)?;
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

pub(super) fn capture_looks_like_echo_only(capture: &str, slash: &str) -> bool {
    let trimmed = capture.trim();
    trimmed == slash.trim() || trimmed.ends_with(&format!("\n{}", slash.trim()))
}

pub(super) async fn poll_slash_reply(
    pane_id: &str,
    slash: &str,
    baseline: &str,
    deadline_ms: u64,
) -> TelegramResult<Option<String>> {
    let started = Instant::now();
    let deadline = Duration::from_millis(deadline_ms);
    let mut candidate: Option<String> = None;
    let mut stable_hits = 0usize;

    loop {
        let capture = tmux_dispatch::capture_pane_tail(pane_id, 28).map_err(telegram_error)?;
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
