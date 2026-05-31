use crate::sidebar::SidebarItem;
use std::ops::Range;

pub(crate) fn render_window<F>(
    len: usize,
    selected: Option<usize>,
    viewport_height: usize,
    mut row_height: F,
) -> Range<usize>
where
    F: FnMut(usize) -> usize,
{
    if len == 0 || viewport_height == 0 {
        return 0..0;
    }

    let selected = selected.unwrap_or(0).min(len - 1);
    let target_before = viewport_height / 2;
    let mut start = selected;
    let mut before = 0usize;

    while start > 0 {
        let height = row_height(start - 1).max(1);
        if before + height > target_before {
            break;
        }
        start -= 1;
        before += height;
    }

    let mut end = selected + 1;
    let mut used = before + row_height(selected).max(1);
    while end < len {
        let height = row_height(end).max(1);
        if used + height > viewport_height {
            break;
        }
        used += height;
        end += 1;
    }

    while start > 0 && used < viewport_height {
        let height = row_height(start - 1).max(1);
        if used + height > viewport_height {
            break;
        }
        start -= 1;
        used += height;
    }

    start..end
}

pub(crate) fn next_jump_badge_for_start(items: &[SidebarItem], start: usize) -> usize {
    items
        .iter()
        .take(start)
        .filter(|item| item.as_thread().is_some())
        .count()
        + 1
}

pub(crate) fn jump_badge_for_item(
    item: &SidebarItem,
    next_jump_badge: &mut usize,
) -> Option<usize> {
    item.as_thread()?;
    let badge = (*next_jump_badge <= 9).then_some(*next_jump_badge);
    *next_jump_badge += 1;
    badge
}

pub(crate) fn item_row_height(item: &SidebarItem) -> usize {
    match item {
        SidebarItem::Folder(_) => 1,
        SidebarItem::Thread(_) => 2,
    }
}

#[cfg(test)]
pub(crate) fn visible_thread_jump_badges(items: &[SidebarItem]) -> Vec<Option<usize>> {
    let mut next_jump_badge = 1usize;
    items
        .iter()
        .map(|item| jump_badge_for_item(item, &mut next_jump_badge))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::render_window;

    #[test]
    fn keeps_selected_near_middle_when_possible() {
        let range = render_window(20, Some(10), 5, |_| 1);

        assert_eq!(range, 8..13);
    }

    #[test]
    fn fills_from_top_when_selection_is_near_start() {
        let range = render_window(20, Some(1), 5, |_| 1);

        assert_eq!(range, 0..5);
    }

    #[test]
    fn respects_tall_thread_rows() {
        let range = render_window(20, Some(5), 6, |idx| if idx % 2 == 0 { 1 } else { 2 });

        assert!(range.contains(&5));
        let total_height: usize = range
            .clone()
            .map(|idx| if idx % 2 == 0 { 1 } else { 2 })
            .sum();
        assert!(total_height <= 6);
    }
}
