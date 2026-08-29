use ratatui::style::Color;

use super::*;

pub(crate) fn single_pane_reserves_one_row_for_tabs() {
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

pub(crate) fn recursive_splits_keep_depth_first_pane_order() {
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

pub(crate) fn two_columns_give_the_odd_cell_to_the_second_pane() {
    let root = split(TerminalSplitAxis::Columns, pane(1), pane(2));

    let placement = place_tree(&root, Rect::new(0, 0, 9, 6));

    assert_eq!(placement.panes[0].outer, Rect::new(0, 1, 4, 5));
    assert_eq!(placement.panes[1].outer, Rect::new(4, 1, 5, 5));
}

pub(crate) fn four_panes_tile_odd_area_without_gaps_or_overlap() {
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

pub(crate) fn stored_split_ratio_is_applied_with_saturating_remainder() {
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

pub(crate) fn tiny_rectangles_saturate_instead_of_underflowing() {
    let root = split(TerminalSplitAxis::Columns, pane(1), pane(2));

    let placement = place_tree(&root, Rect::new(u16::MAX - 1, u16::MAX - 1, 1, 1));

    assert_eq!(placement.tab_bar.height, 1);
    assert_eq!(placement.content.height, 0);
    assert_eq!(placement.panes.len(), 2);
    assert!(placement.panes.iter().all(|pane| pane.outer.height == 0));
    assert!(placement.panes.iter().all(|pane| pane.inner.is_empty()));
}

pub(crate) fn tab_bar_marks_only_the_active_tab() {
    let theme = Theme::by_name("default");
    let tabs = [
        TabLabel {
            label: "Shell".to_string(),
            active: true,
            closable: true,
        },
        TabLabel {
            label: "Codex".to_string(),
            active: false,
            closable: true,
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
    let codex_cell = buffer.cell((14, 0)).expect("codex tab cell");
    assert_eq!(shell_cell.bg, theme.highlight_bg);
    assert_ne!(codex_cell.bg, theme.highlight_bg);
}

pub(crate) fn tab_hit_rects_match_unicode_width_and_visible_clipping() {
    let tabs = [
        TabLabel {
            label: "A".to_string(),
            active: true,
            closable: true,
        },
        TabLabel {
            label: "界".to_string(),
            active: false,
            closable: true,
        },
        TabLabel {
            label: "hidden".to_string(),
            active: false,
            closable: true,
        },
    ];

    let placements = place_tabs(Rect::new(4, 7, 12, 1), &tabs);

    assert_eq!(placements[0].rect, Rect::new(4, 7, 5, 1));
    assert_eq!(placements[0].close, Some(Rect::new(7, 7, 2, 1)));
    // Separator (3) + padded wide label (6) is clipped to 7 remaining cells.
    assert_eq!(placements[1].rect, Rect::new(9, 7, 7, 1));
    assert_eq!(placements[1].close, None);
    assert_eq!(placements[2].rect, Rect::new(16, 7, 0, 1));
}

pub(crate) fn placeholder_uses_focus_border_and_warning_text() {
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
