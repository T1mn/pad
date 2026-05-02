use super::*;

#[test]
fn chunk_text_splits_long_messages() {
    let chunks = chunk_text("abcdef", 3);
    assert_eq!(chunks, vec!["abc", "def"]);
}
#[test]
fn slash_command_builder_preserves_optional_args() {
    assert_eq!(build_slash_command_text("/status", ""), "/status");
    assert_eq!(build_slash_command_text("/fast", "status"), "/fast status");
}
#[test]
fn summarize_pane_capture_trims_blank_edges_and_keeps_tail() {
    let capture = "\n\none\n\ntwo\nthree\n\n";
    assert_eq!(summarize_pane_capture(capture), "one\n\ntwo\nthree");
}
#[test]
fn agent_keyboard_uses_clickable_use_callbacks() {
    let panel = sample_panel_with_turns(Vec::new());
    let keyboard = build_agent_keyboard(&[panel], crate::i18n::Locale::En);
    assert_eq!(keyboard.len(), 1);
    assert_eq!(keyboard[0][0]["callback_data"], "use-pane:%42");
}
#[test]
fn telegram_sound_helper_records_enabled_event() {
    with_temp_home("telegram-sound", |_home| {
        crate::sound::with_test_sound_capture(|| {
            let _ = crate::sound::take_test_playbacks();
            let mut config = crate::theme::Config::default();
            config.sound.approval.enabled = true;

            play_sound_event(&config, crate::sound::SoundEvent::Approval);

            assert_eq!(
                crate::sound::take_test_playbacks(),
                vec![crate::sound::TestPlayback {
                    event: Some(crate::sound::SoundEvent::Approval),
                    preset: "ping".into(),
                }]
            );
        });
    });
}
