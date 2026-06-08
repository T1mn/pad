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
#[path = "tail_tests.rs"]
mod tests;
