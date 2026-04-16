use crate::model::AgentType;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub(crate) fn resolve_session_provider_name(
    agent_type: &AgentType,
    transcript_path: Option<&Path>,
) -> Option<String> {
    match agent_type {
        AgentType::Codex => transcript_path.and_then(read_codex_session_provider_name),
        _ => None,
    }
}

fn read_codex_session_provider_name(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines().take(8) {
        let line = line.ok()?;
        let value = serde_json::from_str::<Value>(line.trim()).ok()?;
        if value.get("type").and_then(Value::as_str) != Some("session_meta") {
            continue;
        }
        let provider_name = value
            .get("payload")
            .and_then(|payload| payload.get("model_provider"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())?;
        return Some(provider_name.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::resolve_session_provider_name;
    use crate::model::AgentType;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_rollout_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("pad-sidebar-provider-{name}-{stamp}.jsonl"))
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
}
