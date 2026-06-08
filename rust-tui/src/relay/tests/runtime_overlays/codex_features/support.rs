pub(super) fn seed_pad_codex_config(contents: &str) -> std::path::PathBuf {
    let config_path = crate::paths::pad_codex_config_path();
    std::fs::create_dir_all(config_path.parent().expect("codex config parent"))
        .expect("create codex config parent");
    std::fs::write(&config_path, contents).expect("seed codex config");
    config_path
}

pub(super) fn codex_agent_without_relay_provider() -> AgentConfig {
    AgentConfig {
        name: "codex".into(),
        cmd: "codex".into(),
        providers: Vec::new(),
        active_provider: None,
        default_model: String::new(),
        small_model: String::new(),
    }
}
