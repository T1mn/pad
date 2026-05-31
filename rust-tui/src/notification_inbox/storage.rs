use super::model::{NotificationEntry, NotificationInbox, INBOX_VERSION};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static INBOX_IO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn load() -> NotificationInbox {
    load_from_path(&crate::paths::notification_inbox_path())
}

pub fn append(entry: NotificationEntry) -> io::Result<NotificationInbox> {
    mutate(|inbox| inbox.push(entry))
}

pub fn mark_read(id: &str) -> io::Result<bool> {
    let mut changed = false;
    mutate(|inbox| changed = inbox.mark_read(id)).map(|_| changed)
}

pub fn mark_all_read() -> io::Result<usize> {
    let mut changed = 0;
    mutate(|inbox| changed = inbox.mark_all_read()).map(|_| changed)
}

pub fn delete(id: &str) -> io::Result<bool> {
    let mut changed = false;
    mutate(|inbox| changed = inbox.delete(id)).map(|_| changed)
}

fn mutate(apply: impl FnOnce(&mut NotificationInbox)) -> io::Result<NotificationInbox> {
    let _guard = INBOX_IO_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("notification inbox lock");
    let path = crate::paths::notification_inbox_path();
    let mut inbox = load_from_path(&path);
    apply(&mut inbox);
    save_to_path(&path, &inbox)?;
    Ok(inbox)
}

pub(crate) fn load_from_path(path: &Path) -> NotificationInbox {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => {
            return NotificationInbox {
                version: INBOX_VERSION,
                entries: Vec::new(),
            }
        }
    };

    serde_json::from_str::<NotificationInbox>(&content)
        .map(NotificationInbox::normalized)
        .unwrap_or_else(|err| {
            log_debug!(
                "notification_inbox: failed to parse {}: {}",
                path.display(),
                err
            );
            NotificationInbox {
                version: INBOX_VERSION,
                entries: Vec::new(),
            }
        })
}

pub(crate) fn save_to_path(path: &Path, inbox: &NotificationInbox) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = tmp_path(path);
    let content = serde_json::to_string_pretty(inbox)?;
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn tmp_path(path: &Path) -> PathBuf {
    path.with_extension(format!("tmp.{}", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notification_inbox::model::NotificationEntry;

    #[test]
    fn save_and_load_round_trips_entries() {
        let dir = std::env::temp_dir().join(format!(
            "pad-inbox-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("inbox.json");
        let inbox = NotificationInbox {
            version: INBOX_VERSION,
            entries: vec![NotificationEntry {
                id: "one".into(),
                ts: 10,
                title: "done".into(),
                body: "body".into(),
                ..NotificationEntry::default()
            }],
        };

        save_to_path(&path, &inbox).unwrap();
        let loaded = load_from_path(&path);

        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].id, "one");
        let _ = std::fs::remove_dir_all(dir);
    }
}
