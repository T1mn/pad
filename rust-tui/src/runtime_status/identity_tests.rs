use super::{parse_etime_seconds, started_at_matches, status_process_alive};
use crate::runtime_status::ProcessStatus;

pub(crate) fn etime_parser_reads_every_ps_shape() {
    assert_eq!(parse_etime_seconds("00:00"), Some(0));
    assert_eq!(parse_etime_seconds("01:23"), Some(83));
    assert_eq!(parse_etime_seconds("   02:03:04"), Some(7384));
    assert_eq!(parse_etime_seconds("5-18:20:43\n"), Some(498043));
    assert_eq!(parse_etime_seconds(" 12-00:00:00 "), Some(1036800));
}

pub(crate) fn etime_parser_rejects_garbage() {
    assert_eq!(parse_etime_seconds(""), None);
    assert_eq!(parse_etime_seconds("   "), None);
    assert_eq!(parse_etime_seconds("42"), None);
    assert_eq!(parse_etime_seconds("ab:cd"), None);
    assert_eq!(parse_etime_seconds("01:99"), None);
    assert_eq!(parse_etime_seconds("1:2:3:4"), None);
    assert_eq!(parse_etime_seconds("-01:02"), None);
}

pub(crate) fn started_at_matches_tolerates_early_start_only() {
    // 状态文件总是在进程起来之后才写的,早于 started_at 都算同一个进程。
    assert!(started_at_matches(1_000, 1_000));
    assert!(started_at_matches(1_000, 9_000));
    assert!(started_at_matches(1_003, 1_000));
    // 晚于 started_at 超过容差 => pid 被回收给了别的进程。
    assert!(!started_at_matches(1_006, 1_000));
    assert!(!started_at_matches(90_000, 1_000));
}

pub(crate) fn status_process_alive_rejects_recycled_pid() {
    let now = crate::time::unix_now_ts();
    let running = ProcessStatus {
        pid: std::process::id(),
        started_at: now,
        mode: "telegram-bot".to_string(),
    };
    assert!(status_process_alive(&running));

    // 同一个 pid,但状态文件是一天前写的:当前进程显然不是它。
    let recycled = ProcessStatus {
        started_at: now - 86_400,
        ..running
    };
    assert!(!status_process_alive(&recycled));
}
