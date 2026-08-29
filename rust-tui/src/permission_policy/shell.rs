use super::path::{matching_protected_namespace, resolve_policy_path};
use super::ProtectedNamespace;
use std::path::{Path, PathBuf};

pub(super) enum ShellCommandAssessment<'a> {
    Verified,
    Protected(&'a ProtectedNamespace),
    Unresolved(&'static str),
}

/// Only one simple, literal argv is eligible for automatic confirmation.
/// Shell expansion is intentionally not emulated here: variables, command
/// substitution, globbing, redirection and control operators are evaluated at
/// runtime and therefore cannot prove that a protected path stays untouched.
pub(super) fn assess_shell_command<'a>(
    command: &str,
    cwd: &Path,
    namespaces: &'a [ProtectedNamespace],
) -> ShellCommandAssessment<'a> {
    if command.len() > 32 * 1024 {
        return ShellCommandAssessment::Unresolved("command exceeds the static-analysis limit");
    }
    if let Some(namespace) = textual_protected_namespace(command, namespaces) {
        return ShellCommandAssessment::Protected(namespace);
    }
    let tokens = match tokenize_literal_shell_command(command) {
        Ok(tokens) if !tokens.is_empty() => tokens,
        Ok(_) => return ShellCommandAssessment::Unresolved("command is empty"),
        Err(reason) => return ShellCommandAssessment::Unresolved(reason),
    };
    if tokens.len() > 256 {
        return ShellCommandAssessment::Unresolved("command has too many arguments");
    }
    if tokens.iter().any(|token| is_shell_assignment(token)) {
        return ShellCommandAssessment::Unresolved("shell assignments require runtime expansion");
    }
    if invokes_runtime_evaluator(&tokens) {
        return ShellCommandAssessment::Unresolved(
            "nested interpreters and evaluators are not statically inspectable",
        );
    }

    for (index, token) in tokens.iter().enumerate() {
        if let Some(namespace) = textual_protected_namespace(token, namespaces) {
            return ShellCommandAssessment::Protected(namespace);
        }
        let Some(candidate) = literal_path_candidate(token, index == 0, cwd, namespaces) else {
            continue;
        };
        match candidate {
            Ok(path) => {
                if let Some(namespace) =
                    matching_protected_namespace(&path, Path::new("/"), namespaces)
                {
                    return ShellCommandAssessment::Protected(namespace);
                }
            }
            Err(reason) => return ShellCommandAssessment::Unresolved(reason),
        }
    }
    ShellCommandAssessment::Verified
}

fn textual_protected_namespace<'a>(
    text: &str,
    namespaces: &'a [ProtectedNamespace],
) -> Option<&'a ProtectedNamespace> {
    let text = text.to_ascii_lowercase();
    namespaces.iter().find(|namespace| {
        let root = namespace.root.to_string_lossy().to_ascii_lowercase();
        if !root.is_empty() && text.contains(&root) {
            return true;
        }
        let name = namespace.name.to_ascii_lowercase();
        (name.contains("codex") && text.contains(".codex"))
            || (name == "pi-home" && (text.contains("~/.pi") || text.contains("/.pi/")))
            || ((name.contains("chatgpt") || name.contains("codex"))
                && (text.contains("com.openai.") || text.contains("group.com.openai.")))
            || (name == "pad-desktop-application-support" && text.contains("pad desktop"))
            || (name == "profile-agent-dir" && text.contains("auth.json"))
    })
}

fn tokenize_literal_shell_command(command: &str) -> Result<Vec<String>, &'static str> {
    #[derive(Clone, Copy)]
    enum Quote {
        Single,
        Double,
    }

    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut token_started = false;
    let mut quote = None;
    for character in command.chars() {
        match quote {
            Some(Quote::Single) => {
                if character == '\'' {
                    quote = None;
                } else if character.is_control() {
                    return Err("control characters are not accepted in quoted arguments");
                } else {
                    token.push(character);
                }
            }
            Some(Quote::Double) => match character {
                '"' => quote = None,
                '$' | '`' | '\\' => {
                    return Err("double-quoted expansion requires runtime evaluation");
                }
                value if value.is_control() => {
                    return Err("control characters are not accepted in quoted arguments");
                }
                value => token.push(value),
            },
            None => match character {
                value if value.is_whitespace() => {
                    if token_started {
                        tokens.push(std::mem::take(&mut token));
                        token_started = false;
                    }
                }
                '\'' => {
                    quote = Some(Quote::Single);
                    token_started = true;
                }
                '"' => {
                    quote = Some(Quote::Double);
                    token_started = true;
                }
                '$' | '`' => return Err("shell expansion requires runtime evaluation"),
                '\\' => return Err("shell escapes require runtime evaluation"),
                ';' | '&' | '|' | '<' | '>' | '(' | ')' => {
                    return Err("shell control operators require runtime evaluation");
                }
                '*' | '?' | '[' | ']' | '{' | '}' => {
                    return Err("shell glob or brace expansion requires runtime evaluation");
                }
                '#' | '!' => return Err("shell comments or history expansion are ambiguous"),
                value if value.is_control() => {
                    return Err("control characters are not accepted in commands");
                }
                value => {
                    token.push(value);
                    token_started = true;
                }
            },
        }
    }
    if quote.is_some() {
        return Err("unterminated shell quoting");
    }
    if token_started {
        tokens.push(token);
    }
    Ok(tokens)
}

fn is_shell_assignment(token: &str) -> bool {
    let Some((name, _)) = token.split_once('=') else {
        return false;
    };
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|value| value == '_' || value.is_ascii_alphabetic())
        && characters.all(|value| value == '_' || value.is_ascii_alphanumeric())
}

fn invokes_runtime_evaluator(tokens: &[String]) -> bool {
    let program = Path::new(&tokens[0])
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&tokens[0])
        .to_ascii_lowercase();
    if matches!(
        program.as_str(),
        "sh" | "bash" | "zsh" | "dash" | "ksh" | "fish" | "eval" | "source" | "." | "env" | "xargs"
    ) {
        return true;
    }
    if program == "find"
        && tokens
            .iter()
            .any(|token| matches!(token.as_str(), "-exec" | "-execdir" | "-ok" | "-okdir"))
    {
        return true;
    }
    matches!(
        program.as_str(),
        "python" | "python3" | "perl" | "ruby" | "node" | "bun" | "osascript"
    ) && tokens
        .iter()
        .skip(1)
        .any(|token| matches!(token.as_str(), "-c" | "-e" | "--eval" | "--print" | "-p"))
}

fn literal_path_candidate(
    token: &str,
    is_program: bool,
    cwd: &Path,
    namespaces: &[ProtectedNamespace],
) -> Option<Result<PathBuf, &'static str>> {
    let token = if token.starts_with('-') {
        token.split_once('=').map(|(_, value)| value)?
    } else {
        token
    };
    if token.is_empty() || (is_program && !token.contains('/')) {
        return None;
    }
    if token == "~" || token.starts_with("~/") {
        let Some(home) = protected_home(namespaces) else {
            return Some(Err("home-relative path cannot be resolved"));
        };
        let suffix = token.strip_prefix("~/").unwrap_or_default();
        return Some(Ok(resolve_policy_path(&home.join(suffix), Path::new("/"))));
    }
    if token.starts_with('~') {
        return Some(Err("named-user home expansion cannot be resolved"));
    }
    Some(Ok(resolve_policy_path(Path::new(token), cwd)))
}

fn protected_home(namespaces: &[ProtectedNamespace]) -> Option<PathBuf> {
    namespaces.iter().find_map(|namespace| {
        if matches!(
            namespace.name.as_str(),
            "codex-home" | "pi-home" | "legacy-pad-home"
        ) {
            namespace.root.parent().map(Path::to_path_buf)
        } else {
            None
        }
    })
}
