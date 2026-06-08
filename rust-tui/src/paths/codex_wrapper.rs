use std::fs;
use std::io;

use super::pad_codex_wrapper_path;

const WRAPPER_VERSION: &str = "pad-codex-wrapper-2026-06-02.1";

pub(super) fn install_pad_codex_wrapper() -> io::Result<()> {
    let path = pad_codex_wrapper_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let desired = pad_codex_wrapper_template();
    if fs::read_to_string(&path).ok().as_deref() != Some(desired.as_str()) {
        fs::write(&path, &desired)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms)?;
    }

    let actual = fs::read_to_string(&path)?;
    if actual != desired {
        return Err(io::Error::other(format!(
            "pad-codex wrapper verify failed at {}",
            path.display()
        )));
    }

    Ok(())
}

fn pad_codex_wrapper_template() -> String {
    format!(
        r#"#!/bin/sh
# pad-wrapper-version: {WRAPPER_VERSION}
# PAD-managed Codex entrypoint. It keeps the pad profile and relay auth together.

set -eu

AUTH_PATH="${{PAD_CODEX_AUTH_PATH:-$HOME/.pad/codex-home/auth.json}}"
PAD_CODEX_HOME="${{PAD_CODEX_HOME:-$HOME/.pad/codex-home}}"
CONFIG_PATH="${{PAD_CODEX_CONFIG_PATH:-$PAD_CODEX_HOME/pad.config.toml}}"
CODEX_BIN="${{PAD_CODEX_BIN:-codex}}"

mkdir -p "$PAD_CODEX_HOME"

if [ -f "$AUTH_PATH" ]; then
  OPENAI_API_KEY="$(AUTH_PATH="$AUTH_PATH" python3 - <<'PY'
import json
import os
from pathlib import Path

path = Path(os.environ["AUTH_PATH"])
try:
    value = json.loads(path.read_text()).get("OPENAI_API_KEY") or ""
except Exception:
    value = ""
print(str(value).strip())
PY
)"
  export OPENAI_API_KEY
fi

if [ -z "${{OPENAI_API_KEY:-}}" ]; then
  REQUIRES_OPENAI_AUTH="$(CONFIG_PATH="$CONFIG_PATH" python3 - <<'PY'
from pathlib import Path
import os

path = Path(os.environ["CONFIG_PATH"])
try:
    import tomllib
    doc = tomllib.loads(path.read_text())
    provider = doc.get("model_provider")
    providers = doc.get("model_providers") or {{}}
    requires = bool((providers.get(provider) or {{}}).get("requires_openai_auth"))
except Exception:
    requires = False
print("1" if requires else "0")
PY
)"
  if [ "$REQUIRES_OPENAI_AUTH" = "1" ]; then
    echo "pad-codex: missing OPENAI_API_KEY. Configure the active Codex relay provider in PAD first." >&2
    echo "pad-codex: expected relay auth at $AUTH_PATH" >&2
    exit 126
  fi
fi

export PAD_CODEX_HOOKS=1
export CODEX_HOME="$PAD_CODEX_HOME"
exec "$CODEX_BIN" --profile pad "$@"
"#
    )
}

#[cfg(test)]
#[path = "codex_wrapper_tests.rs"]
mod tests;
