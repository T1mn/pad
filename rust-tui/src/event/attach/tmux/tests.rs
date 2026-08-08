mod command {
    use super::super::command::format_tmux_args;

    #[test]
    fn tmux_args_format_keeps_attach_log_shape() {
        let args = vec![
            "display-message".to_string(),
            "-p".to_string(),
            "#{pane_id}".to_string(),
        ];

        assert_eq!(format_tmux_args(&args), "display-message -p #{pane_id}");
    }
}

mod query {
    use super::super::query::parse_writable_client;

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
}

mod shell {
    use super::super::shell::{shell_single_quote, wrap_tmux_run_shell};

    #[test]
    fn shell_single_quote_escapes_embedded_single_quotes() {
        assert_eq!(shell_single_quote("bob's pane"), r#"'bob'\''s pane'"#);
    }

    #[test]
    fn wrap_tmux_run_shell_quotes_script() {
        assert_eq!(
            wrap_tmux_run_shell("tmux display-message 'ready'"),
            r#"sh -lc 'tmux display-message '\''ready'\'''"#
        );
    }
}
