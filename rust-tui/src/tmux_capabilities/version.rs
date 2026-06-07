#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TmuxVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: Option<u32>,
    pub suffix: Option<String>,
}

pub(super) fn parse_tmux_version(raw: &str) -> Option<TmuxVersion> {
    let version = raw.strip_prefix("tmux ")?;
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor_part = parts.next()?;
    let minor_digits: String = minor_part
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    let minor = minor_digits.parse().ok()?;
    let minor_suffix = minor_part[minor_digits.len()..].trim();

    let third_part = parts.next();
    let (patch, suffix) = if let Some(third_part) = third_part {
        let patch_digits: String = third_part
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect();
        let patch = if patch_digits.is_empty() {
            None
        } else {
            patch_digits.parse().ok()
        };
        let patch_suffix = third_part[patch_digits.len()..].trim();
        let suffix = if patch_suffix.is_empty() {
            minor_suffix
        } else if minor_suffix.is_empty() {
            patch_suffix
        } else {
            return None;
        };
        (
            patch,
            if suffix.is_empty() {
                None
            } else {
                Some(suffix.to_string())
            },
        )
    } else {
        (
            None,
            if minor_suffix.is_empty() {
                None
            } else {
                Some(minor_suffix.to_string())
            },
        )
    };

    Some(TmuxVersion {
        major,
        minor,
        patch,
        suffix,
    })
}
