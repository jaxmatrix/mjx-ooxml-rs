//! The projection gate: **how much of each binding's surface its own suite ever touches**, and
//! whether the TypeScript names are derived rather than typed (MJXOFF-226).
//!
//! # The trap this file is written against
//!
//! Each of the three walkthroughs — `crates/mjx-ooxml/examples/build_a_deck.rs`,
//! `build_a_document.rs`, `build_a_workbook.rs` — exists a second time under
//! `bindings/mjx-python/tests/` and a third under `bindings/mjx-wasm/tests/node/`, and every one
//! compares its output against the Rust one part by part, byte for byte. That is a strong gate and
//! it proves the projection is **wired**. It does not prove it is right **across the surface**:
//!
//! > Our gates reliably ask whether a value *reaches* somebody; they do not ask *at how many
//! > distinct points* the surface was ever exercised.
//!
//! MJXOFF-225 met the same shape in `mjx-ooxml-types`, where a child-order sweep read as complete
//! while covering three of nine tables. A three-language walkthrough over one deck is that shape
//! again, and no reachability check can see it — the walkthrough passes, and the nine tenths of the
//! surface it never calls passes with it, because nothing asked.
//!
//! So this file asks. It counts the members each binding declares, counts the ones some test names
//! in a position that is a *use*, and pins both totals. A method added without a test lowers the
//! second count and not the first, and the assertion fails with the names.
//!
//! # What "exercised" means here, exactly
//!
//! A declared member is **exercised** when its name appears in one of that binding's own test
//! sources in a position that can only be a use of it:
//!
//! * after a `.` — `spec.applies_font`, `deck.setShapeText(…)`;
//! * immediately before a `(` and not after a `.` — a constructor or a free function;
//! * (Python only) as a keyword argument — preceded by `(` or `,` and followed by a single `=`.
//!
//! **The two directions of this measure are not equally strong, and the asymmetry is the point.**
//! A name that appears is an *upper* bound: it may have been called on a different class that
//! happens to share the name, so "exercised" can over-count. A name that appears **nowhere** is
//! exact — no test can call a member whose name is absent from every test source. The number worth
//! trusting is therefore the un-exercised one, and it is the number the guide quotes.
//!
//! # What is deliberately *not* checked here, and why
//!
//! * **Enumeration members.** `bindings/mjx-python/tests/test_enums.py` already holds all one
//!   hundred and two enumerations to their Rust member names in both directions, and
//!   `bindings/mjx-wasm/tests/node/surface.mjs` names every member of the two `Format`
//!   enumerations. A second, weaker check here would add noise, not cover.
//! * **Which class a name was used on.** See the asymmetry above. Resolving `.rows` to a receiver
//!   would need a type checker for two languages; the un-exercised set does not need one.
//! * **The type graph.** Whether every exported class can be *obtained* from some other method is a
//!   real question with two real answers — `mjx_ooxml::ResolvedColor` and
//!   `mjx_ooxml::TableStyleFlags` are exported by both bindings and returned, taken and constructed
//!   by nothing in either — but answering it mechanically needs a signature parser over two
//!   hand-written crates, and a signature parser that is subtly wrong is worse than none. The two
//!   findings are written down in `bindings/mjx-python/docs/guide/what_is_not_projected.md` with
//!   the command that reproduces them.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

// ===============================================================================================
// The corpus
// ===============================================================================================

/// The workspace root — `xtask/`'s parent.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// Every `.rs`, `.pyi`, `.py` or `.mjs` file in one directory, sorted, read to a string.
fn sources(directory: &Path, extension: &str) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("reading {}: {error}", directory.display()))
        .map(|entry| entry.expect("a readable directory entry").path())
        .filter(|path| path.extension().is_some_and(|found| found == extension))
        .map(|path| {
            let name = path
                .file_name()
                .expect("a file has a name")
                .to_string_lossy()
                .into_owned();
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
            (name, text)
        })
        .collect();
    found.sort();
    assert!(
        !found.is_empty(),
        "no `.{extension}` sources under {} — the walk has stopped matching",
        directory.display()
    );
    found
}

// ===============================================================================================
// The surfaces
// ===============================================================================================

/// One function `wasm-bindgen` exports, as both languages spell it.
#[derive(Debug, Clone)]
struct WasmExport {
    /// The file it is declared in, for the failure message.
    file: String,
    /// The type whose `impl` block it sits in.
    owner: String,
    /// Its Rust name.
    rust_name: String,
    /// Its JavaScript name — the `js_name` when one is given, else the Rust name unchanged.
    js_name: String,
    /// Whether a `js_name` was given at all.
    renamed: bool,
}

/// Every `pub fn` inside a `#[wasm_bindgen]`-annotated `impl` block, with the name it is exported
/// under.
///
/// The scan is brace-matched rather than line-shaped: an `impl` block is found, its extent is
/// measured by counting braces, and only `pub fn` items inside it are read. A `pub fn` in a plain
/// `impl` block is not exported and is skipped — `bindings/mjx-wasm/src/geometry.rs`'s
/// `AdjustmentSpecRef` constructor is the one such item today, and the count below prints it.
fn wasm_exports() -> Vec<WasmExport> {
    let directory = repository_root().join("bindings/mjx-wasm/src");
    let mut exports = Vec::new();
    for (file, text) in sources(&directory, "rs") {
        let lines: Vec<&str> = text.lines().collect();
        let mut index = 0;
        while index < lines.len() {
            let Some(owner) = impl_target(lines[index]) else {
                index += 1;
                continue;
            };
            let exported = preceded_by_wasm_bindgen(&lines, index);
            let end = block_end(&lines, index);
            if exported {
                let mut pending: Option<String> = None;
                for line in &lines[index + 1..=end] {
                    if let Some(name) = js_name_of(line) {
                        pending = Some(name);
                    }
                    if let Some(rust_name) = public_function_name(line) {
                        let renamed = pending.is_some();
                        let js_name = pending.take().unwrap_or_else(|| rust_name.clone());
                        exports.push(WasmExport {
                            file: file.clone(),
                            owner: owner.clone(),
                            rust_name,
                            js_name,
                            renamed,
                        });
                    }
                }
            }
            index = end + 1;
        }
        // Free functions: `#[wasm_bindgen(js_name = "…")] pub fn …` at the top level.
        let mut pending: Option<String> = None;
        for line in text.lines() {
            if line.starts_with("#[wasm_bindgen") {
                pending = js_name_of(line);
                continue;
            }
            if let Some(rust_name) = line.strip_prefix("pub fn ") {
                let rust_name = identifier_prefix(rust_name);
                let renamed = pending.is_some();
                let js_name = pending.take().unwrap_or_else(|| rust_name.clone());
                exports.push(WasmExport {
                    file: file.clone(),
                    owner: "<module>".to_owned(),
                    rust_name,
                    js_name,
                    renamed,
                });
            }
        }
    }
    assert!(
        exports.len() > 1_500,
        "only {} wasm exports found — the `#[wasm_bindgen] impl` scan has stopped matching",
        exports.len()
    );
    exports
}

/// `impl Thing` / `impl<'a> Thing` — the type an `impl` block is for, if the line opens one.
fn impl_target(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("impl ")?;
    let rest = if let Some(after) = rest.strip_prefix('<') {
        after.split_once("> ")?.1
    } else {
        rest
    };
    let name = identifier_prefix(rest);
    (!name.is_empty()).then_some(name)
}

/// Whether the `impl` at `index` carries a `#[wasm_bindgen…]` attribute directly above it.
fn preceded_by_wasm_bindgen(lines: &[&str], index: usize) -> bool {
    let mut cursor = index;
    while cursor > 0 {
        cursor -= 1;
        let line = lines[cursor].trim();
        if line.starts_with("#[wasm_bindgen") {
            return true;
        }
        if !(line.starts_with("#[") || line.starts_with("///") || line.is_empty()) {
            return false;
        }
    }
    false
}

/// The index of the line closing the block that opens at `index`.
fn block_end(lines: &[&str], index: usize) -> usize {
    let mut depth = 0i32;
    let mut opened = false;
    for (offset, line) in lines[index..].iter().enumerate() {
        depth += i32::try_from(line.matches('{').count()).expect("a line has few braces");
        depth -= i32::try_from(line.matches('}').count()).expect("a line has few braces");
        if line.contains('{') {
            opened = true;
        }
        if opened && depth <= 0 {
            return index + offset;
        }
    }
    lines.len() - 1
}

/// The `js_name = "…"` a line states, if it states one.
fn js_name_of(line: &str) -> Option<String> {
    let after = line.split_once("js_name")?.1;
    let after = after.trim_start().strip_prefix('=')?.trim_start();
    let inner = after.strip_prefix('"')?;
    let (name, _) = inner.split_once('"')?;
    Some(name.to_owned())
}

/// The name of the `pub fn` a line declares, if it declares one.
fn public_function_name(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("pub fn ")?;
    let name = identifier_prefix(rest);
    (!name.is_empty()).then_some(name)
}

/// The leading identifier of `text` — everything up to the first character that cannot be in one.
fn identifier_prefix(text: &str) -> String {
    text.chars()
        .take_while(|character| character.is_alphanumeric() || *character == '_')
        .collect()
}

/// One member the committed Python stub declares.
#[derive(Debug, Clone)]
struct PythonMember {
    /// The class it belongs to, or `<module>` for a free function.
    owner: String,
    /// Its name.
    name: String,
}

/// Every method, attribute and free function `bindings/mjx-python/python/mjx_ooxml/__init__.pyi`
/// declares.
///
/// The stub is the surface's own statement in Python — `bindings/mjx-python/tests/test_stub_parity.py` requires it to
/// agree with the compiled module in both directions — so parsing it needs no heuristics: a class
/// opens at column zero, a member is indented by exactly four, a `def` is a method and a
/// `lowercase: Type` line is an attribute.
///
/// **Enumeration members are deliberately excluded**, and their shape is what excludes them: a
/// member of a projected enumeration is `Center: TextAlignment`, capitalised, where an attribute is
/// `font_index: int | None`. See this file's header for why they are covered elsewhere.
fn python_members() -> Vec<PythonMember> {
    let stub = repository_root().join("bindings/mjx-python/python/mjx_ooxml/__init__.pyi");
    let text = std::fs::read_to_string(&stub)
        .unwrap_or_else(|error| panic!("reading {}: {error}", stub.display()));
    let mut members = Vec::new();
    let mut owner = String::from("<module>");
    let mut classes = 0usize;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("class ") {
            owner = identifier_prefix(rest);
            classes += 1;
            continue;
        }
        if let Some(rest) = line.strip_prefix("def ") {
            members.push(PythonMember {
                owner: "<module>".to_owned(),
                name: identifier_prefix(rest),
            });
            continue;
        }
        let Some(rest) = line.strip_prefix("    ") else {
            continue;
        };
        if rest.starts_with(' ') {
            continue;
        }
        if let Some(after) = rest.strip_prefix("def ") {
            members.push(PythonMember {
                owner: owner.clone(),
                name: identifier_prefix(after),
            });
            continue;
        }
        let name = identifier_prefix(rest);
        let annotated = rest[name.len()..].trim_start().starts_with(':');
        let lowercase = name.starts_with(|character: char| character.is_ascii_lowercase());
        if annotated && lowercase && !name.is_empty() {
            members.push(PythonMember {
                owner: owner.clone(),
                name,
            });
        }
    }
    assert!(
        classes > 250,
        "only {classes} classes found in the stub — the `.pyi` walk has stopped matching"
    );
    assert!(
        members.len() > 1_400,
        "only {} members found in the stub — the `.pyi` walk has stopped matching",
        members.len()
    );
    members
}

// ===============================================================================================
// "Exercised" — the matcher
// ===============================================================================================

/// Whether `name` appears in `haystack` in a position that can only be a use of a member.
///
/// Hand-rolled rather than a regular expression because `xtask` carries no regex dependency and the
/// rule is three cases wide. See this file's header for what each case means and why the negative
/// answer is the trustworthy one.
fn is_used(haystack: &str, name: &str, keyword_arguments: bool) -> bool {
    let bytes = haystack.as_bytes();
    let needle = name.as_bytes();
    let mut from = 0usize;
    while let Some(offset) = find(&bytes[from..], needle) {
        let start = from + offset;
        let end = start + needle.len();
        from = start + 1;
        let before = start.checked_sub(1).map(|index| bytes[index]);
        let after = bytes.get(end).copied();
        // A whole token: not part of a longer identifier.
        if before.is_some_and(is_identifier_byte) || after.is_some_and(is_identifier_byte) {
            continue;
        }
        // `object.member`
        if before == Some(b'.') {
            return true;
        }
        let next = skip_spaces(bytes, end);
        // `Class(…)` or `free_function(…)`, but not `.method(…)`, which the case above took.
        if bytes.get(next) == Some(&b'(') {
            return true;
        }
        // `call(keyword=…)` — an `=` that is not `==`, after a `(` or a `,`.
        if keyword_arguments
            && bytes.get(next) == Some(&b'=')
            && bytes.get(next + 1) != Some(&b'=')
            && matches!(back_to_delimiter(bytes, start), Some(b'(' | b','))
        {
            return true;
        }
    }
    false
}

/// Whether a byte can appear inside an identifier.
fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// The index of the first byte at or after `index` that is not a space, tab or newline.
fn skip_spaces(bytes: &[u8], index: usize) -> usize {
    let mut cursor = index;
    while matches!(bytes.get(cursor), Some(b' ' | b'\t' | b'\n' | b'\r')) {
        cursor += 1;
    }
    cursor
}

/// The first non-whitespace byte before `index`, if there is one.
fn back_to_delimiter(bytes: &[u8], index: usize) -> Option<u8> {
    let mut cursor = index;
    while cursor > 0 {
        cursor -= 1;
        if !matches!(bytes[cursor], b' ' | b'\t' | b'\n' | b'\r') {
            return Some(bytes[cursor]);
        }
    }
    None
}

/// The first index at which `needle` occurs in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (0..=haystack.len() - needle.len())
        .find(|&index| &haystack[index..index + needle.len()] == needle)
}

/// One member of one class, as a caller has to learn it: the owner and the member's own name.
type Declaration = (String, String);

/// Every declaration in `declared`, split into the ones some test source uses and the ones none
/// does.
///
/// A declaration is an `(owner, member)` pair, because that is what the surface *is*: `rows` on
/// `CellBlock` and `rows` on `Cells` are two members a caller has to learn separately. The matcher
/// underneath works on the name alone, so a name shared by two classes is credited to both from one
/// use — over-counting the exercised set, never the un-exercised one. See this file's header.
fn split_by_use(
    declared: &[Declaration],
    tests: &[(String, String)],
    keyword_arguments: bool,
) -> (Vec<Declaration>, Vec<Declaration>) {
    let names: BTreeSet<&str> = declared.iter().map(|(_, name)| name.as_str()).collect();
    let exercised: BTreeSet<&str> = names
        .into_iter()
        .filter(|name| {
            tests
                .iter()
                .any(|(_, text)| is_used(text, name, keyword_arguments))
        })
        .collect();
    let (used, unused): (Vec<_>, Vec<_>) = declared
        .iter()
        .cloned()
        .partition(|(_, name)| exercised.contains(name.as_str()));
    (used, unused)
}

// ===============================================================================================
// The checks
// ===============================================================================================

/// Every multi-word Rust name reaches JavaScript as its own camelCase, and no other name is
/// renamed.
///
/// `CLAUDE.md` states the rule as *"an explicit `js_name` on every one"*, which is not what the
/// code does and never has been: 138 of the exported functions carry none. Every one of those is a
/// **single word**, where `wasm-bindgen`'s own snake-to-camel pass is the identity, so the surface
/// a TypeScript caller sees is camelCase throughout either way. What matters is not that the
/// attribute is present but that the name it gives is *derived* rather than typed — a hand-written
/// `js_name` is a place a typo lands silently, and `bindings/mjx-wasm/tests/node/surface.mjs`
/// would only catch it on the methods it happens to call.
#[test]
fn every_javascript_name_is_the_camel_case_of_its_rust_name() {
    let exports = wasm_exports();
    let mut mismatched = Vec::new();
    let mut snake_leaks = Vec::new();
    let mut renamed = 0usize;
    let mut forced = 0usize;
    for export in &exports {
        if export.renamed {
            renamed += 1;
            if FORCED_NAMES.contains(&(export.rust_name.as_str(), export.js_name.as_str())) {
                forced += 1;
                continue;
            }
            let expected = camel_case(&export.rust_name);
            if export.js_name != expected {
                mismatched.push(format!(
                    "{}: {}::{} is exported as `{}`, but the camel case of its own name is `{}`",
                    export.file, export.owner, export.rust_name, export.js_name, expected
                ));
            }
        } else if export.rust_name.contains('_') {
            snake_leaks.push(format!(
                "{}: {}::{} carries no `js_name`, so it reaches TypeScript as `{}`",
                export.file, export.owner, export.rust_name, export.rust_name
            ));
        }
    }
    assert!(
        mismatched.is_empty(),
        "a hand-written `js_name` disagrees with its own Rust name:\n  {}",
        mismatched.join("\n  ")
    );
    assert!(
        snake_leaks.is_empty(),
        "a multi-word name reaches TypeScript in snake case:\n  {}",
        snake_leaks.join("\n  ")
    );
    assert!(
        forced == FORCED_SITES,
        "the ledger of names JavaScript itself forces is out of date: {forced} site(s) matched it, \
         {FORCED_SITES} are written down"
    );
    println!(
        "wasm name mapping: {} exported function(s), {renamed} with an explicit `js_name`, \
         {} single-word names that need none, {forced} forced by JavaScript, 0 snake-case leaks",
        exports.len(),
        exports.len() - renamed
    );
}

/// The renames JavaScript itself forces, which the camel-case rule therefore cannot cover.
///
/// `toString` is a protocol, not a name: `String(value)`, template interpolation and every console
/// call reach for it, and a method called `toDisplayString` would be ignored by all three. The Rust
/// side keeps the longer name because `to_string` on a Rust type means `Display`, which these are
/// not. Nothing else in either binding is renamed for any reason but case.
const FORCED_NAMES: &[(&str, &str)] = &[("to_display_string", "toString")];

/// How many exported functions the ledger above accounts for. Exact, so a new hand-written
/// `js_name` cannot hide behind it.
const FORCED_SITES: usize = 7;

/// `some_name` as `someName`.
fn camel_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut capitalise = false;
    for character in name.chars() {
        if character == '_' {
            capitalise = true;
        } else if capitalise {
            out.extend(character.to_uppercase());
            capitalise = false;
        } else {
            out.push(character);
        }
    }
    out
}

/// How much of each binding's surface its own suite ever names — the figure
/// `bindings/mjx-python/docs/guide/how_much_is_exercised.md` quotes.
///
/// **Both totals are exact, and that is deliberate.** A floor would let the exercised share fall
/// one method at a time; an exact pair fails the moment a member is added without a test *or* a
/// test stops calling one, and the failure names the members. Updating the numbers is then a
/// decision somebody takes on purpose, which is the only property a count in prose can have.
#[test]
fn the_share_of_each_binding_its_suite_exercises_is_what_the_guide_says() {
    let python_tests = sources(&repository_root().join("bindings/mjx-python/tests"), "py");
    let node_tests = sources(
        &repository_root().join("bindings/mjx-wasm/tests/node"),
        "mjs",
    );

    let members = python_members();
    let python_declared: Vec<Declaration> = members
        .iter()
        .filter(|member| !member.name.starts_with("__"))
        .map(|member| (member.owner.clone(), member.name.clone()))
        .collect();
    let (python_used, python_unused) = split_by_use(&python_declared, &python_tests, true);

    let exports = wasm_exports();
    let wasm_declared: Vec<Declaration> = exports
        .iter()
        .map(|export| (export.owner.clone(), export.js_name.clone()))
        .collect();
    let (wasm_used, wasm_unused) = split_by_use(&wasm_declared, &node_tests, false);

    println!(
        "binding surface exercised by its own suite:\n  \
         Python       {:>4} of {:>4} declared member(s) ({:.1}%), {} named by no test\n  \
         WebAssembly  {:>4} of {:>4} declared member(s) ({:.1}%), {} named by no test",
        python_used.len(),
        python_declared.len(),
        percentage(python_used.len(), python_declared.len()),
        python_unused.len(),
        wasm_used.len(),
        wasm_declared.len(),
        percentage(wasm_used.len(), wasm_declared.len()),
        wasm_unused.len(),
    );

    // Anti-vacuity: stated as *the matcher is still matching*, never as *the corpus is this size*.
    assert!(
        python_used.len() > 300 && wasm_used.len() > 300,
        "the use matcher has stopped matching: {} Python and {} JavaScript names found in use",
        python_used.len(),
        wasm_used.len()
    );

    assert_eq!(
        (python_used.len(), python_declared.len()),
        (PYTHON_EXERCISED, PYTHON_DECLARED),
        "the Python surface or the share of it the suite exercises has changed.\n\
         Exercised now: {} of {}. Members no test names:\n  {}",
        python_used.len(),
        python_declared.len(),
        joined(&python_unused),
    );
    assert_eq!(
        (wasm_used.len(), wasm_declared.len()),
        (WASM_EXERCISED, WASM_DECLARED),
        "the WebAssembly surface or the share of it the suite exercises has changed.\n\
         Exercised now: {} of {}. Names no test names:\n  {}",
        wasm_used.len(),
        wasm_declared.len(),
        joined(&wasm_unused),
    );
}

/// Members the committed Python stub declares, enumeration members and dunders aside.
const PYTHON_DECLARED: usize = 1_637;
/// How many of [`PYTHON_DECLARED`] some test under `bindings/mjx-python/tests/` names.
const PYTHON_EXERCISED: usize = 990;
/// Functions `wasm-bindgen` exports to JavaScript.
const WASM_DECLARED: usize = 1_761;
/// How many of [`WASM_DECLARED`] some test under `bindings/mjx-wasm/tests/node/` names.
const WASM_EXERCISED: usize = 925;

/// A percentage, or zero when the denominator is.
fn percentage(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "both counts are in the low thousands"
    )]
    let value = 100.0 * part as f64 / whole as f64;
    value
}

/// The declarations as a comma-separated `Owner.member` list, for a failure message.
fn joined(declarations: &[Declaration]) -> String {
    declarations
        .iter()
        .map(|(owner, name)| format!("{owner}.{name}"))
        .collect::<Vec<_>>()
        .join(", ")
}
