//! Codex-style sidebar read model.
//!
//! The legacy PAD sidebar is pane/thread-centric.  The Desktop sidebar needs
//! a different information architecture: profiles contain projects, projects
//! contain tasks, and sections/pinned/archive only change organization.  This
//! module owns that navigation projection without touching Pi's append-only
//! session journal or the legacy TUI item model.

use crate::permission_policy::{Profile, Project, Section, SectionItem, Task};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CodexSidebarView {
    #[default]
    All,
    Pinned,
    Archive,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub(crate) enum CodexSidebarNode {
    NewTask,
    Profile(String),
    Section(String),
    Project(String),
    Task(String),
}

impl CodexSidebarNode {
    pub(crate) fn key(&self) -> String {
        match self {
            Self::NewTask => "new-task".to_string(),
            Self::Profile(id) => format!("profile:{id}"),
            Self::Section(id) => format!("section:{id}"),
            Self::Project(id) => format!("project:{id}"),
            Self::Task(id) => format!("task:{id}"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct CodexSidebarRow {
    pub depth: u8,
    pub node: CodexSidebarNode,
}

impl CodexSidebarRow {
    pub(crate) fn key(&self) -> String {
        self.node.key()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CodexSidebarState {
    pub profiles: Vec<Profile>,
    pub projects: Vec<Project>,
    pub tasks: Vec<Task>,
    pub sections: Vec<Section>,
    pub active_profile_id: Option<String>,
    pub selected: Option<CodexSidebarNode>,
    pub view: CodexSidebarView,
    pub query: String,
    pub collapsed_sections: HashSet<String>,
    pub collapsed_projects: HashSet<String>,
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "native navigation methods remain available while Electron drives the same read model through IPC"
    )
)]
impl CodexSidebarState {
    pub(crate) fn replace_data(
        &mut self,
        profiles: Vec<Profile>,
        projects: Vec<Project>,
        tasks: Vec<Task>,
        sections: Vec<Section>,
    ) {
        self.profiles = profiles;
        self.projects = projects;
        self.tasks = tasks;
        self.sections = sections;

        let profile_ids: HashSet<_> = self
            .profiles
            .iter()
            .map(|profile| profile.id.as_str())
            .collect();
        if self
            .active_profile_id
            .as_deref()
            .is_none_or(|id| !profile_ids.contains(id))
        {
            self.active_profile_id = self.profiles.first().map(|profile| profile.id.clone());
        }

        let section_ids: HashSet<_> = self
            .sections
            .iter()
            .map(|section| section.id.as_str())
            .collect();
        self.collapsed_sections
            .retain(|id| section_ids.contains(id.as_str()));
        let project_ids: HashSet<_> = self
            .projects
            .iter()
            .map(|project| project.id.as_str())
            .collect();
        self.collapsed_projects
            .retain(|id| project_ids.contains(id.as_str()));

        if let Some(selected) = &self.selected {
            if !self.visible_rows().iter().any(|row| row.node == *selected) {
                self.selected = None;
            }
        }
    }

    pub(crate) fn set_view(&mut self, view: CodexSidebarView) {
        self.view = view;
        self.retain_visible_selection();
    }

    pub(crate) fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.retain_visible_selection();
    }

    #[allow(
        dead_code,
        reason = "direct selection remains part of the renderer-neutral navigation API"
    )]
    pub(crate) fn select(&mut self, node: CodexSidebarNode) -> bool {
        if self.visible_rows().iter().any(|row| row.node == node) {
            self.selected = Some(node);
            true
        } else {
            false
        }
    }

    pub(crate) fn selected_key(&self) -> Option<String> {
        self.selected.as_ref().map(CodexSidebarNode::key)
    }

    pub(crate) fn move_selection(&mut self, delta: isize) -> Option<CodexSidebarNode> {
        let rows = self.visible_rows();
        if rows.is_empty() {
            self.selected = None;
            return None;
        }

        let current = self
            .selected
            .as_ref()
            .and_then(|selected| rows.iter().position(|row| &row.node == selected))
            .unwrap_or(0);
        let len = rows.len() as isize;
        let next = (current as isize + delta).rem_euclid(len) as usize;
        self.selected = Some(rows[next].node.clone());
        self.selected.clone()
    }

    #[allow(
        dead_code,
        reason = "direct section toggling remains part of the renderer-neutral navigation API"
    )]
    pub(crate) fn toggle_section(&mut self, section_id: &str) -> bool {
        let Some(section) = self
            .sections
            .iter()
            .find(|section| section.id == section_id)
        else {
            return false;
        };
        let collapsed = section.collapsed || self.collapsed_sections.contains(section_id);
        if collapsed {
            self.collapsed_sections.remove(section_id);
        } else {
            self.collapsed_sections.insert(section_id.to_string());
        }
        true
    }

    #[allow(
        dead_code,
        reason = "direct project toggling remains part of the renderer-neutral navigation API"
    )]
    pub(crate) fn toggle_project(&mut self, project_id: &str) -> bool {
        if !self.projects.iter().any(|project| project.id == project_id) {
            return false;
        }
        if !self.collapsed_projects.insert(project_id.to_string()) {
            self.collapsed_projects.remove(project_id);
        }
        true
    }

    /// Return the rows the Desktop renderer should draw.  The returned list
    /// is deliberately a lightweight ID projection; renderers can look up the
    /// full record without duplicating or mutating the source data.
    pub(crate) fn visible_rows(&self) -> Vec<CodexSidebarRow> {
        let project_by_id: HashMap<&str, &Project> = self
            .projects
            .iter()
            .map(|project| (project.id.as_str(), project))
            .collect();
        let task_by_id: HashMap<&str, &Task> = self
            .tasks
            .iter()
            .map(|task| (task.id.as_str(), task))
            .collect();

        let project_matches: HashSet<&str> = self
            .projects
            .iter()
            .filter(|project| self.project_in_view(project) && self.matches_project(project))
            .map(|project| project.id.as_str())
            .collect();
        let task_matches: HashSet<&str> = self
            .tasks
            .iter()
            .filter(|task| self.task_in_view(task) && self.matches_task(task))
            .map(|task| task.id.as_str())
            .collect();

        let mut rows = vec![CodexSidebarRow {
            depth: 0,
            node: CodexSidebarNode::NewTask,
        }];
        if self.query.trim().is_empty() {
            rows.extend(self.profiles.iter().map(|profile| CodexSidebarRow {
                depth: 0,
                node: CodexSidebarNode::Profile(profile.id.clone()),
            }));
        }

        let mut rendered_projects = HashSet::new();
        let mut rendered_tasks = HashSet::new();
        let mut sections = self.sections.iter().collect::<Vec<_>>();
        sections.sort_by(|left, right| {
            left.order
                .cmp(&right.order)
                .then_with(|| left.name.cmp(&right.name))
        });

        for section in sections {
            let section_start = rows.len();
            let mut section_has_rows = false;
            let section_collapsed =
                section.collapsed || self.collapsed_sections.contains(&section.id);
            for item in &section.items {
                match item {
                    SectionItem::Project(project_id) => {
                        let Some(project) = project_by_id.get(project_id.as_str()) else {
                            continue;
                        };
                        if !self.project_visible(project, &project_matches, &task_matches) {
                            continue;
                        }
                        if rendered_projects.insert(project_id.clone()) {
                            section_has_rows = true;
                            rows.push(CodexSidebarRow {
                                depth: 1,
                                node: CodexSidebarNode::Project(project_id.clone()),
                            });
                            if !section_collapsed && !self.collapsed_projects.contains(project_id) {
                                self.push_project_tasks(
                                    &mut rows,
                                    project_id,
                                    project_matches.contains(project_id.as_str()),
                                    &task_by_id,
                                    &task_matches,
                                    &mut rendered_tasks,
                                );
                            }
                        }
                    }
                    SectionItem::Task(task_id) => {
                        let Some(task) = task_by_id.get(task_id.as_str()) else {
                            continue;
                        };
                        if !self.task_visible(task, &task_matches)
                            || !rendered_tasks.insert(task_id.clone())
                        {
                            continue;
                        }
                        section_has_rows = true;
                        rows.push(CodexSidebarRow {
                            depth: 1,
                            node: CodexSidebarNode::Task(task_id.clone()),
                        });
                    }
                }
            }
            if section_has_rows {
                rows.insert(
                    section_start,
                    CodexSidebarRow {
                        depth: 0,
                        node: CodexSidebarNode::Section(section.id.clone()),
                    },
                );
            }
        }

        for project in &self.projects {
            if !self.project_visible(project, &project_matches, &task_matches)
                || !rendered_projects.insert(project.id.clone())
            {
                continue;
            }
            rows.push(CodexSidebarRow {
                depth: 0,
                node: CodexSidebarNode::Project(project.id.clone()),
            });
            if !self.collapsed_projects.contains(&project.id) {
                self.push_project_tasks(
                    &mut rows,
                    &project.id,
                    project_matches.contains(project.id.as_str()),
                    &task_by_id,
                    &task_matches,
                    &mut rendered_tasks,
                );
            }
        }

        for task in &self.tasks {
            if task.project_id.is_some()
                || !self.task_visible(task, &task_matches)
                || !rendered_tasks.insert(task.id.clone())
            {
                continue;
            }
            rows.push(CodexSidebarRow {
                depth: 0,
                node: CodexSidebarNode::Task(task.id.clone()),
            });
        }
        rows
    }

    fn push_project_tasks(
        &self,
        rows: &mut Vec<CodexSidebarRow>,
        project_id: &str,
        project_matches: bool,
        task_by_id: &HashMap<&str, &Task>,
        task_matches: &HashSet<&str>,
        rendered_tasks: &mut HashSet<String>,
    ) {
        for task in self
            .tasks
            .iter()
            .filter(|task| task.project_id.as_deref() == Some(project_id))
        {
            if !self.task_visible(task, task_matches)
                || (!project_matches && !task_matches.contains(task.id.as_str()))
                || !rendered_tasks.insert(task.id.clone())
            {
                continue;
            }
            // Keep the lookup argument in the signature so the caller can
            // build one map for all rows; this also documents that task IDs,
            // rather than copied transcripts, are the sidebar contract.
            let _ = task_by_id.get(task.id.as_str());
            rows.push(CodexSidebarRow {
                depth: 1,
                node: CodexSidebarNode::Task(task.id.clone()),
            });
        }
    }

    fn project_visible(
        &self,
        project: &Project,
        project_matches: &HashSet<&str>,
        task_matches: &HashSet<&str>,
    ) -> bool {
        let project_or_child_in_view = self.project_in_view(project)
            || self.tasks.iter().any(|task| {
                task.project_id.as_deref() == Some(project.id.as_str()) && self.task_in_view(task)
            });
        project_or_child_in_view
            && (self.query.trim().is_empty()
                || project_matches.contains(project.id.as_str())
                || self.tasks.iter().any(|task| {
                    task.project_id.as_deref() == Some(project.id.as_str())
                        && task_matches.contains(task.id.as_str())
                }))
    }

    fn task_visible(&self, task: &Task, task_matches: &HashSet<&str>) -> bool {
        self.task_in_view(task)
            && (self.query.trim().is_empty() || task_matches.contains(task.id.as_str()))
    }

    fn project_in_view(&self, project: &Project) -> bool {
        match self.view {
            CodexSidebarView::All => !project.archived,
            CodexSidebarView::Pinned => !project.archived && project.pinned,
            CodexSidebarView::Archive => project.archived,
        }
    }

    fn task_in_view(&self, task: &Task) -> bool {
        match self.view {
            CodexSidebarView::All => !task.archived,
            CodexSidebarView::Pinned => !task.archived && task.pinned,
            CodexSidebarView::Archive => task.archived,
        }
    }

    fn matches_project(&self, project: &Project) -> bool {
        self.matches(&format!(
            "{} {:?} {:?}",
            project.name, project.primary_root, project.id
        ))
    }

    fn matches_task(&self, task: &Task) -> bool {
        self.matches(&format!(
            "{} {} {:?} {:?}",
            task.title, task.summary, task.cwd, task.id
        ))
    }

    fn matches(&self, haystack: &str) -> bool {
        let query = self.query.trim().to_ascii_lowercase();
        query.is_empty() || haystack.to_ascii_lowercase().contains(&query)
    }

    fn retain_visible_selection(&mut self) {
        if let Some(selected) = &self.selected {
            if !self.visible_rows().iter().any(|row| row.node == *selected) {
                self.selected = None;
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
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

    fn task(id: &str, project_id: Option<&str>, title: &str) -> Task {
        Task {
            id: id.to_string(),
            project_id: project_id.map(str::to_string),
            profile_id: "profile-1".to_string(),
            cwd: PathBuf::from("/tmp/work"),
            title: title.to_string(),
            ..Task::default()
        }
    }

    pub(crate) fn renders_codex_project_task_hierarchy_and_projectless_task() {
        let mut state = CodexSidebarState::default();
        state.replace_data(
            vec![Profile {
                id: "profile-1".into(),
                name: "Work".into(),
                ..Profile::default()
            }],
            vec![project("project-1", "PAD")],
            vec![
                task("task-1", Some("project-1"), "Pi runtime"),
                task("task-2", None, "Inbox"),
            ],
            vec![Section {
                id: "section-1".into(),
                name: "Projects".into(),
                items: vec![SectionItem::Project("project-1".into())],
                ..Section::default()
            }],
        );

        assert_eq!(
            state
                .visible_rows()
                .iter()
                .map(CodexSidebarRow::key)
                .collect::<Vec<_>>(),
            vec![
                "new-task",
                "profile:profile-1",
                "section:section-1",
                "project:project-1",
                "task:task-1",
                "task:task-2",
            ]
        );
    }

    pub(crate) fn pinned_view_keeps_project_for_a_pinned_task() {
        let mut state = CodexSidebarState::default();
        let mut pinned_task = task("task-1", Some("project-1"), "Pinned task");
        pinned_task.pinned = true;
        state.replace_data(
            Vec::new(),
            vec![project("project-1", "PAD")],
            vec![pinned_task],
            Vec::new(),
        );
        assert_eq!(state.visible_rows().len(), 3);
        state.set_view(CodexSidebarView::Pinned);
        assert_eq!(
            state
                .visible_rows()
                .iter()
                .map(CodexSidebarRow::key)
                .collect::<Vec<_>>(),
            vec!["new-task", "project:project-1", "task:task-1"]
        );
    }

    pub(crate) fn search_retains_matching_project_ancestor() {
        let mut state = CodexSidebarState::default();
        state.replace_data(
            Vec::new(),
            vec![project("project-1", "PAD")],
            vec![task("task-1", Some("project-1"), "Build Pi")],
            Vec::new(),
        );
        state.set_query("build");
        assert_eq!(
            state
                .visible_rows()
                .iter()
                .map(CodexSidebarRow::key)
                .collect::<Vec<_>>(),
            vec!["new-task", "project:project-1", "task:task-1"]
        );
    }

    pub(crate) fn selection_wraps_over_visible_codex_rows() {
        let mut state = CodexSidebarState::default();
        state.replace_data(
            Vec::new(),
            vec![project("project-1", "PAD")],
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(
            state.move_selection(-1),
            Some(CodexSidebarNode::Project("project-1".into()))
        );
        assert_eq!(state.move_selection(1), Some(CodexSidebarNode::NewTask));
    }
}
