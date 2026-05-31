use super::parse::parse_diff_document;
use super::render_diff_patch;
use super::styles::{ADD_BG, DELETE_BG, SEPARATOR};

const PATCH: &str = concat!(
    "Codex turn diff\n\n",
    "diff --git a/a.rs b/a.rs\n",
    "index 111..222 100644\n",
    "@@ -1,2 +1,2 @@\n",
    " keep\n",
    "-old\n",
    "+new\n",
);

#[test]
fn parser_pairs_delete_and_add_as_changed_rows() {
    let doc = parse_diff_document("diff --git a/a.rs b/a.rs\n@@ -7 +7 @@\n-old\n+new\n");
    let row = &doc.files[0].hunks[0].rows[0];

    assert_eq!(format!("{:?}", row.kind), "Change");
    assert_eq!(row.old_no, Some(7));
    assert_eq!(row.new_no, Some(7));
    assert_eq!(row.old_text, "old");
    assert_eq!(row.new_text, "new");
}

#[test]
fn wide_preview_renders_side_by_side_columns() {
    let text = render_diff_patch(PATCH, 140);
    let joined = joined_text(&text);

    assert!(joined.contains("old"));
    assert!(joined.contains("new"));
    assert!(joined.contains(SEPARATOR));
    assert!(text.lines.iter().any(|line| {
        line.spans
            .iter()
            .any(|span| span.style.bg == Some(DELETE_BG))
    }));
    assert!(text
        .lines
        .iter()
        .any(|line| { line.spans.iter().any(|span| span.style.bg == Some(ADD_BG)) }));
}

#[test]
fn narrow_preview_renders_enhanced_unified_rows() {
    let text = render_diff_patch(PATCH, 80);
    let joined = joined_text(&text);

    assert!(joined.contains("-    2      │ old"));
    assert!(joined.contains("+         2 │ new"));
}

fn joined_text(text: &ratatui::text::Text<'_>) -> String {
    text.lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
