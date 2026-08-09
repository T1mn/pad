use std::fs;
use std::io;
use std::path::{Path, PathBuf};

mod state {
    use std::fs;
    use std::io;
    use std::path::Path;

    const LEGACY_CODEX_JAILBREAK_PROMPT_HASHES: &[&str] = &["c8bf76a53a9b840d52c987ebff0310b2"];

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(in crate::paths) struct ManagedPromptState {
        pub(in crate::paths) version: String,
        pub(in crate::paths) content_md5: String,
    }

    pub(in crate::paths) fn prompt_md5(content: &str) -> String {
        format!("{:x}", md5::compute(content))
    }

    pub(in crate::paths) fn should_refresh_managed_prompt(
        existing_prompt: &str,
        existing_state: Option<&ManagedPromptState>,
        current_state: &ManagedPromptState,
    ) -> bool {
        let existing_md5 = prompt_md5(existing_prompt);
        match existing_state {
            Some(state) => {
                existing_md5 == state.content_md5
                    && (state.version != current_state.version
                        || state.content_md5 != current_state.content_md5)
            }
            None => {
                existing_md5 == current_state.content_md5
                    || LEGACY_CODEX_JAILBREAK_PROMPT_HASHES.contains(&existing_md5.as_str())
            }
        }
    }

    pub(in crate::paths) fn read_managed_prompt_state(
        path: &Path,
    ) -> io::Result<Option<ManagedPromptState>> {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err),
        };

        let mut version = None;
        let mut content_md5 = None;
        for line in content.lines() {
            if let Some(value) = line.strip_prefix("version=") {
                version = Some(value.trim().to_string());
            } else if let Some(value) = line.strip_prefix("content_md5=") {
                content_md5 = Some(value.trim().to_string());
            }
        }

        match (version, content_md5) {
            (Some(version), Some(content_md5))
                if !version.is_empty() && !content_md5.is_empty() =>
            {
                Ok(Some(ManagedPromptState {
                    version,
                    content_md5,
                }))
            }
            _ => Ok(None),
        }
    }

    pub(in crate::paths) fn write_managed_prompt_state(
        path: &Path,
        state: &ManagedPromptState,
    ) -> io::Result<()> {
        fs::write(
            path,
            format!(
                "version={}\ncontent_md5={}\n",
                state.version, state.content_md5
            ),
        )
    }
}

use state::should_refresh_managed_prompt;
pub(super) use state::{
    prompt_md5, read_managed_prompt_state, write_managed_prompt_state, ManagedPromptState,
};

pub(super) const CODEX_JAILBREAK_PROMPT_VERSION: &str = "codex-jailbreak-prompt-2026-04-26.1";
const CODEX_INDEX_PROMPT_VERSION: &str = "codex-index-prompt-2026-05-29.1";
pub const DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE: &str =
    include_str!("../../assets/prompts/codex_jailbreak.md");
pub const DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE: &str =
    include_str!("../../assets/prompts/codex_index.md");

pub fn codex_jailbreak_prompt_file_path() -> PathBuf {
    super::prompts_dir().join("codex_jailbreak.md")
}

pub fn codex_index_prompt_file_path() -> PathBuf {
    super::prompts_dir().join("codex_index.md")
}

pub fn codex_selected_prompt_file_path() -> PathBuf {
    super::prompts_dir().join("codex_selected.md")
}

pub(in crate::paths) fn legacy_codex_prompt_file_path() -> PathBuf {
    super::prompts_dir().join("codex.md")
}

pub(super) fn codex_jailbreak_prompt_state_path() -> PathBuf {
    super::prompts_dir().join("codex_jailbreak.version")
}

fn codex_index_prompt_state_path() -> PathBuf {
    super::prompts_dir().join("codex_index.version")
}

pub fn ensure_codex_jailbreak_prompt_file_seeded() -> io::Result<()> {
    fs::create_dir_all(super::prompts_dir())?;
    let prompt_path = codex_jailbreak_prompt_file_path();
    seed_managed_prompt_file(
        &prompt_path,
        &codex_jailbreak_prompt_state_path(),
        CODEX_JAILBREAK_PROMPT_VERSION,
        DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE,
        Some(&legacy_codex_prompt_file_path()),
    )
}

pub fn ensure_codex_index_prompt_file_seeded() -> io::Result<()> {
    fs::create_dir_all(super::prompts_dir())?;
    seed_managed_prompt_file(
        &codex_index_prompt_file_path(),
        &codex_index_prompt_state_path(),
        CODEX_INDEX_PROMPT_VERSION,
        DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE,
        None,
    )
}

fn seed_managed_prompt_file(
    prompt_path: &Path,
    state_path: &Path,
    version: &str,
    default_template: &str,
    legacy_prompt_path: Option<&Path>,
) -> io::Result<()> {
    let existing_prompt = read_prompt_with_legacy_fallback(prompt_path, legacy_prompt_path)?;
    let current_state = ManagedPromptState {
        version: version.to_string(),
        content_md5: prompt_md5(default_template),
    };
    let existing_state = read_managed_prompt_state(state_path)?;

    let needs_seed = match existing_prompt.as_deref() {
        None => true,
        Some(existing) if existing.trim().is_empty() => true,
        Some(existing) => {
            should_refresh_managed_prompt(existing, existing_state.as_ref(), &current_state)
        }
    };

    if needs_seed {
        fs::write(prompt_path, default_template)?;
        write_managed_prompt_state(state_path, &current_state)?;
    } else if !prompt_path.exists() {
        if let Some(existing) = existing_prompt {
            fs::write(prompt_path, existing)?;
        }
    }

    Ok(())
}

fn read_prompt_with_legacy_fallback(
    prompt_path: &Path,
    legacy_prompt_path: Option<&Path>,
) -> io::Result<Option<String>> {
    match fs::read_to_string(prompt_path) {
        Ok(existing) => Ok(Some(existing)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => match legacy_prompt_path {
            Some(legacy_path) => match fs::read_to_string(legacy_path) {
                Ok(existing) => Ok(Some(existing)),
                Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
                Err(err) => Err(err),
            },
            None => Ok(None),
        },
        Err(err) => Err(err),
    }
}

pub fn write_codex_selected_prompt_file(
    include_jailbreak: bool,
    include_index: bool,
) -> io::Result<Option<PathBuf>> {
    let mut prompt_paths = Vec::new();
    if include_jailbreak {
        ensure_codex_jailbreak_prompt_file_seeded()?;
        prompt_paths.push(codex_jailbreak_prompt_file_path());
    }
    if include_index {
        ensure_codex_index_prompt_file_seeded()?;
        prompt_paths.push(codex_index_prompt_file_path());
    }

    match prompt_paths.as_slice() {
        [] => Ok(None),
        [single] => Ok(Some(single.clone())),
        paths => {
            let mut content = String::from(
                "# Generated by pad from selected Codex prompt candidates. Do not edit directly.\n\n",
            );
            for path in paths {
                content.push_str(&format!("<!-- source: {} -->\n\n", path.display()));
                content.push_str(&fs::read_to_string(path)?);
                if !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push('\n');
            }
            let combined_path = codex_selected_prompt_file_path();
            fs::write(&combined_path, content)?;
            Ok(Some(combined_path))
        }
    }
}
