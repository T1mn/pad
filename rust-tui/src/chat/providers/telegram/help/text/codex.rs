pub(super) fn text(locale: crate::i18n::Locale, key: &str) -> Option<&'static str> {
    Some(match (locale, key) {
        (crate::i18n::Locale::ZhCN, "codex.status") => "抓取当前 Codex pane 的状态摘要。",
        (crate::i18n::Locale::ZhTW, "codex.status") => "擷取目前 Codex pane 的狀態摘要。",
        (crate::i18n::Locale::Ja, "codex.status") => {
            "現在の Codex pane の状態サマリーを取得します。"
        }
        (crate::i18n::Locale::De, "codex.status") => {
            "Liest eine Statuszusammenfassung des aktuellen Codex-Panes aus."
        }
        (crate::i18n::Locale::Fr, "codex.status") => {
            "Récupère un résumé d'état du pane Codex actuel."
        }
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
        (crate::i18n::Locale::Fr, "codex.compact") => {
            "Compacte le contexte courant de la conversation."
        }
        (_, "codex.compact") => "compact the current conversation context.",
        (crate::i18n::Locale::ZhCN, "codex.tip") => "这 3 个命令目前只对选中的 Codex pane 生效。",
        (crate::i18n::Locale::ZhTW, "codex.tip") => "這 3 個命令目前只對選中的 Codex pane 生效。",
        (crate::i18n::Locale::Ja, "codex.tip") => {
            "この 3 つのコマンドは、選択中の Codex pane に対してのみ有効です。"
        }
        (crate::i18n::Locale::De, "codex.tip") => {
            "Diese drei Befehle funktionieren aktuell nur für das ausgewählte Codex-Pane."
        }
        (crate::i18n::Locale::Fr, "codex.tip") => {
            "Ces trois commandes ne fonctionnent actuellement que sur le pane Codex sélectionné."
        }
        (_, "codex.tip") => "These three commands currently work only on the selected Codex pane.",
        _ => return None,
    })
}
