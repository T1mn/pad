use super::model::{DiffDocument, DiffFile, DiffHunk, DiffRow, DiffRowKind};

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

fn diff_title(line: &str) -> String {
    let mut parts = line.split_whitespace().skip(2);
    let left = parts.next().unwrap_or("");
    let right = parts.next().unwrap_or(left);
    let right = right.strip_prefix("b/").unwrap_or(right);
    let left = left.strip_prefix("a/").unwrap_or(left);
    if right == "/dev/null" || right.is_empty() {
        left.to_string()
    } else {
        right.to_string()
    }
}

struct HunkBuilder {
    header: String,
    rows: Vec<DiffRow>,
    old_no: usize,
    new_no: usize,
}

impl HunkBuilder {
    fn new(header: &str) -> Self {
        let (old_no, new_no) = parse_hunk_start(header);
        Self {
            header: header.to_string(),
            rows: Vec::new(),
            old_no,
            new_no,
        }
    }

    fn push(&mut self, line: &str) {
        if line.starts_with('-') && !line.starts_with("---") {
            self.rows.push(DiffRow {
                old_no: Some(self.old_no),
                new_no: None,
                old_text: strip_marker(line),
                new_text: String::new(),
                kind: DiffRowKind::Delete,
            });
            self.old_no += 1;
        } else if line.starts_with('+') && !line.starts_with("+++") {
            self.rows.push(DiffRow {
                old_no: None,
                new_no: Some(self.new_no),
                old_text: String::new(),
                new_text: strip_marker(line),
                kind: DiffRowKind::Add,
            });
            self.new_no += 1;
        } else if let Some(text) = line.strip_prefix(' ') {
            self.rows.push(DiffRow {
                old_no: Some(self.old_no),
                new_no: Some(self.new_no),
                old_text: text.to_string(),
                new_text: text.to_string(),
                kind: DiffRowKind::Context,
            });
            self.old_no += 1;
            self.new_no += 1;
        }
    }

    fn finish(mut self) -> DiffHunk {
        self.rows = pair_adjacent_changes(self.rows);
        DiffHunk {
            header: self.header,
            rows: self.rows,
        }
    }
}

fn pair_adjacent_changes(rows: Vec<DiffRow>) -> Vec<DiffRow> {
    let mut out = Vec::new();
    let mut idx = 0;
    while idx < rows.len() {
        if rows[idx].kind != DiffRowKind::Delete {
            out.push(rows[idx].clone());
            idx += 1;
            continue;
        }

        let del_start = idx;
        while idx < rows.len() && rows[idx].kind == DiffRowKind::Delete {
            idx += 1;
        }
        let add_start = idx;
        while idx < rows.len() && rows[idx].kind == DiffRowKind::Add {
            idx += 1;
        }
        let dels = &rows[del_start..add_start];
        let adds = &rows[add_start..idx];
        if adds.is_empty() {
            out.extend_from_slice(dels);
            continue;
        }
        for i in 0..dels.len().max(adds.len()) {
            match (dels.get(i), adds.get(i)) {
                (Some(left), Some(right)) => out.push(DiffRow {
                    old_no: left.old_no,
                    new_no: right.new_no,
                    old_text: left.old_text.clone(),
                    new_text: right.new_text.clone(),
                    kind: DiffRowKind::Change,
                }),
                (Some(left), None) => out.push(left.clone()),
                (None, Some(right)) => out.push(right.clone()),
                (None, None) => {}
            }
        }
    }
    out
}

fn strip_marker(line: &str) -> String {
    line.chars().skip(1).collect()
}

fn parse_hunk_start(header: &str) -> (usize, usize) {
    let mut old_no = 1;
    let mut new_no = 1;
    for part in header.split_whitespace() {
        if let Some(value) = part.strip_prefix('-') {
            old_no = parse_range_start(value);
        } else if let Some(value) = part.strip_prefix('+') {
            new_no = parse_range_start(value);
        }
    }
    (old_no, new_no)
}

fn parse_range_start(value: &str) -> usize {
    value
        .split(',')
        .next()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
}
