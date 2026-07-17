use super::parse_writable_client;

#[test]
fn prefers_a_writable_client_showing_the_pad_pane() {
    let clients = concat!(
        "client-readonly\t%4\tattached,control-mode,read-only,UTF-8\n",
        "client-other\t%7\tattached,focused,UTF-8\n",
        "client-writable\t%4\tattached,control-mode,UTF-8\n",
    );

    assert_eq!(
        parse_writable_client(clients, "%4").as_deref(),
        Some("client-writable")
    );
}

#[test]
fn refuses_a_read_only_only_match() {
    let clients = "client-readonly\t%4\tattached,control-mode,read-only,UTF-8\n";
    assert_eq!(parse_writable_client(clients, "%4"), None);
}
