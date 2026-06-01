use super::{normalize_prompt, prompt_preview, run_command};
use std::ffi::OsString;

#[test]
fn run_prompt_trims_outer_blank_space_but_keeps_multiline_body() {
    assert_eq!(
        normalize_prompt("\n  fix this\nkeep context  \n").unwrap(),
        "fix this\nkeep context"
    );
    assert!(normalize_prompt(" \n\t ").is_err());
}

#[test]
fn run_command_quotes_prompt_and_resumes_opencode_session() {
    assert_eq!(
        run_command(
            "fix Bob's bug\nnow",
            Some("ses'sion"),
            &OsString::from("/opt/open code/bin/opencode"),
        ),
        "'/opt/open code/bin/opencode' run --session 'ses'\\''sion' -- 'fix Bob'\\''s bug\nnow'"
    );
}

#[test]
fn run_command_can_start_new_session_without_selected_opencode_thread() {
    assert_eq!(
        run_command("hello", None, &OsString::from("opencode")),
        "'opencode' run -- 'hello'"
    );
}

#[test]
fn run_prompt_preview_uses_first_non_empty_line() {
    assert_eq!(prompt_preview("\nfirst\nsecond"), "first");
}
