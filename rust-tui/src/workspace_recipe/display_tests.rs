use super::display_command;
use crate::workspace_recipe::runner::RecipeCommand;

#[test]
fn display_command_quotes_arguments_without_collecting_segments() {
    let command = RecipeCommand {
        program: "tmux".into(),
        args: vec![
            "new-window".into(),
            "-n".into(),
            "agent one".into(),
            "echo ready".into(),
        ],
    };

    assert_eq!(
        display_command(&command),
        "tmux new-window -n 'agent one' 'echo ready'"
    );
}
