//! Generated domain suites; individual case bodies remain beside their implementation.

#[test]
fn app_actions_suite() {
    run_cases!(
        crate::app::actions::codex_restart::tests::restart_command_falls_back_to_last_session,
        crate::app::actions::codex_restart::tests::restart_command_quotes_shell_values,
        crate::app::actions::codex_restart::tests::restart_command_resumes_specific_session,
        crate::app::actions::codex_restart::tests::restart_preflight_does_not_block_non_idle_codex,
        crate::app::actions::codex_restart::tests::restart_preflight_still_blocks_non_codex,
        crate::app::actions::notification_inbox::tests::notification_selection_clamps_to_available_entries,
        crate::app::actions::opencode_diagnostics::collect::tests::opencode_args_format_keeps_diagnostics_error_shape,
        crate::app::actions::opencode_diagnostics::opencode_diagnostics_tests::diagnostics_path_uses_timestamped_txt,
        crate::app::actions::opencode_diagnostics::opencode_diagnostics_tests::diagnostics_report_file_is_owner_only,
        crate::app::actions::opencode_diagnostics::opencode_diagnostics_tests::diagnostics_report_has_expected_sections,
        crate::app::actions::opencode_diagnostics::opencode_diagnostics_tests::diagnostics_report_redacts_sensitive_keys_and_token_prefixes,
        crate::app::actions::opencode_stats::export::tests::stats_uses_selected_project_as_cwd_and_empty_current_project_filter,
        crate::app::actions::opencode_tests::attach::attach_command_preserves_configured_command_and_quotes_url,
        crate::app::actions::opencode_tests::attach::attach_url_accepts_single_http_url_and_strips_quotes,
        crate::app::actions::opencode_tests::attach::attach_url_rejects_multi_line_or_non_http_clipboard,
        crate::app::actions::opencode_tests::cli::configured_command_keeps_shell_expansion_and_flags,
        crate::app::actions::opencode_tests::cli::safe_filename_keeps_underscore_inside_truncated_output,
        crate::app::actions::opencode_tests::cli::safe_filename_limits_output_length,
        crate::app::actions::opencode_tests::cli::safe_filename_sanitizes_and_falls_back,
        crate::app::actions::opencode_tests::export::opencode_export_path_sanitizes_session_id,
        crate::app::actions::opencode_tests::export::opencode_sanitized_export_path_uses_distinct_suffix,
        crate::app::actions::opencode_tests::github::github_install_command_preserves_configured_command,
        crate::app::actions::opencode_tests::import::import_source_accepts_json_path_and_strips_quotes,
        crate::app::actions::opencode_tests::import::import_source_accepts_opencode_share_url,
        crate::app::actions::opencode_tests::import::import_source_rejects_multi_line_clipboard,
        crate::app::actions::opencode_tests::native_launch::action_launches_opencode_in_native_terminal_and_registry,
        crate::app::actions::opencode_tests::plugin::plugin_command_preserves_configured_command_and_quotes_module,
        crate::app::actions::opencode_tests::plugin::plugin_module_accepts_npm_names_scope_and_versions,
        crate::app::actions::opencode_tests::plugin::plugin_module_rejects_empty_multiline_flags_and_whitespace,
        crate::app::actions::opencode_tests::pr::pr_command_preserves_configured_command,
        crate::app::actions::opencode_tests::pr::pr_number_accepts_plain_hash_and_github_url,
        crate::app::actions::opencode_tests::pr::pr_number_rejects_empty_zero_multiline_and_non_pr_url,
        crate::app::actions::opencode_tests::run::run_command_can_start_new_session_without_selected_opencode_thread,
        crate::app::actions::opencode_tests::run::run_command_is_one_shell_line_and_preserves_backslashes,
        crate::app::actions::opencode_tests::run::run_command_quotes_prompt_and_resumes_opencode_session,
        crate::app::actions::opencode_tests::run::run_prompt_preview_uses_first_non_empty_line,
        crate::app::actions::opencode_tests::run::run_prompt_trims_outer_blank_space_but_keeps_multiline_body,
        crate::app::actions::opencode_tests::serve::serve_command_stays_local_and_uses_random_port,
        crate::app::actions::opencode_tests::stats::opencode_stats_path_sanitizes_project,
        crate::app::actions::opencode_tests::web::web_command_preserves_configured_opencode_command,
        crate::app::actions::relay_reload::relay_reload_tests::deferred::external_relay_reload_is_deferred_while_editing,
        crate::app::actions::relay_reload::relay_reload_tests::immediate::external_relay_reload_applies_immediately_when_not_editing,
        crate::app::actions::relay_reload::relay_reload_tests::invalid::invalid_external_relay_config_is_ignored,
        crate::app::actions::relay_reload::relay_reload_tests::selection::external_relay_reload_clamps_provider_selection,
        crate::app::actions::tests::apply_deleted_panel_locally_removes_panel_immediately,
        crate::app::actions::tests::settings_detail_persists_when_filtered_value_changes,
        crate::app::actions::tests::settings_list_hides_refresh_interval_item,
        crate::app::actions::tests::settings_search_matches_english_terms_under_chinese_locale,
        crate::app::actions::tests::settings_search_matches_trash_aliases,
        crate::app::actions::tests::settings_search_no_longer_matches_refresh_interval_aliases,
        crate::app::actions::thread_actions::thread_actions_tests::opencode_thread_can_open_archive_confirm,
    );
}

#[test]
fn app_async_ops_suite() {
    run_cases!(
        crate::app::async_ops::codex_cli::commands::tests::parse_codex_version_from_cli_output,
        crate::app::async_ops::codex_cli::commands::tests::parse_npm_version_json_output,
        crate::app::async_ops::codex_cli::commands::tests::update_uses_detected_codex_binary_native_command,
        crate::app::async_ops::provider_test::claude::model::tests::full_model_1m_strips_display_suffix_first,
        crate::app::async_ops::provider_test::claude::model::tests::opus_1m_prefers_claude_code_wire_model,
    );
}

#[test]
fn app_clipboard_suite() {
    run_cases!(
        crate::app::clipboard::tests::copy_preview_summary_collapses_whitespace,
        crate::app::clipboard::tests::copy_preview_summary_truncates_with_ascii_ellipsis,
    );
}

#[test]
fn app_config_persist_suite() {
    run_cases!(
        crate::app::config_persist::tests::broken_config_reports_recovery_to_the_caller,
        crate::app::config_persist::tests::panel_width_save_failure_is_not_overwritten_by_success_toast,
        crate::app::config_persist::tests::save_config_succeeds_without_warning_toast,
        crate::app::config_persist::tests::save_config_surfaces_failure_instead_of_swallowing_it,
    );
}

#[test]
fn app_hooks_suite() {
    run_cases!(
        crate::app::hooks::hooks_tests::activity::app_stop_hook_does_not_auto_reorder_sidebar,
        crate::app::hooks::hooks_tests::activity::app_thread_activity_prunes_by_ttl_and_cap,
        crate::app::hooks::hooks_tests::activity::pane_stop_hook_does_not_auto_reorder_sidebar,
        crate::app::hooks::hooks_tests::notification::completion_notification_collapses_prompt_whitespace,
        crate::app::hooks::hooks_tests::notification::completion_notification_falls_back_to_workdir_name,
        crate::app::hooks::hooks_tests::notification::completion_notification_prefers_latest_prompt_over_persisted_codex_title,
        crate::app::hooks::hooks_tests::notification::completion_notification_truncates_long_text,
        crate::app::hooks::hooks_tests::notification::completion_notification_uses_prompt_when_lookup_is_unavailable,
        crate::app::hooks::hooks_tests::notification::stop_hook_adds_completion_to_notification_inbox,
        crate::app::hooks::hooks_tests::notification::stop_hook_emits_completion_sound_event,
        crate::app::hooks::hooks_tests::session_cache::new_session_start_does_not_inherit_prior_panel_snapshot,
        crate::app::hooks::hooks_tests::unread::focusing_panel_clears_unread_stop_marker,
        crate::app::hooks::hooks_tests::unread::stop_hook_marks_panel_unread_when_panel_item_is_not_focused,
    );
}

#[test]
fn app_navigation_suite() {
    run_cases!(
        crate::app::navigation::tests::movement::next_skips_expanded_folder_row,
        crate::app::navigation::tests::movement::next_skips_search_expanded_folder_row,
        crate::app::navigation::tests::movement::next_uses_folder_rows_when_not_expanded,
        crate::app::navigation::tests::movement::numeric_jump_ignores_folder_rows_and_hidden_threads,
        crate::app::navigation::tests::movement::numeric_jump_uses_filtered_visible_threads,
        crate::app::navigation::tests::movement::numeric_jump_uses_visible_thread_order,
        crate::app::navigation::tests::movement::shift_j_k_moves_selected_thread_without_following_completion_sort,
        crate::app::navigation::tests::selection::selected_preview_thread_resolves_from_folder_summary_selection,
        crate::app::navigation::tests::selection::sync_sidebar_selection_falls_back_to_first_visible_item_when_selected_key_is_filtered_out,
        crate::app::navigation::tests::selection::sync_sidebar_selection_recovers_collapsed_thread_to_folder_key,
        crate::app::navigation::tests::selection::visible_sidebar_items_sequence_stays_stable_across_expand_and_search,
    );
}

#[test]
fn app_preview_suite() {
    run_cases!(
        crate::app::preview::preview_tests::cache_dirty::detail_cache::detail_view_keeps_background_preview_refresh_alive,
        crate::app::preview::preview_tests::cache_dirty::detail_cache::identical_preview_update_preserves_detail_cache,
        crate::app::preview::preview_tests::cache_dirty::detail_cache::matching_detail_cache_rebases_to_current_turn_allocation,
        crate::app::preview::preview_tests::cache_dirty::dirty_update::identical_preview_update_keeps_dirty_cleared,
        crate::app::preview::preview_tests::cache_dirty::dirty_update::preview_update_marks_dirty_when_content_changes_but_turns_do_not,
        crate::app::preview::preview_tests::cache_dirty::dirty_update::preview_update_marks_dirty_when_turns_change_but_content_does_not,
        crate::app::preview::preview_tests::latest::open_latest_preview_turn_prefers_newer_panel_cached_turns_over_current_preview,
        crate::app::preview::preview_tests::latest::open_latest_preview_turn_uses_selected_panel_cached_turns,
        crate::app::preview::preview_tests::selection_scroll::context::preview_update_context_change_resets_selection_and_scroll,
        crate::app::preview::preview_tests::selection_scroll::detail::preview_update_same_context_clamps_selection_when_turns_shrink,
        crate::app::preview::preview_tests::selection_scroll::detail::preview_update_same_context_preserves_detail_selection_and_scroll,
        crate::app::preview::preview_tests::selection_scroll::panel_cache::preview_update_marks_dirty_when_only_panel_cache_state_changes,
        crate::app::preview::preview_tests::selection_scroll::plain::preview_update_plain_view_follow_bottom_depends_on_target_change,
        crate::app::preview::preview_tests::tick_cache::busy_tick::app_only_busy_thread_keeps_busy_animation_ticking,
        crate::app::preview::preview_tests::tick_cache::busy_tick::busy_threads_use_moderate_tick_rate,
        crate::app::preview::preview_tests::tick_cache::busy_tick::hidden_busy_threads_do_not_force_animation_redraws,
        crate::app::preview::preview_tests::tick_cache::busy_tick::slow_frame_only_slows_busy_animation_instead_of_stopping_it,
        crate::app::preview::preview_tests::tick_cache::debounce_detail::detail_view_applies_preview_updates_immediately,
        crate::app::preview::preview_tests::tick_cache::debounce_detail::detail_view_does_not_pause_busy_animations,
        crate::app::preview::preview_tests::tick_cache::debounce_detail::preview_update_during_navigation_debounce_is_deferred_until_idle,
        crate::app::preview::preview_tests::tick_cache::plain_cache::preview_update_changed_plain_content_bumps_revision_and_drops_cache,
        crate::app::preview::preview_tests::tick_cache::plain_cache::preview_update_identical_plain_view_preserves_plain_cache,
        crate::app::preview::preview_tests::tick_cache::thread_cache::thread_preview_cache_prunes_to_max_entries,
    );
}

#[test]
fn app_socket_api_suite() {
    run_cases!(
        crate::app::socket_api::tests::approval_rejects_keys_outside_the_explicit_allowlist,
        crate::app::socket_api::tests::prompt_dry_run_does_not_require_a_live_pane,
        crate::app::socket_api::tests::status_is_served_from_the_native_ui_state,
    );
}

#[test]
fn app_terminal_suite() {
    run_cases!(
        crate::app::terminal::tests::all_builtin_profiles_have_deterministic_commands,
        crate::app::terminal::tests::app_refuses_to_close_the_last_terminal_pane,
        crate::app::terminal::tests::builtin_opencode_profile_registers_and_restores_its_live_sidebar_entry,
        crate::app::terminal::tests::builtin_opencode_profile_uses_the_full_configured_shell_command,
        crate::app::terminal::tests::closing_an_earlier_tab_reindexes_native_sidebar_entries,
        crate::app::terminal::tests::configured_agent_is_the_direct_pty_child_of_a_known_shell,
        crate::app::terminal::tests::exited_native_agent_rejects_later_prompt_input,
        crate::app::terminal::tests::failed_workspace_save_rolls_back_a_close_before_runtime_mutation,
        crate::app::terminal::tests::keyboard_input_queues_bottom_before_bytes_but_mouse_does_not,
        crate::app::terminal::tests::labels_are_trimmed_and_control_characters_are_rejected,
        crate::app::terminal::tests::native_pane_focus_keeps_sidebar_selection_on_the_same_agent,
        crate::app::terminal::tests::nested_splits_close_and_collapse_without_orphans,
        crate::app::terminal::tests::pane_ids_are_monotonic_across_close_and_restore,
        crate::app::terminal::tests::pending_scroll_is_reset_before_immediate_keyboard_input,
        crate::app::terminal::tests::rename_stays_bound_to_the_pane_that_started_it,
        crate::app::terminal::tests::resize_and_scroll_state_are_independent_per_pane,
        crate::app::terminal::tests::restarting_the_same_app_relaunches_the_retained_workspace,
        crate::app::terminal::tests::restored_commands_are_derived_from_profiles,
        crate::app::terminal::tests::restored_serial_exhaustion_is_rejected_without_panicking,
        crate::app::terminal::tests::scroll_queue_coalesces_lines_and_has_a_hard_limit,
        crate::app::terminal::tests::scroll_reset_keeps_barriers_for_already_queued_input,
        crate::app::terminal::tests::split_rejects_geometry_that_cannot_render_both_children,
        crate::app::terminal::tests::tabs_remember_focus_and_clamp_after_close,
        crate::app::terminal::tests::workspace_json_contains_only_stable_layout_and_launch_data,
    );
}
