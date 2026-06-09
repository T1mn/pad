use super::{FileSearch, SearchItem};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

pub(super) fn initial_filter(items: &[SearchItem]) -> Vec<(usize, u32)> {
    let mut filtered = Vec::with_capacity(items.len());
    fill_initial_filter(&mut filtered, items.len());
    filtered
}

pub(super) fn update_filter(search: &mut FileSearch) {
    if search.query.is_empty() {
        fill_initial_filter(&mut search.filtered, search.items.len());
    } else {
        search.filtered = fuzzy_filter(&search.items, &search.query);
    }

    if search.selected >= search.filtered.len() {
        search.selected = search.filtered.len().saturating_sub(1);
    }
}

fn fill_initial_filter(filtered: &mut Vec<(usize, u32)>, len: usize) {
    filtered.clear();
    filtered.extend((0..len).map(|index| (index, 0)));
}

fn fuzzy_filter(items: &[SearchItem], query: &str) -> Vec<(usize, u32)> {
    let mut matcher = Matcher::default();
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let mut buf = Vec::new();
    let mut filtered = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        buf.clear();
        let utf32 = Utf32Str::new(&item.relative, &mut buf);
        if let Some(score) = pattern.score(utf32, &mut matcher) {
            filtered.push((index, score));
        }
    }
    filtered.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    filtered
}
