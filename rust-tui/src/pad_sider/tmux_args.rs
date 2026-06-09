pub(super) fn format_tmux_args(args: &[&str]) -> String {
    let mut formatted = String::new();
    for arg in args {
        if !formatted.is_empty() {
            formatted.push(' ');
        }
        formatted.push_str(arg);
    }
    formatted
}

#[cfg(test)]
#[path = "tmux_args_tests.rs"]
mod tests;
