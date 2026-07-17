use super::select_cwd_candidate;

#[test]
fn live_cwd_candidate_requires_one_unambiguous_session() {
    assert_eq!(select_cwd_candidate(vec![1], true), Some(1));
    assert_eq!(select_cwd_candidate(vec![1, 2], true), None);
    assert_eq!(select_cwd_candidate(vec![1, 2], false), Some(1));
}
