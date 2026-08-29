//! Read-only indexing for Pi's append-only JSONL session files.
//!
//! Pi owns the session journal. PAD only needs a cheap projection of that
//! journal for the Project/Task sidebar, so this module deliberately never
//! opens a file for writing (and never repairs a truncated final record).
//! A malformed record is returned as a diagnostic error instead of being
//! silently dropped or rewriting the source of truth.

use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

/// A compact representation of one append-only Pi session entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PiIndexedEntry {
    /// Pi's stable tree-entry id.
    pub(crate) id: String,
    /// The tree parent. None is the session root.
    pub(crate) parent_id: Option<String>,
    /// Pi entry discriminator (message, session_info, compaction, ...).
    pub(crate) entry_type: String,
    /// Entry timestamp, when supplied by Pi.
    pub(crate) timestamp: Option<String>,
    /// Byte offset at which this JSONL record starts.
    pub(crate) byte_offset: u64,
}

/// Cursor used to resume a read-only index pass after the last valid entry.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct PiSessionIndexCursor {
    /// Byte offset of the next record to inspect.
    pub(crate) byte_offset: u64,
    /// Last valid entry id, if the file has at least one entry.
    pub(crate) last_entry_id: Option<String>,
}

/// The result of indexing one Pi session file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PiSessionIndex {
    /// The path that was read. It is never used as an output path.
    pub(crate) path: PathBuf,
    /// Header JSON as an object. Keeping the original value allows the
    /// desktop layer to read new Pi header fields without changing this
    /// low-level indexer.
    pub(crate) header: Value,
    /// Session id from the header.
    pub(crate) session_id: String,
    /// Optional parent session path recorded by Pi when this session is a
    /// fork/branch.
    pub(crate) parent_session: Option<String>,
    /// All valid non-header entries in append order.
    pub(crate) entries: Vec<PiIndexedEntry>,
    /// Number of non-header records, including non-message state entries.
    pub(crate) entry_count: u64,
    /// Number of records whose type is message.
    pub(crate) message_count: u64,
    /// Current leaf entry (the final valid append), if any.
    pub(crate) leaf_id: Option<String>,
    /// Parent id for every indexed entry, keyed by entry id. BTreeMap keeps
    /// serialized/debug output deterministic for the sidebar and tests.
    pub(crate) parents: BTreeMap<String, Option<String>>,
    /// Latest entry timestamp, falling back to the header timestamp.
    pub(crate) updated_at: Option<String>,
    /// Position immediately after the final valid record.
    pub(crate) cursor: PiSessionIndexCursor,
}

impl PiSessionIndex {
    /// Return the path to the root of the entry tree for a leaf.
    ///
    /// Broken parent links are left visible in the returned chain rather than
    /// being repaired. The index remains a faithful read model of the file.
    pub(crate) fn lineage(&self, leaf_id: Option<&str>) -> Vec<&PiIndexedEntry> {
        let mut by_id = BTreeMap::new();
        for entry in &self.entries {
            by_id.insert(entry.id.as_str(), entry);
        }

        let mut current = leaf_id
            .or(self.leaf_id.as_deref())
            .and_then(|id| by_id.get(id).copied());
        let mut chain = Vec::new();
        let mut seen = BTreeMap::new();
        while let Some(entry) = current {
            // A malformed/cyclic parent chain is never allowed to loop the
            // sidebar. Keep the visible prefix and stop at the cycle.
            if seen.insert(entry.id.as_str(), ()).is_some() {
                break;
            }
            chain.push(entry);
            current = entry
                .parent_id
                .as_deref()
                .and_then(|parent| by_id.get(parent).copied());
        }
        chain.reverse();
        chain
    }
}

/// A recoverable, path-aware index diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SessionIndexError {
    /// The file could not be opened/read. No output file is ever created.
    Io { path: PathBuf, message: String },
    /// A zero-byte file has no session header.
    EmptyFile { path: PathBuf },
    /// The first physical line was not a valid Pi session header.
    MissingHeader {
        path: PathBuf,
        line: u64,
        detail: String,
    },
    /// A record after the header was not valid JSON. The tail flag denotes
    /// a final record without LF, which commonly means a process was
    /// interrupted during an append.
    MalformedEntry {
        path: PathBuf,
        line: u64,
        byte_offset: u64,
        detail: String,
        tail: bool,
    },
    /// An otherwise valid entry did not contain a stable Pi tree id.
    MissingEntryId {
        path: PathBuf,
        line: u64,
        byte_offset: u64,
    },
    /// An entry contained a non-string/non-null parentId.
    InvalidParentId {
        path: PathBuf,
        line: u64,
        byte_offset: u64,
    },
}

impl SessionIndexError {
    pub(crate) fn path(&self) -> &Path {
        match self {
            Self::Io { path, .. }
            | Self::EmptyFile { path }
            | Self::MissingHeader { path, .. }
            | Self::MalformedEntry { path, .. }
            | Self::MissingEntryId { path, .. }
            | Self::InvalidParentId { path, .. } => path,
        }
    }

    pub(crate) fn is_truncated_tail(&self) -> bool {
        matches!(self, Self::MalformedEntry { tail: true, .. })
    }
}

impl fmt::Display for SessionIndexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => {
                write!(
                    formatter,
                    "cannot read Pi session {}: {message}",
                    path.display()
                )
            }
            Self::EmptyFile { path } => {
                write!(formatter, "Pi session file is empty: {}", path.display())
            }
            Self::MissingHeader { path, line, detail } => write!(
                formatter,
                "Pi session header is missing or invalid at {} line {line}: {detail}",
                path.display()
            ),
            Self::MalformedEntry {
                path,
                line,
                byte_offset,
                detail,
                tail,
            } => write!(
                formatter,
                "malformed Pi session {} line {line} (byte {byte_offset}{}) : {detail}",
                path.display(),
                if *tail { ", truncated tail" } else { "" }
            ),
            Self::MissingEntryId {
                path,
                line,
                byte_offset,
            } => write!(
                formatter,
                "Pi session entry has no string id at {} line {line} (byte {byte_offset})",
                path.display()
            ),
            Self::InvalidParentId {
                path,
                line,
                byte_offset,
            } => write!(
                formatter,
                "Pi session entry has invalid parentId at {} line {line} (byte {byte_offset})",
                path.display()
            ),
        }
    }
}

impl std::error::Error for SessionIndexError {}

/// Read and index exactly one Pi JSONL session file.
///
/// This function is intentionally read-only. In particular, a valid prefix
/// followed by an unterminated JSON record returns
/// SessionIndexError::MalformedEntry and leaves the source bytes untouched.
pub(crate) fn index_file(path: impl AsRef<Path>) -> Result<PiSessionIndex, SessionIndexError> {
    let path = path.as_ref().to_path_buf();
    let file = File::open(&path).map_err(|error| SessionIndexError::Io {
        path: path.clone(),
        message: error.to_string(),
    })?;
    let mut reader = BufReader::new(file);
    let mut line = Vec::new();
    let mut byte_offset = 0_u64;

    let bytes_read = reader
        .read_until(b'\n', &mut line)
        .map_err(|error| io_error(&path, error))?;
    if bytes_read == 0 {
        return Err(SessionIndexError::EmptyFile { path });
    }

    let header_has_lf = line.last() == Some(&b'\n');
    byte_offset = byte_offset.saturating_add(bytes_read as u64);
    let header_line = trim_jsonl_line(&line, header_has_lf);
    let header = serde_json::from_slice::<Value>(header_line).map_err(|error| {
        SessionIndexError::MissingHeader {
            path: path.clone(),
            line: 1,
            detail: error.to_string(),
        }
    })?;
    let header_object = header
        .as_object()
        .ok_or_else(|| SessionIndexError::MissingHeader {
            path: path.clone(),
            line: 1,
            detail: "header must be a JSON object".to_string(),
        })?;
    if header_object.get("type").and_then(Value::as_str) != Some("session") {
        return Err(SessionIndexError::MissingHeader {
            path,
            line: 1,
            detail: "first record type must be \"session\"".to_string(),
        });
    }
    let session_id = header_object
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| SessionIndexError::MissingHeader {
            path: path.clone(),
            line: 1,
            detail: "session header has no non-empty string id".to_string(),
        })?
        .to_string();
    let parent_session = header_object
        .get("parentSession")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let header_timestamp = header_object
        .get("timestamp")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    let mut entries = Vec::new();
    let mut parents = BTreeMap::new();
    let mut message_count = 0_u64;
    let mut updated_at = header_timestamp;
    let mut line_number = 1_u64;
    loop {
        line.clear();
        let start_offset = byte_offset;
        let bytes_read = reader
            .read_until(b'\n', &mut line)
            .map_err(|error| io_error(&path, error))?;
        if bytes_read == 0 {
            break;
        }
        line_number = line_number.saturating_add(1);
        byte_offset = byte_offset.saturating_add(bytes_read as u64);
        let has_lf = line.last() == Some(&b'\n');
        let json_line = trim_jsonl_line(&line, has_lf);
        if json_line.iter().all(u8::is_ascii_whitespace) {
            // Blank lines are not entries and do not disturb the cursor.
            continue;
        }
        let value = serde_json::from_slice::<Value>(json_line).map_err(|error| {
            SessionIndexError::MalformedEntry {
                path: path.clone(),
                line: line_number,
                byte_offset: start_offset,
                detail: error.to_string(),
                tail: !has_lf,
            }
        })?;
        let object = value
            .as_object()
            .ok_or_else(|| SessionIndexError::MalformedEntry {
                path: path.clone(),
                line: line_number,
                byte_offset: start_offset,
                detail: "entry must be a JSON object".to_string(),
                tail: !has_lf,
            })?;
        let id = object
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| SessionIndexError::MissingEntryId {
                path: path.clone(),
                line: line_number,
                byte_offset: start_offset,
            })?
            .to_string();
        let parent_id = match object.get("parentId") {
            None | Some(Value::Null) => None,
            Some(Value::String(parent)) => Some(parent.clone()),
            Some(_) => {
                return Err(SessionIndexError::InvalidParentId {
                    path: path.clone(),
                    line: line_number,
                    byte_offset: start_offset,
                });
            }
        };
        let entry_type = object
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        if entry_type == "message" {
            message_count = message_count.saturating_add(1);
        }
        if let Some(timestamp) = object.get("timestamp").and_then(Value::as_str) {
            updated_at = Some(timestamp.to_string());
        }
        parents.insert(id.clone(), parent_id.clone());
        entries.push(PiIndexedEntry {
            id,
            parent_id,
            entry_type,
            timestamp: object
                .get("timestamp")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            byte_offset: start_offset,
        });
    }

    let leaf_id = entries.last().map(|entry| entry.id.clone());
    let cursor = PiSessionIndexCursor {
        byte_offset,
        last_entry_id: leaf_id.clone(),
    };
    Ok(PiSessionIndex {
        path,
        header,
        session_id,
        parent_session,
        entry_count: entries.len() as u64,
        message_count,
        leaf_id,
        parents,
        updated_at,
        cursor,
        entries,
    })
}

/// Aggregate a read-only rebuild over several session files.
///
/// Files are indexed independently so one interrupted Pi append does not
/// hide healthy sessions from the sidebar. errors contains path-aware
/// diagnostics; this function never creates, truncates, or repairs a file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SessionIndexRebuild {
    pub(crate) indexed: Vec<PiSessionIndex>,
    pub(crate) errors: Vec<SessionIndexError>,
}

impl SessionIndexRebuild {
    pub(crate) fn is_complete(&self) -> bool {
        self.errors.is_empty()
    }
}

pub(crate) fn rebuild<I, P>(paths: I) -> SessionIndexRebuild
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut result = SessionIndexRebuild::default();
    for path in paths {
        match index_file(path) {
            Ok(index) => result.indexed.push(index),
            Err(error) => result.errors.push(error),
        }
    }
    result
}

fn trim_jsonl_line(line: &[u8], has_lf: bool) -> &[u8] {
    let mut end = line.len();
    if has_lf {
        end = end.saturating_sub(1);
    }
    if end > 0 && line[end - 1] == b'\r' {
        end -= 1;
    }
    &line[..end]
}

fn io_error(path: &Path, error: io::Error) -> SessionIndexError {
    SessionIndexError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock before epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("pad-pi-session-index-{nonce}"));
            fs::create_dir_all(&path).expect("create temp directory");
            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write(path: &Path, content: &str) {
        fs::write(path, content).expect("write fixture");
    }

    #[test]
    fn indexes_header_entries_messages_leaf_parent_and_cursor() {
        let dir = TempDir::new();
        let path = dir.path.join("session.jsonl");
        write(
            &path,
            concat!(
                "{\"type\":\"session\",\"version\":3,\"id\":\"sess-1\",\"timestamp\":\"2026-08-29T10:00:00.000Z\",\"cwd\":\"/tmp/project\"}\n",
                "{\"type\":\"message\",\"id\":\"a\",\"parentId\":null,\"timestamp\":\"2026-08-29T10:00:01.000Z\",\"message\":{\"role\":\"user\"}}\n",
                "{\"type\":\"session_info\",\"id\":\"b\",\"parentId\":\"a\",\"timestamp\":\"2026-08-29T10:00:02.000Z\",\"name\":\"Demo\"}\n",
                "{\"type\":\"message\",\"id\":\"c\",\"parentId\":\"b\",\"timestamp\":\"2026-08-29T10:00:03.000Z\",\"message\":{\"role\":\"assistant\"}}\n",
            ),
        );

        let index = index_file(&path).expect("valid Pi session");
        assert_eq!(index.session_id, "sess-1");
        assert_eq!(index.entry_count, 3);
        assert_eq!(index.message_count, 2);
        assert_eq!(index.leaf_id.as_deref(), Some("c"));
        assert_eq!(index.parents["c"].as_deref(), Some("b"));
        assert_eq!(
            index.updated_at.as_deref(),
            Some("2026-08-29T10:00:03.000Z")
        );
        assert_eq!(index.cursor.last_entry_id.as_deref(), Some("c"));
        assert_eq!(index.cursor.byte_offset, fs::metadata(&path).unwrap().len());
        let lineage = index.lineage(None);
        assert_eq!(
            lineage
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
    }

    #[test]
    fn empty_and_missing_header_are_diagnostic_and_read_only() {
        let dir = TempDir::new();
        let empty = dir.path.join("empty.jsonl");
        write(&empty, "");
        assert!(matches!(
            index_file(&empty),
            Err(SessionIndexError::EmptyFile { .. })
        ));
        assert_eq!(fs::read(&empty).unwrap(), b"");

        let missing = dir.path.join("missing-header.jsonl");
        write(&missing, "{\"type\":\"message\",\"id\":\"a\"}\n");
        let error = index_file(&missing).expect_err("non-session first line");
        assert!(matches!(
            error,
            SessionIndexError::MissingHeader { line: 1, .. }
        ));
        assert_eq!(
            fs::read_to_string(&missing).unwrap(),
            "{\"type\":\"message\",\"id\":\"a\"}\n"
        );
    }

    #[test]
    fn unterminated_tail_returns_error_without_repairing_source() {
        let dir = TempDir::new();
        let path = dir.path.join("truncated.jsonl");
        let content = concat!(
            "{\"type\":\"session\",\"version\":3,\"id\":\"sess-1\"}\n",
            "{\"type\":\"message\",\"id\":\"a\",\"parentId\":null}\n",
            "{\"type\":\"message\",\"id\":\"broken\",\"parentId\":"
        );
        write(&path, content);
        let before = fs::read(&path).unwrap();
        let error = index_file(&path).expect_err("truncated append must be surfaced");
        assert!(error.is_truncated_tail());
        assert!(error.to_string().contains("truncated tail"));
        assert_eq!(fs::read(&path).unwrap(), before);
    }

    #[test]
    fn rebuild_is_read_only_and_keeps_healthy_files_when_path_is_invalid() {
        let dir = TempDir::new();
        let healthy = dir.path.join("healthy.jsonl");
        write(&healthy, "{\"type\":\"session\",\"id\":\"ok\"}\n");
        let outside = dir.path.join("..").join("pad-index-must-not-exist.jsonl");
        let outside_before = fs::read(&outside).ok();
        let report = rebuild([healthy.as_path(), outside.as_path()]);
        assert_eq!(report.indexed.len(), 1);
        assert_eq!(report.indexed[0].session_id, "ok");
        assert_eq!(report.errors.len(), 1);
        assert!(!report.is_complete());
        assert_eq!(
            fs::read(&healthy).unwrap(),
            b"{\"type\":\"session\",\"id\":\"ok\"}\n"
        );
        assert_eq!(fs::read(&outside).ok(), outside_before);
    }
}
