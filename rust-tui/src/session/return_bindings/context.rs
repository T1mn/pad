pub(in crate::session) struct ReturnBindingContext<'a> {
    pub(in crate::session) trace_id: &'a str,
    pub(in crate::session) target_session: &'a str,
    pub(in crate::session) target_window: &'a str,
    pub(in crate::session) pad_pane: &'a str,
    pub(in crate::session) pad_window: &'a str,
    pub(in crate::session) pad_session: &'a str,
    pub(in crate::session) status_restore_value: Option<&'a str>,
}
