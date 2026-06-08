pub(super) fn text(key: &str, zh: bool) -> Option<&'static str> {
    Some(match key {
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
            "Pad Telegram 已就绪。用 /list 查看并点击目标，或用 /use <编号> 选择；/history 查看最近 3 条问答，/diag 查看当前连续性诊断，/status 查看 Codex 状态，/fast 切换 Fast mode，/compact 压缩上下文，/padstatus 查看 bot 当前状态，异常占位时可用 /reset 清掉当前目标的 pending，/restart 可远程重编译并重启整个 pad。"
        }
        "start.ready" => {
            "Pad Telegram is ready. Use /list to pick a target, or /use <number>; /history shows the latest three turns, /diag shows session continuity diagnostics, /status shows Codex status, /fast toggles Fast mode, /compact compacts context, /padstatus shows bot state, /reset clears the current target's stuck pending state, and /restart rebuilds and restarts the whole pad."
        }
        "help.body" if zh => {
            "/start\n/help\n/list\n/agents\n/use <编号>\n/history\n/diag [request_id|session_id|pane_id]\n/status\n/fast [on|off|status]\n/compact\n/padstatus\n/restart\n/reset\n/stop\n\n选择目标后，直接发送普通文本即可。"
        }
        "help.body" => {
            "/start\n/help\n/list\n/agents\n/use <number>\n/history\n/diag [request_id|session_id|pane_id]\n/status\n/fast [on|off|status]\n/compact\n/padstatus\n/restart\n/reset\n/stop\n\nAfter selecting a target, just send plain text."
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
        "restart.preparing" if zh => "开始重启当前 pad。接下来会重编译并在原 pane 内重新拉起，Telegram 可能会短暂不可用。",
        "restart.preparing" => "Starting a full pad restart. It will rebuild and relaunch in the current pane, and Telegram may be briefly unavailable.",
        "restart.starting" if zh => "开始启动 pad。接下来会重编译并在 tmux session 里拉起一个新的 pad。",
        "restart.starting" => "Starting pad. It will rebuild and launch a new pad inside the tmux session.",
        "restart.failed" if zh => "Pad 重启失败：{}",
        "restart.failed" => "Pad restart failed: {}",
        "slash.sent" if zh => "已发送 {} 到 {}。",
        "slash.sent" => "Sent {} to {}.",
        "slash.output" if zh => "已发送 {} 到 {}\n\n{}",
        "slash.output" => "Sent {} to {}\n\n{}",
        "stop.sent" if zh => "已向 {} 发送 Escape。",
        "stop.sent" => "Sent Escape to {}.",
        "stop.failed" if zh => "发送中断失败：{}",
        "stop.failed" => "Failed to send interrupt: {}",
        "reset.none" if zh => "{} 当前没有待处理请求。",
        "reset.none" => "{} has no pending request to clear.",
        "reset.status" if zh => "已重置",
        "reset.status" => "Reset",
        "reset.done" if zh => {
            "已重置 Telegram pending：{}（{}）。这不会停止 pane 内正在运行的 agent。"
        }
        "reset.done" => {
            "Cleared Telegram pending request {} for {}. This does not stop the agent running in the pane."
        }
        "target.switched" if zh => "当前目标已切换为：{}",
        "target.switched" => "Current target switched to: {}",
        "list.empty" if zh => "当前没有检测到可用的 agent pane。",
        "list.empty" => "No agent pane is currently available.",
        _ => return None,
    })
}
