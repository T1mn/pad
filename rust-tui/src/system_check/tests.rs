use super::install::{detect_install_plan_for, InstallPlan};
use super::is_yes_answer;

#[test]
fn detect_install_plan_prefers_apt_on_linux() {
    let exists = |command: &str| matches!(command, "apt-get" | "dnf");
    assert_eq!(
        detect_install_plan_for("linux", &exists),
        Some(InstallPlan::Apt)
    );
}

#[test]
fn detect_install_plan_uses_brew_on_macos() {
    let exists = |command: &str| command == "brew";
    assert_eq!(
        detect_install_plan_for("macos", &exists),
        Some(InstallPlan::Brew)
    );
}

#[test]
fn yes_answer_accepts_short_and_long_form() {
    assert!(is_yes_answer("y"));
    assert!(is_yes_answer("YES"));
    assert!(is_yes_answer(" Yes\n"));
    assert!(!is_yes_answer("n"));
    assert!(!is_yes_answer(""));
}
