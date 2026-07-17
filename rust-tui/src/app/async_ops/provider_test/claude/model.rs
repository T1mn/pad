pub(super) fn claude_probe_models(configured_model: &str) -> Vec<String> {
    if let Some(model) = std::env::var("PAD_CLAUDE_PROVIDER_TEST_MODEL")
        .ok()
        .filter(|model| !model.trim().is_empty())
    {
        return expanded_model_candidates(&model);
    }

    let mut models = Vec::new();
    let configured = configured_model.trim();
    if !configured.is_empty() {
        push_expanded_model_candidates(&mut models, configured);
        return models;
    }

    if let Some(model) = claude_settings_model() {
        push_expanded_model_candidates(&mut models, &model);
    }

    push_expanded_model_candidates(&mut models, "claude-haiku-4-5-20251001");
    push_expanded_model_candidates(&mut models, "claude-opus-4-8");
    push_expanded_model_candidates(&mut models, "claude-sonnet-4-5");
    models
}

fn claude_settings_model() -> Option<String> {
    let path = crate::paths::claude_settings_path();
    let content = std::fs::read_to_string(path).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    value
        .pointer("/env/ANTHROPIC_MODEL")
        .and_then(|model| model.as_str())
        .or_else(|| value.get("model").and_then(|model| model.as_str()))
        .map(|model| model.trim().to_string())
        .filter(|model| !model.is_empty())
}

fn expanded_model_candidates(model: &str) -> Vec<String> {
    let mut out = Vec::new();
    push_expanded_model_candidates(&mut out, model);
    out
}

fn push_expanded_model_candidates(out: &mut Vec<String>, model: &str) {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return;
    }

    let lower = trimmed.to_ascii_lowercase();
    match lower.as_str() {
        "opus" | "opus[1m]" => push_unique(out, "claude-opus-4-8"),
        "sonnet" | "sonnet[1m]" => push_unique(out, "claude-sonnet-4-5"),
        "haiku" => push_unique(out, "claude-haiku-4-5-20251001"),
        _ => {
            if let Some(stripped) = lower.strip_suffix("[1m]") {
                push_unique(out, stripped);
            }
        }
    }
    push_unique(out, trimmed);
}

fn push_unique(out: &mut Vec<String>, model: &str) {
    let model = model.trim();
    if !model.is_empty() && !out.iter().any(|existing| existing == model) {
        out.push(model.to_string());
    }
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
