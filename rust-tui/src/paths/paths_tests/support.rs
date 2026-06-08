use std::path::Path;

pub(super) fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
    crate::test_support::with_temp_home("pad-paths", name, f)
}
