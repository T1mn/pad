use super::super::super::common::is_cjk_locale;
use crate::app::ThreadActionKind;
use crate::i18n::Locale;
use crate::model::AgentType;

pub(in crate::ui::modals::dialogs) fn thread_action_modal_title(
    locale: Locale,
    kind: ThreadActionKind,
) -> &'static str {
    match (is_cjk_locale(locale), kind) {
        (true, ThreadActionKind::Archive) => "归档会话",
        (true, ThreadActionKind::Unarchive) => "恢复会话",
        (true, ThreadActionKind::Restore) => "恢复线程",
        (false, ThreadActionKind::Archive) => "Archive Thread",
        (false, ThreadActionKind::Unarchive) => "Restore Thread",
        (false, ThreadActionKind::Restore) => "Restore Thread",
    }
}

pub(in crate::ui::modals::dialogs) fn thread_action_modal_confirm(
    locale: Locale,
    kind: ThreadActionKind,
    agent_type: AgentType,
) -> String {
    match (is_cjk_locale(locale), kind, agent_type) {
        (true, ThreadActionKind::Archive, AgentType::Codex) => "确认归档这个 Codex 会话？".into(),
        (true, ThreadActionKind::Unarchive, AgentType::Codex) => "确认恢复这个 Codex 会话？".into(),
        (true, ThreadActionKind::Restore, AgentType::Codex) => {
            "确认从回收站恢复这个 Codex 会话？".into()
        }
        (true, ThreadActionKind::Archive, AgentType::Claude) => "确认归档这个 Claude 会话？".into(),
        (true, ThreadActionKind::Unarchive, AgentType::Claude) => {
            "确认恢复这个 Claude 会话？".into()
        }
        (true, ThreadActionKind::Restore, AgentType::Claude) => {
            "确认从回收站恢复这个 Claude 会话？".into()
        }
        (true, ThreadActionKind::Archive, AgentType::Gemini) => {
            "确认仅在 pad 侧归档这个 Gemini 会话？不会修改 ~/.gemini。".into()
        }
        (true, ThreadActionKind::Unarchive, AgentType::Gemini) => {
            "确认从 pad 侧归档中恢复这个 Gemini 会话？不会修改 ~/.gemini。".into()
        }
        (true, ThreadActionKind::Restore, AgentType::Gemini) => {
            "确认从回收站恢复这个 Gemini 会话？不会修改 ~/.gemini。".into()
        }
        (false, ThreadActionKind::Archive, AgentType::Codex) => "Archive this Codex thread?".into(),
        (false, ThreadActionKind::Unarchive, AgentType::Codex) => {
            "Restore this Codex thread?".into()
        }
        (false, ThreadActionKind::Restore, AgentType::Codex) => {
            "Restore this Codex thread from trash?".into()
        }
        (false, ThreadActionKind::Archive, AgentType::Claude) => {
            "Archive this Claude thread?".into()
        }
        (false, ThreadActionKind::Unarchive, AgentType::Claude) => {
            "Restore this Claude thread?".into()
        }
        (false, ThreadActionKind::Restore, AgentType::Claude) => {
            "Restore this Claude thread from trash?".into()
        }
        (false, ThreadActionKind::Archive, AgentType::Gemini) => {
            "Archive this Gemini session in pad only? This does not modify ~/.gemini.".into()
        }
        (false, ThreadActionKind::Unarchive, AgentType::Gemini) => {
            "Restore this Gemini session from pad archive? This does not modify ~/.gemini.".into()
        }
        (false, ThreadActionKind::Restore, AgentType::Gemini) => {
            "Restore this Gemini session from trash? This does not modify ~/.gemini.".into()
        }
        (true, ThreadActionKind::Archive, _) => "确认归档这个会话？".into(),
        (true, ThreadActionKind::Unarchive, _) => "确认恢复这个会话？".into(),
        (true, ThreadActionKind::Restore, _) => "确认从回收站恢复这个会话？".into(),
        (false, ThreadActionKind::Archive, _) => "Archive this thread?".into(),
        (false, ThreadActionKind::Unarchive, _) => "Restore this thread?".into(),
        (false, ThreadActionKind::Restore, _) => "Restore this thread from trash?".into(),
    }
}

pub(in crate::ui::modals::dialogs) fn thread_action_live_warning(
    locale: Locale,
    agent_type: AgentType,
) -> &'static str {
    match (is_cjk_locale(locale), agent_type) {
        (true, AgentType::Codex) => "这个会话仍然绑定 live pane。归档只会隐藏 pad 中的线程，并同步修改 Codex 的 sqlite/jsonl；不会关闭 tmux pane 或进程。",
        (false, AgentType::Codex) => "This thread still has a live pane. Archiving only hides it in pad and updates Codex sqlite/jsonl. It does not kill the tmux pane or process.",
        (true, AgentType::Claude) => "这个会话仍然绑定 live pane。归档只会隐藏 pad 中的线程，并更新 pad 的 Claude sqlite 索引；不会关闭 tmux pane 或进程，也不会修改 ~/.claude 原始 jsonl。",
        (false, AgentType::Claude) => "This thread still has a live pane. Archiving only hides it in pad and updates pad's Claude sqlite index. It does not kill the tmux pane or process, and it does not modify the original ~/.claude jsonl.",
        (true, AgentType::Gemini) => "这个会话仍然绑定 live pane。Pad 侧归档只会隐藏 pad 中的条目，不会修改 ~/.gemini，也不会关闭 tmux pane 或进程。",
        (false, AgentType::Gemini) => "This thread still has a live pane. Pad-side archiving only hides it in pad. It does not modify ~/.gemini or kill the tmux pane/process.",
        (true, _) => "这个会话仍然绑定 live pane。归档不会关闭 tmux pane 或进程。",
        (false, _) => "This thread still has a live pane. Archiving does not kill the tmux pane or process.",
    }
}

pub(in crate::ui::modals::dialogs) fn thread_action_confirm_hint(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "按 'y' 确认"
    } else {
        "Press 'y' to confirm"
    }
}

pub(in crate::ui::modals::dialogs) fn thread_action_cancel_hint(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "任意键取消"
    } else {
        "Any other key cancels"
    }
}
