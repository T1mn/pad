use super::*;

#[derive(Clone, Debug)]
pub struct ProviderConfig {
    pub label: String,
    pub base_url: String,
    pub api_key: String,
    pub env_key: String,
    pub wire_api: String,
    pub provider_key: String,
    pub npm_package: String,
    pub disable_thinking: bool,
    pub models: Vec<OpenCodeModelConfig>,
    pub test_status: Option<bool>,
    pub test_http_status: Option<u16>,
    pub test_latency_ms: Option<u64>,
    pub test_result: Option<String>,
}

impl ProviderConfig {
    pub fn codex_provider_name(&self) -> String {
        normalize_provider_name(&self.label)
    }

    pub fn codex_auth_token(&self) -> Option<String> {
        (!self.api_key.trim().is_empty()).then(|| self.api_key.clone())
    }

    pub fn codex_wire_api(&self) -> &str {
        if self.wire_api.trim().is_empty() {
            "responses"
        } else {
            self.wire_api.as_str()
        }
    }

    pub fn codex_base_url(&self) -> String {
        codex_preferred_api_base_url(&self.base_url)
    }

    pub fn opencode_provider_key(&self) -> &str {
        if self.provider_key.trim().is_empty() {
            "provider"
        } else {
            self.provider_key.as_str()
        }
    }

    pub fn opencode_npm_package(&self) -> &str {
        if self.npm_package.trim().is_empty() {
            "@ai-sdk/openai-compatible"
        } else {
            self.npm_package.as_str()
        }
    }
}

fn normalize_provider_name(raw: &str) -> String {
    normalize_with_separator(raw, '_', "relay")
}

pub fn normalize_provider_key(raw: &str) -> String {
    normalize_with_separator(raw, '-', "provider")
}

fn normalize_with_separator(raw: &str, separator: char, fallback: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_sep = false;

    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            last_was_sep = false;
        } else if !normalized.is_empty() && !last_was_sep {
            normalized.push(separator);
            last_was_sep = true;
        }
    }

    let normalized = normalized.trim_matches(separator).to_string();
    if normalized.is_empty() {
        fallback.to_string()
    } else {
        normalized
    }
}

pub(crate) fn codex_api_base_candidates(raw: &str) -> Vec<String> {
    let trimmed = trim_base_url(raw);
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut candidates = vec![trimmed.clone()];
    let Ok(mut parsed) = Url::parse(&trimmed) else {
        return candidates;
    };

    let path = parsed.path().trim_end_matches('/');
    if path.is_empty() || path == "/" {
        parsed.set_path("/v1");
        push_unique_url(&mut candidates, trim_base_url(parsed.as_str()));
    } else if path == "/v1" {
        parsed.set_path("");
        push_unique_url(&mut candidates, trim_base_url(parsed.as_str()));
    }

    candidates
}

pub(crate) fn codex_preferred_api_base_url(raw: &str) -> String {
    let candidates = codex_api_base_candidates(raw);
    if candidates.is_empty() {
        return String::new();
    }

    let trimmed = trim_base_url(raw);
    if let Ok(parsed) = Url::parse(&trimmed) {
        let path = parsed.path().trim_end_matches('/');
        if path.is_empty() || path == "/" {
            if let Some(candidate) = candidates
                .iter()
                .find(|candidate| candidate.ends_with("/v1"))
            {
                return candidate.clone();
            }
        }
    }

    candidates[0].clone()
}

fn trim_base_url(raw: &str) -> String {
    raw.trim().trim_end_matches('/').to_string()
}

fn push_unique_url(out: &mut Vec<String>, value: String) {
    if !value.is_empty() && !out.iter().any(|existing| existing == &value) {
        out.push(value);
    }
}
