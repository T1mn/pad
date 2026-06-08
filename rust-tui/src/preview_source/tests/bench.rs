use super::super::{
    codex, load_preview, session_loader::load_session_preview, session_target, turns,
    PreviewRequest, SessionReadMode,
};
use crate::i18n::Locale;
use crate::model::{AgentState, AgentType, PreviewSessionOrigin, PreviewSource, SessionCacheState};
use std::path::Path;
use std::time::Instant;

mod env {
    include!("bench/env.rs");
}

mod report {
    include!("bench/report.rs");
}

mod rollout {
    use super::*;
    include!("bench/rollout.rs");
}

mod runner {
    use super::*;
    include!("bench/runner.rs");
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
