mod bench {
    use super::super::parse_transcript;
    use crate::preview_source::SessionReadMode;
    use std::fs;
    use std::path::Path;
    use std::time::Instant;

    #[test]
    #[ignore]
    fn bench_parse_transcripts_from_env() {
        let raw_paths = std::env::var("PAD_CODEX_BENCH_PATHS")
            .expect("set PAD_CODEX_BENCH_PATHS to a ';'-separated list of transcript paths");
        let iterations = std::env::var("PAD_CODEX_BENCH_ITERATIONS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(5);

        for raw_path in raw_paths
            .split(';')
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            let path = Path::new(raw_path);
            let metadata = fs::metadata(path)
                .unwrap_or_else(|err| panic!("failed to stat {}: {}", path.display(), err));
            let mut elapsed_ms = Vec::with_capacity(iterations);
            let mut turn_count = None;

            for _ in 0..iterations {
                let started_at = Instant::now();
                let turns = parse_transcript(path, SessionReadMode::FullBackfill)
                    .unwrap_or_else(|err| panic!("failed to parse {}: {}", path.display(), err));
                elapsed_ms.push(started_at.elapsed().as_secs_f64() * 1000.0);
                turn_count = Some(turns.len());
            }

            let total_ms: f64 = elapsed_ms.iter().sum();
            let avg_ms = total_ms / elapsed_ms.len() as f64;
            let min_ms = elapsed_ms.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_ms = elapsed_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

            println!(
                "bench.codex_parse path={} bytes={} turns={} iterations={} runs_ms={:?} avg_ms={:.3} min_ms={:.3} max_ms={:.3}",
                path.display(),
                metadata.len(),
                turn_count.unwrap_or(0),
                iterations,
                elapsed_ms,
                avg_ms,
                min_ms,
                max_ms
            );
        }
    }
}
pub(crate) mod compressed {
    use super::super::parse_transcript;
    use super::support::temp_jsonl_path;
    use crate::preview_source::SessionReadMode;
    use std::fs;
    use std::io::Write;

    pub(crate) fn parse_codex_transcript_reads_compressed_sibling_with_future_fields() {
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
}
pub(crate) mod normalize {
    use super::super::normalize_codex_user_text;

    pub(crate) fn normalize_codex_user_text_handles_image_only_message() {
        let text = "<image name=[Image #1]>\n</image>\n[Image #1]";
        assert_eq!(normalize_codex_user_text(text, Some(1)), "[Image x1]");
    }

    pub(crate) fn normalize_codex_user_text_keeps_image_body_lines() {
        let text = "<image name=[Image #1]>\n</image>\n[Image #1]\n first line \n\nsecond line";
        assert_eq!(
            normalize_codex_user_text(text, Some(1)),
            "[Image x1] first line\nsecond line"
        );
    }

    pub(crate) fn normalize_codex_user_text_does_not_touch_plain_text_without_images() {
        let text = "literal [Image #1] text";
        assert_eq!(normalize_codex_user_text(text, None), text);
    }

    pub(crate) fn normalize_codex_user_text_filters_environment_context_block() {
        let text = "<environment_context>\n  <cwd>/tmp/demo</cwd>\n</environment_context>";
        assert_eq!(normalize_codex_user_text(text, None), "");
    }

    pub(crate) fn normalize_codex_user_text_strips_embedded_environment_context_block() {
        let text = "请分析一下\n<environment_context>\n  <cwd>/tmp/demo</cwd>\n</environment_context>\n这段结构";
        assert_eq!(
            normalize_codex_user_text(text, None),
            "请分析一下\n\n这段结构"
        );
    }

    pub(crate) fn normalize_codex_user_text_filters_turn_aborted_marker() {
        let text = "<turn_aborted>\ninterrupted\n</turn_aborted>";
        assert_eq!(normalize_codex_user_text(text, None), "");
    }

    pub(crate) fn normalize_codex_user_text_summarizes_user_shell_command() {
        let text = "<user_shell_command>\n<command>\necho hi\n</command>\n<result>\nExit code: 0\n</result>\n</user_shell_command>";
        assert_eq!(normalize_codex_user_text(text, None), "[shell] echo hi");
    }
}
mod support {
    pub(super) fn temp_jsonl_path(name: &str) -> std::path::PathBuf {
        crate::test_support::temp_path("pad-preview-jsonl", name)
    }
}
pub(crate) mod transcript {
    use super::super::parse_transcript;
    use super::support::temp_jsonl_path;
    use crate::preview_source::SessionReadMode;
    use std::fs;

    pub(crate) fn parse_codex_transcript_extracts_recent_messages() {
        let path = temp_jsonl_path("codex");
        fs::write(
            &path,
            concat!(
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"developer\",\"content\":[{\"type\":\"input_text\",\"text\":\"skip\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"hello\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"world\"}]}}\n"
            ),
        )
        .unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].question, "hello");
        assert_eq!(turns[0].answer.as_deref(), Some("world"));
    }

    pub(crate) fn parse_codex_transcript_backfills_beyond_six_turns() {
        let path = temp_jsonl_path("codex-history");
        let mut content = String::new();
        for idx in 0..8 {
            content.push_str(&format!(
                "{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"q{idx}\"}}]}}}}\n"
            ));
            content.push_str(&format!(
                "{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{{\"type\":\"output_text\",\"text\":\"a{idx}\"}}]}}}}\n"
            ));
        }
        fs::write(&path, content).unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 8);
        assert_eq!(turns[0].question, "q7");
        assert_eq!(turns[7].question, "q0");
    }

    pub(crate) fn parse_codex_transcript_keeps_latest_real_user_turns() {
        let path = temp_jsonl_path("codex-history-limit");
        let mut content = String::new();
        for idx in 0..60 {
            content.push_str(&format!(
                "{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"q{idx}\"}}]}}}}\n"
            ));
            content.push_str(&format!(
                "{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{{\"type\":\"output_text\",\"text\":\"a{idx}\"}}]}}}}\n"
            ));
        }
        for _ in 0..20 {
            content.push_str(
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"<environment_context>\\n  <cwd>/tmp/demo</cwd>\\n</environment_context>\"}]}}\n",
            );
        }
        fs::write(&path, content).unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(
            turns.len(),
            crate::session_cache::SESSION_HISTORY_TURN_LIMIT
        );
        assert_eq!(turns[0].question, "q59");
        assert_eq!(turns[49].question, "q10");
    }

    pub(crate) fn parse_codex_transcript_includes_subagent_events_in_main_turn() {
        let path = temp_jsonl_path("codex-subagent");
        fs::write(
            &path,
            concat!(
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"run 2 subagents\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"spawn_agent\",\"arguments\":\"{\\\"agent_type\\\":\\\"explorer\\\",\\\"task_name\\\":\\\"audit_event_rs\\\"}\"}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"<subagent_notification>\\n{\\\"agent_path\\\":\\\"/root/audit_event_rs\\\",\\\"status\\\":{\\\"completed\\\":\\\"`src/event.rs` is structurally overloaded and should be split into layers.\\\"}}\\n</subagent_notification>\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"merged result\"}]}}\n"
            ),
        )
        .unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 1);
        let answer = turns[0].answer.as_deref().unwrap_or("");
        assert_eq!(turns[0].question, "run 2 subagents");
        assert!(answer.contains("[subagent/start][explorer] audit_event_rs"));
        assert!(answer.contains("[subagent/completed] audit_event_rs"));
        assert!(answer.contains("merged result"));
        assert!(!answer.contains("<subagent_notification>"));
    }

    pub(crate) fn parse_codex_transcript_normalizes_single_image_user_message() {
        let path = temp_jsonl_path("codex-image-single");
        fs::write(
            &path,
            concat!(
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[",
                "{\"type\":\"input_text\",\"text\":\"<image name=[Image #1]>\"},",
                "{\"type\":\"input_image\",\"image_url\":\"file:///tmp/1.png\"},",
                "{\"type\":\"input_text\",\"text\":\"</image>\"},",
                "{\"type\":\"input_text\",\"text\":\"[Image #1] 为什么 settings 底部有黑边？\"}",
                "]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"看一下\"}]}}\n"
            ),
        )
        .unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns[0].question, "[Image x1] 为什么 settings 底部有黑边？");
    }

    pub(crate) fn parse_codex_transcript_normalizes_multiple_image_user_message() {
        let path = temp_jsonl_path("codex-image-multi");
        fs::write(
            &path,
            concat!(
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[",
                "{\"type\":\"input_text\",\"text\":\"<image name=[Image #1]>\"},",
                "{\"type\":\"input_image\",\"image_url\":\"file:///tmp/1.png\"},",
                "{\"type\":\"input_text\",\"text\":\"</image>\"},",
                "{\"type\":\"input_text\",\"text\":\"<image name=[Image #2]>\"},",
                "{\"type\":\"input_image\",\"image_url\":\"file:///tmp/2.png\"},",
                "{\"type\":\"input_text\",\"text\":\"</image>\"},",
                "{\"type\":\"input_text\",\"text\":\"[Image #1] 左侧不对，[Image #2] 右侧也不对\"}",
                "]}}\n"
            ),
        )
        .unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns[0].question, "[Image x2] 左侧不对， 右侧也不对");
    }

    pub(crate) fn parse_codex_transcript_skips_context_only_user_messages() {
        let path = temp_jsonl_path("codex-context-filter");
        fs::write(
            &path,
            concat!(
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"<environment_context>\\n  <cwd>/tmp/demo</cwd>\\n</environment_context>\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"real question\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"<turn_aborted>\\ninterrupted\\n</turn_aborted>\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"real answer\"}]}}\n"
            ),
        )
        .unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].question, "real question");
        assert_eq!(turns[0].answer.as_deref(), Some("real answer"));
    }
}

pub(crate) mod subagent {
    use super::super::subagent::extract_subagent_notification_summary;

    pub(crate) fn subagent_summary_compacts_whitespace_without_losing_detail() {
        let text = concat!(
            "<subagent_notification>\n",
            r#"{"agent_path":"/tmp/audit","status":{"completed":"  first   line\nsecond line"}}"#,
            "\n</subagent_notification>"
        );

        let summary = extract_subagent_notification_summary(text).unwrap();
        assert_eq!(summary, "[subagent/completed] audit\nfirst line");
    }
}

pub(crate) mod tail {
    use super::super::tail::{grow_tail_bytes, initial_tail_bytes, read_tail_lines};
    use std::fs;

    fn temp_path(name: &str) -> std::path::PathBuf {
        crate::test_support::temp_path("pad-codex-tail", name).with_extension("jsonl")
    }

    pub(crate) fn tail_window_helpers_clamp_and_grow() {
        assert_eq!(initial_tail_bytes(0), 1);
        assert_eq!(initial_tail_bytes(10), 10);
        assert_eq!(initial_tail_bytes(512), 256);
        assert_eq!(grow_tail_bytes(256, 400), 400);
    }

    pub(crate) fn tail_reader_keeps_whole_file_when_short() {
        let path = temp_path("short");
        fs::write(&path, "one\ntwo\n").unwrap();

        let lines = read_tail_lines(&path, 8, 8).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(lines, vec!["one".to_string(), "two".to_string()]);
    }

    pub(crate) fn tail_reader_drops_partial_first_line() {
        let path = temp_path("partial");
        fs::write(&path, "first\nsecond\nthird\n").unwrap();

        let lines = read_tail_lines(&path, 19, 13).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(lines, vec!["second".to_string(), "third".to_string()]);
    }
}
