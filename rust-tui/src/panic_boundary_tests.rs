use super::*;

#[test]
fn marker_is_scoped_and_supports_nesting() {
    assert!(!is_isolated());
    catch_isolated_unwind(|| {
        assert!(is_isolated());
        catch_isolated_unwind(|| assert!(is_isolated())).unwrap();
        assert!(is_isolated());
    })
    .unwrap();
    assert!(!is_isolated());
}

#[test]
fn marker_remains_active_while_panic_hook_runs() {
    struct HookProbe;

    impl Drop for HookProbe {
        fn drop(&mut self) {
            assert!(is_isolated());
        }
    }

    let result = catch_isolated_unwind(|| {
        let _probe = HookProbe;
        panic!("contained panic");
    });
    assert!(result.is_err());
    assert!(!is_isolated());
}
