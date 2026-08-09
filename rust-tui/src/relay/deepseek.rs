mod launcher {
    use std::ffi::OsString;
    use std::fs::{self, File, OpenOptions};
    use std::io::{self, Write};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    pub(super) fn write(path: &Path, content: &[u8]) -> io::Result<()> {
        let (temp_path, mut temp_file) = create_temp_file(path)?;
        let write_result = write_private_content(&mut temp_file, content);
        drop(temp_file);

        if let Err(err) = write_result {
            let _ = fs::remove_file(&temp_path);
            return Err(err);
        }
        if let Err(err) = fs::rename(&temp_path, path) {
            let _ = fs::remove_file(&temp_path);
            return Err(err);
        }
        Ok(())
    }

    fn create_temp_file(path: &Path) -> io::Result<(PathBuf, File)> {
        loop {
            let temp_path = temp_path(path)?;
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o700);
            }

            match options.open(&temp_path) {
                Ok(file) => return Ok((temp_path, file)),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(err) => {
                    let _ = fs::remove_file(&temp_path);
                    return Err(err);
                }
            }
        }
    }

    fn temp_path(path: &Path) -> io::Result<PathBuf> {
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "launcher has no parent"))?;
        let file_name = path.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "launcher has no file name")
        })?;
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let mut temp_name = OsString::from(".");
        temp_name.push(file_name);
        temp_name.push(format!(".pad-tmp-{}-{id}", std::process::id()));
        Ok(parent.join(temp_name))
    }

    fn write_private_content(file: &mut File, content: &[u8]) -> io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o700))?;
        }
        file.write_all(content)?;
        file.flush()
    }
}

use super::common::{
    log_file_error, parse_json_object, serialize_json_pretty, should_restore_standard_relay_config,
    write_text_file,
};
use crate::theme::AgentConfig;
use serde_json::json;
use std::path::PathBuf;

fn deepseek_settings_path() -> PathBuf {
    crate::paths::pad_home_dir()
        .join("deepseek-config")
        .join("settings.json")
}

pub(super) fn apply_deepseek_agent_config(agent: &AgentConfig) {
    let path = deepseek_settings_path();

    if should_restore_standard_relay_config(agent) {
        let _ = std::fs::remove_file(&path);
        remove_deepseek_launcher_script();
        return;
    }

    let Some(prov) = agent.active() else {
        let _ = std::fs::remove_file(&path);
        remove_deepseek_launcher_script();
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let content = std::fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_string());

    let updated = update_deepseek_settings_config(
        &content,
        &prov.base_url,
        &prov.api_key,
        &agent.default_model,
        prov.disable_thinking,
    );
    if let Err(error) = write_text_file(&path, &updated) {
        log_file_error("write", &path, &error);
        return;
    }

    generate_deepseek_launcher_script(&prov.base_url, &prov.api_key, &agent.default_model);
}

fn update_deepseek_settings_config(
    content: &str,
    base_url: &str,
    api_key: &str,
    default_model: &str,
    disable_thinking: bool,
) -> String {
    let mut obj = parse_json_object(content);
    obj.as_object_mut()
        .expect("deepseek settings root")
        .remove("apiUrl");
    obj.as_object_mut()
        .expect("deepseek settings root")
        .remove("apiKey");

    let env = obj
        .as_object_mut()
        .expect("deepseek settings root")
        .entry("env".to_string())
        .or_insert_with(|| json!({}));
    if !env.is_object() {
        *env = json!({});
    }

    let env_obj = env.as_object_mut().expect("deepseek env object");
    env_obj.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        serde_json::Value::String(normalize_base_url(base_url)),
    );
    env_obj.insert(
        "ANTHROPIC_AUTH_TOKEN".to_string(),
        serde_json::Value::String(api_key.to_string()),
    );
    env_obj.remove("ANTHROPIC_API_KEY");
    env_obj.insert(
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
        serde_json::Value::String("1".to_string()),
    );
    env_obj.insert(
        "CLAUDE_CODE_ATTRIBUTION_HEADER".to_string(),
        serde_json::Value::String("0".to_string()),
    );
    env_obj.remove("CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS");
    env_obj.remove("MAX_THINKING_TOKENS");
    if disable_thinking {
        env_obj.insert(
            "CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS".to_string(),
            serde_json::Value::String("1".to_string()),
        );
        env_obj.insert(
            "MAX_THINKING_TOKENS".to_string(),
            serde_json::Value::String("0".to_string()),
        );
    }
    env_obj.remove("ANTHROPIC_MODEL");
    env_obj.remove("ANTHROPIC_CUSTOM_MODEL_OPTION");
    if !default_model.trim().is_empty() {
        env_obj.insert(
            "ANTHROPIC_MODEL".to_string(),
            serde_json::Value::String(default_model.trim().to_string()),
        );
        env_obj.insert(
            "ANTHROPIC_CUSTOM_MODEL_OPTION".to_string(),
            serde_json::Value::String(default_model.trim().to_string()),
        );
    }

    serialize_json_pretty(&obj)
}

fn normalize_base_url(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed == "https://api.deepseek.com" {
        return "https://api.deepseek.com/anthropic".to_string();
    }
    trimmed.strip_suffix("/v1").unwrap_or(trimmed).to_string()
}

fn deepseek_launcher_script_path() -> PathBuf {
    crate::paths::pad_home_dir().join("deepseek-cc")
}

fn generate_deepseek_launcher_script(base_url: &str, api_key: &str, model: &str) {
    let script_path = deepseek_launcher_script_path();
    let normalized_url = normalize_base_url(base_url);
    let model_name = if model.trim().is_empty() {
        "deepseek-v4-flash"
    } else {
        model.trim()
    };

    let deepseek_config_dir = crate::paths::pad_home_dir().join("deepseek-config");
    let script = format!(
        r#"#!/bin/bash
# DeepSeek(cc) launcher - Auto-generated by pad
export CLAUDE_CONFIG_DIR="{}"
export ANTHROPIC_BASE_URL="{}"
export ANTHROPIC_AUTH_TOKEN="{}"
export ANTHROPIC_MODEL="{}"
exec claude "$@"
"#,
        deepseek_config_dir.display(),
        normalized_url,
        api_key,
        model_name
    );

    if let Err(e) = launcher::write(&script_path, script.as_bytes()) {
        eprintln!("Failed to write deepseek launcher: {}", e);
    }
}

fn remove_deepseek_launcher_script() {
    let script_path = deepseek_launcher_script_path();
    let _ = std::fs::remove_file(script_path);
}
