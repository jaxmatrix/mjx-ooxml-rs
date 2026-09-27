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
//! `tokens.json` is *"the ONLY hand-edited artefact in the pipeline"*: nine hundred lines of grouped,
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
//!
//! # ⚠ The half of that validation MJXOFF-166 shipped without, and how it is closed
//!
//! The paragraph above was true of a value's **type** and false of its **contrast**, which is the
//! rule a design tweak is far more likely to trip. `DESIGN_TOKENS.md` §2.2 — every colour tagged for
//! text reaches 4.5 : 1 against its declared background — lived in `xtask/src/codegen/tokens/`, and
//! this crate may not depend on `xtask`. So the editor accepted a text colour that
//! `cargo run -p xtask -- tokens` would refuse, and the person who typed it found out a build
//! later: exactly the failure the comment above says the validation exists to prevent, for the
//! most likely case.
//!
//! The rule moved **down** into `mjx-tokens`, where both writers can reach it, and [`rewrite`] now
//! calls `mjx_tokens::check_usage` over the **rewritten text** before a byte reaches the disk. It
//! checks *every* text-tagged token rather than only the edited one, because the rule is symmetric:
//! darkening `color.paper` breaks every `on-light-text` colour measured against it, and an editor
//! that only looked at the token under the cursor would wave that through. That is the same sweep
//! the generator does, through the same function.

use std::path::{Path, PathBuf};

use mjx_tokens::{Color, ColorUsage, TokenIdentity, TokenValue, Tokens, TOKENS};

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
    /// What this colour may be used for, or `None` for a token that is not a colour.
    ///
    /// Shown on the panel so the contrast rule is visible *before* a refusal rather than only in
    /// one: a token marked `on-light-text` is one the write-back will measure, and `fill-only` is
    /// one it will not.
    pub usage: Option<ColorUsage>,
    /// The dotted path of the token this colour's contrast is measured against, if it declares one.
    pub background: Option<&'static str>,
}

/// Every token the platform defines, with its current value out of `tokens`.
///
/// Derived from [`TOKENS`] rather than listed here: a token added to the source
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
                usage: identity.usage,
                background: identity.background,
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
        TokenValue::Percentage(_) => "percentage",
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
    /// The rewritten source would break `DESIGN_TOKENS.md` §2.2's contrast rule.
    ///
    /// # Why the token named here may not be the token that was edited
    ///
    /// The rule is symmetric. Darkening `color.paper` does not change `color.ink`, and it breaks
    /// every `on-light-text` colour measured against `color.paper` — so the refusal names the
    /// **pair**, not the keystroke. Blaming the edited token would send a person to change a colour
    /// that is fine.
    #[error(
        "`{token}` {detail}\n\
         Nothing was written. This is the same refusal `cargo run -p xtask -- tokens` would make \
         one build later; it is made here so the file on disk never reaches a state the generator \
         will not take. A change that needs a background and its text to move together has to move \
         the text colour first."
    )]
    Contrast {
        /// The dotted path of the token whose contrast fails — **not necessarily the edited one**.
        token: String,
        /// What `mjx_tokens::check_usage` said, measurement included.
        detail: String,
    },
    /// The token is derived, so it has no value of its own to type into.
    ///
    /// Added by MJXOFF-271, when the source grew a second tier. A derived colour's `$value` is a
    /// `color-mix()` of the seeds and knobs above it; replacing it with a literal would not be a
    /// tweak but a **deletion of the derivation**, after which the colour would stop following its
    /// seed and every later skin change would silently miss it. That is a decision to take in the
    /// source with the reasoning written down, not one to make by dragging a colour picker.
    #[error(
        "`{path}` is derived — it is a `color-mix()` of {seeds}. Editing it here would replace the \
         expression with a literal and quietly take the token out of the theme; adjust one of \
         those instead, and every colour mixed from it follows."
    )]
    Derived {
        /// The dotted path of the derived token.
        path: String,
        /// The tokens its expression reads, so the panel can offer them.
        seeds: String,
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

    // A derived token has no literal to replace; see `WriteBackError::Derived`.
    if let Some(derivation) = derivation_of(custom_property) {
        return Err(WriteBackError::Derived {
            path: identity.path.to_owned(),
            seeds: describe_seeds(derivation),
        });
    }

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

    // **`DESIGN_TOKENS.md` §2.2, before a byte reaches the disk.** See the module docs: the type
    // check above is `mjx_tokens`'s parser and this is `mjx_tokens`'s contrast rule, which is the
    // one a design tweak actually trips.
    check_contrast(&text)?;

    Ok(Rewritten {
        text,
        previous,
        previous_was_alias,
    })
}

/// Every text-tagged colour in `source`, measured against the background it declares.
///
/// The sweep is over [`TOKENS`] rather than over the edited token, for the reason
/// [`WriteBackError::Contrast`] gives: the rule binds a **pair**, and either half of a pair can be
/// the one that moved. It is one pass over the file per token to load the literals, and then
/// arithmetic — cheaper than the JSON parse this module refuses to do.
///
/// # Errors
///
/// [`WriteBackError::Contrast`] for a pair that fails the rule, [`WriteBackError::NotInSource`] for
/// a token the generated table has and the file does not, and [`WriteBackError::Malformed`] for a
/// colour the platform's own parser will not take.
fn check_contrast(source: &str) -> Result<(), WriteBackError> {
    let resolved = snapshot_of(source)?;
    let colour_of = |path: &str| -> Result<Color, WriteBackError> {
        let identity =
            mjx_tokens::identity_at(path).ok_or_else(|| WriteBackError::NotInSource {
                path: path.to_owned(),
                file: SOURCE.to_owned(),
            })?;
        match resolved.custom_property(identity.custom_property) {
            Some(TokenValue::Color(colour)) => Ok(colour),
            _ => Err(WriteBackError::Malformed(format!(
                "`{path}` is not a colour, so a contrast rule cannot be measured against it"
            ))),
        }
    };
    for identity in TOKENS {
        let (Some(usage), Some(background_path)) = (identity.usage, identity.background) else {
            continue;
        };
        if !usage.colours_text() {
            continue;
        }
        let colour = colour_of(identity.path)?;
        let background = colour_of(background_path)?;
        mjx_tokens::check_usage(usage, colour, background, background_path).map_err(|error| {
            WriteBackError::Contrast {
                token: identity.path.to_owned(),
                detail: error.to_string(),
            }
        })?;
    }
    Ok(())
}

/// How many alias hops [`literal_in_source`] will follow before it calls the chain a cycle.
///
/// The source's longest chain is two (`document.light.selection-handle` → `color.green-deep` → a
/// literal). Eight is generous and finite; the generator refuses a cycle outright and this must
/// terminate rather than reproduce that analysis.
const ALIAS_HOPS: usize = 8;

/// The whole token set **as this text says it is**, derived tier and all.
///
/// Built from the source being written rather than taken from [`Tokens::DEFAULTS`], because the
/// defaults are what the *last* generator run emitted and the question here is whether the file as
/// it will be on disk is one the *next* run will take. A background the same editing session
/// changed a minute ago is only visible this way.
///
/// # ⚠ The derived tier is re-derived, never re-implemented
///
/// Before MJXOFF-271 this walked the text for one colour, following the W3C alias form. It cannot
/// do that any more: `theme.light.text-secondary` is tagged for text, and its `$value` is a
/// `color-mix()` object rather than a colour — as is the background it declares. Rather than teach
/// this module to evaluate a mix, which would be a **second** `color-mix()` implementation and
/// therefore the exact divergence the four-artefact pipeline exists to prevent, it loads the
/// literal **colours and percentages** out of the text and calls `Tokens::rederive`, which is
/// `mjx_tokens::color_mix` and nothing else.
///
/// # Why only those two kinds
///
/// They are the only ones a contrast measurement or a derivation can read: a colour is the thing
/// being measured or mixed, and a percentage is a mix knob. A radius, a duration, a font stack and a
/// shadow are left at their defaults *deliberately* — loading them would mean parsing a shadow's
/// `$value`, which is a JSON object and not a CSS shadow, and it would gain nothing measurable. The
/// first attempt at this loaded everything, fed the shadow's object body to the resolver, and made
/// the write-back refuse every edit with a message naming a fragment of JSON as a token path. The
/// suite caught it.
fn snapshot_of(source: &str) -> Result<Tokens, WriteBackError> {
    let mut tokens = Tokens::DEFAULTS.clone();
    for identity in TOKENS {
        if derivation_of(identity.custom_property).is_some() {
            // Recomputed below; whatever the file says for it is the *last* run's answer.
            continue;
        }
        match Tokens::DEFAULTS.custom_property(identity.custom_property) {
            Some(TokenValue::Color(_) | TokenValue::Percentage(_)) => {}
            _ => continue,
        }
        let text = literal_in_source(source, identity.path)?;
        tokens
            .set_custom_property(identity.custom_property, &text)
            .map_err(|error| WriteBackError::Malformed(error.to_string()))?;
    }
    tokens.rederive();
    Ok(tokens)
}

/// The derivation for one custom property, if it has one.
fn derivation_of(custom_property: &str) -> Option<&'static mjx_tokens::Derivation> {
    mjx_tokens::DERIVATIONS
        .iter()
        .find(|derivation| derivation.custom_property == custom_property)
}

/// The tokens a derivation reads, as a readable list for [`WriteBackError::Derived`].
fn describe_seeds(derivation: &mjx_tokens::Derivation) -> String {
    let mut named: Vec<&str> = Vec::new();
    match derivation.source {
        mjx_tokens::DerivedFrom::Token(custom_property) => named.push(custom_property),
        mjx_tokens::DerivedFrom::Mix(nodes) => {
            for node in nodes {
                for term in [node.first, node.second] {
                    if let mjx_tokens::MixTerm::Token(custom_property) = term {
                        named.push(custom_property);
                    }
                }
                for percentage in [node.first_percentage, node.second_percentage] {
                    if let Some(mjx_tokens::MixPercentage::Token(custom_property)) = percentage {
                        named.push(custom_property);
                    }
                }
            }
        }
    }
    named.dedup();
    named
        .iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The token path a W3C alias names, or `None` when the text is not an alias.
///
/// ⚠ **The braces alone are not enough to tell**, and reading them that way was a real defect: a
/// shadow's `$value` and a `mix` derivation's are both JSON *objects*, so they also begin with `{`
/// and end with `}`. Treating one as an alias made the write-back refuse every edit with a message
/// naming a fragment of JSON as a token path. An alias is a dotted path, so it carries no
/// whitespace and no quotes, and that is what is checked.
fn alias_target(text: &str) -> Option<&str> {
    let inner = text.strip_prefix('{')?.strip_suffix('}')?;
    if inner.is_empty()
        || inner
            .chars()
            .any(|character| character.is_whitespace() || character == '"' || character == '{')
    {
        return None;
    }
    Some(inner)
}

/// The literal CSS text `path` resolves to **in this text**, following the W3C alias form.
fn literal_in_source(source: &str, path: &str) -> Result<String, WriteBackError> {
    let mut at = path.to_owned();
    for _ in 0..ALIAS_HOPS {
        let text = value_text(source, &at).ok_or_else(|| WriteBackError::NotInSource {
            path: at.clone(),
            file: SOURCE.to_owned(),
        })?;
        match alias_target(&text) {
            Some(target) => at = target.to_owned(),
            None => return Ok(text),
        }
    }
    Err(WriteBackError::Malformed(format!(
        "`{path}` does not resolve to a value in {ALIAS_HOPS} alias hops, so the source has a \
         cycle in it; `cargo run -p xtask -- tokens` names the chain"
    )))
}

/// What `usage` a token declares, in one word, for the editor's panel.
///
/// `None` for everything that is not a colour. The panel shows it so that a person tweaking
/// `--color-green` can see *before* typing that it is `fill-only` and that
/// [`ColorUsage::OnLightText`] tokens are the ones the write-back will measure.
#[must_use]
pub fn usage_of(custom_property: &str) -> Option<ColorUsage> {
    mjx_tokens::identity_of(custom_property).and_then(|identity| identity.usage)
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
/// tokens, which the suite found rather than a reader:
///
/// ```text
/// "medium": { "$value": 500 },              font-weight — a bare number
/// "tight":  { "$value": 1.25 },             leading — a bare number
/// "ink":    { "$value": [0.45, 0, 0.2, 1] } ease — an array
/// ```
///
/// A harness that offered a control for every token and could commit only most of them would be exactly the
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
            // ⚠ The object has to be the value **immediately** after the colon — whitespace and
            // nothing else. Searching for the first `{` anywhere after it was a latent defect, and
            // MJXOFF-271 walked into it: `$extensions.mjx.background` is a key spelled exactly
            // `"background"`, and its value is an alias such as `"{theme.light.background}"`. So a
            // walk looking for the `background` *token* matched an earlier token's contrast
            // metadata, took the `{` from inside that string, and answered with the span of a
            // token path. It only surfaced when a seed carrying that metadata was declared before
            // the token of the same name; before that the token always came first, which is why a
            // wrong reader looked right for two children.
            let value = text[colon..].trim_start();
            if value.starts_with('{') {
                let open = colon + (text[colon..].len() - value.len());
                let close = matching_brace(text, open)?;
                return Some((open + 1, close));
            }
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
    let start = colon + text[colon..].find(|character: char| !character.is_whitespace())?;
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
        // An **object**: a shadow, or — since MJXOFF-271 — a `{ "mix": [ … ] }` derivation, which
        // nests. Matched by counting braces rather than by finding the first `}`, because a
        // derivation's operands are objects too and the first `}` closes one of them.
        b'{' => {
            let mut depth = 0_usize;
            let mut index = start;
            let mut inside_string = false;
            while index < text.len() {
                match bytes[index] {
                    b'\\' if inside_string => index += 1,
                    b'"' => inside_string = !inside_string,
                    b'{' if !inside_string => depth += 1,
                    b'}' if !inside_string => {
                        depth -= 1;
                        if depth == 0 {
                            return Some((start, index + 1));
                        }
                    }
                    _ => {}
                }
                index += 1;
            }
            None
        }
        _ => {
            // A bare scalar runs to whatever ends it — and **`text` here is the token's own object
            // body**, so for a one-line entry such as `"medium": { "$value": 500 }` there is no
            // comma, no brace and no newline left in the slice at all. `find` answering `None` has
            // to mean *"to the end"* rather than *"not present"*: reading it the other way is what
            // left `font-weight.*` and `leading.*` uneditable after the array case was fixed, which
            // the suite caught twice in a row.
            let stop = text[start..]
                .find([',', '}', '\n'])
                .unwrap_or(text.len() - start);
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
    let parsed =
        probe
            .custom_property(custom_property)
            .ok_or_else(|| WriteBackError::UnknownToken {
                custom_property: custom_property.to_owned(),
            })?;
    Ok(match parsed {
        TokenValue::Number(number) => format!("{number}"),
        TokenValue::FontWeight(weight) => format!("{weight}"),
        TokenValue::CubicBezier(curve) => {
            format!("[{}, {}, {}, {}]", curve.x1, curve.y1, curve.x2, curve.y2)
        }
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
