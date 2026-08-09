//! PID 复用防护:状态文件里的 pid 只有配上 `started_at` 才算数。
//!
//! daemon 被 `kill -9` 后状态文件会残留,系统随后可能把同一个 pid 分配给
//! 毫不相干的进程。裸 `kill(pid, 0)` 认不出这种情况,于是 pad 要么误杀别人,
//! 要么因为别人占着 pid 而永远起不来 daemon。

use super::process::process_signalable;
use super::ProcessStatus;
#[cfg(unix)]
use std::process::Command;

/// `ps` 的 etime 只精确到秒,采样与 `started_at` 之间也有抖动,留一点余量。
const STARTED_AT_TOLERANCE_SECS: i64 = 5;

/// 带身份对账的存活探测:pid 活着,并且它就是写下这份状态文件的那个进程。
///
/// 拿不到启动时间(非 unix、`ps` 不可用、进程已消失)一律判定为"不是我们的进程",
/// 宁可少停一次 daemon,也不能拿着陈旧 pid 去 kill 无关进程。
pub fn status_process_alive(status: &ProcessStatus) -> bool {
    if !process_signalable(status.pid) {
        return false;
    }
    match process_started_at(status.pid) {
        Some(observed) => started_at_matches(observed, status.started_at),
        None => false,
    }
}

/// 单侧比较:状态文件是进程起来之后才写的,所以真实启动时间只会早于 `started_at`
/// (embedded daemon 甚至可能早好几个小时)。反过来,观察到的启动时间明显晚于
/// `started_at`,只可能是原进程已经死了、pid 被系统回收给了别人。
pub(in crate::runtime_status) fn started_at_matches(observed: i64, recorded: i64) -> bool {
    observed <= recorded.saturating_add(STARTED_AT_TOLERANCE_SECS)
}

#[cfg(unix)]
fn process_started_at(pid: u32) -> Option<i64> {
    let output = Command::new("ps")
        .args(["-o", "etime=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let elapsed = parse_etime_seconds(&String::from_utf8_lossy(&output.stdout))?;
    Some(crate::time::unix_now_ts().saturating_sub(elapsed))
}

#[cfg(not(unix))]
fn process_started_at(pid: u32) -> Option<i64> {
    let _ = pid;
    None
}

/// 解析 `ps -o etime=` 的输出:`[[DD-]HH:]MM:SS`,macOS 和 Linux 都是这个格式。
pub(in crate::runtime_status) fn parse_etime_seconds(raw: &str) -> Option<i64> {
    let text = raw.trim();
    let (days, clock) = match text.split_once('-') {
        Some((days, rest)) => (parse_field(days)?, rest),
        None => (0, text),
    };
    let mut fields = clock.split(':').rev();
    let seconds = parse_field(fields.next()?)?;
    let minutes = parse_field(fields.next()?)?;
    let hours = match fields.next() {
        Some(field) => parse_field(field)?,
        None => 0,
    };
    if fields.next().is_some() || seconds >= 60 || minutes >= 60 {
        return None;
    }
    Some(days * 86_400 + hours * 3_600 + minutes * 60 + seconds)
}

fn parse_field(raw: &str) -> Option<i64> {
    let text = raw.trim();
    if text.is_empty() || !text.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[cfg(test)]
#[path = "identity_tests.rs"]
pub(crate) mod tests;
