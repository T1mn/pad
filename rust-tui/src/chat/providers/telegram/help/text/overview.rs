pub(super) fn text(locale: crate::i18n::Locale, key: &str) -> Option<&'static str> {
    Some(match (locale, key) {
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
        (crate::i18n::Locale::ZhCN, "overview.history") => "查看当前目标最近 3 条问答。",
        (crate::i18n::Locale::ZhTW, "overview.history") => "查看目前目標最近 3 條問答。",
        (crate::i18n::Locale::Ja, "overview.history") => "現在の対象の直近 3 件のやり取りを表示します。",
        (crate::i18n::Locale::De, "overview.history") => {
            "Zeigt die letzten drei Frage-Antwort-Runden des aktuellen Ziels."
        }
        (crate::i18n::Locale::Fr, "overview.history") => {
            "Affiche les trois derniers échanges question-réponse de la cible actuelle."
        }
        (_, "overview.history") => "show the current target's latest three turns.",
        (crate::i18n::Locale::ZhCN, "overview.diag") => "查看当前会话的 continuity 诊断，可用于排查 frozen 或 lagging。",
        (crate::i18n::Locale::ZhTW, "overview.diag") => "查看目前會話的 continuity 診斷，可用於排查 frozen 或 lagging。",
        (crate::i18n::Locale::Ja, "overview.diag") => "現在のセッションの continuity 診断を表示し、frozen や lagging を確認します。",
        (crate::i18n::Locale::De, "overview.diag") => {
            "Zeigt die Continuity-Diagnose der aktuellen Sitzung, etwa bei frozen oder lagging."
        }
        (crate::i18n::Locale::Fr, "overview.diag") => {
            "Affiche le diagnostic de continuité de la session courante, utile pour frozen ou lagging."
        }
        (_, "overview.diag") => {
            "show the current session continuity diagnostic for frozen or lagging cases."
        }
        (crate::i18n::Locale::ZhCN, "overview.padstatus") => "查看 pad 和 bot 当前运行状态。",
        (crate::i18n::Locale::ZhTW, "overview.padstatus") => "查看 pad 和 bot 目前執行狀態。",
        (crate::i18n::Locale::Ja, "overview.padstatus") => "pad と bot の現在の稼働状態を確認します。",
        (crate::i18n::Locale::De, "overview.padstatus") => "Aktuellen Zustand von pad und Bot anzeigen.",
        (crate::i18n::Locale::Fr, "overview.padstatus") => "Voir l'état actuel de pad et du bot.",
        (_, "overview.padstatus") => "show the current pad and bot state.",
        (crate::i18n::Locale::ZhCN, "overview.restart") => {
            "重编译并重启整个 pad；适合远程恢复当前界面。"
        }
        (crate::i18n::Locale::ZhTW, "overview.restart") => {
            "重編譯並重啟整個 pad；適合遠端恢復目前介面。"
        }
        (crate::i18n::Locale::Ja, "overview.restart") => {
            "pad 全体を再ビルドして再起動します。離席中の復旧向けです。"
        }
        (crate::i18n::Locale::De, "overview.restart") => {
            "Baut das gesamte pad neu und startet es neu. Nützlich für Remote-Wiederherstellung."
        }
        (crate::i18n::Locale::Fr, "overview.restart") => {
            "Recompile et redémarre tout pad. Pratique pour une reprise à distance."
        }
        (_, "overview.restart") => {
            "rebuild and restart the whole pad, useful when recovering remotely."
        }
        (crate::i18n::Locale::ZhCN, "overview.reset") => {
            "清掉当前目标卡住的 Telegram pending，不会中断 pane 内的 agent。"
        }
        (crate::i18n::Locale::ZhTW, "overview.reset") => {
            "清掉目前目標卡住的 Telegram pending，不會中斷 pane 內的 agent。"
        }
        (crate::i18n::Locale::Ja, "overview.reset") => {
            "現在の対象で詰まった Telegram pending を消します。pane 内の agent は停止しません。"
        }
        (crate::i18n::Locale::De, "overview.reset") => {
            "Entfernt einen festhängenden Telegram-Pending-Eintrag für das aktuelle Ziel, ohne den Agent im Pane zu stoppen."
        }
        (crate::i18n::Locale::Fr, "overview.reset") => {
            "Supprime un pending Telegram bloqué pour la cible actuelle sans arrêter l'agent dans le pane."
        }
        (_, "overview.reset") => {
            "clear a stuck Telegram pending state for the current target without stopping the agent in the pane."
        }
        (crate::i18n::Locale::ZhCN, "overview.stop") => "向当前目标发送一次 Escape。",
        (crate::i18n::Locale::ZhTW, "overview.stop") => "向目前目標傳送一次 Escape。",
        (crate::i18n::Locale::Ja, "overview.stop") => "現在の対象へ Escape を 1 回送ります。",
        (crate::i18n::Locale::De, "overview.stop") => "Sendet einmal Escape an das aktuelle Ziel.",
        (crate::i18n::Locale::Fr, "overview.stop") => "Envoie une fois Escape à la cible actuelle.",
        (_, "overview.stop") => "send one Escape to the current target.",
        _ => return None,
    })
}
