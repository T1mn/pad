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
        (Some(version), Some(content_md5)) if !version.is_empty() && !content_md5.is_empty() => {
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
