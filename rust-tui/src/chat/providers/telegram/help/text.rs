pub(super) fn help_text(locale: crate::i18n::Locale, key: &str) -> &'static str {
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
