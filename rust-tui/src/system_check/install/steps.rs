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
