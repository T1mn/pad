const OUTER_PAD: usize = 1;
const CARD_LEFT_PAD: usize = 2;
const CARD_RIGHT_PAD: usize = 1;

pub(super) const JUMP_BADGE_SLOT_WIDTH: usize = 4;

pub(super) struct ThreadRowLayout {
    pub(super) is_minimal: bool,
    pub(super) inner_card_width: usize,
}

pub(super) fn thread_row_layout(content_width: usize) -> ThreadRowLayout {
    let card_width = content_width.saturating_sub(OUTER_PAD * 2);
    ThreadRowLayout {
        is_minimal: content_width < 12,
        inner_card_width: card_width.saturating_sub(CARD_LEFT_PAD + CARD_RIGHT_PAD),
    }
}
