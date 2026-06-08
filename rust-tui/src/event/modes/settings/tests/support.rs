pub(super) fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
    crate::test_support::with_temp_home("pad-settings", name, |_| f())
}
