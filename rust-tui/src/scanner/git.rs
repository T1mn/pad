use crate::model::GitInfo;
use std::collections::HashSet;
use std::process::Command;

const MAX_PARALLEL_GIT_STATUS: usize = 8;

pub(super) fn get_git_info(working_dir: &str) -> Option<GitInfo> {
    let output = Command::new("git")
        .args(["-C", working_dir, "status", "--porcelain=v2", "--branch"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    parse_git_status_porcelain_v2(&String::from_utf8_lossy(&output.stdout))
}

pub(super) fn get_git_info_for_paths(
    working_dirs: &[String],
) -> std::collections::HashMap<String, Option<GitInfo>> {
    let paths = unique_working_dirs(working_dirs.iter().map(String::as_str));
    let mut out = std::collections::HashMap::with_capacity(paths.len());
    if paths.len() <= 1 {
        for path in paths {
            out.insert(path.clone(), get_git_info(&path));
        }
        return out;
    }

    for chunk in paths.chunks(MAX_PARALLEL_GIT_STATUS) {
        std::thread::scope(|scope| {
            let handles = chunk
                .iter()
                .cloned()
                .map(|path| {
                    scope.spawn(move || {
                        let info = get_git_info(&path);
                        (path, info)
                    })
                })
                .collect::<Vec<_>>();
            for handle in handles {
                if let Ok((path, info)) = handle.join() {
                    out.insert(path, info);
                }
            }
        });
    }
    out
}

fn unique_working_dirs<'a>(paths: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for path in paths {
        let path = path.trim();
        if !path.is_empty() && seen.insert(path.to_string()) {
            out.push(path.to_string());
        }
    }
    out
}

pub(super) fn parse_git_status_porcelain_v2(stdout: &str) -> Option<GitInfo> {
    let mut branch = None;
    let mut commit = None;
    let mut changed_files = 0usize;

    for line in stdout.lines() {
        if let Some(value) = line.strip_prefix("# branch.head ") {
            if value != "(unknown)" {
                branch = Some(value.to_string());
            }
            continue;
        }

        if let Some(value) = line.strip_prefix("# branch.oid ") {
            if value != "(initial)" {
                commit = Some(value.to_string());
            }
            continue;
        }

        if !line.trim().is_empty() && !line.starts_with('#') {
            changed_files += 1;
        }
    }

    Some(GitInfo {
        branch,
        commit,
        changed_files,
    })
}

#[cfg(test)]
mod tests {
    use super::unique_working_dirs;

    #[test]
    fn unique_working_dirs_dedupes_and_skips_empty_paths() {
        let paths = unique_working_dirs(["/repo/a", "", " /repo/b ", "/repo/a"]);

        assert_eq!(paths, vec!["/repo/a".to_string(), "/repo/b".to_string()]);
    }
}
