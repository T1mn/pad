mod model {
    use std::path::PathBuf;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct GrokThreadRef {
        pub session_id: String,
        pub cwd: PathBuf,
        pub updated_at: i64,
        pub transcript_path: PathBuf,
        pub title: Option<String>,
        pub model_name: Option<String>,
    }
}
mod scan {
    use super::model::GrokThreadRef;
    use serde::Deserialize;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::time::UNIX_EPOCH;

    #[derive(Deserialize)]
    struct Summary {
        info: Info,
        #[serde(default)]
        session_summary: String,
        generated_title: Option<String>,
        current_model_id: Option<String>,
    }

    #[derive(Deserialize)]
    struct Info {
        id: String,
        cwd: String,
    }

    pub(super) fn all_threads_at(root: &Path) -> io::Result<Vec<GrokThreadRef>> {
        if !root.exists() {
            return Ok(Vec::new());
        }

        let mut threads = Vec::new();
        for summary_path in summary_files(root)? {
            match parse_summary(&summary_path) {
                Ok(thread) => threads.push(thread),
                Err(err) => crate::log_debug!(
                    "grok_history: skip summary path={} err={}",
                    summary_path.display(),
                    err
                ),
            }
        }
        threads.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| right.session_id.cmp(&left.session_id))
        });
        Ok(threads)
    }

    fn summary_files(root: &Path) -> io::Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        for project in fs::read_dir(root)? {
            let project = match project {
                Ok(entry) if entry.path().is_dir() => entry,
                _ => continue,
            };
            for session in match fs::read_dir(project.path()) {
                Ok(entries) => entries,
                Err(_) => continue,
            } {
                let Ok(session) = session else { continue };
                let summary = session.path().join("summary.json");
                if summary.is_file() {
                    files.push(summary);
                }
            }
        }
        Ok(files)
    }

    fn parse_summary(path: &Path) -> io::Result<GrokThreadRef> {
        let body = fs::read_to_string(path)?;
        let summary: Summary = serde_json::from_str(&body).map_err(io::Error::other)?;
        let transcript_path = path
            .parent()
            .ok_or_else(|| io::Error::other("summary has no session directory"))?
            .join("updates.jsonl");
        if !transcript_path.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "updates.jsonl not found",
            ));
        }
        let updated_at = fs::metadata(&transcript_path)
            .and_then(|metadata| metadata.modified())
            .and_then(|modified| {
                modified
                    .duration_since(UNIX_EPOCH)
                    .map_err(io::Error::other)
            })?
            .as_secs() as i64;
        let title = summary
            .generated_title
            .and_then(non_empty)
            .or_else(|| non_empty(summary.session_summary));

        Ok(GrokThreadRef {
            session_id: summary.info.id,
            cwd: PathBuf::from(summary.info.cwd),
            updated_at,
            transcript_path,
            title,
            model_name: summary.current_model_id.and_then(non_empty),
        })
    }

    fn non_empty(value: String) -> Option<String> {
        (!value.trim().is_empty()).then_some(value)
    }
}

use std::io;
use std::path::PathBuf;

pub use model::GrokThreadRef;

pub fn all_threads() -> io::Result<Vec<GrokThreadRef>> {
    scan::all_threads_at(&sessions_root()?)
}

pub fn thread_for_id(session_id: &str) -> io::Result<Option<GrokThreadRef>> {
    Ok(all_threads()?
        .into_iter()
        .find(|thread| thread.session_id == session_id))
}

fn sessions_root() -> io::Result<PathBuf> {
    if let Some(home) = std::env::var_os("GROK_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(home).join("sessions"));
    }
    dirs::home_dir()
        .map(|home| home.join(".grok").join("sessions"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory not found"))
}

#[cfg(test)]
mod tests {
    use super::scan::all_threads_at;
    use std::fs;

    #[test]
    fn scans_official_summary_and_skips_corrupt_sessions() {
        let root = crate::test_support::temp_path("pad", "grok-history");
        let valid = root.join("encoded-cwd").join("session-1");
        fs::create_dir_all(&valid).unwrap();
        fs::write(
            valid.join("summary.json"),
            r#"{
              "info":{"id":"session-1","cwd":"/tmp/project"},
              "session_summary":"fallback",
              "generated_title":"Latest title",
              "created_at":"2026-07-17T00:00:00Z",
              "updated_at":"2026-07-17T00:01:00Z",
              "current_model_id":"grok-code-fast-1",
              "future_field":{"safe":true}
            }"#,
        )
        .unwrap();
        fs::write(valid.join("updates.jsonl"), "{}\n").unwrap();

        let corrupt = root.join("encoded-cwd").join("bad");
        fs::create_dir_all(&corrupt).unwrap();
        fs::write(corrupt.join("summary.json"), "{not json").unwrap();
        fs::write(corrupt.join("updates.jsonl"), "{}\n").unwrap();

        let threads = all_threads_at(&root).unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].session_id, "session-1");
        assert_eq!(threads[0].title.as_deref(), Some("Latest title"));
        assert_eq!(threads[0].model_name.as_deref(), Some("grok-code-fast-1"));
        assert_eq!(threads[0].cwd.to_string_lossy(), "/tmp/project");

        let _ = fs::remove_dir_all(root);
    }
}
