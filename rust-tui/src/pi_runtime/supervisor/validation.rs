//! Validation and shell-word helpers for the synchronous Pi supervisor.

use super::PiSupervisorError;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub(super) fn is_env_program(program: &str) -> bool {
    Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "env")
}

pub(super) fn is_pi_program(program: &str) -> bool {
    Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("pi"))
}

pub(super) fn is_environment_assignment(word: &str) -> bool {
    let Some((key, _)) = word.split_once('=') else {
        return false;
    };
    !key.is_empty()
        && key.bytes().enumerate().all(|(index, byte)| {
            (index == 0 && (byte == b'_' || byte.is_ascii_alphabetic()))
                || (index > 0 && (byte == b'_' || byte.is_ascii_alphanumeric()))
        })
}

pub(super) fn has_rpc_mode(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--mode=rpc")
        || args.windows(2).any(|pair| pair == ["--mode", "rpc"])
}

pub(super) fn validate_runtime_root(path: &Path, label: &str) -> Result<(), PiSupervisorError> {
    if !path.is_absolute() {
        return Err(PiSupervisorError::InvalidCommand(format!(
            "Pi {label} root must be absolute: {}",
            path.display()
        )));
    }
    let provider_namespace = path.components().any(|component| {
        let Component::Normal(value) = component else {
            return false;
        };
        matches!(
            value.to_string_lossy().to_ascii_lowercase().as_str(),
            ".codex"
                | "codex"
                | ".pi"
                | ".chatgpt"
                | "chatgpt"
                | "com.openai.codex"
                | "com.openai.chatgpt"
                | "com.openai.chat"
                | "openai"
        )
    });
    if provider_namespace {
        return Err(PiSupervisorError::InvalidCommand(format!(
            "Pi {label} root is inside a provider-owned namespace: {}",
            path.display()
        )));
    }
    Ok(())
}

pub(super) fn validate_pi_session_args(
    args: &[String],
    session_root: &Path,
    cwd: &Path,
) -> Result<(), PiSupervisorError> {
    let session_root = fs::canonicalize(session_root)
        .map(|path| lexical_normalize(&path))
        .unwrap_or_else(|_| lexical_normalize(session_root));
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        let (kind, candidate) = if let Some(value) = argument.strip_prefix("--session=") {
            ("session", Some(value))
        } else if let Some(value) = argument.strip_prefix("--session-dir=") {
            ("session-dir", Some(value))
        } else if argument == "--session" || argument == "--session-dir" {
            (
                argument.trim_start_matches('-'),
                args.get(index + 1).map(String::as_str),
            )
        } else {
            index += 1;
            continue;
        };
        let candidate = candidate
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                PiSupervisorError::InvalidCommand(format!("Pi {kind} argument is missing"))
            })?;
        let candidate = Path::new(candidate);
        let candidate = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            cwd.join(candidate)
        };
        let within_root = canonicalize_existing_prefix(&candidate)
            .map(|resolved| {
                let resolved = lexical_normalize(&resolved);
                resolved == session_root || resolved.starts_with(&session_root)
            })
            .unwrap_or_else(|| {
                let normalized = lexical_normalize(&candidate);
                normalized == session_root || normalized.starts_with(&session_root)
            });
        if !within_root {
            return Err(PiSupervisorError::InvalidCommand(format!(
                "Pi {kind} path is outside the Profile session root: {}",
                candidate.display()
            )));
        }
        index += if argument == "--session" || argument == "--session-dir" {
            2
        } else {
            1
        };
    }
    Ok(())
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = normalized.pop();
            }
            Component::Normal(value) => normalized.push(value),
        }
    }
    normalized
}

fn canonicalize_existing_prefix(path: &Path) -> Option<PathBuf> {
    let mut existing = path.to_path_buf();
    while !existing.exists() {
        if !existing.pop() {
            return None;
        }
    }
    let canonical_existing = fs::canonicalize(&existing).ok()?;
    let remainder = path.strip_prefix(&existing).ok()?;
    Some(canonical_existing.join(remainder))
}

pub(super) fn shell_words(command: &str) -> Result<Vec<String>, PiSupervisorError> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut chars = command.chars().peekable();
    let mut quote = None;
    let mut started = false;
    while let Some(ch) = chars.next() {
        match quote {
            Some('\'') => {
                if ch == '\'' {
                    quote = None;
                } else {
                    word.push(ch);
                }
            }
            Some('"') => {
                if ch == '"' {
                    quote = None;
                } else if ch == '\\' {
                    let escaped = chars.next().ok_or_else(|| {
                        PiSupervisorError::InvalidCommand("trailing escape".to_string())
                    })?;
                    word.push(escaped);
                } else {
                    word.push(ch);
                }
            }
            None if ch.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            None if ch == '\'' || ch == '"' => {
                quote = Some(ch);
                started = true;
            }
            None if ch == '\\' => {
                word.push(chars.next().ok_or_else(|| {
                    PiSupervisorError::InvalidCommand("trailing escape".to_string())
                })?);
                started = true;
            }
            None => {
                word.push(ch);
                started = true;
            }
            Some(_) => unreachable!("shell parser only creates single or double quotes"),
        }
    }
    if quote.is_some() {
        return Err(PiSupervisorError::InvalidCommand(
            "unterminated quote".to_string(),
        ));
    }
    if started {
        words.push(word);
    }
    Ok(words)
}
