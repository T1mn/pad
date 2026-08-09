//! Generated domain suites; individual case bodies remain beside their implementation.

#[test]
fn browser_remote_tests_suite() {
    run_cases!(
        crate::browser_remote::tests::browser::open_command_strips_trailing_newline_instead_of_passing_it_on,
        crate::browser_remote::tests::browser::open_command_uses_the_trimmed_url_it_validated,
        crate::browser_remote::tests::browser::rejects_option_shaped_and_control_char_urls,
        crate::browser_remote::tests::browser::validates_safe_browser_urls,
        crate::browser_remote::tests::cli::args_format_keeps_space_separated_remote_command,
        crate::browser_remote::tests::cli::command_line_format_includes_program_and_args,
        crate::browser_remote::tests::remote::remote_command_accepts_normal_destinations,
        crate::browser_remote::tests::remote::remote_command_cd_escapes_single_quotes,
        crate::browser_remote::tests::remote::remote_command_cd_quotes_cwd,
        crate::browser_remote::tests::remote::remote_command_puts_separator_before_host,
        crate::browser_remote::tests::remote::remote_command_rejects_option_shaped_hosts,
        crate::browser_remote::tests::remote::remote_command_rejects_proxy_command_injection,
        crate::browser_remote::tests::remote::remote_command_rejects_shell_metacharacters,
        crate::browser_remote::tests::remote::validate_ssh_host_rejects_bad_ports_and_overlong_input,
        crate::browser_remote::tests::remote::validate_ssh_host_trims_surrounding_whitespace,
    );
}

#[test]
fn chat_approval_suite() {
    run_cases!(
        crate::chat::approval::answers::tests::final_answer_ignores_empty_output_text_blocks,
        crate::chat::approval::answers::tests::final_answer_joins_output_text_blocks,
    );
}

#[test]
fn chat_backend_suite() {
    run_cases!(
        crate::chat::backend::tests::panel_display_title_falls_back_to_working_dir_leaf,
        crate::chat::backend::tests::panel_display_title_uses_thread_meta_title_override,
        crate::chat::backend::tests::summarize_pane_capture_preserves_inner_blank_lines,
        crate::chat::backend::tests::summarize_pane_capture_trims_outer_blank_lines_and_keeps_tail,
    );
}

#[test]
fn chat_providers_suite() {
    run_cases!(
        crate::chat::providers::telegram::callbacks::approval::tests::approval_prompt_text_includes_metadata_and_justification,
        crate::chat::providers::telegram::commands::diag::format::tests::diag_message_includes_empty_state_metadata,
        crate::chat::providers::telegram::daemon::process::stop::tests::matching_started_at_runs_the_stop_flow,
        crate::chat::providers::telegram::daemon::process::stop::tests::plan_stop_maps_every_case,
        crate::chat::providers::telegram::daemon::process::stop::tests::recycled_pid_is_never_signalled,
        crate::chat::providers::telegram::daemon::process::stop::tests::stop_preserves_a_new_status_written_during_termination,
        crate::chat::providers::telegram::daemon::process::stop::tests::terminate_rechecks_identity_before_signalling,
        crate::chat::providers::telegram::daemon::state_io::tests::save_state_if_changed_skips_identical_body,
        crate::chat::providers::telegram::daemon::state_io::tests::serialized_state_matches_disk_format,
        crate::chat::providers::telegram::hooks::tests::completion::codex_stop_prefers_transcript_completion_over_stale_hook_payload,
        crate::chat::providers::telegram::hooks::tests::phase_gate::stop_is_ignored_while_pending_still_awaits_submit,
        crate::chat::providers::telegram::hooks::tests::turn_match::codex_stop_without_turn_id_is_ignored_when_pending_turn_exists,
        crate::chat::providers::telegram::hooks::tests::turn_match::pending_turn_must_match_stop_turn_when_both_exist,
        crate::chat::providers::telegram::pending::tests::continuity_status_line_formats_health_and_lag,
        crate::chat::providers::telegram::pending::tests::detect_pending_rollout_failure_removes_pending_and_updates_scan_offset,
        crate::chat::providers::telegram::pending::tests::detect_pending_rollout_failure_updates_last_check_when_no_error_is_found,
        crate::chat::providers::telegram::pending::tests::pending_failure_reply_includes_continuity_details,
        crate::chat::providers::telegram::pending::tests::rollout_failure_check_waits_30_seconds_and_then_5_second_backoff,
        crate::chat::providers::telegram::render::tests::truncate_chars_keeps_ellipsis_behavior,
        crate::chat::providers::telegram::render::tests::truncate_for_log_keeps_existing_marker_behavior,
        crate::chat::providers::telegram::tests::approval::approval_callback_data_round_trips_request_id_and_choice,
        crate::chat::providers::telegram::tests::approval::codex_approval_scan_tracks_open_and_resolved_requests,
        crate::chat::providers::telegram::tests::approval::scan_codex_answer_updates_ignores_commentary_until_task_complete,
        crate::chat::providers::telegram::tests::approval::scan_codex_answer_updates_ignores_old_messages_before_offset,
        crate::chat::providers::telegram::tests::approval::scan_codex_failure_updates_detects_error_after_offset,
        crate::chat::providers::telegram::tests::approval::scan_codex_failure_updates_ignores_mismatched_turn_id,
        crate::chat::providers::telegram::tests::core::agent_keyboard_uses_clickable_use_callbacks,
        crate::chat::providers::telegram::tests::core::chunk_text_splits_long_messages,
        crate::chat::providers::telegram::tests::core::slash_command_builder_preserves_optional_args,
        crate::chat::providers::telegram::tests::core::summarize_pane_capture_keeps_last_eighteen_non_edge_lines,
        crate::chat::providers::telegram::tests::core::summarize_pane_capture_trims_blank_edges_and_keeps_tail,
        crate::chat::providers::telegram::tests::core::telegram_sound_helper_records_enabled_event,
        crate::chat::providers::telegram::tests::help::help_keyboard_marks_active_page,
        crate::chat::providers::telegram::tests::help::help_page_callbacks_parse,
        crate::chat::providers::telegram::tests::help::help_page_html_escapes_target_label,
        crate::chat::providers::telegram::tests::help::help_page_html_includes_target_and_commands,
        crate::chat::providers::telegram::tests::history_restart::recent_history_message_shows_only_latest_three_turns,
        crate::chat::providers::telegram::tests::history_restart::recent_history_turns_prefers_latest_three_cached_turns,
        crate::chat::providers::telegram::tests::history_restart::recent_history_turns_reads_codex_rollout_from_db_by_workdir,
        crate::chat::providers::telegram::tests::history_restart::restart_shell_command_drops_telegram_bot_and_keeps_debug_flag,
        crate::chat::providers::telegram::tests::history_restart::restart_shell_command_uses_release_profile_when_binary_is_release,
        crate::chat::providers::telegram::tests::journal::journal_recovery_probes_if_any_pending_request_is_stalled,
        crate::chat::providers::telegram::tests::journal::journal_recovery_runs_immediately_for_pending_on_startup,
        crate::chat::providers::telegram::tests::journal::journal_recovery_waits_for_stall_when_direct_hook_is_alive,
        crate::chat::providers::telegram::tests::pending::approval_lookup_selects_request_by_request_id,
        crate::chat::providers::telegram::tests::pending::completed_reply_includes_request_attribution,
        crate::chat::providers::telegram::tests::pending::matching_pending_request_index_ignores_stale_stop_in_awaiting_submit,
        crate::chat::providers::telegram::tests::pending::matching_pending_request_index_targets_correct_submit_event,
        crate::chat::providers::telegram::tests::pending::pad_status_lists_all_pending_requests,
        crate::chat::providers::telegram::tests::pending::pending_status_moves_from_accepted_to_working,
        crate::chat::providers::telegram::tests::pending::pending_status_reports_approval_needed,
        crate::chat::providers::telegram::tests::pending::pending_status_summary_line_is_compact_but_identifiable,
        crate::chat::providers::telegram::tests::state::next_request_and_draft_ids_are_unique,
        crate::chat::providers::telegram::tests::state::pending_lookup_is_per_pane_not_global,
        crate::chat::providers::telegram::tests::state::processed_hook_events_are_deduplicated_across_channels,
        crate::chat::providers::telegram::tests::state::processed_updates_are_deduplicated,
        crate::chat::providers::telegram::tests::state::selected_target_reset_removes_only_matching_pending,
        crate::chat::providers::telegram::tests::state::telegram_state_backfills_missing_last_processed_update_id,
        crate::chat::providers::telegram::tests::state::telegram_state_loads_legacy_pending_field_into_pending_requests,
    );
}

#[test]
fn notify_tests_suite() {
    run_cases!(
        crate::notify::tests::command_exists_detects_program_in_path,
        crate::notify::tests::linux_skips_when_notify_send_missing,
        crate::notify::tests::linux_skips_without_desktop_session,
        crate::notify::tests::linux_uses_notify_send_on_wayland,
        crate::notify::tests::linux_uses_notify_send_on_x11,
        crate::notify::tests::linux_uses_notify_send_with_dbus_session_only,
        crate::notify::tests::macos_uses_osascript_with_argument_passing,
    );
}

#[test]
fn relay_common_suite() {
    run_cases!(
        crate::relay::common::files::tests::preserve_backup_creates_a_private_file,
        crate::relay::common::files::tests::preserve_backup_rejects_symlink_path,
        crate::relay::common::files::tests::preserve_backup_tightens_existing_file_without_overwriting_it,
    );
}

#[test]
fn relay_tests_suite() {
    run_cases!(
        crate::relay::tests::provider_configs::claude::claude_provider_clears_stale_default_model_env_when_unconfigured,
        crate::relay::tests::provider_configs::claude::claude_provider_strips_trailing_v1_from_base_url,
        crate::relay::tests::provider_configs::claude::claude_provider_writes_cc_switch_style_env_settings,
        crate::relay::tests::provider_configs::claude::claude_provider_writes_default_model_env_when_configured,
        crate::relay::tests::provider_configs::claude::claude_provider_writes_disable_thinking_env_only_when_enabled,
        crate::relay::tests::provider_configs::claude_safety::claude_provider_does_not_overwrite_malformed_settings,
        crate::relay::tests::provider_configs::claude_safety::claude_provider_follows_claude_config_dir,
        crate::relay::tests::provider_configs::codex::codex_export_writes_pad_yaml_without_wire_api,
        crate::relay::tests::provider_configs::codex::codex_import_restores_exported_pad_yaml,
        crate::relay::tests::provider_configs::codex::codex_relay_normalizes_root_base_url_to_v1,
        crate::relay::tests::provider_configs::codex::codex_relay_preserves_explicit_v1_base_url,
        crate::relay::tests::provider_configs::codex::complete_codex_provider_keeps_relay_config,
        crate::relay::tests::provider_configs::codex::incomplete_codex_provider_restores_native_config,
        crate::relay::tests::provider_configs::deepseek::deepseek_launcher_atomically_replaces_existing_broad_file,
        crate::relay::tests::provider_configs::deepseek::deepseek_launcher_failure_preserves_existing_file,
        crate::relay::tests::provider_configs::deepseek::deepseek_launcher_keeps_secret_owner_only,
        crate::relay::tests::provider_configs::gemini::gemini_provider_writes_env_and_preserves_settings_json,
        crate::relay::tests::provider_configs::gemini::incomplete_gemini_provider_restores_original_files,
        crate::relay::tests::provider_configs::opencode::opencode_provider_prefers_existing_jsonc_and_preserves_urls_in_strings,
        crate::relay::tests::provider_configs::opencode::opencode_provider_writes_additive_live_config_and_models,
        crate::relay::tests::provider_configs::opencode::opencode_sync_removes_previously_managed_provider_keys,
        crate::relay::tests::provider_configs::opencode_runtime::runtime_overlays_do_not_rewrite_opencode_live_provider_config,
        crate::relay::tests::provider_configs::opencode_safety::opencode_provider_does_not_overwrite_malformed_config,
        crate::relay::tests::provider_configs::opencode_safety::opencode_provider_does_not_overwrite_unsupported_jsonc,
        crate::relay::tests::provider_configs::security::relay_provider_files_are_private,
        crate::relay::tests::runtime_overlays::claude_permissions::runtime_configs_apply_claude_full_access_without_relay_provider,
        crate::relay::tests::runtime_overlays::claude_permissions::runtime_configs_restore_previous_claude_permission_fields_when_disabled,
        crate::relay::tests::runtime_overlays::codex_features::fast_mode::runtime_configs_apply_codex_fast_mode_without_relay_provider,
        crate::relay::tests::runtime_overlays::codex_features::fast_mode::runtime_configs_restore_previous_codex_fast_fields_when_disabled,
        crate::relay::tests::runtime_overlays::codex_features::goals::runtime_configs_apply_codex_goals_without_relay_provider,
        crate::relay::tests::runtime_overlays::codex_features::goals::runtime_configs_restore_previous_codex_goals_when_disabled,
        crate::relay::tests::runtime_overlays::codex_features::multi_agent::runtime_configs_apply_codex_multi_agent_without_relay_provider,
        crate::relay::tests::runtime_overlays::codex_features::multi_agent::runtime_configs_restore_previous_codex_multi_agent_when_disabled,
        crate::relay::tests::runtime_overlays::codex_features::web_search::runtime_configs_apply_codex_web_search_without_relay_provider,
        crate::relay::tests::runtime_overlays::codex_features::web_search::runtime_configs_restore_previous_codex_web_search_when_defaulted,
        crate::relay::tests::runtime_overlays::codex_permissions::runtime_configs_apply_codex_full_access_without_relay_provider,
        crate::relay::tests::runtime_overlays::codex_permissions::runtime_configs_restore_previous_codex_permission_fields_when_disabled,
        crate::relay::tests::runtime_overlays::codex_prompts::runtime_configs_apply_codex_index_prompt_file_without_relay_provider,
        crate::relay::tests::runtime_overlays::codex_prompts::runtime_configs_apply_codex_jailbreak_prompt_file_without_relay_provider,
        crate::relay::tests::runtime_overlays::codex_prompts::runtime_configs_apply_combined_codex_prompt_candidates_without_relay_provider,
        crate::relay::tests::runtime_overlays::codex_prompts::runtime_configs_restore_previous_codex_jailbreak_prompt_file_when_disabled,
        crate::relay::tests::runtime_overlays::codex_status_line::runtime_configs_apply_codex_status_line_without_relay_provider,
        crate::relay::tests::runtime_overlays::codex_status_line::runtime_configs_apply_partial_codex_status_line_without_relay_provider,
        crate::relay::tests::runtime_overlays::codex_status_line::runtime_configs_restore_previous_codex_status_line_when_disabled,
        crate::relay::tests::runtime_overlays::combined_overlays::runtime_configs_apply_combined_codex_overlays_together,
        crate::relay::tests::runtime_overlays::combined_overlays::runtime_configs_restore_combined_codex_overlays_to_original_values,
        crate::relay::tests::serialize_env_file_keeps_sorted_lines_and_trailing_newline,
    );
}

#[test]
fn runtime_status_identity_suite() {
    run_cases!(
        crate::runtime_status::identity::tests::etime_parser_reads_every_ps_shape,
        crate::runtime_status::identity::tests::etime_parser_rejects_garbage,
        crate::runtime_status::identity::tests::started_at_matches_tolerates_early_start_only,
        crate::runtime_status::identity::tests::status_process_alive_rejects_recycled_pid,
    );
}

#[test]
fn runtime_status_tests_suite() {
    if std::env::var_os("PAD_STATUS_LOCK_TEST_PATH").is_some() {
        crate::runtime_status::tests::lock::hold_status_lock_for_test();
        return;
    }
    run_cases!(
        crate::runtime_status::tests::guard::concurrent_status_guards_have_one_owner,
        crate::runtime_status::tests::guard::stat_parser_treats_zombies_as_not_alive,
        crate::runtime_status::tests::guard::status_guard_drop_preserves_newer_status_file,
        crate::runtime_status::tests::guard::status_guard_refuses_when_owner_still_runs,
        crate::runtime_status::tests::guard::status_guard_starts_when_pid_was_recycled,
        crate::runtime_status::tests::lock::hold_status_lock_for_test,
        crate::runtime_status::tests::lock::status_lock_is_exclusive_across_processes_and_released_on_crash,
        crate::runtime_status::tests::lock::status_lock_is_exclusive_and_released_on_drop,
    );
}

#[test]
fn socket_api_tests_suite() {
    run_cases!(
        crate::socket_api::tests::handler::browser_open_dry_run_returns_command,
        crate::socket_api::tests::handler::prompt_dry_run_does_not_touch_native_pane,
        crate::socket_api::tests::handler::rejects_unknown_action,
        crate::socket_api::tests::peer::authorize_peer_accepts_same_uid_connection,
        crate::socket_api::tests::peer::foreign_uid_peer_is_rejected,
        crate::socket_api::tests::peer::same_uid_peer_is_allowed,
        crate::socket_api::tests::server::bind_private_listener_creates_owner_only_socket,
        crate::socket_api::tests::server::bind_private_listener_reclaims_dead_socket,
        crate::socket_api::tests::server::bind_private_listener_refuses_live_socket,
        crate::socket_api::tests::server::only_native_pane_actions_are_routed_through_the_ui,
        crate::socket_api::tests::server::ui_call_execution_is_claimed_once_and_cancelled_at_the_boundary,
        crate::socket_api::tests::socket_file::bind_private_listener_reclaims_stale_socket,
        crate::socket_api::tests::socket_file::bind_private_listener_rejects_active_socket,
        crate::socket_api::tests::socket_file::bind_private_listener_sets_private_socket_and_directory_modes,
    );
}

#[test]
fn sound_tests_suite() {
    run_cases!(
        crate::sound::tests::ensure_runtime_assets_writes_all_presets,
        crate::sound::tests::linux_command_spec_uses_expected_priority,
        crate::sound::tests::macos_command_spec_uses_local_wav_path,
        crate::sound::tests::normalize_preset_id_falls_back_to_default,
        crate::sound::tests::play_event_records_test_playback_when_enabled,
        crate::sound::tests::play_event_respects_global_and_event_switches,
    );
}
