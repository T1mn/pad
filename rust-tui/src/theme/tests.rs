mod config;
mod palette {
    use super::super::*;
    use ratatui::style::Color;

    #[test]
    fn readability_boost_keeps_status_text_close_to_primary_fg() {
        let theme = Theme::by_name("catppuccin");
        assert_eq!(theme.status_fg, theme.fg);
    }

    #[test]
    fn readability_boost_lifts_comment_contrast() {
        let boosted = Theme::by_name("one-dark");
        assert_ne!(boosted.comment, Color::Rgb(92, 99, 112));
    }
}
mod persist;
mod provider {
    use super::super::*;

    #[test]
    fn codex_base_url_candidates_try_root_and_v1_variants() {
        assert_eq!(
            codex_api_base_candidates("https://relay.example"),
            vec![
                "https://relay.example".to_string(),
                "https://relay.example/v1".to_string()
            ]
        );
        assert_eq!(
            codex_api_base_candidates("https://relay.example/v1"),
            vec![
                "https://relay.example/v1".to_string(),
                "https://relay.example".to_string()
            ]
        );
        assert_eq!(
            codex_api_base_candidates("https://relay.example/openai/v1"),
            vec!["https://relay.example/openai/v1".to_string()]
        );
    }

    #[test]
    fn codex_base_url_prefers_v1_for_root_inputs() {
        assert_eq!(
            provider::codex_preferred_api_base_url("https://relay.example"),
            "https://relay.example/v1"
        );
        assert_eq!(
            provider::codex_preferred_api_base_url("https://relay.example/"),
            "https://relay.example/v1"
        );
        assert_eq!(
            provider::codex_preferred_api_base_url("https://relay.example/v1"),
            "https://relay.example/v1"
        );
        assert_eq!(
            provider::codex_preferred_api_base_url("https://relay.example/openai/v1"),
            "https://relay.example/openai/v1"
        );
    }
}
mod sound {
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
            config.save().expect("save config");

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
}
mod support {
    pub(super) fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
        crate::test_support::with_temp_home("pad-theme", name, |_| f())
    }
}
