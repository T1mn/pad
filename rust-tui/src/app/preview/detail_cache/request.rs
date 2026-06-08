use crate::app::{App, PreviewDetailRenderRequest};

impl App {
    pub fn current_preview_detail_request(&self) -> Option<PreviewDetailRenderRequest> {
        let selected = self.preview.expanded_turn?;
        let turn = self.preview.turns.get(selected)?;
        Some(PreviewDetailRenderRequest {
            target_key: self.preview.pane_id.clone().unwrap_or_default(),
            turns: self.preview.turns.clone(),
            turn_index: selected,
            width: 0,
            theme_name: self.theme.name.to_string(),
            question: turn.question.clone(),
            answer: turn.answer.clone(),
        })
    }
}
