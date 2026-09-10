//! The projection gate: **how much of each binding's surface its own suite ever touches**, and
//! whether the TypeScript names are derived rather than typed (MJXOFF-226).
//!
//! # The trap this file is written against
//!
//! Each of the three walkthroughs — `crates/mjx-ooxml/examples/build_a_deck.rs`,
//! `build_a_document.rs`, `build_a_workbook.rs` — exists a second time under
//! `bindings/mjx-python/tests/` and a third under `bindings/mjx-wasm/tests/node/`, and every one
//! compares its output against the Rust one part by part, byte for byte — `walkthrough_triples.rs`
//! beside this file is what makes that true of all three rather than of the two it was true of
//! until MJXOFF-239. That is a strong gate and it proves the projection is **wired**. It does not
//! prove it is right **across the surface**:
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
//! # The second question this file asks (MJXOFF-276)
//!
//! An accessor that answers a string naming a kind states its whole contract in its own doc
//! comment, because `str`/`string` states none of it. Nothing compared that sentence to the code:
//! `binding_doc_parity.rs` holds the two bindings' sentences to *each other*,
//! [`the_two_bindings_produce_the_same_data_tokens`] holds their match arms to *each other*, and
//! the exercised measure above counts a member's **name**, never its **answer**. Two identical,
//! identically stale sentences passed all three, and four of them were —
//! [`every_token_vocabulary_a_binding_documents_is_the_one_its_code_answers`] is what asks.
//!
//! # What is deliberately *not* checked here, and why
//!
//! * **Enumeration members.** `bindings/mjx-python/tests/test_enums.py` already holds every
//!   projected enumeration to its Rust member names in both directions, and
//!   `bindings/mjx-wasm/tests/node/surface.mjs` names every member of the two `Format`
//!   enumerations. A second, weaker check here would add noise, not cover.
//! * **Which class a name was used on.** See the asymmetry above. Resolving `.rows` to a receiver
//!   would need a type checker for two languages; the un-exercised set does not need one.
//! * **The type graph.** Whether every exported class can be *obtained* from some other method is a
//!   real question, and it is asked — but not here. It found three answers rather than the two this
//!   file used to name: `mjx_ooxml::ResolvedColor`, `mjx_ooxml::TableStyleFlags` and
//!   `mjx_ooxml::Backdrop` were exported by both bindings and returned, taken and constructed by
//!   nothing in either (MJXOFF-228). The reason it is not asked here still stands: answering it
//!   *from this file* would need a signature parser over two hand-written crates, and a signature
//!   parser that is subtly wrong is worse than none. So
//!   `xtask/tests/facade_curation.rs`'s `every_exported_class_is_obtainable_from_some_other_call`
//!   reads the committed `.pyi` instead — a declaration that already exists and is parity-checked
//!   against the compiled module — and parses nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use xtask::binding_surface;

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
    let exports = binding_surface::wasm_exports(&repository_root());
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

/// **The complement of the rule above: a data *token* is not a name, and does not get camelCased.**
///
/// The wasm binding wrote this rule for itself, in `bindings/mjx-wasm/src/geometry.rs`'s comment on
/// `ShapeGeometry::of`:
///
/// > The keys are the adjustment names, which stay `snake_case`: they are data — the names
/// > ECMA-376's prose gives each `a:gd` — rather than method names, and renaming data would make
/// > `adjustmentNames` disagree with the record it describes.
///
/// It lived in prose and was checked by nothing, and it drifted: `ChartWrap.kind` answered
/// `"topAndBottom"` in JavaScript against `"top_and_bottom"` in Python, so a caller who ported a
/// comparison from one binding to the other got a comparison that silently stopped matching. The
/// sweep that fixed it found no second instance, which is a claim this test is what preserves
/// (MJXOFF-268).
///
/// # What this compares, and why it cannot condemn the binding's names
///
/// A method name and a returned token are told apart by **where they are written**, not by how they
/// are spelled. A JavaScript name is only ever a `js_name` inside an attribute or a Rust
/// identifier; a Python one is only ever a `#[pyo3(name = …)]` or an identifier. Neither is ever a
/// string literal in a function body. So [`binding_surface::wasm_source_literals`] drops every
/// attribute line and every comment line, and what survives is data by construction. `ChartWrap` is
/// the case that shows it: the constructor's `js_name = "topAndBottom"` and the token
/// `"topAndBottom"` its `kind` used to return were the same word in the same `impl` block, and only
/// the second of them is visible here.
///
/// The rule is stated over **both** bindings rather than only the JavaScript one. Python is correct
/// today by construction rather than by check, and a rule that only ever looks at one side is a
/// rule that has picked a reference implementation by accident.
///
/// # The one thing this must never be read as asking for
///
/// **A wire token is not renamed, ever** — `CLAUDE.md` says so, and a camelCase one is not a defect.
/// `ExternalLinkInfo.kind` answers `"oleObject"` in both bindings, which is `CT_ExternalLink`'s own
/// spelling and must stay exactly that. It does not appear here because neither binding *writes*
/// it: the token comes up from `mjx_ooxml`, and the binding hands it straight on. That is the
/// general shape — a binding invents the tokens it spells and preserves the ones it forwards — so
/// every literal this test sees is a binding's own invention. If a wire token ever does need to be
/// written out in one of these crates, the answer is a ledger entry naming the schema it comes
/// from, never a rename.
///
/// The second escape is a **structural key of a JavaScript object** — the `"kind"` and `"index"`
/// `read_surface` reads, the `"code"` and `"detail"` an `OoxmlError` carries. Those are names, and
/// a multi-word one would rightly be camelCase. Every one of them is a single word today, so the
/// rule and the escape do not yet collide; when they do, the fix is a ledger here, and the thing to
/// decide first is which kind of key it is. `ShapeGeometry.adjustments` is the case that shows a
/// key can go either way: its keys are the ECMA-376 adjustment names, and the comment quoted above
/// is the workspace deciding, in writing, that those are data and stay `snake_case`.
#[test]
fn no_data_token_is_spelled_in_camel_case() {
    let root = repository_root();
    let mut offenders = Vec::new();
    let mut underscored = 0usize;
    let mut scanned = 0usize;
    for (binding, literals) in [
        ("wasm", binding_surface::wasm_source_literals(&root)),
        ("python", binding_surface::python_source_literals(&root)),
    ] {
        for literal in literals {
            scanned += 1;
            if literal.produced && literal.value.contains('_') {
                underscored += 1;
            }
            if is_camel_case(&literal.value) {
                offenders.push(format!(
                    "bindings/mjx-{binding}/src/{}:{}: {}::{} spells the data token `\"{}\"` in \
                     camel case; a token is data and stays snake_case",
                    literal.file, literal.line, literal.owner, literal.member, literal.value
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a data token is spelled like a method name:\n  {}",
        offenders.join("\n  ")
    );
    // Anti-vacuity, both halves. The first says the literal scan still sees the sources; the second
    // says it still sees *multi-word* tokens, which are the only ones the casing rule can be broken
    // on — a scan that had stopped matching those would pass this test while asking nothing.
    assert!(
        scanned > 400 && underscored > 25,
        "{scanned} literal(s) and {underscored} multi-word produced token(s) found — the scan has \
         stopped matching"
    );
    println!(
        "data-token casing: {scanned} literal(s) across both bindings, {underscored} of them \
         multi-word tokens, 0 in camel case"
    );
}

/// Every data token both bindings produce for the same member, they spell the same way.
///
/// The check above is one-sided by design: it asks whether a token is camelCase, which is the shape
/// the drift took. This one asks the question underneath it — *does a caller who ported a
/// comparison from Python to TypeScript get the same string?* — and it would catch a divergence
/// that is not a casing difference at all.
///
/// **"Produced" is narrower than "written".** A literal counts only where a value is *made*: on
/// either side of a `match` arm's `=>`, or immediately before `.to_owned()`. That is what a caller
/// receives. A message handed to `expect` or to `invalid_argument` is written in a body too and is
/// not comparable — the two bindings raise through different error models, which
/// `xtask/tests/binding_doc_parity.rs`'s `DIVERGENT` ledger already records at length. Restricting
/// to produced literals is what lets this test carry **no ledger at all**: every member the two
/// bindings share agrees, with nothing excused.
///
/// Members are paired by `(owner, name)`, which is exact rather than approximate — Rust has no
/// overloading, so a type has at most one `fn` of a given name, and the false-pair trap
/// `binding_doc_parity.rs` disposes of with an argument count cannot arise.
///
/// # The way a comparison could quietly stop happening, and what stops it
///
/// A member that produces tokens in one binding and none in the other is not compared, and that is
/// how this check could decay without ever failing: rewrite one side to delegate to the facade —
/// `Surface::kind` already does, in both bindings, which is the healthier shape — and the other
/// side's hand-written tokens become unpaired and unwatched. So the second assertion below asks
/// exactly that: for every member that produces tokens in one binding, if the **other binding
/// declares a member of the same name on the same owner**, it must produce tokens too. The two
/// declaration readers this file already uses answer it — `wasm_exports` for JavaScript, the
/// committed stub for Python — so a free function like `read_surface`, which is plumbing rather
/// than surface, is excluded because neither binding declares it, not because it was excused.
#[test]
fn the_two_bindings_produce_the_same_data_tokens() {
    let root = repository_root();
    let wasm = produced_tokens(&binding_surface::wasm_source_literals(&root));
    let python = produced_tokens(&binding_surface::python_source_literals(&root));
    let mut divergent = Vec::new();
    let mut paired = 0usize;
    let mut tokens = 0usize;
    for (member, javascript) in &wasm {
        let Some(rust) = python.get(member) else {
            continue;
        };
        paired += 1;
        tokens += javascript.len();
        if javascript != rust {
            divergent.push(format!(
                "{}::{} answers {javascript:?} in JavaScript and {rust:?} in Python",
                member.0, member.1
            ));
        }
    }
    assert!(
        divergent.is_empty(),
        "the two bindings hand a caller different strings for the same value:\n  {}",
        divergent.join("\n  ")
    );
    let declared_by_javascript: BTreeSet<(String, String)> = binding_surface::wasm_exports(&root)
        .into_iter()
        .map(|export| (export.owner, export.rust_name))
        .collect();
    let declared_by_python: BTreeSet<(String, String)> = binding_surface::python_members(&root)
        .into_iter()
        .map(|member| (member.owner, member.name))
        .collect();
    let mut unwatched = Vec::new();
    for (producers, counterpart, declared_by_counterpart) in [
        (&wasm, &python, &declared_by_python),
        (&python, &wasm, &declared_by_javascript),
    ] {
        for member in producers.keys() {
            if declared_by_counterpart.contains(member) && !counterpart.contains_key(member) {
                unwatched.push(format!("{}::{}", member.0, member.1));
            }
        }
    }
    assert!(
        unwatched.is_empty(),
        "a member produces a token in one binding and none in the other, which both bindings \
         declare — the comparison above has stopped covering it:\n  {}",
        unwatched.join("\n  ")
    );
    assert!(
        paired > 15 && tokens > 60,
        "only {paired} shared member(s) producing {tokens} token(s) compared — the scan has \
         stopped matching"
    );
    println!(
        "data-token parity: {paired} member(s) declared by both bindings produce {tokens} token(s), \
         all spelled identically"
    );
}

/// The produced literals of each member that produces any, keyed by `(owner, member)`.
fn produced_tokens(
    literals: &[binding_surface::SourceLiteral],
) -> BTreeMap<(String, String), Vec<String>> {
    let mut found: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for literal in literals.iter().filter(|literal| literal.produced) {
        found
            .entry((literal.owner.clone(), literal.member.clone()))
            .or_default()
            .push(literal.value.clone());
    }
    found
}

// ===============================================================================================
// The documented vocabulary — MJXOFF-276
// ===============================================================================================

/// One arity-free accessor whose `///` comment names string literals its own body does not
/// produce, with the reason the comparison stops there.
///
/// A row is a *decision*, not a suppression: every one of the five was read to the place the value
/// is made, and what is recorded is what was found there. `every_unanswerable_row_still_names_an_accessor_that_answers_nothing`
/// holds the list to the measurement, so a row that stops describing its accessor fails rather than
/// lingering.
struct Unanswerable {
    /// The `impl` target the accessor sits in.
    owner: &'static str,
    /// Its Rust name.
    member: &'static str,
    /// Where the value is made, and why the sentence cannot be compared to a body here.
    reason: &'static str,
}

/// The five accessors whose documented literals are not written in the binding that documents them.
const UNANSWERABLE: [Unanswerable; 5] = [
    Unanswerable {
        owner: "Surface",
        member: "kind",
        reason: "the body is `self.0.kind_name()`; the five names are written in \
                 `mjx_ooxml::Surface::kind_name` (crates/mjx-ooxml/src/address.rs), whose own doc \
                 comment lists the same five — read and confirmed complete",
    },
    Unanswerable {
        owner: "WorkbookExternalLinkInfo",
        member: "kind",
        reason: "the body is `self.0.kind`; the four shapes are written in \
                 crates/mjx-ooxml/src/workbook/preserved.rs, and `oleObject` among them is \
                 `CT_ExternalLink`'s own wire spelling, which CLAUDE.md forbids renaming — read \
                 and confirmed complete",
    },
    Unanswerable {
        owner: "AdjustmentSpec",
        member: "wire_name",
        reason: "an open set, not a vocabulary: `adj`, `adj1`, `adj2`… are a preset shape's own \
                 adjustment names, and the sentence's trailing ellipsis says so",
    },
    Unanswerable {
        owner: "Field",
        member: "field_name",
        reason: "an open set, not a vocabulary: ECMA-376's field-type keywords, of which the \
                 sentence shows two and an ellipsis",
    },
    Unanswerable {
        owner: "Format",
        member: "conventional_extension",
        reason: "an open set, not a vocabulary: three of the nine extensions, shown as examples \
                 (`wasm` spells this one as a free function, which no `impl` block scan reaches)",
    },
];

/// Below this many compared vocabularies the scan has stopped matching.
///
/// A floor, never a total: a check over twenty accessors that silently matched none would read
/// exactly like a check over twenty accessors with nothing wrong. The real figure is printed on
/// success.
const MINIMUM_COMPARED_VOCABULARIES: usize = 36;

/// Every backtick-quoted string literal a doc comment spells — the `` `"square"` `` form, and only
/// that form.
///
/// A bare backtick span (`` `Format.WorkbookBinary` ``) names a symbol, not a token, and a bare
/// quoted string outside backticks does not occur in either binding's prose. So the needle is the
/// two characters together, in both directions.
fn documented_tokens(prose: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let bytes = prose.as_bytes();
    let mut index = 0usize;
    while index + 2 < bytes.len() {
        if bytes[index] == b'`' && bytes[index + 1] == b'"' {
            if let Some(close) = prose[index + 2..].find("\"`") {
                found.insert(prose[index + 2..index + 2 + close].to_owned());
                index += 2 + close + 2;
                continue;
            }
        }
        index += 1;
    }
    found
}

/// The member a doc comment defers its vocabulary to, for the *"in the same vocabulary `X.y` uses"*
/// form.
///
/// One sentence uses it — `ResolvedDrawCommand.kind` names `DrawCommand.kind` rather than
/// repeating six tokens — and the form is worth supporting rather than expanding, because a copy of
/// a list is a second thing to keep current. The reference is then checked: the two bodies must
/// answer the same set.
fn referenced_vocabulary(prose: &str) -> Option<(String, String)> {
    let rest = prose.split_once("same vocabulary `")?.1;
    let span = rest.split_once('`')?.0;
    let (owner, member) = span.split_once('.')?;
    Some((owner.to_owned(), member.to_owned()))
}

/// The set of tokens a member's body can answer, and where those tokens are written.
///
/// Two hops, and no more. A member that produces literals of its own answers those. A member that
/// produces none but whose body names exactly one **free function in the same binding** that does —
/// `CellData.kind` is `kind_of(&self.0)`, `Document.conformance` is `conformance_str(…)` — answers
/// that function's, because the vocabulary is genuinely one set written once and called from
/// several accessors. Anything further is a value made outside the binding, and [`UNANSWERABLE`] is
/// where those are named.
fn answered_tokens(
    key: &(String, String),
    body: &str,
    produced: &BTreeMap<(String, String), Vec<String>>,
    helpers: &BTreeMap<String, BTreeSet<String>>,
) -> Option<(BTreeSet<String>, String)> {
    if let Some(own) = produced.get(key) {
        return Some((own.iter().cloned().collect(), String::from("its own arms")));
    }
    let called: Vec<&String> = helpers.keys().filter(|name| mentions(body, name)).collect();
    match called.as_slice() {
        [name] => Some((helpers[*name].clone(), format!("`{name}`"))),
        _ => None,
    }
}

/// Whether `body` names `identifier` as a whole word.
///
/// A whole word rather than a call, because `Document.conformance` hands `conformance_str` on as a
/// function value — `map(conformance_str)`, no parenthesis in sight — and a needle demanding one
/// misses the very accessor the two-hop rule exists for.
fn mentions(body: &str, identifier: &str) -> bool {
    let bytes = body.as_bytes();
    let mut from = 0usize;
    while let Some(offset) = body[from..].find(identifier) {
        let at = from + offset;
        let end = at + identifier.len();
        let before_is_word =
            at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_');
        let after_is_word =
            end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_');
        if !before_is_word && !after_is_word {
            return true;
        }
        from = at + 1;
    }
    false
}

/// **The sentence a caller reads is the vocabulary the code answers** (MJXOFF-276).
///
/// A token accessor's return type is `str`/`string`, so the list in its doc comment is the *whole*
/// contract — and until this test nothing compared it to anything. `binding_doc_parity.rs` holds
/// the two bindings' sentences to each other, `the_two_bindings_produce_the_same_data_tokens` above
/// holds their match arms to each other, and the exercised measure counts a member's *name*, never
/// its answer. Two identical, identically stale sentences passed all three.
///
/// Both directions are asked, because they fail differently:
///
/// * a member whose body can answer two or more tokens must document exactly those — which is what
///   catches a token added to a match and to no sentence, the shape MJXOFF-285's `Unreadable`
///   variant left behind in `CellBlock.kinds`;
/// * an arity-free accessor whose sentence names two or more tokens must have a body that answers
///   exactly those, or stand on [`UNANSWERABLE`] — which is what catches a sentence naming a token
///   nothing writes.
///
/// **Argument vocabularies are in scope only when the binding writes them.** `Workbook.read_range`
/// documents the A1 forms it accepts and `CellWrite.error` an error code or two; neither is a
/// closed set, and neither is checked here, because a taken value is refused by the parser that
/// reads it rather than promised by a return type. `Document.setConformance` *is* checked, and only
/// incidentally: `conformance_from_str` writes its two tokens in this binding, so the first
/// direction reaches it.
#[test]
fn every_token_vocabulary_a_binding_documents_is_the_one_its_code_answers() {
    let root = repository_root();
    let ledger: BTreeSet<(&str, &str)> = UNANSWERABLE
        .iter()
        .map(|row| (row.owner, row.member))
        .collect();
    let mut wrong = Vec::new();
    let mut exercised: BTreeSet<(String, String)> = BTreeSet::new();
    let mut compared = 0usize;
    for (binding, directory, marker, literals) in [
        (
            "JavaScript",
            "bindings/mjx-wasm/src",
            "#[wasm_bindgen]",
            binding_surface::wasm_source_literals(&root),
        ),
        (
            "Python",
            "bindings/mjx-python/src",
            "#[pymethods]",
            binding_surface::python_source_literals(&root),
        ),
    ] {
        let documented = binding_surface::documented_members(&root.join(directory), marker);
        let produced = produced_tokens(&literals);
        let helpers: BTreeMap<String, BTreeSet<String>> = produced
            .iter()
            .filter(|((owner, _), _)| owner == "<module>")
            .map(|((_, member), tokens)| (member.clone(), tokens.iter().cloned().collect()))
            .collect();
        let answers = |key: &(String, String)| {
            documented
                .get(key)
                .and_then(|member| answered_tokens(key, &member.body, &produced, &helpers))
        };

        // Direction one: what the code can answer, it must document.
        let mut subjects: BTreeSet<(String, String)> = produced
            .iter()
            .filter(|((owner, _), tokens)| {
                owner != "<module>" && tokens.iter().collect::<BTreeSet<_>>().len() >= 2
            })
            .map(|(key, _)| key.clone())
            .collect();
        subjects.extend(
            documented
                .keys()
                .filter(|key| answers(key).is_some_and(|(tokens, _)| tokens.len() >= 2))
                .cloned(),
        );
        for key in &subjects {
            let Some(member) = documented.get(key) else {
                wrong.push(format!(
                    "{binding}: {}.{} answers {:?} and carries no `///` comment at all",
                    key.0,
                    key.1,
                    produced
                        .get(key)
                        .map(|tokens| tokens.iter().collect::<BTreeSet<_>>()),
                ));
                continue;
            };
            let Some((answered, source)) = answers(key) else {
                continue;
            };
            let claimed = if let Some(reference) = referenced_vocabulary(&member.prose) {
                let target = documented
                    .keys()
                    .find(|(owner, name)| {
                        *owner == reference.0
                            && (*name == reference.1 || camel_case(name) == reference.1)
                    })
                    .cloned();
                let Some(target) = target.filter(|target| answers(target).is_some()) else {
                    wrong.push(format!(
                        "{binding}: {}.{} defers its vocabulary to `{}.{}`, which this binding \
                         either does not project or does not answer a vocabulary for \
                         ({}:{})",
                        key.0, key.1, reference.0, reference.1, member.file, member.line
                    ));
                    continue;
                };
                answers(&target).expect("just filtered on it").0
            } else {
                documented_tokens(&member.prose)
            };
            compared += 1;
            if claimed != answered {
                let missing: Vec<&String> = answered.difference(&claimed).collect();
                let invented: Vec<&String> = claimed.difference(&answered).collect();
                wrong.push(format!(
                    "{binding}: {}.{} answers {answered:?} from {source} but its sentence names \
                     {claimed:?} — undocumented: {missing:?}, documented and unwritable: \
                     {invented:?} ({}:{})",
                    key.0, key.1, member.file, member.line
                ));
            }
        }

        // Direction two: what an accessor documents, it must be able to answer.
        for (key, member) in &documented {
            if member.arity > 0 || subjects.contains(key) {
                continue;
            }
            if documented_tokens(&member.prose).len() < 2 {
                continue;
            }
            if ledger.contains(&(key.0.as_str(), key.1.as_str())) {
                exercised.insert(key.clone());
                continue;
            }
            wrong.push(format!(
                "{binding}: {}.{} documents {:?} and its body answers none of them, and it stands \
                 on no `UNANSWERABLE` row saying where the value is made ({}:{})",
                key.0,
                key.1,
                documented_tokens(&member.prose),
                member.file,
                member.line
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "a binding's documented vocabulary is not the one its code answers:\n  {}",
        wrong.join("\n  ")
    );
    assert!(
        compared >= MINIMUM_COMPARED_VOCABULARIES,
        "only {compared} vocabulary/ies compared — the scanner has stopped matching, and the claim \
         above would be made over almost nothing"
    );
    println!(
        "documented vocabularies: {compared} compared across the two bindings, {} accessor(s) on \
         the `UNANSWERABLE` ledger",
        exercised.len()
    );
}

/// Every [`UNANSWERABLE`] row still names an accessor that documents tokens and answers none.
///
/// The ledger is held to the measurement in the same way `binding_doc_parity.rs` holds its own: a
/// row kept after its accessor started answering for itself would be a standing exemption nobody
/// reads, and the reason written beside it would be describing something that is no longer there.
#[test]
fn every_unanswerable_row_still_names_an_accessor_that_answers_nothing() {
    let root = repository_root();
    let mut stale = Vec::new();
    for row in &UNANSWERABLE {
        let mut found = false;
        for (directory, marker, literals) in [
            (
                "bindings/mjx-wasm/src",
                "#[wasm_bindgen]",
                binding_surface::wasm_source_literals(&root),
            ),
            (
                "bindings/mjx-python/src",
                "#[pymethods]",
                binding_surface::python_source_literals(&root),
            ),
        ] {
            let documented = binding_surface::documented_members(&root.join(directory), marker);
            let produced = produced_tokens(&literals);
            let helpers: BTreeMap<String, BTreeSet<String>> = produced
                .iter()
                .filter(|((owner, _), _)| owner == "<module>")
                .map(|((_, member), tokens)| (member.clone(), tokens.iter().cloned().collect()))
                .collect();
            let key = (row.owner.to_owned(), row.member.to_owned());
            let Some(member) = documented.get(&key) else {
                continue;
            };
            if member.arity == 0
                && documented_tokens(&member.prose).len() >= 2
                && answered_tokens(&key, &member.body, &produced, &helpers).is_none()
            {
                found = true;
            }
        }
        if !found {
            stale.push(format!("{}.{} — {}", row.owner, row.member, row.reason));
        }
    }
    assert!(
        stale.is_empty(),
        "an `UNANSWERABLE` row names no accessor that documents a vocabulary its own body cannot \
         answer — it has been fixed, renamed or removed, and the row with it:\n  {}",
        stale.join("\n  ")
    );
    println!(
        "the unanswerable ledger: all {} row(s) still name an accessor whose vocabulary is made \
         outside the binding that documents it",
        UNANSWERABLE.len()
    );
}

/// No delimiter this crate's brace and bracket matching cares about is ever written as a character
/// literal in either binding's sources.
///
/// `binding_surface::balance` skips string literals and not character ones, because skipping the
/// latter means telling `'{'` apart from the lifetime in `impl<'a>` — a lexer's job. The shortcut
/// is safe exactly as long as this holds, so it is asked rather than assumed.
#[test]
fn a_delimiter_is_never_written_as_a_character_literal() {
    let root = repository_root();
    let mut offenders = Vec::new();
    for directory in ["bindings/mjx-wasm/src", "bindings/mjx-python/src"] {
        for (file, text) in binding_surface::sources(&root.join(directory), "rs") {
            for (number, line) in text.lines().enumerate() {
                for delimiter in ['{', '}', '[', ']'] {
                    if line.contains(&format!("'{delimiter}'")) {
                        offenders.push(format!("{directory}/{file}:{}", number + 1));
                    }
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a delimiter is written as a character literal, which the brace matching in \
         `xtask/src/binding_surface.rs` does not skip:\n  {}",
        offenders.join("\n  ")
    );
}

/// Whether `value` is a single camelCase word: it begins with a lower-case letter, is made only of
/// letters and digits, and carries at least one capital.
///
/// Deliberately whole-literal. `"SectionLocation.body()"` is a display string that happens to name
/// a method and is not a token; `"topAndBottom"` was a token spelled like a name. Requiring the
/// *entire* literal to be one camelCase word is what separates them.
fn is_camel_case(value: &str) -> bool {
    value.starts_with(|character: char| character.is_ascii_lowercase())
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
        && value
            .chars()
            .any(|character| character.is_ascii_uppercase())
}

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
    let python_tests =
        binding_surface::sources(&repository_root().join("bindings/mjx-python/tests"), "py");
    let node_tests = binding_surface::sources(
        &repository_root().join("bindings/mjx-wasm/tests/node"),
        "mjs",
    );

    let members = binding_surface::python_members(&repository_root());
    let python_declared: Vec<Declaration> = members
        .iter()
        .filter(|member| !member.name.starts_with("__"))
        .map(|member| (member.owner.clone(), member.name.clone()))
        .collect();
    let (python_used, python_unused) = split_by_use(&python_declared, &python_tests, true);

    let exports = binding_surface::wasm_exports(&repository_root());
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
const PYTHON_DECLARED: usize = 1_644;
/// How many of [`PYTHON_DECLARED`] some test under `bindings/mjx-python/tests/` names.
const PYTHON_EXERCISED: usize = 1_012;
/// Functions `wasm-bindgen` exports to JavaScript.
const WASM_DECLARED: usize = 1_768;
/// How many of [`WASM_DECLARED`] some test under `bindings/mjx-wasm/tests/node/` names.
const WASM_EXERCISED: usize = 946;

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

// ===============================================================================================
// The error model, held to its own registrations (MJXOFF-275)
// ===============================================================================================

/// The Python exception classes a hand-written raise in `bindings/mjx-python/src` may construct
/// **without** being one of the twelve `bindings/mjx-python/src/errors.rs` registers, and the
/// reason each is right where it stands.
///
/// These are Python's own vocabulary for a mistake in the *call*, and they are the exact mirror of
/// `bindings/mjx-wasm/src/support.rs`'s `invalid_argument`, which is a `RangeError` and equally not
/// an `OoxmlError`. Routing them through `OoxmlError` would be the divergence, not the fix: PyO3
/// raises `TypeError` for every argument conversion it generates, so a hand-written
/// `FromPyObject` that raised something else would be the one member of the surface a caller could
/// not guard the ordinary way.
const PYTHON_HOST_VOCABULARY: &[(&str, &str)] = &[
    (
        "PyTypeError",
        "an argument of the wrong type — the class PyO3 raises for every conversion it generates",
    ),
    (
        "PyValueError",
        "an argument of the right type carrying a value the call refuses",
    ),
    (
        "PyKeyError",
        "a name a mapping argument does not have, or one it has and the shape did not want",
    ),
];

/// The constructions that are neither a registered class nor the host vocabulary, named by file and
/// by the message they carry, because a line number rots and a message does not.
///
/// There is exactly one, and it is forced: it is raised while the exception hierarchy is being
/// *built*, so it cannot be reported through a hierarchy that does not exist yet.
const PYTHON_LEDGERED_RAISES: &[(&str, &str, &str)] = &[(
    "errors.rs",
    "PyRuntimeError",
    "type() did not return a class",
)];

/// The floor under the Python scan. Well below what it finds, because the number it finds is a
/// measurement and this is only the statement that the scanner still matches at all.
const PYTHON_RAISE_FLOOR: usize = 15;

/// Nothing in `bindings/mjx-python/src` raises a Python exception class the binding does not
/// register, except the host vocabulary above and one ledgered site.
///
/// # The hole this closes, and why an unreachable arm was worth a gate
///
/// `ShapeGeometry.preset` raised `PyRuntimeError::new_err("unreachable")` until MJXOFF-275. A
/// caller writing `except mjx_ooxml.OoxmlError` did not catch it, and no other gate could see it:
/// `binding_doc_parity.rs` reads doc comments rather than runtime messages, and
/// [`the_two_bindings_produce_the_same_data_tokens`] deliberately excludes a message handed to an
/// error constructor. The arm is unreachable — `parts()` answers `None` only for `Unmodeled`, which
/// the arm above it already matched, and its `match` carries no wildcard, so the compiler holds the
/// equivalence rather than a comment — and that is exactly why it was worth closing: an unreachable
/// arm is where an error model stops being total without anything failing.
///
/// So the sweep is asked rather than repeated by hand. It is the class of raise that is checked,
/// not the one instance: `PyValueError`, `PyTypeError`, `PyKeyError` and `PyErr::new::<…>` are all
/// shapes PyO3 offers, and reading the shapes that actually appear is what says how large the class
/// is instead of guessing at it.
#[test]
fn every_exception_the_python_binding_constructs_is_registered_or_ledgered() {
    let sites = binding_surface::python_raise_sites(&repository_root());
    // Anti-vacuity, asked first: a sweep that matches no raise passes exactly as a binding with no
    // unregistered raise does, and the two must not look alike from the outside.
    assert!(
        sites.len() >= PYTHON_RAISE_FLOOR,
        "{} raise(s) found across bindings/mjx-python/src, under the floor of \
         {PYTHON_RAISE_FLOOR} — the scanner has stopped matching",
        sites.len()
    );
    let permitted: BTreeSet<&str> = PYTHON_HOST_VOCABULARY
        .iter()
        .map(|(class, _)| *class)
        .collect();
    let registered = registered_python_classes();
    let mut offenders = Vec::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut ledgered_used = vec![false; PYTHON_LEDGERED_RAISES.len()];
    for site in &sites {
        *seen.entry(site.class.clone()).or_default() += 1;
        if permitted.contains(site.class.as_str()) || registered.contains(&site.class) {
            continue;
        }
        if let Some(index) = PYTHON_LEDGERED_RAISES
            .iter()
            .position(|(file, class, message)| {
                *file == site.file && *class == site.class && site.tail.contains(message)
            })
        {
            ledgered_used[index] = true;
            continue;
        }
        offenders.push(format!(
            "bindings/mjx-python/src/{}:{}: raises `{}`, which `errors.rs` does not register — a \
             caller writing `except mjx_ooxml.OoxmlError` does not catch it. Raise through \
             `crate::errors` instead, or put the class on PYTHON_HOST_VOCABULARY with a reason.\n    \
             {}",
            site.file,
            site.line,
            site.class,
            site.tail.trim()
        ));
    }
    assert!(
        offenders.is_empty(),
        "the Python binding's error model has a hole in it:\n  {}",
        offenders.join("\n  ")
    );
    for (index, (file, class, message)) in PYTHON_LEDGERED_RAISES.iter().enumerate() {
        assert!(
            ledgered_used[index],
            "PYTHON_LEDGERED_RAISES excuses `{class}` in {file} carrying \"{message}\", and no such \
             raise exists — delete the row, or find out why the scan no longer sees it"
        );
    }
    for (class, reason) in PYTHON_HOST_VOCABULARY {
        assert!(
            seen.contains_key(*class),
            "PYTHON_HOST_VOCABULARY permits `{class}` ({reason}) and the scan found none — either \
             the raise moved or the scanner has stopped matching"
        );
    }
    let breakdown: Vec<String> = seen
        .iter()
        .map(|(class, count)| format!("{count} {class}"))
        .collect();
    println!(
        "python error model: {} hand-written raise(s) ({}), every one registered or ledgered",
        sites.len(),
        breakdown.join(", ")
    );
}

/// The two files that may build a JavaScript `Error`, and what each builds.
///
/// The wasm binding funnels every failure through three functions in these two files, so the
/// mirror of the Python question is a stronger one: not *which class* a site constructs, but
/// whether any site constructs one at all outside the factories.
const WASM_ERROR_FACTORIES: &[(&str, &str, &str)] = &[
    (
        "errors.rs",
        "js_sys::Error",
        "`to_js_error` and `unsupported_content` — the `OoxmlError` projection. The name, the \
         `code` and the `detail` are set here and nowhere else",
    ),
    (
        "support.rs",
        "js_sys::RangeError",
        "`invalid_argument` — JavaScript's own argument vocabulary, the mirror of Python's \
         `TypeError`/`ValueError`/`KeyError` and equally not an `OoxmlError`",
    ),
];

/// The floor under the count of hand-written raises in the wasm binding — the population the
/// Python floor above is over, so the two numbers are comparable.
const WASM_RAISE_FLOOR: usize = 15;

/// The mirror direction: the wasm binding builds an `Error` in two files and calls it from
/// everywhere else.
///
/// MJXOFF-275 was written about Python, and the two bindings are held to each other everywhere else
/// in this repository, so the same question has to be asked facing the other way. The answer is not
/// the one the ticket assumed — `invalid_argument` is a `RangeError` with no `name`, no `code` and
/// no `detail`, so `catch (e) { e.code === "InvalidArgument" }` never matched it — but the shape is
/// sound, and it is sound for the same reason Python's `TypeError` is: a mistake in the call is
/// reported in the host language's own vocabulary, and a failure the *library* reported is reported
/// as an `OoxmlError`. What matters is that no third population exists, which is what this asks.
#[test]
fn every_error_the_wasm_binding_constructs_comes_from_one_of_its_two_factories() {
    let root = repository_root();
    let sites = binding_surface::wasm_raise_sites(&root);
    // Anti-vacuity, asked first, and it cannot be the construction count: the whole point of this
    // binding's shape is that there are three of those. What would vanish if the scan stopped
    // matching is the *raising*, so that is what is floored — the same population, counted the same
    // way, as the twenty-two the Python half reports.
    let raises = wasm_hand_written_raises(&root);
    assert!(
        raises >= WASM_RAISE_FLOOR,
        "{raises} hand-written raise(s) in bindings/mjx-wasm/src, under the floor of \
         {WASM_RAISE_FLOOR} — the scanner has stopped matching"
    );
    let mut offenders = Vec::new();
    let mut used = vec![false; WASM_ERROR_FACTORIES.len()];
    for site in &sites {
        match WASM_ERROR_FACTORIES
            .iter()
            .position(|(file, class, _)| *file == site.file && *class == site.class)
        {
            Some(index) => used[index] = true,
            None => offenders.push(format!(
                "bindings/mjx-wasm/src/{}:{}: builds `{}` outside the two factories, so it carries \
                 neither `name = \"OoxmlError\"` nor a `code`. Call `crate::errors` or \
                 `crate::support::invalid_argument` instead.\n    {}",
                site.file,
                site.line,
                site.class,
                site.tail.trim()
            )),
        }
    }
    assert!(
        offenders.is_empty(),
        "the wasm binding raises outside its error model:\n  {}",
        offenders.join("\n  ")
    );
    for (index, (file, class, reason)) in WASM_ERROR_FACTORIES.iter().enumerate() {
        assert!(
            used[index],
            "WASM_ERROR_FACTORIES names `{class}` in {file} ({reason}) and the scan found none — \
             either the factory moved or the scanner has stopped matching"
        );
    }
    println!(
        "wasm error model: {} hand-written raise(s), every one through a factory, and {} `Error` \
         construction(s), both in the two ledgered files",
        raises,
        sites.len()
    );
}

/// How many times `bindings/mjx-wasm/src` raises a failure of its own — a call to
/// `invalid_argument` or to `unsupported_content`, their two definition lines excluded.
///
/// `to_js_error` and `map_error` are deliberately not counted: they *project* a failure
/// `mjx_ooxml` reported, which is mechanical forwarding on nearly every method, and counting them
/// would put a four-figure number beside the Python half's twenty-two and make the two
/// incomparable.
fn wasm_hand_written_raises(root: &std::path::Path) -> usize {
    binding_surface::sources(&root.join("bindings/mjx-wasm/src"), "rs")
        .iter()
        .flat_map(|(_, text)| text.lines())
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//") && !trimmed.starts_with("pub(crate) fn ")
        })
        .map(|line| {
            ["invalid_argument(", "unsupported_content("]
                .iter()
                .map(|needle| line.matches(needle).count())
                .sum::<usize>()
        })
        .sum()
}

/// The twelve class names `bindings/mjx-python/src/errors.rs` adds to the module.
///
/// Read out of `register`'s own table rather than listed here, so a thirteenth class is registered
/// once and is permitted here the same day.
fn registered_python_classes() -> BTreeSet<String> {
    let source =
        std::fs::read_to_string(repository_root().join("bindings/mjx-python/src/errors.rs"))
            .expect("bindings/mjx-python/src/errors.rs is readable");
    let table = source
        .split_once("pub(crate) fn register(")
        .expect("errors.rs declares `register`")
        .1;
    let names: BTreeSet<String> = table
        .lines()
        .filter_map(|line| {
            let start = line.find("(\"")? + 2;
            let end = line[start..].find('"')? + start;
            Some(line[start..end].to_owned())
        })
        .filter(|name| name.ends_with("Error"))
        .map(|name| format!("Py{name}"))
        .collect();
    assert!(
        names.len() >= 12,
        "{} class(es) read out of `register` — the table has stopped matching",
        names.len()
    );
    names
}

/// The floor under the count of no-argument members swept for the rule below.
const ARGUMENTLESS_MEMBER_FLOOR: usize = 1_000;

/// A member that takes no argument never raises the argument vocabulary.
///
/// This is the rule underneath the two checks above, and it is the one that catches the shape
/// MJXOFF-275 found rather than the class it happened to wear. Both bindings reserve a vocabulary
/// for *a mistake in the call* — Python's `TypeError`/`ValueError`/`KeyError`, JavaScript's
/// `RangeError` through `invalid_argument` — and each is right exactly where an argument was
/// refused. A member with no arguments has no call to be mistaken, so reaching for that vocabulary
/// there is a category error, and it is what the wasm half of `ShapeGeometry.preset` did: a getter
/// answering `invalid_argument("this geometry names no preset")`, which no caller could have caused
/// and which carried neither `name = "OoxmlError"` nor a `code`.
///
/// The two checks above could not see it. Python's asks which *class* is constructed, and the wasm
/// one asks only that a construction sit in a factory — `invalid_argument` is a factory, and it was
/// being called from the wrong kind of place, not written in the wrong file.
///
/// Arity comes from [`binding_surface::documented_members`], which already excludes `self` and
/// PyO3's `Python<'_>` token, so a getter and a zero-argument static both count as zero and both
/// are held to the rule for the same reason.
#[test]
fn no_argumentless_member_raises_the_argument_vocabulary() {
    let root = repository_root();
    let mut offenders = Vec::new();
    let mut argumentless = 0usize;
    let mut vocabulary_seen = 0usize;
    for (binding, directory, marker, needles) in [
        (
            "wasm",
            "bindings/mjx-wasm/src",
            "#[wasm_bindgen]",
            vec![String::from("invalid_argument(")],
        ),
        (
            "python",
            "bindings/mjx-python/src",
            "#[pymethods]",
            PYTHON_HOST_VOCABULARY
                .iter()
                .map(|(class, _)| format!("{class}::new_err"))
                .collect(),
        ),
    ] {
        for ((owner, name), member) in
            binding_surface::documented_members(&root.join(directory), marker)
        {
            let raises: Vec<&String> = needles
                .iter()
                .filter(|needle| member.body.contains(needle.as_str()))
                .collect();
            if !raises.is_empty() {
                vocabulary_seen += 1;
            }
            if member.arity > 0 {
                continue;
            }
            argumentless += 1;
            for needle in raises {
                offenders.push(format!(
                    "{directory}/{}:{}: {owner}.{name} takes no argument and raises `{}` — \
                     there is no call for the caller to have got wrong. A failure a no-argument \
                     member reports is the library's, so raise it through `crate::errors` \
                     ({binding}).",
                    member.file,
                    member.line,
                    needle.trim_end_matches('('),
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a no-argument member reports a failure as if the caller had passed something:\n  {}",
        offenders.join("\n  ")
    );
    // Anti-vacuity, both halves. The first says the member walk still sees the surfaces; the second
    // says the needles still match a raise somewhere, so a rename of `invalid_argument` cannot turn
    // this test into a tautology.
    assert!(
        argumentless >= ARGUMENTLESS_MEMBER_FLOOR && vocabulary_seen > 0,
        "{argumentless} no-argument member(s) and {vocabulary_seen} member(s) raising the argument \
         vocabulary at all — the scan has stopped matching"
    );
    println!(
        "argument vocabulary: {argumentless} no-argument member(s) across both bindings raise none \
         of it, and {vocabulary_seen} member(s) that do take arguments do"
    );
}
