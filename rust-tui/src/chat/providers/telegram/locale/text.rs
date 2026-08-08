use super::select::locale_prefers_chinese;

mod approval {
    pub(super) fn text(key: &str, zh: bool) -> Option<&'static str> {
        Some(match key {
            "callback.invalid" if zh => "无效回调",
            "callback.invalid" => "Invalid callback",
            "callback.private_only" if zh => "仅支持私聊",
            "callback.private_only" => "Private chats only",
            "callback.bound_other" if zh => "该 bot 已绑定到其他聊天",
            "callback.bound_other" => "This bot is already linked to another chat",
            "callback.no_data" if zh => "无回调数据",
            "callback.no_data" => "Missing callback data",
            "callback.switched" if zh => "已切换当前目标",
            "callback.switched" => "Target switched",
            "callback.stale" if zh => "目标 pane 已失效，请重新 /list",
            "callback.stale" => "The target pane is gone. Run /list again.",
            "callback.unknown" if zh => "未知操作",
            "callback.unknown" => "Unknown action",
            "approval.none" if zh => "当前没有待处理的 Codex 确认请求",
            "approval.none" => "There is no pending Codex approval request.",
            "approval.failed" if zh => "发送确认失败：{}",
            "approval.failed" => "Failed to send approval input: {}",
            "approval.prompt" if zh => "Codex 需要你确认一条提权命令",
            "approval.prompt" => "Codex needs approval for an escalated command",
            "approval.target" if zh => "目标",
            "approval.target" => "Target",
            "approval.button.once" if zh => "批准一次",
            "approval.button.once" => "Approve once",
            "approval.button.always" if zh => "本次会话总是允许",
            "approval.button.always" => "Always for session",
            "approval.button.reject" if zh => "拒绝",
            "approval.button.reject" => "Reject",
            "approval.sent.once" if zh => "已发送批准一次",
            "approval.sent.once" => "Approve once sent",
            "approval.sent.always" if zh => "已发送本次会话总是允许",
            "approval.sent.always" => "Always for session sent",
            "approval.sent.reject" if zh => "已发送拒绝",
            "approval.sent.reject" => "Reject sent",
            _ => return None,
        })
    }
}
mod command {
    pub(super) fn text(key: &str, zh: bool) -> Option<&'static str> {
        Some(match key {
            "command.start" if zh => "绑定当前聊天并显示欢迎信息",
            "command.start" => "Link the current chat and show welcome text",
            "command.help" if zh => "查看可用命令",
            "command.help" => "Show available commands",
            "command.list" if zh => "列出可点击的 agent pane",
            "command.list" => "List clickable agent panes",
            "command.use" if zh => "按编号选择目标 agent",
            "command.use" => "Select the target agent by number",
            "command.history" if zh => "查看当前目标最近 3 条问答",
            "command.history" => "Show the current target's latest 3 turns",
            "command.diag" if zh => "查看当前会话连续性诊断",
            "command.diag" => "Show the current session continuity diagnostic",
            "command.restart" if zh => "重编译并重启整个 pad",
            "command.restart" => "Rebuild and restart the whole pad",
            "command.status" if zh => "查看当前 Codex 会话状态",
            "command.status" => "Show the selected Codex session status",
            "command.fast" if zh => "切换或查看 Codex Fast mode",
            "command.fast" => "Toggle or inspect Codex Fast mode",
            "command.compact" if zh => "压缩当前 Codex 对话上下文",
            "command.compact" => "Compact the current Codex conversation",
            "command.reset" if zh => "重置当前目标的 Telegram pending",
            "command.reset" => "Clear the current target's Telegram pending state",
            "command.stop" if zh => "尝试中断当前 agent",
            "command.stop" => "Try to interrupt the current agent",
            _ => return None,
        })
    }
}
mod core;
mod status;

pub(in crate::chat::providers::telegram) fn tg(locale: crate::i18n::Locale, key: &str) -> &str {
    let zh = locale_prefers_chinese(locale);
    core::text(key, zh)
        .or_else(|| approval::text(key, zh))
        .or_else(|| status::text(key, zh))
        .or_else(|| command::text(key, zh))
        .unwrap_or(key)
}
