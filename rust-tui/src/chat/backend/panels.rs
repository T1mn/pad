use crate::model::AgentPanel;
use crate::model::{AgentState, AgentType};
use crate::socket_api::model::ApiRequest;
use std::io;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

const PANEL_CACHE_TTL: Duration = Duration::from_millis(800);

struct CachedPanels {
    loaded_at: Instant,
    panels: Vec<AgentPanel>,
}

static PANEL_CACHE: LazyLock<Mutex<Option<CachedPanels>>> = LazyLock::new(|| Mutex::new(None));

pub(crate) fn live_panels() -> Result<Vec<AgentPanel>, Box<dyn std::error::Error>> {
    if let Ok(cache) = PANEL_CACHE.lock() {
        if let Some(cache) = cache.as_ref() {
            if cache.loaded_at.elapsed() <= PANEL_CACHE_TTL {
                return Ok(cache.panels.clone());
            }
        }
    }

    let response = crate::socket_api::client::send_request(&ApiRequest {
        action: "status".to_string(),
        ..ApiRequest::default()
    })?;
    if !response.ok {
        return Err(io::Error::other(response.message).into());
    }
    let panels: Vec<AgentPanel> = response
        .data
        .as_ref()
        .and_then(|data| data.get("panels"))
        .and_then(serde_json::Value::as_array)
        .map(|panels| panels.iter().filter_map(panel_from_json).collect())
        .unwrap_or_default();
    if let Ok(mut cache) = PANEL_CACHE.lock() {
        *cache = Some(CachedPanels {
            loaded_at: Instant::now(),
            panels: panels.clone(),
        });
    }
    Ok(panels)
}

fn panel_from_json(value: &serde_json::Value) -> Option<AgentPanel> {
    let text = |key: &str| value.get(key)?.as_str().map(str::to_string);
    let agent_name = text("agent_type")?;
    let state = match text("state")?.as_str() {
        "busy" => AgentState::Busy,
        "waiting" => AgentState::Waiting,
        _ => AgentState::Idle,
    };
    Some(AgentPanel {
        session: text("session").unwrap_or_else(|| "native".to_string()),
        window: text("window").unwrap_or_else(|| agent_name.clone()),
        window_index: text("window_index").unwrap_or_default(),
        pane: text("pane").unwrap_or_default(),
        pane_id: text("pane_id")?,
        agent_type: AgentType::from_processes(&agent_name),
        working_dir: text("working_dir")?,
        is_active: value
            .get("is_active")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        state,
        transcript_path: text("transcript_path"),
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        agent_session_id: text("agent_session_id"),
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
    })
}

pub(crate) fn invalidate_live_panels() {
    if let Ok(mut cache) = PANEL_CACHE.lock() {
        *cache = None;
    }
}
