use super::util::truncate_for_log;
use crate::model::PreviewTurn;

const MAX_ASSISTANT_SNIPPET_CHARS: usize = 300;

pub(super) fn build_summary_prompt(turns: &[PreviewTurn]) -> String {
    let mut prompt = String::from(
        "Generate one concise title for this coding conversation.\n\
Return exactly one plain-text line in the conversation's main language.\n\
Do not use quotes, markdown, prefixes, or explanations.\n\
Prefer 4-10 words when possible.\n\nConversation:\n",
    );

    for (idx, turn) in turns.iter().enumerate() {
        let turn_no = idx + 1;
        prompt.push_str(&format!("User {turn_no}: {}\n", turn.question.trim()));
        if let Some(answer) = turn
            .answer
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            prompt.push_str(&format!(
                "Assistant {turn_no}: {}\n",
                truncate_for_log(answer, MAX_ASSISTANT_SNIPPET_CHARS)
            ));
        }
    }

    prompt
}
