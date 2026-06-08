use super::super::handle_settings_mode;
use super::support::with_temp_home;
use crate::app::state::{Mode, SettingsDetailKind, SettingsFocus};
use crate::app::App;
use crossterm::event::KeyCode;

#[test]
fn sound_settings_toggle_cycle_and_preview_work() {
    with_temp_home("sound-settings", || {
        let mut app = App::new();
        app.mode = Mode::Settings;
        app.settings_open = true;
        app.settings_focus = SettingsFocus::Detail;
        app.active_settings_detail = Some(SettingsDetailKind::Sound);
        app.sound_settings_selected = 0;
        crate::sound::with_test_sound_capture(|| {
            let _ = crate::sound::take_test_playbacks();

            handle_settings_mode(&mut app, KeyCode::Enter);
            assert!(!app.config.sound.enabled);

            handle_settings_mode(&mut app, KeyCode::Down);
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert!(!app.config.sound.completion.enabled);

            handle_settings_mode(&mut app, KeyCode::Down);
            let original = app.config.sound.completion.preset.clone();
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert_ne!(app.config.sound.completion.preset, original);

            let cycled = app.config.sound.completion.preset.clone();
            handle_settings_mode(&mut app, KeyCode::Char(' '));
            assert_eq!(app.config.sound.completion.preset, cycled);
            assert_eq!(
                crate::sound::take_test_playbacks(),
                vec![crate::sound::TestPlayback {
                    event: None,
                    preset: cycled,
                }]
            );
        });
    });
}
