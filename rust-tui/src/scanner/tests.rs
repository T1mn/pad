use super::{
    capture_pane_content, detect_agent_type, parse_git_status_porcelain_v2, scan_panels, ScanCaches,
};
use crate::model::{AgentState, AgentType, GitInfo};
use std::process::Command;
use std::time::Instant;

#[test]
fn detect_agent_type_prefers_tmux_current_command() {
    let mut caches = ScanCaches::default();
    let (agent_type, main_process, child_processes) =
        detect_agent_type("codex", "123", &mut caches);

    assert_eq!(agent_type, AgentType::Codex);
    assert_eq!(main_process, "codex");
    assert!(child_processes.is_empty());
    assert!(!caches.processes.is_loaded());
}

#[test]
fn cached_git_info_reuses_existing_result() {
    let mut caches = ScanCaches::default();
    caches.git_info.insert(
        "/tmp/project".to_string(),
        Some(GitInfo {
            branch: Some("main".to_string()),
            commit: Some("abc".to_string()),
            changed_files: 3,
        }),
    );

    let first = caches.cached_git_info("/tmp/project");
    let second = caches.cached_git_info("/tmp/project");

    assert_eq!(first, second);
    assert_eq!(caches.git_info.len(), 1);
}

#[test]
fn parses_git_status_porcelain_v2_branch_commit_and_changes() {
    let info = parse_git_status_porcelain_v2(
            "# branch.oid abcdef1234567890\n# branch.head main\n1 .M N... 100644 100644 100644 old old src/lib.rs\n? new.txt\n",
        )
        .unwrap();

    assert_eq!(info.branch.as_deref(), Some("main"));
    assert_eq!(info.commit.as_deref(), Some("abcdef1234567890"));
    assert_eq!(info.changed_files, 2);
}

#[test]
#[ignore]
fn bench_scan_panels_from_env() {
    let iterations = std::env::var("PAD_SCAN_BENCH_ITERATIONS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(5);
    let mut runs_ms = Vec::with_capacity(iterations);
    let mut agent_count = 0usize;
    let mut last_breakdown = None;

    for _ in 0..iterations {
        let started_at = Instant::now();
        let panels = scan_panels().expect("scan_panels should run against local tmux");
        runs_ms.push(started_at.elapsed().as_secs_f64() * 1000.0);
        agent_count = panels.len();
        last_breakdown = Some(measure_scan_breakdown());
    }

    let total_ms: f64 = runs_ms.iter().sum();
    let avg_ms = total_ms / runs_ms.len() as f64;
    let min_ms = runs_ms.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_ms = runs_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let breakdown = last_breakdown.unwrap_or_default();
    println!(
            "bench.scan_panels iterations={} agents={} runs_ms={:?} avg_ms={:.3} min_ms={:.3} max_ms={:.3} panes={} list_ms={:.3} detect_ms={:.3} capture_ms={:.3} git_ms={:.3}",
            iterations, agent_count, runs_ms, avg_ms, min_ms, max_ms, breakdown.panes, breakdown.list_ms, breakdown.detect_ms, breakdown.capture_ms, breakdown.git_ms
        );
}

#[derive(Default, Debug)]
struct ScanBreakdown {
    panes: usize,
    agents: usize,
    list_ms: f64,
    detect_ms: f64,
    capture_ms: f64,
    git_ms: f64,
}

fn measure_scan_breakdown() -> ScanBreakdown {
    let list_started_at = Instant::now();
    let output = Command::new("tmux")
            .args([
                "list-panes",
                "-a",
                "-F",
                "#{session_name}|#{window_name}|#{window_index}|#{pane_index}|#{pane_id}|#{pane_pid}|#{pane_current_command}|#{pane_current_path}",
            ])
            .output()
            .expect("tmux list-panes should run");
    let list_ms = list_started_at.elapsed().as_secs_f64() * 1000.0;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pane_pids = stdout
        .lines()
        .filter_map(|line| line.split('|').nth(5))
        .filter(|pid| !pid.trim().is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut caches = ScanCaches::with_pane_pids(pane_pids);
    let mut out = ScanBreakdown {
        panes: stdout.lines().count(),
        list_ms,
        ..ScanBreakdown::default()
    };

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 8 {
            continue;
        }
        let pane_id = parts[4];
        let pane_pid = parts[5];
        let current_cmd = parts[6];
        let working_dir = parts[7];

        let started_at = Instant::now();
        let (agent_type, _, _) = detect_agent_type(current_cmd, pane_pid, &mut caches);
        out.detect_ms += started_at.elapsed().as_secs_f64() * 1000.0;
        if matches!(agent_type, AgentType::Unknown) {
            continue;
        }

        out.agents += 1;
        let started_at = Instant::now();
        let _state = if let Ok(content) = capture_pane_content(pane_id, 20) {
            crate::detector::detect_state(&content)
        } else {
            AgentState::Idle
        };
        out.capture_ms += started_at.elapsed().as_secs_f64() * 1000.0;

        let started_at = Instant::now();
        let _git = caches.cached_git_info(working_dir);
        out.git_ms += started_at.elapsed().as_secs_f64() * 1000.0;
    }

    out
}
