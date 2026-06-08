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
