use super::*;
use serde_json::json;
use std::thread;

fn wait_for_output(supervisor: &PiSupervisor) -> PiPoll {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut output = PiPoll::default();
    while Instant::now() < deadline {
        let next = supervisor.poll().expect("poll Pi child");
        output.messages.extend(next.messages);
        output.events.extend(next.events);
        output.stderr.extend(next.stderr);
        output.diagnostics.extend(next.diagnostics);
        output.dropped_stale += next.dropped_stale;
        output.exit_status = next.exit_status.or(output.exit_status);
        if output.exit_status.is_some() && !output.events.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    output
}

pub(crate) fn command_environment_is_private_and_generation_scoped() {
    let supervisor = PiSupervisor::spawn(
        "env PI_CODING_AGENT_DIR='/tmp/wrong-codex' PI_CODING_AGENT_SESSION_DIR='/tmp/wrong-pi' /bin/sh -c 'printf \"%s\\n%s\\n\" \"$PI_CODING_AGENT_DIR\" \"$PI_CODING_AGENT_SESSION_DIR\"'",
        ".",
        9101,
    )
    .unwrap();
    let output = wait_for_output(&supervisor);
    let text = String::from_utf8(output.stderr).unwrap_or_default();
    let root = supervisor.root().unwrap();
    assert!(root.ends_with("pi-agent/rpc/9101"));
    assert!(!root.to_string_lossy().contains(".codex"));
    assert!(!root.to_string_lossy().ends_with("/.pi"));
    let _ = supervisor.kill();
    assert!(text.is_empty());
}

pub(crate) fn malformed_frame_does_not_poison_a_later_valid_event() {
    let supervisor = PiSupervisor::spawn(
        "/bin/sh -c 'printf \"%s\\n%s\\n\" \"not-json\" \"{\\\"type\\\":\\\"agent_settled\\\"}\"'",
        ".",
        9102,
    )
    .unwrap();
    let mut output = supervisor.poll().unwrap();
    for _ in 0..30 {
        if !output.events.is_empty() {
            break;
        }
        let next = supervisor.poll().unwrap();
        output.diagnostics.extend(next.diagnostics);
        output.events.extend(next.events);
        thread::sleep(Duration::from_millis(5));
    }
    assert!(!output.diagnostics.is_empty());
    assert_eq!(
        output.events[0].kind,
        super::super::PiEventKind::AgentSettled
    );
    let _ = supervisor.kill();
}

pub(crate) fn stale_generation_messages_are_dropped() {
    let supervisor = PiSupervisor::spawn(
        "/bin/sh -c 'printf \"%s\\n%s\\n\" \"{\\\"type\\\":\\\"agent_start\\\",\\\"generation\\\":1}\" \"{\\\"type\\\":\\\"agent_start\\\",\\\"generation\\\":9103}\"'",
        ".",
        9103,
    )
    .unwrap();
    let output = wait_for_output(&supervisor);
    assert_eq!(output.dropped_stale, 1);
    assert_eq!(output.events.len(), 1);
    assert_eq!(output.events[0].generation, Some(9103));
    let _ = supervisor.kill();
}

pub(crate) fn shutdown_kills_a_stuck_child_without_hanging() {
    let supervisor = PiSupervisor::spawn("/bin/sleep 30", ".", 9104).unwrap();
    let started = Instant::now();
    let status = supervisor.shutdown().unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(status.killed || !status.success());
}

pub(crate) fn send_rejects_non_object_commands() {
    let supervisor = PiSupervisor::spawn("/bin/sleep 30", ".", 9105).unwrap();
    let error = supervisor.send(json!("not a command")).unwrap_err();
    assert!(matches!(
        error,
        PiSupervisorError::Jsonl(JsonlError::NonObjectMessage)
    ));
    let _ = supervisor.kill();
}

pub(crate) fn profile_spawn_uses_profile_specific_agent_and_session_roots() {
    let profile_root = std::env::temp_dir().join(format!(
        "pad-pi-profile-supervisor-{}-{}",
        std::process::id(),
        9106
    ));
    let profile = crate::permission_policy::Profile {
        id: "profile-a".to_string(),
        agent_dir: profile_root.join("agent"),
        session_dir: profile_root.join("sessions"),
        ..Default::default()
    };
    let supervisor = PiSupervisor::spawn_for_profile("/bin/sleep 30", ".", 9106, &profile)
        .expect("spawn profile-scoped Pi process");
    let expected_root = profile.agent_dir.clone();
    let expected_sessions = profile.session_dir.clone();
    assert_eq!(supervisor.root().unwrap(), expected_root);
    assert!(expected_root.is_dir());
    assert!(expected_sessions.is_dir());
    let _ = supervisor.kill();
    let _ = fs::remove_dir_all(profile_root);
}

pub(crate) fn profile_spawn_rejects_provider_owned_roots() {
    let profile = crate::permission_policy::Profile {
        id: "profile-b".to_string(),
        agent_dir: std::env::temp_dir().join(".codex").join("pad-agent"),
        session_dir: std::env::temp_dir().join("pad-session"),
        ..Default::default()
    };
    let error = PiSupervisor::spawn_for_profile("/bin/sleep 1", ".", 9107, &profile)
        .err()
        .expect("provider-owned root must be rejected");
    assert!(
        matches!(error, PiSupervisorError::InvalidCommand(message) if message.contains("provider-owned"))
    );
}

pub(crate) fn profile_spawn_rejects_session_path_outside_profile_root() {
    let profile_root = std::env::temp_dir().join(format!(
        "pad-pi-profile-session-{}-{}",
        std::process::id(),
        9108
    ));
    let profile = crate::permission_policy::Profile {
        id: "profile-c".to_string(),
        agent_dir: profile_root.join("agent"),
        session_dir: profile_root.join("sessions"),
        ..Default::default()
    };
    let error = PiSupervisor::spawn_for_profile(
        "pi --session /tmp/not-a-pad-session.jsonl",
        ".",
        9108,
        &profile,
    )
    .err()
    .expect("session path outside profile root must be rejected");
    assert!(
        matches!(error, PiSupervisorError::InvalidCommand(message) if message.contains("outside the Profile session root"))
    );
    let _ = fs::remove_dir_all(profile_root);
}

#[cfg(unix)]
pub(crate) fn profile_spawn_rejects_session_symlink_escape() {
    use std::os::unix::fs::symlink;

    let profile_root = std::env::temp_dir().join(format!(
        "pad-pi-profile-symlink-{}-{}",
        std::process::id(),
        9109
    ));
    let outside = profile_root.join("outside");
    let profile = crate::permission_policy::Profile {
        id: "profile-d".to_string(),
        agent_dir: profile_root.join("agent"),
        session_dir: profile_root.join("sessions"),
        ..Default::default()
    };
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(&profile.session_dir).unwrap();
    symlink(&outside, profile.session_dir.join("link")).unwrap();
    let session = profile.session_dir.join("link").join("escape.jsonl");
    let error = PiSupervisor::spawn_for_profile(
        &format!("pi --session {}", session.display()),
        ".",
        9109,
        &profile,
    )
    .err()
    .expect("session symlink escape must be rejected");
    assert!(
        matches!(error, PiSupervisorError::InvalidCommand(message) if message.contains("outside the Profile session root"))
    );
    let _ = fs::remove_dir_all(profile_root);
}
