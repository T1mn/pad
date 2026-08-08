mod detect {
    use super::InstallPlan;
    use std::process::Command;

    pub(in crate::system_check) fn tmux_exists() -> bool {
        Command::new("tmux")
            .arg("-V")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    pub(in crate::system_check) fn detect_install_plan() -> Option<InstallPlan> {
        detect_install_plan_for(std::env::consts::OS, &command_exists)
    }

    pub(in crate::system_check) fn detect_install_plan_for(
        os: &str,
        command_exists: &dyn Fn(&str) -> bool,
    ) -> Option<InstallPlan> {
        match os {
            "macos" if command_exists("brew") => Some(InstallPlan::Brew),
            "linux" => {
                if command_exists("apt-get") {
                    Some(InstallPlan::Apt)
                } else if command_exists("dnf") {
                    Some(InstallPlan::Dnf)
                } else if command_exists("yum") {
                    Some(InstallPlan::Yum)
                } else if command_exists("pacman") {
                    Some(InstallPlan::Pacman)
                } else if command_exists("zypper") {
                    Some(InstallPlan::Zypper)
                } else if command_exists("apk") {
                    Some(InstallPlan::Apk)
                } else if command_exists("brew") {
                    Some(InstallPlan::Brew)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(super) fn command_exists(command: &str) -> bool {
        Command::new("sh")
            .args(["-lc", &format!("command -v {command} >/dev/null 2>&1")])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}
mod model {
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
}
mod steps {
    use super::detect::command_exists;
    use super::InstallPlan;
    use std::io;
    use std::process::Command;

    pub(in crate::system_check) fn install_tmux(plan: InstallPlan) -> io::Result<()> {
        for (program, args) in install_steps(plan) {
            let status = Command::new(program).args(args).status()?;
            if !status.success() {
                return Err(io::Error::other(format!(
                    "tmux 安装失败，请手动执行 `{}`。",
                    plan.manual_hint()
                )));
            }
        }
        Ok(())
    }

    fn install_steps(plan: InstallPlan) -> Vec<(&'static str, Vec<&'static str>)> {
        let use_sudo = command_exists("sudo");
        match plan {
            InstallPlan::Brew => vec![("brew", vec!["install", "tmux"])],
            InstallPlan::Apt => apt_steps(use_sudo),
            InstallPlan::Dnf => single_install_steps(use_sudo, "dnf", &["install", "-y", "tmux"]),
            InstallPlan::Yum => single_install_steps(use_sudo, "yum", &["install", "-y", "tmux"]),
            InstallPlan::Pacman => {
                single_install_steps(use_sudo, "pacman", &["-Sy", "--noconfirm", "tmux"])
            }
            InstallPlan::Zypper => single_install_steps(
                use_sudo,
                "zypper",
                &["--non-interactive", "install", "tmux"],
            ),
            InstallPlan::Apk => single_install_steps(use_sudo, "apk", &["add", "tmux"]),
        }
    }

    fn apt_steps(use_sudo: bool) -> Vec<(&'static str, Vec<&'static str>)> {
        if use_sudo {
            vec![
                ("sudo", vec!["apt-get", "update"]),
                ("sudo", vec!["apt-get", "install", "-y", "tmux"]),
            ]
        } else {
            vec![
                ("apt-get", vec!["update"]),
                ("apt-get", vec!["install", "-y", "tmux"]),
            ]
        }
    }

    fn single_install_steps(
        use_sudo: bool,
        program: &'static str,
        args: &[&'static str],
    ) -> Vec<(&'static str, Vec<&'static str>)> {
        if use_sudo {
            let mut sudo_args = vec![program];
            sudo_args.extend_from_slice(args);
            vec![("sudo", sudo_args)]
        } else {
            vec![(program, args.to_vec())]
        }
    }
}

#[cfg(test)]
pub(super) use detect::detect_install_plan_for;
pub(super) use detect::{detect_install_plan, tmux_exists};
pub(super) use model::InstallPlan;
pub(super) use steps::install_tmux;
