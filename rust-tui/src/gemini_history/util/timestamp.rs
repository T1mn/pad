pub(crate) fn parse_timestamp(text: &str) -> Option<i64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    let text = text.strip_suffix('Z').unwrap_or(text);
    let (date, time) = text.split_once('T').or_else(|| text.split_once(' '))?;
    let (year, month, day) = parse_date(date)?;
    let (time, offset_secs) = parse_time_and_offset(time)?;
    let (hour, minute, second) = parse_hms(time)?;
    let days = days_from_civil(year, month, day);
    let seconds = days
        .saturating_mul(86_400)
        .saturating_add(hour * 3_600)
        .saturating_add(minute * 60)
        .saturating_add(second)
        .saturating_sub(offset_secs);
    Some(seconds)
}

fn parse_date(text: &str) -> Option<(i64, i64, i64)> {
    let mut parts = text.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    Some((year, month, day))
}

fn parse_time_and_offset(text: &str) -> Option<(&str, i64)> {
    if let Some(time) = text.strip_suffix('Z') {
        return Some((time, 0));
    }

    if let Some((time, offset)) = text.rsplit_once('+') {
        return Some((time, parse_offset(offset)?));
    }

    if let Some((time, offset)) = text.rsplit_once('-') {
        if time.contains(':') {
            return Some((time, -parse_offset(offset)?));
        }
    }

    Some((text, 0))
}

fn parse_offset(text: &str) -> Option<i64> {
    let mut parts = text.split(':');
    let hours: i64 = parts.next()?.parse().ok()?;
    let minutes: i64 = parts.next()?.parse().ok()?;
    Some(
        hours
            .saturating_mul(3600)
            .saturating_add(minutes.saturating_mul(60)),
    )
}

fn parse_hms(text: &str) -> Option<(i64, i64, i64)> {
    let time = text.split('.').next().unwrap_or(text);
    let mut parts = time.split(':');
    let hour = parts.next()?.parse().ok()?;
    let minute = parts.next()?.parse().ok()?;
    let second = parts.next()?.parse().ok()?;
    Some((hour, minute, second))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * month + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
