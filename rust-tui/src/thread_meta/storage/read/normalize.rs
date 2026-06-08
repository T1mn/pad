use super::super::super::ThreadMeta;
use std::collections::HashSet;

pub(super) fn normalize_meta(meta: &mut ThreadMeta) {
    dedup_tags(&mut meta.tags);
    meta.title_override = meta.title_override.as_ref().and_then(|s| clean_text(s));
    meta.generated_title = meta.generated_title.as_ref().and_then(|s| clean_text(s));
    meta.note = meta.note.as_ref().and_then(|s| clean_text(s));
}

fn clean_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn dedup_tags(tags: &mut Vec<String>) {
    let mut seen = HashSet::new();
    tags.retain(|tag| seen.insert(tag.to_lowercase()));
}
