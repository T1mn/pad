//! Pure display-model adapter for the Codex-style Desktop sidebar.
//!
//! `crate::sidebar::codex` owns navigation and visibility.  This module only
//! resolves its lightweight ID rows against the durable Profile/Project/Task
//! records and returns values that a native or WebKit renderer can consume.
//! Keeping this boundary free of `ratatui` is intentional: the same model can
//! drive the macOS Desktop renderer, previews, and snapshot tests.

use crate::permission_policy::{Profile, Project, Section, Task, TaskStatus};
use crate::sidebar::codex::{
    CodexSidebarNode, CodexSidebarRow, CodexSidebarState, CodexSidebarView,
};
use serde::Serialize;

/// Number of visual columns represented by one sidebar hierarchy level.
///
/// The renderer may choose a different pixel value; this stable logical value
/// makes indentation testable without coupling the model to a UI toolkit.
pub(crate) const CODEX_SIDEBAR_INDENT_COLUMNS: u16 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CodexSidebarIcon {
    NewTask,
    Profile,
    Section,
    Project,
    Task,
    Missing,
}

impl CodexSidebarIcon {
    /// Stable, renderer-neutral symbol names.  A renderer can map these to
    /// SF Symbols, SVGs, or text without changing the sidebar data contract.
    pub(crate) const fn symbol(self) -> &'static str {
        match self {
            Self::NewTask => "plus.message",
            Self::Profile => "person.crop.circle",
            Self::Section => "folder.badge.gearshape",
            Self::Project => "folder",
            Self::Task => "text.bubble",
            Self::Missing => "questionmark.folder",
        }
    }
}

/// Display state for a sidebar task.  This intentionally mirrors the stable
/// domain states rather than exposing a UI-specific color or widget type.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CodexSidebarStatus {
    None,
    Idle,
    Starting,
    Running,
    Streaming,
    ToolRunning,
    NeedsApproval,
    NeedsInput,
    Compacting,
    Retrying,
    Disconnected,
    Failed,
    Completed,
}

impl From<TaskStatus> for CodexSidebarStatus {
    fn from(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Idle => Self::Idle,
            TaskStatus::Starting => Self::Starting,
            TaskStatus::Running => Self::Running,
            TaskStatus::Streaming => Self::Streaming,
            TaskStatus::ToolRunning => Self::ToolRunning,
            TaskStatus::NeedsApproval => Self::NeedsApproval,
            TaskStatus::NeedsInput => Self::NeedsInput,
            TaskStatus::Compacting => Self::Compacting,
            TaskStatus::Retrying => Self::Retrying,
            TaskStatus::Disconnected => Self::Disconnected,
            TaskStatus::Failed => Self::Failed,
            TaskStatus::Completed => Self::Completed,
        }
    }
}

impl CodexSidebarStatus {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Idle => "Idle",
            Self::Starting => "Starting",
            Self::Running => "Running",
            Self::Streaming => "Streaming",
            Self::ToolRunning => "Tool running",
            Self::NeedsApproval => "Needs approval",
            Self::NeedsInput => "Needs input",
            Self::Compacting => "Compacting",
            Self::Retrying => "Retrying",
            Self::Disconnected => "Disconnected",
            Self::Failed => "Failed",
            Self::Completed => "Completed",
        }
    }

    /// A compact renderer-neutral status symbol for rows that need a status
    /// affordance in a narrow sidebar.
    pub(crate) const fn symbol(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Idle => "circle",
            Self::Starting => "circle.dotted",
            Self::Running | Self::Streaming | Self::ToolRunning => "arrow.triangle.2.circlepath",
            Self::NeedsApproval => "hand.raised",
            Self::NeedsInput => "questionmark.circle",
            Self::Compacting => "arrow.down.right.and.arrow.up.left",
            Self::Retrying => "arrow.clockwise",
            Self::Disconnected => "wifi.slash",
            Self::Failed => "xmark.circle",
            Self::Completed => "checkmark.circle",
        }
    }
}

/// Stable, renderer-independent representation of one visible Codex sidebar
/// row.  All relationships are resolved by ID and missing references are
/// represented explicitly instead of causing a panic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct CodexSidebarDisplayRow {
    pub key: String,
    pub node: CodexSidebarNode,
    pub depth: u8,
    pub indent_columns: u16,
    pub icon: CodexSidebarIcon,
    pub title: String,
    pub status: CodexSidebarStatus,
    pub unread: bool,
    pub pinned: bool,
    pub archived: bool,
    pub missing_reference: bool,
}

/// Complete renderer-neutral snapshot for one Desktop sidebar frame.  It is
/// intentionally JSON-serializable so the future Tauri/WebKit bridge can send
/// one immutable payload instead of exposing Rust collections over IPC.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct CodexSidebarSnapshot {
    pub view: CodexSidebarView,
    pub query: String,
    pub active_profile_id: Option<String>,
    pub selected_key: Option<String>,
    pub rows: Vec<CodexSidebarDisplayRow>,
}

pub(crate) fn snapshot(state: &CodexSidebarState) -> CodexSidebarSnapshot {
    let rows = state.visible_rows();
    CodexSidebarSnapshot {
        view: state.view,
        query: state.query.clone(),
        active_profile_id: state.active_profile_id.clone(),
        selected_key: state.selected_key(),
        rows: display_rows(
            &rows,
            &state.profiles,
            &state.projects,
            &state.tasks,
            &state.sections,
        ),
    }
}

/// Resolve navigation rows against the current sidebar records.
///
/// The input slices are borrowed only for the duration of this pure mapping;
/// the output owns titles and IDs so a renderer can retain a snapshot safely.
/// `profiles` is accepted for Profile row resolution and future profile-level
/// badges, while Sections are used for section titles.
pub(crate) fn display_rows(
    rows: &[CodexSidebarRow],
    profiles: &[Profile],
    projects: &[Project],
    tasks: &[Task],
    sections: &[Section],
) -> Vec<CodexSidebarDisplayRow> {
    rows.iter()
        .map(|row| display_row(row, profiles, projects, tasks, sections))
        .collect()
}

/// Resolve one row.  This is public within the crate to make it useful for
/// incremental updates when only one task changes state.
pub(crate) fn display_row(
    row: &CodexSidebarRow,
    profiles: &[Profile],
    projects: &[Project],
    tasks: &[Task],
    sections: &[Section],
) -> CodexSidebarDisplayRow {
    let depth = row.depth;
    let base = |icon: CodexSidebarIcon,
                title: String,
                status: CodexSidebarStatus,
                unread: bool,
                pinned: bool,
                archived: bool,
                missing_reference: bool| CodexSidebarDisplayRow {
        key: row.key(),
        node: row.node.clone(),
        depth,
        indent_columns: u16::from(depth) * CODEX_SIDEBAR_INDENT_COLUMNS,
        icon,
        title,
        status,
        unread,
        pinned,
        archived,
        missing_reference,
    };

    match &row.node {
        CodexSidebarNode::NewTask => base(
            CodexSidebarIcon::NewTask,
            "New task".to_string(),
            CodexSidebarStatus::None,
            false,
            false,
            false,
            false,
        ),
        CodexSidebarNode::Profile(id) => match profiles.iter().find(|profile| profile.id == *id) {
            Some(profile) => base(
                CodexSidebarIcon::Profile,
                preferred_profile_title(profile, id),
                CodexSidebarStatus::None,
                false,
                false,
                false,
                false,
            ),
            None => base(
                CodexSidebarIcon::Missing,
                missing_title("profile", id),
                CodexSidebarStatus::None,
                false,
                false,
                false,
                true,
            ),
        },
        CodexSidebarNode::Section(id) => match sections.iter().find(|section| section.id == *id) {
            Some(section) => base(
                CodexSidebarIcon::Section,
                preferred_section_title(section, id),
                CodexSidebarStatus::None,
                false,
                false,
                false,
                false,
            ),
            None => base(
                CodexSidebarIcon::Missing,
                missing_title("section", id),
                CodexSidebarStatus::None,
                false,
                false,
                false,
                true,
            ),
        },
        CodexSidebarNode::Project(id) => match projects.iter().find(|project| project.id == *id) {
            Some(project) => base(
                CodexSidebarIcon::Project,
                preferred_project_title(project, id),
                CodexSidebarStatus::None,
                false,
                project.pinned,
                project.archived,
                false,
            ),
            None => base(
                CodexSidebarIcon::Missing,
                missing_title("project", id),
                CodexSidebarStatus::None,
                false,
                false,
                false,
                true,
            ),
        },
        CodexSidebarNode::Task(id) => match tasks.iter().find(|task| task.id == *id) {
            Some(task) => base(
                CodexSidebarIcon::Task,
                preferred_task_title(task, id),
                task.status.into(),
                task.unread,
                task.pinned,
                task.archived,
                false,
            ),
            None => base(
                CodexSidebarIcon::Missing,
                missing_title("task", id),
                CodexSidebarStatus::None,
                false,
                false,
                false,
                true,
            ),
        },
    }
}

fn preferred_profile_title(profile: &Profile, id: &str) -> String {
    non_empty_or(profile.name.as_str(), id)
}

fn preferred_section_title(section: &Section, id: &str) -> String {
    non_empty_or(section.name.as_str(), id)
}

fn preferred_project_title(project: &Project, id: &str) -> String {
    if !project.name.trim().is_empty() {
        project.name.clone()
    } else if !project.primary_root.as_os_str().is_empty() {
        project.primary_root.display().to_string()
    } else {
        id.to_string()
    }
}

fn preferred_task_title(task: &Task, id: &str) -> String {
    if !task.title.trim().is_empty() {
        task.title.clone()
    } else if !task.summary.trim().is_empty() {
        task.summary.clone()
    } else {
        id.to_string()
    }
}

fn non_empty_or(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn missing_title(kind: &str, id: &str) -> String {
    format!("Missing {kind} · {id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn project(id: &str, name: &str) -> Project {
        Project {
            id: id.to_string(),
            name: name.to_string(),
            primary_root: PathBuf::from(format!("/tmp/{id}")),
            ..Project::default()
        }
    }

    fn task(id: &str, status: TaskStatus) -> Task {
        Task {
            id: id.to_string(),
            profile_id: "profile-1".to_string(),
            cwd: PathBuf::from("/tmp/work"),
            title: format!("Task {id}"),
            status,
            ..Task::default()
        }
    }

    #[test]
    fn project_and_task_keep_row_depth_and_logical_indent() {
        let rows = vec![
            CodexSidebarRow {
                depth: 1,
                node: CodexSidebarNode::Project("project-1".into()),
            },
            CodexSidebarRow {
                depth: 2,
                node: CodexSidebarNode::Task("task-1".into()),
            },
        ];
        let tasks = vec![task("task-1", TaskStatus::Idle)];

        let display = display_rows(&rows, &[], &[project("project-1", "PAD")], &tasks, &[]);

        assert_eq!(display[0].icon, CodexSidebarIcon::Project);
        assert_eq!(display[0].depth, 1);
        assert_eq!(display[0].indent_columns, 2);
        assert_eq!(display[1].icon, CodexSidebarIcon::Task);
        assert_eq!(display[1].depth, 2);
        assert_eq!(display[1].indent_columns, 4);
    }

    #[test]
    fn task_status_mapping_covers_domain_states() {
        let statuses = [
            TaskStatus::Idle,
            TaskStatus::Starting,
            TaskStatus::Running,
            TaskStatus::Streaming,
            TaskStatus::ToolRunning,
            TaskStatus::NeedsApproval,
            TaskStatus::NeedsInput,
            TaskStatus::Compacting,
            TaskStatus::Retrying,
            TaskStatus::Disconnected,
            TaskStatus::Failed,
            TaskStatus::Completed,
        ];

        for status in statuses {
            assert_eq!(
                CodexSidebarStatus::from(status).label(),
                match status {
                    TaskStatus::Idle => "Idle",
                    TaskStatus::Starting => "Starting",
                    TaskStatus::Running => "Running",
                    TaskStatus::Streaming => "Streaming",
                    TaskStatus::ToolRunning => "Tool running",
                    TaskStatus::NeedsApproval => "Needs approval",
                    TaskStatus::NeedsInput => "Needs input",
                    TaskStatus::Compacting => "Compacting",
                    TaskStatus::Retrying => "Retrying",
                    TaskStatus::Disconnected => "Disconnected",
                    TaskStatus::Failed => "Failed",
                    TaskStatus::Completed => "Completed",
                }
            );
        }
    }

    #[test]
    fn important_runtime_states_are_visible_on_task_rows() {
        let rows = vec![
            CodexSidebarRow {
                depth: 1,
                node: CodexSidebarNode::Task("approval".into()),
            },
            CodexSidebarRow {
                depth: 1,
                node: CodexSidebarNode::Task("running".into()),
            },
            CodexSidebarRow {
                depth: 1,
                node: CodexSidebarNode::Task("completed".into()),
            },
        ];
        let mut approval = task("approval", TaskStatus::NeedsApproval);
        approval.unread = true;
        approval.pinned = true;
        let running = task("running", TaskStatus::Running);
        let mut completed = task("completed", TaskStatus::Completed);
        completed.archived = true;

        let display = display_rows(&rows, &[], &[], &[approval, running, completed], &[]);

        assert_eq!(display[0].status, CodexSidebarStatus::NeedsApproval);
        assert!(display[0].unread);
        assert!(display[0].pinned);
        assert_eq!(display[1].status, CodexSidebarStatus::Running);
        assert_eq!(display[2].status, CodexSidebarStatus::Completed);
        assert!(display[2].archived);
    }

    #[test]
    fn missing_references_are_safe_and_explicit() {
        let rows = vec![
            CodexSidebarRow {
                depth: 0,
                node: CodexSidebarNode::Project("gone-project".into()),
            },
            CodexSidebarRow {
                depth: 1,
                node: CodexSidebarNode::Task("gone-task".into()),
            },
            CodexSidebarRow {
                depth: 0,
                node: CodexSidebarNode::Profile("gone-profile".into()),
            },
            CodexSidebarRow {
                depth: 0,
                node: CodexSidebarNode::Section("gone-section".into()),
            },
        ];

        let display = display_rows(&rows, &[], &[], &[], &[]);

        assert_eq!(display.len(), 4);
        assert!(display.iter().all(|row| row.missing_reference));
        assert!(display
            .iter()
            .all(|row| row.icon == CodexSidebarIcon::Missing));
        assert_eq!(display[0].title, "Missing project · gone-project");
        assert_eq!(display[1].title, "Missing task · gone-task");
    }

    #[test]
    fn display_rows_have_stable_ipc_shape() {
        let row = display_row(
            &CodexSidebarRow {
                depth: 1,
                node: CodexSidebarNode::Task("task-1".into()),
            },
            &[],
            &[],
            &[],
            &[],
        );
        let value = serde_json::to_value(row).expect("serialize sidebar row");
        assert_eq!(value["node"]["kind"], "task");
        assert_eq!(value["node"]["id"], "task-1");
        assert_eq!(value["icon"], "missing");
        assert_eq!(value["missing_reference"], true);
    }

    #[test]
    fn snapshot_keeps_navigation_state_and_display_rows_together() {
        let mut state = CodexSidebarState::default();
        state.set_query("pi");
        let snapshot = snapshot(&state);
        assert_eq!(snapshot.query, "pi");
        assert_eq!(snapshot.view, CodexSidebarView::All);
        assert_eq!(snapshot.selected_key, None);
        assert_eq!(snapshot.rows[0].key, "new-task");
    }
}
