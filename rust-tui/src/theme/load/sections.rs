use crate::theme::{Config, SoundEventConfig};
use std::collections::HashMap;

pub(super) fn apply_root_fields(table: &HashMap<String, toml::Value>, config: &mut Config) {
    if let Some(toml::Value::String(theme)) = table.get("theme") {
        config.theme = theme.clone();
    }
    if let Some(toml::Value::Boolean(auto)) = table.get("auto_refresh") {
        config.auto_refresh = *auto;
    }
    if let Some(toml::Value::Integer(interval)) = table.get("refresh_interval") {
        config.refresh_interval = *interval as u64;
    }
    if let Some(toml::Value::String(lang)) = table.get("language") {
        config.language = lang.clone();
    }
    if let Some(toml::Value::String(sb)) = table.get("status_bar") {
        config.desired_agent_style.status = match sb.as_str() {
            "hidden" => "hide".to_string(),
            "show" => "show".to_string(),
            other => other.to_string(),
        };
    }
}

pub(super) fn apply_section_fields(table: &HashMap<String, toml::Value>, config: &mut Config) {
    apply_desired_style(table.get("desired_agent_style"), config);
    apply_preview(table.get("preview"), config);
    apply_display(table.get("display"), config);
    apply_sound(table.get("sound"), config);
    apply_telegram(table.get("telegram"), config);
    super::codex::apply_codex(table.get("codex"), config);
    apply_permissions(table.get("agent_permissions"), config);
}

fn apply_desired_style(value: Option<&toml::Value>, config: &mut Config) {
    let Some(toml::Value::Table(das)) = value else {
        return;
    };
    if let Some(toml::Value::String(z)) = das.get("zoom") {
        config.desired_agent_style.zoom = z.clone();
    }
    if let Some(toml::Value::String(s)) = das.get("status") {
        config.desired_agent_style.status = s.clone();
    }
}

fn apply_preview(value: Option<&toml::Value>, config: &mut Config) {
    let Some(toml::Value::Table(preview)) = value else {
        return;
    };
    if let Some(toml::Value::String(mode)) = preview.get("mode") {
        config.preview.mode = match mode.as_str() {
            "tmux" => "tmux".to_string(),
            "session" => "session".to_string(),
            _ => "auto".to_string(),
        };
    }
}

fn apply_display(value: Option<&toml::Value>, config: &mut Config) {
    let Some(toml::Value::Table(display)) = value else {
        return;
    };
    if let Some(toml::Value::String(scope)) = display.get("session_scope") {
        config.display.session_scope = match scope.as_str() {
            "all" => "all".to_string(),
            _ => "live".to_string(),
        };
    }
}

fn apply_sound(value: Option<&toml::Value>, config: &mut Config) {
    let Some(toml::Value::Table(sound)) = value else {
        return;
    };
    if let Some(toml::Value::Boolean(enabled)) = sound.get("enabled") {
        config.sound.enabled = *enabled;
    }
    load_sound_event_config(
        sound.get("completion"),
        &mut config.sound.completion,
        crate::sound::SoundEvent::Completion,
    );
    load_sound_event_config(
        sound.get("approval"),
        &mut config.sound.approval,
        crate::sound::SoundEvent::Approval,
    );
    load_sound_event_config(
        sound.get("timeout"),
        &mut config.sound.timeout,
        crate::sound::SoundEvent::Timeout,
    );
    load_sound_event_config(
        sound.get("failure"),
        &mut config.sound.failure,
        crate::sound::SoundEvent::Failure,
    );
}

fn apply_telegram(value: Option<&toml::Value>, config: &mut Config) {
    let Some(toml::Value::Table(telegram)) = value else {
        return;
    };
    if let Some(toml::Value::Boolean(enabled)) = telegram.get("enabled") {
        config.telegram.enabled = *enabled;
    }
    if let Some(toml::Value::String(token)) = telegram.get("bot_token") {
        config.telegram.bot_token = token.clone();
    }
    if let Some(toml::Value::String(chat_id)) = telegram.get("chat_id") {
        config.telegram.chat_id = chat_id.clone();
    }
    if let Some(toml::Value::String(bot_username)) = telegram.get("bot_username") {
        config.telegram.bot_username = bot_username.clone();
    }
}

fn apply_permissions(value: Option<&toml::Value>, config: &mut Config) {
    let Some(toml::Value::Table(agent_permissions)) = value else {
        return;
    };
    if let Some(toml::Value::Boolean(enabled)) = agent_permissions.get("codex_auto_full_access") {
        config.agent_permissions.codex_auto_full_access = *enabled;
    }
    if let Some(toml::Value::Boolean(enabled)) = agent_permissions.get("claude_auto_full_access") {
        config.agent_permissions.claude_auto_full_access = *enabled;
    }
}

fn load_sound_event_config(
    value: Option<&toml::Value>,
    target: &mut SoundEventConfig,
    event: crate::sound::SoundEvent,
) {
    let Some(toml::Value::Table(table)) = value else {
        return;
    };
    if let Some(toml::Value::Boolean(enabled)) = table.get("enabled") {
        target.enabled = *enabled;
    }
    if let Some(toml::Value::String(preset)) = table.get("preset") {
        target.preset = SoundEventConfig::normalize_preset_for(event, preset);
    }
}
