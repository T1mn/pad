use std::path::{Path, PathBuf};

/// 最多保留这么多份损坏快照，避免反复启动时把目录刷满。
const MAX_BACKUPS: usize = 20;

/// 把解析失败的 config.toml 复制到 `config.toml.bak`（已占用则 `.bak.1`、`.bak.2` ...）。
///
/// 用复制而不是移动：原文件留在原地方便用户直接修，备份保证即便下一次 `save()`
/// 用默认值覆盖了原文件，provider / api_key / bot_token 仍然找得回来。
/// 内容与已有备份完全相同时复用该备份，不产生新文件。
pub(super) fn preserve_broken_config(path: &Path) -> Option<PathBuf> {
    let content = std::fs::read(path).ok()?;
    for candidate in backup_candidates(path) {
        match std::fs::read(&candidate) {
            Ok(existing) if existing == content => return Some(candidate),
            Ok(_) => continue,
            Err(_) => {
                return crate::atomic_file::write_private(&candidate, &content)
                    .ok()
                    .map(|()| candidate);
            }
        }
    }
    None
}

fn backup_candidates(path: &Path) -> Vec<PathBuf> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "config.toml".to_string());
    (0..MAX_BACKUPS)
        .map(|idx| {
            let suffix = if idx == 0 {
                "bak".to_string()
            } else {
                format!("bak.{idx}")
            };
            path.with_file_name(format!("{name}.{suffix}"))
        })
        .collect()
}

#[cfg(test)]
#[path = "backup_tests.rs"]
mod tests;
