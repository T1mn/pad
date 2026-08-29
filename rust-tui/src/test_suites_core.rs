//! Generated domain suites; individual case bodies remain beside their implementation.

#[test]
fn atomic_file_tests_suite() {
    crate::compact_tests::storage_policy_and_sidebar_cases();
    run_cases!(
        crate::atomic_file::tests::write_private_creates_missing_parent_dirs,
        crate::atomic_file::tests::write_private_forces_owner_only_permissions,
        crate::atomic_file::tests::write_private_replaces_existing_content_without_leftover_temp_files,
    );
}

#[test]
fn claude_history_tests_suite() {
    run_cases!(
        crate::claude_history::tests::archive::archived_threads_are_excluded_from_active_list_and_visible_in_archived_list,
        crate::claude_history::tests::archive::hook_upsert_inserts_session_when_index_is_empty,
        crate::claude_history::tests::config_dir::default_history_scan_follows_claude_config_dir,
        crate::claude_history::tests::parse::local_command_scaffold_filter_is_case_insensitive,
        crate::claude_history::tests::parse::local_command_scaffold_is_not_used_as_title,
        crate::claude_history::tests::parse::parse_claude_thread_file_extracts_session_cwd_and_title,
        crate::claude_history::tests::parse::progress_only_stub_file_is_filtered_out,
        crate::claude_history::tests::parse::read_threads_ignores_subagents_directory,
        crate::claude_history::tests::parse::sidechain_file_is_filtered_out,
        crate::claude_history::tests::sync::incremental_sync_skips_unchanged_files_and_removes_deleted_ones,
        crate::claude_history::tests::sync::stale_threads_without_recent_assistant_are_filtered_out,
        crate::claude_history::tests::sync::thread_lookup_works_without_active_filtering,
    );
}

#[test]
fn cli_tests_suite() {
    run_cases!(crate::cli::tests::detects_internal_command_prefix,);
}

#[test]
fn codex_provider_sync_rollout_suite() {
    run_cases!(
        crate::codex_provider_sync::rollout::apply::tests::atomic_write_removes_temp_after_rename_failure,
    );
}

#[test]
fn codex_provider_sync_tests_suite() {
    run_cases!(
        crate::codex_provider_sync::tests::sync_preserves_rollout_file_permissions,
        crate::codex_provider_sync::tests::sync_skips_when_state_db_is_missing,
        crate::codex_provider_sync::tests::sync_to_provider_uses_pad_private_codex_home,
        crate::codex_provider_sync::tests::sync_updates_rollout_files_and_sqlite_provider,
    );
}

#[test]
fn codex_runtime_tests_suite() {
    run_cases!(
        crate::codex_runtime::tests::claude_agent_command_defaults_to_claude_when_empty,
        crate::codex_runtime::tests::claude_agent_command_unsets_inherited_anthropic_env,
        crate::codex_runtime::tests::codex_agent_command_replaces_existing_profile_with_pad,
        crate::codex_runtime::tests::codex_agent_command_strips_profile_variants,
        crate::codex_runtime::tests::codex_agent_command_uses_pad_profile_without_codex_home,
        crate::codex_runtime::tests::codex_agent_command_uses_wrapper_instead_of_inlining_auth,
        crate::codex_runtime::tests::codex_prepare_allows_required_auth_when_pad_key_exists,
        crate::codex_runtime::tests::codex_prepare_fails_when_pad_provider_requires_auth_but_key_is_missing,
        crate::codex_runtime::tests::first_command_token_accepts_absolute_codex_path,
        crate::codex_runtime::tests::non_codex_agent_is_not_wrapped,
    );
}

#[test]
fn codex_state_archive_suite() {
    run_cases!(
        crate::codex_state::archive::path::tests::rollout_date_parts_reads_normal_timestamp,
        crate::codex_state::archive::path::tests::rollout_date_parts_rejects_missing_prefix_and_short_stem,
        crate::codex_state::archive::path::tests::rollout_date_parts_rejects_multibyte_day_with_valid_separators,
        crate::codex_state::archive::path::tests::rollout_date_parts_rejects_multibyte_name_instead_of_panicking,
    );
}

#[test]
fn codex_state_tests_suite() {
    run_cases!(
        crate::codex_state::tests::archive::archive_thread_accepts_rollout_through_symlinked_sessions_dir,
        crate::codex_state::tests::archive::archive_thread_moves_rollout_and_updates_db,
        crate::codex_state::tests::archive::unarchive_thread_moves_rollout_back_and_updates_db,
        crate::codex_state::tests::archive_compressed::archive_thread_resolves_compressed_rollout_sibling,
        crate::codex_state::tests::archive_compressed::unarchive_thread_resolves_compressed_rollout_sibling,
        crate::codex_state::tests::migration::normalize_pad_codex_home_rollout_paths_handles_non_ascii_home,
        crate::codex_state::tests::migration::normalize_pad_codex_home_rollout_paths_rewrites_shared_prefixes,
        crate::codex_state::tests::query::archived_threads_are_loaded_without_recent_filter,
        crate::codex_state::tests::query::loads_threads_from_state_db,
        crate::codex_state::tests::query::old_threads_without_recent_updated_at_are_filtered_out,
        crate::codex_state::tests::query::thread_for_id_reads_single_row_without_recent_filter,
        crate::codex_state::tests::selection::component_prefix_does_not_match_sibling_paths,
        crate::codex_state::tests::selection::falls_back_to_closest_related_thread_when_exact_match_missing,
        crate::codex_state::tests::selection::prefers_exact_cwd_match_before_related_threads,
    );
}

#[test]
fn codex_turn_diff_tests_suite() {
    run_cases!(
        crate::codex_turn_diff::tests::cli_hook_records_from_normalized_hook_json,
        crate::codex_turn_diff::tests::git::git_args_format_keeps_error_message_shape,
        crate::codex_turn_diff::tests::records_only_changes_between_submit_and_stop,
        crate::codex_turn_diff::tests::records_untracked_files_created_by_turn,
        crate::codex_turn_diff::tests::storage_paths::safe_name_collapses_trims_and_limits,
    );
}

#[test]
fn fuzzy_tests_suite() {
    run_cases!(
        crate::fuzzy::tests::scan_directories_skips_hidden_dirs_and_sorts,
        crate::fuzzy::tests::shift_delete_clears_query,
    );
}

#[test]
fn gemini_history_tests_suite() {
    run_cases!(
        crate::gemini_history::tests::archive::archive_by_session_id_updates_all_matching_rows,
        crate::gemini_history::tests::archive::main_snapshot_wins_over_subagent_and_archive_is_local,
        crate::gemini_history::tests::query::normalized_project_root_matches_cwd_query,
        crate::gemini_history::tests::query::threads_for_cwd_uses_project_root,
        crate::gemini_history::tests::scan::indexed_rows_survive_when_source_snapshots_disappear,
        crate::gemini_history::tests::scan::invalid_snapshot_does_not_break_sync,
        crate::gemini_history::tests::scan::scan_joins_nested_message_parts_without_empty_entries,
    );
}

#[test]
fn grok_history_tests_suite() {
    run_cases!(crate::grok_history::tests::scans_official_summary_and_skips_corrupt_sessions,);
}

#[test]
fn i18n_tests_suite() {
    run_cases!(
        crate::i18n::tests::all_static_i18n_keys_are_defined,
        crate::i18n::tests::settings_on_is_defined_for_all_locales,
    );
}

#[test]
fn model_tests_suite() {
    run_cases!(
        crate::model::tests::agent::from_processes_detects_agent_case_insensitively,
        crate::model::tests::agent::from_processes_returns_unknown_without_agent_name,
        crate::model::tests::preview::shared_preview_turns_clone_reuses_allocation,
        crate::model::tests::preview::shared_preview_turns_equality_uses_same_allocation,
    );
}

#[test]
fn notification_inbox_model_suite() {
    run_cases!(
        crate::notification_inbox::model::inbox::tests::inbox_keeps_newest_first_and_counts_unread,
        crate::notification_inbox::model::inbox::tests::mark_read_and_delete_report_changes,
    );
}

#[test]
fn notification_inbox_storage_suite() {
    run_cases!(crate::notification_inbox::storage::tests::save_and_load_round_trips_entries,);
}

#[test]
fn opencode_history_tests_suite() {
    run_cases!(
        crate::opencode_history::tests::archive::archive_matches_upstream_semantics_without_reordering_session,
        crate::opencode_history::tests::missing_thread_archive_returns_not_found,
        crate::opencode_history::tests::query::query_threads_reads_opencode_sqlite,
        crate::opencode_history::tests::query::query_threads_supports_older_opencode_schema_without_stats,
        crate::opencode_history::tests::stats::session_stats_select_uses_fallbacks_for_old_schema,
        crate::opencode_history::tests::stats::token_summary_formats_total_breakdown_and_cache,
        crate::opencode_history::tests::stats::token_summary_omits_empty_stats,
    );
}

#[test]
fn opencode_history_util_suite() {
    run_cases!(
        crate::opencode_history::util::db_paths::tests::configured_db_paths_keeps_default_discovery_without_env_override,
        crate::opencode_history::util::db_paths::tests::configured_db_paths_resolves_relative_env_under_xdg_data,
        crate::opencode_history::util::db_paths::tests::configured_db_paths_skips_in_memory_database,
        crate::opencode_history::util::db_paths::tests::configured_db_paths_uses_absolute_env_exclusively,
        crate::opencode_history::util::db_paths::tests::default_db_paths_does_not_fall_back_for_in_memory_override,
    );
}

#[test]
fn opencode_text_tests_suite() {
    run_cases!(
        crate::opencode_text::tests::display_text_skips_non_display_part_types,
        crate::opencode_text::tests::joins_nested_text_parts_without_empty_items,
        crate::opencode_text::tests::reads_supported_message_roles,
    );
}

#[test]
fn panic_boundary_tests_suite() {
    run_cases!(
        crate::panic_boundary::tests::marker_is_scoped_and_supports_nesting,
        crate::panic_boundary::tests::marker_remains_active_while_panic_hook_runs,
    );
}

#[test]
fn paths_base_suite() {
    run_cases!(
        crate::paths::base::tests::explicit_pad_home_is_used_without_rewriting_process_home,
        crate::paths::base::tests::desktop_store_uses_a_separate_application_data_suffix,
        crate::paths::base::tests::terminal_workspace_lives_under_pad_home,
    );
}

#[test]
fn paths_codex_wrapper_suite() {
    run_cases!(
        crate::paths::codex_wrapper::tests::wrapper_template_reads_pad_auth_and_forces_pad_profile,
    );
}

#[test]
fn paths_paths_tests_suite() {
    run_cases!(
        crate::paths::paths_tests::bridge_hooks::claude_bridge_template_stays_minimal_and_forwards_turn_id,
        crate::paths::paths_tests::bridge_hooks::codex_bridge_template_keeps_required_stdin_and_turn_id_handling,
        crate::paths::paths_tests::bridge_hooks::codex_hooks_feature_key_switches_at_0130,
        crate::paths::paths_tests::bridge_hooks::parse_codex_cli_version_accepts_plain_and_prefixed_versions,
        crate::paths::paths_tests::bridge_hooks::remove_toml_key_in_section_removes_compact_assignment_only,
        crate::paths::paths_tests::bridge_hooks::remove_toml_key_in_section_removes_legacy_codex_hooks_key,
        crate::paths::paths_tests::bridge_hooks::set_toml_bool_in_section_preserves_leading_blank_line,
        crate::paths::paths_tests::bridge_hooks::set_toml_bool_in_section_updates_compact_assignment_only,
        crate::paths::paths_tests::bridge_hooks::set_toml_bool_in_section_writes_new_hooks_key,
        crate::paths::paths_tests::claude_paths::claude_paths_fall_back_for_missing_or_empty_override,
        crate::paths::paths_tests::claude_paths::claude_paths_follow_config_dir_override,
        crate::paths::paths_tests::codex_home::ensure_pad_codex_home_layout_copies_config_to_profile_but_not_auth,
        crate::paths::paths_tests::codex_home::ensure_pad_codex_home_layout_does_not_create_session_or_db_links,
        crate::paths::paths_tests::codex_home::ensure_pad_codex_home_layout_unlinks_legacy_shared_state_symlinks,
        crate::paths::paths_tests::prompts::ensure_runtime_layout_migrates_custom_legacy_codex_prompt_to_jailbreak_name,
        crate::paths::paths_tests::prompts::ensure_runtime_layout_preserves_custom_codex_jailbreak_prompt_edits,
        crate::paths::paths_tests::prompts::ensure_runtime_layout_refreshes_outdated_managed_codex_jailbreak_prompt,
        crate::paths::paths_tests::prompts::ensure_runtime_layout_reseeds_empty_codex_jailbreak_prompt_file,
        crate::paths::paths_tests::prompts::ensure_runtime_layout_tracks_current_codex_jailbreak_prompt_version,
        crate::paths::paths_tests::prompts::write_codex_selected_prompt_file_combines_selected_candidates,
        crate::paths::paths_tests::prompts::write_codex_selected_prompt_file_returns_single_candidate_directly,
        crate::paths::paths_tests::runtime_layout::ensure_runtime_layout_creates_codex_jailbreak_prompt_file,
        crate::paths::paths_tests::runtime_layout::ensure_runtime_layout_enables_codex_hooks_in_pad_profile_only,
        crate::paths::paths_tests::runtime_layout::ensure_runtime_layout_installs_executable_pad_codex_wrapper,
    );
}

#[test]
fn preview_source_claude_suite() {
    run_cases!(
        crate::preview_source::claude::tests::parse_claude_transcript_joins_text_array_parts,
        crate::preview_source::claude::tests::parse_claude_transcript_skips_meta_thinking_and_tools,
    );
}

#[test]
fn preview_source_codex_suite() {
    run_cases!(
        crate::preview_source::codex::tests::compressed::parse_codex_transcript_reads_compressed_sibling_with_future_fields,
        crate::preview_source::codex::tests::normalize::normalize_codex_user_text_does_not_touch_plain_text_without_images,
        crate::preview_source::codex::tests::normalize::normalize_codex_user_text_filters_environment_context_block,
        crate::preview_source::codex::tests::normalize::normalize_codex_user_text_filters_turn_aborted_marker,
        crate::preview_source::codex::tests::normalize::normalize_codex_user_text_handles_image_only_message,
        crate::preview_source::codex::tests::normalize::normalize_codex_user_text_keeps_image_body_lines,
        crate::preview_source::codex::tests::normalize::normalize_codex_user_text_strips_embedded_environment_context_block,
        crate::preview_source::codex::tests::normalize::normalize_codex_user_text_summarizes_user_shell_command,
        crate::preview_source::codex::tests::subagent::subagent_summary_compacts_whitespace_without_losing_detail,
        crate::preview_source::codex::tests::tail::tail_reader_drops_partial_first_line,
        crate::preview_source::codex::tests::tail::tail_reader_keeps_whole_file_when_short,
        crate::preview_source::codex::tests::tail::tail_window_helpers_clamp_and_grow,
        crate::preview_source::codex::tests::transcript::parse_codex_transcript_backfills_beyond_six_turns,
        crate::preview_source::codex::tests::transcript::parse_codex_transcript_extracts_recent_messages,
        crate::preview_source::codex::tests::transcript::parse_codex_transcript_includes_subagent_events_in_main_turn,
        crate::preview_source::codex::tests::transcript::parse_codex_transcript_keeps_latest_real_user_turns,
        crate::preview_source::codex::tests::transcript::parse_codex_transcript_normalizes_multiple_image_user_message,
        crate::preview_source::codex::tests::transcript::parse_codex_transcript_normalizes_single_image_user_message,
        crate::preview_source::codex::tests::transcript::parse_codex_transcript_skips_context_only_user_messages,
    );
}

#[test]
fn preview_source_gemini_suite() {
    run_cases!(
        crate::preview_source::gemini::tests::extract_session_id_from_transcript_reads_root_metadata,
        crate::preview_source::gemini::tests::parse_gemini_transcript_joins_nested_non_empty_text_parts,
        crate::preview_source::gemini::tests::parse_gemini_transcript_skips_info_and_keeps_pairs,
    );
}

#[test]
fn preview_source_grok_suite() {
    run_cases!(
        crate::preview_source::grok::tests::accepts_direct_update_shape_for_older_logs,
        crate::preview_source::grok::tests::parses_official_0_2_102_envelopes_and_skips_unknown_lines,
    );
}

#[test]
fn preview_source_opencode_suite() {
    run_cases!(crate::preview_source::opencode::tests::parses_opencode_sqlite_messages_into_turns,);
}

#[test]
fn preview_source_session_target_suite() {
    run_cases!(
        crate::preview_source::session_target::sources::tests::codex::codex_db_canonical_path_resolves_compressed_sibling,
        crate::preview_source::session_target::sources::tests::path::live_cwd_candidate_requires_one_unambiguous_session,
        crate::preview_source::session_target::tests::claude_target_follows_claude_config_dir,
        crate::preview_source::session_target::tests::gemini_session_id_can_be_read_from_transcript_path,
        crate::preview_source::session_target::tests::persistence_panel_uses_resolved_target_session_id,
    );
}

#[test]
fn preview_source_tests_suite() {
    run_cases!(
        crate::preview_source::tests::bench::rollout::rollout_session_id_extracts_uuid_suffix,
        crate::preview_source::tests::bench::rollout::rollout_session_id_rejects_non_rollout_or_invalid_uuid,
        crate::preview_source::tests::cases::confirmed_cached_preview_returns_without_resolving_target,
        crate::preview_source::tests::cases::request_refresh_interval_is_adaptive_to_state_and_origin,
        crate::preview_source::tests::cases::session_preview_update_keeps_content_empty_for_memory,
    );
}

#[test]
fn preview_source_turns_suite() {
    run_cases!(
        crate::preview_source::turns::tests::assistant_messages_append_to_last_question,
        crate::preview_source::turns::tests::formatting_uses_q_and_a_blocks,
    );
}

#[test]
fn session_cache_tests_suite() {
    run_cases!(
        crate::session_cache::tests::lookup::loads_only_requested_agent_snapshots_with_source_state,
        crate::session_cache::tests::persist::session_start_for_new_id_does_not_inherit_panel_session_state,
        crate::session_cache::tests::persist::session_start_for_same_id_keeps_panel_fallbacks,
        crate::session_cache::tests::turns::merge_recent_turns_does_not_reuse_previous_answer_for_new_prompt,
        crate::session_cache::tests::turns::merge_recent_turns_prefers_latest_prompt_and_answer,
    );
}

#[test]
fn session_cache_turns_suite() {
    run_cases!(
        crate::session_cache::turns::tests::normalize_cached_codex_prompt_filters_codex_context,
        crate::session_cache::turns::tests::normalize_cached_codex_prompt_trims_without_precopy,
        crate::session_cache::turns::tests::normalize_turns_matches_for_owned_and_borrowed_inputs,
        crate::session_cache::turns::tests::normalize_turns_stops_after_history_limit_valid_turns,
    );
}

#[test]
fn session_continuity_tests_suite() {
    run_cases!(
        crate::session_continuity::tests::bootstrap_classification_clears_once_transcript_is_known,
        crate::session_continuity::tests::frozen_decision_marks_cache_fallback,
        crate::session_continuity::tests::preview_health_promotes_to_frozen_with_strong_runtime_signal,
        crate::session_continuity::tests::record_becomes_frozen_after_repeated_stale_runtime_activity,
    );
}

#[test]
fn shell_quote_tests_suite() {
    run_cases!(
        crate::shell_quote::tests::single_quote_escapes_embedded_single_quotes,
        crate::shell_quote::tests::single_quote_wraps_plain_value,
    );
}

#[test]
fn sidebar_build_suite() {
    run_cases!(
        crate::sidebar::build::tests::activity::merge_or_insert_preserves_history_prompt_when_live_thread_lacks_one,
        crate::sidebar::build::tests::activity::runtime_sort_activity_updates_history_order,
        crate::sidebar::build::tests::history::active_view_history_entries_do_not_sort_by_updated_at_without_explicit_activity,
        crate::sidebar::build::tests::history::archived_view_history_entries_keep_updated_at_sorting,
        crate::sidebar::build::tests::history::codex_history_prefers_session_cache_prompt_for_subtitle,
        crate::sidebar::build::tests::meta::generated_summary_does_not_replace_session_title,
        crate::sidebar::build::tests::meta::manual_title_override_wins_over_generated_summary_for_title,
    );
}

#[test]
fn sidebar_display_suite() {
    run_cases!(
        crate::sidebar::display::tests::clean_title_trims_and_keeps_first_line,
        crate::sidebar::display::tests::folder_display_label_includes_parent_leaf,
    );
}

#[test]
fn sidebar_provider_suite() {
    run_cases!(
        crate::sidebar::provider::tests::resolve_session_provider_name_reads_codex_session_meta,
    );
}

#[test]
fn sidebar_search_suite() {
    run_cases!(
        crate::sidebar::search::tests::search_expands_matching_folder_threads,
        crate::sidebar::search::tests::search_keeps_unicode_case_fold_behavior,
        crate::sidebar::search::tests::search_matches_agent_type_without_string_allocation,
        crate::sidebar::search::tests::search_matches_ascii_case_insensitively_without_lowercase_copy,
        crate::sidebar::search::tests::source_json_detects_subagent_thread,
        crate::sidebar::search::tests::visible_items_keep_collapsed_folder_without_threads,
        crate::sidebar::search::tests::visible_items_reuse_thread_allocation_when_folder_is_expanded,
    );
}

#[test]
fn text_match_tests_suite() {
    run_cases!(
        crate::text_match::tests::ascii_contains_ignores_case_without_unicode_fold,
        crate::text_match::tests::unicode_contains_keeps_case_fold_behavior,
    );
}

#[test]
fn text_normalize_tests_suite() {
    run_cases!(
        crate::text_normalize::tests::collapse_whitespace_joins_non_empty_parts_with_single_spaces,
        crate::text_normalize::tests::collapse_whitespace_returns_empty_for_blank_input,
    );
}

#[test]
fn theme_load_suite() {
    run_cases!(
        crate::theme::load::backup::tests::backup_returns_none_when_source_is_unreadable,
        crate::theme::load::backup::tests::backup_reuses_slot_for_identical_content_and_advances_for_new_damage,
    );
}

#[test]
fn theme_save_suite() {
    run_cases!(
        crate::theme::save::render::tests::multiline_value_round_trips,
        crate::theme::save::render::tests::plain_values_stay_basic_strings,
        crate::theme::save::render::tests::quotes_tabs_and_control_characters_round_trip,
        crate::theme::save::render::tests::rendered_default_config_is_parseable,
        crate::theme::save::render::tests::value_ending_with_backslash_round_trips,
        crate::theme::save::render::tests::windows_style_backslash_value_round_trips,
    );
}

#[test]
fn theme_tests_suite() {
    run_cases!(
        crate::theme::tests::config::config_defaults_agent_permissions_to_enabled,
        crate::theme::tests::config::config_loads_profile_full_access_compatibility_alias,
        crate::theme::tests::config::config_loads_legacy_codex_prompt_file_as_jailbreak_prompt_file,
        crate::theme::tests::config::config_profile_mode_wins_over_compatibility_alias,
        crate::theme::tests::config::config_round_trips_opencode_provider_models,
        crate::theme::tests::config::profile_config_normalizes_modes_and_keeps_alias_in_sync,
        crate::theme::tests::config::config_save_omits_wire_api_entries,
        crate::theme::tests::config::load_from_path_reports_invalid_toml,
        crate::theme::tests::config::resolved_config_path_prefers_pad_home_over_legacy_path,
        crate::theme::tests::palette::readability_boost_keeps_status_text_close_to_primary_fg,
        crate::theme::tests::palette::readability_boost_lifts_comment_contrast,
        crate::theme::tests::persist::api_key_with_backslashes_survives_round_trip,
        crate::theme::tests::persist::broken_config_is_backed_up_before_falling_back_to_defaults,
        crate::theme::tests::persist::multiline_and_control_character_values_survive_round_trip,
        crate::theme::tests::persist::saved_config_is_owner_only_readable,
        crate::theme::tests::persist::value_ending_with_backslash_does_not_break_the_file,
        crate::theme::tests::provider::codex_base_url_candidates_try_root_and_v1_variants,
        crate::theme::tests::provider::codex_base_url_prefers_v1_for_root_inputs,
        crate::theme::tests::sound::config_normalizes_invalid_sound_presets,
        crate::theme::tests::sound::config_round_trips_sound_section,
    );
}

#[test]
fn thread_meta_tests_suite() {
    run_cases!(
        crate::thread_meta::tests::ensure_schema_adds_generated_title_columns_to_existing_db,
        crate::thread_meta::tests::generated_title_updates_do_not_clobber_manual_override,
        crate::thread_meta::tests::load_deleted_thread_meta_hydrates_tags_once,
        crate::thread_meta::tests::load_deleted_thread_meta_returns_only_deleted_rows,
        crate::thread_meta::tests::load_thread_meta_reads_generated_fields,
        crate::thread_meta::tests::set_thread_deleted_marks_and_clears_deleted_state,
    );
}

#[test]
fn title_summary_tests_suite() {
    run_cases!(
        crate::title_summary::tests::initial_window_uses_three_turns_in_chronological_order,
        crate::title_summary::tests::refresh_window_keeps_six_newest_turns,
        crate::title_summary::tests::responses_is_default_wire_api,
        crate::title_summary::tests::title_normalization_collapses_internal_whitespace,
        crate::title_summary::tests::title_normalization_trims_wrappers_and_prefixes,
        crate::title_summary::tests::title_refresh_triggers_after_initial_threshold,
        crate::title_summary::tests::title_response_text_joins_chat_content_array_blocks,
        crate::title_summary::tests::title_response_text_joins_response_output_blocks,
        crate::title_summary::tests::title_response_text_preserves_empty_text_block_semantics,
    );
}

#[test]
fn tree_tests_suite() {
    run_cases!(
        crate::tree::tests::preview_type_detects_known_suffixes_case_insensitively,
        crate::tree::tests::search_filters_entries_case_insensitively,
    );
}
