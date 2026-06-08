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
    let preferred = choose_preferred_snapshot(&snapshots)?;
    let updated_at = snapshots
        .iter()
        .map(|snapshot| snapshot.last_updated)
        .max()
        .unwrap_or(preferred.last_updated);
    let start_time = snapshots
        .iter()
        .map(|snapshot| snapshot.start_time)
        .min()
        .unwrap_or(preferred.start_time);
    let has_subagent = snapshots.iter().any(|snapshot| snapshot.kind == "subagent");
    let payload_hash = snapshots
        .iter()
        .map(|snapshot| snapshot.payload_hash.as_str())
        .collect::<Vec<_>>();
    let mut payload_hash = payload_hash;
    payload_hash.sort_unstable();
    let payload_hash = payload_hash.join(":");

    Some(GeminiThreadRecord {
        session_id: preferred.session_id.clone(),
        cwd: preferred.project_root.clone(),
        project_alias: preferred.project_alias.clone(),
        transcript_path: preferred.transcript_path.clone(),
        kind: preferred.kind.clone(),
        start_time,
        updated_at,
        title: preferred
            .summary
            .clone()
            .or_else(|| preferred.first_user_message.clone()),
        subtitle: preferred.last_user_message.clone(),
        summary: preferred.summary.clone(),
        first_user_message: preferred.first_user_message.clone(),
        last_user_message: preferred.last_user_message.clone(),
        last_assistant_message: preferred.last_assistant_message.clone(),
        has_subagent,
        payload_hash: md5_hex(&payload_hash),
        snapshot_count: snapshots.len() as i64,
    })
}

fn choose_preferred_snapshot(snapshots: &[GeminiSnapshot]) -> Option<&GeminiSnapshot> {
    snapshots
        .iter()
        .filter(|snapshot| snapshot.kind == "main")
        .max_by_key(|snapshot| (snapshot.last_updated, snapshot.start_time))
        .or_else(|| {
            snapshots
                .iter()
                .max_by_key(|snapshot| (snapshot.last_updated, snapshot.start_time))
        })
}
