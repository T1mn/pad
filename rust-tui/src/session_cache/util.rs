use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn prefer_non_empty(
    first: Option<&String>,
    second: Option<&String>,
    third: Option<&String>,
) -> Option<String> {
    first
        .and_then(|value| clean_text(Some(value.as_str())))
        .or_else(|| second.and_then(|value| clean_text(Some(value.as_str()))))
        .or_else(|| third.and_then(|value| clean_text(Some(value.as_str()))))
}

pub(super) fn clean_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
