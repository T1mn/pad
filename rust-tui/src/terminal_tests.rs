use super::panic_requires_terminal_restore;

#[test]
fn isolated_worker_boundary_keeps_terminal_active() {
    assert!(panic_requires_terminal_restore());
    crate::panic_boundary::catch_isolated_unwind(|| {
        assert!(!panic_requires_terminal_restore());
    })
    .unwrap();
    assert!(panic_requires_terminal_restore());
}
