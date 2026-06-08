use super::super::model::{DiffRow, DiffRowKind};

pub(super) fn pair_adjacent_changes(rows: Vec<DiffRow>) -> Vec<DiffRow> {
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
