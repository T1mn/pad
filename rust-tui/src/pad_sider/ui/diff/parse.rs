mod hunk;
mod pairing;
mod title;

use super::model::{DiffDocument, DiffFile};
use hunk::HunkBuilder;
use title::diff_title;

pub(super) fn parse_diff_document(content: &str) -> DiffDocument {
    let mut prelude = Vec::new();
    let mut files = Vec::new();
    let mut current_file: Option<DiffFile> = None;
    let mut current_hunk: Option<HunkBuilder> = None;

    for line in content.lines() {
        if line.starts_with("diff --git ") {
            flush_hunk(&mut current_file, &mut current_hunk);
            if let Some(file) = current_file.take() {
                files.push(file);
            }
            current_file = Some(DiffFile {
                title: diff_title(line),
                meta: Vec::new(),
                hunks: Vec::new(),
            });
            continue;
        }

        let Some(file) = current_file.as_mut() else {
            prelude.push(line.to_string());
            continue;
        };

        if line.starts_with("@@") {
            flush_hunk(&mut current_file, &mut current_hunk);
            current_hunk = Some(HunkBuilder::new(line));
        } else if let Some(hunk) = current_hunk.as_mut() {
            hunk.push(line);
        } else if !line.starts_with("--- ") && !line.starts_with("+++ ") {
            file.meta.push(line.to_string());
        }
    }

    flush_hunk(&mut current_file, &mut current_hunk);
    if let Some(file) = current_file {
        files.push(file);
    }

    DiffDocument { prelude, files }
}

fn flush_hunk(file: &mut Option<DiffFile>, hunk: &mut Option<HunkBuilder>) {
    if let (Some(file), Some(hunk)) = (file.as_mut(), hunk.take()) {
        file.hunks.push(hunk.finish());
    }
}
