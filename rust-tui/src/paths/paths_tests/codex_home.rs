use super::support::with_temp_home;
use super::*;

#[test]
fn ensure_pad_codex_home_layout_copies_config_to_profile_but_not_auth() {
    with_temp_home("pad-codex-profile-config", |home| {
        let canonical = home.join(".codex");
        fs::create_dir_all(&canonical).expect("create canonical codex home");
        fs::write(canonical.join("config.toml"), "model_provider = \"cpa\"\n")
            .expect("seed canonical config");
        fs::write(
            canonical.join("auth.json"),
            "{\"OPENAI_API_KEY\":\"sk-live\"}\n",
        )
        .expect("seed canonical auth");

        ensure_pad_codex_home_layout().expect("ensure pad codex home");

        assert_eq!(
            fs::read_to_string(pad_codex_config_path()).expect("read pad config"),
            "model_provider = \"cpa\"\n"
        );
        assert_eq!(
            pad_codex_config_path(),
            pad_codex_home_dir().join("pad.config.toml")
        );
        assert!(!pad_codex_auth_path().exists());
    });
}

#[test]
fn ensure_pad_codex_home_layout_does_not_create_session_or_db_links() {
    with_temp_home("pad-codex-profile-no-links", |_home| {
        ensure_pad_codex_home_layout().expect("ensure pad codex home");

        assert_eq!(
            pad_codex_config_path(),
            pad_codex_home_dir().join("pad.config.toml")
        );
        assert!(!pad_codex_home_dir().join("sessions").exists());
        assert!(!pad_codex_home_dir().join("state_5.sqlite").exists());
        assert!(!pad_codex_home_dir().join("state_5.sqlite-wal").exists());
    });
}

#[cfg(unix)]
#[test]
fn ensure_pad_codex_home_layout_unlinks_legacy_shared_state_symlinks() {
    with_temp_home("pad-codex-profile-unlink-legacy", |home| {
        use std::os::unix::fs::symlink;

        let canonical = home.join(".codex");
        let canonical_sessions = canonical.join("sessions");
        let canonical_archived = canonical.join("archived_sessions");
        let canonical_db = canonical.join("state_5.sqlite");
        fs::create_dir_all(&canonical_sessions).expect("create canonical sessions");
        fs::create_dir_all(&canonical_archived).expect("create canonical archived");
        fs::write(&canonical_db, "db").expect("write canonical db");

        fs::create_dir_all(pad_codex_home_dir()).expect("create pad codex home");
        symlink(&canonical_sessions, pad_codex_home_dir().join("sessions"))
            .expect("symlink sessions");
        symlink(
            &canonical_archived,
            pad_codex_home_dir().join("archived_sessions"),
        )
        .expect("symlink archived");
        symlink(&canonical_db, pad_codex_home_dir().join("state_5.sqlite")).expect("symlink db");

        ensure_pad_codex_home_layout().expect("ensure pad codex home");

        assert!(!pad_codex_home_dir().join("sessions").exists());
        assert!(!pad_codex_home_dir().join("archived_sessions").exists());
        assert!(!pad_codex_home_dir().join("state_5.sqlite").exists());
        assert!(canonical_sessions.is_dir());
        assert!(canonical_archived.is_dir());
        assert!(canonical_db.is_file());
    });
}
