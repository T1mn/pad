use super::opencode_stats_path;
use std::path::Path;

#[test]
fn opencode_stats_path_sanitizes_project() {
    assert_eq!(
        opencode_stats_path("/Users/tim/my repo", Path::new("/tmp/stats"), 42),
        Path::new("/tmp/stats/Users_tim_my_repo-42.txt")
    );
}
