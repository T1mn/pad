use super::collect_stats_output;
use std::ffi::OsString;

#[cfg(unix)]
#[test]
fn stats_uses_selected_project_as_cwd_and_empty_current_project_filter() {
    use std::os::unix::fs::PermissionsExt;

    let root = crate::test_support::temp_path("pad-opencode-stats", "mock");
    let project = root.join("selected project");
    std::fs::create_dir_all(&project).expect("create selected project");
    let command = root.join("opencode-mock");
    std::fs::write(
        &command,
        r#"#!/bin/sh
root=$(dirname "$0")
pwd > "$root/cwd"
printf '%s\n' "$@" > "$root/args"
printf 'stats ok\n'
"#,
    )
    .expect("write mock opencode");
    std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o755))
        .expect("make mock executable");

    let output = collect_stats_output(
        project.to_str().expect("utf-8 project path"),
        &OsString::from(command.as_os_str()),
    )
    .expect("collect stats");

    assert_eq!(output, "stats ok\n");
    let observed_cwd = std::fs::read_to_string(root.join("cwd")).expect("read cwd");
    assert_eq!(
        std::fs::canonicalize(observed_cwd.trim()).expect("canonical observed cwd"),
        std::fs::canonicalize(&project).expect("canonical project cwd")
    );
    assert_eq!(
        std::fs::read_to_string(root.join("args")).expect("read args"),
        "stats\n--project\n\n--models\n10\n--tools\n10\n"
    );

    let _ = std::fs::remove_dir_all(root);
}
