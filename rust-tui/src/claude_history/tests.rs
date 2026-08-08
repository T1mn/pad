mod archive;
mod config_dir {
    use super::support::write_thread;

    #[test]
    fn default_history_scan_follows_claude_config_dir() {
        crate::test_support::with_temp_home("pad-claude-history", "config-dir", |home| {
            let config_dir = home.join("custom-claude");
            let transcript = config_dir.join("projects/demo/session-config.jsonl");
            std::fs::create_dir_all(transcript.parent().expect("transcript parent"))
                .expect("create transcript parent");
            write_thread(
                &transcript,
                "session-config",
                "/tmp/project",
                "custom config",
                "2099-03-10T05:41:54.280Z",
            );

            let previous = std::env::var_os("CLAUDE_CONFIG_DIR");
            std::env::set_var("CLAUDE_CONFIG_DIR", &config_dir);
            let threads = super::super::api::all_threads().expect("scan Claude history");
            restore_config_dir(previous);

            assert_eq!(threads.len(), 1);
            assert_eq!(threads[0].session_id, "session-config");
            assert_eq!(threads[0].transcript_path, transcript);
        });
    }

    fn restore_config_dir(previous: Option<std::ffi::OsString>) {
        if let Some(previous) = previous {
            std::env::set_var("CLAUDE_CONFIG_DIR", previous);
        } else {
            std::env::remove_var("CLAUDE_CONFIG_DIR");
        }
    }
}
mod parse;
mod support {
    use std::fs;
    use std::path::Path;

    pub(super) fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = crate::test_support::temp_path("pad-claude-history", name);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    pub(super) fn temp_db(name: &str) -> std::path::PathBuf {
        temp_dir(name).join("claude.sqlite")
    }

    pub(super) fn write_thread(
        file: &Path,
        session_id: &str,
        cwd: &str,
        title: &str,
        assistant_ts: &str,
    ) {
        fs::write(
            file,
            format!(
                concat!(
                    "{{\"type\":\"user\",\"sessionId\":\"{}\",\"cwd\":\"{}\",\"message\":{{\"role\":\"user\",\"content\":\"{}\"}}}}\n",
                    "{{\"type\":\"assistant\",\"sessionId\":\"{}\",\"cwd\":\"{}\",\"message\":{{\"role\":\"assistant\",\"content\":\"ok\"}},\"timestamp\":\"{}\"}}\n"
                ),
                session_id, cwd, title, session_id, cwd, assistant_ts
            ),
        )
        .unwrap();
    }
}
mod sync;
