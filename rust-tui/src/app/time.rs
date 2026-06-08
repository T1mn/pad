pub(crate) use crate::time::unix_now_ts;

pub(crate) fn new_handoff_trace(prefix: &str) -> String {
    let stamp = crate::time::unix_now_millis();
    format!("{prefix}-{stamp}-{}", std::process::id())
}
