pub(super) fn temp_jsonl_path(name: &str) -> std::path::PathBuf {
    crate::test_support::temp_path("pad-preview-jsonl", name)
}
