use super::support::with_temp_home;
use super::*;

#[test]
fn ensure_runtime_layout_creates_codex_jailbreak_prompt_file() {
    with_temp_home("runtime-layout", |_home| {
        ensure_runtime_layout().expect("ensure runtime layout");
        assert!(prompts_dir().is_dir());
        assert!(codex_jailbreak_prompt_file_path().is_file());
        assert_eq!(
            std::fs::read_to_string(codex_jailbreak_prompt_file_path()).expect("read prompt file"),
            DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE
        );
        assert!(codex_index_prompt_file_path().is_file());
        assert_eq!(
            std::fs::read_to_string(codex_index_prompt_file_path()).expect("read prompt file"),
            DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE
        );
        assert!(pad_codex_wrapper_path().is_file());
    });
}

#[test]
fn ensure_runtime_layout_installs_executable_pad_codex_wrapper() {
    with_temp_home("runtime-layout-wrapper", |_home| {
        ensure_runtime_layout().expect("ensure runtime layout");

        let wrapper = pad_codex_wrapper_path();
        let content = fs::read_to_string(&wrapper).expect("read wrapper");
        assert!(content.contains(".pad/codex-home/auth.json"));
        assert!(content.contains("exec \"$CODEX_BIN\" --profile pad \"$@\""));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&wrapper)
                .expect("wrapper metadata")
                .permissions()
                .mode();
            assert_ne!(mode & 0o111, 0);
        }
    });
}

#[test]
fn ensure_runtime_layout_enables_codex_hooks_in_pad_profile_only() {
    with_temp_home("runtime-layout-codex-profile-hooks", |home| {
        let canonical_config = home.join(".codex").join("config.toml");
        fs::create_dir_all(canonical_config.parent().expect("canonical config parent"))
            .expect("create canonical config parent");
        fs::write(&canonical_config, "model = \"gpt-5\"\n").expect("seed canonical config");

        ensure_runtime_layout().expect("ensure runtime layout");

        let canonical = fs::read_to_string(&canonical_config).expect("read canonical config");
        let profile = fs::read_to_string(pad_codex_config_path()).expect("read pad profile");

        assert_eq!(canonical, "model = \"gpt-5\"\n");
        assert!(profile.contains("model = \"gpt-5\""));
        assert!(profile.contains("[features]"));
        assert!(profile.contains("codex_hooks = true") || profile.contains("hooks = true"));
        assert_eq!(
            pad_codex_hooks_path(),
            pad_codex_home_dir().join("hooks.json")
        );
        assert!(pad_codex_hooks_path().exists());
    });
}
