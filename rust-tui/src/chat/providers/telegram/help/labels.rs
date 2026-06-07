use super::page::HelpPage;

pub(super) fn help_page_title(locale: crate::i18n::Locale, page: HelpPage) -> &'static str {
    match (locale, page) {
        (crate::i18n::Locale::ZhCN, HelpPage::Overview) => "快速开始",
        (crate::i18n::Locale::ZhTW, HelpPage::Overview) => "快速開始",
        (crate::i18n::Locale::Ja, HelpPage::Overview) => "クイックスタート",
        (crate::i18n::Locale::De, HelpPage::Overview) => "Schnellstart",
        (crate::i18n::Locale::Fr, HelpPage::Overview) => "Démarrage rapide",
        (_, HelpPage::Overview) => "Quick Start",
        (crate::i18n::Locale::ZhCN, HelpPage::Codex) => "Codex 命令",
        (crate::i18n::Locale::ZhTW, HelpPage::Codex) => "Codex 命令",
        (crate::i18n::Locale::Ja, HelpPage::Codex) => "Codex コマンド",
        (crate::i18n::Locale::De, HelpPage::Codex) => "Codex-Befehle",
        (crate::i18n::Locale::Fr, HelpPage::Codex) => "Commandes Codex",
        (_, HelpPage::Codex) => "Codex Commands",
        (crate::i18n::Locale::ZhCN, HelpPage::Workflow) => "状态流程",
        (crate::i18n::Locale::ZhTW, HelpPage::Workflow) => "狀態流程",
        (crate::i18n::Locale::Ja, HelpPage::Workflow) => "ステータスの流れ",
        (crate::i18n::Locale::De, HelpPage::Workflow) => "Ablauf",
        (crate::i18n::Locale::Fr, HelpPage::Workflow) => "Flux d'état",
        (_, HelpPage::Workflow) => "Workflow",
    }
}

pub(super) fn help_button_label(locale: crate::i18n::Locale, page: HelpPage) -> &'static str {
    match (locale, page) {
        (crate::i18n::Locale::ZhCN, HelpPage::Overview) => "概览",
        (crate::i18n::Locale::ZhTW, HelpPage::Overview) => "概覽",
        (crate::i18n::Locale::Ja, HelpPage::Overview) => "概要",
        (crate::i18n::Locale::De, HelpPage::Overview) => "Überblick",
        (crate::i18n::Locale::Fr, HelpPage::Overview) => "Vue",
        (_, HelpPage::Overview) => "Overview",
        (crate::i18n::Locale::ZhCN, HelpPage::Codex) => "Codex",
        (crate::i18n::Locale::ZhTW, HelpPage::Codex) => "Codex",
        (crate::i18n::Locale::Ja, HelpPage::Codex) => "Codex",
        (crate::i18n::Locale::De, HelpPage::Codex) => "Codex",
        (crate::i18n::Locale::Fr, HelpPage::Codex) => "Codex",
        (_, HelpPage::Codex) => "Codex",
        (crate::i18n::Locale::ZhCN, HelpPage::Workflow) => "流程",
        (crate::i18n::Locale::ZhTW, HelpPage::Workflow) => "流程",
        (crate::i18n::Locale::Ja, HelpPage::Workflow) => "フロー",
        (crate::i18n::Locale::De, HelpPage::Workflow) => "Flow",
        (crate::i18n::Locale::Fr, HelpPage::Workflow) => "Flux",
        (_, HelpPage::Workflow) => "Flow",
    }
}

pub(super) fn help_action_label(locale: crate::i18n::Locale, key: &str) -> &'static str {
    match (locale, key) {
        (crate::i18n::Locale::ZhCN, "list") => "选择 Agent",
        (crate::i18n::Locale::ZhTW, "list") => "選擇 Agent",
        (crate::i18n::Locale::Ja, "list") => "Agent を選ぶ",
        (crate::i18n::Locale::De, "list") => "Agent wählen",
        (crate::i18n::Locale::Fr, "list") => "Choisir un agent",
        (_, "list") => "Pick Agent",
        (crate::i18n::Locale::ZhCN, "padstatus") => "当前状态",
        (crate::i18n::Locale::ZhTW, "padstatus") => "目前狀態",
        (crate::i18n::Locale::Ja, "padstatus") => "現在の状態",
        (crate::i18n::Locale::De, "padstatus") => "Status",
        (crate::i18n::Locale::Fr, "padstatus") => "État",
        (_, "padstatus") => "Current Status",
        _ => "",
    }
}
