use super::support::{remove_status_artifacts, temp_status_path};
use std::fs;
use std::path::Path;
#[cfg(any(unix, windows))]
use std::thread;
#[cfg(any(unix, windows))]
use std::time::Duration;

#[cfg(any(unix, windows))]
#[test]
fn status_lock_is_exclusive_and_released_on_drop() {
    let path = temp_status_path();
    let first = super::super::StatusLock::acquire(&path).unwrap();
    let contender_path = path.clone();
    let contender =
        thread::spawn(
            move || match super::super::StatusLock::acquire(&contender_path) {
                Err(error) => error.kind(),
                Ok(lock) => {
                    drop(lock);
                    panic!("a second process/thread must not own the status lock");
                }
            },
        );
    assert_eq!(contender.join().unwrap(), std::io::ErrorKind::AlreadyExists);

    drop(first);
    let second = super::super::StatusLock::acquire(&path).unwrap();
    drop(second);
    remove_status_artifacts(&path);
}

#[cfg(any(unix, windows))]
#[test]
fn status_lock_is_exclusive_across_processes_and_released_on_crash() {
    let path = temp_status_path();
    let ready_path = temp_status_path();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "runtime_status::tests::lock::hold_status_lock_for_test",
            "--nocapture",
        ])
        .env("PAD_STATUS_LOCK_TEST_PATH", &path)
        .env("PAD_STATUS_LOCK_TEST_READY", &ready_path)
        .spawn()
        .unwrap();

    let mut ready = false;
    for _ in 0..500 {
        if ready_path.exists() {
            ready = true;
            break;
        }
        if child.try_wait().unwrap().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    let contender_kind = if ready {
        match super::super::StatusLock::acquire(&path) {
            Err(error) => Some(error.kind()),
            Ok(lock) => {
                drop(lock);
                None
            }
        }
    } else {
        None
    };

    let _ = child.kill();
    let _ = child.wait();
    let released = super::super::StatusLock::acquire(&path).is_ok();
    assert!(ready, "lock-holder test process did not acquire the lock");
    assert_eq!(contender_kind, Some(std::io::ErrorKind::AlreadyExists));
    assert!(released, "crashing lock-holder must release its OS lock");
    remove_status_artifacts(&path);
    let _ = fs::remove_file(ready_path);
}

#[cfg(any(unix, windows))]
#[test]
fn hold_status_lock_for_test() {
    let (Ok(path), Ok(ready_path)) = (
        std::env::var("PAD_STATUS_LOCK_TEST_PATH"),
        std::env::var("PAD_STATUS_LOCK_TEST_READY"),
    ) else {
        return;
    };
    let _lock = super::super::StatusLock::acquire(Path::new(&path)).unwrap();
    fs::write(ready_path, b"ready").unwrap();
    thread::sleep(Duration::from_secs(30));
}
