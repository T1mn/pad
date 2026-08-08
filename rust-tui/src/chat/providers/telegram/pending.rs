pub(super) use super::*;

mod approval;
mod failures;
mod feedback {
    use super::*;

    pub(crate) struct DraftFeedbackGate {
        pub(super) latest_seq: AtomicU64,
        pub(super) send_lock: AsyncMutex<()>,
    }

    pub(crate) fn refresh_pending_feedback(
        config: &Config,
        state: &mut TelegramState,
        force: bool,
    ) {
        let locale = telegram_locale(config);
        let now = now_ts();

        for pending in &mut state.pending_requests {
            if !force {
                let Some(accepted_at) = pending.accepted_at else {
                    continue;
                };
                if accepted_at <= 0 {
                    continue;
                }
                if let Some(last_status_at) = pending.last_status_at {
                    if now.saturating_sub(last_status_at) < 4 {
                        continue;
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
    }

    pub(crate) fn finalize_pending_feedback(
        config: &Config,
        pending: &PendingRequest,
        status: &str,
    ) {
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
}
mod journal {
    use super::*;

    pub(crate) async fn process_hook_journal(
        config: &Config,
        state: &mut TelegramState,
    ) -> TelegramResult<()> {
        super::super::hooks::sync_state_from_disk_public(state);
        if state.pending_requests.is_empty() {
            state.journal_position = journal_len();
            return Ok(());
        }

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
            super::super::hooks::sync_state_from_disk_public(state);
            if state.pending_requests.is_empty() {
                line.clear();
                break;
            }

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
                    let _ = apply_hook_event_to_pending(config, state, &event).await?;
                }
                Err(err) => {
                    log_debug!("telegram: invalid hook journal line: {}", err);
                }
            }
            line.clear();
        }

        Ok(())
    }
}
mod results {
    use super::*;

    pub(crate) async fn process_pending_result_delivery(
        config: &Config,
        state: &mut TelegramState,
    ) -> TelegramResult<()> {
        let now = now_ts();
        let request_ids = state
            .pending_requests
            .iter()
            .filter(|pending| {
                pending.phase == "delivering_result" && pending.delivery_retry_at <= now
            })
            .map(|pending| pending.request_id.clone())
            .collect::<Vec<_>>();

        for request_id in request_ids {
            if let Err(err) =
                deliver_pending_result(config, state, telegram_locale(config), &request_id).await
            {
                log_debug!(
                    "telegram: result delivery retry failed request_id={} err={}",
                    request_id,
                    err
                );
            }
        }

        Ok(())
    }

    pub(crate) fn completed_reply_text(
        locale: crate::i18n::Locale,
        pending: &PendingRequest,
        result_text: &str,
    ) -> String {
        let mut reply = String::new();
        push_reply_line(&mut reply, tg(locale, "result.title"));
        push_reply_line(
            &mut reply,
            &format!("{}: {}", tg(locale, "meta.request"), pending.request_id),
        );
        push_reply_line(
            &mut reply,
            &format!("{}: {}", tg(locale, "meta.target"), pending.target_label),
        );
        push_reply_line(
            &mut reply,
            &format!("{}: {}", tg(locale, "meta.pane"), pending.pane_id),
        );
        if let Some(session_id) = pending
            .session_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            push_reply_line(
                &mut reply,
                &format!("{}: {}", tg(locale, "meta.session"), session_id),
            );
        }
        if let Some(turn_id) = pending.turn_id.as_deref().filter(|value| !value.is_empty()) {
            push_reply_line(
                &mut reply,
                &format!("{}: {}", tg(locale, "meta.turn"), turn_id),
            );
        }
        if !pending.working_dir.trim().is_empty() {
            push_reply_line(
                &mut reply,
                &format!("{}: {}", tg(locale, "meta.dir"), pending.working_dir),
            );
        }
        reply.push_str("\n\n");
        reply.push_str(result_text);
        reply
    }

    fn push_reply_line(out: &mut String, line: &str) {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(line);
    }

    pub(crate) async fn deliver_pending_result(
        config: &Config,
        state: &mut TelegramState,
        locale: crate::i18n::Locale,
        request_id: &str,
    ) -> TelegramResult<()> {
        let Some(snapshot) = state
            .pending_requests
            .iter()
            .find(|pending| pending.request_id == request_id)
            .cloned()
        else {
            return Ok(());
        };
        if snapshot.phase != "delivering_result" {
            return Ok(());
        }

        let result_text = snapshot
            .completed_text
            .clone()
            .unwrap_or_else(|| tg(locale, "result.missing").to_string());
        let reply = completed_reply_text(locale, &snapshot, &result_text);
        match send_text(&config.telegram.bot_token, &snapshot.chat_id, &reply).await {
            Ok(()) => {
                finalize_pending_feedback(config, &snapshot, tg(locale, "phase.completed"));
                remove_pending_request(state, request_id);
                Ok(())
            }
            Err(err) => {
                if let Some(index) = pending_request_index_by_id(state, request_id) {
                    let pending = &mut state.pending_requests[index];
                    pending.delivery_attempts = pending.delivery_attempts.saturating_add(1);
                    pending.delivery_retry_at = now_ts().saturating_add(RESULT_DELIVERY_RETRY_SECS);
                    pending.last_status_at = None;
                }
                Err(err)
            }
        }
    }
}
mod status {
    use super::*;

    mod continuity {
        use super::super::*;

        pub(in crate::chat::providers::telegram::pending) fn continuity_status_line(
            locale: crate::i18n::Locale,
            snapshot: &crate::session_continuity::ContinuitySnapshot,
        ) -> String {
            let mut line = format!(
                "{}: {}",
                tg(locale, "diag.summary"),
                continuity_health_text(locale, snapshot.health)
            );
            if let Some(lag_seconds) = snapshot.lag_seconds.filter(|lag| *lag > 0) {
                line.push_str(" · ");
                line.push_str(&tg_fmt(locale, "diag.lag_short", lag_seconds));
            }
            if snapshot.attempt_classification
                != crate::session_continuity::ContinuityAttemptClassification::Normal
            {
                line.push_str(" · ");
                line.push_str(continuity_attempt_text(
                    locale,
                    snapshot.attempt_classification,
                ));
            }
            line
        }

        pub(crate) fn continuity_detail_lines(
            locale: crate::i18n::Locale,
            snapshot: &crate::session_continuity::ContinuitySnapshot,
        ) -> Vec<String> {
            let mut lines = vec![
                format!(
                    "{}: {}",
                    tg(locale, "diag.health"),
                    continuity_health_text(locale, snapshot.health)
                ),
                format!(
                    "{}: {}",
                    tg(locale, "diag.classification"),
                    continuity_attempt_text(locale, snapshot.attempt_classification)
                ),
            ];
            if let Some(lag_seconds) = snapshot.lag_seconds {
                lines.push(format!(
                    "{}: {}",
                    tg(locale, "diag.lag"),
                    tg_fmt(locale, "diag.lag_short", lag_seconds)
                ));
            }
            if let Some(event) = snapshot
                .last_hook_event
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                lines.push(format!("{}: {}", tg(locale, "diag.last_hook"), event));
            }
            if snapshot.stale_event_count > 0 {
                lines.push(format!(
                    "{}: {}",
                    tg(locale, "diag.stale_events"),
                    snapshot.stale_event_count
                ));
            }
            if let Some(path) = snapshot
                .transcript_path
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                lines.push(format!("{}: {}", tg(locale, "diag.transcript"), path));
            }
            lines
        }

        fn continuity_health_text(
            locale: crate::i18n::Locale,
            health: crate::session_continuity::ContinuityHealth,
        ) -> &'static str {
            match (locale_prefers_chinese(locale), health) {
                (true, crate::session_continuity::ContinuityHealth::Healthy) => "健康",
                (true, crate::session_continuity::ContinuityHealth::Lagging) => "滞后",
                (true, crate::session_continuity::ContinuityHealth::Frozen) => "冻结",
                (false, crate::session_continuity::ContinuityHealth::Healthy) => "healthy",
                (false, crate::session_continuity::ContinuityHealth::Lagging) => "lagging",
                (false, crate::session_continuity::ContinuityHealth::Frozen) => "frozen",
            }
        }

        fn continuity_attempt_text(
            locale: crate::i18n::Locale,
            attempt: crate::session_continuity::ContinuityAttemptClassification,
        ) -> &'static str {
            match (locale_prefers_chinese(locale), attempt) {
                (true, crate::session_continuity::ContinuityAttemptClassification::Normal) => "正常",
                (
                    true,
                    crate::session_continuity::ContinuityAttemptClassification::TransientResumeBootstrap,
                ) => "短暂 resume 引导",
                (false, crate::session_continuity::ContinuityAttemptClassification::Normal) => "normal",
                (
                    false,
                    crate::session_continuity::ContinuityAttemptClassification::TransientResumeBootstrap,
                ) => "transient_resume_bootstrap",
            }
        }
    }
    mod metadata {
        use super::super::*;

        pub(super) fn pending_metadata_lines(
            locale: crate::i18n::Locale,
            pending: &PendingRequest,
            include_turn: bool,
        ) -> Vec<String> {
            let mut lines = vec![
                format!("{}: {}", tg(locale, "meta.request"), pending.request_id),
                format!("{}: {}", tg(locale, "meta.pane"), pending.pane_id),
            ];
            if let Some(session_id) = pending
                .session_id
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                lines.push(format!("{}: {}", tg(locale, "meta.session"), session_id));
            }
            if include_turn {
                if let Some(turn_id) = pending.turn_id.as_deref().filter(|value| !value.is_empty())
                {
                    lines.push(format!("{}: {}", tg(locale, "meta.turn"), turn_id));
                }
            }
            if !pending.working_dir.trim().is_empty() {
                lines.push(format!(
                    "{}: {}",
                    tg(locale, "meta.dir"),
                    pending.working_dir
                ));
            }
            lines
        }
    }

    pub(crate) use continuity::continuity_detail_lines;
    pub(super) use continuity::continuity_status_line;
    use metadata::pending_metadata_lines;

    pub(super) fn phase_label(locale: crate::i18n::Locale, phase: &str) -> String {
        match phase {
            "awaiting_submit" => tg(locale, "phase.awaiting_submit").to_string(),
            "awaiting_confirm" => tg(locale, "phase.awaiting_confirm").to_string(),
            "awaiting_stop" => tg(locale, "phase.accepted").to_string(),
            "delivering_result" => tg(locale, "phase.delivering").to_string(),
            _ => phase.to_string(),
        }
    }

    pub(crate) fn pending_status_summary_line(
        locale: crate::i18n::Locale,
        pending: &PendingRequest,
    ) -> String {
        format!(
            "{} • {} • {} • {}",
            pending.request_id,
            pending.pane_id,
            pending.target_label,
            phase_label(locale, &pending.phase)
        )
    }

    pub(crate) fn pending_status_text(
        locale: crate::i18n::Locale,
        pending: &PendingRequest,
        now: i64,
    ) -> String {
        if pending.approval_call_id.is_some() {
            let mut text = String::new();
            push_status_line(&mut text, tg(locale, "phase.awaiting_confirm"));
            push_status_line(&mut text, &pending.target_label);
            for line in pending_metadata_lines(locale, pending, false) {
                push_status_line(&mut text, &line);
            }
            if let Some(justification) = pending.approval_justification.as_deref() {
                push_status_line(&mut text, &truncate_chars(justification, 220));
            }
            return text;
        }

        let headline = match pending.phase.as_str() {
            "awaiting_submit" => tg(locale, "phase.awaiting_submit").to_string(),
            "awaiting_stop" => match pending.accepted_at {
                Some(accepted_at) if now.saturating_sub(accepted_at) >= 4 => {
                    tg_fmt(locale, "phase.working", now.saturating_sub(accepted_at))
                }
                _ => tg(locale, "phase.accepted").to_string(),
            },
            "delivering_result" => tg(locale, "phase.delivering").to_string(),
            _ => tg(locale, "phase.completed").to_string(),
        };

        let mut text = String::new();
        push_status_line(&mut text, &headline);
        push_status_line(&mut text, &pending.target_label);
        for line in pending_metadata_lines(locale, pending, false) {
            push_status_line(&mut text, &line);
        }
        if let Some(snapshot) = pending_continuity_snapshot(pending) {
            if snapshot.health != crate::session_continuity::ContinuityHealth::Healthy
                || snapshot.attempt_classification
                    != crate::session_continuity::ContinuityAttemptClassification::Normal
            {
                push_status_line(&mut text, &continuity_status_line(locale, &snapshot));
            }
        }
        text
    }

    fn push_status_line(out: &mut String, line: &str) {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(line);
    }

    fn pending_continuity_snapshot(
        pending: &PendingRequest,
    ) -> Option<crate::session_continuity::ContinuitySnapshot> {
        crate::session_continuity::load_snapshot_for(
            pending.session_id.as_deref(),
            pending.transcript_path.as_deref(),
        )
    }
}
mod timeouts {
    use super::*;

    pub(crate) async fn process_pending_timeout(
        config: &Config,
        state: &mut TelegramState,
    ) -> TelegramResult<()> {
        let locale = telegram_locale(config);
        let now = now_ts();
        let timed_out = state
            .pending_requests
            .iter()
            .filter(|pending| now.saturating_sub(pending.sent_at) >= PENDING_TIMEOUT_SECS)
            .cloned()
            .collect::<Vec<_>>();

        for pending in timed_out {
            remove_pending_request(state, &pending.request_id);
            finalize_pending_feedback(config, &pending, tg(locale, "phase.completed"));
            send_text(
                &config.telegram.bot_token,
                &pending.chat_id,
                &tg_fmt(locale, "timeout", &pending.request_id),
            )
            .await?;
            play_sound_event(config, crate::sound::SoundEvent::Timeout);
        }

        Ok(())
    }
}
mod timing {
    use super::*;

    pub(crate) fn pending_sent_ms(pending: &PendingRequest) -> i64 {
        if pending.sent_at_ms > 0 {
            pending.sent_at_ms
        } else {
            pending.sent_at.saturating_mul(1000)
        }
    }

    pub(crate) fn pending_accepted_ms(pending: &PendingRequest) -> i64 {
        pending.accepted_at_ms.unwrap_or_else(|| {
            pending
                .accepted_at
                .unwrap_or(pending.sent_at)
                .saturating_mul(1000)
        })
    }
}

pub(super) use approval::process_codex_pending_approval;
pub(super) use failures::process_pending_rollout_failures;
pub(super) use feedback::{finalize_pending_feedback, refresh_pending_feedback, DraftFeedbackGate};
pub(super) use journal::process_hook_journal;
#[cfg(test)]
pub(super) use results::completed_reply_text;
pub(super) use results::{deliver_pending_result, process_pending_result_delivery};
pub(super) use status::{
    continuity_detail_lines, pending_status_summary_line, pending_status_text,
};
pub(super) use timeouts::process_pending_timeout;
pub(super) use timing::{pending_accepted_ms, pending_sent_ms};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_continuity::{
        ContinuityAttemptClassification, ContinuityHealth, ContinuitySnapshot,
    };
    use std::fs;

    fn sample_pending() -> PendingRequest {
        PendingRequest {
            request_id: "tg-1".into(),
            chat_id: "1".into(),
            pane_id: "%1".into(),
            agent_kind: "codex".into(),
            target_label: "CODEX • 1".into(),
            session_id: Some("session-1".into()),
            working_dir: "/tmp/test".into(),
            prompt_text: "hi".into(),
            prompt_hash: "abc".into(),
            turn_id: Some("turn-1".into()),
            sent_at: 100,
            sent_at_ms: 100_000,
            accepted_at: Some(105),
            accepted_at_ms: Some(105_000),
            last_status_at: None,
            draft_id: 7,
            phase: "awaiting_stop".into(),
            transcript_path: None,
            result_scan_offset: 0,
            failure_scan_offset: 0,
            last_failure_check_at: None,
            approval_scan_offset: 0,
            approval_call_id: None,
            approval_justification: None,
            completed_text: None,
            completed_source: None,
            delivery_attempts: 0,
            delivery_retry_at: 0,
        }
    }

    #[test]
    fn rollout_failure_check_waits_30_seconds_and_then_5_second_backoff() {
        let pending = sample_pending();
        assert!(!super::failures::pending_rollout_failure_check_due(
            &pending, 134
        ));
        assert!(super::failures::pending_rollout_failure_check_due(
            &pending, 135
        ));

        let mut checked = sample_pending();
        checked.last_failure_check_at = Some(135);
        assert!(!super::failures::pending_rollout_failure_check_due(
            &checked, 139
        ));
        assert!(super::failures::pending_rollout_failure_check_due(
            &checked, 140
        ));
    }

    #[test]
    fn detect_pending_rollout_failure_removes_pending_and_updates_scan_offset() {
        let path = std::env::temp_dir().join(format!(
            "pad-telegram-rollout-failure-{}.jsonl",
            std::process::id()
        ));
        let body = concat!(
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"commentary\",\"content\":[{\"type\":\"output_text\",\"text\":\"still working\"}]}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"error\",\"message\":\"unexpected status 502 Bad Gateway\",\"codex_error_info\":\"other\"}}\n"
        );
        fs::write(&path, body).unwrap();

        let mut pending = sample_pending();
        pending.transcript_path = Some(path.to_string_lossy().into_owned());
        pending.failure_scan_offset = body.lines().next().unwrap().len() as u64 + 1;
        let mut state = TelegramState {
            pending_requests: vec![pending],
            ..TelegramState::default()
        };

        let resolution =
            super::failures::detect_pending_rollout_failure_for_request(&mut state, "tg-1", 140)
                .unwrap();
        let resolution = resolution.expect("failure resolution");
        assert_eq!(resolution.pending.request_id, "tg-1");
        assert_eq!(
            resolution.failure.message,
            "unexpected status 502 Bad Gateway"
        );
        assert_eq!(resolution.failure.error_info.as_deref(), Some("other"));
        assert!(state.pending_requests.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn detect_pending_rollout_failure_updates_last_check_when_no_error_is_found() {
        let path = std::env::temp_dir().join(format!(
            "pad-telegram-rollout-no-failure-{}.jsonl",
            std::process::id()
        ));
        let body = "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"commentary\",\"content\":[{\"type\":\"output_text\",\"text\":\"still working\"}]}}\n";
        fs::write(&path, body).unwrap();

        let mut pending = sample_pending();
        pending.transcript_path = Some(path.to_string_lossy().into_owned());
        let mut state = TelegramState {
            pending_requests: vec![pending],
            ..TelegramState::default()
        };

        let resolution =
            super::failures::detect_pending_rollout_failure_for_request(&mut state, "tg-1", 140)
                .unwrap();
        assert!(resolution.is_none());
        assert_eq!(state.pending_requests.len(), 1);
        assert_eq!(state.pending_requests[0].last_failure_check_at, Some(140));
        assert_eq!(
            state.pending_requests[0].failure_scan_offset,
            fs::metadata(&path).unwrap().len()
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn continuity_status_line_formats_health_and_lag() {
        let snapshot = ContinuitySnapshot {
            session_id: "session-1".into(),
            agent_type: Some("codex".into()),
            transcript_path: Some("/tmp/demo.jsonl".into()),
            last_hook_event: Some("user_prompt_submit".into()),
            last_turn_id: Some("turn-1".into()),
            last_hook_event_at: Some(200),
            last_prompt_submit_at: Some(200),
            last_stop_at: None,
            last_assistant_message_at: None,
            last_hook_cache_persist_at: Some(201),
            last_resolver_sync_at: None,
            last_thread_updated_at: Some(201),
            last_rollout_seen_at: Some(160),
            last_rollout_mtime: Some(160),
            last_rollout_size: Some(12),
            lag_seconds: Some(41),
            stale_event_count: 2,
            bootstrap_event_count: 0,
            health: ContinuityHealth::Frozen,
            attempt_classification: ContinuityAttemptClassification::Normal,
            updated_at: 201,
        };

        let line = super::status::continuity_status_line(crate::i18n::Locale::En, &snapshot);
        assert!(line.contains("Continuity"));
        assert!(line.contains("frozen"));
        assert!(line.contains("41s"));
    }

    #[test]
    fn pending_failure_reply_includes_continuity_details() {
        let pending = sample_pending();
        let failure = crate::chat::approval::CodexFailureEvent {
            message: "unexpected status 502 Bad Gateway".into(),
            error_info: Some("other".into()),
        };
        let snapshot = ContinuitySnapshot {
            session_id: "session-1".into(),
            agent_type: Some("codex".into()),
            transcript_path: Some("/tmp/demo.jsonl".into()),
            last_hook_event: Some("user_prompt_submit".into()),
            last_turn_id: Some("turn-1".into()),
            last_hook_event_at: Some(200),
            last_prompt_submit_at: Some(200),
            last_stop_at: None,
            last_assistant_message_at: None,
            last_hook_cache_persist_at: Some(201),
            last_resolver_sync_at: None,
            last_thread_updated_at: Some(201),
            last_rollout_seen_at: Some(160),
            last_rollout_mtime: Some(160),
            last_rollout_size: Some(12),
            lag_seconds: Some(41),
            stale_event_count: 2,
            bootstrap_event_count: 0,
            health: ContinuityHealth::Frozen,
            attempt_classification: ContinuityAttemptClassification::Normal,
            updated_at: 201,
        };

        let reply = super::failures::pending_failure_reply_text(
            crate::i18n::Locale::En,
            &pending,
            &failure,
            Some(&snapshot),
        );
        assert!(reply.contains("Error kind: other"));
        assert!(reply.contains("Health: frozen"));
        assert!(reply.contains("Lag: 41s"));
        assert!(reply.contains("Transcript: /tmp/demo.jsonl"));
        assert!(reply.ends_with("unexpected status 502 Bad Gateway"));
    }
}
