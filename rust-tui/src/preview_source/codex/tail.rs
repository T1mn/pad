use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

#[cfg(not(test))]
const INITIAL_TAIL_BYTES: u64 = 2 * 1024 * 1024;
#[cfg(test)]
const INITIAL_TAIL_BYTES: u64 = 256;

pub(super) fn file_len(path: &Path) -> io::Result<u64> {
    std::fs::metadata(path).map(|metadata| metadata.len())
}

pub(super) fn initial_tail_bytes(file_len: u64) -> u64 {
    INITIAL_TAIL_BYTES.min(file_len).max(1)
}

pub(super) fn grow_tail_bytes(current: u64, file_len: u64) -> u64 {
    current.saturating_mul(2).min(file_len)
}

pub(super) fn read_tail_lines(
    path: &Path,
    file_len: u64,
    tail_bytes: u64,
) -> io::Result<Vec<String>> {
    let start = file_len.saturating_sub(tail_bytes);
    let mut file = File::open(path)?;
    let read_start = start.saturating_sub(1);
    file.seek(SeekFrom::Start(read_start))?;

    let capacity = tail_bytes
        .saturating_add(u64::from(start > 0))
        .min(usize::MAX as u64) as usize;
    let mut bytes = Vec::with_capacity(capacity);
    file.read_to_end(&mut bytes)?;
    if start > 0 {
        if bytes.first() == Some(&b'\n') {
            bytes.drain(..1);
        } else if let Some(pos) = bytes.iter().position(|byte| *byte == b'\n') {
            bytes.drain(..=pos);
        } else {
            bytes.clear();
        }
    }

    Ok(String::from_utf8_lossy(&bytes)
        .lines()
        .map(str::to_string)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{grow_tail_bytes, initial_tail_bytes, read_tail_lines};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pad-codex-tail-{name}-{stamp}.jsonl"))
    }

    #[test]
    fn tail_window_helpers_clamp_and_grow() {
        assert_eq!(initial_tail_bytes(0), 1);
        assert_eq!(initial_tail_bytes(10), 10);
        assert_eq!(initial_tail_bytes(512), 256);
        assert_eq!(grow_tail_bytes(256, 400), 400);
    }

    #[test]
    fn tail_reader_keeps_whole_file_when_short() {
        let path = temp_path("short");
        fs::write(&path, "one\ntwo\n").unwrap();

        let lines = read_tail_lines(&path, 8, 8).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(lines, vec!["one".to_string(), "two".to_string()]);
    }

    #[test]
    fn tail_reader_drops_partial_first_line() {
        let path = temp_path("partial");
        fs::write(&path, "first\nsecond\nthird\n").unwrap();

        let lines = read_tail_lines(&path, 19, 13).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(lines, vec!["second".to_string(), "third".to_string()]);
    }
}
