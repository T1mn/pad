use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitTreeSnapshot {
    pub repo_root: PathBuf,
    pub tree: String,
}

pub fn repo_root_for_cwd(cwd: &Path) -> io::Result<PathBuf> {
    let output = git_output(cwd, &["rev-parse", "--show-toplevel"])?;
    let root = PathBuf::from(output.trim());
    fs::canonicalize(&root).or(Ok(root))
}

pub fn capture_worktree_tree(cwd: &Path) -> io::Result<GitTreeSnapshot> {
    let repo_root = repo_root_for_cwd(cwd)?;
    let index_path = temp_index_path();

    let result = (|| {
        if head_exists(&repo_root)? {
            git_output_with_index(&repo_root, &index_path, &["read-tree", "HEAD"])?;
        }
        git_output_with_index(&repo_root, &index_path, &["add", "-A", "--", "."])?;
        let tree = git_output_with_index(&repo_root, &index_path, &["write-tree"])?;
        Ok(GitTreeSnapshot {
            repo_root,
            tree: tree.trim().to_string(),
        })
    })();

    let _ = fs::remove_file(&index_path);
    let _ = fs::remove_file(index_path.with_extension("lock"));
    result
}

pub fn diff_trees(repo_root: &Path, base_tree: &str, end_tree: &str) -> io::Result<String> {
    git_output(
        repo_root,
        &[
            "diff",
            "--no-ext-diff",
            "--find-renames",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            base_tree,
            end_tree,
        ],
    )
}

pub fn diff_pending_to_worktree(repo_root: &Path, base_tree: &str) -> io::Result<String> {
    let end = capture_worktree_tree(repo_root)?;
    diff_trees(&end.repo_root, base_tree, &end.tree)
}

fn head_exists(repo_root: &Path) -> io::Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "--verify", "HEAD^{tree}"])
        .output()?;
    Ok(output.status.success())
}

fn git_output(cwd: &Path, args: &[&str]) -> io::Result<String> {
    let output = Command::new("git").arg("-C").arg(cwd).args(args).output()?;
    command_output(output, args)
}

fn git_output_with_index(cwd: &Path, index_path: &Path, args: &[&str]) -> io::Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .env("GIT_INDEX_FILE", index_path)
        .args(args)
        .output()?;
    command_output(output, args)
}

fn command_output(output: std::process::Output, args: &[&str]) -> io::Result<String> {
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    Err(io::Error::other(format!(
        "git {} failed: {}",
        format_git_args(args),
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

fn temp_index_path() -> PathBuf {
    let stamp = crate::time::unix_now_nanos();
    std::env::temp_dir().join(format!(
        "pad-codex-turn-diff-{}-{stamp}.index",
        std::process::id()
    ))
}

fn format_git_args(args: &[&str]) -> String {
    let mut formatted = String::new();
    for arg in args {
        if !formatted.is_empty() {
            formatted.push(' ');
        }
        formatted.push_str(arg);
    }
    formatted
}

#[cfg(test)]
#[path = "git_tests.rs"]
mod tests;
