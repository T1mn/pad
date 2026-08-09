//! Generated domain suites; individual case bodies remain beside their implementation.

#[test]
fn event_input_clear_suite() {
    run_cases!(
        crate::event::input_clear::input_clear_tests::plain_delete_does_not_clear_search,
        crate::event::input_clear::input_clear_tests::shift_delete_clears_edit_buffers,
        crate::event::input_clear::input_clear_tests::shift_delete_clears_panel_search,
        crate::event::input_clear::input_clear_tests::shift_delete_clears_settings_search_and_resets_selection,
        crate::event::input_clear::input_clear_tests::shift_delete_clears_tree_search_without_leaving_search_mode,
    );
}

#[test]
fn event_modes_suite() {
    run_cases!(
        crate::event::modes::agent_launcher::tests::native_agent_labels_are_human_readable,
        crate::event::modes::agent_launcher::tests::native_launcher_opens_selected_agent_in_a_real_terminal_tab,
        crate::event::modes::notification_inbox::tests::escape_closes_inbox,
        crate::event::modes::relay_settings::tests::navigation::relay_escape_from_settings_host_steps_back_by_level,
        crate::event::modes::relay_settings::tests::navigation::relay_escape_from_standalone_provider_list_returns_to_agent_list,
        crate::event::modes::relay_settings::tests::opencode::opencode_model_id_edit_is_uniquified_and_updates_model_refs,
        crate::event::modes::relay_settings::tests::opencode::opencode_model_popup_supports_add_edit_and_delete_flow,
        crate::event::modes::relay_settings::tests::opencode::opencode_provider_key_edit_is_uniquified_and_updates_model_refs,
        crate::event::modes::relay_settings::tests::opencode::opencode_small_model_picker_can_clear_selection,
        crate::event::modes::relay_settings::tests::provider::provider_toggle_updates_active_provider_and_persists_overlay,
        crate::event::modes::settings::codex::tests::actions::runtime_category_preserves_original_switches_and_web_search_cycle,
        crate::event::modes::settings::codex::tests::actions::status_prompt_and_preview_categories_preserve_original_toggles,
        crate::event::modes::settings::codex::tests::navigation::cli_category_keeps_check_update_actions_separate_from_config_toggles,
        crate::event::modes::settings::codex::tests::navigation::root_groups_options_and_back_navigation,
        crate::event::modes::settings::tests::appearance::language_detail_escape_restores_locale_and_enter_persists_selection,
        crate::event::modes::settings::tests::appearance::theme_detail_escape_restores_preview_and_enter_applies_selection,
        crate::event::modes::settings::tests::search::settings_f1_closes_modal_from_detail_view,
        crate::event::modes::settings::tests::search::settings_numeric_shortcuts_and_detail_search_shortcut_keep_current_behavior,
        crate::event::modes::settings::tests::search::settings_search_allows_arrow_navigation_with_filtered_results,
        crate::event::modes::settings::tests::search::settings_search_allows_arrow_navigation_without_query,
        crate::event::modes::settings::tests::search::settings_search_can_route_to_codex_relay_agent,
        crate::event::modes::settings::tests::search::settings_search_can_route_to_codex_settings_subpage,
        crate::event::modes::settings::tests::search::settings_search_enter_opens_first_match_directly,
        crate::event::modes::settings::tests::search::settings_search_enter_with_no_match_stays_in_list,
        crate::event::modes::settings::tests::sound::sound_settings_toggle_cycle_and_preview_work,
        crate::event::modes::settings::tests::telegram::telegram_settings_edit_escape_discards_buffer_and_r_keeps_detail_open,
        crate::event::modes::settings::tests::telegram::telegram_settings_toggle_and_edit_fields_persist_without_leaving_detail,
        crate::event::modes::thread_action_confirm::tests::plain_delete_does_not_clear_thread_meta_buffer,
        crate::event::modes::thread_action_confirm::tests::shift_delete_clears_thread_meta_buffer,
    );
}

#[test]
fn event_mouse_pipeline_suite() {
    run_cases!(
        crate::event::mouse_pipeline::tests::non_wheel_events_do_not_enter_scroll_routing,
        crate::event::mouse_pipeline::tests::shift_wheel_forces_pad_scrollback,
        crate::event::mouse_pipeline::tests::wheel_routes_to_child_only_while_mouse_reporting_is_active,
    );
}

#[test]
fn event_normal_suite() {
    run_cases!(
        crate::event::normal::global_keys::primary::tests::c_keeps_opening_the_global_index_in_native_mode,
        crate::event::normal::terminal_keys::tests::command_actions_cover_layout_profiles_and_navigation,
        crate::event::normal::terminal_keys::tests::command_layer_accepts_explicit_prefixes,
        crate::event::normal::terminal_keys::tests::shift_navigation_keys_control_pad_scrollback,
    );
}

#[test]
fn event_tests_suite() {
    run_cases!(
        crate::event::tests::mouse_tests::mouse_click_on_panel_row_accounts_for_scroll_offset,
        crate::event::tests::mouse_tests::mouse_click_on_panel_row_selects_it_and_focuses_panel,
        crate::event::tests::mouse_tests::mouse_click_on_second_thread_row_selects_the_second_thread,
        crate::event::tests::mouse_tests::mouse_click_on_session_gap_does_not_change_selection,
        crate::event::tests::mouse_tests::mouse_click_on_session_turn_selects_then_expands_on_repeat,
        crate::event::tests::mouse_tests::mouse_wheel_over_preview_scrolls_and_focuses_preview,
        crate::event::tests::preview_tab::double_tab_from_detail_restores_session_list_and_keeps_panel_focus,
        crate::event::tests::preview_tab::single_tab_from_detail_keeps_current_behavior_and_focuses_panel,
        crate::event::tests::sidebar_keys::double_space_collapses_all_folders_when_any_are_expanded,
        crate::event::tests::sidebar_keys::double_space_expands_all_folders_when_none_are_expanded,
        crate::event::tests::sidebar_keys::enter_on_native_agent_thread_focuses_its_terminal_tab,
        crate::event::tests::sidebar_keys::enter_on_stale_external_live_entry_shows_native_mode_notice,
        crate::event::tests::sidebar_keys::j_k_skip_expanded_folder_rows,
        crate::event::tests::sidebar_keys::numeric_jump_targets_visible_threads_only,
        crate::event::tests::sidebar_keys::space_on_selected_thread_collapses_parent_folder,
    );
}

#[test]
fn ui_layout_suite() {
    run_cases!(
        crate::ui::layout::tests::normal_layout_allows_wider_agents_panel_on_large_terminals,
        crate::ui::layout::tests::normal_layout_keeps_preview_space_on_medium_terminals,
    );
}

#[test]
fn ui_modals_suite() {
    run_cases!(
        crate::ui::modals::common::tests::mask_secret_prefix_cuts_multibyte_secret_on_char_boundary,
        crate::ui::modals::common::tests::mask_secret_prefix_keeps_ascii_behavior,
        crate::ui::modals::common::tests::trailing_chars_handles_zero_count,
        crate::ui::modals::common::tests::truncate_modal_line_middle_handles_short_width,
        crate::ui::modals::common::tests::truncate_modal_line_middle_keeps_existing_shape,
        crate::ui::modals::dialogs::thread_action::tests::thread_action_body_includes_warning_block,
        crate::ui::modals::dialogs::thread_action::tests::thread_action_body_keeps_blank_lines_without_warning,
        crate::ui::modals::relay::detail::opencode::tests::opencode_models_summary_counts_remaining_models,
        crate::ui::modals::relay::detail::opencode::tests::opencode_models_summary_formats_first_two_models,
        crate::ui::modals::relay::list::provider::tests::provider_subtitle_falls_back_when_empty,
        crate::ui::modals::relay::list::provider::tests::provider_subtitle_keeps_opencode_summary_shape,
        crate::ui::modals::settings::tests::settings_selection_keyword_includes_english_aliases,
        crate::ui::modals::telegram::values::tests::mask_secret_keeps_existing_empty_and_short_behavior,
        crate::ui::modals::telegram::values::tests::mask_secret_keeps_first_and_last_four_for_long_values,
    );
}

#[test]
fn ui_panel_list_suite() {
    run_cases!(
        crate::ui::panel_list::tests::folder_row::folder_count_uses_accent_without_dim,
        crate::ui::panel_list::tests::folder_row::folder_label_uses_readable_text_without_dim,
        crate::ui::panel_list::tests::shimmer_preserves_text_content,
        crate::ui::panel_list::tests::thread_row::jump_badge_is_fixed_width_and_limited_to_nine,
        crate::ui::panel_list::tests::viewport_tests::fills_from_top_when_selection_is_near_start,
        crate::ui::panel_list::tests::viewport_tests::keeps_selected_near_middle_when_possible,
        crate::ui::panel_list::tests::viewport_tests::respects_tall_thread_rows,
        crate::ui::panel_list::tests::visible_thread_count_ignores_folder_rows,
        crate::ui::panel_list::tests::visible_thread_jump_badges_ignore_folders_and_cap_at_nine,
        crate::ui::panel_list::tests::waiting_threads_do_not_breathe,
        crate::ui::panel_list::tests::width::manual_width_is_used_as_minimum,
        crate::ui::panel_list::tests::width::preferred_panel_width_cache_clears_on_sidebar_invalidation,
        crate::ui::panel_list::tests::width::preferred_panel_width_keeps_short_name_visible,
        crate::ui::panel_list::tests::width::thread_width_grows_with_long_titles,
    );
}

#[test]
fn ui_preview_suite() {
    run_cases!(
        crate::ui::preview::layout::info_card::values::tests::shortened_thread_path_uses_last_two_segments_without_vec,
        crate::ui::preview::layout::selection::tests::preview_plain_visible_rows_respects_scroll_window_after_wrapping,
        crate::ui::preview::layout::selection::tests::preview_selection_text_preserves_multiline_range,
        crate::ui::preview::layout::tests::preview_info_value_hit_test_returns_full_truncated_value,
        crate::ui::preview::layout::tests::preview_provider_value_falls_back_to_active_provider_without_session_binding,
        crate::ui::preview::layout::tests::preview_provider_value_prefers_session_bound_provider,
        crate::ui::preview::markdown::tests::inline::format_line_detects_error_case_insensitively,
        crate::ui::preview::markdown::tests::inline::format_line_detects_success_case_insensitively,
        crate::ui::preview::markdown::tests::normalize::inserts_paragraph_gaps_between_plain_lines,
        crate::ui::preview::markdown::tests::normalize::keeps_fenced_code_lines_together,
        crate::ui::preview::plain::tests::ensure_plain_preview_cache_reuses_existing_cache_when_context_is_unchanged,
        crate::ui::preview::plain::window::tests::visible_plain_line_window_keeps_only_rows_needed_for_viewport,
        crate::ui::preview::plain::window::tests::visible_plain_line_window_starts_inside_wrapped_line,
        crate::ui::preview::session::tests::gap_line_has_no_turn_hit_target,
        crate::ui::preview::session::tests::selected_range_excludes_gap_line,
        crate::ui::preview::session::tests::session_card_renders_two_answer_lines,
        crate::ui::preview::session::tests::session_detail_prompt_uses_primary_text_color,
        crate::ui::preview::session_list_cache::tests::cache_keeps_turn_allocation_for_fast_hits,
    );
}

#[test]
fn ui_selection_suite() {
    run_cases!(
        crate::ui::selection::model::tests::matches_query_checks_ascii_case_without_lowercase_copy,
        crate::ui::selection::model::tests::matches_query_checks_value_text,
        crate::ui::selection::model::tests::matches_query_keeps_unicode_case_fold_behavior,
        crate::ui::selection::row::tests::title_line_marks_selection_and_right_aligns_value,
        crate::ui::selection::row::tests::unselected_value_uses_accent_color,
    );
}

#[test]
fn ui_status_bar_suite() {
    run_cases!(
        crate::ui::status_bar::tests::normal_status_hides_selected_panel_identity_details,
        crate::ui::status_bar::tests::status_remainder_preserves_right_hint_with_mode_badge_width,
    );
}

#[test]
fn ui_terminal_suite() {
    run_cases!(
        crate::ui::terminal::tests::four_panes_tile_odd_area_without_gaps_or_overlap,
        crate::ui::terminal::tests::placeholder_uses_focus_border_and_warning_text,
        crate::ui::terminal::tests::recursive_splits_keep_depth_first_pane_order,
        crate::ui::terminal::tests::single_pane_reserves_one_row_for_tabs,
        crate::ui::terminal::tests::stored_split_ratio_is_applied_with_saturating_remainder,
        crate::ui::terminal::tests::tab_bar_marks_only_the_active_tab,
        crate::ui::terminal::tests::tab_hit_rects_match_unicode_width_and_visible_clipping,
        crate::ui::terminal::tests::tiny_rectangles_saturate_instead_of_underflowing,
        crate::ui::terminal::tests::two_columns_give_the_odd_cell_to_the_second_pane,
    );
}
