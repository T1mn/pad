use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub(in crate::pad_sider::ui::markdown) struct Renderer {
    pub(in crate::pad_sider::ui::markdown) lines: Vec<Line<'static>>,
    pub(in crate::pad_sider::ui::markdown) current: Vec<Span<'static>>,
    pub(in crate::pad_sider::ui::markdown) blockquote_depth: usize,
    pub(in crate::pad_sider::ui::markdown) lists: Vec<ListState>,
    pub(in crate::pad_sider::ui::markdown) item_prefix: Option<ItemPrefix>,
    pub(in crate::pad_sider::ui::markdown) heading_level: Option<u8>,
    pub(in crate::pad_sider::ui::markdown) link_depth: usize,
    pub(in crate::pad_sider::ui::markdown) emphasis_depth: usize,
    pub(in crate::pad_sider::ui::markdown) strong_depth: usize,
    pub(in crate::pad_sider::ui::markdown) strike_depth: usize,
    pub(in crate::pad_sider::ui::markdown) code_block: bool,
    pub(in crate::pad_sider::ui::markdown) code_language: Option<String>,
}

pub(in crate::pad_sider::ui::markdown) struct ListState {
    pub(in crate::pad_sider::ui::markdown) ordered: bool,
    pub(in crate::pad_sider::ui::markdown) next_index: u64,
}

pub(in crate::pad_sider::ui::markdown) struct ItemPrefix {
    pub(in crate::pad_sider::ui::markdown) first: String,
    pub(in crate::pad_sider::ui::markdown) continuation: String,
    pub(in crate::pad_sider::ui::markdown) used_first: bool,
}

impl Renderer {
    pub(in crate::pad_sider::ui::markdown) fn new() -> Self {
        Self {
            lines: Vec::new(),
            current: Vec::new(),
            blockquote_depth: 0,
            lists: Vec::new(),
            item_prefix: None,
            heading_level: None,
            link_depth: 0,
            emphasis_depth: 0,
            strong_depth: 0,
            strike_depth: 0,
            code_block: false,
            code_language: None,
        }
    }

    pub(in crate::pad_sider::ui::markdown) fn push_span(&mut self, text: String, style: Style) {
        self.ensure_prefix();
        self.current.push(Span::styled(text, style));
    }
}
