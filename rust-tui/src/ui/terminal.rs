use crate::app::{
    App, TerminalInteractionState, TerminalLayoutNode, TerminalPaneId, TerminalPaneLifecycle,
    TerminalPaneView, TerminalSplitAxis,
};
use crate::terminal_runtime::TerminalPaneWidget;
use crate::theme::Theme;
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

pub const TERMINAL_TAB_BAR_HEIGHT: u16 = 1;

/// Geometry for one terminal pane.
///
/// `outer` includes the PAD-owned pane border and `inner` is the terminal
/// cell grid. Drawing, resize, and mouse hit testing share these placements
/// so those paths cannot disagree about split or border offsets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanePlacement {
    pub pane_id: TerminalPaneId,
    pub outer: Rect,
    pub inner: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TabPlacement {
    pub index: usize,
    pub rect: Rect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalPlacement {
    pub tab_bar: Rect,
    pub content: Rect,
    pub tabs: Vec<TabPlacement>,
    pub panes: Vec<PanePlacement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TabLabel {
    label: String,
    active: bool,
}

struct TabBarWidget<'a> {
    tabs: &'a [TabLabel],
    theme: &'a Theme,
}

impl Widget for TabBarWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let spans = self
            .tabs
            .iter()
            .enumerate()
            .flat_map(|(index, tab)| {
                let separator = (index > 0)
                    .then(|| Span::styled(" │ ", Style::default().fg(self.theme.border)));
                let style = if tab.active {
                    Style::default()
                        .fg(self.theme.highlight_fg)
                        .bg(self.theme.highlight_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(self.theme.comment)
                };
                separator.into_iter().chain(std::iter::once(Span::styled(
                    format!(" {} ", tab.label),
                    style,
                )))
            })
            .collect::<Vec<_>>();
        Paragraph::new(Line::from(spans))
            .style(Style::default().bg(self.theme.bg))
            .render(area, buffer);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PanePlaceholder<'a> {
    Opening,
    Closing,
    Error(&'a str),
    Exited { code: Option<i32>, signaled: bool },
}

struct PanePlaceholderWidget<'a> {
    label: &'a str,
    state: PanePlaceholder<'a>,
    focused: bool,
    theme: &'a Theme,
}

impl Widget for PanePlaceholderWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let block = Block::default()
            .title(format!(" {} ", self.label))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(self.theme.bg).fg(self.theme.fg))
            .border_style(Style::default().fg(if self.focused {
                self.theme.border_focused
            } else {
                self.theme.border
            }));
        let message = match self.state {
            PanePlaceholder::Opening => "Opening terminal…".to_string(),
            PanePlaceholder::Closing => "Closing terminal…".to_string(),
            PanePlaceholder::Error(error) => format!("Terminal failed: {error}"),
            PanePlaceholder::Exited {
                code: Some(code), ..
            } => format!("Terminal exited with code {code}"),
            PanePlaceholder::Exited { signaled: true, .. } => {
                "Terminal exited by signal".to_string()
            }
            PanePlaceholder::Exited { .. } => "Terminal exited".to_string(),
        };
        let message_color = match self.state {
            PanePlaceholder::Error(_) | PanePlaceholder::Exited { .. } => self.theme.warning,
            PanePlaceholder::Opening | PanePlaceholder::Closing => self.theme.comment,
        };
        Paragraph::new(message)
            .style(Style::default().bg(self.theme.bg).fg(message_color))
            .block(block)
            .render(area, buffer);
    }
}

/// Returns active-tab pane geometry in deterministic depth-first order.
pub fn placement(app: &App, area: Rect) -> TerminalPlacement {
    let mut placement = match app.terminal_workspace().active_tab() {
        Some(tab) => place_tree(&tab.root, area),
        // The initial PTY size is queried before pane 1 is created. Reserving
        // normal tab and border space here also keeps the old single-pane
        // viewport getter compatible during the transition.
        None => place_tree(&TerminalLayoutNode::pane(TerminalPaneId::new(1)), area),
    };
    placement.tabs = place_tabs(placement.tab_bar, &tab_labels(app));
    placement
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let placement = placement(app, area);
    let tabs = tab_labels(app);
    f.render_widget(
        TabBarWidget {
            tabs: &tabs,
            theme: &app.theme,
        },
        placement.tab_bar,
    );
    draw_command_layer(f, app, placement.tab_bar);

    let focused_pane = app.focused_terminal_pane_id();
    for pane in &placement.panes {
        let focused = app.terminal_is_focused() && focused_pane == Some(pane.pane_id);
        if let Some(view) = app.terminal_pane(pane.pane_id) {
            draw_pane(f, app, pane, view, focused);
        } else if placement.panes.len() == 1 {
            draw_legacy_pane(f, app, pane, focused);
        } else {
            f.render_widget(
                PanePlaceholderWidget {
                    label: "Terminal",
                    state: PanePlaceholder::Error("pane is missing from the workspace"),
                    focused,
                    theme: &app.theme,
                },
                pane.outer,
            );
        }
    }
}

fn draw_command_layer(f: &mut Frame, app: &App, area: Rect) {
    let text = match app.terminal_interaction() {
        TerminalInteractionState::Direct => return,
        TerminalInteractionState::Command => {
            " PAD TERM  1 Shell  2 Codex  3 Claude  4 GitHub  v/s split  h/j pane  [/] tab  r rename  x close  Esc cancel ".to_string()
        }
        TerminalInteractionState::Rename { buffer, .. } => {
            format!(" Rename pane: {buffer}▏  Enter save · Esc cancel ")
        }
    };
    f.render_widget(
        Paragraph::new(text).style(
            Style::default()
                .fg(app.theme.highlight_fg)
                .bg(app.theme.highlight_bg)
                .add_modifier(Modifier::BOLD),
        ),
        area,
    );
}

fn tab_labels(app: &App) -> Vec<TabLabel> {
    let workspace = app.terminal_workspace();
    if workspace.tabs.is_empty() {
        vec![TabLabel {
            label: legacy_label(app).to_string(),
            active: true,
        }]
    } else {
        workspace
            .tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| TabLabel {
                label: tab.label.clone().unwrap_or_else(|| {
                    tab.pane_ids()
                        .first()
                        .and_then(|pane_id| workspace.pane(*pane_id))
                        .map(|pane| pane.label.clone())
                        .unwrap_or_else(|| format!("Tab {}", index + 1))
                }),
                active: index == workspace.active_tab,
            })
            .collect()
    }
}

fn place_tabs(area: Rect, tabs: &[TabLabel]) -> Vec<TabPlacement> {
    let mut cursor = area.x;
    let right = area.right();
    tabs.iter()
        .enumerate()
        .map(|(index, tab)| {
            let separator_width = if index == 0 { 0 } else { 3 };
            let desired_width =
                separator_width + UnicodeWidthStr::width(tab.label.as_str()).saturating_add(2);
            let width = desired_width.min(usize::from(right.saturating_sub(cursor))) as u16;
            let rect = Rect::new(cursor, area.y, width, area.height);
            cursor = cursor.saturating_add(width).min(right);
            TabPlacement { index, rect }
        })
        .collect()
}

fn draw_pane(
    f: &mut Frame,
    app: &App,
    placement: &PanePlacement,
    pane: TerminalPaneView<'_>,
    focused: bool,
) {
    let state = if let Some(error) = pane.error() {
        Some(PanePlaceholder::Error(error))
    } else if let Some(exit) = pane.exit() {
        Some(PanePlaceholder::Exited {
            code: exit.code,
            signaled: exit.signaled,
        })
    } else {
        match pane.lifecycle() {
            TerminalPaneLifecycle::Opening => Some(PanePlaceholder::Opening),
            TerminalPaneLifecycle::Closing => Some(PanePlaceholder::Closing),
            TerminalPaneLifecycle::Exited => Some(PanePlaceholder::Exited {
                code: None,
                signaled: false,
            }),
            TerminalPaneLifecycle::Failed => {
                Some(PanePlaceholder::Error("terminal failed to open"))
            }
            TerminalPaneLifecycle::Running => None,
        }
    };

    if let Some(state) = state {
        f.render_widget(
            PanePlaceholderWidget {
                label: pane.label(),
                state,
                focused,
                theme: &app.theme,
            },
            placement.outer,
        );
    } else if let Some(frame) = pane.frame() {
        f.render_widget(
            TerminalPaneWidget::new(frame).focused(focused),
            placement.outer,
        );
    } else {
        f.render_widget(
            PanePlaceholderWidget {
                label: pane.label(),
                state: PanePlaceholder::Opening,
                focused,
                theme: &app.theme,
            },
            placement.outer,
        );
    }
}

fn draw_legacy_pane(f: &mut Frame, app: &App, pane: &PanePlacement, focused: bool) {
    let label = legacy_label(app);
    if let Some(error) = app.terminal_error() {
        f.render_widget(
            PanePlaceholderWidget {
                label,
                state: PanePlaceholder::Error(error),
                focused,
                theme: &app.theme,
            },
            pane.outer,
        );
    } else if let Some(exit) = app.terminal_exit() {
        f.render_widget(
            PanePlaceholderWidget {
                label,
                state: PanePlaceholder::Exited {
                    code: exit.code,
                    signaled: exit.signaled,
                },
                focused,
                theme: &app.theme,
            },
            pane.outer,
        );
    } else if let Some(frame) = app.terminal_frame() {
        f.render_widget(TerminalPaneWidget::new(frame).focused(focused), pane.outer);
    } else {
        f.render_widget(
            PanePlaceholderWidget {
                label,
                state: PanePlaceholder::Opening,
                focused,
                theme: &app.theme,
            },
            pane.outer,
        );
    }
}

fn legacy_label(app: &App) -> &str {
    app.terminal_frame()
        .map(|frame| frame.metadata.label.as_str())
        .unwrap_or("Terminal")
}

fn terminal_areas(area: Rect) -> (Rect, Rect) {
    let tab_height = area.height.min(TERMINAL_TAB_BAR_HEIGHT);
    let tab_bar = Rect::new(area.x, area.y, area.width, tab_height);
    let content = Rect::new(
        area.x,
        area.y.saturating_add(tab_height),
        area.width,
        area.height.saturating_sub(tab_height),
    );
    (tab_bar, content)
}

fn place_tree(root: &TerminalLayoutNode, area: Rect) -> TerminalPlacement {
    let (tab_bar, content) = terminal_areas(area);
    let mut panes = Vec::new();
    place_node(root, content, &mut panes);
    TerminalPlacement {
        tab_bar,
        content,
        tabs: Vec::new(),
        panes,
    }
}

fn place_node(node: &TerminalLayoutNode, area: Rect, panes: &mut Vec<PanePlacement>) {
    match node {
        TerminalLayoutNode::Pane { pane_id } => panes.push(PanePlacement {
            pane_id: *pane_id,
            outer: area,
            inner: area.inner(Margin::new(1, 1)),
        }),
        TerminalLayoutNode::Split {
            axis,
            ratio_per_mille,
            first,
            second,
        } => {
            let (first_area, second_area) = split_rect(area, *axis, *ratio_per_mille);
            place_node(first, first_area, panes);
            place_node(second, second_area, panes);
        }
    }
}

fn split_rect(area: Rect, axis: TerminalSplitAxis, ratio_per_mille: u16) -> (Rect, Rect) {
    let ratio = ratio_per_mille.clamp(1, 999);
    match axis {
        TerminalSplitAxis::Columns => {
            let first_width = ratio_size(area.width, ratio);
            let second_width = area.width.saturating_sub(first_width);
            (
                Rect::new(area.x, area.y, first_width, area.height),
                Rect::new(
                    area.x.saturating_add(first_width),
                    area.y,
                    second_width,
                    area.height,
                ),
            )
        }
        TerminalSplitAxis::Rows => {
            let first_height = ratio_size(area.height, ratio);
            let second_height = area.height.saturating_sub(first_height);
            (
                Rect::new(area.x, area.y, area.width, first_height),
                Rect::new(
                    area.x,
                    area.y.saturating_add(first_height),
                    area.width,
                    second_height,
                ),
            )
        }
    }
}

fn ratio_size(total: u16, ratio_per_mille: u16) -> u16 {
    (u32::from(total) * u32::from(ratio_per_mille) / 1000) as u16
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::*;

    #[test]
    fn single_pane_reserves_one_row_for_tabs() {
        let root = pane(1);
        let placement = place_tree(&root, Rect::new(3, 5, 11, 7));

        assert_eq!(placement.tab_bar, Rect::new(3, 5, 11, 1));
        assert_eq!(placement.content, Rect::new(3, 6, 11, 6));
        assert_eq!(
            placement.panes,
            vec![PanePlacement {
                pane_id: TerminalPaneId::new(1),
                outer: Rect::new(3, 6, 11, 6),
                inner: Rect::new(4, 7, 9, 4),
            }]
        );
    }

    #[test]
    fn recursive_splits_keep_depth_first_pane_order() {
        let root = split(
            TerminalSplitAxis::Columns,
            pane(1),
            split(TerminalSplitAxis::Rows, pane(2), pane(3)),
        );

        let placement = place_tree(&root, Rect::new(0, 0, 15, 10));

        assert_eq!(pane_ids(&placement), [1, 2, 3]);
        assert_eq!(placement.panes[0].outer, Rect::new(0, 1, 7, 9));
        assert_eq!(placement.panes[1].outer, Rect::new(7, 1, 8, 4));
        assert_eq!(placement.panes[2].outer, Rect::new(7, 5, 8, 5));
    }

    #[test]
    fn two_columns_give_the_odd_cell_to_the_second_pane() {
        let root = split(TerminalSplitAxis::Columns, pane(1), pane(2));

        let placement = place_tree(&root, Rect::new(0, 0, 9, 6));

        assert_eq!(placement.panes[0].outer, Rect::new(0, 1, 4, 5));
        assert_eq!(placement.panes[1].outer, Rect::new(4, 1, 5, 5));
    }

    #[test]
    fn four_panes_tile_odd_area_without_gaps_or_overlap() {
        let root = split(
            TerminalSplitAxis::Rows,
            split(TerminalSplitAxis::Columns, pane(1), pane(2)),
            split(TerminalSplitAxis::Columns, pane(3), pane(4)),
        );

        let placement = place_tree(&root, Rect::new(2, 4, 9, 8));

        assert_eq!(pane_ids(&placement), [1, 2, 3, 4]);
        assert_eq!(placement.panes[0].outer, Rect::new(2, 5, 4, 3));
        assert_eq!(placement.panes[1].outer, Rect::new(6, 5, 5, 3));
        assert_eq!(placement.panes[2].outer, Rect::new(2, 8, 4, 4));
        assert_eq!(placement.panes[3].outer, Rect::new(6, 8, 5, 4));
    }

    #[test]
    fn stored_split_ratio_is_applied_with_saturating_remainder() {
        let root = TerminalLayoutNode::Split {
            axis: TerminalSplitAxis::Columns,
            ratio_per_mille: 250,
            first: Box::new(pane(1)),
            second: Box::new(pane(2)),
        };

        let placement = place_tree(&root, Rect::new(0, 0, 11, 5));

        assert_eq!(placement.panes[0].outer.width, 2);
        assert_eq!(placement.panes[1].outer.width, 9);
    }

    #[test]
    fn tiny_rectangles_saturate_instead_of_underflowing() {
        let root = split(TerminalSplitAxis::Columns, pane(1), pane(2));

        let placement = place_tree(&root, Rect::new(u16::MAX - 1, u16::MAX - 1, 1, 1));

        assert_eq!(placement.tab_bar.height, 1);
        assert_eq!(placement.content.height, 0);
        assert_eq!(placement.panes.len(), 2);
        assert!(placement.panes.iter().all(|pane| pane.outer.height == 0));
        assert!(placement.panes.iter().all(|pane| pane.inner.is_empty()));
    }

    #[test]
    fn tab_bar_marks_only_the_active_tab() {
        let theme = Theme::by_name("default");
        let tabs = [
            TabLabel {
                label: "Shell".to_string(),
                active: true,
            },
            TabLabel {
                label: "Codex".to_string(),
                active: false,
            },
        ];
        let area = Rect::new(0, 0, 24, 1);
        let mut buffer = Buffer::empty(area);

        TabBarWidget {
            tabs: &tabs,
            theme: &theme,
        }
        .render(area, &mut buffer);

        let shell_cell = buffer.cell((1, 0)).expect("shell tab cell");
        let codex_cell = buffer.cell((12, 0)).expect("codex tab cell");
        assert_eq!(shell_cell.bg, theme.highlight_bg);
        assert_ne!(codex_cell.bg, theme.highlight_bg);
    }

    #[test]
    fn tab_hit_rects_match_unicode_width_and_visible_clipping() {
        let tabs = [
            TabLabel {
                label: "A".to_string(),
                active: true,
            },
            TabLabel {
                label: "界".to_string(),
                active: false,
            },
            TabLabel {
                label: "hidden".to_string(),
                active: false,
            },
        ];

        let placements = place_tabs(Rect::new(4, 7, 8, 1), &tabs);

        assert_eq!(placements[0].rect, Rect::new(4, 7, 3, 1));
        // Separator (3) + padded wide label (4) is clipped to 5 remaining cells.
        assert_eq!(placements[1].rect, Rect::new(7, 7, 5, 1));
        assert_eq!(placements[2].rect, Rect::new(12, 7, 0, 1));
    }

    #[test]
    fn placeholder_uses_focus_border_and_warning_text() {
        let theme = Theme::by_name("default");
        let area = Rect::new(0, 0, 28, 4);
        let mut buffer = Buffer::empty(area);

        PanePlaceholderWidget {
            label: "Codex",
            state: PanePlaceholder::Error("pty unavailable"),
            focused: true,
            theme: &theme,
        }
        .render(area, &mut buffer);

        assert_eq!(buffer[(0, 0)].fg, theme.border_focused);
        assert_eq!(buffer[(1, 1)].fg, theme.warning);
        assert_ne!(buffer[(1, 1)].fg, Color::Reset);
        assert!(row_text(&buffer, 1).contains("Terminal failed"));
    }

    fn pane(serial: u64) -> TerminalLayoutNode {
        TerminalLayoutNode::pane(TerminalPaneId::new(serial))
    }

    fn split(
        axis: TerminalSplitAxis,
        first: TerminalLayoutNode,
        second: TerminalLayoutNode,
    ) -> TerminalLayoutNode {
        TerminalLayoutNode::Split {
            axis,
            ratio_per_mille: 500,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    fn pane_ids<const N: usize>(placement: &TerminalPlacement) -> [u64; N] {
        placement
            .panes
            .iter()
            .map(|pane| pane.pane_id.serial())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }

    fn row_text(buffer: &Buffer, row: u16) -> String {
        (buffer.area.x..buffer.area.right())
            .filter_map(|column| buffer.cell((column, row)))
            .map(|cell| cell.symbol())
            .collect()
    }
}
