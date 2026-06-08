use super::*;

#[test]
fn claude_bridge_template_stays_minimal_and_forwards_turn_id() {
    let template = claude_hook_bridge_template();
    assert!(template.contains(&format!("# pad-bridge-version: {}", CLAUDE_BRIDGE_VERSION)));
    assert!(template.contains("\"turn_id\": payload.get(\"turn_id\")"));
    assert!(!template.contains("def silence_stdio():"));
    assert!(!template.contains("def load_payload():"));
    assert!(!template.contains("stderr=subprocess.DEVNULL"));
}

#[test]
fn codex_bridge_template_keeps_required_stdin_and_turn_id_handling() {
    let template = codex_hook_bridge_template();
    assert!(template.contains(&format!("# pad-bridge-version: {}", CODEX_BRIDGE_VERSION)));
    assert!(template.contains("\"turn_id\": payload.get(\"turn_id\")"));
    assert!(template.contains("def silence_stdio():"));
    assert!(template.contains("def load_payload():"));
    assert!(template.contains("stderr=subprocess.DEVNULL"));
    assert!(template.contains("payload.get(\"hook_event_name\") or hook_type"));
    assert!(template.contains("def pad_codex_hooks_enabled():"));
    assert!(template.contains("PAD_CODEX_HOOKS"));
    assert!(template.contains("__internal\", \"codex-turn-diff\", \"hook\""));
    assert!(template.contains("record_codex_turn_diff(message)"));
}

#[test]
fn codex_hooks_feature_key_switches_at_0130() {
    assert_eq!(
        codex_hooks_feature_key_for_version(Some("0.129.9")),
        "codex_hooks"
    );
    assert_eq!(
        codex_hooks_feature_key_for_version(Some("0.130.0")),
        "hooks"
    );
    assert_eq!(
        codex_hooks_feature_key_for_version(Some("codex 0.130.0")),
        "codex_hooks"
    );
    assert_eq!(codex_hooks_feature_key_for_version(None), "codex_hooks");
}

#[test]
fn parse_codex_cli_version_accepts_plain_and_prefixed_versions() {
    assert_eq!(parse_codex_cli_version("0.130.0"), Some((0, 130, 0)));
    assert_eq!(parse_codex_cli_version("v0.130.1"), Some((0, 130, 1)));
    assert_eq!(parse_codex_cli_version("0.130.0-beta"), Some((0, 130, 0)));
    assert_eq!(parse_codex_cli_version("codex 0.130.0"), None);
}

#[test]
fn set_toml_bool_in_section_writes_new_hooks_key() {
    let updated = set_toml_bool_in_section(
        "[features]\ncodex_hooks = true\n",
        "features",
        "hooks",
        true,
    );

    assert!(updated.contains("[features]\n"));
    assert!(updated.contains("codex_hooks = true\n"));
    assert!(updated.contains("hooks = true\n"));
}

#[test]
fn remove_toml_key_in_section_removes_legacy_codex_hooks_key() {
    let updated = remove_toml_key_in_section(
        "[features]\ncodex_hooks = true\nhooks = true\n",
        "features",
        "codex_hooks",
    );

    assert!(updated.contains("[features]\n"));
    assert!(updated.contains("hooks = true\n"));
    assert!(!updated.contains("codex_hooks = true\n"));
}
