//! Domain-level harness entries for the existing unit-test case bodies.

macro_rules! run_cases {
    ($($case:path),+ $(,)?) => {{
        let mut failures = Vec::new();
        $(
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $case())).is_err() {
                failures.push(stringify!($case));
            }
        )+
        assert!(
            failures.is_empty(),
            "{} case(s) failed:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }};
}

#[path = "test_suites_app.rs"]
mod app;
#[path = "test_suites_core.rs"]
mod core;
#[path = "test_suites_integrations.rs"]
mod integrations;
#[path = "test_suites_terminal.rs"]
mod terminal;
#[path = "test_suites_ui_event.rs"]
mod ui_event;
