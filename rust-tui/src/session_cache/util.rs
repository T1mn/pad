pub(super) fn first_non_empty_str<'a>(
    values: impl IntoIterator<Item = Option<&'a str>>,
) -> Option<&'a str> {
    values
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|text| !text.is_empty())
}

pub(super) fn prefer_non_empty_str<'a>(
    values: impl IntoIterator<Item = Option<&'a str>>,
) -> Option<String> {
    first_non_empty_str(values).map(ToOwned::to_owned)
}

pub(super) fn clean_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn now_ts() -> i64 {
    crate::time::unix_now_ts()
}
