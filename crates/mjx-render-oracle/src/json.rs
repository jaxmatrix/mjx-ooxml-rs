//! Just enough JSON to **write the plate manifest and read it back**.
//!
//! # Why both halves
//!
//! Writing it is the easy half and the one a manifest needs. Reading it is what makes the manifest a
//! *contract*: `tests/the_plate_manifest_is_loadable.rs` writes the manifest, parses it back with
//! this reader, and asserts every field R11's Storybook loader will look for — from the consumer's
//! side. A generator checked by asserting on the string it just produced proves that `format!`
//! works. The question a hand-off has to answer is not *"is this produced?"* but *"does it reach
//! anyone?"*, and only a reader can answer that.
//!
//! # Why not a dependency
//!
//! `CLAUDE.md`'s hand-written-de/serialization decision, and the shape of this crate: nothing
//! depends on it, so a JSON crate here would be a dependency the workspace carries for a file
//! nobody ships. `xtask` reached the same conclusion for `cargo metadata` and wrote the same fifty
//! lines; this one is separate because nothing may depend on `xtask`.
//!
//! What is deliberately **not** here: `\u` escapes on the way out are only produced for the control
//! characters that require them, and numbers are written from Rust's own `Display`. Both are
//! sufficient for a manifest this crate writes and neither is a general JSON library.

use std::collections::BTreeMap;

/// A JSON value.
#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// Any number, as a `f64` — a manifest's numbers are counts and dimensions.
    Number(f64),
    /// A string, unescaped.
    String(String),
    /// An array.
    Array(Vec<Value>),
    /// An object. Ordered by key, so a reader's iteration is deterministic.
    Object(BTreeMap<String, Value>),
}

impl Value {
    /// The value at `key`, for an object.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(map) => map.get(key),
            _ => None,
        }
    }

    /// The string, for a string.
    #[must_use]
    pub fn string(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    /// The number, for a number.
    #[must_use]
    pub const fn number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }

    /// The boolean, for a boolean.
    #[must_use]
    pub const fn boolean(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// The elements, for an array.
    #[must_use]
    pub fn array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }
}

/// Parse `text`.
///
/// # Errors
///
/// A sentence naming the byte offset and what was expected there.
pub fn parse(text: &str) -> Result<Value, String> {
    let bytes = text.as_bytes();
    let mut cursor = 0usize;
    let value = parse_value(bytes, &mut cursor)?;
    skip_whitespace(bytes, &mut cursor);
    if cursor != bytes.len() {
        return Err(format!("trailing content at byte {cursor}"));
    }
    Ok(value)
}

fn skip_whitespace(bytes: &[u8], cursor: &mut usize) {
    while *cursor < bytes.len() && bytes[*cursor].is_ascii_whitespace() {
        *cursor += 1;
    }
}

fn parse_value(bytes: &[u8], cursor: &mut usize) -> Result<Value, String> {
    skip_whitespace(bytes, cursor);
    match bytes.get(*cursor) {
        None => Err("the document ends where a value was expected".to_owned()),
        Some(b'{') => parse_object(bytes, cursor),
        Some(b'[') => parse_array(bytes, cursor),
        Some(b'"') => Ok(Value::String(parse_string(bytes, cursor)?)),
        Some(b't') => literal(bytes, cursor, "true", Value::Bool(true)),
        Some(b'f') => literal(bytes, cursor, "false", Value::Bool(false)),
        Some(b'n') => literal(bytes, cursor, "null", Value::Null),
        Some(_) => parse_number(bytes, cursor),
    }
}

fn literal(bytes: &[u8], cursor: &mut usize, word: &str, value: Value) -> Result<Value, String> {
    if bytes[*cursor..].starts_with(word.as_bytes()) {
        *cursor += word.len();
        return Ok(value);
    }
    Err(format!("expected `{word}` at byte {cursor}"))
}

fn parse_number(bytes: &[u8], cursor: &mut usize) -> Result<Value, String> {
    let start = *cursor;
    while *cursor < bytes.len()
        && matches!(
            bytes[*cursor],
            b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
        )
    {
        *cursor += 1;
    }
    std::str::from_utf8(&bytes[start..*cursor])
        .ok()
        .and_then(|text| text.parse::<f64>().ok())
        .map(Value::Number)
        .ok_or_else(|| format!("expected a number at byte {start}"))
}

fn parse_string(bytes: &[u8], cursor: &mut usize) -> Result<String, String> {
    // The opening quote.
    *cursor += 1;
    let mut text = String::new();
    while let Some(byte) = bytes.get(*cursor) {
        *cursor += 1;
        match byte {
            b'"' => return Ok(text),
            b'\\' => {
                let escape = bytes
                    .get(*cursor)
                    .ok_or_else(|| format!("the document ends inside an escape at {cursor}"))?;
                *cursor += 1;
                match escape {
                    b'"' => text.push('"'),
                    b'\\' => text.push('\\'),
                    b'/' => text.push('/'),
                    b'b' => text.push('\u{8}'),
                    b'f' => text.push('\u{c}'),
                    b'n' => text.push('\n'),
                    b'r' => text.push('\r'),
                    b't' => text.push('\t'),
                    b'u' => {
                        let hex = bytes
                            .get(*cursor..*cursor + 4)
                            .and_then(|slice| std::str::from_utf8(slice).ok())
                            .ok_or_else(|| format!("a short `\\u` escape at byte {cursor}"))?;
                        *cursor += 4;
                        let code = u32::from_str_radix(hex, 16)
                            .map_err(|_| format!("`\\u{hex}` is not four hex digits"))?;
                        text.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                    }
                    other => return Err(format!("`\\{}` is not an escape", *other as char)),
                }
            }
            _ => {
                // Multi-byte UTF-8 arrives one byte at a time; collect the whole sequence.
                let start = *cursor - 1;
                let length = utf8_length(*byte);
                *cursor = start + length;
                let slice = bytes
                    .get(start..*cursor)
                    .ok_or_else(|| format!("a truncated character at byte {start}"))?;
                text.push_str(
                    std::str::from_utf8(slice)
                        .map_err(|_| format!("invalid UTF-8 at byte {start}"))?,
                );
            }
        }
    }
    Err("the document ends inside a string".to_owned())
}

/// How many bytes a UTF-8 sequence starting with `lead` occupies.
const fn utf8_length(lead: u8) -> usize {
    match lead {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

fn parse_array(bytes: &[u8], cursor: &mut usize) -> Result<Value, String> {
    *cursor += 1;
    let mut items = Vec::new();
    skip_whitespace(bytes, cursor);
    if bytes.get(*cursor) == Some(&b']') {
        *cursor += 1;
        return Ok(Value::Array(items));
    }
    loop {
        items.push(parse_value(bytes, cursor)?);
        skip_whitespace(bytes, cursor);
        match bytes.get(*cursor) {
            Some(b',') => *cursor += 1,
            Some(b']') => {
                *cursor += 1;
                return Ok(Value::Array(items));
            }
            _ => return Err(format!("expected `,` or `]` at byte {cursor}")),
        }
    }
}

fn parse_object(bytes: &[u8], cursor: &mut usize) -> Result<Value, String> {
    *cursor += 1;
    let mut map = BTreeMap::new();
    skip_whitespace(bytes, cursor);
    if bytes.get(*cursor) == Some(&b'}') {
        *cursor += 1;
        return Ok(Value::Object(map));
    }
    loop {
        skip_whitespace(bytes, cursor);
        if bytes.get(*cursor) != Some(&b'"') {
            return Err(format!("expected a key at byte {cursor}"));
        }
        let key = parse_string(bytes, cursor)?;
        skip_whitespace(bytes, cursor);
        if bytes.get(*cursor) != Some(&b':') {
            return Err(format!("expected `:` at byte {cursor}"));
        }
        *cursor += 1;
        map.insert(key, parse_value(bytes, cursor)?);
        skip_whitespace(bytes, cursor);
        match bytes.get(*cursor) {
            Some(b',') => *cursor += 1,
            Some(b'}') => {
                *cursor += 1;
                return Ok(Value::Object(map));
            }
            _ => return Err(format!("expected `,` or `}}` at byte {cursor}")),
        }
    }
}

/// `text` as a JSON string literal, quotes included.
#[must_use]
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if (control as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}
