use std::any::Any;
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};

thread_local! {
    static ISOLATED_DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Runs code under a panic boundary recognized by PAD's global panic hook.
///
/// The marker is thread-local and active while the panic hook runs, so a
/// caught worker panic is logged without tearing down the application's TUI.
pub(crate) fn catch_isolated_unwind<F, R>(operation: F) -> Result<R, Box<dyn Any + Send + 'static>>
where
    F: FnOnce() -> R,
{
    let _guard = IsolatedGuard::enter();
    catch_unwind(AssertUnwindSafe(operation))
}

pub(crate) fn is_isolated() -> bool {
    ISOLATED_DEPTH.with(|depth| depth.get() > 0)
}

struct IsolatedGuard;

impl IsolatedGuard {
    fn enter() -> Self {
        ISOLATED_DEPTH.with(|depth| {
            depth.set(
                depth
                    .get()
                    .checked_add(1)
                    .expect("isolated panic boundary depth overflowed"),
            );
        });
        Self
    }
}

impl Drop for IsolatedGuard {
    fn drop(&mut self) {
        ISOLATED_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

#[cfg(test)]
#[path = "panic_boundary_tests.rs"]
pub(crate) mod tests;
