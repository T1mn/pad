pub fn short_time(ts: i64) -> String {
    if ts <= 0 {
        return "unknown".to_string();
    }
    let now = crate::app::unix_now_ts();
    let age = now.saturating_sub(ts);
    if age < 60 {
        format!("{age}s ago")
    } else if age < 3600 {
        format!("{}m ago", age / 60)
    } else if age < 86_400 {
        format!("{}h ago", age / 3600)
    } else {
        format!("{}d ago", age / 86_400)
    }
}
