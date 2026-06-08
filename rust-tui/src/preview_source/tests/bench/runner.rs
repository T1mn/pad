use super::report::print_bench_summary;
use super::rollout::rollout_session_id;

pub(super) fn bench_preview_load_breakdown(path: &Path, iterations: usize) {
    let metadata = std::fs::metadata(path)
        .unwrap_or_else(|err| panic!("failed to stat {}: {}", path.display(), err));
    let session_id = rollout_session_id(path);
    let target_key = session_id
        .clone()
        .unwrap_or_else(|| path.display().to_string());
    let request = preview_request(path, target_key.clone(), session_id.clone());
    let mut request_path_only = request.clone();
    request_path_only.agent_session_id = None;

    let resolve_with_sid_ms = bench_component(iterations, || {
        let _ = session_target::resolve_session_target(&request);
    });
    let resolve_path_only_ms = bench_component(iterations, || {
        let _ = session_target::resolve_session_target(&request_path_only);
    });

    let target = session_target::resolve_session_target(&request)
        .unwrap_or_else(|| panic!("failed to resolve session target for {}", path.display()));
    let parse_ms = bench_component(iterations, || {
        let _ = codex::parse_transcript(&target.transcript_path, SessionReadMode::FullBackfill)
            .unwrap_or_else(|err| panic!("parse failed for {}: {}", path.display(), err));
    });
    let turns = codex::parse_transcript(&target.transcript_path, SessionReadMode::FullBackfill)
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

    print_bench_summaries(
        &target_key,
        metadata.len(),
        turns.len(),
        formatted.len(),
        iterations,
        BenchBreakdown {
            resolve_with_sid_ms,
            resolve_path_only_ms,
            parse_ms,
            format_ms,
            load_session_ms,
            load_preview_ms,
            cached_load_preview_ms,
        },
    );
}

fn preview_request(
    path: &Path,
    target_key: String,
    session_id: Option<String>,
) -> PreviewRequest {
    PreviewRequest {
        target_key,
        live_pane_id: None,
        agent_type: AgentType::Codex,
        working_dir: String::new(),
        state: AgentState::Idle,
        transcript_path: Some(path.display().to_string()),
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        agent_session_id: session_id,
        session_origin: Some(PreviewSessionOrigin::App),
        persist_resolved_session: false,
        known_updated_at: None,
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

struct BenchBreakdown {
    resolve_with_sid_ms: Vec<f64>,
    resolve_path_only_ms: Vec<f64>,
    parse_ms: Vec<f64>,
    format_ms: Vec<f64>,
    load_session_ms: Vec<f64>,
    load_preview_ms: Vec<f64>,
    cached_load_preview_ms: Vec<f64>,
}

fn print_bench_summaries(
    target_key: &str,
    bytes: u64,
    turns: usize,
    formatted_bytes: usize,
    iterations: usize,
    breakdown: BenchBreakdown,
) {
    for (component, runs_ms) in [
        ("resolve_target_with_sid", &breakdown.resolve_with_sid_ms),
        ("resolve_target_path_only", &breakdown.resolve_path_only_ms),
        ("parse_transcript", &breakdown.parse_ms),
        ("format_session_turns", &breakdown.format_ms),
        ("load_session_preview", &breakdown.load_session_ms),
        ("load_preview_total", &breakdown.load_preview_ms),
        ("load_preview_cached", &breakdown.cached_load_preview_ms),
    ] {
        print_bench_summary(
            target_key,
            bytes,
            turns,
            formatted_bytes,
            iterations,
            component,
            runs_ms,
        );
    }
}
