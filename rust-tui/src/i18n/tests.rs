use super::{t, Locale};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn settings_on_is_defined_for_all_locales() {
    assert_eq!(t(Locale::ZhCN, "settings.on"), "开");
    assert_eq!(t(Locale::ZhTW, "settings.on"), "開");
    assert_eq!(t(Locale::En, "settings.on"), "On");
    assert_eq!(t(Locale::Ja, "settings.on"), "オン");
    assert_eq!(t(Locale::De, "settings.on"), "Ein");
    assert_eq!(t(Locale::Fr, "settings.on"), "Activé");
}

#[test]
fn all_static_i18n_keys_are_defined() {
    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let defined = parse_defined_i18n_keys(&src_dir.join("i18n"));

    let mut missing = Vec::new();
    for path in rust_files_under(&src_dir) {
        let source = fs::read_to_string(&path).expect("read source");
        for key in extract_static_i18n_keys(&source) {
            if !defined.contains(key.as_str()) {
                missing.push(format!("{} :: {}", path.display(), key));
            }
        }
    }

    missing.sort();
    missing.dedup();

    assert!(
        missing.is_empty(),
        "missing i18n keys:\n{}",
        missing.join("\n")
    );
}

fn rust_files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_files(dir, &mut files);
    files
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).expect("read dir");
    for entry in entries {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn parse_defined_i18n_keys(dir: &Path) -> HashSet<String> {
    let mut keys = HashSet::new();
    for path in rust_files_under(dir) {
        let source = fs::read_to_string(path).expect("read i18n source");
        let bytes = source.as_bytes();
        let mut idx = 0usize;

        while idx < bytes.len() {
            if bytes[idx] != b'"' {
                idx += 1;
                continue;
            }
            let Some(end) = find_string_end(bytes, idx + 1) else {
                break;
            };
            let token = &source[idx + 1..end];
            let after = source[end + 1..].trim_start();
            if after.starts_with("=>") {
                keys.insert(token.to_string());
            }
            idx = end + 1;
        }
    }
    keys
}

fn extract_static_i18n_keys(source: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let bytes = source.as_bytes();
    let mut idx = 0usize;
    let allow_bare_t = source.contains("use crate::i18n::{t,")
        || source.contains("use crate::i18n::{")
            && (source.contains("{t,") || source.contains(", t,") || source.contains(", t}"))
        || source.contains("use super::{t,");

    while idx < bytes.len() {
        let matched = if bytes[idx..].starts_with(b"crate::i18n::t(") {
            Some("crate::i18n::t(".len())
        } else if allow_bare_t
            && bytes[idx..].starts_with(b"t(")
            && idx > 0
            && !is_identifier_byte(bytes[idx - 1])
        {
            Some("t(".len())
        } else {
            None
        };

        let Some(needle_len) = matched else {
            idx += 1;
            continue;
        };

        let call_start = idx + needle_len;
        if let Some((key, advance)) = parse_i18n_call_key(&source[call_start..]) {
            keys.push(key);
            idx = call_start + advance;
        } else {
            idx = call_start;
        }
    }

    keys
}

fn parse_i18n_call_key(source: &str) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut idx = 0usize;

    while idx < bytes.len() {
        match bytes[idx] {
            b'(' => depth += 1,
            b')' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
            }
            b',' if depth == 0 => break,
            b'"' => idx = find_string_end(bytes, idx + 1)?,
            _ => {}
        }
        idx += 1;
    }

    if idx >= bytes.len() || bytes[idx] != b',' {
        return None;
    }

    let mut key_start = idx + 1;
    while key_start < bytes.len() && bytes[key_start].is_ascii_whitespace() {
        key_start += 1;
    }
    if bytes.get(key_start) != Some(&b'"') {
        return None;
    }

    let key_end = find_string_end(bytes, key_start + 1)?;
    Some((source[key_start + 1..key_end].to_string(), key_end + 1))
}

fn find_string_end(bytes: &[u8], mut idx: usize) -> Option<usize> {
    while idx < bytes.len() {
        match bytes[idx] {
            b'\\' => idx += 2,
            b'"' => return Some(idx),
            _ => idx += 1,
        }
    }
    None
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}
