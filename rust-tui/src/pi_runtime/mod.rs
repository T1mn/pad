//! Pure Pi RPC integration primitives.
//!
//! The desktop host can run Pi as an isolated `--mode rpc` sidecar and feed
//! these codecs/reducers from any process supervisor. No Pi TUI is embedded,
//! and no Codex/ChatGPT session path is opened by this module.

pub(crate) mod approval;
pub(crate) mod events;
pub(crate) mod jsonl;
#[allow(
    dead_code,
    reason = "the read-only Pi session index is staged for history recovery and remains test-covered"
)]
pub(crate) mod session_index;
pub(crate) mod supervisor;

pub(crate) use approval::{pi_policy_operation, PiApprovalRequest, PiApprovalResponse};
#[cfg(test)]
pub(crate) use events::PiEventKind;
pub(crate) use events::{PiEvent, PiEventReducer, PiRuntimeSnapshot, PiRuntimeStatus};
pub(crate) use jsonl::{encode_command, JsonlCodec, JsonlError, PiMessage};
pub(crate) use supervisor::{PiPoll, PiRpcSupervisor, PiSupervisorError};

pub(crate) const PAD_FAST_MODE_EXTENSION: &str = include_str!("pad-fast-mode.ts");
pub(crate) const PAD_FAST_MODE_FILE: &str = "pad-fast-mode";

pub(crate) fn set_profile_fast_mode(
    profile: &crate::permission_policy::Profile,
    enabled: bool,
) -> std::io::Result<()> {
    let (agent_dir, _) = profile_pi_roots(profile);
    crate::paths::base::ensure_private_dir(&agent_dir)?;
    crate::atomic_file::write_private(
        &agent_dir.join(PAD_FAST_MODE_FILE),
        if enabled { "on\n" } else { "off\n" },
    )
}

/// Resolve the Pi executable owned by the Desktop host.
///
/// Packaged builds place the Rust host at `Contents/Resources/pad` and the Pi
/// launcher at `Contents/Resources/bin/pi`.  Development builds may instead
/// use a system installation.  The renderer never supplies this value.
pub(crate) fn desktop_pi_program() -> std::path::PathBuf {
    if let Ok(host) = std::env::current_exe() {
        if let Some(program) = bundled_pi_program_for_host(&host) {
            return program;
        }
    }
    ["/opt/homebrew/bin/pi", "/usr/local/bin/pi", "/usr/bin/pi"]
        .into_iter()
        .map(std::path::PathBuf::from)
        .find(|candidate| is_executable_file(candidate))
        .unwrap_or_else(|| std::path::PathBuf::from("pi"))
}

fn bundled_pi_program_for_host(host: &std::path::Path) -> Option<std::path::PathBuf> {
    let host_dir = host.parent()?;
    let mut candidates = vec![host_dir.join("bin").join("pi")];
    if let Some(contents_dir) = host_dir.parent() {
        candidates.push(contents_dir.join("Resources").join("bin").join("pi"));
    }
    candidates
        .into_iter()
        .find(|candidate| is_executable_file(candidate))
        .and_then(|candidate| candidate.canonicalize().ok().or(Some(candidate)))
}

fn is_executable_file(path: &std::path::Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// String-based provider/profile mapping keeps this module usable while the
/// legacy terminal enums are being extended by the host integration.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "native profile detection remains as a tested compatibility entry point"
    )
)]
pub(crate) fn is_pi_agent(agent_name: &str, command: &str) -> bool {
    agent_name.trim().eq_ignore_ascii_case("pi")
        || command
            .split_whitespace()
            .next()
            .and_then(|token| token.rsplit_once('/').map(|(_, name)| name).or(Some(token)))
            .is_some_and(|token| token.eq_ignore_ascii_case("pi"))
}

pub(crate) fn pi_command_or_default(command: &str) -> &str {
    let command = command.trim();
    if command.is_empty() {
        "pi"
    } else {
        command
    }
}

/// Build a shell command for the native terminal profile. Pi's config and
/// session roots are deliberately placed under PAD's private namespace and
/// are never pointed at `~/.codex` or the user's standalone `~/.pi` store.
pub(crate) fn build_pi_rpc_command(command: &str) -> String {
    let command = pi_command_or_default(command);
    let (agent_dir, session_dir) = default_pi_roots();
    build_pi_rpc_command_with_roots(command, &agent_dir, &session_dir)
}

/// Build the Desktop launch command for one persisted Profile.  Profile
/// roots are PAD-owned values, so a task switching accounts never falls back
/// to another profile's Pi config or session journal.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "free-form profile launch remains only for compatibility tests; Desktop uses the fixed executable path"
    )
)]
pub(crate) fn build_pi_rpc_command_for_profile(
    command: &str,
    profile: &crate::permission_policy::Profile,
) -> String {
    let command = pi_command_or_default(command);
    let (agent_dir, session_dir) = profile_pi_roots(profile);
    build_pi_rpc_command_with_roots(command, &agent_dir, &session_dir)
}

/// Resolve the private roots used by a Desktop Profile.  Older records may
/// have empty path fields; those records get a deterministic path below the
/// Desktop application-data root until the settings migration fills them.
pub(crate) fn profile_pi_roots(
    profile: &crate::permission_policy::Profile,
) -> (std::path::PathBuf, std::path::PathBuf) {
    let fallback = crate::paths::pad_desktop_data_dir()
        .join("v1")
        .join("profiles")
        .join(profile_storage_segment(&profile.id));
    let agent_dir = if profile.agent_dir.as_os_str().is_empty() {
        fallback.join("pi-agent")
    } else {
        profile.agent_dir.clone()
    };
    let session_dir = if profile.session_dir.as_os_str().is_empty() {
        fallback.join("pi-sessions")
    } else {
        profile.session_dir.clone()
    };
    (agent_dir, session_dir)
}

fn default_pi_roots() -> (std::path::PathBuf, std::path::PathBuf) {
    let agent_dir = crate::paths::pad_home_dir().join("pi-agent");
    let session_dir = agent_dir.join("sessions");
    (agent_dir, session_dir)
}

fn build_pi_rpc_command_with_roots(
    command: &str,
    agent_dir: &std::path::Path,
    session_dir: &std::path::Path,
) -> String {
    let command = if command
        .split_whitespace()
        .any(|argument| argument == "--mode=rpc")
        || command
            .split_whitespace()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|pair| pair == ["--mode", "rpc"])
    {
        command.to_string()
    } else {
        format!("{command} --mode rpc")
    };
    format!(
        "env PI_CODING_AGENT_DIR={} PI_CODING_AGENT_SESSION_DIR={} {command}",
        crate::shell_quote::single_quote(&agent_dir.to_string_lossy()),
        crate::shell_quote::single_quote(&session_dir.to_string_lossy()),
    )
}

pub(crate) fn profile_storage_segment(profile_id: &str) -> String {
    if profile_id.is_empty() {
        return "default".to_string();
    }
    let mut segment = String::with_capacity(profile_id.len());
    for byte in profile_id.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.') {
            segment.push(char::from(*byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(segment, "%{byte:02X}");
        }
    }
    match segment.as_str() {
        "." => "%2E".to_string(),
        ".." => "%2E%2E".to_string(),
        _ => segment,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn pi_agent_detection_accepts_binary_paths() {
        assert!(is_pi_agent("pi", "pi --mode rpc"));
        assert!(is_pi_agent("custom", "/opt/tools/pi"));
        assert!(!is_pi_agent("codex", "codex"));
    }

    pub(crate) fn pi_command_defaults_without_rewriting_explicit_commands() {
        assert_eq!(pi_command_or_default(""), "pi");
        assert_eq!(
            pi_command_or_default("  /opt/pi --mode rpc  "),
            "/opt/pi --mode rpc"
        );
    }

    pub(crate) fn pi_rpc_command_isolated_from_codex_and_pi_homes() {
        let command = build_pi_rpc_command("pi");
        assert!(command.contains("PI_CODING_AGENT_DIR='/"));
        assert!(command.contains("PI_CODING_AGENT_SESSION_DIR='/"));
        assert!(command.contains("/pi-agent"));
        assert!(command.ends_with(" pi --mode rpc"));
        assert!(!command.contains(".codex"));
        assert!(!command.contains("/.pi"));
    }

    pub(crate) fn profile_pi_roots_are_isolated_and_safe_for_empty_records() {
        let profile = crate::permission_policy::Profile {
            id: "account/acme".to_string(),
            ..Default::default()
        };
        let (agent_dir, session_dir) = profile_pi_roots(&profile);
        assert!(agent_dir.ends_with("v1/profiles/account%2Facme/pi-agent"));
        assert!(session_dir.ends_with("v1/profiles/account%2Facme/pi-sessions"));
        let command = build_pi_rpc_command_for_profile("pi", &profile);
        assert!(command.contains("account%2Facme/pi-agent"));
        assert!(command.contains("account%2Facme/pi-sessions"));
        assert!(!command.contains(".codex"));
        assert!(!command.contains("/.pi"));
    }

    pub(crate) fn profile_storage_segments_are_injective_for_unsafe_ids() {
        let values = [
            "a/b",
            "a?b",
            "a_b",
            "a%2Fb",
            ".",
            "..",
            "账号/甲",
            "账号?甲",
        ];
        let segments = values
            .iter()
            .map(|value| profile_storage_segment(value))
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(segments.len(), values.len());
        assert_eq!(profile_storage_segment("a/b"), "a%2Fb");
        assert_eq!(profile_storage_segment("a?b"), "a%3Fb");
        assert_eq!(profile_storage_segment("a%2Fb"), "a%252Fb");
        assert!(!profile_storage_segment("账号/甲").contains('/'));
    }

    #[cfg(unix)]
    pub(crate) fn desktop_pi_program_prefers_the_host_bundle() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let root = std::env::temp_dir().join(format!(
            "pad-desktop-pi-bundle-{}-{}",
            std::process::id(),
            crate::time::unix_now_nanos()
        ));
        let host = root.join("Contents/Resources/pad");
        let pi = root.join("Contents/Resources/bin/pi");
        fs::create_dir_all(pi.parent().unwrap()).unwrap();
        fs::write(&host, "host").unwrap();
        fs::write(&pi, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&pi, fs::Permissions::from_mode(0o700)).unwrap();

        assert_eq!(
            bundled_pi_program_for_host(&host).unwrap(),
            pi.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(root);
    }
}
