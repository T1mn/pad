#![allow(dead_code)]

use ratatui::text::Line;

#[derive(Clone, Debug, Default)]
pub struct SelectionItem {
    pub title: String,
    pub subtitle: Option<String>,
    pub keyword: Option<String>,
    pub detail: Option<SelectionDetail>,
    pub disabled: bool,
}

#[derive(Clone, Debug, Default)]
pub struct SelectionDetail {
    pub title: Option<String>,
    pub body: Vec<Line<'static>>,
}

impl SelectionItem {
    pub fn matches_query(&self, query: &str) -> bool {
        let query = query.trim();
        if query.is_empty() {
            return true;
        }
        let query = query.to_lowercase();
        self.title.to_lowercase().contains(&query)
            || self
                .subtitle
                .as_ref()
                .is_some_and(|value| value.to_lowercase().contains(&query))
            || self
                .keyword
                .as_ref()
                .is_some_and(|value| value.to_lowercase().contains(&query))
    }
}
