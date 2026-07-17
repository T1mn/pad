use super::{parse_json_string, parse_version, run_codex_cli_update};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn parse_codex_version_from_cli_output() {
    assert_eq!(
        parse_version("codex-cli 0.125.0"),
        Some("0.125.0".to_string())
    );
}

#[test]
fn parse_npm_version_json_output() {
    assert_eq!(
        parse_json_string("\"0.125.0\""),
        Some("0.125.0".to_string())
    );
}

#[cfg(unix)]
#[test]
fn update_uses_detected_codex_binary_native_command() {
    let temp_dir = unique_temp_dir("native-update");
    let binary = temp_dir.join("codex");
    let args_log = temp_dir.join("codex.args");
    fs::create_dir_all(&temp_dir).expect("create temp dir");
    fs::write(
        &binary,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"${0}.args\"\n",
    )
    .expect("write fake codex");
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).expect("make executable");

    run_codex_cli_update(binary.to_str().expect("utf-8 path")).expect("native update succeeds");

    assert_eq!(fs::read_to_string(args_log).expect("read args"), "update\n");
    fs::remove_dir_all(temp_dir).expect("clean temp dir");
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "pad-codex-cli-{label}-{}-{nanos}",
        std::process::id()
    ))
}
