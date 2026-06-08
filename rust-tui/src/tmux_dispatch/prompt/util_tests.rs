use super::{split_literal_chunks, submit_delay_for};

#[test]
fn split_literal_chunks_preserves_text() {
    let text = "abcdefghijklmnopqrstuvwxyz";
    let chunks = split_literal_chunks(text, 5);
    assert_eq!(chunks.join(""), text);
    assert!(chunks.iter().all(|chunk| chunk.chars().count() <= 5));
}

#[test]
fn submit_delay_grows_for_longer_prompts() {
    let short = submit_delay_for("short prompt", false);
    let long = submit_delay_for(&"x".repeat(320), false);
    assert!(long > short);
}
