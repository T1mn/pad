use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const GEMINI_INDEX_DB_FILE: &str = "gemini_history.sqlite";

pub(crate) fn default_gemini_tmp_dir() -> io::Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory not found"))?;
    Ok(home.join(".gemini").join("tmp"))
}

pub(crate) fn default_index_db_path() -> io::Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory not found"))?;
    Ok(home.join(".pad").join(GEMINI_INDEX_DB_FILE))
}

pub(crate) fn ensure_parent_dir(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[allow(dead_code)]
pub(crate) fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

pub(crate) fn md5_hex(text: &str) -> String {
    format!("{:x}", md5::compute(text.as_bytes()))
}

pub(crate) fn file_mtime_secs(path: &Path) -> Option<i64> {
    path.metadata()
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
}

pub(crate) fn read_text(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
}

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
