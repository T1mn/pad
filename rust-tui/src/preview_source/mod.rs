mod claude;
pub(crate) mod codex;
mod gemini;
mod session_target;
mod turns;

use crate::i18n::{self, Locale};
use crate::model::{
    AgentPanel, AgentState, AgentType, PreviewSessionOrigin, PreviewSource, SessionCacheState,
    SharedPreviewTurns,
};
use std::time::Instant;

const TMUX_CAPTURE_LINES: usize = 50;
const BUSY_REFRESH_MS: u64 = 200;
const WAITING_REFRESH_MS: u64 = 700;
const APP_IDLE_REFRESH_MS: u64 = 1200;
const LIVE_IDLE_REFRESH_MS: u64 = 2500;
const HISTORY_IDLE_REFRESH_MS: u64 = 4000;

#[derive(Clone, Debug)]
pub struct PreviewRequest {
    pub target_key: String,
    pub live_pane_id: Option<String>,
    pub agent_type: AgentType,
    pub working_dir: String,
    pub state: AgentState,
    pub transcript_path: Option<String>,
    pub cached_preview_turns: SharedPreviewTurns,
    pub session_cache_state: Option<SessionCacheState>,
    pub agent_session_id: Option<String>,
    pub session_origin: Option<PreviewSessionOrigin>,
    pub persist_resolved_session: bool,
    pub known_updated_at: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct PreviewUpdate {
    pub target_key: String,
    pub live_pane_id: Option<String>,
    pub content: String,
    pub source: PreviewSource,
    pub session_origin: Option<PreviewSessionOrigin>,
    pub session_id: Option<String>,
    pub turns: SharedPreviewTurns,
    pub transcript_path: Option<String>,
    pub session_cache_state: Option<SessionCacheState>,
    pub updated_at: Option<i64>,
}

struct SessionPreviewData {
    turns: SharedPreviewTurns,
    session_origin: PreviewSessionOrigin,
    session_id: Option<String>,
    transcript_path: Option<String>,
    cache_state: SessionCacheState,
    updated_at: Option<i64>,
}

#[derive(Clone, Copy)]
pub(super) enum SessionReadMode {
    FullBackfill,
}

#[allow(dead_code)]
pub fn preview_refresh_interval_ms(panel: &AgentPanel) -> u64 {
    match panel.state {
        AgentState::Busy => BUSY_REFRESH_MS,
        AgentState::Waiting => WAITING_REFRESH_MS,
        AgentState::Idle => LIVE_IDLE_REFRESH_MS,
    }
}

#[allow(dead_code)]
pub fn preview_refresh_interval_ms_for_state(state: &AgentState) -> u64 {
    match state {
        AgentState::Busy => BUSY_REFRESH_MS,
        AgentState::Waiting => WAITING_REFRESH_MS,
        AgentState::Idle => LIVE_IDLE_REFRESH_MS,
    }
}

pub fn preview_refresh_interval_ms_for_request(request: &PreviewRequest) -> u64 {
    match request.state {
        AgentState::Busy => BUSY_REFRESH_MS,
        AgentState::Waiting => WAITING_REFRESH_MS,
        AgentState::Idle => match request.session_origin {
            Some(PreviewSessionOrigin::App) => APP_IDLE_REFRESH_MS,
            _ if request.live_pane_id.is_some() => LIVE_IDLE_REFRESH_MS,
            _ => HISTORY_IDLE_REFRESH_MS,
        },
    }
}

pub fn load_preview(request: &PreviewRequest, mode: &str, locale: Locale) -> PreviewUpdate {
    let preferred_source = resolve_preferred_source(request, mode);
    let (
        content,
        source,
        session_origin,
        session_id,
        turns,
        transcript_path,
        session_cache_state,
        updated_at,
    ) = match preferred_source {
        PreviewSource::Tmux => (
            load_tmux_preview(request),
            PreviewSource::Tmux,
            None,
            None,
            SharedPreviewTurns::default(),
            None,
            None,
            None,
        ),
        PreviewSource::Session => match load_session_preview(request, locale) {
            Ok(data) => (
                turns::format_session_turns(&data.turns),
                PreviewSource::Session,
                Some(data.session_origin),
                data.session_id,
                data.turns,
                data.transcript_path,
                Some(data.cache_state),
                data.updated_at,
            ),
            Err(_err) if mode == "auto" => (
                load_tmux_preview(request),
                PreviewSource::Tmux,
                None,
                None,
                SharedPreviewTurns::default(),
                None,
                None,
                None,
            ),
            Err(err) => (
                err,
                PreviewSource::Session,
                None,
                None,
                SharedPreviewTurns::default(),
                None,
                None,
                None,
            ),
        },
    };

    PreviewUpdate {
        target_key: request.target_key.clone(),
        live_pane_id: request.live_pane_id.clone(),
        content,
        source,
        session_origin,
        session_id,
        turns,
        transcript_path,
        session_cache_state,
        updated_at,
    }
}

fn resolve_preferred_source(request: &PreviewRequest, mode: &str) -> PreviewSource {
    match mode {
        "tmux" => PreviewSource::Tmux,
        "session" => PreviewSource::Session,
        _ => {
            if supports_session_preview(request) {
                PreviewSource::Session
            } else {
                PreviewSource::Tmux
            }
        }
    }
}

fn supports_session_preview(request: &PreviewRequest) -> bool {
    match request.agent_type {
        AgentType::Codex => true,
        AgentType::Claude => {
            request.transcript_path.is_some()
                || request.agent_session_id.is_some()
                || !request.cached_preview_turns.is_empty()
        }
        AgentType::Gemini => true,
        _ => false,
    }
}

fn load_tmux_preview(request: &PreviewRequest) -> String {
    let Some(pane_id) = request.live_pane_id.as_deref() else {
        return String::from("No live pane available");
    };

    match crate::pty::capture_pane(pane_id, TMUX_CAPTURE_LINES) {
        Ok(content) => content,
        Err(_) => String::from("Failed to capture pane"),
    }
}

fn load_session_preview(
    request: &PreviewRequest,
    locale: Locale,
) -> Result<SessionPreviewData, String> {
    let started_at = Instant::now();
    let can_use_cached = !request.cached_preview_turns.is_empty()
        && request.session_cache_state == Some(SessionCacheState::Confirmed)
        && request.known_updated_at.is_some();

    if can_use_cached {
        return Ok(cached_session_preview(request));
    }

    let resolve_started_at = Instant::now();
    let target = session_target::resolve_session_target(request);
    let resolve_elapsed = resolve_started_at.elapsed();
    let target_updated_at = target.as_ref().and_then(|target| target.updated_at);
    let cache_stale = match (request.known_updated_at, target_updated_at) {
        (Some(known), Some(current)) => current > known,
        (None, Some(_)) => true,
        _ => false,
    };
    if !request.cached_preview_turns.is_empty()
        && request.session_cache_state == Some(SessionCacheState::Confirmed)
        && !cache_stale
    {
        return Ok(cached_session_preview(request));
    }

    if let Some(target) = target {
        let transcript_path = target.transcript_path.clone();
        let transcript_updated_at = session_target::transcript_updated_at(&transcript_path);
        let parse_started_at = Instant::now();
        let turns = match request.agent_type {
            AgentType::Codex => {
                codex::parse_transcript(&transcript_path, SessionReadMode::FullBackfill)
            }
            AgentType::Claude => {
                claude::parse_transcript(&transcript_path, SessionReadMode::FullBackfill)
            }
            AgentType::Gemini => {
                gemini::parse_transcript(&transcript_path, SessionReadMode::FullBackfill)
            }
            _ => Ok(Vec::new()),
        };
        let parse_elapsed = parse_started_at.elapsed();

        match turns {
            Ok(turns) if !turns.is_empty() => {
                let continuity_decision = if !request.cached_preview_turns.is_empty()
                    && request.session_cache_state == Some(SessionCacheState::Confirmed)
                {
                    crate::session_continuity::assess_preview_fallback(
                        &request.agent_type,
                        target
                            .session_id
                            .as_deref()
                            .or(request.agent_session_id.as_deref()),
                        Some(&transcript_path),
                        transcript_updated_at,
                        target.updated_at,
                        request.known_updated_at,
                        request.cached_preview_turns.len(),
                        turns.len(),
                    )
                } else {
                    None
                };

                if let Some(decision) = continuity_decision.as_ref() {
                    crate::session_continuity::record_preview_assessment(
                        &request.agent_type,
                        target
                            .session_id
                            .as_deref()
                            .or(request.agent_session_id.as_deref()),
                        Some(&transcript_path),
                        target.updated_at,
                        request.cached_preview_turns.len(),
                        turns.len(),
                        decision,
                    );

                    if decision.prefer_cache {
                        log_debug!(
                            "preview.session: target={} agent={} continuity={} lag_s={} prefer_cache=1",
                            request.target_key,
                            request.agent_type,
                            decision.reason,
                            decision.lag_seconds.unwrap_or_default()
                        );
                        return Ok(cached_session_preview_with_metadata(
                            request,
                            Some(target.origin),
                            target.session_id.clone(),
                            Some(transcript_path.to_string_lossy().to_string()),
                            max_i64(target.updated_at, request.known_updated_at),
                        ));
                    }
                }

                let transcript_string = transcript_path.to_string_lossy().to_string();
                if target.origin == PreviewSessionOrigin::Pane && request.persist_resolved_session {
                    if let Some(panel) =
                        session_target::persistence_panel_from_request(request, &target)
                    {
                        if let Err(err) = crate::session_cache::persist_resolved_session(
                            &panel,
                            &transcript_path,
                            &turns,
                        ) {
                            log_debug!("session_cache: persist resolved failed: {}", err);
                        }
                    }
                }
                let total_elapsed = started_at.elapsed();
                if total_elapsed.as_millis() >= 40 {
                    log_debug!(
                        "preview.session: target={} agent={} resolve_ms={} parse_ms={} turns={} total_ms={}",
                        request.target_key,
                        request.agent_type,
                        resolve_elapsed.as_millis(),
                        parse_elapsed.as_millis(),
                        turns.len(),
                        total_elapsed.as_millis()
                    );
                }
                return Ok(SessionPreviewData {
                    turns: turns.into(),
                    session_origin: target.origin,
                    session_id: target.session_id,
                    transcript_path: Some(transcript_string),
                    cache_state: SessionCacheState::Confirmed,
                    updated_at: target.updated_at,
                });
            }
            Ok(_) => {
                if started_at.elapsed().as_millis() >= 40 {
                    log_debug!(
                        "preview.session: target={} agent={} resolve_ms={} parse_ms={} turns=0 total_ms={}",
                        request.target_key,
                        request.agent_type,
                        resolve_elapsed.as_millis(),
                        parse_elapsed.as_millis(),
                        started_at.elapsed().as_millis()
                    );
                }
                if !request.cached_preview_turns.is_empty() {
                    return Ok(cached_session_preview(request));
                }
                return Err(session_unavailable_message(
                    locale,
                    i18n::t(locale, "preview.session_empty"),
                ));
            }
            Err(err) => {
                if started_at.elapsed().as_millis() >= 40 {
                    log_debug!(
                        "preview.session: target={} agent={} resolve_ms={} parse_ms={} error=1 total_ms={}",
                        request.target_key,
                        request.agent_type,
                        resolve_elapsed.as_millis(),
                        parse_elapsed.as_millis(),
                        started_at.elapsed().as_millis()
                    );
                }
                if !request.cached_preview_turns.is_empty() {
                    return Ok(cached_session_preview(request));
                }
                return Err(session_unavailable_message(
                    locale,
                    &format!(
                        "{}: {}",
                        i18n::t(locale, "preview.session_parse_failed"),
                        err
                    ),
                ));
            }
        }
    }

    if !request.cached_preview_turns.is_empty() {
        return Ok(cached_session_preview(request));
    }

    if started_at.elapsed().as_millis() >= 20 {
        log_debug!(
            "preview.session: target={} agent={} missing_target=1 resolve_ms={} total_ms={}",
            request.target_key,
            request.agent_type,
            resolve_elapsed.as_millis(),
            started_at.elapsed().as_millis()
        );
    }

    Err(session_unavailable_message(
        locale,
        i18n::t(locale, "preview.session_missing"),
    ))
}

fn cached_session_preview(request: &PreviewRequest) -> SessionPreviewData {
    cached_session_preview_with_metadata(
        request,
        request.session_origin,
        request.agent_session_id.clone(),
        request.transcript_path.clone(),
        request.known_updated_at,
    )
}

fn cached_session_preview_with_metadata(
    request: &PreviewRequest,
    session_origin: Option<PreviewSessionOrigin>,
    session_id: Option<String>,
    transcript_path: Option<String>,
    updated_at: Option<i64>,
) -> SessionPreviewData {
    SessionPreviewData {
        turns: request.cached_preview_turns.clone(),
        session_origin: session_origin.unwrap_or(PreviewSessionOrigin::Pane),
        session_id,
        transcript_path,
        cache_state: request
            .session_cache_state
            .unwrap_or(SessionCacheState::Cached),
        updated_at,
    }
}

fn max_i64(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn session_unavailable_message(locale: Locale, detail: &str) -> String {
    format!(
        "{}\n\n{}",
        i18n::t(locale, "preview.session_unavailable"),
        detail
    )
}

#[cfg(test)]
mod tests {
    use super::{
        codex, load_preview, load_session_preview, preview_refresh_interval_ms_for_request,
        session_target, turns, PreviewRequest, SessionReadMode,
    };
    use crate::i18n::Locale;
    use crate::model::{
        AgentState, AgentType, PreviewSessionOrigin, PreviewSource, PreviewTurn, SessionCacheState,
    };
    use std::path::Path;
    use std::time::Instant;

    #[test]
    fn request_refresh_interval_is_adaptive_to_state_and_origin() {
        let base = PreviewRequest {
            target_key: "demo".into(),
            live_pane_id: Some("%1".into()),
            agent_type: AgentType::Codex,
            working_dir: "/tmp/demo".into(),
            state: AgentState::Idle,
            transcript_path: None,
            cached_preview_turns: Default::default(),
            session_cache_state: None,
            agent_session_id: None,
            session_origin: None,
            persist_resolved_session: false,
            known_updated_at: None,
        };

        assert_eq!(preview_refresh_interval_ms_for_request(&base), 2500);

        let mut waiting = base.clone();
        waiting.state = AgentState::Waiting;
        assert_eq!(preview_refresh_interval_ms_for_request(&waiting), 700);

        let mut busy = base.clone();
        busy.state = AgentState::Busy;
        assert_eq!(preview_refresh_interval_ms_for_request(&busy), 200);

        let mut app_idle = base.clone();
        app_idle.live_pane_id = None;
        app_idle.session_origin = Some(PreviewSessionOrigin::App);
        assert_eq!(preview_refresh_interval_ms_for_request(&app_idle), 1200);

        let mut history_idle = base;
        history_idle.live_pane_id = None;
        history_idle.session_origin = None;
        assert_eq!(preview_refresh_interval_ms_for_request(&history_idle), 4000);
    }

    #[test]
    fn confirmed_cached_preview_returns_without_resolving_target() {
        let request = PreviewRequest {
            target_key: "codex:test".into(),
            live_pane_id: None,
            agent_type: AgentType::Codex,
            working_dir: "/tmp/demo".into(),
            state: AgentState::Idle,
            transcript_path: Some("/definitely/missing/rollout.jsonl".into()),
            cached_preview_turns: vec![PreviewTurn {
                question: "hello".into(),
                answer: Some("world".into()),
            }]
            .into(),
            session_cache_state: Some(SessionCacheState::Confirmed),
            agent_session_id: Some("019d2e9d-9010-79a2-b381-52af55b198f6".into()),
            session_origin: Some(PreviewSessionOrigin::App),
            persist_resolved_session: false,
            known_updated_at: Some(42),
        };

        let preview = load_session_preview(&request, Locale::ZhCN).unwrap();
        assert_eq!(preview.turns.len(), 1);
        assert_eq!(preview.turns[0].question, "hello");
        assert_eq!(preview.turns[0].answer.as_deref(), Some("world"));
        assert_eq!(preview.updated_at, Some(42));
    }

    #[test]
    #[ignore]
    fn bench_preview_load_breakdown_from_env() {
        let raw_paths = std::env::var("PAD_PREVIEW_BENCH_PATHS")
            .expect("set PAD_PREVIEW_BENCH_PATHS to a ';'-separated list of transcript paths");
        let iterations = std::env::var("PAD_PREVIEW_BENCH_ITERATIONS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(5);

        for raw_path in raw_paths
            .split(';')
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            let path = Path::new(raw_path);
            let metadata = std::fs::metadata(path)
                .unwrap_or_else(|err| panic!("failed to stat {}: {}", path.display(), err));
            let session_id = rollout_session_id(path);
            let target_key = session_id
                .clone()
                .unwrap_or_else(|| path.display().to_string());
            let request = PreviewRequest {
                target_key: target_key.clone(),
                live_pane_id: None,
                agent_type: AgentType::Codex,
                working_dir: String::new(),
                state: AgentState::Idle,
                transcript_path: Some(path.display().to_string()),
                cached_preview_turns: Default::default(),
                session_cache_state: None,
                agent_session_id: session_id.clone(),
                session_origin: Some(PreviewSessionOrigin::App),
                persist_resolved_session: false,
                known_updated_at: None,
            };
            let mut request_path_only = request.clone();
            request_path_only.agent_session_id = None;

            let resolve_with_sid_ms = bench_component(iterations, || {
                let _ = session_target::resolve_session_target(&request);
            });
            let resolve_path_only_ms = bench_component(iterations, || {
                let _ = session_target::resolve_session_target(&request_path_only);
            });

            let target = session_target::resolve_session_target(&request).unwrap_or_else(|| {
                panic!("failed to resolve session target for {}", path.display())
            });
            let parse_ms = bench_component(iterations, || {
                let _ =
                    codex::parse_transcript(&target.transcript_path, SessionReadMode::FullBackfill)
                        .unwrap_or_else(|err| {
                            panic!("parse failed for {}: {}", path.display(), err)
                        });
            });
            let turns =
                codex::parse_transcript(&target.transcript_path, SessionReadMode::FullBackfill)
                    .unwrap_or_else(|err| panic!("parse failed for {}: {}", path.display(), err));
            let format_ms = bench_component(iterations, || {
                let _ = turns::format_session_turns(&turns);
            });
            let formatted = turns::format_session_turns(&turns);
            let load_session_ms = bench_component(iterations, || {
                let _ = load_session_preview(&request, Locale::ZhCN).unwrap_or_else(|err| {
                    panic!(
                        "load_session_preview failed for {}: {}",
                        path.display(),
                        err
                    )
                });
            });
            let load_preview_ms = bench_component(iterations, || {
                let update = load_preview(&request, "session", Locale::ZhCN);
                assert_eq!(update.source, PreviewSource::Session);
            });
            let cached_request = PreviewRequest {
                cached_preview_turns: turns.clone().into(),
                session_cache_state: Some(SessionCacheState::Confirmed),
                known_updated_at: target.updated_at,
                ..request.clone()
            };
            let cached_load_preview_ms = bench_component(iterations, || {
                let update = load_preview(&cached_request, "session", Locale::ZhCN);
                assert_eq!(update.source, PreviewSource::Session);
            });

            print_bench_summary(
                &target_key,
                metadata.len(),
                turns.len(),
                formatted.len(),
                iterations,
                "resolve_target_with_sid",
                &resolve_with_sid_ms,
            );
            print_bench_summary(
                &target_key,
                metadata.len(),
                turns.len(),
                formatted.len(),
                iterations,
                "resolve_target_path_only",
                &resolve_path_only_ms,
            );
            print_bench_summary(
                &target_key,
                metadata.len(),
                turns.len(),
                formatted.len(),
                iterations,
                "parse_transcript",
                &parse_ms,
            );
            print_bench_summary(
                &target_key,
                metadata.len(),
                turns.len(),
                formatted.len(),
                iterations,
                "format_session_turns",
                &format_ms,
            );
            print_bench_summary(
                &target_key,
                metadata.len(),
                turns.len(),
                formatted.len(),
                iterations,
                "load_session_preview",
                &load_session_ms,
            );
            print_bench_summary(
                &target_key,
                metadata.len(),
                turns.len(),
                formatted.len(),
                iterations,
                "load_preview_total",
                &load_preview_ms,
            );
            print_bench_summary(
                &target_key,
                metadata.len(),
                turns.len(),
                formatted.len(),
                iterations,
                "load_preview_cached",
                &cached_load_preview_ms,
            );
        }
    }

    fn bench_component<F>(iterations: usize, mut f: F) -> Vec<f64>
    where
        F: FnMut(),
    {
        let mut out = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let started_at = Instant::now();
            f();
            out.push(started_at.elapsed().as_secs_f64() * 1000.0);
        }
        out
    }

    fn print_bench_summary(
        session: &str,
        bytes: u64,
        turns: usize,
        formatted_bytes: usize,
        iterations: usize,
        component: &str,
        runs_ms: &[f64],
    ) {
        let total_ms: f64 = runs_ms.iter().sum();
        let avg_ms = total_ms / runs_ms.len() as f64;
        let min_ms = runs_ms.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_ms = runs_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        println!(
            "bench.preview_load session={} component={} bytes={} turns={} formatted_bytes={} iterations={} runs_ms={:?} avg_ms={:.3} min_ms={:.3} max_ms={:.3}",
            session,
            component,
            bytes,
            turns,
            formatted_bytes,
            iterations,
            runs_ms,
            avg_ms,
            min_ms,
            max_ms
        );
    }

    fn rollout_session_id(path: &Path) -> Option<String> {
        let file_name = path.file_name()?.to_string_lossy();
        let stem = file_name.strip_suffix(".jsonl")?;
        let stem = stem.strip_prefix("rollout-")?;
        if stem.len() < 36 {
            return None;
        }
        let candidate = &stem[stem.len().saturating_sub(36)..];
        for (idx, byte) in candidate.bytes().enumerate() {
            if matches!(idx, 8 | 13 | 18 | 23) {
                if byte != b'-' {
                    return None;
                }
            } else if !(byte as char).is_ascii_hexdigit() {
                return None;
            }
        }
        Some(candidate.to_string())
    }
}
