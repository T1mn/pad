use pyo3::prelude::*;
use pyo3::types::PyDict;

mod tmux;
mod detector;

use detector::AIPanel;

/// Scan all tmux panes and identify AI panels
#[pyfunction]
fn scan_ai_panels(py: Python) -> PyResult<Vec<PyObject>> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let panels = runtime.block_on(detector::scan_ai_panels());
    
    let result: Vec<PyObject> = panels
        .into_iter()
        .map(|panel| panel_to_pydict(py, panel).unwrap().into())
        .collect();
    
    Ok(result)
}

/// List all tmux panes
#[pyfunction]
fn list_panes(py: Python) -> PyResult<Vec<PyObject>> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let panes = runtime.block_on(tmux::list_panes());
    
    let result: Vec<PyObject> = panes
        .into_iter()
        .map(|pane| {
            let dict = PyDict::new(py);
            dict.set_item("session_name", pane.session_name).unwrap();
            dict.set_item("window_name", pane.window_name).unwrap();
            dict.set_item("pane_index", pane.pane_index).unwrap();
            dict.set_item("pane_id", pane.pane_id).unwrap();
            dict.set_item("pane_pid", pane.pane_pid).unwrap();
            dict.set_item("pane_current_command", pane.pane_current_command).unwrap();
            dict.set_item("pane_current_path", pane.pane_current_path).unwrap();
            dict.into()
        })
        .collect();
    
    Ok(result)
}

/// Capture pane content
#[pyfunction]
fn capture_pane(pane_id: &str) -> PyResult<String> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let content = runtime.block_on(tmux::capture_pane(pane_id));
    Ok(content)
}

fn panel_to_pydict(py: Python, panel: AIPanel) -> PyResult<&PyDict> {
    let dict = PyDict::new(py);
    dict.set_item("session", panel.session)?;
    dict.set_item("window", panel.window)?;
    dict.set_item("pane", panel.pane)?;
    dict.set_item("pane_id", panel.pane_id)?;
    dict.set_item("ai_type", panel.ai_type)?;
    dict.set_item("working_dir", panel.working_dir)?;
    dict.set_item("is_active", panel.is_active)?;
    dict.set_item("last_activity", panel.last_activity)?;
    Ok(dict)
}

#[pymodule]
fn tmux_kanban_core(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(scan_ai_panels, m)?)?;
    m.add_function(wrap_pyfunction!(list_panes, m)?)?;
    m.add_function(wrap_pyfunction!(capture_pane, m)?)?;
    Ok(())
}
