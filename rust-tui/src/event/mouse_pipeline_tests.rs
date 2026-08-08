use crossterm::event::MouseEventKind;

use super::*;

#[test]
fn wheel_routes_to_child_only_while_mouse_reporting_is_active() {
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

#[test]
fn shift_wheel_forces_pad_scrollback() {
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

#[test]
fn non_wheel_events_do_not_enter_scroll_routing() {
    let moved = mouse(MouseEventKind::Moved, KeyModifiers::NONE);
    assert_eq!(terminal_wheel_route(&moved, TerminalMode::default()), None);
}

fn mouse(kind: MouseEventKind, modifiers: KeyModifiers) -> MouseEvent {
    MouseEvent {
        kind,
        column: 4,
        row: 2,
        modifiers,
    }
}
