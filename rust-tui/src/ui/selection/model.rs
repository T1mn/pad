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

        contains_query_ignore_case(&self.title, query)
            || self
                .value
                .as_deref()
                .is_some_and(|value| contains_query_ignore_case(value, query))
            || self
                .subtitle
                .as_deref()
                .is_some_and(|value| contains_query_ignore_case(value, query))
            || self
                .keyword
                .as_deref()
                .is_some_and(|value| contains_query_ignore_case(value, query))
    }
}

fn contains_query_ignore_case(value: &str, query: &str) -> bool {
    if value.is_ascii() && query.is_ascii() {
        return value
            .as_bytes()
            .windows(query.len())
            .any(|window| window.eq_ignore_ascii_case(query.as_bytes()));
    }

    let query = query.to_lowercase();
    value.to_lowercase().contains(&query)
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
