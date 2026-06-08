use super::{FileSearch, SearchItem};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

pub(super) fn initial_filter(items: &[SearchItem]) -> Vec<(usize, u32)> {
    (0..items.len()).map(|index| (index, 0)).collect()
}

pub(super) fn update_filter(search: &mut FileSearch) {
    if search.query.is_empty() {
        search.filtered = initial_filter(&search.items);
    } else {
        search.filtered = fuzzy_filter(&search.items, &search.query);
    }

    if search.selected >= search.filtered.len() {
        search.selected = search.filtered.len().saturating_sub(1);
    }
}

fn fuzzy_filter(items: &[SearchItem], query: &str) -> Vec<(usize, u32)> {
    let mut matcher = Matcher::default();
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let mut buf = Vec::new();
    let mut filtered = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            buf.clear();
            let utf32 = Utf32Str::new(&item.relative, &mut buf);
            pattern
                .score(utf32, &mut matcher)
                .map(|score| (index, score))
        })
        .collect::<Vec<_>>();
    filtered.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    filtered
}
