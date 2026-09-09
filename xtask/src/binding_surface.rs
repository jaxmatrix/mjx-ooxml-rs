//! **What each binding actually declares**, read out of the two committed sources (MJXOFF-261).
//!
//! Two gates need the same answer to the same question — *is this name reachable from Python? from
//! JavaScript?* — and they need it for opposite reasons:
//!
//! * `xtask/tests/binding_projection.rs` asks **how much** of each surface the binding's own suite
//!   ever exercises, and whether every JavaScript name is the camel case of the Rust name under it.
//! * `xtask/tests/guide_examples.rs` asks whether a guide block **declared Rust-only** is telling
//!   the truth: the names that make it Rust-only must be reachable from neither binding.
//!
//! Until MJXOFF-261 the extraction lived inside the first of those, where the second could not
//! reach it — an integration test compiles into its own crate. The answer to a second consumer is
//! one implementation both can reach rather than a second parser, which is the same reasoning
//! `CLAUDE.md` records for `mjx-allocation-counter`. **A signature parser that is subtly wrong is
//! worse than none**, and two of them are worse still: they would disagree, and neither would be
//! obviously the wrong one.
//!
//! # The two sources, and why each is the right one
//!
//! | Binding | Source | Why |
//! |---|---|---|
//! | Python | `bindings/mjx-python/python/mjx_ooxml/__init__.pyi` | the surface's own statement in Python, held to the compiled module in both directions by `bindings/mjx-python/tests/test_stub_parity.py` |
//! | JavaScript | `bindings/mjx-wasm/src/*.rs` | the generated `.d.ts` is build output and is git-ignored, so the committed `#[wasm_bindgen]` declarations *are* the surface |
//!
//! That asymmetry is forced and it matters for one thing: a name that appears in a `bindings/
//! mjx-wasm/src/` **comment** is not a declaration, so nothing here reads comments. The escape
//! hatches are the case that proves it — `bindings/mjx-wasm/src/deck.rs` carries a comment naming
//! `presentation_mut` to say the binding does *not* have it, and a text search would read that
//! comment as evidence of the opposite.

use std::collections::BTreeSet;
use std::path::Path;

/// One function `wasm-bindgen` exports, as both languages spell it.
#[derive(Debug, Clone)]
pub struct WasmExport {
    /// The file it is declared in, for a failure message.
    pub file: String,
    /// The type whose `impl` block it sits in, or `<module>` for a free function.
    pub owner: String,
    /// Its Rust name.
    pub rust_name: String,
    /// Its JavaScript name — the `js_name` when one is given, else the Rust name unchanged.
    pub js_name: String,
    /// Whether a `js_name` was given at all.
    pub renamed: bool,
}

/// One member the committed Python stub declares.
#[derive(Debug, Clone)]
pub struct PythonMember {
    /// The class it belongs to, or `<module>` for a free function.
    pub owner: String,
    /// Its name.
    pub name: String,
}

/// Every `.rs`, `.pyi`, `.py` or `.mjs` file in one directory, sorted, read to a string.
///
/// # Panics
/// If the directory cannot be read, or holds no file with that extension — a walk that has stopped
/// matching is a gate that has stopped asking.
#[must_use]
pub fn sources(directory: &Path, extension: &str) -> Vec<(String, String)> {
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

/// Every `pub fn` inside a `#[wasm_bindgen]`-annotated `impl` block, with the name it is exported
/// under, plus the free functions the crate exports.
///
/// The scan is brace-matched rather than line-shaped: an `impl` block is found, its extent is
/// measured by counting braces, and only `pub fn` items inside it are read. A `pub fn` in a plain
/// `impl` block is not exported and is skipped — `bindings/mjx-wasm/src/geometry.rs`'s
/// `AdjustmentSpecRef` constructor is the one such item today.
///
/// # Panics
/// If the scan finds implausibly few exports, which is what a scanner that has stopped matching
/// looks like from the outside.
#[must_use]
pub fn wasm_exports(root: &Path) -> Vec<WasmExport> {
    let directory = root.join("bindings/mjx-wasm/src");
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

/// Every type `wasm-bindgen` exports as a JavaScript class or enumeration.
///
/// Derived from the `impl` targets of [`wasm_exports`] rather than from `pub struct` lines, and the
/// difference is not a shortcut: **most of this crate's classes are declared by a macro**
/// (`value_class!`, `sealed_enums!`, `open_enums!` in `bindings/mjx-wasm/src/support.rs` and
/// `enums.rs`), so a `pub struct` scan finds sixteen of them and misses the rest — a scanner that
/// silently sees a tenth of its subject is worse than none. Every exported class has at least one
/// `#[wasm_bindgen] impl` block written out in a file, because that is where its methods are, and
/// the block names the class.
///
/// # Panics
/// If the scan finds implausibly few types.
#[must_use]
pub fn wasm_types(root: &Path) -> BTreeSet<String> {
    let types: BTreeSet<String> = wasm_exports(root)
        .into_iter()
        .map(|export| export.owner)
        .filter(|owner| owner != "<module>")
        .collect();
    assert!(
        types.len() > 100,
        "only {} wasm type(s) found — the `#[wasm_bindgen] impl` target scan has stopped matching",
        types.len()
    );
    types
}

/// Every method, attribute and free function `bindings/mjx-python/python/mjx_ooxml/__init__.pyi`
/// declares.
///
/// The stub is the surface's own statement in Python —
/// `bindings/mjx-python/tests/test_stub_parity.py` requires it to agree with the compiled module in
/// both directions — so parsing it needs no heuristics: a class opens at column zero, a member is
/// indented by exactly four, a `def` is a method and a `lowercase: Type` line is an attribute.
///
/// **Enumeration members are deliberately excluded**, and their shape is what excludes them: a
/// member of a projected enumeration is `Center: TextAlignment`, capitalised, where an attribute is
/// `font_index: int | None`. `bindings/mjx-python/tests/test_enums.py` already holds every
/// projected enumeration to its Rust member names in both directions.
///
/// # Panics
/// If the stub cannot be read, or the scan finds implausibly few members.
#[must_use]
pub fn python_members(root: &Path) -> Vec<PythonMember> {
    let stub = root.join("bindings/mjx-python/python/mjx_ooxml/__init__.pyi");
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
        classes > 50 && members.len() > 1_400,
        "only {classes} class(es) and {} member(s) found in the stub — the `.pyi` walk has stopped \
         matching",
        members.len()
    );
    members
}

/// Every class the committed Python stub declares.
///
/// # Panics
/// If the stub cannot be read, or the scan finds implausibly few classes.
#[must_use]
pub fn python_classes(root: &Path) -> BTreeSet<String> {
    let stub = root.join("bindings/mjx-python/python/mjx_ooxml/__init__.pyi");
    let text = std::fs::read_to_string(&stub)
        .unwrap_or_else(|error| panic!("reading {}: {error}", stub.display()));
    let classes: BTreeSet<String> = text
        .lines()
        .filter_map(|line| line.strip_prefix("class "))
        .map(identifier_prefix)
        .filter(|name| !name.is_empty())
        .collect();
    assert!(
        classes.len() > 50,
        "only {} class(es) found in the stub — the scan has stopped matching",
        classes.len()
    );
    classes
}

/// `impl Thing` / `impl<'a> Thing` — the type an `impl` block is for, if the line opens one.
#[must_use]
pub fn impl_target(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("impl ")?;
    let rest = if let Some(after) = rest.strip_prefix('<') {
        after.split_once("> ")?.1
    } else {
        rest
    };
    let name = identifier_prefix(rest);
    (!name.is_empty()).then_some(name)
}

/// Whether the item at `index` carries a `#[wasm_bindgen…]` attribute directly above it.
#[must_use]
pub fn preceded_by_wasm_bindgen(lines: &[&str], index: usize) -> bool {
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
///
/// # Panics
/// Never in practice: only if a single line held more than `i32::MAX` braces.
#[must_use]
pub fn block_end(lines: &[&str], index: usize) -> usize {
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
#[must_use]
pub fn js_name_of(line: &str) -> Option<String> {
    let after = line.split_once("js_name")?.1;
    let after = after.trim_start().strip_prefix('=')?.trim_start();
    let inner = after.strip_prefix('"')?;
    let (name, _) = inner.split_once('"')?;
    Some(name.to_owned())
}

/// The name of the `pub fn` a line declares, if it declares one.
#[must_use]
pub fn public_function_name(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("pub fn ")?;
    let name = identifier_prefix(rest);
    (!name.is_empty()).then_some(name)
}

/// The leading identifier of `text` — everything up to the first character that cannot be in one.
#[must_use]
pub fn identifier_prefix(text: &str) -> String {
    text.chars()
        .take_while(|character| character.is_alphanumeric() || *character == '_')
        .collect()
}

/// Every name a Python caller can write: the classes, and every method, attribute and free
/// function on them.
///
/// # Panics
/// If either scan above does.
#[must_use]
pub fn python_names(root: &Path) -> BTreeSet<String> {
    let mut names = python_classes(root);
    names.extend(python_members(root).into_iter().map(|member| member.name));
    names
}

/// Every name a JavaScript caller can write, **plus the Rust name under each one**.
///
/// Both spellings are in the set on purpose. A guide block declared Rust-only names the *Rust*
/// symbols it uses, and the question that block is asking is whether the binding has the thing at
/// all — which it does whether it spells it `presentation_mut` or `presentationMut`.
///
/// # Panics
/// If either scan above does.
#[must_use]
pub fn wasm_names(root: &Path) -> BTreeSet<String> {
    let mut names = wasm_types(root);
    for export in wasm_exports(root) {
        names.insert(export.rust_name);
        names.insert(export.js_name);
    }
    names
}
