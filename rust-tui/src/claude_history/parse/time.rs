pub(super) fn parse_rfc3339_utc_ts(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if bytes.len() < 20 {
        return None;
    }
    let year = parse_i32(bytes, 0, 4)?;
    let month = parse_u32(bytes, 5, 7)?;
    let day = parse_u32(bytes, 8, 10)?;
    let hour = parse_u32(bytes, 11, 13)?;
    let minute = parse_u32(bytes, 14, 16)?;
    let second = parse_u32(bytes, 17, 19)?;
    let tz_start = text[19..]
        .find(['Z', '+', '-'])
        .map(|idx| idx + 19)
        .unwrap_or(bytes.len());
    let offset_secs = if bytes.get(tz_start) == Some(&b'Z') || tz_start == bytes.len() {
        0
    } else {
        let sign = if bytes.get(tz_start) == Some(&b'-') {
            -1_i64
        } else {
            1_i64
        };
        let offset_hour = parse_u32(bytes, tz_start + 1, tz_start + 3)? as i64;
        let offset_minute = parse_u32(bytes, tz_start + 4, tz_start + 6)? as i64;
        sign * (offset_hour * 3600 + offset_minute * 60)
    };

    let days = days_from_civil(year, month, day)?;
    Some(days * 86_400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64 - offset_secs)
}

fn parse_i32(bytes: &[u8], start: usize, end: usize) -> Option<i32> {
    std::str::from_utf8(bytes.get(start..end)?)
        .ok()?
        .parse()
        .ok()
}

fn parse_u32(bytes: &[u8], start: usize, end: usize) -> Option<u32> {
    std::str::from_utf8(bytes.get(start..end)?)
        .ok()?
        .parse()
        .ok()
}

fn days_from_civil(year: i32, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let adjust = if month <= 2 { 1 } else { 0 };
    let year = year - adjust;
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = month as i32 + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era as i64 * 146_097 + doe as i64 - 719_468)
}
