use super::codex::should_restore_native_codex_config;
use super::common::{
    claude_permission_state_path, codex_permission_state_path, opencode_managed_state_path,
    parse_env_file, serialize_env_file,
};
use super::{
    apply_relay_configs, apply_runtime_configs, apply_runtime_overlays, read_codex_relay_import,
    write_codex_relay_export,
};
use crate::paths::{
    codex_index_prompt_file_path, codex_jailbreak_prompt_file_path,
    codex_selected_prompt_file_path, DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE,
    DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE,
};
use crate::theme::{AgentConfig, AgentPermissionsConfig, CodexConfig, ProviderConfig};
use std::path::Path;

fn sample_provider(base_url: &str, api_key: &str) -> ProviderConfig {
    ProviderConfig {
        label: "Relay A".into(),
        base_url: base_url.into(),
        api_key: api_key.into(),
        env_key: String::new(),
        wire_api: "responses".into(),
        provider_key: "relay-a".into(),
        npm_package: "@ai-sdk/openai-compatible".into(),
        disable_thinking: false,
        models: Vec::new(),
        test_status: None,
        test_http_status: None,
        test_latency_ms: None,
        test_result: None,
    }
}

fn sample_permissions() -> AgentPermissionsConfig {
    AgentPermissionsConfig {
        codex_auto_full_access: true,
        claude_auto_full_access: true,
    }
}

fn sample_codex_config() -> CodexConfig {
    CodexConfig {
        goals: false,
        ..CodexConfig::default()
    }
}

fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
    crate::test_support::with_temp_home("pad-relay", name, f)
}

pub(crate) fn serialize_env_file_keeps_sorted_lines_and_trailing_newline() {
    let mut env = std::collections::BTreeMap::new();
    env.insert("ZED".to_string(), "last".to_string());
    env.insert("ALPHA".to_string(), "first".to_string());

    assert_eq!(serialize_env_file(&env), "ALPHA=first\nZED=last\n");
}

pub(crate) mod provider_configs {
    use super::*;
    include!("tests/provider_configs.rs");
}

pub(crate) mod runtime_overlays {
    use super::*;
    include!("tests/runtime_overlays.rs");
}

pub(crate) fn pi_runtime_adapter_tests() {
    crate::pi_runtime::jsonl::tests::split_chunks_preserve_jsonl_messages();
    crate::pi_runtime::jsonl::tests::reject_crlf_and_oversized_frames();
    crate::pi_runtime::jsonl::tests::command_and_message_validation_use_type_discriminator();
    crate::pi_runtime::events::tests::settled_is_the_only_completion_boundary();
    crate::pi_runtime::events::tests::stale_generations_and_sequences_are_ignored();
    crate::pi_runtime::events::tests::approval_and_tool_events_update_runtime_status();
    crate::pi_runtime::approval::tests::full_access_auto_answers_confirm_but_not_unknown_ui();
    crate::pi_runtime::approval::tests::unattended_uses_input_default_and_select_default();
    crate::pi_runtime::approval::tests::select_response_uses_the_option_value_not_its_index();
    crate::pi_runtime::approval::tests::protected_paths_never_get_auto_answers();
    crate::pi_runtime::approval::tests::tool_operation_and_target_are_conservative();
    crate::pi_runtime::tests::pi_agent_detection_accepts_binary_paths();
    crate::pi_runtime::tests::pi_command_defaults_without_rewriting_explicit_commands();
    crate::pi_runtime::tests::pi_rpc_command_isolated_from_codex_and_pi_homes();
    crate::pi_runtime::tests::profile_pi_roots_are_isolated_and_safe_for_empty_records();
    crate::pi_runtime::tests::profile_storage_segments_are_injective_for_unsafe_ids();
}
