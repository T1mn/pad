use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

pub(super) fn filter_items(items: &[String], query: &str) -> Vec<(String, u32)> {
    if query.is_empty() {
        return items.iter().map(|s| (s.clone(), 0)).collect();
    }

    let mut matcher = Matcher::default();
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let mut buf = Vec::new();
    let mut results: Vec<(String, u32)> = items
        .iter()
        .filter_map(|item| {
            buf.clear();
            let utf32_str = Utf32Str::new(item, &mut buf);
            pattern
                .score(utf32_str, &mut matcher)
                .map(|score| (item.clone(), score))
        })
        .collect();

    results.sort_by_key(|entry| std::cmp::Reverse(entry.1));
    results
}
