use super::model::TranscriptContent;
use crate::preview_source::turns::SessionRole;

pub(super) fn extract_message_text_from_items(
    content: &[TranscriptContent<'_>],
    role: SessionRole,
) -> (SessionRole, String) {
    if role == SessionRole::User {
        let text = extract_codex_user_message_text_from_items(content);
        if let Some(summary) = super::super::subagent::extract_subagent_notification_summary(&text)
        {
            return (SessionRole::Assistant, summary);
        }
        return (role, text);
    }

    (role, join_message_text_from_items(content, "output_text"))
}

fn join_message_text_from_items(content: &[TranscriptContent<'_>], target_type: &str) -> String {
    let mut out = String::new();
    for item in content {
        if item.kind.as_deref() == Some(target_type) {
            if let Some(text) = item
                .text
                .as_deref()
                .map(str::trim)
                .filter(|text| !text.is_empty())
            {
                push_joined_part(&mut out, text);
            }
        }
    }
    out
}

fn extract_codex_user_message_text_from_items(content: &[TranscriptContent<'_>]) -> String {
    let mut image_count = 0usize;
    let mut text = String::new();

    for item in content {
        match item.kind.as_deref() {
            Some("input_image") => image_count += 1,
            Some("input_text") => {
                if let Some(part) = item
                    .text
                    .as_deref()
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                {
                    push_joined_part(&mut text, part);
                }
            }
            _ => {}
        }
    }

    super::super::normalize_codex_user_text(&text, Some(image_count))
}

fn push_joined_part(out: &mut String, part: &str) {
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(part);
}
