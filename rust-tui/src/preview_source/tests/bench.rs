use super::super::{
    codex, load_preview, session_loader::load_session_preview, session_target, turns,
    PreviewRequest, SessionReadMode,
};
use crate::i18n::Locale;
use crate::model::{AgentState, AgentType, PreviewSessionOrigin, PreviewSource, SessionCacheState};
use std::path::Path;
use std::time::Instant;

mod env {
    pub(super) fn bench_paths_from_env() -> Vec<String> {
        std::env::var("PAD_PREVIEW_BENCH_PATHS")
            .expect("set PAD_PREVIEW_BENCH_PATHS to a ';'-separated list of transcript paths")
            .split(';')
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(str::to_string)
            .collect()
    }

    pub(super) fn bench_iterations_from_env() -> usize {
        std::env::var("PAD_PREVIEW_BENCH_ITERATIONS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(5)
    }
}

mod report {
    pub(super) fn print_bench_summary(
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
}

mod rollout {
    use super::*;
    pub(super) fn rollout_session_id(path: &Path) -> Option<String> {
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

    #[test]
    fn rollout_session_id_extracts_uuid_suffix() {
        let path = Path::new("/tmp/rollout-extra-123e4567-e89b-12d3-a456-426614174000.jsonl");
        assert_eq!(
            rollout_session_id(path).as_deref(),
            Some("123e4567-e89b-12d3-a456-426614174000")
        );
    }

    #[test]
    fn rollout_session_id_rejects_non_rollout_or_invalid_uuid() {
        assert!(rollout_session_id(Path::new("/tmp/session.jsonl")).is_none());
        assert!(rollout_session_id(Path::new("/tmp/rollout-not-a-uuid.jsonl")).is_none());
    }
}

mod runner {
    use super::report::print_bench_summary;
    use super::rollout::rollout_session_id;
    use super::*;

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
}

#[test]
#[ignore]
fn bench_preview_load_breakdown_from_env() {
    let paths = env::bench_paths_from_env();
    let iterations = env::bench_iterations_from_env();

    for raw_path in paths {
        runner::bench_preview_load_breakdown(Path::new(&raw_path), iterations);
    }
}
