use super::support::with_temp_home;
use super::*;

#[test]
fn write_codex_selected_prompt_file_combines_selected_candidates() {
    with_temp_home("selected-prompt-combine", |_home| {
        fs::create_dir_all(prompts_dir()).expect("create prompt dir");

        let selected = write_codex_selected_prompt_file(true, true).expect("write selected prompt");

        let selected_path = codex_selected_prompt_file_path();
        assert_eq!(selected.as_deref(), Some(selected_path.as_path()));
        let content = fs::read_to_string(codex_selected_prompt_file_path()).expect("read combined");
        assert!(content.contains(DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE));
        assert!(content.contains(DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE));
    });
}

#[test]
fn write_codex_selected_prompt_file_returns_single_candidate_directly() {
    with_temp_home("selected-prompt-single", |_home| {
        fs::create_dir_all(prompts_dir()).expect("create prompt dir");

        let selected =
            write_codex_selected_prompt_file(false, true).expect("write selected prompt");

        let index_path = codex_index_prompt_file_path();
        assert_eq!(selected.as_deref(), Some(index_path.as_path()));
        assert!(!codex_selected_prompt_file_path().exists());
    });
}

#[test]
fn ensure_runtime_layout_reseeds_empty_codex_jailbreak_prompt_file() {
    with_temp_home("runtime-layout-empty-prompt", |_home| {
        fs::create_dir_all(prompts_dir()).expect("create prompt dir");
        fs::write(codex_jailbreak_prompt_file_path(), "\n\n").expect("seed empty prompt file");

        ensure_runtime_layout().expect("ensure runtime layout");

        assert_eq!(
            std::fs::read_to_string(codex_jailbreak_prompt_file_path()).expect("read prompt file"),
            DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE
        );
    });
}

#[test]
fn ensure_runtime_layout_tracks_current_codex_jailbreak_prompt_version() {
    with_temp_home("runtime-layout-codex-prompt-version", |_home| {
        fs::create_dir_all(prompts_dir()).expect("create prompt dir");
        fs::write(
            codex_jailbreak_prompt_file_path(),
            DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE,
        )
        .expect("seed prompt file");

        ensure_runtime_layout().expect("ensure runtime layout");

        let state = read_managed_prompt_state(&codex_jailbreak_prompt_state_path())
            .expect("read prompt state")
            .expect("managed prompt state");
        assert_eq!(state.version, CODEX_JAILBREAK_PROMPT_VERSION);
        assert_eq!(
            state.content_md5,
            prompt_md5(DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE)
        );
    });
}

#[test]
fn ensure_runtime_layout_refreshes_outdated_managed_codex_jailbreak_prompt() {
    with_temp_home("runtime-layout-refresh-managed-prompt", |_home| {
        let old_prompt = "legacy managed prompt";
        fs::create_dir_all(prompts_dir()).expect("create prompt dir");
        fs::write(codex_jailbreak_prompt_file_path(), old_prompt).expect("seed old prompt");
        write_managed_prompt_state(
            &codex_jailbreak_prompt_state_path(),
            &ManagedPromptState {
                version: "codex-jailbreak-prompt-2026-04-20.1".into(),
                content_md5: prompt_md5(old_prompt),
            },
        )
        .expect("seed prompt state");

        ensure_runtime_layout().expect("ensure runtime layout");

        assert_eq!(
            fs::read_to_string(codex_jailbreak_prompt_file_path()).expect("read prompt file"),
            DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE
        );
        let state = read_managed_prompt_state(&codex_jailbreak_prompt_state_path())
            .expect("read prompt state")
            .expect("managed prompt state");
        assert_eq!(state.version, CODEX_JAILBREAK_PROMPT_VERSION);
        assert_eq!(
            state.content_md5,
            prompt_md5(DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE)
        );
    });
}

#[test]
fn ensure_runtime_layout_preserves_custom_codex_jailbreak_prompt_edits() {
    with_temp_home("runtime-layout-preserve-custom-prompt", |_home| {
        let custom_prompt = "custom operator prompt";
        fs::create_dir_all(prompts_dir()).expect("create prompt dir");
        fs::write(codex_jailbreak_prompt_file_path(), custom_prompt).expect("seed custom prompt");
        write_managed_prompt_state(
            &codex_jailbreak_prompt_state_path(),
            &ManagedPromptState {
                version: "codex-jailbreak-prompt-2026-04-20.1".into(),
                content_md5: prompt_md5("legacy managed prompt"),
            },
        )
        .expect("seed prompt state");

        ensure_runtime_layout().expect("ensure runtime layout");

        assert_eq!(
            fs::read_to_string(codex_jailbreak_prompt_file_path()).expect("read prompt file"),
            custom_prompt
        );
    });
}

#[test]
fn ensure_runtime_layout_migrates_custom_legacy_codex_prompt_to_jailbreak_name() {
    with_temp_home("runtime-layout-migrate-legacy-prompt", |_home| {
        let custom_prompt = "legacy custom jailbreak prompt";
        fs::create_dir_all(prompts_dir()).expect("create prompt dir");
        fs::write(legacy_codex_prompt_file_path(), custom_prompt).expect("seed legacy prompt");

        ensure_runtime_layout().expect("ensure runtime layout");

        assert_eq!(
            fs::read_to_string(codex_jailbreak_prompt_file_path()).expect("read prompt file"),
            custom_prompt
        );
    });
}
