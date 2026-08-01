use super::super::{read_status, write_status_body, ProcessStatus, StatusGuard};
use super::support::{remove_status_artifacts, temp_status_path};
#[cfg(any(unix, windows))]
use std::sync::{mpsc, Arc, Barrier};
#[cfg(any(unix, windows))]
use std::thread;

#[test]
fn status_guard_drop_preserves_newer_status_file() {
    let path = temp_status_path();
    let guard = StatusGuard::new(path.clone(), "telegram-bot").unwrap();
    write_status_body(
        &path,
        &ProcessStatus {
            pid: guard.pid.saturating_add(1),
            started_at: guard.started_at.saturating_add(1),
            mode: "telegram-bot".to_string(),
        },
    )
    .unwrap();
    drop(guard);

    let status = read_status(&path).unwrap();
    assert_eq!(status.pid, std::process::id().saturating_add(1));
    remove_status_artifacts(&path);
}

#[cfg(any(unix, windows))]
#[test]
fn concurrent_status_guards_have_one_owner() {
    let path = temp_status_path();
    let start = Arc::new(Barrier::new(3));
    let finish = Arc::new(Barrier::new(3));
    let (sender, receiver) = mpsc::channel();
    let mut threads = Vec::new();

    for _ in 0..2 {
        let path = path.clone();
        let start = Arc::clone(&start);
        let finish = Arc::clone(&finish);
        let sender = sender.clone();
        threads.push(thread::spawn(move || {
            start.wait();
            match StatusGuard::new(path, "telegram-bot") {
                Ok(guard) => {
                    sender.send(true).unwrap();
                    finish.wait();
                    drop(guard);
                }
                Err(error) => {
                    sender.send(false).unwrap();
                    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
                    finish.wait();
                }
            }
        }));
    }
    drop(sender);
    start.wait();

    let outcomes = [receiver.recv().unwrap(), receiver.recv().unwrap()];
    assert_eq!(outcomes.iter().filter(|owned| **owned).count(), 1);
    assert_eq!(outcomes.iter().filter(|owned| !**owned).count(), 1);
    finish.wait();
    for thread in threads {
        thread.join().unwrap();
    }
    remove_status_artifacts(&path);
}

#[cfg(unix)]
fn spawn_live_child() -> std::process::Child {
    std::process::Command::new("sleep")
        .arg("5")
        .spawn()
        .expect("spawn sleep")
}

#[cfg(unix)]
#[test]
fn status_guard_starts_when_pid_was_recycled() {
    let path = temp_status_path();
    let mut child = spawn_live_child();
    write_status_body(
        &path,
        &ProcessStatus {
            pid: child.id(),
            started_at: crate::time::unix_now_ts() - 86_400,
            mode: "telegram-bot".to_string(),
        },
    )
    .unwrap();

    let guard = StatusGuard::new(path.clone(), "telegram-bot")
        .expect("recycled pid must not block daemon startup");
    assert_eq!(read_status(&path).unwrap().pid, std::process::id());
    drop(guard);
    let _ = child.kill();
    let _ = child.wait();
    remove_status_artifacts(&path);
}

#[cfg(unix)]
#[test]
fn status_guard_refuses_when_owner_still_runs() {
    let path = temp_status_path();
    let mut child = spawn_live_child();
    write_status_body(
        &path,
        &ProcessStatus {
            pid: child.id(),
            started_at: crate::time::unix_now_ts(),
            mode: "telegram-bot".to_string(),
        },
    )
    .unwrap();

    let err = match StatusGuard::new(path.clone(), "telegram-bot") {
        Ok(_) => panic!("owner still runs, guard must refuse"),
        Err(err) => err,
    };
    assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(read_status(&path).unwrap().pid, child.id());
    let _ = child.kill();
    let _ = child.wait();
    remove_status_artifacts(&path);
}

#[test]
fn stat_parser_treats_zombies_as_not_alive() {
    assert!(super::super::process::stat_indicates_zombie("Z+"));
    assert!(super::super::process::stat_indicates_zombie("SZ"));
    assert!(!super::super::process::stat_indicates_zombie("S+"));
    assert!(!super::super::process::stat_indicates_zombie("R"));
}
