pub(crate) fn single_quote(value: &str) -> String {
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

#[cfg(test)]
#[path = "shell_quote_tests.rs"]
mod tests;
