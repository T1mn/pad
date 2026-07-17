use super::scan::all_threads_at;
use std::fs;

#[test]
fn scans_official_summary_and_skips_corrupt_sessions() {
    let root = crate::test_support::temp_path("pad", "grok-history");
    let valid = root.join("encoded-cwd").join("session-1");
    fs::create_dir_all(&valid).unwrap();
    fs::write(
        valid.join("summary.json"),
        r#"{
          "info":{"id":"session-1","cwd":"/tmp/project"},
          "session_summary":"fallback",
          "generated_title":"Latest title",
          "created_at":"2026-07-17T00:00:00Z",
          "updated_at":"2026-07-17T00:01:00Z",
          "current_model_id":"grok-code-fast-1",
          "future_field":{"safe":true}
        }"#,
    )
    .unwrap();
    fs::write(valid.join("updates.jsonl"), "{}\n").unwrap();

    let corrupt = root.join("encoded-cwd").join("bad");
    fs::create_dir_all(&corrupt).unwrap();
    fs::write(corrupt.join("summary.json"), "{not json").unwrap();
    fs::write(corrupt.join("updates.jsonl"), "{}\n").unwrap();

    let threads = all_threads_at(&root).unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].session_id, "session-1");
    assert_eq!(threads[0].title.as_deref(), Some("Latest title"));
    assert_eq!(threads[0].model_name.as_deref(), Some("grok-code-fast-1"));
    assert_eq!(threads[0].cwd.to_string_lossy(), "/tmp/project");

    let _ = fs::remove_dir_all(root);
}
