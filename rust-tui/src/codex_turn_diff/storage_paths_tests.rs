use super::safe_name;

#[test]
fn safe_name_collapses_trims_and_limits() {
    assert_eq!(safe_name("  turn:/abc  "), "turn_abc");
    assert_eq!(safe_name("a///b"), "a_b");
    assert_eq!(safe_name("!@#"), "");
    assert_eq!(safe_name(&"a".repeat(120)).len(), 96);
}
