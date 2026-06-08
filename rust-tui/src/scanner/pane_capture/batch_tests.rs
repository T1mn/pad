use super::parse_batch_capture;

#[test]
fn batch_capture_parser_maps_marker_sections_to_panes() {
    let panes = vec!["%1".to_string(), "%2".to_string()];
    let captures = parse_batch_capture(
        "__PAD_CAPTURE_1_0__\nhello\x1b[31m red\x1b[0m\n__PAD_CAPTURE_1_1__\nwaiting\n",
        &panes,
        "__PAD_CAPTURE_1_",
    );

    assert_eq!(captures.get("%1").map(String::as_str), Some("hello red\n"));
    assert_eq!(captures.get("%2").map(String::as_str), Some("waiting\n"));
}
