use super::super::App;
use crate::theme::Config;

fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
    crate::test_support::with_temp_home("pad-config-persist", name, |_| f())
}

pub(crate) fn save_config_succeeds_without_warning_toast() {
    with_temp_home("save-ok", || {
        let mut app = App::new();
        app.config.theme = "gruvbox".to_string();

        assert!(app.save_config());
        assert!(app.preview.copy_toast.is_none());
        assert_eq!(Config::load().theme, "gruvbox");
    });
}

pub(crate) fn save_config_surfaces_failure_instead_of_swallowing_it() {
    with_temp_home("save-fail", || {
        let mut app = App::new();
        // 目标路径被目录占住，rename 必然失败：以前的 `let _ = fs::write(..)` 会静默丢改动。
        std::fs::create_dir_all(Config::config_path()).expect("occupy config path");

        assert!(!app.save_config());
        let toast = app.preview.copy_toast.as_ref().expect("failure toast");
        assert!(
            toast.title.contains("失败") || toast.title.contains("failed"),
            "unexpected toast title: {}",
            toast.title
        );
    });
}

pub(crate) fn panel_width_save_failure_is_not_overwritten_by_success_toast() {
    with_temp_home("panel-width-save-fail", || {
        let mut app = App::new();
        std::fs::create_dir_all(Config::config_path()).expect("occupy config path");

        app.widen_agent_panel_width(40);

        let toast = app.preview.copy_toast.as_ref().expect("failure toast");
        assert!(
            toast.title.contains("失败") || toast.title.contains("failed"),
            "save failure must remain visible, got: {}",
            toast.title
        );
    });
}

pub(crate) fn broken_config_reports_recovery_to_the_caller() {
    with_temp_home("recovery-notice", || {
        let path = Config::config_path();
        std::fs::create_dir_all(path.parent().expect("config parent")).expect("create config dir");
        std::fs::write(&path, "theme = \"broken").expect("write broken config");

        let report = Config::load_reported();
        let recovery = report.recovery.expect("recovery report");
        assert_eq!(recovery.backup, Some(path.with_extension("toml.bak")));

        let mut app = App::new();
        app.notify_config_recovery(&recovery);
        assert!(app.preview.copy_toast.is_some());
    });
}
