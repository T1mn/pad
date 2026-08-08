use super::*;

mod command;
mod diag;
mod help_actions {
    use super::*;

    pub(crate) async fn send_help_message(
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

    pub(crate) async fn edit_help_message(
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

    pub(crate) async fn send_agent_list(
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

        let body = agent_list_body(&snapshot);
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

    fn agent_list_body(snapshot: &[AgentSnapshotEntry]) -> String {
        let mut body = String::new();
        for (idx, entry) in snapshot.iter().enumerate() {
            if idx > 0 {
                body.push('\n');
            }
            body.push_str(&entry.label);
        }
        body
    }
}
mod history {
    use super::*;
    use std::fmt::Write;

    pub(crate) async fn send_recent_history(
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

    fn history_supported_agent(agent_type: &AgentType) -> bool {
        matches!(
            agent_type,
            AgentType::Codex | AgentType::Claude | AgentType::Gemini
        )
    }

    pub(crate) fn recent_history_turns(
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

    pub(crate) fn format_recent_history_message(
        locale: crate::i18n::Locale,
        target_label: &str,
        turns: &[crate::model::PreviewTurn],
    ) -> String {
        let mut body = String::new();
        body.push_str(tg(locale, "history.title"));
        body.push('\n');
        body.push_str(target_label);

        for (idx, turn) in turns.iter().enumerate() {
            body.push_str("\n\n");
            let _ = writeln!(body, "{}. Q:", idx + 1);
            body.push_str(turn.question.trim());
            body.push_str("\n\nA:\n");
            body.push_str(
                turn.answer
                    .as_deref()
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .unwrap_or(tg(locale, "history.answer_missing")),
            );
        }

        body
    }
}
mod plain {
    use super::*;

    pub(crate) async fn handle_plain_text(
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

        terminal_remote::dispatch_prompt(&panel.pane_id, text).map_err(telegram_error)?;
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
}
mod restart {
    use super::{PadRestartPlan, PadRestartTarget, PAD_CARGO_MANIFEST_DIR};

    pub(super) fn execute_pad_restart_plan(plan: &PadRestartPlan) -> Result<(), String> {
        crate::log_debug!(
            "telegram: spawning native PAD restart start_dir={} command={}",
            plan.start_dir,
            plan.shell_command
        );
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        std::process::Command::new(shell)
            .args(["-lc", &plan.shell_command])
            .current_dir(&plan.start_dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub(crate) fn build_pad_restart_shell_command(
        current_exe: &std::path::Path,
        current_args: &[String],
        cargo_target_dir: Option<&str>,
    ) -> String {
        let mut command = String::new();
        if let Some(target_dir) = cargo_target_dir.filter(|value| !value.trim().is_empty()) {
            command.push_str("export CARGO_TARGET_DIR=");
            command.push_str(&crate::shell_quote::single_quote(target_dir));
            command.push_str(" && ");
        }
        let profile = if current_exe
            .components()
            .any(|component| component.as_os_str() == "release")
        {
            "cargo build --release"
        } else {
            "cargo build"
        };
        command.push_str(profile);
        command.push_str(" && exec ");
        command.push_str(&crate::shell_quote::single_quote(
            &current_exe.to_string_lossy(),
        ));
        for argument in current_args
            .iter()
            .skip(1)
            .filter(|argument| argument.as_str() != "telegram-bot")
        {
            command.push(' ');
            command.push_str(&crate::shell_quote::single_quote(argument));
        }
        command
    }

    pub(super) fn current_pad_restart_plan() -> Result<PadRestartPlan, String> {
        let build_dir = std::path::Path::new(PAD_CARGO_MANIFEST_DIR);
        if !build_dir.join("Cargo.toml").exists() {
            return Err(format!(
                "cargo manifest not found in {}",
                build_dir.display()
            ));
        }
        let current_exe = std::env::current_exe().map_err(|error| error.to_string())?;
        let current_args = std::env::args().collect::<Vec<_>>();
        Ok(PadRestartPlan {
            target: PadRestartTarget::NativeProcess,
            start_dir: build_dir.to_string_lossy().to_string(),
            shell_command: build_pad_restart_shell_command(
                &current_exe,
                &current_args,
                std::env::var("CARGO_TARGET_DIR").ok().as_deref(),
            ),
        })
    }
}
mod slash {
    mod poll {
        use super::*;

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
                let capture =
                    terminal_remote::capture_pane_tail(pane_id, 28).map_err(telegram_error)?;
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

        fn capture_looks_like_echo_only(capture: &str, slash: &str) -> bool {
            let trimmed = capture.trim();
            trimmed == slash.trim() || trimmed.ends_with(&format!("\n{}", slash.trim()))
        }
    }
    mod target {
        use super::*;

        pub(super) async fn resolve_codex_slash_panel(
            config: &Config,
            state: &TelegramState,
            chat_id: &str,
            locale: crate::i18n::Locale,
        ) -> TelegramResult<Option<AgentPanel>> {
            let Some(target) = state.selected_target.as_ref() else {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    tg(locale, "target.none"),
                )
                .await?;
                return Ok(None);
            };

            let panels = live_panels().map_err(telegram_error)?;
            let Some(panel) = panels.iter().find(|panel| panel.pane_id == target.pane_id) else {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    tg(locale, "pane.stale"),
                )
                .await?;
                return Ok(None);
            };

            if !matches!(&panel.agent_type, &AgentType::Codex) {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    tg(locale, "target.not_codex"),
                )
                .await?;
                return Ok(None);
            }

            if panel.state == AgentState::Busy {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    tg(locale, "agent.busy"),
                )
                .await?;
                return Ok(None);
            }
            if panel.state == AgentState::Waiting {
                send_text(
                    &config.telegram.bot_token,
                    chat_id,
                    tg(locale, "agent.waiting"),
                )
                .await?;
                return Ok(None);
            }

            Ok(Some(panel.clone()))
        }
    }

    use super::*;

    pub(crate) async fn dispatch_codex_slash_command(
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

        let Some(panel) = target::resolve_codex_slash_panel(config, state, chat_id, locale).await?
        else {
            return Ok(());
        };

        let slash = build_slash_command_text(command, arg);
        let baseline = terminal_remote::capture_pane_tail(&panel.pane_id, 28)
            .map(|capture| summarize_pane_capture(&capture))
            .unwrap_or_default();
        terminal_remote::dispatch_prompt(&panel.pane_id, &slash).map_err(telegram_error)?;
        invalidate_live_panels();
        log_debug!(
            "telegram: dispatched codex slash command pane={} command={}",
            panel.pane_id,
            slash
        );

        let reply =
            match poll::poll_slash_reply(&panel.pane_id, &slash, &baseline, deadline_ms).await {
                Ok(Some(capture)) => slash_reply_with_capture(locale, &slash, &panel, &capture),
                Ok(None) => slash_sent_reply(locale, &slash, &panel),
                Err(err) => {
                    log_debug!(
                        "telegram: capture after slash command failed pane={} command={} err={}",
                        panel.pane_id,
                        slash,
                        err
                    );
                    slash_sent_reply(locale, &slash, &panel)
                }
            };
        send_text(&config.telegram.bot_token, chat_id, &reply).await?;
        Ok(())
    }

    fn slash_reply_with_capture(
        locale: crate::i18n::Locale,
        slash: &str,
        panel: &AgentPanel,
        capture: &str,
    ) -> String {
        if capture.is_empty() {
            slash_sent_reply(locale, slash, panel)
        } else {
            tg_fmt3(
                locale,
                "slash.output",
                slash,
                compact_target_label(panel),
                capture,
            )
        }
    }

    fn slash_sent_reply(locale: crate::i18n::Locale, slash: &str, panel: &AgentPanel) -> String {
        tg_fmt2(locale, "slash.sent", slash, compact_target_label(panel))
    }
}
mod update {
    use super::*;

    pub(crate) async fn handle_update(
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
                config.save_or_log();
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
            super::handle_command(config, state, &chat_id, &text).await?;
        } else {
            super::handle_plain_text(config, state, &chat_id, &text).await?;
        }

        Ok(())
    }
}

const PAD_CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum PadRestartTarget {
    NativeProcess,
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

use command::handle_command;
use diag::send_session_diag;
use help_actions::send_help_message;
use history::send_recent_history;
use plain::handle_plain_text;
use slash::dispatch_codex_slash_command;

pub(super) use diag::send_pad_status_report;
pub(super) use help_actions::{edit_help_message, send_agent_list};
pub(super) use update::handle_update;

#[cfg(test)]
pub(super) use diag::build_pad_status_body;
#[cfg(test)]
pub(super) use history::{format_recent_history_message, recent_history_turns};
#[cfg(test)]
pub(super) use restart::build_pad_restart_shell_command;
