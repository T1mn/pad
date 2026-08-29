//! Renderer-safe Desktop terminal control plane.
//!
//! Electron may select an existing PAD Task and send bytes to its pane, but
//! it never selects a program, cwd, or environment. The Rust runtime resolves
//! those trusted values from the private Store and owns every PTY child.

use super::DesktopRuntime;
use crate::permission_policy::{Profile, Task};
use crate::terminal_runtime::{
    AlacrittyEngineFactory, ControllerQueueError, CursorShape, EngineId, EngineRegistry,
    EngineRuntime, LivePaneRuntime, NativePaneRequest, NativePtyCommand, PaneEpoch, PaneId,
    PaneRuntime, PaneSpec, TerminalController, TerminalFrameReader, TerminalSize, TransportId,
    TransportRuntime, ALACRITTY_ENGINE_ID,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(crate) const MAX_TERMINAL_PANES: usize = 8;
pub(crate) const MAX_TERMINAL_COLUMNS: u16 = 240;
pub(crate) const MAX_TERMINAL_ROWS: u16 = 80;
pub(crate) const MAX_TERMINAL_INPUT_BYTES: usize = 64 * 1024;
const MAX_TERMINAL_PANE_ID_BYTES: usize = 128;
const MAX_TERMINAL_LABEL_CHARS: usize = 128;
const MAX_TERMINAL_LINE_BYTES: usize = 4096;

#[derive(Debug)]
pub(crate) struct DesktopTerminalError {
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

impl DesktopTerminalError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct TerminalSizeDto {
    pub(crate) columns: u16,
    pub(crate) rows: u16,
}

impl From<TerminalSize> for TerminalSizeDto {
    fn from(size: TerminalSize) -> Self {
        Self {
            columns: size.columns,
            rows: size.rows,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct TerminalOpenDto {
    pub(crate) pane_id: String,
    pub(crate) task_id: String,
    pub(crate) epoch: u64,
    pub(crate) status: &'static str,
    pub(crate) size: TerminalSizeDto,
}

#[derive(Debug, Serialize)]
pub(crate) struct TerminalAcceptedDto {
    pub(crate) pane_id: String,
    pub(crate) accepted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) bytes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) size: Option<TerminalSizeDto>,
}

#[derive(Debug, Serialize)]
pub(crate) struct TerminalCloseDto {
    pub(crate) pane_id: String,
    pub(crate) closed: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct TerminalCursorDto {
    pub(crate) column: u16,
    pub(crate) row: u16,
    pub(crate) shape: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct TerminalModeDto {
    pub(crate) alternate_screen: bool,
    pub(crate) bracketed_paste: bool,
    pub(crate) mouse_reporting: bool,
    pub(crate) sgr_mouse: bool,
    pub(crate) application_cursor: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct TerminalViewportDto {
    pub(crate) display_offset: usize,
    pub(crate) history_size: usize,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct TerminalExitDto {
    pub(crate) code: Option<i32>,
    pub(crate) signaled: bool,
}

/// A deliberately bounded, style-free terminal projection. It cannot contain
/// a process command, cwd, environment, raw cell attributes, or scrollback.
#[derive(Debug, Serialize)]
pub(crate) struct TerminalSnapshotDto {
    pub(crate) pane_id: String,
    pub(crate) task_id: String,
    pub(crate) epoch: u64,
    pub(crate) revision: u64,
    pub(crate) status: &'static str,
    pub(crate) is_open: bool,
    pub(crate) size: TerminalSizeDto,
    pub(crate) lines: Vec<String>,
    pub(crate) cursor: Option<TerminalCursorDto>,
    pub(crate) mode: TerminalModeDto,
    pub(crate) viewport: TerminalViewportDto,
    pub(crate) error: Option<String>,
    pub(crate) exit: Option<TerminalExitDto>,
}

struct PaneState {
    runtime_id: PaneId,
    task_id: String,
    profile_id: String,
    epoch: PaneEpoch,
    size: TerminalSize,
}

pub(crate) struct DesktopTerminalRuntime {
    controller: Option<TerminalController>,
    frames: Option<TerminalFrameReader>,
    panes: BTreeMap<(String, String), PaneState>,
    next_pane: u64,
    #[cfg(test)]
    program_override: Option<PathBuf>,
}

impl Default for DesktopTerminalRuntime {
    fn default() -> Self {
        Self {
            controller: None,
            frames: None,
            panes: BTreeMap::new(),
            next_pane: 1,
            #[cfg(test)]
            program_override: None,
        }
    }
}

impl Drop for DesktopTerminalRuntime {
    fn drop(&mut self) {
        if let Some(controller) = self.controller.take() {
            let _ = controller.shutdown();
        }
    }
}

impl DesktopRuntime {
    pub(crate) fn terminal_open(
        &mut self,
        task_id: &str,
        requested_pane_id: Option<&str>,
        label: Option<&str>,
        columns: Option<u16>,
        rows: Option<u16>,
    ) -> Result<TerminalOpenDto, DesktopTerminalError> {
        let active_profile_id = self.terminal_active_profile_id()?;
        let task = self
            .store
            .get_task(task_id)
            .map_err(|error| DesktopTerminalError::new("store_error", error.to_string()))?
            .ok_or_else(|| {
                DesktopTerminalError::new(
                    "task_not_found",
                    "task is unavailable for the active profile",
                )
            })?;
        if task.profile_id != active_profile_id {
            return Err(DesktopTerminalError::new(
                "task_not_found",
                "task is unavailable for the active profile",
            ));
        }
        let profile = self
            .store
            .get_profile(&task.profile_id)
            .map_err(|error| DesktopTerminalError::new("store_error", error.to_string()))?
            .ok_or_else(|| {
                DesktopTerminalError::new(
                    "profile_not_found",
                    format!("Desktop profile '{}' was not found", task.profile_id),
                )
            })?;
        self.terminal
            .open(&task, &profile, requested_pane_id, label, columns, rows)
    }

    pub(crate) fn terminal_input(
        &mut self,
        pane_id: &str,
        data: &str,
    ) -> Result<TerminalAcceptedDto, DesktopTerminalError> {
        let active_profile_id = self.terminal_active_profile_id()?;
        self.terminal.input(pane_id, &active_profile_id, data)
    }

    pub(crate) fn terminal_resize(
        &mut self,
        pane_id: &str,
        columns: u16,
        rows: u16,
    ) -> Result<TerminalAcceptedDto, DesktopTerminalError> {
        let active_profile_id = self.terminal_active_profile_id()?;
        self.terminal
            .resize(pane_id, &active_profile_id, columns, rows)
    }

    pub(crate) fn terminal_snapshot(
        &self,
        pane_id: &str,
    ) -> Result<TerminalSnapshotDto, DesktopTerminalError> {
        let active_profile_id = self.terminal_active_profile_id()?;
        self.terminal.snapshot(pane_id, &active_profile_id)
    }

    pub(crate) fn terminal_close(
        &mut self,
        pane_id: &str,
    ) -> Result<TerminalCloseDto, DesktopTerminalError> {
        let active_profile_id = self.terminal_active_profile_id()?;
        self.terminal.close(pane_id, &active_profile_id)
    }

    fn terminal_active_profile_id(&self) -> Result<String, DesktopTerminalError> {
        self.desktop_ui_state()
            .map_err(|error| DesktopTerminalError::new("store_error", error.to_string()))?
            .active_profile_id
            .ok_or_else(|| {
                DesktopTerminalError::new(
                    "profile_not_active",
                    "profile is unavailable for the active profile",
                )
            })
    }

    #[cfg(test)]
    pub(crate) fn set_terminal_program_for_test(&mut self, program: PathBuf) {
        self.terminal.program_override = Some(program);
    }
}

impl DesktopTerminalRuntime {
    fn ensure_controller(&mut self) -> Result<&TerminalController, DesktopTerminalError> {
        if self.controller.is_none() {
            let worker_count = std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1)
                .min(4);
            let mut registry = EngineRegistry::default();
            registry.register(EngineId::new(ALACRITTY_ENGINE_ID), AlacrittyEngineFactory);
            let engines = EngineRuntime::start(worker_count, registry)
                .map_err(|error| terminal_error("terminal_unavailable", error))?;
            let runtime =
                LivePaneRuntime::new(PaneRuntime::new(engines), TransportRuntime::default());
            let controller = TerminalController::start(runtime)
                .map_err(|error| terminal_error("terminal_unavailable", error))?;
            self.frames = Some(controller.frames());
            self.controller = Some(controller);
        }
        self.controller.as_ref().ok_or_else(|| {
            DesktopTerminalError::new("terminal_unavailable", "terminal controller is unavailable")
        })
    }

    fn open(
        &mut self,
        task: &Task,
        profile: &Profile,
        requested_pane_id: Option<&str>,
        label: Option<&str>,
        columns: Option<u16>,
        rows: Option<u16>,
    ) -> Result<TerminalOpenDto, DesktopTerminalError> {
        if self
            .panes
            .values()
            .filter(|pane| pane.profile_id == profile.id)
            .count()
            >= MAX_TERMINAL_PANES
        {
            return Err(DesktopTerminalError::new(
                "terminal_pane_limit",
                "Desktop terminal pane limit reached",
            ));
        }
        let cwd = safe_task_cwd(task, profile)?;
        let size = validate_size(columns.unwrap_or(100), rows.unwrap_or(30))?;
        let sequence = self.next_pane;
        self.next_pane = self.next_pane.saturating_add(1).max(1);
        let pane_id = match requested_pane_id {
            Some(pane_id) => validate_pane_id(pane_id)?.to_string(),
            None => format!("terminal-{}-{sequence}", std::process::id()),
        };
        let pane_key = (profile.id.clone(), pane_id.clone());
        if self.panes.contains_key(&pane_key) {
            return Err(DesktopTerminalError::new(
                "terminal_pane_exists",
                "terminal pane already exists for the active profile",
            ));
        }
        let label = label.unwrap_or(&task.title);
        if label.chars().count() > MAX_TERMINAL_LABEL_CHARS {
            return Err(DesktopTerminalError::new(
                "invalid_terminal_label",
                "terminal label exceeds 128 characters",
            ));
        }
        let runtime_id = PaneId::new(format!("desktop-{}-{sequence}", std::process::id()));
        let request = NativePaneRequest {
            spec: PaneSpec {
                id: runtime_id.clone(),
                label: label.to_string(),
                engine_id: EngineId::new(ALACRITTY_ENGINE_ID),
                transport_id: TransportId::new(format!(
                    "desktop:{}:{sequence}",
                    std::process::id()
                )),
            },
            size,
            command: self.native_command(&cwd),
        };
        let epoch = self
            .ensure_controller()?
            .open_native(request)
            .map_err(queue_error)?;
        self.panes.insert(
            pane_key,
            PaneState {
                runtime_id,
                task_id: task.id.clone(),
                profile_id: task.profile_id.clone(),
                epoch,
                size,
            },
        );
        Ok(TerminalOpenDto {
            pane_id,
            task_id: task.id.clone(),
            epoch: epoch.get(),
            status: "opening",
            size: size.into(),
        })
    }

    fn input(
        &self,
        pane_id: &str,
        active_profile_id: &str,
        data: &str,
    ) -> Result<TerminalAcceptedDto, DesktopTerminalError> {
        validate_pane_id(pane_id)?;
        if data.len() > MAX_TERMINAL_INPUT_BYTES {
            return Err(DesktopTerminalError::new(
                "terminal_input_too_large",
                "terminal input exceeds 64 KiB",
            ));
        }
        let pane = self.pane_for_profile(pane_id, active_profile_id)?;
        self.controller()?
            .input(
                pane.runtime_id.clone(),
                pane.epoch,
                data.as_bytes().to_vec(),
            )
            .map_err(queue_error)?;
        Ok(TerminalAcceptedDto {
            pane_id: pane_id.to_string(),
            accepted: true,
            bytes: Some(data.len()),
            size: None,
        })
    }

    fn resize(
        &mut self,
        pane_id: &str,
        active_profile_id: &str,
        columns: u16,
        rows: u16,
    ) -> Result<TerminalAcceptedDto, DesktopTerminalError> {
        validate_pane_id(pane_id)?;
        let size = validate_size(columns, rows)?;
        let pane = self.pane_for_profile(pane_id, active_profile_id)?;
        let runtime_id = pane.runtime_id.clone();
        let epoch = pane.epoch;
        self.controller()?
            .resize(runtime_id, epoch, size)
            .map_err(queue_error)?;
        if let Some(pane) = self
            .panes
            .get_mut(&(active_profile_id.to_string(), pane_id.to_string()))
        {
            pane.size = size;
        }
        Ok(TerminalAcceptedDto {
            pane_id: pane_id.to_string(),
            accepted: true,
            bytes: None,
            size: Some(size.into()),
        })
    }

    fn snapshot(
        &self,
        pane_id: &str,
        active_profile_id: &str,
    ) -> Result<TerminalSnapshotDto, DesktopTerminalError> {
        validate_pane_id(pane_id)?;
        let pane = self.pane_for_profile(pane_id, active_profile_id)?;
        let published = self
            .frames
            .as_ref()
            .and_then(|frames| frames.latest_for_epoch(&pane.runtime_id, pane.epoch));
        let (revision, is_open, frame, error, exit) =
            published
                .as_ref()
                .map_or((0, false, None, None, None), |published| {
                    (
                        published.revision,
                        published.is_open,
                        published.frame.as_ref(),
                        published.error.as_deref().map(str::to_string),
                        published.exit,
                    )
                });
        let size = frame.map_or(pane.size, |frame| frame.terminal.size);
        let lines = frame.map_or_else(Vec::new, |frame| {
            let rows = frame.terminal.size.rows.min(MAX_TERMINAL_ROWS);
            (0..rows)
                .filter_map(|row| frame.terminal.row_text(row))
                .map(|line| truncate_utf8(line, MAX_TERMINAL_LINE_BYTES))
                .collect()
        });
        let cursor =
            frame
                .and_then(|frame| frame.terminal.cursor)
                .map(|cursor| TerminalCursorDto {
                    column: cursor.column,
                    row: cursor.row,
                    shape: match cursor.shape {
                        CursorShape::Block => "block",
                        CursorShape::Underline => "underline",
                        CursorShape::Beam => "beam",
                        CursorShape::HollowBlock => "hollow_block",
                    },
                });
        let mode = frame.map(|frame| frame.terminal.mode).unwrap_or_default();
        let viewport = frame
            .map(|frame| frame.terminal.viewport)
            .unwrap_or_default();
        let exit = exit.map(|exit| TerminalExitDto {
            code: exit.code,
            signaled: exit.signaled,
        });
        let status = if error.is_some() {
            "failed"
        } else if exit.is_some() {
            "exited"
        } else if is_open {
            "running"
        } else {
            "opening"
        };
        Ok(TerminalSnapshotDto {
            pane_id: pane_id.to_string(),
            task_id: pane.task_id.clone(),
            epoch: pane.epoch.get(),
            revision,
            status,
            is_open,
            size: size.into(),
            lines,
            cursor,
            mode: TerminalModeDto {
                alternate_screen: mode.alternate_screen,
                bracketed_paste: mode.bracketed_paste,
                mouse_reporting: mode.mouse_reporting,
                sgr_mouse: mode.sgr_mouse,
                application_cursor: mode.application_cursor,
            },
            viewport: TerminalViewportDto {
                display_offset: viewport.display_offset,
                history_size: viewport.history_size,
            },
            error,
            exit,
        })
    }

    fn close(
        &mut self,
        pane_id: &str,
        active_profile_id: &str,
    ) -> Result<TerminalCloseDto, DesktopTerminalError> {
        validate_pane_id(pane_id)?;
        let pane = self.pane_for_profile(pane_id, active_profile_id)?;
        self.controller()?
            .close(pane.runtime_id.clone(), pane.epoch)
            .map_err(queue_error)?;
        self.panes
            .remove(&(active_profile_id.to_string(), pane_id.to_string()));
        Ok(TerminalCloseDto {
            pane_id: pane_id.to_string(),
            closed: true,
        })
    }

    fn pane_for_profile(
        &self,
        pane_id: &str,
        active_profile_id: &str,
    ) -> Result<&PaneState, DesktopTerminalError> {
        self.panes
            .get(&(active_profile_id.to_string(), pane_id.to_string()))
            .ok_or_else(|| {
                DesktopTerminalError::new(
                    "terminal_pane_not_found",
                    "terminal pane is unavailable for the active profile",
                )
            })
    }

    fn controller(&self) -> Result<&TerminalController, DesktopTerminalError> {
        self.controller.as_ref().ok_or_else(|| {
            DesktopTerminalError::new("terminal_unavailable", "terminal controller is unavailable")
        })
    }

    fn native_command(&self, cwd: &Path) -> NativePtyCommand {
        #[cfg(test)]
        let command = self
            .program_override
            .as_ref()
            .map(NativePtyCommand::new)
            .unwrap_or_else(NativePtyCommand::default_program);
        #[cfg(not(test))]
        let command = NativePtyCommand::default_program();
        let mut command = command.cwd(cwd).clear_env();
        for (key, value) in std::env::vars_os() {
            let key_text = key.to_string_lossy();
            if matches!(
                key_text.as_ref(),
                "HOME" | "USER" | "LOGNAME" | "SHELL" | "PATH" | "TMPDIR" | "LANG" | "TERMINFO"
            ) || key_text.starts_with("LC_")
            {
                command = command.env(&key, &value);
            }
        }
        command
    }
}

fn validate_pane_id(pane_id: &str) -> Result<&str, DesktopTerminalError> {
    if pane_id.is_empty()
        || pane_id.len() > MAX_TERMINAL_PANE_ID_BYTES
        || !pane_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(DesktopTerminalError::new(
            "invalid_terminal_pane_id",
            "pane_id must be 1-128 ASCII letters, digits, '.', '_' or '-'",
        ));
    }
    Ok(pane_id)
}

fn validate_size(columns: u16, rows: u16) -> Result<TerminalSize, DesktopTerminalError> {
    if columns == 0 || columns > MAX_TERMINAL_COLUMNS || rows == 0 || rows > MAX_TERMINAL_ROWS {
        return Err(DesktopTerminalError::new(
            "invalid_terminal_size",
            format!(
                "terminal size must be within 1..={MAX_TERMINAL_COLUMNS} columns and 1..={MAX_TERMINAL_ROWS} rows"
            ),
        ));
    }
    Ok(TerminalSize::new(columns, rows))
}

fn safe_task_cwd(task: &Task, profile: &Profile) -> Result<PathBuf, DesktopTerminalError> {
    let cwd = task.cwd.canonicalize().map_err(|_| {
        DesktopTerminalError::new(
            "invalid_terminal_workspace",
            "task workspace is unavailable",
        )
    })?;
    if !cwd.is_dir() {
        return Err(DesktopTerminalError::new(
            "invalid_terminal_workspace",
            "task workspace is not a directory",
        ));
    }
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    let (agent_dir, session_dir) = crate::pi_runtime::profile_pi_roots(profile);
    let mut private_roots = vec![
        crate::paths::pad_desktop_data_dir(),
        crate::paths::pad_home_dir(),
        home.join(".codex"),
        home.join(".chatgpt"),
        agent_dir,
        session_dir,
    ];
    if let Some(codex_home) = crate::paths::base::protected_codex_home() {
        private_roots.push(codex_home);
    }
    if private_roots.iter().any(|root| path_is_within(&cwd, root)) {
        return Err(DesktopTerminalError::new(
            "protected_terminal_workspace",
            "task workspace is inside a protected application-data namespace",
        ));
    }
    Ok(cwd)
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    if root.as_os_str().is_empty() {
        return false;
    }
    if path.starts_with(root) {
        return true;
    }
    root.canonicalize()
        .ok()
        .is_some_and(|root| path.starts_with(root))
}

fn truncate_utf8(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let mut boundary = max_bytes;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
    value
}

fn queue_error<T>(error: ControllerQueueError<T>) -> DesktopTerminalError {
    let code = if error.is_full() {
        "terminal_busy"
    } else {
        "terminal_unavailable"
    };
    DesktopTerminalError::new(code, error.to_string())
}

fn terminal_error(code: &'static str, error: impl std::fmt::Display) -> DesktopTerminalError {
    DesktopTerminalError::new(code, error.to_string())
}
