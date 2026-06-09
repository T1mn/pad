use super::{shell_single_quote, wrap_tmux_run_shell};

#[test]
fn shell_single_quote_escapes_embedded_single_quotes() {
    assert_eq!(shell_single_quote("bob's pane"), r#"'bob'\''s pane'"#);
}

#[test]
fn wrap_tmux_run_shell_quotes_script() {
    assert_eq!(
        wrap_tmux_run_shell("tmux display-message 'ready'"),
        r#"sh -lc 'tmux display-message '\''ready'\'''"#
    );
}
