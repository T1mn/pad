use super::state::TelegramState;
use serde_json::json;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HelpPage {
    Overview,
    Codex,
    Workflow,
}

impl HelpPage {
    pub(super) fn from_callback(data: &str) -> Option<Self> {
        match data {
            "help:overview" => Some(Self::Overview),
            "help:codex" => Some(Self::Codex),
            "help:workflow" => Some(Self::Workflow),
            _ => None,
        }
    }

    pub(super) fn callback_data(self) -> &'static str {
        match self {
            Self::Overview => "help:overview",
            Self::Codex => "help:codex",
            Self::Workflow => "help:workflow",
        }
    }
}

pub(super) fn help_message_payload(
    locale: crate::i18n::Locale,
    state: &TelegramState,
    chat_id: serde_json::Value,
    message_id: Option<i64>,
    page: HelpPage,
) -> serde_json::Value {
    let mut payload = json!({
        "chat_id": chat_id,
        "text": help_page_html(locale, state, page),
        "parse_mode": "HTML",
        "disable_web_page_preview": true,
        "reply_markup": {
            "inline_keyboard": build_help_keyboard(locale, page)
        }
    });
    if let Some(message_id) = message_id {
        payload["message_id"] = serde_json::Value::from(message_id);
    }
    payload
}

pub(super) fn build_help_keyboard(
    locale: crate::i18n::Locale,
    page: HelpPage,
) -> Vec<Vec<serde_json::Value>> {
    let nav_button = |target: HelpPage| {
        let text = if target == page {
            format!("• {}", help_button_label(locale, target))
        } else {
            help_button_label(locale, target).to_string()
        };
        json!({
            "text": text,
            "callback_data": target.callback_data(),
        })
    };
    vec![
        vec![
            nav_button(HelpPage::Overview),
            nav_button(HelpPage::Codex),
            nav_button(HelpPage::Workflow),
        ],
        vec![
            json!({
                "text": help_action_label(locale, "list"),
                "callback_data": "help:list",
            }),
            json!({
                "text": help_action_label(locale, "padstatus"),
                "callback_data": "help:padstatus",
            }),
        ],
    ]
}

pub(super) fn help_page_html(
    locale: crate::i18n::Locale,
    state: &TelegramState,
    page: HelpPage,
) -> String {
    let target = state
        .selected_target
        .as_ref()
        .map(|target| html_escape(&target.label))
        .unwrap_or_else(|| help_text(locale, "target.none").to_string());
    let page_title = help_page_title(locale, page);
    let page_body = match page {
        HelpPage::Overview => help_overview_body(locale),
        HelpPage::Codex => help_codex_body(locale),
        HelpPage::Workflow => help_workflow_body(locale),
    };
    format!(
        "<b>{}</b>\n<blockquote>{}: <code>{}</code></blockquote>\n\n<b>{}</b>\n{}",
        help_text(locale, "title"),
        help_text(locale, "target"),
        target,
        page_title,
        page_body
    )
}

fn help_overview_body(locale: crate::i18n::Locale) -> String {
    format!(
        "{}\n\n<b>{}</b>\n1. <code>/list</code> {}\n2. {}\n3. {}\n\n<b>{}</b>\n• <code>/use &lt;n&gt;</code> {}\n• <code>/padstatus</code> {}\n• <code>/stop</code> {}",
        help_intro(locale, "overview"),
        help_text(locale, "section.quick"),
        help_text(locale, "overview.step1"),
        help_text(locale, "overview.step2"),
        help_text(locale, "overview.step3"),
        help_text(locale, "section.utility"),
        help_text(locale, "overview.use"),
        help_text(locale, "overview.padstatus"),
        help_text(locale, "overview.stop"),
    )
}

fn help_codex_body(locale: crate::i18n::Locale) -> String {
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

fn help_workflow_body(locale: crate::i18n::Locale) -> String {
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

fn help_page_title(locale: crate::i18n::Locale, page: HelpPage) -> &'static str {
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

fn help_button_label(locale: crate::i18n::Locale, page: HelpPage) -> &'static str {
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

fn help_action_label(locale: crate::i18n::Locale, key: &str) -> &'static str {
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

fn help_text(locale: crate::i18n::Locale, key: &str) -> &'static str {
    match (locale, key) {
        (crate::i18n::Locale::ZhCN, "title") => "Pad Telegram",
        (crate::i18n::Locale::ZhTW, "title") => "Pad Telegram",
        (crate::i18n::Locale::Ja, "title") => "Pad Telegram",
        (crate::i18n::Locale::De, "title") => "Pad Telegram",
        (crate::i18n::Locale::Fr, "title") => "Pad Telegram",
        (_, "title") => "Pad Telegram",
        (crate::i18n::Locale::ZhCN, "target") => "当前目标",
        (crate::i18n::Locale::ZhTW, "target") => "目前目標",
        (crate::i18n::Locale::Ja, "target") => "現在のターゲット",
        (crate::i18n::Locale::De, "target") => "Aktuelles Ziel",
        (crate::i18n::Locale::Fr, "target") => "Cible actuelle",
        (_, "target") => "Current Target",
        (crate::i18n::Locale::ZhCN, "target.none") => "尚未选择",
        (crate::i18n::Locale::ZhTW, "target.none") => "尚未選擇",
        (crate::i18n::Locale::Ja, "target.none") => "未選択",
        (crate::i18n::Locale::De, "target.none") => "nichts ausgewählt",
        (crate::i18n::Locale::Fr, "target.none") => "aucune cible",
        (_, "target.none") => "none selected",
        (crate::i18n::Locale::ZhCN, "section.quick") => "开始使用",
        (crate::i18n::Locale::ZhTW, "section.quick") => "開始使用",
        (crate::i18n::Locale::Ja, "section.quick") => "はじめ方",
        (crate::i18n::Locale::De, "section.quick") => "Loslegen",
        (crate::i18n::Locale::Fr, "section.quick") => "Pour démarrer",
        (_, "section.quick") => "Getting Started",
        (crate::i18n::Locale::ZhCN, "section.utility") => "常用补充",
        (crate::i18n::Locale::ZhTW, "section.utility") => "常用補充",
        (crate::i18n::Locale::Ja, "section.utility") => "補助コマンド",
        (crate::i18n::Locale::De, "section.utility") => "Nützlich",
        (crate::i18n::Locale::Fr, "section.utility") => "Utilitaires",
        (_, "section.utility") => "Useful Extras",
        (crate::i18n::Locale::ZhCN, "section.commands") => "已接入命令",
        (crate::i18n::Locale::ZhTW, "section.commands") => "已接入命令",
        (crate::i18n::Locale::Ja, "section.commands") => "対応済みコマンド",
        (crate::i18n::Locale::De, "section.commands") => "Verfügbare Befehle",
        (crate::i18n::Locale::Fr, "section.commands") => "Commandes disponibles",
        (_, "section.commands") => "Available Commands",
        (crate::i18n::Locale::ZhCN, "section.flow") => "执行阶段",
        (crate::i18n::Locale::ZhTW, "section.flow") => "執行階段",
        (crate::i18n::Locale::Ja, "section.flow") => "実行ステージ",
        (crate::i18n::Locale::De, "section.flow") => "Phasen",
        (crate::i18n::Locale::Fr, "section.flow") => "Étapes",
        (_, "section.flow") => "Stages",
        (crate::i18n::Locale::ZhCN, "overview.step1") => "查看在线 agent，并点选一个目标。",
        (crate::i18n::Locale::ZhTW, "overview.step1") => "查看在線 agent，並點選一個目標。",
        (crate::i18n::Locale::Ja, "overview.step1") => "オンラインの agent を一覧し、対象を 1 つ選びます。",
        (crate::i18n::Locale::De, "overview.step1") => "Verfügbare Agents anzeigen und ein Ziel auswählen.",
        (crate::i18n::Locale::Fr, "overview.step1") => "Afficher les agents disponibles et choisir une cible.",
        (_, "overview.step1") => "show online agents and pick one target.",
        (crate::i18n::Locale::ZhCN, "overview.step2") => "选定后，直接发送普通文本作为 prompt。",
        (crate::i18n::Locale::ZhTW, "overview.step2") => "選定後，直接傳送普通文字作為 prompt。",
        (crate::i18n::Locale::Ja, "overview.step2") => "選んだら、そのまま通常の文章を prompt として送れます。",
        (crate::i18n::Locale::De, "overview.step2") => "Danach einfach normalen Text als Prompt senden.",
        (crate::i18n::Locale::Fr, "overview.step2") => "Ensuite, envoyez simplement du texte normal comme prompt.",
        (_, "overview.step2") => "send plain text as the prompt.",
        (crate::i18n::Locale::ZhCN, "overview.step3") => "执行过程中会显示状态、审批按钮和最终结果。",
        (crate::i18n::Locale::ZhTW, "overview.step3") => "執行過程中會顯示狀態、審批按鈕和最終結果。",
        (crate::i18n::Locale::Ja, "overview.step3") => "実行中は状態、承認ボタン、最終結果が順に表示されます。",
        (crate::i18n::Locale::De, "overview.step3") => "Währenddessen erscheinen Status, Freigabeknöpfe und das Endergebnis.",
        (crate::i18n::Locale::Fr, "overview.step3") => "Pendant l'exécution, l'état, les boutons d'approbation et le résultat final s'affichent.",
        (_, "overview.step3") => "watch status, approval buttons, and the final result.",
        (crate::i18n::Locale::ZhCN, "overview.use") => "按编号切换目标。",
        (crate::i18n::Locale::ZhTW, "overview.use") => "按編號切換目標。",
        (crate::i18n::Locale::Ja, "overview.use") => "番号で対象を切り替えます。",
        (crate::i18n::Locale::De, "overview.use") => "Ziel per Nummer umschalten.",
        (crate::i18n::Locale::Fr, "overview.use") => "Changer de cible par numéro.",
        (_, "overview.use") => "switch target by number.",
        (crate::i18n::Locale::ZhCN, "overview.padstatus") => "查看 pad 和 bot 当前运行状态。",
        (crate::i18n::Locale::ZhTW, "overview.padstatus") => "查看 pad 和 bot 目前執行狀態。",
        (crate::i18n::Locale::Ja, "overview.padstatus") => "pad と bot の現在の稼働状態を確認します。",
        (crate::i18n::Locale::De, "overview.padstatus") => "Aktuellen Zustand von pad und Bot anzeigen.",
        (crate::i18n::Locale::Fr, "overview.padstatus") => "Voir l'état actuel de pad et du bot.",
        (_, "overview.padstatus") => "show the current pad and bot state.",
        (crate::i18n::Locale::ZhCN, "overview.stop") => "向当前目标发送一次 Escape。",
        (crate::i18n::Locale::ZhTW, "overview.stop") => "向目前目標傳送一次 Escape。",
        (crate::i18n::Locale::Ja, "overview.stop") => "現在の対象へ Escape を 1 回送ります。",
        (crate::i18n::Locale::De, "overview.stop") => "Sendet einmal Escape an das aktuelle Ziel.",
        (crate::i18n::Locale::Fr, "overview.stop") => "Envoie une fois Escape à la cible actuelle.",
        (_, "overview.stop") => "send one Escape to the current target.",
        (crate::i18n::Locale::ZhCN, "codex.status") => "抓取当前 Codex pane 的状态摘要。",
        (crate::i18n::Locale::ZhTW, "codex.status") => "擷取目前 Codex pane 的狀態摘要。",
        (crate::i18n::Locale::Ja, "codex.status") => "現在の Codex pane の状態サマリーを取得します。",
        (crate::i18n::Locale::De, "codex.status") => "Liest eine Statuszusammenfassung des aktuellen Codex-Panes aus.",
        (crate::i18n::Locale::Fr, "codex.status") => "Récupère un résumé d'état du pane Codex actuel.",
        (_, "codex.status") => "capture a short status summary from the current Codex pane.",
        (crate::i18n::Locale::ZhCN, "codex.fast") => "切换或查看 Fast mode。",
        (crate::i18n::Locale::ZhTW, "codex.fast") => "切換或查看 Fast mode。",
        (crate::i18n::Locale::Ja, "codex.fast") => "Fast mode を切り替えるか確認します。",
        (crate::i18n::Locale::De, "codex.fast") => "Schaltet den Fast-Modus um oder zeigt ihn an.",
        (crate::i18n::Locale::Fr, "codex.fast") => "Active, désactive ou affiche le mode Fast.",
        (_, "codex.fast") => "toggle or inspect Fast mode.",
        (crate::i18n::Locale::ZhCN, "codex.compact") => "压缩当前对话上下文。",
        (crate::i18n::Locale::ZhTW, "codex.compact") => "壓縮目前對話上下文。",
        (crate::i18n::Locale::Ja, "codex.compact") => "現在の会話コンテキストを圧縮します。",
        (crate::i18n::Locale::De, "codex.compact") => "Verdichtet den aktuellen Gesprächskontext.",
        (crate::i18n::Locale::Fr, "codex.compact") => "Compacte le contexte courant de la conversation.",
        (_, "codex.compact") => "compact the current conversation context.",
        (crate::i18n::Locale::ZhCN, "codex.tip") => "这 3 个命令目前只对选中的 Codex pane 生效。",
        (crate::i18n::Locale::ZhTW, "codex.tip") => "這 3 個命令目前只對選中的 Codex pane 生效。",
        (crate::i18n::Locale::Ja, "codex.tip") => "この 3 つのコマンドは、選択中の Codex pane に対してのみ有効です。",
        (crate::i18n::Locale::De, "codex.tip") => "Diese drei Befehle funktionieren aktuell nur für das ausgewählte Codex-Pane.",
        (crate::i18n::Locale::Fr, "codex.tip") => "Ces trois commandes ne fonctionnent actuellement que sur le pane Codex sélectionné.",
        (_, "codex.tip") => "These three commands currently work only on the selected Codex pane.",
        (crate::i18n::Locale::ZhCN, "workflow.accepted") => "已提交：prompt 已成功送入目标 pane。",
        (crate::i18n::Locale::ZhTW, "workflow.accepted") => "已提交：prompt 已成功送入目標 pane。",
        (crate::i18n::Locale::Ja, "workflow.accepted") => "Submitted: prompt が対象 pane に正常に投入されました。",
        (crate::i18n::Locale::De, "workflow.accepted") => "Submitted: Der Prompt wurde erfolgreich an das Ziel-Pane gesendet.",
        (crate::i18n::Locale::Fr, "workflow.accepted") => "Submitted : le prompt a bien été injecté dans le pane cible.",
        (_, "workflow.accepted") => "Submitted: the prompt has been injected into the target pane.",
        (crate::i18n::Locale::ZhCN, "workflow.working") => "进行中：bot 会持续显示工作状态。",
        (crate::i18n::Locale::ZhTW, "workflow.working") => "進行中：bot 會持續顯示工作狀態。",
        (crate::i18n::Locale::Ja, "workflow.working") => "Working: bot が作業中ステータスを継続表示します。",
        (crate::i18n::Locale::De, "workflow.working") => "Working: Der Bot zeigt fortlaufend den Arbeitsstatus an.",
        (crate::i18n::Locale::Fr, "workflow.working") => "Working : le bot affiche l'état de travail en continu.",
        (_, "workflow.working") => "Working: the bot keeps showing live status.",
        (crate::i18n::Locale::ZhCN, "workflow.approval") => "等待确认：Codex 需要提权时会弹出审批按钮。",
        (crate::i18n::Locale::ZhTW, "workflow.approval") => "等待確認：Codex 需要提權時會彈出審批按鈕。",
        (crate::i18n::Locale::Ja, "workflow.approval") => "承認待ち: Codex が権限昇格を求めると承認ボタンが表示されます。",
        (crate::i18n::Locale::De, "workflow.approval") => "Freigabe nötig: Wenn Codex erhöhte Rechte braucht, erscheinen Freigabeknöpfe.",
        (crate::i18n::Locale::Fr, "workflow.approval") => "Approbation requise : si Codex demande une élévation, des boutons d'approbation apparaissent.",
        (_, "workflow.approval") => "Needs approval: approval buttons appear when Codex requests escalation.",
        (crate::i18n::Locale::ZhCN, "workflow.completed") => "已完成：Stop hook 到达后回传最终结果。",
        (crate::i18n::Locale::ZhTW, "workflow.completed") => "已完成：Stop hook 到達後回傳最終結果。",
        (crate::i18n::Locale::Ja, "workflow.completed") => "Completed: Stop hook 到着後に最終結果が返送されます。",
        (crate::i18n::Locale::De, "workflow.completed") => "Completed: Nach dem Stop-Hook wird das Endergebnis zurückgesendet.",
        (crate::i18n::Locale::Fr, "workflow.completed") => "Completed : le résultat final est renvoyé après le hook Stop.",
        (_, "workflow.completed") => "Completed: the final result is sent back after the stop hook.",
        (crate::i18n::Locale::ZhCN, "workflow.tip") => "如果长时间卡在 Waiting，先在 pad 里确认当前 pane 是否还在等待审批或用户输入。",
        (crate::i18n::Locale::ZhTW, "workflow.tip") => "如果長時間卡在 Waiting，先在 pad 裡確認目前 pane 是否仍在等待審批或使用者輸入。",
        (crate::i18n::Locale::Ja, "workflow.tip") => "Waiting が長く続く場合は、pad 側でその pane がまだ承認や入力待ちかを先に確認してください。",
        (crate::i18n::Locale::De, "workflow.tip") => "Wenn Waiting lange bestehen bleibt, prüfe zuerst in pad, ob das Pane noch auf Freigabe oder Eingabe wartet.",
        (crate::i18n::Locale::Fr, "workflow.tip") => "Si l'état Waiting dure trop longtemps, vérifiez d'abord dans pad si le pane attend encore une approbation ou une saisie.",
        (_, "workflow.tip") => "If Waiting lasts too long, check in pad whether the pane is still waiting for approval or user input.",
        _ => "",
    }
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
