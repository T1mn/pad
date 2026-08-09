use super::{plan_stop, stop_daemon_at, terminate_process, ProcessStatus, StopPlan};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

fn temp_path(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "pad-stop-{}-{}-{}.json",
        tag,
        std::process::id(),
        crate::time::unix_now_nanos()
    ))
}

fn write_status(path: &Path, status: &ProcessStatus) {
    fs::write(path, serde_json::to_string(status).unwrap()).unwrap();
}

#[cfg(unix)]
fn spawn_live_child() -> std::process::Child {
    std::process::Command::new("sleep")
        .arg("5")
        .spawn()
        .expect("spawn sleep")
}

fn status_for(pid: u32, started_at: i64) -> ProcessStatus {
    ProcessStatus {
        pid,
        started_at,
        mode: "telegram-bot".to_string(),
    }
}

pub(crate) fn plan_stop_maps_every_case() {
    let status = status_for(4242, 100);
    assert_eq!(plan_stop(None, 7, |_| true), StopPlan::CleanupStale);
    assert_eq!(
        plan_stop(Some(&status), 4242, |_| true),
        StopPlan::KeepSelf,
        "自己的状态文件不能自己停自己"
    );
    assert_eq!(plan_stop(Some(&status), 7, |_| true), StopPlan::Terminate);
    assert_eq!(
        plan_stop(Some(&status), 7, |_| false),
        StopPlan::CleanupStale,
        "对不上账的 pid 只清状态文件"
    );
}

#[cfg(unix)]
pub(crate) fn recycled_pid_is_never_signalled() {
    let status_path = temp_path("recycled");
    let socket_path = temp_path("recycled-sock");
    let mut child = spawn_live_child();
    // 一天前写下的状态文件,pid 早已被系统回收给这个无关进程。
    write_status(
        &status_path,
        &status_for(child.id(), crate::time::unix_now_ts() - 86_400),
    );

    let kills = AtomicUsize::new(0);
    let stopped = stop_daemon_at(&status_path, &socket_path, |_| {
        kills.fetch_add(1, Ordering::SeqCst);
        Ok(())
    })
    .unwrap();

    assert!(!stopped);
    assert_eq!(kills.load(Ordering::SeqCst), 0, "不能对无关进程发信号");
    assert!(!status_path.exists(), "陈旧状态文件必须被清掉");
    assert!(
        crate::runtime_status::process_alive(child.id()),
        "无关进程必须毫发无损"
    );

    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_file(crate::runtime_status::status_lock_path(&status_path));
}

#[cfg(unix)]
pub(crate) fn terminate_rechecks_identity_before_signalling() {
    let mut child = spawn_live_child();
    let stale = status_for(child.id(), crate::time::unix_now_ts() - 86_400);

    terminate_process(&stale).unwrap();

    assert!(crate::runtime_status::process_alive(child.id()));
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
pub(crate) fn stop_preserves_a_new_status_written_during_termination() {
    let status_path = temp_path("replacement");
    let socket_path = temp_path("replacement-sock");
    let mut child = spawn_live_child();
    let original = status_for(child.id(), crate::time::unix_now_ts());
    let replacement = status_for(std::process::id(), original.started_at.saturating_add(1));
    write_status(&status_path, &original);

    let stopped = stop_daemon_at(&status_path, &socket_path, |_| {
        write_status(&status_path, &replacement);
        Ok(())
    })
    .unwrap();

    assert!(stopped);
    assert_eq!(
        crate::runtime_status::read_status(&status_path)
            .unwrap()
            .pid,
        replacement.pid
    );

    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_file(&status_path);
    let _ = fs::remove_file(crate::runtime_status::status_lock_path(&status_path));
}

#[cfg(unix)]
pub(crate) fn matching_started_at_runs_the_stop_flow() {
    let status_path = temp_path("match");
    let socket_path = temp_path("match-sock");
    let mut child = spawn_live_child();
    let status = status_for(child.id(), crate::time::unix_now_ts());
    write_status(&status_path, &status);

    let stopped = stop_daemon_at(&status_path, &socket_path, terminate_process).unwrap();

    assert!(stopped);
    assert!(!status_path.exists());
    assert!(!crate::runtime_status::status_process_alive(&status));

    let _ = child.wait();
    let _ = fs::remove_file(crate::runtime_status::status_lock_path(&status_path));
}
