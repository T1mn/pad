use super::App;

const AGENT_PANEL_WIDTH_STEP: u16 = 6;
const MAX_AGENT_PANEL_WIDTH: u16 = 90;

impl App {
    pub fn widen_agent_panel_width(&mut self, current_width: u16) {
        let next = current_width
            .saturating_add(AGENT_PANEL_WIDTH_STEP)
            .min(MAX_AGENT_PANEL_WIDTH);
        self.config.display.agent_panel_width = Some(next);
        self.sidebar.preferred_panel_width_cache = None;
        self.config.save();
        self.show_action_toast(
            panel_width_toast_title(self.locale),
            &panel_width_toast_body(self.locale, next),
        );
        self.dirty = true;
    }
}

fn panel_width_toast_title(locale: crate::i18n::Locale) -> &'static str {
    match locale {
        crate::i18n::Locale::ZhCN => "左侧宽度已保存",
        crate::i18n::Locale::ZhTW => "左側寬度已儲存",
        _ => "Sidebar width saved",
    }
}

fn panel_width_toast_body(locale: crate::i18n::Locale, width: u16) -> String {
    match locale {
        crate::i18n::Locale::ZhCN => format!("Agent 列表宽度：{width}"),
        crate::i18n::Locale::ZhTW => format!("Agent 列表寬度：{width}"),
        _ => format!("Agent list width: {width}"),
    }
}
