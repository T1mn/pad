pub(super) fn temp_dir(name: &str) -> std::path::PathBuf {
    crate::test_support::temp_path("pad-sider", name)
}
