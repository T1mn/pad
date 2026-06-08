use super::resolve_session_provider_name;
use crate::model::AgentType;
use std::fs;

fn temp_rollout_path(name: &str) -> std::path::PathBuf {
    crate::test_support::temp_path("pad-sidebar-provider", name).with_extension("jsonl")
}

#[test]
fn resolve_session_provider_name_reads_codex_session_meta() {
    let path = temp_rollout_path("codex");
    fs::write(
        &path,
        concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"model_provider\":\"relay_a\"}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\"}}\n"
        ),
    )
    .expect("write rollout");

    let resolved = resolve_session_provider_name(&AgentType::Codex, Some(&path));
    assert_eq!(resolved.as_deref(), Some("relay_a"));

    let _ = fs::remove_file(path);
}
