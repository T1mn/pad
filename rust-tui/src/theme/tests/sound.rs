use super::super::*;
use super::support::with_temp_home;

#[test]
fn config_round_trips_sound_section() {
    with_temp_home("sound-roundtrip", || {
        let mut config = Config::default();
        config.sound.enabled = true;
        config.sound.completion.enabled = false;
        config.sound.completion.preset = "pop".into();
        config.sound.approval.enabled = true;
        config.sound.approval.preset = "glass".into();
        config.sound.timeout.enabled = true;
        config.sound.timeout.preset = "warning".into();
        config.sound.failure.enabled = true;
        config.sound.failure.preset = "alert".into();
        config.save();

        let loaded = Config::load();
        assert!(loaded.sound.enabled);
        assert!(!loaded.sound.completion.enabled);
        assert_eq!(loaded.sound.completion.preset, "pop");
        assert!(loaded.sound.approval.enabled);
        assert_eq!(loaded.sound.approval.preset, "glass");
        assert!(loaded.sound.timeout.enabled);
        assert_eq!(loaded.sound.timeout.preset, "warning");
        assert!(loaded.sound.failure.enabled);
        assert_eq!(loaded.sound.failure.preset, "alert");
    });
}

#[test]
fn config_normalizes_invalid_sound_presets() {
    with_temp_home("sound-preset-normalize", || {
        let path = Config::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create pad parent");
        }
        std::fs::write(
            &path,
            r#"[sound]
enabled = true

[sound.completion]
enabled = true
preset = "bogus"

[sound.approval]
enabled = true
preset = "also-bogus"
"#,
        )
        .expect("write config");

        let loaded = Config::load();
        assert_eq!(loaded.sound.completion.preset, "glass");
        assert_eq!(loaded.sound.approval.preset, "ping");
        assert_eq!(loaded.sound.timeout.preset, "warning");
        assert_eq!(loaded.sound.failure.preset, "alert");
    });
}
