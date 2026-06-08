use super::{command_args_may_name_agent, command_may_hide_agent};

#[test]
fn distinguishes_shells_from_arg_wrappers() {
    assert!(command_may_hide_agent("/bin/zsh -l"));
    assert!(!command_args_may_name_agent("/bin/zsh -l"));
    assert!(command_may_hide_agent("node"));
    assert!(command_args_may_name_agent("node"));
}

#[test]
fn wrapper_detection_is_ascii_case_insensitive_without_allocating() {
    assert!(command_may_hide_agent("/BIN/ZSH -l"));
    assert!(command_args_may_name_agent("/usr/local/bin/Node server.js"));
}
