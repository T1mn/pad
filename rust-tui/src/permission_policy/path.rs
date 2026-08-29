use super::{EffectivePolicy, OperationKind, PolicyOperation, ProtectedNamespace, RiskClass};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Lexically canonicalize a path without touching the filesystem.
///
/// This resolves `.` and `..`, removes duplicate separators through
/// `Path::components`, and makes relative paths relative to `base_dir`.  The
/// host should call [`canonicalize_existing_prefix`] when it needs symlink
/// resolution for an existing prefix.
pub(crate) fn canonicalize_policy_path(path: &Path, base_dir: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    };
    let mut result = PathBuf::new();

    for component in joined.components() {
        match component {
            Component::Prefix(prefix) => result.push(prefix.as_os_str()),
            Component::RootDir => result.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                let popped = result.pop();
                if !popped && !result.has_root() {
                    result.push(Component::ParentDir.as_os_str());
                }
            }
            Component::Normal(part) => result.push(part),
        }
    }

    if result.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        result
    }
}

/// Resolve every existing path component through the filesystem while keeping
/// a not-yet-created suffix intact.  This closes the common `workspace/link`
/// escape where `link` is a symlink into a protected provider namespace.
///
/// The function never creates or opens the requested target.  Callers retain
/// the lexical path as a conservative fallback when no prefix can be resolved.
pub(crate) fn canonicalize_existing_prefix(path: &Path) -> Option<PathBuf> {
    let mut existing = path.to_path_buf();
    while fs::symlink_metadata(&existing).is_err() {
        if !existing.pop() {
            return None;
        }
    }
    let canonical = fs::canonicalize(&existing).ok()?;
    let remainder = path.strip_prefix(&existing).ok()?;
    Some(canonicalize_policy_path(
        &canonical.join(remainder),
        Path::new("/"),
    ))
}

pub(super) fn resolve_policy_path(path: &Path, base_dir: &Path) -> PathBuf {
    let lexical = canonicalize_policy_path(path, base_dir);
    canonicalize_existing_prefix(&lexical).unwrap_or(lexical)
}

/// Return the protected namespace containing `path`, if any.
pub(crate) fn matching_protected_namespace<'a>(
    path: &Path,
    base_dir: &Path,
    namespaces: &'a [ProtectedNamespace],
) -> Option<&'a ProtectedNamespace> {
    let canonical_path = resolve_policy_path(path, base_dir);
    namespaces.iter().find(|namespace| {
        let canonical_root = resolve_policy_path(&namespace.root, base_dir);
        canonical_path == canonical_root || canonical_path.starts_with(&canonical_root)
    })
}

/// Classify an operation by action and canonical workspace scope.
pub(super) fn classify_risk(
    policy: &EffectivePolicy,
    operation: &PolicyOperation,
    cwd: &Path,
) -> RiskClass {
    let in_workspace = operation.path.as_deref().is_some_and(|path| {
        let target = resolve_policy_path(path, cwd);
        policy.workspace_roots.iter().any(|root| {
            let canonical_root = resolve_policy_path(root, cwd);
            target == canonical_root || target.starts_with(&canonical_root)
        })
    });

    match operation.kind {
        OperationKind::Read => {
            if operation.path.is_none() || in_workspace {
                RiskClass::ReadOnly
            } else {
                RiskClass::ExternalRead
            }
        }
        OperationKind::Write => {
            if in_workspace {
                RiskClass::WorkspaceWrite
            } else {
                RiskClass::ExternalWrite
            }
        }
        OperationKind::Execute => {
            if in_workspace {
                RiskClass::WorkspaceExecute
            } else {
                RiskClass::ExternalExecute
            }
        }
        OperationKind::Delete => {
            if in_workspace {
                RiskClass::WorkspaceDestructive
            } else {
                RiskClass::ExternalDestructive
            }
        }
        OperationKind::Network => RiskClass::Network,
        OperationKind::Credential => RiskClass::Credential,
        OperationKind::Install => RiskClass::Install,
        OperationKind::ProcessControl => RiskClass::ProcessControl,
    }
}
