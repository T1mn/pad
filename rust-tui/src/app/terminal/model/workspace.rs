use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::*;

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
