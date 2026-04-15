use crate::theme::Config;

pub(super) fn telegram_locale(config: &Config) -> crate::i18n::Locale {
    crate::i18n::Locale::from_str(&config.language)
}

pub(super) fn locale_prefers_chinese(locale: crate::i18n::Locale) -> bool {
    matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
    )
}

pub(super) fn tg(locale: crate::i18n::Locale, key: &str) -> &str {
    let zh = locale_prefers_chinese(locale);
    match key {
        "bind.success" if zh => {
            "Pad Telegram 已绑定。先用 /list 查看并点击目标，或用 /use <编号> 选择目标，然后直接发消息。"
        }
        "bind.success" => {
            "Pad Telegram is linked. Use /list to pick a target, or /use <number>, then send plain text."
        }
        "bind.start_required" if zh => "请先发送 /start 以绑定当前聊天。",
        "bind.start_required" => "Send /start first to link this chat.",
        "bind.other_chat" if zh => "这个 bot 已绑定到另一个 Telegram 聊天。",
        "bind.other_chat" => "This bot is already linked to another Telegram chat.",
        "start.ready" if zh => {
            "Pad Telegram 已就绪。用 /list 查看并点击目标，或用 /use <编号> 选择；/status 查看 Codex 状态，/fast 切换 Fast mode，/compact 压缩上下文，/padstatus 查看 bot 当前状态。"
        }
        "start.ready" => {
            "Pad Telegram is ready. Use /list to pick a target, or /use <number>; /status shows Codex status, /fast toggles Fast mode, /compact compacts context, and /padstatus shows bot state."
        }
        "help.body" if zh => {
            "/start\n/help\n/list\n/agents\n/use <编号>\n/status\n/fast [on|off|status]\n/compact\n/padstatus\n/stop\n\n选择目标后，直接发送普通文本即可。"
        }
        "help.body" => {
            "/start\n/help\n/list\n/agents\n/use <number>\n/status\n/fast [on|off|status]\n/compact\n/padstatus\n/stop\n\nAfter selecting a target, just send plain text."
        }
        "use.usage" if zh => "用法：/use <编号>。先执行 /agents 获取编号。",
        "use.usage" => "Usage: /use <number>. Run /agents first to get a fresh list.",
        "use.invalid" if zh => "编号无效。请先执行 /agents 获取最新列表。",
        "use.invalid" => "Invalid number. Run /agents first to get the latest list.",
        "pane.stale" if zh => "目标 pane 已失效，请重新执行 /agents。",
        "pane.stale" => "The target pane is no longer available. Run /agents again.",
        "target.none" if zh => "还没有当前目标。请先 /agents 再 /use。",
        "target.none" => "No target selected yet. Use /agents and then /use first.",
        "target.not_codex" if zh => "当前目标不是 Codex pane，这 3 个命令暂时只支持 Codex。",
        "target.not_codex" => "The selected target is not a Codex pane. These commands currently support Codex only.",
        "unknown.command" if zh => "未知命令。发送 /help 查看可用命令。",
        "unknown.command" => "Unknown command. Send /help to see available commands.",
        "pending.exists" if zh => "当前目标已有待处理请求，请等待这一轮完成。",
        "pending.exists" => "The selected target already has a request in progress. Wait for it to finish first.",
        "pad.offline" if zh => "pad 当前未运行，无法派发 prompt。",
        "pad.offline" => "pad is not running, so the prompt can't be dispatched.",
        "agent.busy" if zh => "该 agent 当前正忙，请等待本轮结束后再发送。",
        "agent.busy" => "That agent is busy. Wait for the current turn to finish first.",
        "agent.waiting" if zh => "该 agent 当前正在等待确认，请先处理这条确认请求。",
        "agent.waiting" => "That agent is waiting for confirmation. Resolve it before sending a new prompt.",
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
        "status.none" if zh => "未选择",
        "status.none" => "none",
        "status.pending_none" if zh => "无",
        "status.pending_none" => "none",
        "status.pad" if zh => "Pad",
        "status.pad" => "Pad",
        "status.target" if zh => "目标",
        "status.target" => "Target",
        "status.pending" if zh => "请求",
        "status.pending" => "Pending",
        "slash.sent" if zh => "已发送 {} 到 {}。",
        "slash.sent" => "Sent {} to {}.",
        "slash.output" if zh => "已发送 {} 到 {}\n\n{}",
        "slash.output" => "Sent {} to {}\n\n{}",
        "stop.sent" if zh => "已向 {} 发送 Escape。",
        "stop.sent" => "Sent Escape to {}.",
        "stop.failed" if zh => "发送中断失败：{}",
        "stop.failed" => "Failed to send interrupt: {}",
        "target.switched" if zh => "当前目标已切换为：{}",
        "target.switched" => "Current target switched to: {}",
        "list.empty" if zh => "当前没有检测到可用的 agent pane。",
        "list.empty" => "No agent pane is currently available.",
        "timeout" if zh => "请求超时：{}。请回 pad 检查当前 pane 状态。",
        "timeout" => "Request timed out: {}. Check the pane in pad.",
        "result.missing" if zh => "任务结束，但未拿到回答正文，请回 pad 查看详细内容。",
        "result.missing" => "The task finished, but no answer text was captured. Check pad for details.",
        "result.completed" if zh => "完成：{}\n\n{}",
        "result.completed" => "Completed: {}\n\n{}",
        "result.title" if zh => "完成",
        "result.title" => "Completed",
        "meta.request" if zh => "请求",
        "meta.request" => "Request",
        "meta.target" if zh => "目标",
        "meta.target" => "Target",
        "meta.pane" if zh => "Pane",
        "meta.pane" => "Pane",
        "meta.session" if zh => "Session",
        "meta.session" => "Session",
        "meta.turn" if zh => "Turn",
        "meta.turn" => "Turn",
        "meta.dir" if zh => "目录",
        "meta.dir" => "Dir",
        "phase.awaiting_submit" if zh => "等待提交",
        "phase.awaiting_submit" => "Waiting for submit",
        "phase.awaiting_confirm" if zh => "等待确认",
        "phase.awaiting_confirm" => "Needs approval",
        "phase.accepted" if zh => "已提交",
        "phase.accepted" => "Submitted",
        "phase.working" if zh => "进行中 · {}s",
        "phase.working" => "Working · {}s",
        "phase.delivering" if zh => "发送结果中",
        "phase.delivering" => "Delivering result",
        "phase.completed" if zh => "已完成",
        "phase.completed" => "Completed",
        "typing.action" => "typing",
        "command.start" if zh => "绑定当前聊天并显示欢迎信息",
        "command.start" => "Link the current chat and show welcome text",
        "command.help" if zh => "查看可用命令",
        "command.help" => "Show available commands",
        "command.list" if zh => "列出可点击的 agent pane",
        "command.list" => "List clickable agent panes",
        "command.use" if zh => "按编号选择目标 agent",
        "command.use" => "Select the target agent by number",
        "command.status" if zh => "查看当前 Codex 会话状态",
        "command.status" => "Show the selected Codex session status",
        "command.fast" if zh => "切换或查看 Codex Fast mode",
        "command.fast" => "Toggle or inspect Codex Fast mode",
        "command.compact" if zh => "压缩当前 Codex 对话上下文",
        "command.compact" => "Compact the current Codex conversation",
        "command.stop" if zh => "尝试中断当前 agent",
        "command.stop" => "Try to interrupt the current agent",
        _ => key,
    }
}

pub(super) fn tg_fmt(
    locale: crate::i18n::Locale,
    key: &str,
    arg: impl std::fmt::Display,
) -> String {
    tg(locale, key).replacen("{}", &arg.to_string(), 1)
}

pub(super) fn tg_fmt2(
    locale: crate::i18n::Locale,
    key: &str,
    arg1: impl std::fmt::Display,
    arg2: impl std::fmt::Display,
) -> String {
    tg(locale, key)
        .replacen("{}", &arg1.to_string(), 1)
        .replacen("{}", &arg2.to_string(), 1)
}

pub(super) fn tg_fmt3(
    locale: crate::i18n::Locale,
    key: &str,
    arg1: impl std::fmt::Display,
    arg2: impl std::fmt::Display,
    arg3: impl std::fmt::Display,
) -> String {
    tg(locale, key)
        .replacen("{}", &arg1.to_string(), 1)
        .replacen("{}", &arg2.to_string(), 1)
        .replacen("{}", &arg3.to_string(), 1)
}
