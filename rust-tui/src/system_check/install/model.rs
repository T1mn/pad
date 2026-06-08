#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::system_check) enum InstallPlan {
    Brew,
    Apt,
    Dnf,
    Yum,
    Pacman,
    Zypper,
    Apk,
}

impl InstallPlan {
    pub(in crate::system_check) fn manual_hint(self) -> &'static str {
        match self {
            Self::Brew => "brew install tmux",
            Self::Apt => "sudo apt-get update && sudo apt-get install -y tmux",
            Self::Dnf => "sudo dnf install -y tmux",
            Self::Yum => "sudo yum install -y tmux",
            Self::Pacman => "sudo pacman -Sy --noconfirm tmux",
            Self::Zypper => "sudo zypper --non-interactive install tmux",
            Self::Apk => "sudo apk add tmux",
        }
    }
}
