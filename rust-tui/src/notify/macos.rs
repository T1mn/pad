use super::NotificationRequest;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MacCommandSpec {
    pub(super) program: String,
    pub(super) args: Vec<String>,
}

pub(super) fn command_spec(request: &NotificationRequest) -> MacCommandSpec {
    MacCommandSpec {
        program: "osascript".into(),
        args: vec![
            "-e".into(),
            "on run argv".into(),
            "-e".into(),
            "display notification (item 2 of argv) with title (item 1 of argv)".into(),
            "-e".into(),
            "end run".into(),
            request.title.clone(),
            request.body.clone(),
        ],
    }
}
