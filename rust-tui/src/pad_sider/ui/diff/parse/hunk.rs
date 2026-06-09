use super::super::model::{DiffHunk, DiffRow, DiffRowKind};
use super::pairing::pair_adjacent_changes;

pub(super) struct HunkBuilder {
    header: String,
    rows: Vec<DiffRow>,
    old_no: usize,
    new_no: usize,
}

impl HunkBuilder {
    pub(super) fn new(header: &str) -> Self {
        let (old_no, new_no) = parse_hunk_start(header);
        Self {
            header: header.to_string(),
            rows: Vec::new(),
            old_no,
            new_no,
        }
    }

    pub(super) fn push(&mut self, line: &str) {
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

    pub(super) fn finish(mut self) -> DiffHunk {
        self.rows = pair_adjacent_changes(self.rows);
        DiffHunk {
            header: self.header,
            rows: self.rows,
        }
    }
}

fn strip_marker(line: &str) -> String {
    line[1..].to_string()
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
