use super::shell::shell_display_quote;

pub fn display_tmux_command(args: &[String]) -> String {
    std::iter::once("tmux")
        .chain(args.iter().map(String::as_str))
        .map(shell_display_quote)
        .collect::<Vec<_>>()
        .join(" ")
}
