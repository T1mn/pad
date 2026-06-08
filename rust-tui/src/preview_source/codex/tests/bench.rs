use super::super::parse_transcript;
use crate::preview_source::SessionReadMode;
use std::fs;
use std::path::Path;
use std::time::Instant;

#[test]
#[ignore]
fn bench_parse_transcripts_from_env() {
    let raw_paths = std::env::var("PAD_CODEX_BENCH_PATHS")
        .expect("set PAD_CODEX_BENCH_PATHS to a ';'-separated list of transcript paths");
    let iterations = std::env::var("PAD_CODEX_BENCH_ITERATIONS")
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
        let metadata = fs::metadata(path)
            .unwrap_or_else(|err| panic!("failed to stat {}: {}", path.display(), err));
        let mut elapsed_ms = Vec::with_capacity(iterations);
        let mut turn_count = None;

        for _ in 0..iterations {
            let started_at = Instant::now();
            let turns = parse_transcript(path, SessionReadMode::FullBackfill)
                .unwrap_or_else(|err| panic!("failed to parse {}: {}", path.display(), err));
            elapsed_ms.push(started_at.elapsed().as_secs_f64() * 1000.0);
            turn_count = Some(turns.len());
        }

        let total_ms: f64 = elapsed_ms.iter().sum();
        let avg_ms = total_ms / elapsed_ms.len() as f64;
        let min_ms = elapsed_ms.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_ms = elapsed_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        println!(
            "bench.codex_parse path={} bytes={} turns={} iterations={} runs_ms={:?} avg_ms={:.3} min_ms={:.3} max_ms={:.3}",
            path.display(),
            metadata.len(),
            turn_count.unwrap_or(0),
            iterations,
            elapsed_ms,
            avg_ms,
            min_ms,
            max_ms
        );
    }
}
