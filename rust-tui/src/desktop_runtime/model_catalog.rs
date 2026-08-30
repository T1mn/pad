//! Pi-backed model catalog for the Desktop control plane.
//!
//! Pi owns provider composition, authentication, dynamic model stores, and
//! availability checks.  The Rust host intentionally calls that public SDK
//! surface instead of parsing `models-store.json` itself.  Only normalized
//! model metadata crosses the bridge; credentials, SDK errors, and private
//! paths stay inside this module.

use crate::permission_policy::Profile;
#[cfg(test)]
use serde_json::json;
use serde_json::Value;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

const MAX_MODEL_CATALOG_OUTPUT_BYTES: usize = 8 * 1024 * 1024;

/// The helper runs inside the bundled Pi package and uses the same
/// `ModelRuntime` API as Pi's own `--list-models` command.  `getModels()` is
/// the complete composed catalog; `getAvailable()` is the authenticated and
/// currently usable subset displayed by Desktop.
const MODEL_CATALOG_SCRIPT: &str = r#"
import path from "node:path";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";

const agentDir = process.env.PAD_MODEL_CATALOG_AGENT_DIR;
const refresh = process.env.PAD_MODEL_CATALOG_REFRESH === "1";

const finiteNumber = (value) => Number.isFinite(value) ? value : null;
const stringValue = (value, fallback = "") => typeof value === "string" ? value : fallback;
const inputValues = (value) => Array.isArray(value)
  ? value.filter((item) => typeof item === "string").slice(0, 8)
  : [];
const reasoningLevels = (value) => value && typeof value === "object" && !Array.isArray(value)
  ? Object.keys(value).filter((item) => typeof item === "string").slice(0, 8)
  : [];

const modelValue = (model) => ({
  provider: stringValue(model.provider),
  id: stringValue(model.id),
  name: stringValue(model.name, stringValue(model.id)),
  api: stringValue(model.api),
  reasoning: model.reasoning === true,
  reasoning_levels: reasoningLevels(model.thinkingLevelMap),
  input: inputValues(model.input),
  context_window: finiteNumber(model.contextWindow),
  max_tokens: finiteNumber(model.maxTokens),
});

const uniqueModels = (models) => {
  const seen = new Set();
  return models.map(modelValue).filter((model) => {
    if (!model.provider || !model.id) return false;
    const key = `${model.provider}\0${model.id}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
};

try {
  const runtime = await ModelRuntime.create({
    authPath: path.join(agentDir, "auth.json"),
    modelsPath: path.join(agentDir, "models.json"),
    modelsStorePath: path.join(agentDir, "models-store.json"),
    refreshOnCreate: false,
    allowModelNetwork: false,
  });
  // A caller may explicitly request a cache-only refresh after login.  This
  // restores the latest persisted provider catalog without making a network
  // request on the Desktop UI path.
  if (refresh) await runtime.refresh({ allowNetwork: false });
  const allModels = uniqueModels(runtime.getModels());
  const availableModels = uniqueModels(await runtime.getAvailable());
  const availableByProvider = new Map();
  for (const model of availableModels) {
    const models = availableByProvider.get(model.provider) ?? [];
    models.push(model);
    availableByProvider.set(model.provider, models);
  }
  const providers = [...availableByProvider.entries()].sort(([left], [right]) => left.localeCompare(right))
    .map(([id, models]) => ({
      id,
      name: stringValue(runtime.getProvider(id)?.name, id),
      authenticated: true,
      models,
    }));
  process.stdout.write(JSON.stringify({
    status: "ready",
    source: "pi_model_runtime",
    models: availableModels,
    available_models: availableModels,
    all_models: allModels,
    providers,
    counts: { all: allModels.length, available: availableModels.length },
    checked_at: Date.now(),
  }));
} catch {
  // Do not pass SDK errors through: they can contain auth paths, package
  // paths, URLs, or provider-specific credential details.
  process.stdout.write(JSON.stringify({
    status: "unavailable",
    source: "pi_model_runtime",
    models: [],
    available_models: [],
    all_models: [],
    providers: [],
    counts: { all: 0, available: 0 },
    error: "Pi model catalog is unavailable",
  }));
  process.exitCode = 1;
}
"#;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ModelCatalogError {
    Store,
    ProfileNotFound,
    RuntimeMissing,
    RuntimeFailed,
    InvalidResponse,
    ResponseTooLarge,
}

impl ModelCatalogError {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Store => "model_catalog_store_error",
            Self::ProfileNotFound => "profile_not_found",
            Self::RuntimeMissing => "model_catalog_runtime_missing",
            Self::RuntimeFailed => "model_catalog_unavailable",
            Self::InvalidResponse => "model_catalog_invalid_response",
            Self::ResponseTooLarge => "model_catalog_response_too_large",
        }
    }

    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::Store => "model catalog storage is unavailable",
            Self::ProfileNotFound => "profile is unavailable for the active profile",
            Self::RuntimeMissing => "trusted Pi model runtime was not found",
            Self::RuntimeFailed => "Pi model catalog is unavailable",
            Self::InvalidResponse => "Pi model catalog returned an invalid response",
            Self::ResponseTooLarge => "Pi model catalog response exceeds the protocol limit",
        }
    }
}

impl fmt::Display for ModelCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl std::error::Error for ModelCatalogError {}

impl super::DesktopRuntime {
    /// Read the current Profile's Pi model runtime and return only safe,
    /// renderer-facing metadata.  The default path is cache-only and fast;
    /// `refresh` restores Pi's persisted model store after authentication.
    pub(crate) fn model_catalog(
        &self,
        profile_id: &str,
        refresh: bool,
    ) -> Result<Value, ModelCatalogError> {
        let profile = self
            .store()
            .get_profile(profile_id)
            .map_err(|_| ModelCatalogError::Store)?
            .ok_or(ModelCatalogError::ProfileNotFound)?;
        let (agent_dir, _) = crate::pi_runtime::profile_pi_roots(&profile);
        let (program, package_root) = self.model_catalog_launcher()?;
        let mut value = query_model_catalog(&program, &package_root, &agent_dir, refresh)?;
        enrich_catalog_selection(&mut value, &profile);
        if let Some(object) = value.as_object_mut() {
            object.insert("profile_id".to_string(), Value::String(profile.id.clone()));
            object.insert(
                "authenticated_providers".to_string(),
                Value::Array(
                    self.authenticated_providers(&profile)
                        .into_iter()
                        .map(Value::String)
                        .collect(),
                ),
            );
        }
        Ok(value)
    }

    fn model_catalog_launcher(&self) -> Result<(PathBuf, PathBuf), ModelCatalogError> {
        #[cfg(test)]
        if let Some(launcher) = self.model_catalog_launcher.clone() {
            return Ok(launcher);
        }

        let resource_root = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf));
        let program = [
            resource_root.as_ref().map(|root| root.join("bin/node")),
            Some(PathBuf::from("/opt/homebrew/bin/node")),
            Some(PathBuf::from("/usr/local/bin/node")),
            Some(PathBuf::from("/usr/bin/node")),
        ]
        .into_iter()
        .flatten()
        .find(|path| is_executable_file(path))
        .ok_or(ModelCatalogError::RuntimeMissing)?;
        let package_root = [
            resource_root.as_ref().map(|root| root.join("pi")),
            Some(PathBuf::from(
                "/opt/homebrew/lib/node_modules/@earendil-works/pi-coding-agent",
            )),
            Some(PathBuf::from(
                "/usr/local/lib/node_modules/@earendil-works/pi-coding-agent",
            )),
        ]
        .into_iter()
        .flatten()
        .find(|path| path.join("package.json").is_file())
        .ok_or(ModelCatalogError::RuntimeMissing)?;
        Ok((program, package_root))
    }

    #[cfg(test)]
    pub(crate) fn set_model_catalog_launcher_for_test(
        &mut self,
        program: PathBuf,
        package_root: PathBuf,
    ) {
        self.model_catalog_launcher = Some((program, package_root));
    }
}

fn query_model_catalog(
    program: &Path,
    package_root: &Path,
    agent_dir: &Path,
    refresh: bool,
) -> Result<Value, ModelCatalogError> {
    let mut command = Command::new(program);
    command
        .args(["--input-type=module", "-e", MODEL_CATALOG_SCRIPT])
        .current_dir(package_root)
        .env_clear()
        .env("PAD_MODEL_CATALOG_AGENT_DIR", agent_dir)
        .env("PAD_MODEL_CATALOG_REFRESH", if refresh { "1" } else { "0" })
        .env("PI_CODING_AGENT_DIR", agent_dir)
        // Bun lazily transpiles Pi provider modules. Keep both of its runtime
        // caches in the Profile-private directory so reading a catalog never
        // mutates the signed app bundle under Contents/Resources.
        .env("BUN_INSTALL_CACHE_DIR", agent_dir.join("bun-cache"))
        .env(
            "BUN_RUNTIME_TRANSPILER_CACHE_PATH",
            agent_dir.join("bun-transpiler-cache"),
        )
        .env("PATH", trusted_child_path(program));
    let output = command
        .output()
        .map_err(|_| ModelCatalogError::RuntimeMissing)?;
    if output.stdout.len() > MAX_MODEL_CATALOG_OUTPUT_BYTES {
        return Err(ModelCatalogError::ResponseTooLarge);
    }
    let value = serde_json::from_slice::<Value>(&output.stdout)
        .map_err(|_| ModelCatalogError::InvalidResponse)?;
    if !output.status.success() {
        // The helper emits a sanitized unavailable object for SDK failures.
        // Treat any non-zero process as unavailable even if a future helper
        // accidentally emits a success-shaped object.
        return Err(ModelCatalogError::RuntimeFailed);
    }
    if !value.is_object() {
        return Err(ModelCatalogError::InvalidResponse);
    }
    Ok(value)
}

fn enrich_catalog_selection(value: &mut Value, profile: &Profile) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    let available = object
        .get("available_models")
        .and_then(Value::as_array)
        .or_else(|| object.get("models").and_then(Value::as_array))
        .cloned()
        .unwrap_or_default();
    let first_provider = available
        .iter()
        .filter_map(|model| model.get("provider").and_then(Value::as_str))
        .find(|provider| !provider.is_empty());
    let selected_provider = profile
        .default_provider
        .as_deref()
        .filter(|provider| !provider.trim().is_empty())
        .or(first_provider);
    let selected_model = profile
        .default_model
        .as_deref()
        .filter(|model| !model.trim().is_empty())
        .or_else(|| {
            available.iter().find_map(|model| {
                let provider_matches = selected_provider.is_none_or(|provider| {
                    model.get("provider").and_then(Value::as_str) == Some(provider)
                });
                provider_matches
                    .then(|| model.get("id").and_then(Value::as_str))
                    .flatten()
            })
        });
    object.insert(
        "selected_provider".to_string(),
        selected_provider.map_or(Value::Null, |value| Value::String(value.to_string())),
    );
    object.insert(
        "selected_model".to_string(),
        selected_model.map_or(Value::Null, |value| Value::String(value.to_string())),
    );
}

fn trusted_child_path(program: &Path) -> String {
    let mut entries = Vec::new();
    if let Some(parent) = program.parent() {
        entries.push(parent.to_string_lossy().into_owned());
    }
    entries.extend(["/usr/bin".to_string(), "/bin".to_string()]);
    entries.join(":")
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn enriches_selected_model_without_exposing_private_fields() {
        let mut value = json!({
            "status": "ready",
            "models": [
                {"provider": "openai-codex", "id": "gpt-5.4", "name": "GPT-5.4"}
            ],
            "all_models": [
                {"provider": "openai-codex", "id": "gpt-5.4", "name": "GPT-5.4"}
            ],
            "secret": "must-not-be-created"
        });
        let profile = Profile {
            default_provider: Some("openai-codex".to_string()),
            default_model: Some("gpt-5.4".to_string()),
            ..Profile::default()
        };
        enrich_catalog_selection(&mut value, &profile);
        assert_eq!(value["selected_provider"], "openai-codex");
        assert_eq!(value["selected_model"], "gpt-5.4");
        assert!(value.get("auth_path").is_none());
    }

    pub(crate) fn falls_back_to_first_available_model() {
        let mut value = json!({
            "models": [
                {"provider": "openai-codex", "id": "gpt-5.4", "name": "GPT-5.4"}
            ]
        });
        enrich_catalog_selection(&mut value, &Profile::default());
        assert_eq!(value["selected_provider"], "openai-codex");
        assert_eq!(value["selected_model"], "gpt-5.4");
    }

    pub(crate) fn model_catalog_error_messages_are_path_free() {
        for error in [
            ModelCatalogError::Store,
            ModelCatalogError::RuntimeMissing,
            ModelCatalogError::RuntimeFailed,
            ModelCatalogError::InvalidResponse,
            ModelCatalogError::ResponseTooLarge,
        ] {
            assert!(!error.message().contains('/'));
            assert!(!error.message().contains("auth"));
        }
    }
}
