#[cfg(unix)]
fn deepseek_agent(secret: &str) -> AgentConfig {
    AgentConfig {
        name: "deepseek".into(),
        cmd: "claude".into(),
        providers: vec![sample_provider("https://relay.example", secret)],
        active_provider: Some(0),
        default_model: String::new(),
        small_model: String::new(),
    }
}

#[cfg(unix)]
#[test]
fn deepseek_launcher_keeps_secret_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    with_temp_home("deepseek-launcher-permissions", |home| {
        let secret = "sk-deepseek-private-test";
        apply_relay_configs(&[deepseek_agent(secret)]);

        let launcher = home.join(".pad").join("deepseek-cc");
        let content = std::fs::read_to_string(&launcher).expect("read DeepSeek launcher");
        let mode = std::fs::metadata(&launcher)
            .expect("stat DeepSeek launcher")
            .permissions()
            .mode()
            & 0o777;

        assert!(content.contains(secret), "launcher must contain the test secret");
        assert_eq!(mode, 0o700, "launcher must be owner read/write/execute");
    });
}

#[cfg(unix)]
#[test]
fn deepseek_launcher_atomically_replaces_existing_broad_file() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    with_temp_home("deepseek-launcher-replace", |home| {
        let pad_dir = home.join(".pad");
        let launcher = pad_dir.join("deepseek-cc");
        std::fs::create_dir_all(&pad_dir).expect("create pad dir");
        std::fs::write(&launcher, "old secret").expect("seed launcher");
        std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755))
            .expect("seed broad launcher mode");
        let old_inode = std::fs::metadata(&launcher).expect("stat old launcher").ino();

        apply_relay_configs(&[deepseek_agent("new secret")]);

        let metadata = std::fs::metadata(&launcher).expect("stat new launcher");
        let content = std::fs::read_to_string(&launcher).expect("read new launcher");
        assert!(content.contains("new secret"));
        assert!(!content.contains("old secret"));
        assert_ne!(metadata.ino(), old_inode, "launcher must be replaced by rename");
        assert_eq!(metadata.permissions().mode() & 0o777, 0o700);
    });
}

#[cfg(unix)]
#[test]
fn deepseek_launcher_failure_preserves_existing_file() {
    use std::os::unix::fs::PermissionsExt;

    with_temp_home("deepseek-launcher-failure", |home| {
        let pad_dir = home.join(".pad");
        let launcher = pad_dir.join("deepseek-cc");
        std::fs::create_dir_all(&pad_dir).expect("create pad dir");
        std::fs::write(&launcher, "old secret").expect("seed launcher");
        std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o700))
            .expect("seed launcher mode");
        std::fs::set_permissions(&pad_dir, std::fs::Permissions::from_mode(0o555))
            .expect("make launcher dir read-only");

        apply_relay_configs(&[deepseek_agent("new secret")]);

        std::fs::set_permissions(&pad_dir, std::fs::Permissions::from_mode(0o755))
            .expect("restore launcher dir permissions");
        assert_eq!(
            std::fs::read_to_string(&launcher).expect("read preserved launcher"),
            "old secret"
        );
        let temp_files = std::fs::read_dir(&pad_dir)
            .expect("read launcher dir")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".pad-tmp"))
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        assert!(temp_files.is_empty(), "leftover temp files: {temp_files:?}");
    });
}
