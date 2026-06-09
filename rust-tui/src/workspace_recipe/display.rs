use super::runner::RecipeCommand;

pub fn display_command(command: &RecipeCommand) -> String {
    let mut display = shell_quote(&command.program);
    for arg in &command.args {
        display.push(' ');
        display.push_str(&shell_quote(arg));
    }
    display
}

pub(super) fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | '='))
    {
        value.to_string()
    } else {
        crate::shell_quote::single_quote(value)
    }
}

#[cfg(test)]
#[path = "display_tests.rs"]
mod tests;
