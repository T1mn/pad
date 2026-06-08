use super::super::*;

#[test]
fn codex_base_url_candidates_try_root_and_v1_variants() {
    assert_eq!(
        codex_api_base_candidates("https://relay.example"),
        vec![
            "https://relay.example".to_string(),
            "https://relay.example/v1".to_string()
        ]
    );
    assert_eq!(
        codex_api_base_candidates("https://relay.example/v1"),
        vec![
            "https://relay.example/v1".to_string(),
            "https://relay.example".to_string()
        ]
    );
    assert_eq!(
        codex_api_base_candidates("https://relay.example/openai/v1"),
        vec!["https://relay.example/openai/v1".to_string()]
    );
}

#[test]
fn codex_base_url_prefers_v1_for_root_inputs() {
    assert_eq!(
        provider::codex_preferred_api_base_url("https://relay.example"),
        "https://relay.example/v1"
    );
    assert_eq!(
        provider::codex_preferred_api_base_url("https://relay.example/"),
        "https://relay.example/v1"
    );
    assert_eq!(
        provider::codex_preferred_api_base_url("https://relay.example/v1"),
        "https://relay.example/v1"
    );
    assert_eq!(
        provider::codex_preferred_api_base_url("https://relay.example/openai/v1"),
        "https://relay.example/openai/v1"
    );
}
