use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

pub const DEFAULT_SPLIT_RATIO_PER_MILLE: u16 = 500;
pub(super) const MAX_TERMINAL_LABEL_CHARS: usize = 120;
pub(super) const MAX_TERMINAL_PANES: usize = 32;
pub(super) const MAX_TERMINAL_TABS: usize = 16;
const MAX_TERMINAL_LAYOUT_DEPTH: usize = MAX_TERMINAL_PANES;
const MAX_ALLOCATABLE_PANE_SERIAL: u64 = u64::MAX - 2;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TerminalPaneId(u64);

impl TerminalPaneId {
    pub fn new(serial: u64) -> Self {
        Self(serial)
    }

    pub fn serial(self) -> u64 {
        self.0
    }
}

impl fmt::Display for TerminalPaneId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalProfile {
    #[default]
    Shell,
    Codex,
    Claude,
    GithubCli,
}

impl TerminalProfile {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Shell => "Shell",
            Self::Codex => "Codex",
            Self::Claude => "Claude",
            Self::GithubCli => "GitHub CLI",
        }
    }

    pub fn default_command(self) -> TerminalCommandDefinition {
        match self {
            Self::Shell => TerminalCommandDefinition::default_shell(),
            Self::Codex => TerminalCommandDefinition::program("codex"),
            Self::Claude => TerminalCommandDefinition::program("claude"),
            // `gh` is a one-shot CLI and exits immediately without a
            // subcommand. Keep this profile alive as an interactive shell in
            // a clearly labelled GitHub workspace instead.
            Self::GithubCli => TerminalCommandDefinition::default_shell(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TerminalCommandDefinition {
    /// `None` selects the platform's default interactive shell.
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
}

impl TerminalCommandDefinition {
    pub fn default_shell() -> Self {
        Self::default()
    }

    pub fn program(program: impl Into<String>) -> Self {
        Self {
            program: Some(program.into()),
            args: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TerminalPaneDefinition {
    pub id: TerminalPaneId,
    pub label: String,
    pub profile: TerminalProfile,
    pub cwd: PathBuf,
    /// Runtime launch data is derived from `profile` on restore. It is never
    /// trusted from disk, which prevents edited workspace JSON from launching
    /// an arbitrary executable on PAD startup.
    #[serde(skip)]
    pub command: TerminalCommandDefinition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalSplitAxis {
    /// Divide the area into top and bottom rows.
    Rows,
    /// Divide the area into left and right columns.
    Columns,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerminalLayoutNode {
    Pane {
        pane_id: TerminalPaneId,
    },
    Split {
        axis: TerminalSplitAxis,
        ratio_per_mille: u16,
        first: Box<TerminalLayoutNode>,
        second: Box<TerminalLayoutNode>,
    },
}

impl TerminalLayoutNode {
    pub fn pane(pane_id: TerminalPaneId) -> Self {
        Self::Pane { pane_id }
    }

    pub fn contains(&self, target: TerminalPaneId) -> bool {
        match self {
            Self::Pane { pane_id } => *pane_id == target,
            Self::Split { first, second, .. } => first.contains(target) || second.contains(target),
        }
    }

    pub fn pane_ids(&self) -> Vec<TerminalPaneId> {
        let mut panes = Vec::new();
        self.collect_pane_ids(&mut panes);
        panes
    }

    pub fn split_pane(
        &mut self,
        target: TerminalPaneId,
        new_pane: TerminalPaneId,
        axis: TerminalSplitAxis,
    ) -> bool {
        match self {
            Self::Pane { pane_id } if *pane_id == target => {
                let original = *pane_id;
                *self = Self::Split {
                    axis,
                    ratio_per_mille: DEFAULT_SPLIT_RATIO_PER_MILLE,
                    first: Box::new(Self::pane(original)),
                    second: Box::new(Self::pane(new_pane)),
                };
                true
            }
            Self::Pane { .. } => false,
            Self::Split { first, second, .. } => {
                first.split_pane(target, new_pane, axis)
                    || second.split_pane(target, new_pane, axis)
            }
        }
    }

    fn collect_pane_ids(&self, panes: &mut Vec<TerminalPaneId>) {
        match self {
            Self::Pane { pane_id } => panes.push(*pane_id),
            Self::Split { first, second, .. } => {
                first.collect_pane_ids(panes);
                second.collect_pane_ids(panes);
            }
        }
    }

    fn remove_pane(self, target: TerminalPaneId) -> (Option<Self>, bool) {
        match self {
            Self::Pane { pane_id } if pane_id == target => (None, true),
            pane @ Self::Pane { .. } => (Some(pane), false),
            Self::Split {
                axis,
                ratio_per_mille,
                first,
                second,
            } => {
                let (first, removed_first) = first.remove_pane(target);
                if removed_first {
                    return match (first, Some(*second)) {
                        (Some(first), Some(second)) => (
                            Some(Self::Split {
                                axis,
                                ratio_per_mille,
                                first: Box::new(first),
                                second: Box::new(second),
                            }),
                            true,
                        ),
                        (Some(remaining), None) | (None, Some(remaining)) => {
                            (Some(remaining), true)
                        }
                        (None, None) => (None, true),
                    };
                }

                let first = first.expect("unremoved split child remains present");
                let (second, removed_second) = second.remove_pane(target);
                if removed_second {
                    return match second {
                        Some(second) => (
                            Some(Self::Split {
                                axis,
                                ratio_per_mille,
                                first: Box::new(first),
                                second: Box::new(second),
                            }),
                            true,
                        ),
                        None => (Some(first), true),
                    };
                }

                (
                    Some(Self::Split {
                        axis,
                        ratio_per_mille,
                        first: Box::new(first),
                        second: Box::new(second.expect("unremoved split child remains present")),
                    }),
                    false,
                )
            }
        }
    }

    fn validate(
        &self,
        known: &HashSet<TerminalPaneId>,
        visited: &mut HashSet<TerminalPaneId>,
        depth: usize,
    ) -> Result<(), String> {
        if depth > MAX_TERMINAL_LAYOUT_DEPTH {
            return Err(format!(
                "terminal layout cannot exceed {MAX_TERMINAL_LAYOUT_DEPTH} nested levels"
            ));
        }
        match self {
            Self::Pane { pane_id } => {
                if !known.contains(pane_id) {
                    return Err(format!("layout references unknown terminal pane {pane_id}"));
                }
                if !visited.insert(*pane_id) {
                    return Err(format!("terminal pane {pane_id} occurs more than once"));
                }
            }
            Self::Split {
                ratio_per_mille,
                first,
                second,
                ..
            } => {
                if !(1..1000).contains(ratio_per_mille) {
                    return Err(format!(
                        "terminal split ratio {ratio_per_mille} must be between 1 and 999"
                    ));
                }
                first.validate(known, visited, depth + 1)?;
                second.validate(known, visited, depth + 1)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TerminalTab {
    pub root: TerminalLayoutNode,
    pub focused: TerminalPaneId,
    #[serde(default)]
    pub label: Option<String>,
}

impl TerminalTab {
    pub fn new(pane_id: TerminalPaneId) -> Self {
        Self {
            root: TerminalLayoutNode::pane(pane_id),
            focused: pane_id,
            label: None,
        }
    }

    pub fn pane_ids(&self) -> Vec<TerminalPaneId> {
        self.root.pane_ids()
    }

    fn remove_pane(&mut self, target: TerminalPaneId) -> bool {
        let panes = self.pane_ids();
        let Some(position) = panes.iter().position(|pane_id| *pane_id == target) else {
            return false;
        };
        let next_focus = panes
            .get(position + 1)
            .or_else(|| position.checked_sub(1).and_then(|index| panes.get(index)))
            .copied();
        let (root, removed) = self.root.clone().remove_pane(target);
        debug_assert!(removed);
        let became_empty = root.is_none();
        if let Some(root) = root {
            self.root = root;
            if self.focused == target {
                self.focused = next_focus.expect("non-empty tab has a neighboring pane");
            }
        }
        became_empty
    }
}

fn first_pane_serial() -> u64 {
    1
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TerminalWorkspace {
    #[serde(default)]
    pub tabs: Vec<TerminalTab>,
    #[serde(default)]
    pub active_tab: usize,
    #[serde(default)]
    pub panes: Vec<TerminalPaneDefinition>,
    #[serde(default = "first_pane_serial")]
    pub next_pane_serial: u64,
}

impl Default for TerminalWorkspace {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
            panes: Vec::new(),
            next_pane_serial: first_pane_serial(),
        }
    }
}

impl TerminalWorkspace {
    pub fn focused_pane_id(&self) -> Option<TerminalPaneId> {
        self.tabs.get(self.active_tab).map(|tab| tab.focused)
    }

    #[cfg(test)]
    pub fn visible_pane_ids(&self) -> Vec<TerminalPaneId> {
        self.tabs
            .get(self.active_tab)
            .map(TerminalTab::pane_ids)
            .unwrap_or_default()
    }

    pub fn pane(&self, pane_id: TerminalPaneId) -> Option<&TerminalPaneDefinition> {
        self.panes.iter().find(|pane| pane.id == pane_id)
    }

    pub fn active_tab(&self) -> Option<&TerminalTab> {
        self.tabs.get(self.active_tab)
    }

    pub fn add_tab(&mut self, profile: TerminalProfile, cwd: PathBuf) -> Option<TerminalPaneId> {
        if self.tabs.len() >= MAX_TERMINAL_TABS || self.panes.len() >= MAX_TERMINAL_PANES {
            return None;
        }
        let pane = self.allocate_pane(profile, cwd)?;
        let pane_id = pane.id;
        self.panes.push(pane);
        self.tabs.push(TerminalTab::new(pane_id));
        self.active_tab = self.tabs.len() - 1;
        Some(pane_id)
    }

    pub fn split_focused(
        &mut self,
        axis: TerminalSplitAxis,
        profile: TerminalProfile,
        cwd: PathBuf,
    ) -> Option<TerminalPaneId> {
        if self.panes.len() >= MAX_TERMINAL_PANES {
            return None;
        }
        let tab = self.tabs.get_mut(self.active_tab)?;
        let focused = tab.focused;
        let pane = allocate_pane(&mut self.next_pane_serial, profile, cwd)?;
        let pane_id = pane.id;
        if !tab.root.split_pane(focused, pane_id, axis) {
            return None;
        }
        tab.focused = pane_id;
        self.panes.push(pane);
        Some(pane_id)
    }

    pub fn close_pane(&mut self, pane_id: TerminalPaneId) -> bool {
        let Some(tab_index) = self.tabs.iter().position(|tab| tab.root.contains(pane_id)) else {
            return false;
        };
        let tab_became_empty = self.tabs[tab_index].remove_pane(pane_id);
        if tab_became_empty {
            self.tabs.remove(tab_index);
            if self.tabs.is_empty() {
                self.active_tab = 0;
            } else if tab_index < self.active_tab {
                self.active_tab -= 1;
            } else if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
        }
        self.panes.retain(|pane| pane.id != pane_id);
        true
    }

    pub fn focus_pane(&mut self, pane_id: TerminalPaneId) -> bool {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return false;
        };
        if !tab.root.contains(pane_id) {
            return false;
        }
        if tab.focused == pane_id {
            return false;
        }
        tab.focused = pane_id;
        true
    }

    pub fn cycle_pane(&mut self, delta: isize) -> bool {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return false;
        };
        let panes = tab.pane_ids();
        let Some(current) = panes.iter().position(|pane_id| *pane_id == tab.focused) else {
            return false;
        };
        let next = wrapped_index(current, panes.len(), delta);
        if next == current {
            return false;
        }
        tab.focused = panes[next];
        true
    }

    pub fn cycle_tab(&mut self, delta: isize) -> bool {
        if self.tabs.is_empty() {
            return false;
        }
        let next = wrapped_index(self.active_tab, self.tabs.len(), delta);
        if next == self.active_tab {
            return false;
        }
        self.active_tab = next;
        true
    }

    pub fn focus_tab(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() || index == self.active_tab {
            return false;
        }
        self.active_tab = index;
        true
    }

    pub fn rename_pane(&mut self, pane_id: TerminalPaneId, label: String) -> bool {
        let Some(pane) = self.panes.iter_mut().find(|pane| pane.id == pane_id) else {
            return false;
        };
        pane.label = label;
        true
    }

    pub fn normalize_after_restore(&mut self) -> Result<(), String> {
        for pane in &mut self.panes {
            pane.command = pane.profile.default_command();
        }
        let minimum_next = self
            .panes
            .iter()
            .map(|pane| pane.id.serial())
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| "terminal pane serial cannot be incremented".to_string())?
            .max(first_pane_serial());
        self.next_pane_serial = self.next_pane_serial.max(minimum_next);
        self.validate()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.tabs.len() > MAX_TERMINAL_TABS {
            return Err(format!(
                "terminal workspace cannot exceed {MAX_TERMINAL_TABS} tabs"
            ));
        }
        if self.panes.len() > MAX_TERMINAL_PANES {
            return Err(format!(
                "terminal workspace cannot exceed {MAX_TERMINAL_PANES} panes"
            ));
        }
        if !(first_pane_serial()..=MAX_ALLOCATABLE_PANE_SERIAL + 1).contains(&self.next_pane_serial)
        {
            return Err(format!(
                "next terminal pane serial {} is outside the supported range",
                self.next_pane_serial
            ));
        }
        if self.tabs.is_empty() {
            if self.active_tab != 0 {
                return Err("empty terminal workspace must have active_tab 0".to_string());
            }
        } else if self.active_tab >= self.tabs.len() {
            return Err(format!(
                "active terminal tab {} is outside {} tabs",
                self.active_tab,
                self.tabs.len()
            ));
        }

        let mut known = HashSet::new();
        for pane in &self.panes {
            if !known.insert(pane.id) {
                return Err(format!("duplicate terminal pane definition {}", pane.id));
            }
            if !(first_pane_serial()..=MAX_ALLOCATABLE_PANE_SERIAL).contains(&pane.id.serial()) {
                return Err(format!(
                    "terminal pane serial {} is outside the supported range",
                    pane.id.serial()
                ));
            }
            validate_label(&pane.label)?;
            validate_command(&pane.command)?;
            if pane.command != pane.profile.default_command() {
                return Err(format!(
                    "terminal pane {} command does not match its profile",
                    pane.id
                ));
            }
        }

        let mut visited = HashSet::new();
        for tab in &self.tabs {
            tab.root.validate(&known, &mut visited, 1)?;
            if !tab.root.contains(tab.focused) {
                return Err(format!(
                    "terminal tab focuses pane {} outside its layout",
                    tab.focused
                ));
            }
        }
        if visited != known {
            let missing = known
                .difference(&visited)
                .next()
                .expect("different pane sets have at least one missing item");
            return Err(format!(
                "terminal pane definition {missing} is not present in any tab"
            ));
        }
        Ok(())
    }

    fn allocate_pane(
        &mut self,
        profile: TerminalProfile,
        cwd: PathBuf,
    ) -> Option<TerminalPaneDefinition> {
        allocate_pane(&mut self.next_pane_serial, profile, cwd)
    }
}

pub fn normalize_label(label: &str) -> Result<String, String> {
    let label = label.trim();
    validate_label(label)?;
    Ok(label.to_string())
}

fn validate_label(label: &str) -> Result<(), String> {
    if label.is_empty() {
        return Err("terminal pane label cannot be empty".to_string());
    }
    if label.chars().any(char::is_control) {
        return Err("terminal pane label cannot contain control characters".to_string());
    }
    if label.chars().count() > MAX_TERMINAL_LABEL_CHARS {
        return Err(format!(
            "terminal pane label cannot exceed {MAX_TERMINAL_LABEL_CHARS} characters"
        ));
    }
    Ok(())
}

fn validate_command(command: &TerminalCommandDefinition) -> Result<(), String> {
    match command.program.as_deref() {
        Some(program) if program.is_empty() || program.chars().any(char::is_control) => Err(
            "terminal command program cannot be empty or contain control characters".to_string(),
        ),
        None if !command.args.is_empty() => {
            Err("the platform default terminal shell cannot receive explicit arguments".to_string())
        }
        _ if command
            .args
            .iter()
            .any(|argument| argument.chars().any(|character| character == '\0')) =>
        {
            Err("terminal command arguments cannot contain NUL characters".to_string())
        }
        _ => Ok(()),
    }
}

fn allocate_pane(
    next_serial: &mut u64,
    profile: TerminalProfile,
    cwd: PathBuf,
) -> Option<TerminalPaneDefinition> {
    let id = TerminalPaneId::new((*next_serial).max(first_pane_serial()));
    if id.serial() > MAX_ALLOCATABLE_PANE_SERIAL {
        return None;
    }
    *next_serial = id.serial() + 1;
    Some(TerminalPaneDefinition {
        id,
        label: default_pane_label(profile, &cwd, id),
        profile,
        cwd,
        command: profile.default_command(),
    })
}

fn default_pane_label(profile: TerminalProfile, cwd: &Path, pane_id: TerminalPaneId) -> String {
    let directory = cwd
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty() && !name.chars().any(char::is_control));
    match directory {
        Some(directory) => format!(
            "{} {} · {directory}",
            profile.display_name(),
            pane_id.serial()
        ),
        None => format!("{} {}", profile.display_name(), pane_id.serial()),
    }
}

fn wrapped_index(current: usize, len: usize, delta: isize) -> usize {
    debug_assert!(len > 0);
    (current as isize + delta).rem_euclid(len as isize) as usize
}
