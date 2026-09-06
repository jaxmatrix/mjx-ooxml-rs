//! Just enough JSON for `xtask`'s two readers of it.
//!
//! `xtask` carries no JSON dependency and nothing in the shipped graph wants one, so the reader is
//! here rather than in the dependency graph. It is a complete value parser — not a scan for the
//! fields of interest — because a scanner that misreads a nested string is a scanner that drops a
//! member, and dropping a member is exactly how a gate built on it would pass without doing
//! anything.
//!
//! # Two compilations, one source
//!
//! Its consumers are the design-token generator ([`crate::codegen::tokens`], which reads
//! `docs/client-platform/data/tokens.json`) and the layering gate (`xtask/tests/layering.rs`, which
//! reads `cargo metadata --no-deps`). An integration test cannot reach a binary crate's private
//! modules, so the gate pulls this same file in with
//! `#[path = "../src/json.rs"] mod json;` rather than keeping a second parser of its own — a
//! workspace with two JSON readers in it has one reader too many, and the one that is not exercised
//! is the one that is wrong. Each compilation uses a different part of the surface, which is why
//! the module allows dead code.
#![allow(dead_code)]

/// A parsed JSON value.
///
/// An object keeps its members **in source order**. `cargo metadata` does not care, but the
/// design-token generator does: it is what lets the emitted artefacts read in the order the source
/// file was written rather than in an order a hash map chose.
pub(crate) enum Value {
    Null,
    Bool(bool),
    /// Held as `f64` and re-rendered from that, never echoed back as written. A token spelled
    /// `1.250` in the source and `1.25` in an artefact would otherwise look like a divergence
    /// between two artefacts that agree perfectly about the value.
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    /// The value at `key`, if this is an object that has one and it is not `null`.
    pub(crate) fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(members) => members
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value)
                .filter(|value| !matches!(value, Value::Null)),
            _ => None,
        }
    }

    /// This value's elements, if it is an array.
    pub(crate) fn array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    /// This value's text, if it is a string.
    pub(crate) fn string(&self) -> Option<&str> {
        match self {
            Value::String(text) => Some(text),
            _ => None,
        }
    }

    /// This value, if it is a number.
    pub(crate) fn number(&self) -> Option<f64> {
        match self {
            Value::Number(number) => Some(*number),
            _ => None,
        }
    }

    /// This value, if it is a boolean.
    pub(crate) fn boolean(&self) -> Option<bool> {
        match self {
            Value::Bool(flag) => Some(*flag),
            _ => None,
        }
    }

    /// This object's members, in source order, if it is an object.
    pub(crate) fn object(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Object(members) => Some(members),
            _ => None,
        }
    }
}

/// Parses a whole JSON document, or reports the byte offset it gave up at.
pub(crate) fn parse(text: &str) -> Result<Value, String> {
    let bytes = text.as_bytes();
    let mut at = 0;
    let value = value(bytes, &mut at)?;
    skip_whitespace(bytes, &mut at);
    if at != bytes.len() {
        return Err(format!("trailing input at byte {at}"));
    }
    Ok(value)
}

fn skip_whitespace(bytes: &[u8], at: &mut usize) {
    while *at < bytes.len() && matches!(bytes[*at], b' ' | b'\t' | b'\n' | b'\r') {
        *at += 1;
    }
}

fn expect(bytes: &[u8], at: &mut usize, byte: u8) -> Result<(), String> {
    if bytes.get(*at) == Some(&byte) {
        *at += 1;
        Ok(())
    } else {
        Err(format!(
            "expected `{}` at byte {at}",
            char::from(byte),
            at = *at
        ))
    }
}

fn value(bytes: &[u8], at: &mut usize) -> Result<Value, String> {
    skip_whitespace(bytes, at);
    match bytes.get(*at) {
        Some(b'{') => object(bytes, at),
        Some(b'[') => array(bytes, at),
        Some(b'"') => string(bytes, at).map(Value::String),
        Some(b't') => literal(bytes, at, "true").map(|()| Value::Bool(true)),
        Some(b'f') => literal(bytes, at, "false").map(|()| Value::Bool(false)),
        Some(b'n') => literal(bytes, at, "null").map(|()| Value::Null),
        Some(_) => number(bytes, at),
        None => Err("unexpected end of input".to_owned()),
    }
}

fn literal(bytes: &[u8], at: &mut usize, word: &str) -> Result<(), String> {
    if bytes[*at..].starts_with(word.as_bytes()) {
        *at += word.len();
        Ok(())
    } else {
        Err(format!("expected `{word}` at byte {at}", at = *at))
    }
}

fn number(bytes: &[u8], at: &mut usize) -> Result<Value, String> {
    let start = *at;
    while *at < bytes.len() && matches!(bytes[*at], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
    {
        *at += 1;
    }
    if start == *at {
        return Err(format!("expected a value at byte {start}"));
    }
    let text = std::str::from_utf8(&bytes[start..*at]).map_err(|error| error.to_string())?;
    text.parse::<f64>()
        .map(Value::Number)
        .map_err(|error| format!("`{text}` at byte {start} is not a number: {error}"))
}

fn string(bytes: &[u8], at: &mut usize) -> Result<String, String> {
    expect(bytes, at, b'"')?;
    let mut out = String::new();
    loop {
        let byte = *bytes
            .get(*at)
            .ok_or_else(|| "unterminated string".to_owned())?;
        *at += 1;
        match byte {
            b'"' => return Ok(out),
            b'\\' => {
                let escape = *bytes
                    .get(*at)
                    .ok_or_else(|| "unterminated escape".to_owned())?;
                *at += 1;
                match escape {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{8}'),
                    b'f' => out.push('\u{c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => out.push(unicode_escape(bytes, at)?),
                    other => {
                        return Err(format!("unknown escape `\\{}`", char::from(other)));
                    }
                }
            }
            // A raw byte of a multi-byte UTF-8 sequence lands here too; pushing the bytes and
            // decoding at the end would be equivalent, but this keeps `out` a `String`
            // throughout. The input came from `String::from_utf8`, so the sequence is valid.
            _ => {
                let start = *at - 1;
                let width = utf8_width(byte);
                *at = start + width;
                let text =
                    std::str::from_utf8(&bytes[start..*at]).map_err(|error| error.to_string())?;
                out.push_str(text);
            }
        }
    }
}

/// How many bytes the UTF-8 sequence starting with `lead` occupies.
fn utf8_width(lead: u8) -> usize {
    match lead {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

/// A `\uXXXX` escape, with the surrogate pair a character outside the BMP is written as.
fn unicode_escape(bytes: &[u8], at: &mut usize) -> Result<char, String> {
    let first = hex4(bytes, at)?;
    if (0xd800..0xdc00).contains(&first) {
        expect(bytes, at, b'\\')?;
        expect(bytes, at, b'u')?;
        let second = hex4(bytes, at)?;
        let combined =
            0x1_0000 + ((u32::from(first) - 0xd800) << 10) + (u32::from(second) - 0xdc00);
        return char::from_u32(combined).ok_or_else(|| "invalid surrogate pair".to_owned());
    }
    char::from_u32(u32::from(first)).ok_or_else(|| "invalid escape".to_owned())
}

fn hex4(bytes: &[u8], at: &mut usize) -> Result<u16, String> {
    let digits = bytes
        .get(*at..*at + 4)
        .ok_or_else(|| "truncated \\u escape".to_owned())?;
    *at += 4;
    let text = std::str::from_utf8(digits).map_err(|error| error.to_string())?;
    u16::from_str_radix(text, 16).map_err(|error| error.to_string())
}

fn array(bytes: &[u8], at: &mut usize) -> Result<Value, String> {
    expect(bytes, at, b'[')?;
    let mut items = Vec::new();
    skip_whitespace(bytes, at);
    if bytes.get(*at) == Some(&b']') {
        *at += 1;
        return Ok(Value::Array(items));
    }
    loop {
        items.push(value(bytes, at)?);
        skip_whitespace(bytes, at);
        match bytes.get(*at) {
            Some(b',') => *at += 1,
            Some(b']') => {
                *at += 1;
                return Ok(Value::Array(items));
            }
            _ => return Err(format!("expected `,` or `]` at byte {at}", at = *at)),
        }
    }
}

fn object(bytes: &[u8], at: &mut usize) -> Result<Value, String> {
    expect(bytes, at, b'{')?;
    let mut members = Vec::new();
    skip_whitespace(bytes, at);
    if bytes.get(*at) == Some(&b'}') {
        *at += 1;
        return Ok(Value::Object(members));
    }
    loop {
        skip_whitespace(bytes, at);
        let key = string(bytes, at)?;
        skip_whitespace(bytes, at);
        expect(bytes, at, b':')?;
        members.push((key, value(bytes, at)?));
        skip_whitespace(bytes, at);
        match bytes.get(*at) {
            Some(b',') => *at += 1,
            Some(b'}') => {
                *at += 1;
                return Ok(Value::Object(members));
            }
            _ => return Err(format!("expected `,` or `}}` at byte {at}", at = *at)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reader has to survive the shapes `cargo metadata` actually emits: nested objects and
    /// arrays, `null` (which is how a normal dependency's `kind` is spelled), escapes inside
    /// strings, and non-ASCII text. A reader that mis-tracked a string's end would find the
    /// wrong keys, so this is checked rather than assumed.
    #[test]
    fn the_reader_handles_the_shapes_cargo_emits() {
        let text = r#"{
            "packages": [
                {"name": "a", "kind": null, "path": "C:\\x\\y", "note": "a \"quoted\" ünïcode ☃ \u2603 \ud83d\ude00"},
                {"name": "b", "deps": [], "meta": {}, "n": -1.5e3, "ok": true}
            ]
        }"#;
        let root = parse(text).expect("parses");
        let packages = root.get("packages").and_then(Value::array).expect("array");
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].get("name").and_then(Value::string), Some("a"));
        // `null` reads as absent, which is exactly how a normal dependency's `kind` is meant to
        // be understood.
        assert!(packages[0].get("kind").is_none());
        assert_eq!(
            packages[0].get("path").and_then(Value::string),
            Some(r"C:\x\y")
        );
        assert_eq!(
            packages[0].get("note").and_then(Value::string),
            Some("a \"quoted\" ünïcode ☃ ☃ 😀")
        );
        assert_eq!(packages[1].get("name").and_then(Value::string), Some("b"));
        assert_eq!(
            packages[1]
                .get("deps")
                .and_then(Value::array)
                .map(<[_]>::len),
            Some(0)
        );
    }

    /// The design-token source needs three things `cargo metadata` never asked for: numbers with
    /// their payload intact, booleans with theirs, and objects whose members stay in source order.
    /// The last is the one that would fail silently — an artefact emitted in hash order still
    /// contains every token, it just never stops changing between runs.
    #[test]
    fn numbers_booleans_and_member_order_survive_the_reader() {
        let root = parse(
            r#"{"zebra": 1.25, "apple": -0.025, "medium": 500, "on": true, "off": false,
                "exponent": 1.5e2, "nested": {"b": [1, 2], "a": "x"}}"#,
        )
        .expect("parses");

        assert_eq!(root.get("zebra").and_then(Value::number), Some(1.25));
        assert_eq!(root.get("apple").and_then(Value::number), Some(-0.025));
        assert_eq!(root.get("medium").and_then(Value::number), Some(500.0));
        assert_eq!(root.get("exponent").and_then(Value::number), Some(150.0));
        assert_eq!(root.get("on").and_then(Value::boolean), Some(true));
        assert_eq!(root.get("off").and_then(Value::boolean), Some(false));

        // Source order, not sorted order — `zebra` is written first and must come back first.
        let keys: Vec<&str> = root
            .object()
            .expect("object")
            .iter()
            .map(|(key, _)| key.as_str())
            .collect();
        assert_eq!(
            keys,
            ["zebra", "apple", "medium", "on", "off", "exponent", "nested"]
        );
        let nested: Vec<&str> = root
            .get("nested")
            .and_then(Value::object)
            .expect("nested object")
            .iter()
            .map(|(key, _)| key.as_str())
            .collect();
        assert_eq!(nested, ["b", "a"]);
    }

    #[test]
    fn malformed_input_is_an_error_rather_than_a_wrong_answer() {
        for bad in [
            "{",
            "{\"a\"}",
            "[1,]",
            "\"unterminated",
            "{} trailing",
            "{\"a\": \\}",
        ] {
            assert!(parse(bad).is_err(), "`{bad}` should not parse");
        }
    }
}
