use super::{parse_pane_info, PANE_INFO_SEP};
use std::path::PathBuf;

#[test]
fn pane_info_parser_reads_zoom_flag() {
    let raw = format!(
        "%7{PANE_INFO_SEP}@1{PANE_INFO_SEP}@2{PANE_INFO_SEP}codex{PANE_INFO_SEP}/tmp/repo{PANE_INFO_SEP}1"
    );
    let info = parse_pane_info(&raw).unwrap();

    assert_eq!(info.pane_id, "%7");
    assert_eq!(info.window_id, "@2");
    assert_eq!(info.command, "codex");
    assert_eq!(info.cwd, PathBuf::from("/tmp/repo"));
    assert!(info.zoomed);
}
