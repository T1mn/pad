use super::shell::shell_display_quote;

pub fn display_tmux_command(args: &[String]) -> String {
    let mut display = String::from("tmux");
    for arg in args {
        display.push(' ');
        display.push_str(&shell_display_quote(arg));
    }
    display
}

#[cfg(test)]
#[path = "display_tests.rs"]
mod tests;
