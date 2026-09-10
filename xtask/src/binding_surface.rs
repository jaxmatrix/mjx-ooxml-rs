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

// ===============================================================================================
// Data tokens — the string literals a binding *hands back*, as opposed to the names it is called by
// ===============================================================================================

/// One string literal a binding's own source spells **in code**.
///
/// "In code" is the whole discriminator, and it is what keeps a token check from condemning the
/// binding's names: a JavaScript method name is only ever written in a `js_name` attribute or as a
/// Rust identifier, and a Python one in a `#[pyo3(name = …)]`. Neither is ever a literal in a
/// function body. So attribute lines and comment lines are dropped, and what is left is data.
#[derive(Debug, Clone)]
pub struct SourceLiteral {
    /// The file it is written in, for a failure message.
    pub file: String,
    /// The `impl` target it sits under, or `<module>` for a free item.
    pub owner: String,
    /// The enclosing `fn`, or `<item>` for a literal outside one.
    pub member: String,
    /// Its one-based line number.
    pub line: usize,
    /// The literal's text, as written, escapes and all.
    pub value: String,
    /// Whether it stands where a value is **produced**: on either side of a `match` arm's `=>`, or
    /// immediately before `.to_owned()`.
    ///
    /// This is the narrower of the two questions this type answers. Every literal is a candidate
    /// for the casing rule — a data key read off a JavaScript object is data as much as a returned
    /// token is — but only a produced one is comparable across the two bindings, because only a
    /// produced one is something a *caller* receives. A message handed to `expect` or to
    /// `invalid_argument` is neither.
    pub produced: bool,
}

/// Every string literal `bindings/mjx-wasm/src/*.rs` spells in code.
///
/// # Panics
/// If the directory cannot be read, or the scan finds implausibly few literals.
#[must_use]
pub fn wasm_source_literals(root: &Path) -> Vec<SourceLiteral> {
    source_literals(&root.join("bindings/mjx-wasm/src"))
}

/// Every string literal `bindings/mjx-python/src/*.rs` spells in code.
///
/// # Panics
/// If the directory cannot be read, or the scan finds implausibly few literals.
#[must_use]
pub fn python_source_literals(root: &Path) -> Vec<SourceLiteral> {
    source_literals(&root.join("bindings/mjx-python/src"))
}

/// Every string literal one binding's sources spell in code, attributed to the `impl` and `fn` it
/// sits in.
///
/// The walk is brace-matched over lines with **string literals masked out first**, so a
/// `format!("{index}")` cannot close a block that is still open — a hazard [`block_end`] beside it
/// carries and this one does not. Character literals are deliberately left alone, because telling
/// `'{'` apart from the lifetime in `impl<'a>` needs a lexer;
/// `a_delimiter_is_never_written_as_a_character_literal` in `xtask/tests/binding_projection.rs` is
/// what makes that shortcut safe to keep.
///
/// # Panics
/// If the directory cannot be read, or the scan finds implausibly few literals — a scanner that has
/// stopped matching is a gate that has stopped asking.
#[must_use]
pub fn source_literals(directory: &Path) -> Vec<SourceLiteral> {
    let mut found = Vec::new();
    for (file, text) in sources(directory, "rs") {
        let mut owners: Vec<(String, i32)> = Vec::new();
        let mut owner = String::from("<module>");
        let mut member: Option<(String, i32)> = None;
        let mut depth = 0i32;
        let mut attribute = 0i32;
        for (offset, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            // An attribute, possibly spanning lines: `js_name`, `pyo3(name = …)` and
            // `typescript_type` all live here, and every one of them is a *name*.
            if attribute > 0 || trimmed.starts_with("#[") {
                attribute += bracket_balance(line);
                attribute = attribute.max(0);
                continue;
            }
            let code = code_of(line);
            let opened_impl = impl_target(code);
            let opened_fn = function_name(code);
            for (column, value) in string_literals(code) {
                let before = code[..column].trim_end();
                let after = code[column + value.written_len()..].trim_start();
                found.push(SourceLiteral {
                    file: file.clone(),
                    owner: owner.clone(),
                    member: member
                        .as_ref()
                        .map_or_else(|| String::from("<item>"), |(name, _)| name.clone()),
                    line: offset + 1,
                    produced: before.ends_with("=>")
                        || after.starts_with("=>")
                        || after.starts_with(".to_owned()"),
                    value: value.text,
                });
            }
            if member.is_none() {
                if let Some(name) = opened_fn {
                    member = Some((name, depth));
                }
            }
            if let Some(name) = opened_impl {
                owners.push((owner, depth));
                owner = name;
            }
            depth += brace_balance(code);
            if member.as_ref().is_some_and(|(_, opened)| depth <= *opened) {
                member = None;
            }
            while owners.last().is_some_and(|(_, opened)| depth <= *opened) {
                let (previous, _) = owners.pop().expect("just checked");
                owner = previous;
            }
        }
    }
    assert!(
        found.len() > 100,
        "only {} string literal(s) found under {} — the scan has stopped matching",
        found.len(),
        directory.display()
    );
    found
}

/// One string literal as it was written, and as it reads.
struct WrittenLiteral {
    /// The literal's text with the delimiters removed, escapes left as written.
    text: String,
}

impl WrittenLiteral {
    /// How many bytes the literal occupies in the source, delimiters included.
    fn written_len(&self) -> usize {
        self.text.len() + 2
    }
}

/// `line` with any trailing `//` comment removed, string-aware so a `"//"` inside a literal stays.
///
/// A line that is *only* a comment becomes empty, which is what drops doc comments: the prose the
/// two bindings write is `xtask/tests/binding_doc_parity.rs`'s question, not this one's.
fn code_of(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut index = 0usize;
    let mut in_string = false;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if in_string => index += 1,
            b'"' => in_string = !in_string,
            b'/' if !in_string && bytes.get(index + 1) == Some(&b'/') => return &line[..index],
            _ => {}
        }
        index += 1;
    }
    line
}

/// Every string literal in `code`, as `(byte offset of the opening quote, the literal)`.
fn string_literals(code: &str) -> Vec<(usize, WrittenLiteral)> {
    let bytes = code.as_bytes();
    let mut found = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'\'' {
            // A character literal or a lifetime. Either way it holds no string.
            index += 1;
            continue;
        }
        if bytes[index] != b'"' {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        let inner = index;
        while index < bytes.len() && bytes[index] != b'"' {
            index += if bytes[index] == b'\\' { 2 } else { 1 };
        }
        if index >= bytes.len() {
            break;
        }
        found.push((
            start,
            WrittenLiteral {
                text: code[inner..index].to_owned(),
            },
        ));
        index += 1;
    }
    found
}

/// The name of the `fn` a line declares, whatever its visibility, if it declares one.
#[must_use]
pub fn function_name(line: &str) -> Option<String> {
    let mut rest = line.trim_start();
    for prefix in [
        "pub(crate) ",
        "pub(super) ",
        "pub ",
        "const ",
        "async ",
        "unsafe ",
    ] {
        if let Some(after) = rest.strip_prefix(prefix) {
            rest = after;
        }
    }
    let rest = rest.strip_prefix("fn ")?;
    let name = identifier_prefix(rest);
    (!name.is_empty()).then_some(name)
}

/// `{` minus `}` on a line, counting neither inside a string or character literal.
fn brace_balance(code: &str) -> i32 {
    balance(code, b'{', b'}')
}

/// `[` minus `]` on a line, counting neither inside a string or character literal.
fn bracket_balance(code: &str) -> i32 {
    balance(code_of(code), b'[', b']')
}

/// `open` minus `close` on a line, ignoring both inside a string literal.
///
/// **Character literals are deliberately not skipped.** Skipping them would mean telling `'{'`
/// apart from the lifetime in `impl<'a>`, which needs a lexer; not skipping them costs nothing,
/// because neither binding's sources hold a character literal for any of the four delimiters this
/// is asked about. That is checked rather than assumed —
/// `a_delimiter_is_never_written_as_a_character_literal` in
/// `xtask/tests/binding_projection.rs` is what makes the shortcut safe to keep.
fn balance(code: &str, open: u8, close: u8) -> i32 {
    let bytes = code.as_bytes();
    let mut depth = 0i32;
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'"' {
                    index += if bytes[index] == b'\\' { 2 } else { 1 };
                }
            }
            byte if byte == open => depth += 1,
            byte if byte == close => depth -= 1,
            _ => {}
        }
        index += 1;
    }
    depth
}

// ===============================================================================================
// Documented members — the prose beside a projected method, and the body under it
// ===============================================================================================

/// One `///`-documented member of one binding.
///
/// The prose is what a caller reads: PyO3 compiles each `///` comment verbatim into the member's
/// `__doc__` and `bindings/mjx-python/tools/stub_docs.py` copies that into the committed `.pyi`,
/// while `wasm-bindgen` writes the same comment into `mjx_ooxml.d.ts`. The body is what the caller
/// actually gets, and holding those two against each other is
/// `every_token_vocabulary_a_binding_documents_is_the_one_its_code_answers`'s whole question.
#[derive(Debug, Clone)]
pub struct DocumentedMember {
    /// The file it is declared in, for a failure message.
    pub file: String,
    /// The one-based line its `fn` stands on.
    pub line: usize,
    /// The `///` comment, every line joined with one space.
    pub prose: String,
    /// How many arguments the projected method takes, `self` and PyO3's `Python<'_>` token
    /// excluded — see [`arity`].
    pub arity: usize,
    /// Its body as written, from the `fn` line to the line before the next member's first `///` or
    /// attribute — comments and all, and deliberately not brace-matched, because the question asked
    /// of it is only *which names does this member mention*.
    pub body: String,
}

/// Every `///`-documented `fn` inside an `impl` block annotated with `marker`, keyed by
/// `(owner, Rust name)`.
///
/// Deliberately over `src/` rather than over the generated `.d.ts` or the committed `.pyi`: the
/// `.d.ts` is build output and git-ignored, and the `.pyi`'s docstrings are themselves generated
/// from these comments (MJXOFF-234). The `///` comments *are* both surfaces' prose.
///
/// The three needles below are this scanner's own and **not** the module's public
/// [`impl_target`], [`block_end`] and [`function_name`]: those answer questions asked of a
/// `#[wasm_bindgen]` export list, and are deliberately laxer — [`impl_target`] accepts
/// `impl Trait for Type`, which this scan must not, because a trait implementation projects
/// nothing. Two needles with two names beats one needle that is subtly wrong for one of its two
/// callers.
///
/// # Panics
/// If the directory cannot be read.
#[must_use]
pub fn documented_members(
    directory: &Path,
    marker: &str,
) -> std::collections::BTreeMap<(String, String), DocumentedMember> {
    let mut found = std::collections::BTreeMap::new();
    for (file, text) in sources(directory, "rs") {
        let lines: Vec<&str> = text.lines().collect();
        let mut index = 0;
        while index < lines.len() {
            let Some(owner) = documented_impl_target(lines[index]) else {
                index += 1;
                continue;
            };
            if !lines[index.saturating_sub(6)..index]
                .iter()
                .any(|line| line.trim() == marker)
            {
                index += 1;
                continue;
            }
            let end = documented_block_end(&lines, index);
            let mut prose: Vec<String> = Vec::new();
            let mut at = index + 1;
            while at <= end {
                let line = lines[at].trim();
                if let Some(rest) = line.strip_prefix("///") {
                    prose.push(rest.trim().to_owned());
                } else if let Some(name) = documented_function_name(line) {
                    if !prose.is_empty() {
                        found.insert(
                            (owner.clone(), name),
                            DocumentedMember {
                                file: file.clone(),
                                line: at + 1,
                                prose: prose.join(" ").trim().to_owned(),
                                arity: arity(&documented_signature(&lines, at, end)),
                                body: documented_body(&lines, at, end),
                            },
                        );
                    }
                    prose.clear();
                } else if !line.is_empty() && !line.starts_with("#[") && !line.starts_with("//") {
                    prose.clear();
                }
                at += 1;
            }
            index = end + 1;
        }
    }
    found
}

/// `impl Foo {` — the type an inherent `impl` block is written for, or `None` for anything else,
/// `impl Trait for Type` included.
fn documented_impl_target(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("impl ")?;
    let name = identifier_prefix(rest);
    let tail = rest[name.len()..].trim();
    (!name.is_empty() && (tail == "{" || tail.is_empty())).then_some(name)
}

/// The index of the line closing the block opened at `from`.
fn documented_block_end(lines: &[&str], from: usize) -> usize {
    let mut depth = 0i32;
    for (offset, line) in lines[from..].iter().enumerate() {
        depth += i32::try_from(line.matches('{').count()).unwrap_or(0);
        depth -= i32::try_from(line.matches('}').count()).unwrap_or(0);
        if depth == 0 && offset > 0 {
            return from + offset;
        }
    }
    lines.len() - 1
}

/// `pub fn name(` / `fn name(` — the Rust name, or `None` for anything else.
///
/// The `<` is not optional decoration: `bindings/mjx-python/src/deck.rs` writes
/// `fn save<'py>(&self, python: Python<'py>)`, and a needle that demands `(` immediately after the
/// name misses every `PyBytes`-returning method in the crate — which is `save`, `save_unchecked`
/// and every `*_bytes` reader, the members most worth comparing.
fn documented_function_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("pub ").unwrap_or(line);
    let rest = rest.strip_prefix("fn ")?;
    let name = identifier_prefix(rest);
    let after = &rest[name.len()..];
    (!name.is_empty() && (after.starts_with('(') || after.starts_with('<'))).then_some(name)
}

/// The text between the parentheses of the signature beginning on line `at`, however many lines it
/// is wrapped across.
fn documented_signature(lines: &[&str], at: usize, end: usize) -> String {
    let mut text = String::new();
    for line in &lines[at..=end] {
        text.push_str(line.trim());
        text.push(' ');
        if text.matches('(').count() > 0 && text.matches('(').count() == text.matches(')').count() {
            break;
        }
    }
    let Some(open) = text.find('(') else {
        return String::new();
    };
    let mut depth = 0usize;
    for (offset, character) in text[open..].char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return text[open + 1..open + offset].to_owned();
                }
            }
            _ => {}
        }
    }
    String::new()
}

/// The member beginning on line `at`, up to the line before the next member's first `///` or
/// attribute, or the end of the `impl` block.
///
/// Brace matching is deliberately not used. It would have to be string-aware to survive a
/// `format!("{index}")`, and the only question asked of a body here is which *names* it mentions —
/// a question a few trailing blank lines cannot change the answer to, and one that a body cut short
/// by an unbalanced brace would silently get wrong.
fn documented_body(lines: &[&str], at: usize, end: usize) -> String {
    let mut last = at;
    while last < end {
        let next = lines[last + 1].trim();
        if next.starts_with("///") || next.starts_with("#[") {
            break;
        }
        last += 1;
    }
    lines[at..=last].join("\n")
}

/// How many arguments a signature takes, excluding `self` and PyO3's `Python<'_>` token.
///
/// The token is machinery rather than an argument of the projected method: counting it would put
/// `Deck.open` at two against JavaScript's one and exclude thirty-three pairs that are the same
/// method.
#[must_use]
pub fn arity(signature: &str) -> usize {
    let mut depth = 0i32;
    let mut arguments = Vec::new();
    let mut current = String::new();
    for character in signature.chars() {
        match character {
            '(' | '<' | '[' => depth += 1,
            ')' | '>' | ']' => depth -= 1,
            ',' if depth == 0 => {
                arguments.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(character);
    }
    arguments.push(current);
    arguments
        .iter()
        .map(|argument| argument.trim())
        .filter(|argument| !argument.is_empty())
        .filter(|argument| {
            let bare = argument.trim_start_matches('&').trim_start();
            let bare = bare.strip_prefix("mut ").unwrap_or(bare);
            !bare.starts_with("self")
        })
        .filter(|argument| !argument.contains("Python<") && !argument.ends_with(": Python"))
        .count()
}
