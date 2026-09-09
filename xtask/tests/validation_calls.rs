//! The checklist gate: **every call chain names a method that exists, in all three languages, and
//! every artefact a check names is a file this repository produces** (MJXOFF-128).
//!
//! # The two claims, and why a machine has to make them
//!
//! `docs/validation/` is prose a person works through with Office open. Its two machine-checkable
//! claims are exactly the two that rot fastest:
//!
//! * **A call chain naming a method that does not exist is worse than no entry** — it sends a
//!   reader to an API that cannot be called, in a language they may not know well enough to notice.
//! * **An artefact nobody produces is a check nobody can do.** `xtask/tests/validation_index.rs`
//!   already binds every *area* to a generated artefact; this file binds every *check* to a file,
//!   and a check may name an example's source or a committed fixture as well as a generated
//!   artefact.
//!
//! # Where each side of the comparison comes from
//!
//! Neither side is generated from the other, and no side is generated from the documents:
//!
//! | Language | Source of truth | What is read out of it |
//! |---|---|---|
//! | Rust | `crates/mjx-ooxml/src/{deck,document,workbook}{,/*}.rs` | every `pub fn` inside an `impl Deck` / `impl Document` / `impl Workbook` block |
//! | Python | `bindings/mjx-python/python/mjx_ooxml/__init__.pyi` | every `def` inside `class Deck:` / `class Document:` / `class Workbook:` |
//! | TypeScript | `bindings/mjx-wasm/src/*.rs` | every `#[wasm_bindgen(js_name = "…")]` attached to a `pub fn` inside the same three `impl` blocks |
//!
//! The TypeScript side is the strongest of the three, because it is checked *against the Rust half
//! of the same chain*: the documented camelCase name has to be the `js_name` the binding declares
//! for that exact `snake_case` method. A chain that renamed one half and not the other fails even
//! though both names exist.
//!
//! # The floors, and what they say
//!
//! Every assertion here is of the form "for each X, …", and every one of them is green when there
//! are no X. The floors are therefore stated as **the parser is working** — chains were found on
//! all three format pages, artefacts were found, and there are enough of each for a partial parse
//! to be visible — rather than as *the checklist is this size*, which would fire ahead of the
//! comparison it is meant to protect and leave the real assertion unexecuted. That distinction was
//! measured rather than guessed: it is the shape `xtask/tests/validation_index.rs` records for its
//! own two floors.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use xtask::validation::{ArtefactFormat, Variant, AREAS};

/// The workspace root — `xtask/..`.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Every `.md` under `docs/validation/`, in a stable order.
fn validation_pages() -> Vec<PathBuf> {
    let mut pages: Vec<PathBuf> = std::fs::read_dir(root().join("docs/validation"))
        .expect("docs/validation")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect();
    pages.sort();
    pages
}

/// The text of every code span on one line, in order.
fn code_spans(line: &str) -> Vec<&str> {
    let mut spans = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find('`') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('`') else { break };
        spans.push(&rest[..close]);
        rest = &rest[close + 1..];
    }
    spans
}

// ---------------------------------------------------------------------------------------------
// The three surfaces
// ---------------------------------------------------------------------------------------------

/// The three facade types a call chain may name.
const FACADE_TYPES: [&str; 3] = ["Deck", "Document", "Workbook"];

/// Every `pub fn` the facade declares on each of the three types.
///
/// Read by walking each file and tracking which `impl` block the cursor is in, rather than by
/// trusting the directory name: a helper `impl` in the same file would otherwise contribute its
/// private vocabulary to the type's.
fn rust_surface() -> BTreeMap<&'static str, BTreeSet<String>> {
    let mut surface: BTreeMap<&'static str, BTreeSet<String>> =
        FACADE_TYPES.iter().map(|t| (*t, BTreeSet::new())).collect();
    let facade = root().join("crates/mjx-ooxml/src");
    let mut files = vec![
        facade.join("deck.rs"),
        facade.join("document.rs"),
        facade.join("workbook.rs"),
    ];
    for directory in ["deck", "document", "workbook"] {
        for entry in std::fs::read_dir(facade.join(directory)).expect("a facade directory") {
            files.push(entry.expect("a directory entry").path());
        }
    }
    for file in files {
        let text = read(&file);
        let mut current: Option<&'static str> = None;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("impl ") {
                // `impl Deck {` in the type's own file, `impl super::Deck {` in its submodules —
                // both spellings are in the tree and reading only one of them was worth eleven
                // methods instead of a hundred and thirty.
                let name = rest
                    .trim_end_matches(" {")
                    .trim()
                    .trim_start_matches("super::");
                current = FACADE_TYPES.iter().copied().find(|t| *t == name);
                continue;
            }
            if line == "}" {
                current = None;
                continue;
            }
            let Some(kind) = current else { continue };
            if let Some(name) = method_name(line, "    pub fn ") {
                surface
                    .get_mut(kind)
                    .expect("a known facade type")
                    .insert(name);
            }
        }
    }
    surface
}

/// The identifier a declaration line names, when the line starts with `prefix`.
fn method_name(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix(prefix)?;
    let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))?;
    (end > 0).then(|| rest[..end].to_owned())
}

/// Every `def` the committed Python stub declares on each of the three classes.
fn python_surface() -> BTreeMap<&'static str, BTreeSet<String>> {
    let mut surface: BTreeMap<&'static str, BTreeSet<String>> =
        FACADE_TYPES.iter().map(|t| (*t, BTreeSet::new())).collect();
    let text = read(&root().join("bindings/mjx-python/python/mjx_ooxml/__init__.pyi"));
    let mut current: Option<&'static str> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("class ") {
            let name = rest.trim_end_matches(':').trim();
            current = FACADE_TYPES.iter().copied().find(|t| *t == name);
            continue;
        }
        let Some(kind) = current else { continue };
        if let Some(name) = method_name(line, "    def ") {
            surface
                .get_mut(kind)
                .expect("a known facade type")
                .insert(name);
        }
    }
    surface
}

/// For each of the three types, the `js_name` the WebAssembly binding declares for each
/// `snake_case` method.
///
/// This is what makes the TypeScript half of a chain checkable rather than merely spelled: the
/// documented camelCase name is compared against the name the binding actually publishes for the
/// Rust method the same chain names.
fn wasm_surface() -> BTreeMap<&'static str, BTreeMap<String, String>> {
    let mut surface: BTreeMap<&'static str, BTreeMap<String, String>> =
        FACADE_TYPES.iter().map(|t| (*t, BTreeMap::new())).collect();
    for entry in std::fs::read_dir(root().join("bindings/mjx-wasm/src")).expect("the wasm source") {
        let path = entry.expect("a directory entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text = read(&path);
        let mut current: Option<&'static str> = None;
        let mut pending: Option<String> = None;
        // An attribute may run over several lines — `#[allow(…, reason = "…")]` does, and
        // `add_floating_chart` carries exactly that between its `js_name` and its `pub fn`. The
        // scanner therefore joins an attribute until its brackets balance rather than treating
        // every physical line as an item, which is what made that one method invisible.
        let mut attribute = String::new();
        let mut depth = 0isize;
        for line in text.lines() {
            let trimmed = line.trim();
            if depth == 0 && !trimmed.starts_with('#') {
                if let Some(rest) = trimmed.strip_prefix("impl ") {
                    let name = rest
                        .trim_end_matches('{')
                        .trim()
                        .trim_start_matches("super::");
                    current = FACADE_TYPES.iter().copied().find(|t| *t == name);
                    pending = None;
                    continue;
                }
                if trimmed == "}" && line == "}" {
                    current = None;
                    pending = None;
                    continue;
                }
                if let Some(name) = method_name(trimmed, "pub fn ") {
                    if let (Some(kind), Some(js)) = (current, pending.take()) {
                        surface
                            .get_mut(kind)
                            .expect("a known facade type")
                            .insert(name, js);
                    }
                    continue;
                }
                // Anything that is neither an attribute, a doc comment nor blank ends the run of
                // attributes a `js_name` could belong to, so it cannot be read onto a later item.
                if !trimmed.starts_with("///") && !trimmed.starts_with("//") && !trimmed.is_empty()
                {
                    pending = None;
                }
                continue;
            }
            if depth == 0 {
                attribute.clear();
            }
            attribute.push_str(trimmed);
            depth += bracket_balance(trimmed);
            if depth <= 0 {
                depth = 0;
                if let Some(js) = js_name_of(&attribute) {
                    pending = Some(js);
                }
                attribute.clear();
            }
        }
    }
    surface
}

/// How far one line opens or closes square brackets — the signed difference, used to tell an
/// attribute that ends on its own line from one that runs over several.
fn bracket_balance(line: &str) -> isize {
    line.chars()
        .fold(0isize, |depth, character| match character {
            '[' => depth + 1,
            ']' => depth - 1,
            _ => depth,
        })
}

/// The `js_name = "…"` an attribute declares, if it declares one.
fn js_name_of(line: &str) -> Option<String> {
    if !line.starts_with("#[wasm_bindgen(") {
        return None;
    }
    let rest = line.split_once("js_name = \"")?.1;
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

// ---------------------------------------------------------------------------------------------
// What the documents say
// ---------------------------------------------------------------------------------------------

/// One `Calls:` line, as a document states it.
#[derive(Debug)]
struct Chain {
    page: PathBuf,
    line: String,
    rust: String,
    python: String,
    typescript: String,
}

/// A `Type::method` / `Type.method` pair, split.
fn split_call<'a>(call: &'a str, separator: &str) -> Option<(&'a str, &'a str)> {
    call.split_once(separator)
}

fn documented_chains() -> Vec<Chain> {
    let mut chains = Vec::new();
    for page in validation_pages() {
        for line in read(&page).lines() {
            let trimmed = line.trim();
            let Some(rest) = trimmed.strip_prefix("Calls:") else {
                continue;
            };
            let spans = code_spans(rest);
            assert_eq!(
                spans.len(),
                3,
                "{}: a `Calls:` line must carry exactly three code spans — Rust, Python, \
                 TypeScript — and this one carries {}:\n  {trimmed}",
                page.display(),
                spans.len()
            );
            chains.push(Chain {
                page: page.clone(),
                line: trimmed.to_owned(),
                rust: spans[0].to_owned(),
                python: spans[1].to_owned(),
                typescript: spans[2].to_owned(),
            });
        }
    }
    chains
}

/// One `**Artefact**` line, as a document states it.
#[derive(Debug)]
struct ArtefactLine {
    page: PathBuf,
    line: String,
    named: Vec<String>,
    blocked: bool,
}

fn documented_artefacts() -> Vec<ArtefactLine> {
    let mut lines = Vec::new();
    for page in validation_pages() {
        for line in read(&page).lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("- **Artefact**") {
                continue;
            }
            lines.push(ArtefactLine {
                page: page.clone(),
                line: trimmed.to_owned(),
                // A check's artefact line also carries prose: a command it quotes, and the
                // markup the artefact is interesting for. A *file* is a span with no whitespace
                // that is either a path or a generated artefact's name, and every one of those
                // has to resolve.
                named: code_spans(trimmed)
                    .into_iter()
                    .filter(|span| names_a_file(span))
                    .map(str::to_owned)
                    .collect(),
                blocked: trimmed.contains("**blocked**"),
            });
        }
    }
    lines
}

// ---------------------------------------------------------------------------------------------
// The floors
// ---------------------------------------------------------------------------------------------

/// Enough of each that a parser which stopped matching part-way is visible.
const MINIMUM_CHAINS: usize = 30;
/// The same, for artefact lines.
const MINIMUM_ARTEFACT_LINES: usize = 20;

/// The floor: chains were found, on all three format pages, and enough of them.
fn assert_chain_parser_is_working(chains: &[Chain]) {
    assert!(
        chains.len() >= MINIMUM_CHAINS,
        "only {} call chain(s) were parsed out of docs/validation/ — the parser is not reaching \
         the pages, and every comparison below would pass on an almost empty list",
        chains.len()
    );
    for format in ArtefactFormat::all() {
        let page = format.page();
        assert!(
            chains.iter().any(|chain| chain.page.ends_with(page)),
            "not one call chain was parsed out of docs/validation/{page}; the parser is reaching \
             some of the pages and not that one"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The gates
// ---------------------------------------------------------------------------------------------

#[test]
fn every_documented_call_chain_names_a_method_that_exists_in_all_three_languages() {
    let chains = documented_chains();
    assert_chain_parser_is_working(&chains);

    let rust = rust_surface();
    let python = python_surface();
    let wasm = wasm_surface();
    for kind in FACADE_TYPES {
        assert!(
            rust[kind].len() >= 50,
            "only {} `pub fn` were read off `impl {kind}` in the facade; the Rust scanner is not \
             matching and every chain below would pass against an almost empty set",
            rust[kind].len()
        );
        assert!(
            python[kind].len() >= 50,
            "only {} `def` were read off `class {kind}` in the Python stub; the stub scanner is \
             not matching",
            python[kind].len()
        );
        assert!(
            wasm[kind].len() >= 50,
            "only {} `js_name` were read off `impl {kind}` in the WebAssembly binding; the \
             attribute scanner is not matching",
            wasm[kind].len()
        );
    }

    for chain in &chains {
        let (rust_type, rust_method) = split_call(&chain.rust, "::").unwrap_or_else(|| {
            panic!(
                "{}: the Rust half of a chain must read `Type::method`:\n  {}",
                chain.page.display(),
                chain.line
            )
        });
        let (python_type, python_method) = split_call(&chain.python, ".").unwrap_or_else(|| {
            panic!(
                "{}: the Python half of a chain must read `Type.method`:\n  {}",
                chain.page.display(),
                chain.line
            )
        });
        let (ts_type, ts_method) = split_call(&chain.typescript, ".").unwrap_or_else(|| {
            panic!(
                "{}: the TypeScript half of a chain must read `Type.method`:\n  {}",
                chain.page.display(),
                chain.line
            )
        });
        assert!(
            rust_type == python_type && rust_type == ts_type,
            "{}: the three halves of a chain name different types ({rust_type}, {python_type}, \
             {ts_type}):\n  {}",
            chain.page.display(),
            chain.line
        );
        let surface = rust.get(rust_type).unwrap_or_else(|| {
            panic!(
                "{}: {rust_type} is not one of the three facade types:\n  {}",
                chain.page.display(),
                chain.line
            )
        });
        assert!(
            surface.contains(rust_method),
            "{}: `{rust_type}::{rust_method}` is on no `impl {rust_type}` in the facade:\n  {}",
            chain.page.display(),
            chain.line
        );
        assert!(
            python[python_type].contains(python_method),
            "{}: `{python_type}.{python_method}` is on no `class {python_type}` in \
             bindings/mjx-python/python/mjx_ooxml/__init__.pyi:\n  {}",
            chain.page.display(),
            chain.line
        );
        let published = wasm[ts_type].get(rust_method).unwrap_or_else(|| {
            panic!(
                "{}: the WebAssembly binding publishes no `{rust_method}` on `impl {ts_type}`:\n  \
                 {}",
                chain.page.display(),
                chain.line
            )
        });
        assert_eq!(
            published, ts_method,
            "{}: the WebAssembly binding publishes `{rust_method}` as `{published}`, and the chain \
             says `{ts_method}`:\n  {}",
            chain.page.display(),
            chain.line
        );
    }
    println!(
        "calls: {} chain(s) across {} page(s), each resolved in Rust, Python and TypeScript",
        chains.len(),
        validation_pages().len()
    );
}

#[test]
fn every_artefact_a_check_names_is_a_file_this_repository_produces() {
    let lines = documented_artefacts();
    assert!(
        lines.len() >= MINIMUM_ARTEFACT_LINES,
        "only {} `**Artefact**` line(s) were parsed out of docs/validation/ — the parser is not \
         reaching the checks",
        lines.len()
    );

    let generated: BTreeSet<String> = AREAS
        .iter()
        .flat_map(|area| {
            [
                area.artefact_name(Variant::Authored),
                area.artefact_name(Variant::Edited),
            ]
        })
        .collect();
    assert!(
        generated.len() >= 20,
        "only {} generated artefact name(s) came out of `AREAS`",
        generated.len()
    );

    let mut resolved = 0usize;
    let mut blocked = 0usize;
    for line in &lines {
        if line.named.is_empty() {
            assert!(
                line.blocked,
                "{}: an `**Artefact**` line names no file and does not say **blocked**:\n  {}",
                line.page.display(),
                line.line
            );
            blocked += 1;
            continue;
        }
        for name in &line.named {
            assert!(
                artefact_exists(name, &generated),
                "{}: the artefact `{name}` is produced by nothing in this repository — it is not \
                 a name `xtask validation-artefacts` writes, not a file under tests/fixtures/, and \
                 not an example's source:\n  {}",
                line.page.display(),
                line.line
            );
            resolved += 1;
        }
    }
    assert!(
        resolved >= MINIMUM_ARTEFACT_LINES,
        "only {resolved} artefact name(s) resolved; the span scanner is matching almost nothing"
    );
    println!(
        "artefacts: {resolved} name(s) resolved across {} line(s), {blocked} blocked and named as \
         such",
        lines.len()
    );
}

/// Whether a code span is *shaped* like a file name at all — a path, or the name of an artefact
/// the generator writes. Anything else on an artefact line is prose: the command that produces the
/// file, or the markup the file is interesting for.
fn names_a_file(span: &str) -> bool {
    if span.contains(char::is_whitespace) {
        return false;
    }
    span.contains('/')
        || ArtefactFormat::all()
            .into_iter()
            .any(|format| span.ends_with(&format!(".{}", format.extension())))
}

/// Whether one code span from an `**Artefact**` line names a file something here produces.
fn artefact_exists(name: &str, generated: &BTreeSet<String>) -> bool {
    if generated.contains(name) {
        return true;
    }
    // A committed fixture, an example's source, or a document carried in from earlier work. Each
    // is a path from the workspace root, and each has to be on disk.
    let allowed_prefix = name.starts_with("tests/fixtures/")
        || (name.starts_with("crates/") && name.contains("/examples/"))
        || name.starts_with("docs/");
    allowed_prefix && root().join(name).is_file()
}
