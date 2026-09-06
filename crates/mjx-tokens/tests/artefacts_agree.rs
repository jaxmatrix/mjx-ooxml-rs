//! The three artefacts say the same thing (MJXOFF-156).
//!
//! # The trap this file exists for
//!
//! *"The three artefacts are generated and committed"* is satisfied by three files nothing reads.
//! `xtask/tests/tokens.rs` proves they are **derived** from the source; this file proves they are
//! **equal to each other**, which is the failure derivation alone cannot catch: an emitter that
//! rendered a colour one way for CSS and another for TypeScript would regenerate perfectly and be
//! wrong in exactly one consumer.
//!
//! So it parses the emitted CSS and the emitted TypeScript — as text, from disk, with no help from
//! the generator that wrote them — and compares every value against `mjx_tokens`'s own table. The
//! three names a token goes by come from [`mjx_tokens::TOKENS`] rather than being re-derived here:
//! a test that recomputed `--color-ink-soft` from `color.inkSoft` itself could agree perfectly with
//! a wrong rule, which is the whole class of bug this is written against.
//!
//! Proved by mutation: change one hexadecimal digit in `ui/tokens/tokens.css` and
//! `every_token_has_the_same_value_in_all_three_artefacts` goes red naming the token.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_tokens::{Tokens, TOKENS};

/// `ui/tokens/`, from this crate's manifest directory.
fn ui_tokens_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../ui/tokens")
        .canonicalize()
        .expect("ui/tokens exists — the generated artefacts are committed")
}

fn read(name: &str) -> String {
    let path = ui_tokens_dir().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

// ---------------------------------------------------------------------------------------------
// Readers for the two artefacts this crate cannot import
// ---------------------------------------------------------------------------------------------

/// Every `--name: value;` declaration in the stylesheet whose value is a literal.
///
/// The scheme layer's declarations are `var(--theme-light-surface)` references rather than values;
/// they are the subject of [`the_scheme_layer_points_at_tokens_that_exist`] and are skipped here.
fn css_declarations(css: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in strip_css_comments(css).lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("--") else {
            continue;
        };
        let Some((name, value)) = rest.split_once(':') else {
            continue;
        };
        let Some(value) = value.trim().strip_suffix(';') else {
            continue;
        };
        let value = value.trim();
        if value.starts_with("var(") {
            continue;
        }
        let previous = out.insert(format!("--{}", name.trim()), value.to_owned());
        assert!(
            previous.is_none(),
            "`--{}` is declared twice in tokens.css",
            name.trim()
        );
    }
    out
}

/// Every `--name: var(--other);` declaration, as `(name, other)` pairs, in file order.
fn css_scheme_references(css: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in strip_css_comments(css).lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("--") else {
            continue;
        };
        let Some((name, value)) = rest.split_once(':') else {
            continue;
        };
        let value = value.trim().trim_end_matches(';').trim();
        if let Some(target) = value
            .strip_prefix("var(")
            .and_then(|it| it.strip_suffix(')'))
        {
            out.push((format!("--{}", name.trim()), target.trim().to_owned()));
        }
    }
    out
}

fn strip_css_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(open) = rest.find("/*") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        match after.find("*/") {
            Some(close) => {
                // Keep the newlines so line-oriented scanning above still sees one declaration a
                // line rather than a comment and a declaration run together.
                out.extend(after[..close].chars().filter(|it| *it == '\n'));
                rest = &after[close + 2..];
            }
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// The leaves of `export const tokens: Tokens = { … };`, keyed by their dotted path.
///
/// A hand-written reader rather than a JSON one: the literal is TypeScript, its keys are bare
/// identifiers and its strings are single-quoted, and reading it as it is written is the only way
/// this test sees what a TypeScript consumer sees.
fn typescript_tokens(source: &str) -> BTreeMap<String, String> {
    let start = source
        .find("export const tokens: Tokens = {")
        .expect("tokens.ts declares `export const tokens`");
    let body = &source[start..];
    let open = body.find('{').expect("the literal opens");
    let mut out = BTreeMap::new();
    let mut path: Vec<String> = Vec::new();
    let mut pending: Option<String> = None;

    let bytes = body.as_bytes();
    let mut at = open + 1;
    while at < bytes.len() {
        match bytes[at] {
            b' ' | b'\n' | b'\r' | b'\t' | b',' => at += 1,
            b'}' => {
                if path.pop().is_none() {
                    break;
                }
                at += 1;
            }
            b'{' => {
                path.push(pending.take().expect("a nested object has a key"));
                at += 1;
            }
            b'\'' => {
                let (text, next) = typescript_string(body, at);
                let key = pending.take().expect("a value has a key");
                let mut full = path.clone();
                full.push(key);
                out.insert(full.join("."), text);
                at = next;
            }
            _ => {
                let end = at
                    + body[at..]
                        .find([':', ',', '\n'])
                        .expect("a key or a bare value ends");
                let word = body[at..end].trim().to_owned();
                if body.as_bytes().get(end) == Some(&b':') {
                    pending = Some(word);
                    at = end + 1;
                } else {
                    // A bare value: a number.
                    let key = pending.take().expect("a value has a key");
                    let mut full = path.clone();
                    full.push(key);
                    out.insert(full.join("."), word);
                    at = end;
                }
            }
        }
    }
    assert!(path.is_empty(), "the object literal did not close");
    out
}

/// Reads one single-quoted TypeScript string starting at `at`, returning it and the index after it.
fn typescript_string(source: &str, at: usize) -> (String, usize) {
    let bytes = source.as_bytes();
    let mut text = String::new();
    let mut cursor = at + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\'' => return (text, cursor + 1),
            b'\\' => {
                let escaped = source[cursor + 1..]
                    .chars()
                    .next()
                    .expect("an escape has a character");
                text.push(escaped);
                cursor += 1 + escaped.len_utf8();
            }
            _ => {
                let character = source[cursor..].chars().next().expect("valid UTF-8");
                text.push(character);
                cursor += character.len_utf8();
            }
        }
    }
    panic!("unterminated string in tokens.ts");
}

/// `'color.paper': '--color-paper',` — the map a runtime resolver reads host overrides through.
fn typescript_custom_properties(source: &str) -> BTreeMap<String, String> {
    let start = source
        .find("export const customProperties")
        .expect("tokens.ts declares `customProperties`");
    let mut out = BTreeMap::new();
    for line in source[start..].lines().skip(1) {
        let line = line.trim();
        if line == "};" {
            break;
        }
        let Some((key, value)) = line.trim_end_matches(',').split_once(':') else {
            continue;
        };
        out.insert(
            key.trim().trim_matches('\'').to_owned(),
            value.trim().trim_matches('\'').to_owned(),
        );
    }
    out
}

// ---------------------------------------------------------------------------------------------
// The gates
// ---------------------------------------------------------------------------------------------

#[test]
fn every_token_has_the_same_value_in_all_three_artefacts() {
    let css = css_declarations(&read("tokens.css"));
    let typescript = typescript_tokens(&read("tokens.ts"));
    let tokens = Tokens::DEFAULTS;

    assert!(
        TOKENS.len() > 50,
        "the Rust table has only {} rows, which cannot be this token set",
        TOKENS.len()
    );

    for identity in TOKENS {
        let rust = tokens
            .custom_property(identity.custom_property)
            .unwrap_or_else(|| {
                panic!(
                    "`{}`: the Rust table has no value for `{}`",
                    identity.path, identity.custom_property
                )
            })
            .to_string();

        let in_css = css.get(identity.custom_property).unwrap_or_else(|| {
            panic!(
                "`{}`: tokens.css declares no `{}`",
                identity.path, identity.custom_property
            )
        });
        assert_eq!(
            in_css, &rust,
            "token `{}` diverges: tokens.css says `{in_css}`, the Rust table says `{rust}`",
            identity.path
        );

        let in_typescript = typescript.get(identity.typescript_path).unwrap_or_else(|| {
            panic!(
                "`{}`: tokens.ts has no `{}`",
                identity.path, identity.typescript_path
            )
        });
        assert_eq!(
            in_typescript, &rust,
            "token `{}` diverges: tokens.ts says `{in_typescript}`, the Rust table says `{rust}`",
            identity.path
        );
    }
}

/// Agreement about the tokens all three carry is only half of it: an artefact with a *stale* token
/// the other two dropped agrees about every token it shares with them.
#[test]
fn no_artefact_carries_a_token_the_others_do_not() {
    let css = css_declarations(&read("tokens.css"));
    let typescript = typescript_tokens(&read("tokens.ts"));

    let mut expected_css: Vec<&str> = TOKENS.iter().map(|token| token.custom_property).collect();
    expected_css.sort_unstable();
    let mut actual_css: Vec<&str> = css.keys().map(String::as_str).collect();
    actual_css.sort_unstable();
    assert_eq!(
        actual_css, expected_css,
        "tokens.css and the Rust table declare different tokens"
    );

    let mut expected_typescript: Vec<&str> =
        TOKENS.iter().map(|token| token.typescript_path).collect();
    expected_typescript.sort_unstable();
    let mut actual_typescript: Vec<&str> = typescript.keys().map(String::as_str).collect();
    actual_typescript.sort_unstable();
    assert_eq!(
        actual_typescript, expected_typescript,
        "tokens.ts and the Rust table declare different tokens"
    );
}

/// The map a runtime resolver reads host overrides through has to name the same properties the
/// stylesheet declares, or the shell would look for a name no host ever sets.
#[test]
fn the_typescript_custom_property_map_matches_the_rust_table() {
    let map = typescript_custom_properties(&read("tokens.ts"));
    for identity in TOKENS {
        let name = map.get(identity.typescript_path).unwrap_or_else(|| {
            panic!(
                "`{}`: customProperties has no entry for `{}`",
                identity.path, identity.typescript_path
            )
        });
        assert_eq!(
            name, identity.custom_property,
            "`{}`: customProperties maps it to `{name}`",
            identity.path
        );
    }
    assert_eq!(
        map.len(),
        TOKENS.len(),
        "customProperties has {} entries for {} tokens",
        map.len(),
        TOKENS.len()
    );
}

/// The scheme layer is the only part of the stylesheet with no counterpart in the Rust table — the
/// renderer picks a scheme with [`mjx_tokens::ColorScheme`] instead. So what is checked here is
/// that every reference it makes resolves to a property that exists.
#[test]
fn the_scheme_layer_points_at_tokens_that_exist() {
    let css = read("tokens.css");
    let declared = css_declarations(&css);
    let references = css_scheme_references(&css);
    assert!(
        references.len() >= TOKENS.len() / 4,
        "only {} scheme references, which cannot cover two schemes of theme and document tokens",
        references.len()
    );
    for (name, target) in &references {
        assert!(
            declared.contains_key(target),
            "the scheme layer maps `{name}` to `{target}`, which nothing declares"
        );
        assert!(
            !declared.contains_key(name),
            "`{name}` is both a scheme alias and a token in its own right"
        );
    }
}
