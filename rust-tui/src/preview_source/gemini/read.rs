use serde_json::Value;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub(super) fn read_transcript_value(path: &Path) -> Result<Value, String> {
    let file = File::open(path).map_err(|err| err.to_string())?;
    let mut reader = BufReader::new(file);
    let mut text = String::new();
    reader
        .read_to_string(&mut text)
        .map_err(|err| err.to_string())?;

    serde_json::from_str(&text).map_err(|err| err.to_string())
}
