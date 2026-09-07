//! The API list, read out of the facade's own source rather than written down.
//!
//! # Why this exists at all
//!
//! MJXOFF-208 and MJXOFF-209 were both edits that destroyed content in a file a user opened, and
//! both landed because the method they landed on was outside every preservation test in the
//! workspace. A suite whose method list is hand-maintained has exactly that hole one level up: the
//! *next* method to be added is outside the gate by default, and nothing says so. So the list is
//! derived here, and [`crate::the_registry_and_the_facade_agree_in_both_directions`] compares it
//! against the registry **in both directions** — a method missing from the registry fails, and a
//! registry entry naming a method the facade no longer has fails too.
//!
//! # The predicate, and why it is not "mutating"
//!
//! The ticket asks for *"every mutating public API"*, and a `&mut self` receiver is the mechanical
//! stand-in for it. **In this codebase it over-selects heavily and that is a feature, not a
//! tolerated inaccuracy**: 452 of the facade's public methods take `&mut self`, and well over half
//! of them are *readers*. They need `&mut self` because every part is parsed lazily, so reading one
//! materializes its model. That makes them exactly as interesting as the mutators here — a reader
//! that left a part dirty would re-serialize it on save and rewrite a part the caller only looked
//! at — and it makes the predicate one no judgement is needed to apply. A boundary drawn at "the
//! methods that look like mutators" would be a hand-maintained list wearing a derivation's clothes.
//!
//! # The boundary: the facade, and nothing below it
//!
//! `mjx-ooxml` is the surface both bindings project, and `CLAUDE.md` requires that *"when the facade
//! grows a method, both bindings grow it"* — so the facade is where the whole callable mutating
//! surface of this library is stated. `mjx-docx` alone has 269 further `&mut self` methods, but a
//! caller reaching them is reaching past the facade into the tier below, and the tier below has its
//! own suites. Drawing the line here is what makes the list finite and the coverage claim
//! checkable; drawing it at "every crate" would produce a number nobody could keep at 100%.
//!
//! # How the scan works, and what it deliberately cannot see
//!
//! Each surface owns a file and a directory — `src/deck.rs` and `src/deck/`, and the same for
//! `document` and `workbook` — and every `impl` block in them is an inherent block on that surface.
//! So the scan attributes a method by **path**, which needs no brace matching and cannot be fooled
//! by a brace inside a string. Comment lines are stripped first, so a `pub fn` inside a doc example
//! is not counted. The scan was cross-checked against an independent one that brace-matches
//! `impl Deck`/`impl super::Deck` blocks: both report 242 / 118 / 92.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::Api;

/// The facade's `src` directory.
fn source_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Which surface a source file belongs to, or `None` for a file that is not one of the three
/// surfaces' own (`address.rs`, `error.rs`, `format.rs`, …).
fn surface_of(path: &Path) -> Option<Api> {
    let root = source_dir();
    let relative = path.strip_prefix(&root).ok()?;
    let first = relative.components().next()?.as_os_str().to_str()?;
    match first {
        "deck.rs" | "deck" => Some(Api::Deck),
        "document.rs" | "document" => Some(Api::Document),
        "workbook.rs" | "workbook" => Some(Api::Workbook),
        _ => None,
    }
}

/// Every `.rs` file under the facade's `src`, sorted.
fn source_files() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let entries = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
            .map(|entry| entry.expect("a directory entry").path());
        for path in entries {
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&source_dir(), &mut out);
    out.sort();
    out
}

/// The source with every comment line removed, so a `pub fn` inside a doc example is not counted as
/// a method.
fn without_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether the attributes immediately preceding `at` gate the item behind a Cargo feature that is
/// off in this build.
///
/// Comment lines are already gone, so an item's attributes sit directly in front of it. The walk
/// steps backwards over whitespace and over complete `#[…]` groups, and stops at the first thing
/// that is neither — which is the end of the previous item.
fn gated_off(flat: &str, at: usize) -> bool {
    let bytes = flat.as_bytes();
    let mut end = at;
    loop {
        while end > 0 && bytes[end - 1] == b' ' {
            end -= 1;
        }
        if end == 0 || bytes[end - 1] != b']' {
            return false;
        }
        let mut depth = 0i32;
        let mut start = end;
        while start > 0 {
            start -= 1;
            match bytes[start] {
                b']' => depth += 1,
                b'[' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
        if start == 0 || bytes[start - 1] != b'#' {
            return false;
        }
        let attribute = &flat[start..end];
        // The facade has exactly one feature, and one `&mut self` method behind it.
        if attribute.contains("cfg(feature = \"vml\")") && !cfg!(feature = "vml") {
            return true;
        }
        end = start - 1;
    }
}

/// Every `pub fn NAME` in `source` whose first parameter is `&mut self`, skipping any gated behind a
/// Cargo feature this build does not have.
///
/// Signatures wrap over several lines, so the source is whitespace-normalized first and the scan
/// then reads forward from each `pub fn`: the name, an optional generic list, the opening
/// parenthesis, and `&mut self`.
fn mutating_methods_in(source: &str) -> Vec<String> {
    let flat = source.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut found = Vec::new();
    let mut consumed = 0usize;
    let mut rest = flat.as_str();
    while let Some(at) = rest.find("pub fn ") {
        let keyword = consumed + at;
        consumed = keyword + "pub fn ".len();
        rest = &rest[at + "pub fn ".len()..];
        if gated_off(&flat, keyword) {
            continue;
        }
        let name_end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        let (name, mut tail) = rest.split_at(name_end);
        if name.is_empty() {
            continue;
        }
        // An optional generic parameter list, then the argument list.
        if tail.starts_with('<') {
            let Some(close) = tail.find('>') else {
                continue;
            };
            tail = &tail[close + 1..];
        }
        let tail = tail.trim_start();
        let Some(args) = tail.strip_prefix('(') else {
            continue;
        };
        if args.trim_start().starts_with("&mut self") {
            found.push(name.to_owned());
        }
    }
    found
}

/// Every public `&mut self` method of the three facade surfaces, as `(surface, method)`.
///
/// # Panics
/// If the facade's source cannot be read.
pub(crate) fn facade_mutating_methods() -> BTreeSet<(Api, String)> {
    let mut methods = BTreeSet::new();
    for path in source_files() {
        let Some(surface) = surface_of(&path) else {
            continue;
        };
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        for name in mutating_methods_in(&without_comments(&source)) {
            methods.insert((surface, name));
        }
    }
    methods
}
