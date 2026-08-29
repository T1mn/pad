use crossterm::event::MouseEventKind;

use super::*;

pub(crate) fn wheel_routes_to_child_only_while_mouse_reporting_is_active() {
    let wheel = mouse(MouseEventKind::ScrollUp, KeyModifiers::NONE);
    assert_eq!(
        terminal_wheel_route(
            &wheel,
            TerminalMode {
                mouse_reporting: true,
                sgr_mouse: true,
                ..TerminalMode::default()
            }
        ),
        Some(TerminalWheelRoute::Child)
    );
    assert_eq!(
        terminal_wheel_route(&wheel, TerminalMode::default()),
        Some(TerminalWheelRoute::Scroll(TerminalScroll::Lines(3)))
    );
}

pub(crate) fn shift_wheel_forces_pad_scrollback() {
    let wheel = mouse(MouseEventKind::ScrollDown, KeyModifiers::SHIFT);
    assert_eq!(
        terminal_wheel_route(
            &wheel,
            TerminalMode {
                mouse_reporting: true,
                sgr_mouse: true,
                ..TerminalMode::default()
            }
        ),
        Some(TerminalWheelRoute::Scroll(TerminalScroll::Lines(-3)))
    );
}

pub(crate) fn non_wheel_events_do_not_enter_scroll_routing() {
    let moved = mouse(MouseEventKind::Moved, KeyModifiers::NONE);
    assert_eq!(terminal_wheel_route(&moved, TerminalMode::default()), None);

    assert_eq!(
        crate::event::mouse::mouse_horizontal_scroll_delta(&MouseEventKind::ScrollLeft),
        Some(-1)
    );
    assert_eq!(
        crate::event::mouse::mouse_horizontal_scroll_delta(&MouseEventKind::ScrollRight),
        Some(1)
    );
    tab_pointer_switches_and_closes_tabs();

    divider_drag_resizes_and_persists_sidebar();
}

fn tab_pointer_switches_and_closes_tabs() {
    let placement = crate::ui::terminal::TerminalPlacement {
        tab_bar: ratatui::layout::Rect::new(0, 0, 24, 1),
        content: ratatui::layout::Rect::new(0, 1, 24, 9),
        tabs: vec![crate::ui::terminal::TabPlacement {
            index: 2,
            rect: ratatui::layout::Rect::new(3, 0, 10, 1),
            close: Some(ratatui::layout::Rect::new(11, 0, 2, 1)),
        }],
        panes: Vec::new(),
    };

    assert_eq!(
        terminal_tab_pointer_action(
            &placement,
            pointer(MouseEventKind::Down(MouseButton::Left), 5, 0)
        ),
        Some(TerminalTabPointerAction::Focus(2))
    );
    assert_eq!(
        terminal_tab_pointer_action(
            &placement,
            pointer(MouseEventKind::Drag(MouseButton::Left), 8, 0)
        ),
        Some(TerminalTabPointerAction::Focus(2))
    );
    assert_eq!(
        terminal_tab_pointer_action(
            &placement,
            pointer(MouseEventKind::Down(MouseButton::Left), 11, 0)
        ),
        Some(TerminalTabPointerAction::Close(2))
    );
    assert_eq!(
        terminal_tab_pointer_action(
            &placement,
            pointer(MouseEventKind::Down(MouseButton::Middle), 5, 0)
        ),
        Some(TerminalTabPointerAction::Close(2))
    );
}

fn divider_drag_resizes_and_persists_sidebar() {
    crate::test_support::with_temp_home("pad-mouse-pipeline", "sidebar-divider", |_| {
        let mut app = App::new();
        app.config.display.agent_panel_width = Some(20);
        let area = ratatui::layout::Rect::new(0, 0, 100, 30);

        assert!(handle_sidebar_resize_mouse(
            &mut app,
            area,
            pointer(MouseEventKind::Down(MouseButton::Left), 20, 2)
        ));
        assert!(app.sidebar.panel_resize_dragging);
        assert!(handle_sidebar_resize_mouse(
            &mut app,
            area,
            pointer(MouseEventKind::Drag(MouseButton::Left), 31, 2)
        ));
        assert_eq!(app.config.display.agent_panel_width, Some(31));
        assert!(handle_sidebar_resize_mouse(
            &mut app,
            area,
            pointer(MouseEventKind::Up(MouseButton::Left), 31, 2)
        ));
        assert!(!app.sidebar.panel_resize_dragging);
        assert_eq!(
            crate::theme::Config::load().display.agent_panel_width,
            Some(31)
        );
    });
}

fn mouse(kind: MouseEventKind, modifiers: KeyModifiers) -> MouseEvent {
    MouseEvent {
        kind,
        column: 4,
        row: 2,
        modifiers,
    }
}

fn pointer(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}
