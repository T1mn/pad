use super::{pad_codex_wrapper_template, WRAPPER_VERSION};

#[test]
fn wrapper_template_reads_pad_auth_and_forces_pad_profile() {
    let template = pad_codex_wrapper_template();

    assert!(template.contains(&format!("# pad-wrapper-version: {WRAPPER_VERSION}")));
    assert!(template.contains(".pad/codex-home/auth.json"));
    assert!(template
        .contains("CONFIG_PATH=\"${PAD_CODEX_CONFIG_PATH:-$PAD_CODEX_HOME/pad.config.toml}\""));
    assert!(template.contains("CODEX_HOME=\"$PAD_CODEX_HOME\""));
    assert!(!template.contains("cp \"$CONFIG_PATH\""));
    assert!(template.contains("OPENAI_API_KEY"));
    assert!(template.contains("requires_openai_auth"));
    assert!(template.contains("PAD_CODEX_HOOKS=1"));
    assert!(template.contains("exec \"$CODEX_BIN\" --profile pad \"$@\""));
}
