/// Strip ANSI escape sequences and control characters from captured pane content.
pub fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            skip_escape_sequence(&mut chars);
        } else if c == '\n' || c == '\t' || !c.is_control() {
            result.push(c);
        }
    }
    result
}

fn skip_escape_sequence<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    if chars.peek() == Some(&'[') {
        chars.next();
        for nc in chars.by_ref() {
            if nc.is_ascii_alphabetic() || nc == 'm' || nc == '~' {
                break;
            }
        }
        return;
    }

    if chars.peek() == Some(&']') {
        chars.next();
        while let Some(oc) = chars.next() {
            if oc == '\x07' {
                break;
            }
            if oc == '\x1b' && chars.peek() == Some(&'\\') {
                chars.next();
                break;
            }
        }
        return;
    }

    chars.next();
}
