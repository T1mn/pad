use super::{format_args, format_command_line};

#[test]
fn command_line_format_includes_program_and_args() {
    assert_eq!(
        format_command_line("open", ["https://example.com", "--background"]),
        "open https://example.com --background"
    );
}

#[test]
fn args_format_keeps_space_separated_remote_command() {
    let args = vec!["echo".to_string(), "hello".to_string()];
    assert_eq!(format_args(&args), "echo hello");
}
