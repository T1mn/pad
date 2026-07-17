use super::super::parse_transcript;
use super::support::temp_jsonl_path;
use crate::preview_source::SessionReadMode;
use std::fs;
use std::io::Write;

#[test]
fn parse_codex_transcript_reads_compressed_sibling_with_future_fields() {
    let canonical_path = temp_jsonl_path("codex-compressed").with_extension("jsonl");
    let compressed_path = canonical_path.with_extension("jsonl.zst");
    let file = fs::File::create(&compressed_path).unwrap();
    let mut encoder = zstd::stream::write::Encoder::new(file, 1).unwrap();
    encoder
        .write_all(
            concat!(
                "{\"type\":\"future_event\",\"payload\":{\"type\":\"future_payload\"}}\n",
                "{\"type\":\"response_item\",\"future_top_level\":true,\"payload\":{\"type\":\"message\",\"role\":\"user\",\"future_payload\":{\"enabled\":true},\"content\":[{\"type\":\"input_text\",\"text\":\"hello compressed\",\"future_content\":1}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"world compressed\"}]}}\n"
            )
            .as_bytes(),
        )
        .unwrap();
    encoder.finish().unwrap();

    let turns = parse_transcript(&canonical_path, SessionReadMode::FullBackfill).unwrap();
    fs::remove_file(&compressed_path).ok();

    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].question, "hello compressed");
    assert_eq!(turns[0].answer.as_deref(), Some("world compressed"));
}
