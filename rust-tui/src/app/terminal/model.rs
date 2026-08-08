mod workspace;

pub use workspace::{TerminalTab, TerminalWorkspace};

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;

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
