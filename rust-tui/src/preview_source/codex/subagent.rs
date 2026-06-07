use serde_json::Value;

pub(super) fn extract_subagent_notification_summary(text: &str) -> Option<String> {
    const OPEN: &str = "<subagent_notification>";
    const CLOSE: &str = "</subagent_notification>";

    let start = text.find(OPEN)?;
    let rest = &text[start + OPEN.len()..];
    let end = rest.find(CLOSE)?;
    let json = rest[..end].trim();
    let value = serde_json::from_str::<Value>(json).ok()?;

    let agent_path = value
        .get("agent_path")
        .and_then(Value::as_str)
        .unwrap_or("subagent");
    let agent_label = agent_path.rsplit('/').next().unwrap_or(agent_path);
    let status = value.get("status").and_then(Value::as_object);
    let (status_label, detail) = if let Some(status) = status {
        if let Some(completed) = status.get("completed").and_then(Value::as_str) {
            ("completed", completed)
        } else if let Some(failed) = status.get("failed").and_then(Value::as_str) {
            ("failed", failed)
        } else if let Some(running) = status.get("running").and_then(Value::as_str) {
            ("running", running)
        } else {
            ("updated", "")
        }
    } else {
        ("updated", "")
    };

    let compact = compact_subagent_detail(detail);
    if compact.is_empty() {
        Some(format!("[subagent/{}] {}", status_label, agent_label))
    } else {
        Some(format!(
            "[subagent/{}] {}\n{}",
            status_label, agent_label, compact
        ))
    }
}

fn compact_subagent_detail(text: &str) -> String {
    let line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();
    if line.is_empty() {
        return String::new();
    }

    let compact = line.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_chars_with_ellipsis(&compact, 220)
}

fn truncate_chars_with_ellipsis(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let mut out = String::new();
    for ch in text.chars().take(max_chars.saturating_sub(1)) {
        out.push(ch);
    }
    out.push('…');
    out
}
