use super::archive_thread;

#[test]
fn missing_thread_archive_returns_not_found() {
    let err = archive_thread("missing-session").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}
