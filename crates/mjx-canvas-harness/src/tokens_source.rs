//! The live token editor's other half: reading `docs/client-platform/data/tokens.json` and
//! **writing back to it**.
//!
//! # Why the write-back exists at all
//!
//! `docs/client-platform/CANVAS_UI_INVENTORY.md` §3: *"a live token editor — every design token
//! adjustable at runtime, because the point of the audit is to **tweak**. Changes write back to the
//! token source, so an approved value is captured rather than transcribed."* A harness whose editor
//! only changed the running process would end an audit session with a screenshot and a number in
//! somebody's notes, and the number would be retyped — or not.
//!
//! # ⚠ Why this edits the file as text rather than round-tripping it as JSON
//!
//! `tokens.json` is *"the ONLY hand-edited artefact in the pipeline"*: 491 lines of grouped,
//! commented, deliberately laid-out source whose `$description` fields are prose a person wrote.
//! Parsing it into a generic value tree and printing it back would reformat every line of it, so a
//! one-character tweak in the harness would arrive as a five-hundred-line diff and the review that
//! is supposed to catch a bad colour would be a review of whitespace.
//!
//! So [`rewrite`] performs the narrowest possible edit: it finds the token's own object, finds the
//! `"$value"` inside it, and replaces the quoted string. **Everything else in the file is byte for
//! byte what it was** — asserted in `tests/the_token_editor_writes_back.rs`, which checks the whole
//! file outside the replaced span rather than checking that the new value is present.
//!
//! # And why a value is validated before it is written
//!
//! Through `mjx_tokens::Tokens::set_custom_property`, which is the generated parser the platform
//! itself resolves with. A harness that wrote `#gg0000` into the source would leave the workspace
//! in a state where `cargo run -p xtask -- tokens` fails and the person who typed it has already
//! closed the tab. The failure belongs at the keystroke.

use std::path::{Path, PathBuf};

use mjx_tokens::{TokenIdentity, TokenValue, Tokens, TOKENS};

/// The design-token source, relative to the workspace root.
pub const SOURCE: &str = "docs/client-platform/data/tokens.json";

/// The workspace root, from this crate's manifest directory.
///
/// `CARGO_MANIFEST_DIR/../..` — the crate lives at `crates/mjx-canvas-harness`. Written once here
/// rather than at three call sites that could each get the number of `..` wrong.
#[must_use]
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// The token source's path.
#[must_use]
pub fn source_path() -> PathBuf {
    workspace_root().join(SOURCE)
}

/// One token, as the harness's editor shows it.
#[derive(Clone, PartialEq, Debug)]
pub struct Editable {
    /// The dotted path in `tokens.json`, e.g. `document.light.selection-handle`.
    pub path: &'static str,
    /// The CSS custom property, e.g. `--document-light-selection-handle`.
    pub custom_property: &'static str,
    /// The path in `ui/tokens/tokens.ts`.
    pub typescript_path: &'static str,
    /// The current value, as CSS text — byte for byte what `tokens.css` declares.
    pub value: String,
    /// Which kind of value it is, so the editor can offer a colour picker rather than a text box.
    pub kind: &'static str,
}

/// Every token the platform defines, with its current value out of `tokens`.
///
/// Ninety-two of them, derived from [`TOKENS`] rather than listed here: a token added to the source
/// appears in the editor the moment the generator has run, and one that is removed disappears.
#[must_use]
pub fn editable(tokens: &Tokens) -> Vec<Editable> {
    TOKENS
        .iter()
        .filter_map(|identity: &'static TokenIdentity| {
            let value = tokens.custom_property(identity.custom_property)?;
            Some(Editable {
                path: identity.path,
                custom_property: identity.custom_property,
                typescript_path: identity.typescript_path,
                value: value.to_string(),
                kind: kind_of(&value),
            })
        })
        .collect()
}

/// What kind of value a token holds, in one word.
#[must_use]
pub fn kind_of(value: &TokenValue) -> &'static str {
    match value {
        TokenValue::Color(_) => "color",
        TokenValue::Dimension(_) => "dimension",
        TokenValue::Duration(_) => "duration",
        TokenValue::Number(_) => "number",
        TokenValue::FontWeight(_) => "font-weight",
        TokenValue::FontStack(_) => "font-stack",
        TokenValue::CubicBezier(_) => "cubic-bezier",
        TokenValue::Shadow(_) => "shadow",
    }
}

/// What went wrong with a write-back.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum WriteBackError {
    /// The custom property is not one this platform defines.
    #[error("`{custom_property}` is not a design token this platform defines")]
    UnknownToken {
        /// The name that was offered.
        custom_property: String,
    },
    /// The value is not a value of that token's type.
    #[error("{0}")]
    Malformed(String),
    /// The token's path is not in the source file where its identity says it is.
    #[error(
        "`{path}` is a token this platform defines but `{file}` has no `\"$value\"` under it. \
         The two are meant to be the same table: `crates/mjx-tokens/src/generated.rs` is emitted \
         from that file, so a token in one and not the other means the generator has not been run \
         — try `cargo run -p xtask -- tokens`."
    )]
    NotInSource {
        /// The dotted path that was looked for.
        path: String,
        /// The file it was looked for in. **Named `file` and not `source`**: `thiserror` reads a
        /// field called `source` as the error's cause and requires it to implement `Error`, so the
        /// obvious name is the one name this field may not have.
        file: String,
    },
    /// The file could not be read or written.
    #[error("{0}")]
    Io(String),
}

/// What one write-back did.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Rewritten {
    /// The whole source, with one span replaced.
    pub text: String,
    /// What the token's `$value` said before, verbatim.
    pub previous: String,
    /// **Whether the value that was replaced was an alias** — `{color.green-deep}` rather than
    /// `#1e7a49`.
    ///
    /// # ⚠ Why this is reported rather than silently allowed or silently refused
    ///
    /// The token source is written in the W3C format's alias form, and about a third of it uses it:
    /// `document.light.selection-handle` says `{color.green-deep}`, which is not a colour but a
    /// *statement that this role follows that one*. Replacing it with a literal is a change to the
    /// token **system**, not to a value — the handle stops following the accent, and a later change
    /// to the accent will silently not reach it.
    ///
    /// That is sometimes exactly the intended tweak, so it is not refused; the audit's whole purpose
    /// is to change things. What it may not be is *invisible*, because the person doing the tweak is
    /// looking at a canvas and not at the file. So the harness says so on the panel, in the same
    /// breath as the confirmation.
    pub previous_was_alias: bool,
}

/// The source text with `custom_property`'s `$value` replaced by `value`, and nothing else changed.
///
/// # Errors
///
/// [`WriteBackError::UnknownToken`] for a name that is not a token, [`WriteBackError::Malformed`]
/// for text that is not a value of its token's type, and [`WriteBackError::NotInSource`] when the
/// source does not carry the token the generated table says it does.
pub fn rewrite(
    source: &str,
    custom_property: &str,
    value: &str,
) -> Result<Rewritten, WriteBackError> {
    let identity =
        mjx_tokens::identity_of(custom_property).ok_or_else(|| WriteBackError::UnknownToken {
            custom_property: custom_property.to_owned(),
        })?;

    // **Validated through the platform's own parser**, before a byte is written, and encoded into
    // the shape the token's own `$type` calls for. A value the resolver would reject is a value that
    // breaks `cargo run -p xtask -- tokens` for whoever runs it next, which is a failure a long way
    // from the keystroke that caused it.
    let encoded = json_for(custom_property, value)?;

    let span = value_span(source, identity.path).ok_or_else(|| WriteBackError::NotInSource {
        path: identity.path.to_owned(),
        file: SOURCE.to_owned(),
    })?;
    let previous = value_text(source, identity.path).unwrap_or_default();
    let previous_was_alias = previous.starts_with('{') && previous.ends_with('}');
    let mut text = String::with_capacity(source.len() + encoded.len());
    text.push_str(&source[..span.0]);
    text.push_str(&encoded);
    text.push_str(&source[span.1..]);
    Ok(Rewritten {
        text,
        previous,
        previous_was_alias,
    })
}

/// Write `value` into the token source on disk.
///
/// # Errors
///
/// As [`rewrite`], plus [`WriteBackError::Io`] for a file that cannot be read or written.
pub fn write_back(
    path: &Path,
    custom_property: &str,
    value: &str,
) -> Result<Rewritten, WriteBackError> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| WriteBackError::Io(format!("reading {}: {error}", path.display())))?;
    let rewritten = rewrite(&source, custom_property, value)?;
    std::fs::write(path, &rewritten.text)
        .map_err(|error| WriteBackError::Io(format!("writing {}: {error}", path.display())))?;
    Ok(rewritten)
}

/// The byte range of `path`'s **whole `$value`**, whatever shape it has.
///
/// # ⚠ A `$value` is not always a quoted string, and assuming it was left ten tokens uneditable
///
/// The first version of this function looked for `"$value": "…"` and nothing else. That is right for
/// a colour, a dimension, a duration, a font stack, a shadow and an alias — and wrong for ten
/// tokens of the ninety-two, which the suite found rather than a reader:
///
/// ```text
/// "medium": { "$value": 500 },              font-weight — a bare number
/// "tight":  { "$value": 1.25 },             leading — a bare number
/// "ink":    { "$value": [0.45, 0, 0.2, 1] } ease — an array
/// ```
///
/// A harness that offered ninety-two controls and could commit eighty-two would be exactly the
/// shape of hole this phase keeps finding: produced, reachable, and silently doing nothing for a
/// tenth of its surface. So the span is the JSON *value* — quotes included for a string — and
/// [`json_for`] writes back whichever shape the token's type calls for.
///
/// The walk is deliberately literal: descend one key at a time, tracking the depth of the object
/// each key opens, so that `color.green` cannot be answered by a `"green"` key inside
/// `theme.light`. A regular expression over the whole file could not tell the two apart, and the
/// one it picked would be whichever came first.
#[must_use]
pub fn value_span(source: &str, path: &str) -> Option<(usize, usize)> {
    let mut cursor = 0_usize;
    let mut end = source.len();
    for segment in path.split('.') {
        let (start, stop) = object_of(&source[cursor..end], segment)?;
        end = cursor + stop;
        cursor += start;
    }
    let (start, stop) = json_value_after(&source[cursor..end], "$value")?;
    Some((cursor + start, cursor + stop))
}

/// What `path`'s `$value` says, as a reader would quote it — a string's body without its quotes, and
/// a number or an array exactly as the file writes it.
#[must_use]
pub fn value_text(source: &str, path: &str) -> Option<String> {
    let (start, end) = value_span(source, path)?;
    let raw = &source[start..end];
    Some(
        raw.strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or(raw)
            .to_owned(),
    )
}

/// The byte range of the object `"key": { … }` opens, within `text`.
fn object_of(text: &str, key: &str) -> Option<(usize, usize)> {
    let needle = format!("\"{key}\"");
    let mut from = 0_usize;
    loop {
        let at = from + text[from..].find(&needle)?;
        let after = at + needle.len();
        let rest = text[after..].trim_start();
        if rest.starts_with(':') {
            let colon = after + (text[after..].len() - rest.len()) + 1;
            let open = colon + text[colon..].find('{')?;
            let close = matching_brace(text, open)?;
            return Some((open + 1, close));
        }
        from = after;
    }
}

/// The index of the `}` closing the `{` at `open`, skipping braces inside strings.
fn matching_brace(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0_i32;
    let mut in_string = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().enumerate().skip(open) {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// The byte range of the whole JSON value `"key": …` holds, within `text`.
///
/// Three shapes, because the token source uses three: a quoted string (span includes the quotes), a
/// bracketed array, and a bare scalar up to the `,` or `}` that ends it.
fn json_value_after(text: &str, key: &str) -> Option<(usize, usize)> {
    let needle = format!("\"{key}\"");
    let at = text.find(&needle)?;
    let after = at + needle.len();
    let colon = after + text[after..].find(':')? + 1;
    let start = colon
        + text[colon..]
            .find(|character: char| !character.is_whitespace())?;
    let bytes = text.as_bytes();
    match bytes.get(start)? {
        b'"' => {
            let mut index = start + 1;
            while index < text.len() {
                match bytes[index] {
                    b'\\' => index += 2,
                    b'"' => return Some((start, index + 1)),
                    _ => index += 1,
                }
            }
            None
        }
        b'[' => {
            let close = start + text[start..].find(']')?;
            Some((start, close + 1))
        }
        _ => {
            // A bare scalar runs to whatever ends it — and **`text` here is the token's own object
            // body**, so for a one-line entry such as `"medium": { "$value": 500 }` there is no
            // comma, no brace and no newline left in the slice at all. `find` answering `None` has
            // to mean *"to the end"* rather than *"not present"*: reading it the other way is what
            // left `font-weight.*` and `leading.*` uneditable after the array case was fixed, which
            // the suite caught twice in a row.
            let stop = text[start..].find([',', '}', '\n']).unwrap_or(text.len() - start);
            Some((start, start + text[start..start + stop].trim_end().len()))
        }
    }
}

/// One token's value as the source spells it: quoted for a string, bare for a number, bracketed for
/// an easing curve.
///
/// **The shape follows the token's type, not the text the person typed.** A person editing
/// `font-weight.medium` types `600` in a text box and the panel reads it back as CSS, and the source
/// has to receive `600` and not `"600"` — the token source is `$type`-annotated and a string where a
/// number belongs is a file the generator would refuse.
///
/// # Errors
///
/// [`WriteBackError::Malformed`] when the platform's own parser will not take the text, which is the
/// same refusal [`rewrite`] makes and is made here so that the encoding cannot be chosen for a value
/// that was never valid.
pub fn json_for(custom_property: &str, value: &str) -> Result<String, WriteBackError> {
    let mut probe = Tokens::DEFAULTS.clone();
    probe
        .set_custom_property(custom_property, value)
        .map_err(|error| WriteBackError::Malformed(error.to_string()))?;
    let parsed = probe
        .custom_property(custom_property)
        .ok_or_else(|| WriteBackError::UnknownToken {
            custom_property: custom_property.to_owned(),
        })?;
    Ok(match parsed {
        TokenValue::Number(number) => format!("{number}"),
        TokenValue::FontWeight(weight) => format!("{weight}"),
        TokenValue::CubicBezier(curve) => format!(
            "[{}, {}, {}, {}]",
            curve.x1, curve.y1, curve.x2, curve.y2
        ),
        other => format!("\"{}\"", escape_json(&other.to_string())),
    })
}

/// A value as a JSON string body — the two characters that must not reach one as themselves.
///
/// Not a general escaper, and deliberately: a design token's value is a colour, a length, a
/// duration or a font stack, and the only one of those that can contain a quote is a font family
/// name. Anything a token value can hold, this handles; anything else was already refused by the
/// parser above.
fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
