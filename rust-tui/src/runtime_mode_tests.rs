use super::*;

#[test]
fn native_is_default_and_tmux_requires_an_explicit_flag() {
    assert_eq!(RuntimeMode::from_args(&["pad".into()]), RuntimeMode::Native);
    assert_eq!(
        RuntimeMode::from_args(&["pad".into(), "--native".into()]),
        RuntimeMode::Native
    );
    assert_eq!(
        RuntimeMode::from_args(&["pad".into(), "--tmux".into()]),
        RuntimeMode::TmuxCompatibility
    );
}
