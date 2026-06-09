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
        let quote_count = value.matches('\'').count();
        let mut quoted = String::with_capacity(value.len() + 2 + quote_count * 3);
        quoted.push('\'');
        if quote_count == 0 {
            quoted.push_str(value);
        } else {
            for ch in value.chars() {
                if ch == '\'' {
                    quoted.push_str(r#"'\''"#);
                } else {
                    quoted.push(ch);
                }
            }
        }
        quoted.push('\'');
        quoted
    }
}

#[cfg(test)]
#[path = "display_tests.rs"]
mod tests;
