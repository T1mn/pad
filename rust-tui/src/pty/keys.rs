/// Find detach key in buffer using multiple encoding formats.
pub fn find_detach_key(data: &[u8], detach_byte: u8) -> Option<usize> {
    if let Some(pos) = data.iter().position(|&b| b == detach_byte) {
        return Some(pos);
    }

    let key_code = match detach_byte {
        1..=26 => detach_byte + 96,
        28..=31 => detach_byte + 64,
        _ => 0,
    };

    if key_code > 0 {
        let xterm_seq = format!("\x1b[27;5;{}~", key_code);
        if let Some(pos) = data
            .windows(xterm_seq.len())
            .position(|w| w == xterm_seq.as_bytes())
        {
            return Some(pos);
        }

        let kitty_seq = format!("\x1b[{};5u", key_code);
        if let Some(pos) = data
            .windows(kitty_seq.len())
            .position(|w| w == kitty_seq.as_bytes())
        {
            return Some(pos);
        }
    }

    None
}

/// Find F12 key sequence.
pub fn find_f12_key(data: &[u8]) -> Option<usize> {
    if let Some(pos) = data.windows(5).position(|w| w == b"\x1b[24~") {
        return Some(pos);
    }

    if data.len() >= 6 {
        if let Some(pos) = data.windows(4).position(|w| w == b"\x1b[24") {
            if data[pos + 4] == b';' {
                for byte in data.iter().skip(pos + 5) {
                    if *byte == b'~' {
                        return Some(pos);
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
