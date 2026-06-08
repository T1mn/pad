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
