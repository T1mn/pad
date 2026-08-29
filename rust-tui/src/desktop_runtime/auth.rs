//! Rust-owned Pi authentication coordinator for Desktop protocol v2.
//!
//! The renderer only exchanges typed JSON values with this coordinator.  It
//! never receives the profile auth path and never selects an executable or
//! package directory.  The helper script intentionally mirrors the original
//! native Swift JSONL login protocol while process ownership now lives in the
//! Rust control plane.

use crate::permission_policy::Profile;
use serde::Serialize;
use serde_json::{json, Value};
use std::fmt;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

const MAX_AUTH_LINE_BYTES: usize = 256 * 1024;
const MAX_AUTH_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_AUTH_ERROR_CHARS: usize = 512;
const MAX_AUTH_NOTICES: usize = 32;

const LOGIN_HELPER_SCRIPT: &str = r#"
import { createInterface } from "node:readline";
import { randomUUID } from "node:crypto";
import path from "node:path";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";

const send = (value) => process.stdout.write(JSON.stringify(value) + "\n");
const pending = new Map();
const input = createInterface({ input: process.stdin });
input.on("line", (line) => {
  try {
    const value = JSON.parse(line);
    if (value.type === "response" && pending.has(value.id)) {
      pending.get(value.id)(value);
      pending.delete(value.id);
    }
  } catch {
    // Rust validates every renderer response before it reaches this helper.
  }
});
const waitForResponse = (id) => new Promise((resolve) => pending.set(id, resolve));
const interaction = {
  prompt: async (value) => {
    const id = randomUUID();
    send({ type: "prompt", id, kind: value.type, message: value.message,
      placeholder: value.placeholder, options: value.options ?? [] });
    const response = await waitForResponse(id);
    if (response.cancelled) throw new Error("Authentication cancelled");
    return String(response.value ?? "");
  },
  notify: (event) => send({ type: "event", event }),
};

try {
  const agentDir = process.env.PAD_AUTH_AGENT_DIR;
  const runtime = await ModelRuntime.create({
    authPath: path.join(agentDir, "auth.json"),
    modelsPath: path.join(agentDir, "models.json"),
    refreshOnCreate: false,
  });
  const provider = process.env.PAD_AUTH_PROVIDER;
  if (process.env.PAD_AUTH_OPERATION === "logout") {
    await runtime.logout(provider);
  } else {
    await runtime.login(provider, process.env.PAD_AUTH_TYPE, interaction);
  }
  send({ type: "success", provider });
} catch (error) {
  send({ type: "error", message: error instanceof Error ? error.message : String(error) });
  process.exitCode = 1;
} finally {
  input.close();
}
"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuthPhase {
    Idle,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct AuthOptionDto {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct AuthPromptDto {
    pub id: String,
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    pub options: Vec<AuthOptionDto>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct AuthNoticeDto {
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_code: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct AuthSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_type: Option<String>,
    pub operation: &'static str,
    pub phase: AuthPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<AuthPromptDto>,
    pub notices: Vec<AuthNoticeDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub updated_at: u64,
}

impl Default for AuthSnapshot {
    fn default() -> Self {
        Self {
            attempt_id: None,
            profile_id: None,
            provider: None,
            auth_type: None,
            operation: "login",
            phase: AuthPhase::Idle,
            prompt: None,
            notices: Vec::new(),
            error: None,
            updated_at: super::unix_timestamp(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct AuthError {
    pub code: &'static str,
    pub message: String,
}

impl AuthError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AuthError {}

enum HelperMessage {
    Value(Value),
    Invalid(String),
    Closed,
}

struct AuthAttempt {
    child: Child,
    input: Option<ChildStdin>,
    messages: Receiver<HelperMessage>,
    stderr: Receiver<String>,
    private_roots: Vec<PathBuf>,
}

pub(crate) struct PiAuthCoordinator {
    snapshot: AuthSnapshot,
    attempt: Option<AuthAttempt>,
    program: Option<PathBuf>,
    package_root: Option<PathBuf>,
}

impl PiAuthCoordinator {
    pub(crate) fn new() -> Self {
        Self {
            snapshot: AuthSnapshot::default(),
            attempt: None,
            program: None,
            package_root: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn set_launcher_for_test(&mut self, program: PathBuf, package_root: PathBuf) {
        self.program = Some(program);
        self.package_root = Some(package_root);
    }

    pub(crate) fn begin(
        &mut self,
        profile: &Profile,
        provider: &str,
        auth_type: &str,
    ) -> Result<AuthSnapshot, AuthError> {
        self.start(profile, provider, auth_type, "login")
    }

    pub(crate) fn logout(
        &mut self,
        profile: &Profile,
        provider: &str,
    ) -> Result<AuthSnapshot, AuthError> {
        self.start(profile, provider, "logout", "logout")
    }

    fn start(
        &mut self,
        profile: &Profile,
        provider: &str,
        auth_type: &str,
        operation: &'static str,
    ) -> Result<AuthSnapshot, AuthError> {
        self.poll();
        if self.snapshot.phase == AuthPhase::Running {
            return Err(AuthError::new(
                "auth_in_progress",
                "another Pi authentication operation is already running",
            ));
        }
        validate_identifier(provider, "provider")?;
        if operation == "login" && !matches!(auth_type, "oauth" | "api_key") {
            return Err(AuthError::new(
                "invalid_auth_type",
                "auth_type must be oauth or api_key",
            ));
        }
        let (program, package_root) = self.resolve_launcher()?;
        if !profile.agent_dir.is_dir() {
            std::fs::create_dir_all(&profile.agent_dir).map_err(|error| {
                AuthError::new(
                    "auth_storage_error",
                    format!("cannot prepare profile auth storage: {error}"),
                )
            })?;
        }

        self.stop_child();
        let mut command = Command::new(&program);
        command
            .args(["--input-type=module", "-e", LOGIN_HELPER_SCRIPT])
            .current_dir(&package_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear()
            .env("PAD_AUTH_OPERATION", operation)
            .env("PAD_AUTH_PROVIDER", provider)
            .env("PAD_AUTH_TYPE", auth_type)
            .env("PAD_AUTH_AGENT_DIR", &profile.agent_dir)
            .env("PI_CODING_AGENT_DIR", &profile.agent_dir)
            .env("PATH", trusted_child_path(&program));
        copy_safe_network_environment(&mut command);
        let mut child = command.spawn().map_err(|error| {
            AuthError::new(
                "auth_spawn_failed",
                format!("could not start Pi authentication: {error}"),
            )
        })?;
        let input = child.stdin.take();
        let stdout = child.stdout.take().ok_or_else(|| {
            AuthError::new("auth_spawn_failed", "authentication stdout is unavailable")
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            AuthError::new("auth_spawn_failed", "authentication stderr is unavailable")
        })?;
        let (message_tx, message_rx) = mpsc::channel();
        thread::spawn(move || read_helper_output(stdout, message_tx));
        let (stderr_tx, stderr_rx) = mpsc::channel();
        thread::spawn(move || {
            let mut value = String::new();
            let _ = BufReader::new(stderr)
                .take(MAX_AUTH_LINE_BYTES as u64)
                .read_to_string(&mut value);
            let _ = stderr_tx.send(value);
        });

        let attempt_id = format!("auth-{}", crate::time::unix_now_nanos());
        self.snapshot = AuthSnapshot {
            attempt_id: Some(attempt_id),
            profile_id: Some(profile.id.clone()),
            provider: Some(provider.to_string()),
            auth_type: (operation == "login").then(|| auth_type.to_string()),
            operation,
            phase: AuthPhase::Running,
            prompt: None,
            notices: Vec::new(),
            error: None,
            updated_at: super::unix_timestamp(),
        };
        self.attempt = Some(AuthAttempt {
            child,
            input,
            messages: message_rx,
            stderr: stderr_rx,
            private_roots: vec![
                profile.agent_dir.clone(),
                profile.session_dir.clone(),
                package_root,
            ],
        });
        Ok(self.snapshot.clone())
    }

    pub(crate) fn status(&mut self) -> (AuthSnapshot, bool) {
        let changed = self.poll();
        (self.snapshot.clone(), changed)
    }

    pub(crate) fn owner_profile_id(&self) -> Option<&str> {
        self.snapshot.profile_id.as_deref()
    }

    pub(crate) fn running_profile_id(&self) -> Option<&str> {
        (self.snapshot.phase == AuthPhase::Running)
            .then_some(self.snapshot.profile_id.as_deref())
            .flatten()
    }

    pub(crate) fn respond(
        &mut self,
        attempt_id: &str,
        prompt_id: &str,
        value: Value,
        cancelled: bool,
    ) -> Result<AuthSnapshot, AuthError> {
        self.poll();
        self.require_attempt(attempt_id)?;
        let prompt = self.snapshot.prompt.as_ref().ok_or_else(|| {
            AuthError::new(
                "auth_prompt_missing",
                "authentication has no pending prompt",
            )
        })?;
        if prompt.id != prompt_id {
            return Err(AuthError::new(
                "auth_prompt_mismatch",
                "prompt_id does not match the active authentication prompt",
            ));
        }
        let payload = json!({
            "type": "response",
            "id": prompt_id,
            "value": value,
            "cancelled": cancelled,
        });
        let encoded = serde_json::to_vec(&payload)
            .map_err(|error| AuthError::new("invalid_auth_response", error.to_string()))?;
        if encoded.len() > MAX_AUTH_RESPONSE_BYTES {
            return Err(AuthError::new(
                "auth_response_too_large",
                "authentication response exceeds the protocol limit",
            ));
        }
        let input = self
            .attempt
            .as_mut()
            .and_then(|attempt| attempt.input.as_mut())
            .ok_or_else(|| AuthError::new("auth_disconnected", "authentication input is closed"))?;
        input
            .write_all(&encoded)
            .and_then(|_| input.write_all(b"\n"))
            .and_then(|_| input.flush())
            .map_err(|error| AuthError::new("auth_disconnected", error.to_string()))?;
        self.snapshot.prompt = None;
        self.snapshot.updated_at = super::unix_timestamp();
        Ok(self.snapshot.clone())
    }

    pub(crate) fn cancel(&mut self, attempt_id: &str) -> Result<AuthSnapshot, AuthError> {
        self.poll();
        self.require_attempt(attempt_id)?;
        self.stop_child();
        self.snapshot.phase = AuthPhase::Cancelled;
        self.snapshot.prompt = None;
        self.snapshot.error = None;
        self.snapshot.updated_at = super::unix_timestamp();
        Ok(self.snapshot.clone())
    }

    fn require_attempt(&self, attempt_id: &str) -> Result<(), AuthError> {
        if self.snapshot.attempt_id.as_deref() != Some(attempt_id) {
            return Err(AuthError::new(
                "auth_attempt_mismatch",
                "attempt_id does not match the active authentication operation",
            ));
        }
        if self.snapshot.phase != AuthPhase::Running {
            return Err(AuthError::new(
                "auth_not_running",
                "authentication operation is not running",
            ));
        }
        Ok(())
    }

    fn poll(&mut self) -> bool {
        let mut changed = false;
        let Some(attempt) = self.attempt.as_mut() else {
            return false;
        };
        loop {
            match attempt.messages.try_recv() {
                Ok(HelperMessage::Value(value)) => {
                    changed |=
                        apply_helper_value(&mut self.snapshot, value, &attempt.private_roots);
                }
                Ok(HelperMessage::Invalid(message)) => {
                    self.snapshot.phase = AuthPhase::Failed;
                    self.snapshot.prompt = None;
                    self.snapshot.error = Some(sanitize_message(&message, &attempt.private_roots));
                    changed = true;
                }
                Ok(HelperMessage::Closed) | Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        match attempt.child.try_wait() {
            Ok(Some(status)) => {
                if self.snapshot.phase == AuthPhase::Running {
                    if status.success() {
                        self.snapshot.phase = AuthPhase::Succeeded;
                    } else {
                        let stderr = attempt.stderr.try_recv().unwrap_or_default();
                        self.snapshot.phase = AuthPhase::Failed;
                        self.snapshot.error = Some(if stderr.trim().is_empty() {
                            "Pi authentication failed".to_string()
                        } else {
                            sanitize_message(&stderr, &attempt.private_roots)
                        });
                    }
                    self.snapshot.prompt = None;
                    changed = true;
                }
                attempt.input = None;
            }
            Ok(None) => {}
            Err(error) => {
                self.snapshot.phase = AuthPhase::Failed;
                self.snapshot.prompt = None;
                self.snapshot.error =
                    Some(sanitize_message(&error.to_string(), &attempt.private_roots));
                changed = true;
            }
        }
        if changed {
            self.snapshot.updated_at = super::unix_timestamp();
        }
        changed
    }

    fn resolve_launcher(&self) -> Result<(PathBuf, PathBuf), AuthError> {
        if let (Some(program), Some(package_root)) = (&self.program, &self.package_root) {
            return Ok((program.clone(), package_root.clone()));
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
        .ok_or_else(|| {
            AuthError::new(
                "auth_runtime_missing",
                "trusted Pi login runtime was not found",
            )
        })?;
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
        .ok_or_else(|| {
            AuthError::new("auth_sdk_missing", "trusted Pi SDK package was not found")
        })?;
        Ok((program, package_root))
    }

    fn stop_child(&mut self) {
        if let Some(mut attempt) = self.attempt.take() {
            attempt.input.take();
            let _ = attempt.child.kill();
            let _ = attempt.child.wait();
        }
    }
}

impl Drop for PiAuthCoordinator {
    fn drop(&mut self) {
        self.stop_child();
    }
}

fn read_helper_output(stdout: impl std::io::Read, sender: mpsc::Sender<HelperMessage>) {
    let mut reader = BufReader::new(stdout);
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) => break,
            Ok(_) if line.len() > MAX_AUTH_LINE_BYTES => {
                let _ = sender.send(HelperMessage::Invalid(
                    "Pi authentication emitted an oversized frame".to_string(),
                ));
                break;
            }
            Ok(_) => {
                while matches!(line.last(), Some(b'\n' | b'\r')) {
                    line.pop();
                }
                match serde_json::from_slice::<Value>(&line) {
                    Ok(value) => {
                        if sender.send(HelperMessage::Value(value)).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        let _ = sender.send(HelperMessage::Invalid(
                            "Pi authentication emitted invalid JSON".to_string(),
                        ));
                        break;
                    }
                }
            }
            Err(error) => {
                let _ = sender.send(HelperMessage::Invalid(error.to_string()));
                break;
            }
        }
    }
    let _ = sender.send(HelperMessage::Closed);
}

fn apply_helper_value(snapshot: &mut AuthSnapshot, value: Value, roots: &[PathBuf]) -> bool {
    match value.get("type").and_then(Value::as_str) {
        Some("prompt") => {
            let Some(id) = value.get("id").and_then(Value::as_str) else {
                return false;
            };
            snapshot.prompt = Some(AuthPromptDto {
                id: limited(id, 128),
                kind: limited(
                    value.get("kind").and_then(Value::as_str).unwrap_or("text"),
                    32,
                ),
                message: sanitize_message(
                    value
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("Authentication input required"),
                    roots,
                ),
                placeholder: value
                    .get("placeholder")
                    .and_then(Value::as_str)
                    .map(|value| sanitize_message(value, roots)),
                options: auth_options(value.get("options"), roots),
            });
            true
        }
        Some("event") => {
            let event = value.get("event").unwrap_or(&Value::Null);
            let kind = limited(
                event.get("type").and_then(Value::as_str).unwrap_or("info"),
                64,
            );
            let raw_url = event
                .get("url")
                .and_then(Value::as_str)
                .or_else(|| event.get("verificationUri").and_then(Value::as_str));
            let url = raw_url
                .filter(|url| url.starts_with("https://") || url.starts_with("http://localhost:"))
                .map(|url| limited(url, 4096));
            let raw_message = event
                .get("message")
                .and_then(Value::as_str)
                .or_else(|| event.get("instructions").and_then(Value::as_str))
                .or(raw_url)
                .unwrap_or("");
            snapshot.notices.push(AuthNoticeDto {
                kind,
                message: sanitize_message(raw_message, roots),
                url,
                user_code: event
                    .get("userCode")
                    .and_then(Value::as_str)
                    .map(|value| limited(value, 128)),
            });
            if snapshot.notices.len() > MAX_AUTH_NOTICES {
                snapshot.notices.remove(0);
            }
            true
        }
        Some("success") => {
            snapshot.phase = AuthPhase::Succeeded;
            snapshot.prompt = None;
            snapshot.error = None;
            true
        }
        Some("error") => {
            snapshot.phase = AuthPhase::Failed;
            snapshot.prompt = None;
            snapshot.error = Some(sanitize_message(
                value
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Pi authentication failed"),
                roots,
            ));
            true
        }
        _ => false,
    }
}

fn auth_options(value: Option<&Value>, roots: &[PathBuf]) -> Vec<AuthOptionDto> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(64)
        .filter_map(|item| {
            if let Some(label) = item.as_str() {
                let label = sanitize_message(label, roots);
                return Some(AuthOptionDto {
                    id: label.clone(),
                    label,
                    description: None,
                });
            }
            let label = item.get("label")?.as_str()?;
            Some(AuthOptionDto {
                id: limited(item.get("id").and_then(Value::as_str).unwrap_or(label), 256),
                label: sanitize_message(label, roots),
                description: item
                    .get("description")
                    .and_then(Value::as_str)
                    .map(|value| sanitize_message(value, roots)),
            })
        })
        .collect()
}

fn validate_identifier(value: &str, field: &'static str) -> Result<(), AuthError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(AuthError::new(
            "invalid_auth_request",
            format!("{field} is invalid"),
        ));
    }
    Ok(())
}

fn sanitize_message(value: &str, roots: &[PathBuf]) -> String {
    let mut safe = value.to_string();
    for root in roots {
        let root = root.to_string_lossy();
        if !root.is_empty() {
            safe = safe.replace(root.as_ref(), "[private]");
        }
    }
    if let Some(home) = dirs::home_dir() {
        let home = home.to_string_lossy();
        safe = safe.replace(home.as_ref(), "~");
    }
    limited(safe.trim(), MAX_AUTH_ERROR_CHARS)
}

fn limited(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn copy_safe_network_environment(command: &mut Command) {
    for key in [
        "LANG",
        "LC_ALL",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "NO_PROXY",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
}

fn trusted_child_path(program: &Path) -> String {
    let parent = program.parent().unwrap_or_else(|| Path::new("/usr/bin"));
    format!("{}:/usr/bin:/bin:/usr/sbin:/sbin", parent.display())
}

fn is_executable_file(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}
