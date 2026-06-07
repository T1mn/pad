use super::text::help_text;

pub(super) fn help_overview_body(locale: crate::i18n::Locale) -> String {
    format!(
        "{}\n\n<b>{}</b>\n1. <code>/list</code> {}\n2. {}\n3. {}\n\n<b>{}</b>\n• <code>/use &lt;n&gt;</code> {}\n• <code>/history</code> {}\n• <code>/diag</code> {}\n• <code>/padstatus</code> {}\n• <code>/restart</code> {}\n• <code>/reset</code> {}\n• <code>/stop</code> {}",
        help_intro(locale, "overview"),
        help_text(locale, "section.quick"),
        help_text(locale, "overview.step1"),
        help_text(locale, "overview.step2"),
        help_text(locale, "overview.step3"),
        help_text(locale, "section.utility"),
        help_text(locale, "overview.use"),
        help_text(locale, "overview.history"),
        help_text(locale, "overview.diag"),
        help_text(locale, "overview.padstatus"),
        help_text(locale, "overview.restart"),
        help_text(locale, "overview.reset"),
        help_text(locale, "overview.stop"),
    )
}

pub(super) fn help_codex_body(locale: crate::i18n::Locale) -> String {
    format!(
        "{}\n\n<b>{}</b>\n• <code>/status</code> {}\n• <code>/fast on|off|status</code> {}\n• <code>/compact</code> {}\n\n<blockquote>{}</blockquote>",
        help_intro(locale, "codex"),
        help_text(locale, "section.commands"),
        help_text(locale, "codex.status"),
        help_text(locale, "codex.fast"),
        help_text(locale, "codex.compact"),
        help_text(locale, "codex.tip"),
    )
}

pub(super) fn help_workflow_body(locale: crate::i18n::Locale) -> String {
    format!(
        "{}\n\n<b>{}</b>\n• {}\n• {}\n• {}\n• {}\n\n<blockquote>{}</blockquote>",
        help_intro(locale, "workflow"),
        help_text(locale, "section.flow"),
        help_text(locale, "workflow.accepted"),
        help_text(locale, "workflow.working"),
        help_text(locale, "workflow.approval"),
        help_text(locale, "workflow.completed"),
        help_text(locale, "workflow.tip"),
    )
}

fn help_intro(locale: crate::i18n::Locale, key: &str) -> &'static str {
    match (locale, key) {
        (crate::i18n::Locale::ZhCN, "overview") => "更快的开始方式：先选目标，再直接发自然语言。",
        (crate::i18n::Locale::ZhTW, "overview") => "更快的開始方式：先選目標，再直接發自然語言。",
        (crate::i18n::Locale::Ja, "overview") => "いちばん速い使い方は、先に対象を選んでからそのまま自然文を送ることです。",
        (crate::i18n::Locale::De, "overview") => "Am schnellsten geht es so: zuerst ein Ziel wählen und dann einfach normalen Text senden.",
        (crate::i18n::Locale::Fr, "overview") => "Le plus rapide : choisissez d'abord une cible, puis envoyez directement du texte normal.",
        (_, "overview") => "Fastest path: pick a target first, then send plain language directly.",
        (crate::i18n::Locale::ZhCN, "codex") => "这三条是目前已接入 Telegram 的 Codex 命令。",
        (crate::i18n::Locale::ZhTW, "codex") => "這三條是目前已接入 Telegram 的 Codex 命令。",
        (crate::i18n::Locale::Ja, "codex") => "現在 Telegram から使える Codex コマンドはこの 3 つです。",
        (crate::i18n::Locale::De, "codex") => "Diese drei Codex-Befehle sind aktuell direkt über Telegram angebunden.",
        (crate::i18n::Locale::Fr, "codex") => "Ces trois commandes Codex sont actuellement reliées directement à Telegram.",
        (_, "codex") => "These are the Codex commands currently wired into Telegram.",
        (crate::i18n::Locale::ZhCN, "workflow") => "一次完整流程通常会经过受理、执行、审批和完成几个阶段。",
        (crate::i18n::Locale::ZhTW, "workflow") => "一次完整流程通常會經過受理、執行、審批和完成幾個階段。",
        (crate::i18n::Locale::Ja, "workflow") => "1 回の実行は通常、受理、実行、承認待ち、完了のいくつかの段階を通ります。",
        (crate::i18n::Locale::De, "workflow") => "Ein kompletter Ablauf durchläuft meist Annahme, Ausführung, Freigabe und Abschluss.",
        (crate::i18n::Locale::Fr, "workflow") => "Un cycle complet passe généralement par l'acceptation, l'exécution, l'approbation puis la fin.",
        (_, "workflow") => "A full run usually moves through accepted, working, approval, and completed states.",
        _ => "",
    }
}
