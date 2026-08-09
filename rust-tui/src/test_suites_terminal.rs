//! Generated domain suites; individual case bodies remain beside their implementation.

#[test]
fn terminal_tests_suite() {
    run_cases!(crate::terminal::tests::isolated_worker_boundary_keeps_terminal_active,);
}

#[test]
fn terminal_runtime_alacritty_suite() {
    run_cases!(
        crate::terminal_runtime::alacritty::tests::alternate_screen_has_no_scrollback_and_restores_primary_viewport,
        crate::terminal_runtime::alacritty::tests::listener_maps_exit_without_a_terminal_event_loop,
        crate::terminal_runtime::alacritty::tests::scrollback_clamps_and_stays_anchored_during_output,
        crate::terminal_runtime::alacritty::tests::snapshot_indexes_a_scrollback_viewport_from_zero,
    );
}

#[test]
fn terminal_runtime_controller_suite() {
    run_cases!(
        crate::terminal_runtime::controller::tests::controller_queue_backpressure_returns_original_input,
        crate::terminal_runtime::controller::tests::controller_queue_backpressure_returns_original_scroll,
        crate::terminal_runtime::controller::tests::delayed_older_open_cannot_replace_a_newer_epoch,
        crate::terminal_runtime::controller::tests::downstream_input_and_resize_backpressure_is_retried_in_order,
        crate::terminal_runtime::controller::tests::drop_is_nonblocking_while_explicit_shutdown_joins,
        crate::terminal_runtime::controller::tests::label_and_close_publish_new_revisions,
        crate::terminal_runtime::controller::tests::replay_frames_and_exit_are_published_without_ui_runtime_calls,
        crate::terminal_runtime::controller::tests::round_robin_keeps_a_quiet_pane_moving_during_noisy_output,
        crate::terminal_runtime::controller::tests::scroll_publishes_a_new_immutable_frame_without_transport_output,
        crate::terminal_runtime::controller::tests::stale_epoch_commands_cannot_mutate_a_reopened_pane,
        crate::terminal_runtime::controller::tests::transport_failure_is_published_with_the_last_frame,
    );
}

#[test]
fn terminal_runtime_input_suite() {
    run_cases!(
        crate::terminal_runtime::input::tests::characters_control_and_alt_are_encoded_without_shell_interpretation,
        crate::terminal_runtime::input::tests::cursor_and_function_keys_honor_modes_and_modifiers,
        crate::terminal_runtime::input::tests::editing_keys_and_bracketed_paste_match_xterm_sequences,
        crate::terminal_runtime::input::tests::sgr_mouse_uses_inner_relative_coordinates_and_filters_borders,
    );
}

#[test]
fn terminal_runtime_live_pane_suite() {
    run_cases!(
        crate::terminal_runtime::live_pane::tests::close_removes_engine_metadata_and_transport_together,
        crate::terminal_runtime::live_pane::tests::drain_panic_becomes_a_stable_failure_and_rejects_new_commands,
        crate::terminal_runtime::live_pane::tests::duplicate_and_out_of_order_resize_acks_fail_deterministically,
        crate::terminal_runtime::live_pane::tests::exit_is_stored_without_removing_the_pane,
        crate::terminal_runtime::live_pane::tests::final_output_is_applied_before_transport_failure_surfaces,
        crate::terminal_runtime::live_pane::tests::host_title_and_bell_events_are_observable,
        crate::terminal_runtime::live_pane::tests::input_and_resize_are_forwarded_while_the_engine_is_resized,
        crate::terminal_runtime::live_pane::tests::mismatch_duplicate_and_missing_pane_errors_do_not_change_state,
        crate::terminal_runtime::live_pane::tests::parser_replies_are_routed_back_to_transport_in_order,
        crate::terminal_runtime::live_pane::tests::parser_reply_survives_a_full_command_queue,
        crate::terminal_runtime::live_pane::tests::pump_has_a_fixed_event_budget_and_coalesces_output,
        crate::terminal_runtime::live_pane::tests::repeated_title_and_bell_events_are_coalesced,
        crate::terminal_runtime::live_pane::tests::replay_mismatch_surfaces_worker_error_and_keeps_pane_accessible,
        crate::terminal_runtime::live_pane::tests::replay_output_is_pumped_into_the_terminal_snapshot_in_order,
        crate::terminal_runtime::live_pane::tests::resize_ack_orders_old_and_new_output_around_engine_resize,
        crate::terminal_runtime::live_pane::tests::saturated_and_disconnected_command_queues_return_without_blocking,
        crate::terminal_runtime::live_pane::tests::successful_completion_without_exit_event_is_still_observable,
    );
}

#[test]
fn terminal_runtime_native_pty_suite() {
    run_cases!(
        crate::terminal_runtime::native_pty::tests::blocked_large_input_does_not_starve_shutdown,
        crate::terminal_runtime::native_pty::tests::default_program_rejects_arguments_without_panicking,
        crate::terminal_runtime::native_pty::tests::dropping_handle_reaps_the_owned_process,
        crate::terminal_runtime::native_pty::tests::full_single_slot_output_queue_does_not_starve_shutdown,
        crate::terminal_runtime::native_pty::tests::native_pty_preserves_binary_output_and_explicit_env_removal,
        crate::terminal_runtime::native_pty::tests::native_pty_preserves_io_resize_env_and_exit,
        crate::terminal_runtime::native_pty::tests::native_pty_shutdown_terminates_the_owned_child,
        crate::terminal_runtime::native_pty::tests::shutdown_escalates_past_ignored_hup_and_term,
        crate::terminal_runtime::native_pty::tests::spawn_failure_is_reported_through_completion,
        crate::terminal_runtime::native_pty::tests::unix_pty_eio_is_treated_as_end_of_stream,
    );
}

#[test]
fn terminal_runtime_pane_suite() {
    run_cases!(
        crate::terminal_runtime::pane::tests::close_panic_removes_metadata_and_allows_reopen,
        crate::terminal_runtime::pane::tests::close_removes_metadata_and_terminal_engine_together,
        crate::terminal_runtime::pane::tests::duplicate_pane_keeps_original_metadata,
        crate::terminal_runtime::pane::tests::pane_metadata_can_change_without_recreating_engine,
        crate::terminal_runtime::pane::tests::pane_scroll_updates_only_its_immutable_frame_viewport,
    );
}

#[test]
fn terminal_runtime_stress_tests_suite() {
    run_cases!(
        crate::terminal_runtime::stress_tests::eight_panes_process_output_resize_snapshot_and_close_concurrently,
    );
}

#[test]
fn terminal_runtime_tests_suite() {
    run_cases!(
        crate::terminal_runtime::tests::alacritty_engine_emits_dsr_and_device_attribute_replies,
        crate::terminal_runtime::tests::alacritty_engine_emits_title_reset_bell_and_explicit_unsupported_requests,
        crate::terminal_runtime::tests::alacritty_engine_keeps_wide_wrap_placeholders_textless,
        crate::terminal_runtime::tests::alacritty_engine_normalizes_zero_dimensions_at_boundaries,
        crate::terminal_runtime::tests::alacritty_engine_parses_ansi_and_resize,
        crate::terminal_runtime::tests::alacritty_engine_preserves_ansi_colors_and_attributes,
        crate::terminal_runtime::tests::alacritty_engine_preserves_wide_and_combining_cells,
        crate::terminal_runtime::tests::alacritty_engine_reflows_content_across_resize,
        crate::terminal_runtime::tests::alacritty_engine_reports_character_size_without_fabricating_pixel_size,
        crate::terminal_runtime::tests::alacritty_engine_restores_primary_screen_and_cursor_after_alt_screen,
        crate::terminal_runtime::tests::alacritty_engine_snapshots_scrolled_output_in_row_major_order,
        crate::terminal_runtime::tests::alacritty_engine_tracks_cursor_shape_visibility_position_and_modes,
        crate::terminal_runtime::tests::alacritty_engine_tracks_tui_modes_and_unicode,
        crate::terminal_runtime::tests::pane_runtime_keeps_label_outside_terminal_engine,
        crate::terminal_runtime::tests::terminal_snapshot_row_text_only_trims_empty_ascii_cells,
        crate::terminal_runtime::tests::worker_runtime_drains_engine_events_in_parser_order,
        crate::terminal_runtime::tests::worker_runtime_supports_multiple_registered_engines,
    );
}

#[test]
fn terminal_runtime_transport_suite() {
    run_cases!(
        crate::terminal_runtime::transport::tests::bounded_queues_apply_backpressure_without_reordering,
        crate::terminal_runtime::transport::tests::cloned_recording_replays_identically,
        crate::terminal_runtime::transport::tests::commands_gate_later_events_in_recorded_order,
        crate::terminal_runtime::transport::tests::disconnected_command_sender_is_reported,
        crate::terminal_runtime::transport::tests::disconnected_event_receiver_is_reported,
        crate::terminal_runtime::transport::tests::exit_must_be_the_final_step_and_is_not_partially_replayed,
        crate::terminal_runtime::transport::tests::mismatched_command_fails_at_the_exact_step,
        crate::terminal_runtime::transport::tests::output_recording_preserves_chunks_and_exit,
    );
}

#[test]
fn terminal_runtime_transport_runtime_suite() {
    run_cases!(
        crate::terminal_runtime::transport_runtime::tests::bounded_event_queue_applies_backpressure_and_keeps_order,
        crate::terminal_runtime::transport_runtime::tests::disconnecting_event_receiver_releases_replay_sender,
        crate::terminal_runtime::transport_runtime::tests::drain_events_returns_all_buffered_events_in_order,
        crate::terminal_runtime::transport_runtime::tests::drop_disconnects_a_full_event_queue_without_waiting,
        crate::terminal_runtime::transport_runtime::tests::drop_disconnects_a_worker_waiting_for_more_commands,
        crate::terminal_runtime::transport_runtime::tests::panic_is_converted_to_a_worker_error,
        crate::terminal_runtime::transport_runtime::tests::replay_preserves_bidirectional_order_and_graceful_shutdown,
        crate::terminal_runtime::transport_runtime::tests::runtime_rejects_unbounded_configuration,
        crate::terminal_runtime::transport_runtime::tests::shutdown_does_not_block_when_command_and_event_queues_are_full,
        crate::terminal_runtime::transport_runtime::tests::worker_error_and_completion_observation_are_repeatable,
        crate::terminal_runtime::transport_runtime::tests::worker_name_is_safe_and_visible_inside_transport,
    );
}

#[test]
fn terminal_runtime_widget_suite() {
    run_cases!(
        crate::terminal_runtime::widget::tests::cursor_modifier_preserves_the_cells_existing_style,
        crate::terminal_runtime::widget::tests::terminal_attributes_map_to_ratatui_modifiers,
        crate::terminal_runtime::widget::tests::underline_cursor_uses_underline_fallback,
        crate::terminal_runtime::widget::tests::widget_clips_snapshot_to_inner_area,
        crate::terminal_runtime::widget::tests::widget_does_not_render_wide_grapheme_across_right_border,
        crate::terminal_runtime::widget::tests::widget_renders_label_terminal_cells_and_cursor,
    );
}

#[test]
fn terminal_runtime_worker_suite() {
    run_cases!(
        crate::terminal_runtime::worker::tests::bounded_queue_preserves_order_and_close_waits_for_destruction,
        crate::terminal_runtime::worker::tests::engine_panics_remove_only_the_faulting_pane_and_keep_the_shard_alive,
        crate::terminal_runtime::worker::tests::feed_resize_and_scroll_propagate_engine_and_missing_pane_errors,
        crate::terminal_runtime::worker::tests::separate_shards_make_progress_independently,
        crate::terminal_runtime::worker::tests::shutdown_drains_commands_already_accepted_by_a_full_queue,
    );
}

#[test]
fn terminal_workspace_tests_suite() {
    run_cases!(
        crate::terminal_workspace::tests::invalid_or_future_workspace_is_rejected,
        crate::terminal_workspace::tests::invalid_workspace_is_quarantined_without_overwriting_recovery_files,
        crate::terminal_workspace::tests::missing_workspace_is_not_an_error,
        crate::terminal_workspace::tests::workspace_round_trips_without_runtime_state,
    );
}
