mod parse;
mod walk;

use super::model::{GeminiSnapshot, GeminiThreadKey, GeminiThreadRecord};
use super::util::md5_hex;
use parse::parse_snapshot;
use std::collections::HashMap;
use std::io;
use std::path::Path;
use walk::walk_session_files;

pub(crate) fn collect_records(root: &Path) -> io::Result<Vec<GeminiThreadRecord>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut groups: HashMap<GeminiThreadKey, Vec<GeminiSnapshot>> = HashMap::new();
    for session_path in walk_session_files(root)? {
        match parse_snapshot(&session_path) {
            Ok(Some(snapshot)) => {
                let key = GeminiThreadKey::new(
                    snapshot.session_id.clone(),
                    snapshot.project_root.to_string_lossy().to_string(),
                );
                groups.entry(key).or_default().push(snapshot);
            }
            Ok(None) => {}
            Err(_err) => {
                crate::log_debug!(
                    "gemini_history: skip unreadable snapshot path={} err={}",
                    session_path.display(),
                    _err
                );
            }
        }
    }

    let mut records = groups
        .into_iter()
        .filter_map(|(key, snapshots)| build_record(key, snapshots))
        .collect::<Vec<_>>();
    records.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| right.session_id.cmp(&left.session_id))
    });
    Ok(records)
}

fn build_record(
    _key: GeminiThreadKey,
    snapshots: Vec<GeminiSnapshot>,
) -> Option<GeminiThreadRecord> {
    let mut stats = SnapshotGroupStats::new(snapshots.len());
    for snapshot in &snapshots {
        stats.observe(snapshot);
    }
    let preferred = stats.preferred()?;
    let mut payload_hashes = stats.payload_hashes;
    payload_hashes.sort_unstable();
    let payload_hash = payload_hashes.join(":");

    Some(GeminiThreadRecord {
        session_id: preferred.session_id.clone(),
        cwd: preferred.project_root.clone(),
        project_alias: preferred.project_alias.clone(),
        transcript_path: preferred.transcript_path.clone(),
        kind: preferred.kind.clone(),
        start_time: stats.start_time,
        updated_at: stats.updated_at,
        title: preferred
            .summary
            .clone()
            .or_else(|| preferred.first_user_message.clone()),
        subtitle: preferred.last_user_message.clone(),
        summary: preferred.summary.clone(),
        first_user_message: preferred.first_user_message.clone(),
        last_user_message: preferred.last_user_message.clone(),
        last_assistant_message: preferred.last_assistant_message.clone(),
        has_subagent: stats.has_subagent,
        payload_hash: md5_hex(&payload_hash),
        snapshot_count: snapshots.len() as i64,
    })
}

struct SnapshotGroupStats<'a> {
    preferred_any: Option<&'a GeminiSnapshot>,
    preferred_main: Option<&'a GeminiSnapshot>,
    start_time: i64,
    updated_at: i64,
    has_subagent: bool,
    payload_hashes: Vec<&'a str>,
}

impl<'a> SnapshotGroupStats<'a> {
    fn new(snapshot_count: usize) -> Self {
        Self {
            preferred_any: None,
            preferred_main: None,
            start_time: i64::MAX,
            updated_at: i64::MIN,
            has_subagent: false,
            payload_hashes: Vec::with_capacity(snapshot_count),
        }
    }

    fn observe(&mut self, snapshot: &'a GeminiSnapshot) {
        self.start_time = self.start_time.min(snapshot.start_time);
        self.updated_at = self.updated_at.max(snapshot.last_updated);
        self.has_subagent |= snapshot.kind == "subagent";
        self.payload_hashes.push(snapshot.payload_hash.as_str());

        if is_newer_snapshot(snapshot, self.preferred_any) {
            self.preferred_any = Some(snapshot);
        }
        if snapshot.kind == "main" && is_newer_snapshot(snapshot, self.preferred_main) {
            self.preferred_main = Some(snapshot);
        }
    }

    fn preferred(&self) -> Option<&'a GeminiSnapshot> {
        self.preferred_main.or(self.preferred_any)
    }
}

fn is_newer_snapshot(snapshot: &GeminiSnapshot, current: Option<&GeminiSnapshot>) -> bool {
    current
        .map(|current| {
            (snapshot.last_updated, snapshot.start_time)
                >= (current.last_updated, current.start_time)
        })
        .unwrap_or(true)
}
