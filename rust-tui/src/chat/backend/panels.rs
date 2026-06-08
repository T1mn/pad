use crate::model::AgentPanel;
use crate::scanner;
use crate::session_cache;
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

    let mut panels = scanner::scan_panels().map_err(|err| io::Error::other(err.to_string()))?;
    let _ = session_cache::preload_panels(&mut panels);
    if let Ok(mut cache) = PANEL_CACHE.lock() {
        *cache = Some(CachedPanels {
            loaded_at: Instant::now(),
            panels: panels.clone(),
        });
    }
    Ok(panels)
}

pub(crate) fn invalidate_live_panels() {
    if let Ok(mut cache) = PANEL_CACHE.lock() {
        *cache = None;
    }
}
