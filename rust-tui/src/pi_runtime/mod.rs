//! Pure Pi RPC integration primitives.
//!
//! The desktop host can run Pi as an isolated `--mode rpc` sidecar and feed
//! these codecs/reducers from any process supervisor. No Pi TUI is embedded,
//! and no Codex/ChatGPT session path is opened by this module.

pub(crate) mod approval;
pub(crate) mod events;
pub(crate) mod jsonl;
pub(crate) mod session_index;
pub(crate) mod supervisor;

pub(crate) use approval::{
    classify_pi_approval, tool_operation, tool_target_path, PiApprovalAction, PiApprovalRequest,
    PiApprovalResponse,
};
pub(crate) use events::{PiEvent, PiEventKind, PiEventReducer, PiRuntimeSnapshot, PiRuntimeStatus};
pub(crate) use jsonl::{
    encode_command, encode_json_line, JsonlCodec, JsonlError, PiMessage, DEFAULT_MAX_FRAME_BYTES,
};
pub(crate) use session_index::{
    index_file as index_session_file, rebuild as rebuild_session_index, PiIndexedEntry,
    PiSessionIndex, PiSessionIndexCursor, SessionIndexError, SessionIndexRebuild,
};
pub(crate) use supervisor::{
    PiExitStatus, PiPoll, PiRpcSupervisor, PiSupervisorError, PiSupervisorMessage,
};

/// String-based provider/profile mapping keeps this module usable while the
/// legacy terminal enums are being extended by the host integration.
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
    let segment = profile_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    match segment.as_str() {
        "" | "." | ".." => "default".to_string(),
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
        assert!(agent_dir.ends_with("v1/profiles/account_acme/pi-agent"));
        assert!(session_dir.ends_with("v1/profiles/account_acme/pi-sessions"));
        let command = build_pi_rpc_command_for_profile("pi", &profile);
        assert!(command.contains("account_acme/pi-agent"));
        assert!(command.contains("account_acme/pi-sessions"));
        assert!(!command.contains(".codex"));
        assert!(!command.contains("/.pi"));
    }
}
