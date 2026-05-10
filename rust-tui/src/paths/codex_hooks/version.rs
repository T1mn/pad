use std::process::Command;

const NEW_HOOKS_FEATURE_VERSION: (u64, u64, u64) = (0, 130, 0);

pub(crate) fn codex_hooks_feature_key_for_version(version: Option<&str>) -> &'static str {
    match version.and_then(parse_codex_cli_version) {
        Some(version) if version >= NEW_HOOKS_FEATURE_VERSION => "hooks",
        _ => "codex_hooks",
    }
}

pub(super) fn detect_codex_cli_version() -> Option<String> {
    Command::new("codex")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|raw| {
            raw.split_whitespace()
                .rev()
                .find(|token| parse_codex_cli_version(token).is_some())
                .map(str::to_string)
        })
}

pub(crate) fn parse_codex_cli_version(raw: &str) -> Option<(u64, u64, u64)> {
    let clean = raw.trim().trim_start_matches('v');
    let mut parts = clean.split(['.', '-']);
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}
