#[derive(Clone, Debug, Default)]
pub struct SelectionItem {
    pub title: String,
    pub value: Option<String>,
    pub subtitle: Option<String>,
    pub keyword: Option<String>,
    pub disabled: bool,
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
                .value
                .as_ref()
                .is_some_and(|value| value.to_lowercase().contains(&query))
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

#[cfg(test)]
mod tests {
    use super::SelectionItem;

    #[test]
    fn matches_query_checks_value_text() {
        let item = SelectionItem {
            title: "Theme".into(),
            value: Some("dark".into()),
            ..Default::default()
        };

        assert!(item.matches_query("dark"));
    }
}
