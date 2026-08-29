//! Consolidated entry points for the small legacy modules that are not part of
//! the generated domain suites. Keeping one entry per boundary preserves all
//! case bodies while keeping the top-level Rust test surface easy to scan.

pub(crate) fn storage_policy_and_sidebar_cases() {
    crate::pad_store::tests::builds_private_schema_with_foreign_keys_and_version();
    crate::pad_store::tests::desktop_ui_state_round_trip_survives_close_and_reopen();
    crate::pad_store::tests::migration_from_v1_preserves_all_existing_records();
    crate::pad_store::tests::desktop_ui_state_enforces_bounds_and_reports_corruption();
    crate::pad_store::tests::desktop_ui_state_document_is_pad_private_and_secret_free();
    crate::pad_store::tests::crud_round_trip_preserves_profile_project_task_and_section_items();
    crate::pad_store::tests::data_survives_close_and_reopen();
    crate::pad_store::tests::foreign_keys_and_polymorphic_triggers_reject_orphan_references();
    crate::pad_store::tests::deleting_targets_removes_section_items_without_orphans();
    crate::pad_store::tests::provider_owned_paths_are_rejected_before_database_creation();

    crate::permission_policy_tests::permission_modes_use_stable_snake_case_json();
    crate::permission_policy_tests::lexical_canonicalization_resolves_traversal();
    crate::permission_policy_tests::protected_namespace_matching_is_component_safe();
    crate::permission_policy_tests::guarded_allows_reads_and_prompts_for_writes();
    crate::permission_policy_tests::workspace_full_allows_workspace_mutation_but_prompts_external_work();
    crate::permission_policy_tests::system_full_allows_external_operations_but_never_protected_namespace();
    crate::permission_policy_tests::system_full_only_auto_allows_statically_verified_shell_commands(
    );
    crate::permission_policy_tests::quoted_and_concatenated_shell_literals_cannot_hide_protected_paths();
    crate::permission_policy_tests::symlink_targets_are_resolved_before_full_access_is_allowed();
    crate::permission_policy_tests::unattended_policy_turns_confirmation_into_deny();
    crate::permission_policy_tests::merge_layers_uses_specific_scalar_and_additive_collections();
    crate::permission_policy_tests::model_hierarchy_adds_project_roots_and_task_cwd();
    crate::permission_policy_tests::defaults_protect_pad_pi_and_codex_namespaces();
    crate::permission_policy_tests::pi_session_metadata_round_trips_parent_and_cursor_fields();
    crate::permission_policy_tests::sidebar_section_serializes_polymorphic_items();

    crate::pi_runtime::session_index::tests::indexes_header_entries_messages_leaf_parent_and_cursor(
    );
    crate::pi_runtime::session_index::tests::empty_and_missing_header_are_diagnostic_and_read_only(
    );
    crate::pi_runtime::session_index::tests::unterminated_tail_returns_error_without_repairing_source();
    crate::pi_runtime::session_index::tests::rebuild_is_read_only_and_keeps_healthy_files_when_path_is_invalid();

    crate::relay::tests::pi_runtime_adapter_tests();
    crate::sidebar::codex::tests::renders_codex_project_task_hierarchy_and_projectless_task();
    crate::sidebar::codex::tests::pinned_view_keeps_project_for_a_pinned_task();
    crate::sidebar::codex::tests::search_retains_matching_project_ancestor();
    crate::sidebar::codex::tests::selection_wraps_over_visible_codex_rows();
    crate::ui::codex_sidebar::tests::project_and_task_keep_row_depth_and_logical_indent();
    crate::ui::codex_sidebar::tests::task_status_mapping_covers_domain_states();
    crate::ui::codex_sidebar::tests::important_runtime_states_are_visible_on_task_rows();
    crate::ui::codex_sidebar::tests::missing_references_are_safe_and_explicit();
    crate::ui::codex_sidebar::tests::display_rows_have_stable_ipc_shape();
    crate::ui::codex_sidebar::tests::snapshot_keeps_navigation_state_and_display_rows_together();
}

pub(crate) fn desktop_runtime_and_supervisor_cases() {
    crate::desktop_runtime::data_root_lock::same_root_has_exactly_one_owner();
    crate::desktop_runtime::tests::profile_scoped_process_events_update_the_private_task_record();
    crate::desktop_runtime::tests::sidebar_snapshot_is_read_from_the_pad_store_only();
    crate::desktop_runtime::tests::empty_task_cwd_inherits_its_selected_project_root();
    crate::desktop_runtime::tests::explicit_permission_gate_is_the_only_full_access_ui_auto_response();
    crate::desktop_runtime::tests::cross_profile_project_cannot_supply_automatic_approval_policy();
    crate::desktop_runtime::tests::existing_task_session_is_restored_and_state_metadata_is_persisted();
    crate::desktop_runtime::tests::existing_task_session_outside_profile_root_is_rejected();
    crate::desktop_runtime::tests::history_falls_back_to_read_only_profile_session_journal();

    crate::desktop_runtime::bridge::tests::protocol_returns_codex_sidebar_for_bootstrap();
    crate::desktop_runtime::bridge::tests::create_task_start_poll_and_stop_round_trip();
    crate::desktop_runtime::bridge::tests::create_project_persists_the_selected_workspace_root();
    crate::desktop_runtime::bridge::tests::malformed_input_is_one_error_response_and_does_not_write_stdout();
    crate::desktop_runtime::bridge::tests::full_access_fields_are_persisted_on_profile_and_task_requests();
    crate::desktop_runtime::bridge::tests::set_task_persists_sidebar_flags();
    crate::desktop_runtime::bridge::tests::set_profile_persists_policy();
    crate::desktop_runtime::bridge::tests::response_shape_is_id_ok_result_or_error();
    crate::desktop_runtime::bridge::tests::renderer_alias_fields_deserialize_for_model_and_ui_response();
    crate::desktop_runtime::bridge::tests::provider_status_reports_profile_scoped_authentication_shape();
    crate::desktop_runtime::bridge::tests::poll_exposes_structured_ui_requests_without_answering_them();
    crate::desktop_runtime::bridge::tests::provider_auth_errors_are_distinguished_from_transport_errors();
    crate::desktop_runtime::bridge::tests::protocol_v2_hello_is_versioned_and_v1_ping_is_unchanged(
    );
    crate::desktop_runtime::bridge::tests::protocol_v2_rejects_illegal_fields_and_long_ids();
    crate::desktop_runtime::bridge::tests::bounded_frames_recover_after_oversize_and_report_disconnect();
    crate::desktop_runtime::bridge::tests::protocol_v2_bootstrap_uses_renderer_safe_records();
    crate::desktop_runtime::bridge::tests::v2_redaction_covers_every_default_protected_namespace();
    #[cfg(unix)]
    crate::desktop_runtime::bridge::tests::protocol_v2_enforces_active_profile_for_records_and_task_controls();
    crate::desktop_runtime::bridge::tests::v2_history_poll_and_events_redact_private_tool_data_only(
    );
    crate::desktop_runtime::bridge::tests::actual_v2_history_and_poll_routes_apply_redaction();
    crate::desktop_runtime::bridge::tests::actual_v2_error_route_redacts_private_session_path();
    crate::desktop_runtime::bridge::tests::v2_server_events_cover_task_runtime_account_and_auth();
    crate::desktop_runtime::bridge::tests::ui_state_v2_normalizes_references_and_drives_sidebar_snapshot();
    crate::desktop_runtime::bridge::tests::ui_state_v2_survives_runtime_restart();
    #[cfg(unix)]
    crate::desktop_runtime::bridge::tests::terminal_v2_bridge_is_task_bound_bounded_and_redacted();
    #[cfg(unix)]
    crate::desktop_runtime::bridge::tests::rust_auth_control_plane_owns_prompt_response_and_secret(
    );

    crate::pi_runtime::supervisor::tests::command_environment_is_private_and_generation_scoped();
    crate::pi_runtime::supervisor::tests::malformed_frame_does_not_poison_a_later_valid_event();
    crate::pi_runtime::supervisor::tests::stale_generation_messages_are_dropped();
    crate::pi_runtime::supervisor::tests::shutdown_kills_a_stuck_child_without_hanging();
    crate::pi_runtime::supervisor::tests::send_rejects_non_object_commands();
    crate::pi_runtime::supervisor::tests::profile_spawn_uses_profile_specific_agent_and_session_roots();
    crate::pi_runtime::supervisor::tests::profile_spawn_rejects_provider_owned_roots();
    crate::pi_runtime::supervisor::tests::profile_spawn_rejects_session_path_outside_profile_root();
    crate::pi_runtime::supervisor::tests::profile_spawn_rejects_session_symlink_escape();
    #[cfg(unix)]
    crate::pi_runtime::tests::desktop_pi_program_prefers_the_host_bundle();
}
