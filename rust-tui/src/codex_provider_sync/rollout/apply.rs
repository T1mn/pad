use super::line::split_first_line;
use super::RolloutChange;
use std::fs;
use std::io;

pub(in crate::codex_provider_sync) fn apply_rollout_changes(
    changes: &[RolloutChange],
) -> io::Result<usize> {
    let mut updated = 0usize;
    for change in changes {
        apply_rollout_change(change)?;
        updated += 1;
    }
    Ok(updated)
}

fn apply_rollout_change(change: &RolloutChange) -> io::Result<()> {
    let current = fs::read_to_string(&change.path)?;
    let (first_line, separator, rest) = split_first_line(&current);

    if first_line != change.original_first_line || separator != change.original_separator {
        return Err(io::Error::other(format!(
            "rollout file changed during provider sync: {}",
            change.path.display()
        )));
    }

    let updated_content = format!(
        "{}{}{}",
        change.updated_first_line, change.original_separator, rest
    );
    let tmp_path = change
        .path
        .with_extension(format!("jsonl.pad-sync.{}", std::process::id()));
    fs::write(&tmp_path, updated_content)?;
    fs::rename(tmp_path, &change.path)?;
    Ok(())
}
