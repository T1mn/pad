use super::super::model::{DiffRow, DiffRowKind};

pub(super) fn pair_adjacent_changes(rows: Vec<DiffRow>) -> Vec<DiffRow> {
    let mut out = Vec::with_capacity(rows.len());
    let mut rows = rows.into_iter().peekable();
    while let Some(row) = rows.next() {
        if row.kind != DiffRowKind::Delete {
            out.push(row);
            continue;
        }

        let mut dels = vec![row];
        while rows
            .peek()
            .is_some_and(|row| row.kind == DiffRowKind::Delete)
        {
            dels.push(rows.next().expect("peeked row"));
        }

        let mut adds = Vec::new();
        while rows.peek().is_some_and(|row| row.kind == DiffRowKind::Add) {
            adds.push(rows.next().expect("peeked row"));
        }

        if adds.is_empty() {
            out.extend(dels);
            continue;
        }

        let mut dels = dels.into_iter();
        let mut adds = adds.into_iter();
        loop {
            match (dels.next(), adds.next()) {
                (Some(left), Some(right)) => out.push(DiffRow {
                    old_no: left.old_no,
                    new_no: right.new_no,
                    old_text: left.old_text,
                    new_text: right.new_text,
                    kind: DiffRowKind::Change,
                }),
                (Some(left), None) => out.push(left),
                (None, Some(right)) => out.push(right),
                (None, None) => break,
            }
        }
    }
    out
}
