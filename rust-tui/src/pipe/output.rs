// This parser is staged ahead of the tmux terminal transport which will
// consume it. Remove this allowance when that integration lands.
#![allow(dead_code)]

use std::error::Error;
use std::fmt;

const OUTPUT: &[u8] = b"%output";
const EXTENDED_OUTPUT: &[u8] = b"%extended-output";

/// Decoded bytes from a tmux control-mode output notification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ParsedOutput {
    pub(crate) pane_id: String,
    pub(crate) bytes: Vec<u8>,
}

/// A recognized output notification which does not match tmux's wire format.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum OutputParseError {
    ExpectedSpaceAfterNotification { offset: usize },
    MissingPaneId { offset: usize },
    InvalidPaneId { offset: usize },
    MissingSpaceAfterPaneId { offset: usize },
    MissingAge { offset: usize },
    InvalidAge { offset: usize },
    MissingSpaceAfterAge { offset: usize },
    MissingExtendedDelimiter { offset: usize },
    EmptyExtendedArgument { offset: usize },
    MissingSpaceAfterExtendedDelimiter { offset: usize },
    IncompleteOctalEscape { offset: usize },
    InvalidOctalDigit { offset: usize },
    OctalEscapeOutOfRange { offset: usize },
    NonPrintableLiteral { offset: usize, byte: u8 },
}

impl fmt::Display for OutputParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedSpaceAfterNotification { offset } => {
                write!(
                    formatter,
                    "expected a space after notification at byte {offset}"
                )
            }
            Self::MissingPaneId { offset } => {
                write!(formatter, "missing pane id at byte {offset}")
            }
            Self::InvalidPaneId { offset } => {
                write!(formatter, "invalid pane id at byte {offset}")
            }
            Self::MissingSpaceAfterPaneId { offset } => {
                write!(formatter, "missing space after pane id at byte {offset}")
            }
            Self::MissingAge { offset } => {
                write!(formatter, "missing extended-output age at byte {offset}")
            }
            Self::InvalidAge { offset } => {
                write!(formatter, "invalid extended-output age at byte {offset}")
            }
            Self::MissingSpaceAfterAge { offset } => {
                write!(
                    formatter,
                    "missing space after extended-output age at byte {offset}"
                )
            }
            Self::MissingExtendedDelimiter { offset } => {
                write!(
                    formatter,
                    "missing extended-output ':' delimiter at byte {offset}"
                )
            }
            Self::EmptyExtendedArgument { offset } => {
                write!(formatter, "empty extended-output argument at byte {offset}")
            }
            Self::MissingSpaceAfterExtendedDelimiter { offset } => write!(
                formatter,
                "missing space after extended-output ':' delimiter at byte {offset}"
            ),
            Self::IncompleteOctalEscape { offset } => {
                write!(formatter, "incomplete octal escape at byte {offset}")
            }
            Self::InvalidOctalDigit { offset } => {
                write!(formatter, "invalid octal digit at byte {offset}")
            }
            Self::OctalEscapeOutOfRange { offset } => {
                write!(
                    formatter,
                    "octal escape exceeds byte range at byte {offset}"
                )
            }
            Self::NonPrintableLiteral { offset, byte } => write!(
                formatter,
                "unescaped non-printable byte 0x{byte:02x} at byte {offset}"
            ),
        }
    }
}

impl Error for OutputParseError {}

/// Parse `%output` and `%extended-output` notifications from tmux control mode.
///
/// The input is one protocol line without its terminating newline. Unknown
/// notifications return `Ok(None)`. Once an output notification is recognized,
/// malformed fields or payload escaping are reported as errors rather than
/// silently accepted.
pub(crate) fn parse_output_notification(
    line: &[u8],
) -> Result<Option<ParsedOutput>, OutputParseError> {
    let notification = line
        .split(|byte| byte.is_ascii_whitespace())
        .next()
        .unwrap_or_default();

    match notification {
        OUTPUT => parse_output(line).map(Some),
        EXTENDED_OUTPUT => parse_extended_output(line).map(Some),
        _ => Ok(None),
    }
}

fn parse_output(line: &[u8]) -> Result<ParsedOutput, OutputParseError> {
    let (pane_id, payload, payload_offset) = parse_pane_and_rest(line, OUTPUT)?;
    let bytes = decode_payload(payload, payload_offset)?;
    Ok(ParsedOutput { pane_id, bytes })
}

fn parse_extended_output(line: &[u8]) -> Result<ParsedOutput, OutputParseError> {
    let (pane_id, rest, mut rest_offset) = parse_pane_and_rest(line, EXTENDED_OUTPUT)?;

    if rest.is_empty() {
        return Err(OutputParseError::MissingAge {
            offset: rest_offset,
        });
    }
    let Some(age_end) = rest.iter().position(|byte| *byte == b' ') else {
        return Err(OutputParseError::MissingSpaceAfterAge { offset: line.len() });
    };
    let age = &rest[..age_end];
    if age.is_empty() {
        return Err(OutputParseError::MissingAge {
            offset: rest_offset,
        });
    }
    if !age.iter().all(u8::is_ascii_digit) {
        return Err(OutputParseError::InvalidAge {
            offset: rest_offset,
        });
    }

    let mut arguments = &rest[age_end + 1..];
    rest_offset += age_end + 1;
    loop {
        if arguments.is_empty() {
            return Err(OutputParseError::MissingExtendedDelimiter {
                offset: rest_offset,
            });
        }

        let Some(argument_end) = arguments.iter().position(|byte| *byte == b' ') else {
            if arguments == b":" {
                return Err(OutputParseError::MissingSpaceAfterExtendedDelimiter {
                    offset: rest_offset + 1,
                });
            }
            return Err(OutputParseError::MissingExtendedDelimiter {
                offset: rest_offset + arguments.len(),
            });
        };
        let argument = &arguments[..argument_end];
        if argument.is_empty() {
            return Err(OutputParseError::EmptyExtendedArgument {
                offset: rest_offset,
            });
        }

        if argument == b":" {
            let payload = &arguments[argument_end + 1..];
            let payload_offset = rest_offset + argument_end + 1;
            let bytes = decode_payload(payload, payload_offset)?;
            return Ok(ParsedOutput { pane_id, bytes });
        }

        arguments = &arguments[argument_end + 1..];
        rest_offset += argument_end + 1;
    }
}

fn parse_pane_and_rest<'a>(
    line: &'a [u8],
    notification: &[u8],
) -> Result<(String, &'a [u8], usize), OutputParseError> {
    let separator_offset = notification.len();
    if line.get(separator_offset) != Some(&b' ') {
        return Err(OutputParseError::ExpectedSpaceAfterNotification {
            offset: separator_offset,
        });
    }

    let fields = &line[separator_offset + 1..];
    let fields_offset = separator_offset + 1;
    if fields.is_empty() {
        return Err(OutputParseError::MissingPaneId {
            offset: fields_offset,
        });
    }
    let Some(pane_end) = fields.iter().position(|byte| *byte == b' ') else {
        return Err(OutputParseError::MissingSpaceAfterPaneId { offset: line.len() });
    };
    let pane_id = &fields[..pane_end];
    if !valid_pane_id(pane_id) {
        return Err(OutputParseError::InvalidPaneId {
            offset: fields_offset,
        });
    }

    // Pane IDs have a leading '%' and contain only ASCII digits, so this
    // conversion cannot fail after validation.
    let pane_id = String::from_utf8(pane_id.to_vec()).expect("validated ASCII pane id");
    let rest_offset = fields_offset + pane_end + 1;
    Ok((pane_id, &fields[pane_end + 1..], rest_offset))
}

fn valid_pane_id(pane_id: &[u8]) -> bool {
    pane_id.len() > 1 && pane_id[0] == b'%' && pane_id[1..].iter().all(u8::is_ascii_digit)
}

fn decode_payload(payload: &[u8], payload_offset: usize) -> Result<Vec<u8>, OutputParseError> {
    let mut decoded = Vec::with_capacity(payload.len());
    let mut literal_start = 0;
    let mut index = 0;

    while index < payload.len() {
        if payload[index] != b'\\' {
            index += 1;
            continue;
        }

        decode_literal(
            &payload[literal_start..index],
            payload_offset + literal_start,
            &mut decoded,
        )?;
        if payload.len() - index < 4 {
            return Err(OutputParseError::IncompleteOctalEscape {
                offset: payload_offset + index,
            });
        }

        let digits = &payload[index + 1..index + 4];
        let Some(first) = octal_digit(digits[0]) else {
            return Err(OutputParseError::InvalidOctalDigit {
                offset: payload_offset + index + 1,
            });
        };
        let Some(second) = octal_digit(digits[1]) else {
            return Err(OutputParseError::InvalidOctalDigit {
                offset: payload_offset + index + 2,
            });
        };
        let Some(third) = octal_digit(digits[2]) else {
            return Err(OutputParseError::InvalidOctalDigit {
                offset: payload_offset + index + 3,
            });
        };
        if first > 3 {
            return Err(OutputParseError::OctalEscapeOutOfRange {
                offset: payload_offset + index,
            });
        }

        decoded.push((first << 6) | (second << 3) | third);
        index += 4;
        literal_start = index;
    }

    decode_literal(
        &payload[literal_start..],
        payload_offset + literal_start,
        &mut decoded,
    )?;
    Ok(decoded)
}

fn decode_literal(
    literal: &[u8],
    literal_offset: usize,
    decoded: &mut Vec<u8>,
) -> Result<(), OutputParseError> {
    if let Some((index, byte)) = literal
        .iter()
        .copied()
        .enumerate()
        .find(|(_, byte)| *byte < b' ')
    {
        return Err(OutputParseError::NonPrintableLiteral {
            offset: literal_offset + index,
            byte,
        });
    }

    decoded.extend_from_slice(literal);
    Ok(())
}

fn octal_digit(byte: u8) -> Option<u8> {
    byte.checked_sub(b'0').filter(|digit| *digit < 8)
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
