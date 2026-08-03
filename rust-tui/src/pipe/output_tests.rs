use super::*;

fn parsed(line: &[u8]) -> ParsedOutput {
    parse_output_notification(line)
        .expect("valid output notification")
        .expect("recognized output notification")
}

#[test]
fn output_decodes_octal_and_preserves_direct_utf8() {
    let output = parsed(r"%output %12 hello 界́\134\033[31m\015\012".as_bytes());

    assert_eq!(output.pane_id, "%12");
    assert_eq!(output.bytes, "hello 界́\\\x1b[31m\r\n".as_bytes());
}

#[test]
fn output_preserves_spaces_colons_and_empty_payloads() {
    let cases: &[(&[u8], &str, &[u8])] = &[
        (b"%output %0 ", "%0", b""),
        (b"%output %1  ", "%1", b" "),
        (b"%output %2 :", "%2", b":"),
        (b"%output %3 a:b c", "%3", b"a:b c"),
    ];

    for (line, pane_id, bytes) in cases {
        let output = parsed(line);
        assert_eq!(&output.pane_id, pane_id, "line={line:?}");
        assert_eq!(&output.bytes, bytes, "line={line:?}");
    }
}

#[test]
fn extended_output_ignores_future_fields_until_single_colon_token() {
    let cases: &[(&[u8], &str, &[u8])] = &[
        (b"%extended-output %1 0 : ", "%1", b""),
        (b"%extended-output %2 19 : value", "%2", b"value"),
        (
            b"%extended-output %3 42 future fields : a:b c",
            "%3",
            b"a:b c",
        ),
        (
            b"%extended-output %4 7 colon:inside ignored :  leading",
            "%4",
            b" leading",
        ),
        (
            br"%extended-output %0005 999999999999999999999999 : \000\377",
            "%0005",
            b"\x00\xff",
        ),
    ];

    for (line, pane_id, bytes) in cases {
        let output = parsed(line);
        assert_eq!(&output.pane_id, pane_id, "line={line:?}");
        assert_eq!(&output.bytes, bytes, "line={line:?}");
    }
}

#[test]
fn unknown_notifications_and_non_notifications_are_not_claimed() {
    let cases: &[&[u8]] = &[
        b"",
        b"plain command output",
        b" %output %1 data",
        b"%message %output %1 data",
        b"%output-extra %1 data",
        b"%extended-output-extra %1 0 : data",
        b"%future-notification arbitrary fields",
    ];

    for line in cases {
        assert_eq!(parse_output_notification(line), Ok(None), "line={line:?}");
    }
}

#[test]
fn recognized_notifications_reject_malformed_protocol_fields() {
    let cases: &[(&[u8], OutputParseError)] = &[
        (
            b"%output",
            OutputParseError::ExpectedSpaceAfterNotification { offset: 7 },
        ),
        (
            b"%output\t%1 data",
            OutputParseError::ExpectedSpaceAfterNotification { offset: 7 },
        ),
        (b"%output ", OutputParseError::MissingPaneId { offset: 8 }),
        (
            b"%output pane data",
            OutputParseError::InvalidPaneId { offset: 8 },
        ),
        (
            b"%output % data",
            OutputParseError::InvalidPaneId { offset: 8 },
        ),
        (
            b"%output %-1 data",
            OutputParseError::InvalidPaneId { offset: 8 },
        ),
        (
            b"%output %1",
            OutputParseError::MissingSpaceAfterPaneId { offset: 10 },
        ),
        (
            b"%extended-output %1 ",
            OutputParseError::MissingAge { offset: 20 },
        ),
        (
            b"%extended-output %1 -1 : data",
            OutputParseError::InvalidAge { offset: 20 },
        ),
        (
            b"%extended-output %1 age : data",
            OutputParseError::InvalidAge { offset: 20 },
        ),
        (
            b"%extended-output %1 1",
            OutputParseError::MissingSpaceAfterAge { offset: 21 },
        ),
        (
            b"%extended-output %1 1 ",
            OutputParseError::MissingExtendedDelimiter { offset: 22 },
        ),
        (
            b"%extended-output %1 1 future",
            OutputParseError::MissingExtendedDelimiter { offset: 28 },
        ),
        (
            b"%extended-output %1 1 :",
            OutputParseError::MissingSpaceAfterExtendedDelimiter { offset: 23 },
        ),
        (
            b"%extended-output %1 1  : data",
            OutputParseError::EmptyExtendedArgument { offset: 22 },
        ),
    ];

    for (line, expected) in cases {
        assert_eq!(
            parse_output_notification(line),
            Err(expected.clone()),
            "line={line:?}"
        );
    }
}

#[test]
fn malformed_octal_escapes_report_the_exact_failure() {
    let cases: &[(&[u8], OutputParseError)] = &[
        (
            b"%output %1 \\",
            OutputParseError::IncompleteOctalEscape { offset: 11 },
        ),
        (
            b"%output %1 \\0",
            OutputParseError::IncompleteOctalEscape { offset: 11 },
        ),
        (
            b"%output %1 \\00",
            OutputParseError::IncompleteOctalEscape { offset: 11 },
        ),
        (
            b"%output %1 \\800",
            OutputParseError::InvalidOctalDigit { offset: 12 },
        ),
        (
            b"%output %1 \\080",
            OutputParseError::InvalidOctalDigit { offset: 13 },
        ),
        (
            b"%output %1 \\008",
            OutputParseError::InvalidOctalDigit { offset: 14 },
        ),
        (
            b"%output %1 \\400",
            OutputParseError::OctalEscapeOutOfRange { offset: 11 },
        ),
        (
            b"%extended-output %1 0 : ok\\12",
            OutputParseError::IncompleteOctalEscape { offset: 26 },
        ),
    ];

    for (line, expected) in cases {
        assert_eq!(
            parse_output_notification(line),
            Err(expected.clone()),
            "line={line:?}"
        );
    }
}

#[test]
fn unescaped_control_bytes_are_rejected_while_arbitrary_high_bytes_pass() {
    let cases = vec![
        (b"%output %1 raw\nline".to_vec(), 14),
        (b"%output %1 raw\rline".to_vec(), 14),
        (b"%output %1 raw\tline".to_vec(), 14),
    ];

    for (line, offset) in cases {
        let error = parse_output_notification(&line).unwrap_err();
        match error {
            OutputParseError::NonPrintableLiteral { offset: actual, .. } => {
                assert_eq!(actual, offset, "line={line:?}");
            }
            other => panic!("unexpected error for {line:?}: {other:?}"),
        }
    }

    assert_eq!(parsed(b"%output %1 \xff").bytes, b"\xff");
    assert_eq!(parsed(b"%output %1 \xc3(").bytes, b"\xc3(");
    assert_eq!(parsed(b"%output %1 \x7f").bytes, b"\x7f");
}

#[test]
fn every_byte_round_trips_through_a_three_digit_octal_escape() {
    for expected in u8::MIN..=u8::MAX {
        let line = format!("%output %9 \\{expected:03o}");
        let output = parsed(line.as_bytes());
        assert_eq!(output.bytes, [expected], "escape={line:?}");
    }
}

#[test]
fn every_single_direct_byte_has_a_deterministic_classification() {
    for byte in u8::MIN..=u8::MAX {
        let mut line = b"%output %1 ".to_vec();
        line.push(byte);
        let result = parse_output_notification(&line);
        let is_direct_payload = byte >= b' ' && byte != b'\\';

        if is_direct_payload {
            assert_eq!(result.unwrap().unwrap().bytes, [byte], "byte=0x{byte:02x}");
        } else {
            assert!(result.is_err(), "byte=0x{byte:02x}");
        }
    }
}
