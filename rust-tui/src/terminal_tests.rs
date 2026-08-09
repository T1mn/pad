use super::{panic_requires_terminal_restore, KITTY_PAD_ACTIVE, KITTY_PAD_INACTIVE};

pub(crate) fn isolated_worker_boundary_keeps_terminal_active() {
    assert_eq!(KITTY_PAD_ACTIVE, "\x1b]1337;SetUserVar=pad=MQ==\x07");
    assert_eq!(KITTY_PAD_INACTIVE, "\x1b]1337;SetUserVar=pad\x07");
    assert!(panic_requires_terminal_restore());
    crate::panic_boundary::catch_isolated_unwind(|| {
        assert!(!panic_requires_terminal_restore());
    })
    .unwrap();
    assert!(panic_requires_terminal_restore());
}
