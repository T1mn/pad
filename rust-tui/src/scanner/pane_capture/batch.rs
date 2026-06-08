use super::ansi::strip_ansi;
use std::collections::HashMap;

pub(super) fn batch_capture_args(
    pane_ids: &[String],
    lines: usize,
    marker_prefix: &str,
) -> Vec<String> {
    let mut args = Vec::with_capacity(pane_ids.len() * 10);
    for (idx, pane_id) in pane_ids.iter().enumerate() {
        if idx > 0 {
            args.push(";".into());
        }
        args.extend([
            "display-message".to_string(),
            "-p".to_string(),
            format!("{marker_prefix}{idx}__"),
            ";".to_string(),
            "capture-pane".to_string(),
            "-p".to_string(),
            "-t".to_string(),
            pane_id.clone(),
            "-S".to_string(),
            format!("-{lines}"),
        ]);
    }
    args
}

pub(super) fn parse_batch_capture(
    stdout: &str,
    pane_ids: &[String],
    marker_prefix: &str,
) -> HashMap<String, String> {
    let mut captures = HashMap::with_capacity(pane_ids.len());
    let mut current_idx = None;
    let mut current = String::new();

    for line in stdout.lines() {
        if let Some(idx) = marker_index(line, marker_prefix) {
            flush_capture(&mut captures, pane_ids, current_idx, &mut current);
            current_idx = Some(idx);
            continue;
        }

        if current_idx.is_some() {
            current.push_str(line);
            current.push('\n');
        }
    }
    flush_capture(&mut captures, pane_ids, current_idx, &mut current);
    captures
}

fn marker_index(line: &str, marker_prefix: &str) -> Option<usize> {
    line.strip_prefix(marker_prefix)?
        .strip_suffix("__")?
        .parse::<usize>()
        .ok()
}

fn flush_capture(
    captures: &mut HashMap<String, String>,
    pane_ids: &[String],
    current_idx: Option<usize>,
    current: &mut String,
) {
    let Some(idx) = current_idx else {
        return;
    };
    let Some(pane_id) = pane_ids.get(idx) else {
        current.clear();
        return;
    };
    captures.insert(pane_id.clone(), strip_ansi(current));
    current.clear();
}

#[cfg(test)]
mod tests {
    use super::parse_batch_capture;

    #[test]
    fn batch_capture_parser_maps_marker_sections_to_panes() {
        let panes = vec!["%1".to_string(), "%2".to_string()];
        let captures = parse_batch_capture(
            "__PAD_CAPTURE_1_0__\nhello\x1b[31m red\x1b[0m\n__PAD_CAPTURE_1_1__\nwaiting\n",
            &panes,
            "__PAD_CAPTURE_1_",
        );

        assert_eq!(captures.get("%1").map(String::as_str), Some("hello red\n"));
        assert_eq!(captures.get("%2").map(String::as_str), Some("waiting\n"));
    }
}
