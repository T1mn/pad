use serde::Deserialize;
use std::borrow::Cow;

#[derive(Deserialize)]
pub(super) struct TranscriptLine<'a> {
    #[serde(rename = "type", borrow)]
    pub(super) event_type: Option<Cow<'a, str>>,
    #[serde(borrow)]
    pub(super) payload: Option<TranscriptPayload<'a>>,
}

#[derive(Deserialize)]
pub(super) struct TranscriptPayload<'a> {
    #[serde(rename = "type", borrow)]
    pub(super) kind: Option<Cow<'a, str>>,
    #[serde(borrow)]
    pub(super) role: Option<Cow<'a, str>>,
    #[serde(borrow)]
    pub(super) name: Option<Cow<'a, str>>,
    #[serde(borrow)]
    pub(super) arguments: Option<Cow<'a, str>>,
    #[serde(borrow)]
    pub(super) content: Option<Vec<TranscriptContent<'a>>>,
}

#[derive(Deserialize)]
pub(super) struct TranscriptContent<'a> {
    #[serde(rename = "type", borrow)]
    pub(super) kind: Option<Cow<'a, str>>,
    #[serde(borrow)]
    pub(super) text: Option<Cow<'a, str>>,
}
