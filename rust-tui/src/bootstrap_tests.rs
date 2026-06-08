use super::*;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn bootstrap_needed_for_interactive_plain_launch() {
    assert!(should_bootstrap_into_tmux(
        &args(&["pad", "--debug"]),
        false,
        false,
        false,
        true,
        true,
    ));
}

#[test]
fn bootstrap_skips_info_and_daemon_commands() {
    assert!(!should_bootstrap_into_tmux(
        &args(&["pad", "--help"]),
        false,
        false,
        false,
        true,
        true,
    ));
    assert!(!should_bootstrap_into_tmux(
        &args(&["pad", "telegram-bot"]),
        false,
        false,
        false,
        true,
        true,
    ));
    assert!(!should_bootstrap_into_tmux(
        &args(&["pad", "__internal", "pad-sider", "toggle"]),
        false,
        false,
        false,
        true,
        true,
    ));
}

#[test]
fn bootstrap_skips_when_already_inside_tmux_or_reentered() {
    assert!(!should_bootstrap_into_tmux(
        &args(&["pad"]),
        true,
        true,
        false,
        true,
        true,
    ));
    assert!(!should_bootstrap_into_tmux(
        &args(&["pad"]),
        false,
        false,
        true,
        true,
        true,
    ));
}

#[test]
fn bootstrap_skips_without_interactive_terminal() {
    assert!(!should_bootstrap_into_tmux(
        &args(&["pad"]),
        false,
        false,
        false,
        false,
        true,
    ));
}

#[test]
fn bootstrap_command_quotes_executable_and_args() {
    let command = bootstrap_command(
        &args(&["pad", "--debug", "work tree"]),
        std::path::Path::new("/tmp/pad bin"),
    );
    assert_eq!(
        command,
        "env PAD_TMUX_BOOTSTRAPPED=1 '/tmp/pad bin' '--debug' 'work tree'"
    );
}
