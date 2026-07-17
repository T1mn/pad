use super::parse_reader;
use std::io::Cursor;

#[test]
fn parses_official_0_2_102_envelopes_and_skips_unknown_lines() {
    let input = concat!(
        "{not-json\n",
        r#"{"timestamp":1,"method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"hello"}}}}"#,
        "\n",
        r#"{"timestamp":2,"method":"_x.ai/session/update","params":{"sessionId":"s","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"hi "}}}}"#,
        "\n",
        r#"{"timestamp":3,"method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"there"},"future":true}}}"#,
        "\n",
        r#"{"timestamp":4,"method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"tool_call","title":"ignored"}}}"#,
        "\n",
    );

    let turns = parse_reader(Cursor::new(input)).unwrap();
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].question, "hello");
    assert_eq!(turns[0].answer.as_deref(), Some("hi there"));
}

#[test]
fn accepts_direct_update_shape_for_older_logs() {
    let input = concat!(
        r#"{"update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"q"}}}"#,
        "\n",
        r#"{"update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"a"}}}"#,
        "\n",
    );
    let turns = parse_reader(Cursor::new(input)).unwrap();
    assert_eq!(turns[0].question, "q");
    assert_eq!(turns[0].answer.as_deref(), Some("a"));
}
