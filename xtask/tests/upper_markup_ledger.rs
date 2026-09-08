//! **The serialization ledger for rank 2.2 — `mjx-chart`, `mjx-omml`, `mjx-vml`** (MJXOFF-221, the
//! third and last instalment of MJXOFF-218's question).
//!
//! # The question here is neither of the two already answered
//!
//! `crates/mjx-dml/tests/serialization_ledger.rs` (MJXOFF-217) asks, of eight hand-written
//! `FromXml`/`ToXml` **pairs**, whether a type that reads an element by hand and rebuilds it by hand
//! loses anything in between. Six types were losing content, and the bodies were six different
//! mistakes. `crates/mjx-sml/tests/serialization_ledger.rs` (MJXOFF-220) found that question did not
//! transfer: `mjx-sml`'s writers are `ToXml`-only and 57 of 58 are the *same* three-line delegation,
//! so the risk had moved one hop, into the `as_raw_element` each one delegates to.
//!
//! **It does not transfer here either, and this time the reason is arithmetic.** These three crates
//! contain, between them, **zero** hand-written `FromXml` or `ToXml` impls and **zero**
//! `as_raw_element` rebuilders. Every type that models an element reaches XML through one of exactly
//! two mechanisms:
//!
//! * `#[derive(FromXml, ToXml)]` — `mjx-derive`'s codegen, one implementation for the whole
//!   workspace, backed by `crates/mjx-derive/tests/derive.rs`; or
//! * the crate's **own** `fidelity_*!` macro — one body per crate, three bodies in total.
//!
//! So there is no impl body to audit, and a ledger of impl bodies would be an empty list. **The risk
//! is therefore not in any body; it is in *which mechanism a type is on*, and in the fact that
//! nothing before this file would have noticed a type going on neither.** That is precisely the hole
//! MJXOFF-216's `Picture::to_xml` came through one rank below: a type that stops using the shared
//! mechanism and starts reading and rebuilding by hand is invisible to the derive's tests (they do
//! not apply to it), to the per-type suites (they name a subset), and to the preservation gate
//! (per-fixture, over a canonical corpus that need not contain the element).
//!
//! # How a zero is kept from being vacuous
//!
//! A check phrased *every hand-written impl is on the ledger*, over a crate with none, passes
//! forever whatever the scanner does — MJXOFF-88 §7's shape in its purest form. A floor of the usual
//! kind is impossible, because the honest count really is zero.
//!
//! So the scanner is **calibrated against the two crates that do have them**. It is pointed at
//! `mjx-dml` and `mjx-sml` as well, and required to find their known populations there. A scanner
//! that has stopped matching reports zero for all five crates and fails on the calibration long
//! before it reports a false clean bill for these three. See
//! [`the_impl_scanner_is_calibrated_against_the_two_crates_that_do_hand_write`].
//!
//! # Why one file for three crates, where `mjx-dml` and `mjx-sml` each got their own
//!
//! MJXOFF-218 §3 asks for this decision to be made and recorded. `mjx-sml`'s ledger records the
//! opposite call for the opposite reason, and both are consistent: *"the idioms **are** the finding
//! and they differ per crate"*. There, the expensive half — the ledger rows, the reasons, the
//! idioms — was per crate, and only the cheap half (a sixty-line source scanner) was shared, so
//! sharing bought nothing.
//!
//! Here the expensive half **is** shared. The finding is a property of the mechanism rather than of
//! any crate's bodies, all three crates answer it the same way, and the three crates are one unit
//! precisely because they are one rank of one kind of markup. Three copies of one scanner asserting
//! one shared fact would be the list that passes once it is written.
//!
//! It is hosted by `xtask` for the reason `xtask/tests/doc_gate.rs`, `layering.rs` and
//! `facade_curation.rs` are: **every cross-crate structural gate in this workspace lives here**, and
//! `xtask` is outside the layering graph, so a gate over three same-rank crates needs no edge
//! between them. Hosting it in `mjx-chart` would make that crate's suite fail on a change to
//! `mjx-omml`, which reads as a coupling that does not exist — `mjx-chart` cannot depend on either
//! sibling, and `xtask/tests/layering.rs` fails if it ever does. A test binary reads *source text*,
//! which needs no dependency at all.
//!
//! # What is deliberately not checked here, and why
//!
//! * **That a derived content enum has its `Raw` catch-all.** It cannot not have one:
//!   `crates/mjx-derive/src/parse.rs` hard-codes the variant name and
//!   `crates/mjx-derive/src/expand.rs` emits the arm unconditionally, so an enum without it does not
//!   compile. A test asserting it would be checking the Rust compiler.
//! * **What a typed accessor reads.** This file is about what survives a round trip, not about
//!   whether an accessor answers the right thing; the per-crate suites
//!   (`crates/mjx-chart/tests/`, `crates/mjx-omml/tests/deep_nesting.rs`,
//!   `crates/mjx-vml/tests/drawing.rs`) are where that lives.
//! * **`mjx-docx`.** 158 hand-written pairs, and the remaining half of MJXOFF-218.
//!
//! # Every arm here was made to fail, with a mutation a person could actually make
//!
//! A gate nobody has seen fail is a gate nobody has tested. Each of these was applied, run and
//! reverted under MJXOFF-221:
//!
//! | Mutation | What failed |
//! |---|---|
//! | `crates/mjx-chart/src/build.rs`: `let children = self.children.clone()` → `Vec::new()`, MJXOFF-216's exact shape | [`each_fidelity_macro_writes_back_every_field_it_read`] |
//! | `crates/mjx-chart/src/axis.rs`: `fidelity_element_impls!(Legend)` replaced by a written-out `FromXml`/`ToXml` pair | [`no_type_in_the_upper_markup_serializes_itself_by_hand`] |
//! | `crates/mjx-vml/src/shape.rs`: a new `ShapeHandles` with the framework fields and no mechanism | [`every_element_modelling_type_is_on_one_of_the_two_mechanisms`] |
//! | this file: `fidelity_leaf` renamed, as a refactor of the macro would | [`the_scanner_still_sees_both_mechanisms_in_all_three_crates`] and two others |
//! | this file: the impl scanner's `strip_prefix("impl ")` mistyped as `"impl<"` | **only** [`the_impl_scanner_is_calibrated_against_the_two_crates_that_do_hand_write`] |
//!
//! **The last row is the one worth reading.** With the impl scanner broken,
//! [`no_type_in_the_upper_markup_serializes_itself_by_hand`] still reported *0 hand-written impls,
//! 0 on the ledger* and passed — a clean bill it had not earned, which is what a zero always risks.
//! The calibration is the only thing that noticed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ===============================================================================================
// The crates this file speaks for
// ===============================================================================================

/// One crate under audit, and what the scanners must find in it.
struct AuditedCrate {
    /// The crate directory name under `crates/`.
    name: &'static str,
    /// The `#[derive(…FromXml…)]` sites the derive scanner must still find.
    minimum_derived: usize,
    /// The `fidelity_*!` invocations the macro scanner must still find.
    minimum_macro_invocations: usize,
    /// The element declarations the struct scanner must still find — concrete types plus
    /// `macro_rules!` families, counted once each.
    minimum_element_declarations: usize,
    /// The name of this crate's own fidelity macro, whose body is checked below.
    fidelity_macro: &'static str,
}

/// The three rank-2.2 crates, in the order the guide introduces them.
///
/// At 0.0.140 the scanners find **62 element declarations** between them and **no** hand-written
/// impl: `mjx-chart` 36 declarations (30 derived — 29 written out plus `declare_plot!`, which backs
/// ten — and 6 on `fidelity_element_impls!`), `mjx-omml` 10 (1 derived, 9 on the macro, of which
/// `fidelity_struct!` backs 38 types), `mjx-vml` 16 (5 derived, 11 on `fidelity_leaf!`).
///
/// The floors below sit well under those, and are stated as *the scanner is still matching* rather
/// than as *the crate holds exactly this many*: a floor pinned to the exact population fires before
/// the assertion it guards and hides the mutation meant to prove that assertion.
/// `xtask/tests/doc_gate.rs`'s header states the same rule and gives its history.
const UPPER_MARKUP: &[AuditedCrate] = &[
    AuditedCrate {
        name: "mjx-chart",
        minimum_element_declarations: 25,
        minimum_derived: 20,
        minimum_macro_invocations: 4,
        fidelity_macro: "fidelity_element_impls",
    },
    AuditedCrate {
        name: "mjx-omml",
        minimum_element_declarations: 6,
        minimum_derived: 1,
        minimum_macro_invocations: 30,
        fidelity_macro: "fidelity_element_impls",
    },
    AuditedCrate {
        name: "mjx-vml",
        minimum_element_declarations: 10,
        minimum_derived: 3,
        minimum_macro_invocations: 8,
        fidelity_macro: "fidelity_leaf",
    },
];

/// The two crates the impl scanner is calibrated against — the ones that **do** hand-write, and
/// whose own ledgers say how many.
///
/// The numbers are floors, for the same reason as above, and they are what stops this file's zero
/// from being a statement about a broken scanner. `mjx-dml` holds 7 `FromXml` and 6 `ToXml`;
/// `mjx-sml` holds 6 and 58 (MJXOFF-218's census reported 5 and 57, having missed the one impl
/// written with a qualified trait path — `crates/mjx-sml/src/font/color.rs`'s `ColorElement`).
const CALIBRATION: &[(&str, usize)] = &[("mjx-dml", 10), ("mjx-sml", 50)];

/// Every hand-written `FromXml`/`ToXml` impl in the three crates above, with the reason the two
/// generic mechanisms do not fit it.
///
/// **It is empty, and that is the finding**, not an omission: at 0.0.140 no type in rank 2.2 reads
/// or rebuilds an element by hand. A new one fails
/// [`no_type_in_the_upper_markup_serializes_itself_by_hand`] until somebody adds a row here, which
/// is a person writing down what the type keeps and why — the review MJXOFF-216 shows nothing else
/// performs.
const HAND_WRITTEN_IMPLS: &[HandWrittenLedgerRow] = &[];

/// A row of [`HAND_WRITTEN_IMPLS`].
struct HandWrittenLedgerRow {
    /// The crate directory name.
    #[allow(dead_code)]
    krate: &'static str,
    /// The source file, relative to that crate's `src/`.
    file: &'static str,
    /// The type the impl is written for.
    ty: &'static str,
    /// Why neither the derive nor the crate's fidelity macro fits. A reader who disagrees with this
    /// sentence has found a defect.
    #[allow(dead_code)]
    reason: &'static str,
}

/// Every type in the three crates that models an element and is on **neither** mechanism, with the
/// reason.
///
/// Also empty at 0.0.140, and checked by
/// [`every_element_modelling_type_is_on_one_of_the_two_mechanisms`].
const OFF_BOTH_MECHANISMS: &[&str] = &[];

// ===============================================================================================
// The scanner
// ===============================================================================================

/// The workspace root — `xtask/`'s parent.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// Every `.rs` file under `crates/<name>/src`, keyed by its path relative to that `src/`.
fn sources(krate: &str) -> BTreeMap<String, String> {
    fn walk(directory: &Path, root: &Path, into: &mut BTreeMap<String, String>) {
        let entries = std::fs::read_dir(directory).expect("reading a source directory");
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(&path, root, into);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let relative = path
                    .strip_prefix(root)
                    .expect("under the source root")
                    .to_string_lossy()
                    .replace('\\', "/");
                into.insert(
                    relative,
                    std::fs::read_to_string(&path).expect("reading a source file"),
                );
            }
        }
    }
    let root = repository_root().join("crates").join(krate).join("src");
    assert!(
        root.is_dir(),
        "{} does not exist — the crate has moved and this gate is now checking nothing",
        root.display()
    );
    let mut found = BTreeMap::new();
    walk(&root, &root, &mut found);
    assert!(
        !found.is_empty(),
        "no source files under {} — the walk has stopped walking",
        root.display()
    );
    found
}

/// The `{ … }` block beginning at or after `from`, brace-balanced.
fn balanced_block(text: &str, from: usize) -> String {
    let bytes = text.as_bytes();
    let mut at = from;
    while at < bytes.len() && bytes[at] != b'{' {
        at += 1;
    }
    let start = at;
    let mut depth = 0usize;
    while at < bytes.len() {
        match bytes[at] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return text[start..=at].to_owned();
                }
            }
            _ => {}
        }
        at += 1;
    }
    text[start..].to_owned()
}

/// `text` with every run of whitespace collapsed to one space.
fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One `impl FromXml for T` / `impl ToXml for T` written out at column zero.
struct HandWritten {
    /// The source file, relative to the crate's `src/`.
    file: String,
    /// The type it is written for.
    ty: String,
}

/// Every hand-written `FromXml`/`ToXml` impl in `krate`.
///
/// **Column zero only.** An `impl` indented inside a `macro_rules!` body is one place backing many
/// types — that is the second mechanism, counted separately by
/// [`fidelity_macro_bodies`] — and is not a hand-written impl per type.
///
/// The trait may be written qualified: `crates/mjx-sml/src/font/color.rs` writes
/// `impl mjx_ooxml_core::FromXml for ColorElement`, and a scanner keyed on a bare `impl FromXml for`
/// misses it. MJXOFF-218's own census did exactly that and reported one fewer than there is, which
/// is why the path between `impl ` and the trait name is admitted here.
fn hand_written_impls(krate: &str) -> Vec<HandWritten> {
    let mut found = Vec::new();
    for (file, text) in sources(krate) {
        for trait_name in ["FromXml", "ToXml"] {
            for (at, _) in text.match_indices(&format!("{trait_name} for ")) {
                let line_start = text[..at].rfind('\n').map_or(0, |newline| newline + 1);
                let Some(prefix) = text[line_start..at].strip_prefix("impl ") else {
                    continue;
                };
                if !prefix.chars().all(|character| {
                    character.is_alphanumeric() || character == '_' || character == ':'
                }) {
                    continue;
                }
                let after = at + trait_name.len() + " for ".len();
                let ty: String = text[after..]
                    .chars()
                    .take_while(|character| character.is_alphanumeric() || *character == '_')
                    .collect();
                assert!(
                    !ty.is_empty(),
                    "{krate}/{file}: `impl {trait_name} for` with no type name after it"
                );
                found.push(HandWritten {
                    file: file.clone(),
                    ty,
                });
            }
        }
    }
    found
}

/// One crate's `fidelity_*!` macro, split into the two impl bodies it generates.
struct FidelityMacro {
    /// The source file the `macro_rules!` is written in, relative to `src/`.
    file: String,
    /// The `from_xml` body, braces included.
    reader: String,
    /// The `to_xml` body, braces included.
    writer: String,
}

/// The `macro_rules! <name>` in `krate` that generates the fidelity impls, and the two bodies inside
/// it.
fn fidelity_macro_bodies(krate: &str, macro_name: &str) -> FidelityMacro {
    let needle = format!("macro_rules! {macro_name}");
    let mut found: Vec<FidelityMacro> = Vec::new();
    for (file, text) in sources(krate) {
        let Some(at) = text.find(&needle) else {
            continue;
        };
        let body = balanced_block(&text, at + needle.len());
        let reader_at = body
            .find("fn from_xml")
            .unwrap_or_else(|| panic!("{krate}/{file}: {macro_name}! generates no `from_xml`"));
        let writer_at = body
            .find("fn to_xml")
            .unwrap_or_else(|| panic!("{krate}/{file}: {macro_name}! generates no `to_xml`"));
        // The body of a function begins after its return type, which is the first `{` after the
        // `->`. `balanced_block` finds the first `{` from where it is told to look, so it is told to
        // look from the arrow.
        let reader_arrow = body[reader_at..]
            .find("->")
            .map(|offset| reader_at + offset)
            .expect("from_xml declares a return type");
        let writer_arrow = body[writer_at..]
            .find("->")
            .map(|offset| writer_at + offset)
            .expect("to_xml declares a return type");
        found.push(FidelityMacro {
            file: file.clone(),
            reader: balanced_block(&body, reader_arrow),
            writer: balanced_block(&body, writer_arrow),
        });
    }
    assert_eq!(
        found.len(),
        1,
        "{krate} declares {} macros named `{macro_name}` — this gate reads exactly one, and two \
         would mean half the crate is checked against the wrong body",
        found.len()
    );
    found.pop().expect("exactly one")
}

/// How a type that models an element reaches XML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mechanism {
    /// `#[derive(FromXml, ToXml)]` — `mjx-derive`'s codegen.
    Derive,
    /// The crate's own `fidelity_*!` macro.
    FidelityMacro,
    /// Neither, which is what this file exists to notice.
    Neither,
}

/// One declaration of a type that models an XML element.
struct ElementDeclaration {
    /// The source file, relative to the crate's `src/`.
    file: String,
    /// The line the `struct` keyword is on.
    line: usize,
    /// The type's name, or the metavariable a `macro_rules!` declares it under (`$ty`, `$name`).
    name: String,
    /// Whether this declaration is inside a `macro_rules!` body, and therefore backs a family of
    /// types rather than one.
    family: bool,
    /// How it reaches XML.
    mechanism: Mechanism,
}

/// Every `fidelity_*!(Type)` argument named in `krate`, so a concrete declaration can be matched to
/// the macro that wires it.
fn fidelity_macro_invocations(krate: &str, macro_name: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for (_, text) in sources(krate) {
        for (at, _) in text.match_indices(&format!("{macro_name}!(")) {
            let line_start = text[..at].rfind('\n').map_or(0, |newline| newline + 1);
            if text[line_start..at].contains("macro_rules! ") {
                continue;
            }
            let argument: String = text[at + macro_name.len() + 2..]
                .chars()
                .take_while(|character| character.is_alphanumeric() || *character == '_')
                .collect();
            if !argument.is_empty() {
                found.insert(argument);
            }
        }
        // `fidelity_struct! { … pub struct Name }` — `mjx-omml`'s wrapper, which declares the struct
        // and invokes `fidelity_element_impls!` on it in one statement. 38 of that crate's types
        // arrive this way, and a scanner reading only the inner invocation would see one site.
        for (at, _) in text.match_indices("fidelity_struct!") {
            let block = balanced_block(&text, at);
            if let Some(struct_at) = block.find("pub struct ") {
                let name: String = block[struct_at + "pub struct ".len()..]
                    .chars()
                    .take_while(|character| character.is_alphanumeric() || *character == '_')
                    .collect();
                if !name.is_empty() {
                    found.insert(name);
                }
            }
        }
    }
    found
}

/// Whether the `struct` keyword at `at` is an item declaration rather than the word *struct* inside
/// a comment or a doc sentence.
///
/// The line before it may hold only whitespace and a visibility — `pub`, `pub(crate)`, or a
/// `macro_rules!` metavariable standing in for one (`$vis`). This is the check that keeps
/// `mjx-chart`'s own *"a wrapper `struct` whose fields are exactly …"* out of the inventory.
fn is_item_declaration(text: &str, at: usize) -> bool {
    let line_start = text[..at].rfind('\n').map_or(0, |newline| newline + 1);
    let prefix = text[line_start..at].trim();
    prefix.is_empty()
        || prefix == "pub"
        || prefix == "pub(crate)"
        || prefix == "$vis"
        || (prefix.starts_with("pub(") && prefix.ends_with(')'))
}

/// Whether the `derive` attribute nearest above `at` names `FromXml` or `ToXml`.
///
/// A window rather than a parse: a derive list may wrap across lines, and everything between an
/// attribute and the item it decorates is doc comments and further attributes.
fn is_derived_at(text: &str, at: usize) -> bool {
    const WINDOW: usize = 600;
    let from = at.saturating_sub(WINDOW);
    let Some(open) = text[from..at].rfind("#[derive(") else {
        return false;
    };
    let open = from + open + "#[derive(".len();
    match text[open..at].find(")]") {
        Some(close) => {
            let list = &text[open..open + close];
            list.contains("FromXml") || list.contains("ToXml")
        }
        None => false,
    }
}

/// Every type in `krate` that models an XML element, and how it reaches XML.
///
/// A type models an element when its fields are the ones the derive and both fidelity macros are
/// written around — `name: RawName` beside `empty: bool` — or when its whole state is a
/// `RawElement`. The two shapes are the two macros: `mjx-chart`'s and `mjx-omml`'s keep the four
/// fields apart, `mjx-vml`'s keeps the element whole, which is what gives its types
/// `raw()`/`raw_mut()`.
///
/// **A declaration inside a `macro_rules!` body is counted once, not once per type it backs.** That
/// is the same rule this file's header states for impls, and it is what makes the check tractable:
/// `crates/mjx-chart/src/plot.rs`'s `declare_plot!` declares ten plot types from one `pub struct
/// $ty`, and reading that one declaration establishes the mechanism for all ten.
fn element_declarations(krate: &str, fidelity_macro: &str) -> Vec<ElementDeclaration> {
    let wired = fidelity_macro_invocations(krate, fidelity_macro);
    let mut found = Vec::new();
    for (file, text) in sources(krate) {
        let macro_bodies: Vec<(usize, usize)> = text
            .match_indices("macro_rules! ")
            .map(|(at, _)| {
                let block = balanced_block(&text, at);
                let start = text[at..].find('{').map_or(at, |offset| at + offset);
                (start, start + block.len())
            })
            .collect();
        for (at, _) in text.match_indices("struct ") {
            if !is_item_declaration(&text, at) {
                continue;
            }
            let after = at + "struct ".len();
            let rest = &text[after..];
            let name: String = if let Some(metavariable) = rest.strip_prefix('$') {
                let ident: String = metavariable
                    .chars()
                    .take_while(|character| character.is_alphanumeric() || *character == '_')
                    .collect();
                format!("${ident}")
            } else {
                rest.chars()
                    .take_while(|character| character.is_alphanumeric() || *character == '_')
                    .collect()
            };
            if name.is_empty() || name == "$" {
                continue;
            }
            let Some(brace) = text[after..]
                .find(['{', ';', '(', '<'])
                .map(|off| after + off)
            else {
                continue;
            };
            if text.as_bytes()[brace] != b'{' {
                continue;
            }
            let block = collapsed(&balanced_block(&text, brace));
            let framework = block.contains("name: RawName") && block.contains("empty: bool");
            let whole_element = block.contains("element: RawElement");
            if !(framework || whole_element) {
                continue;
            }
            let family = macro_bodies
                .iter()
                .any(|(start, end)| at > *start && at < *end);
            let mechanism = if is_derived_at(&text, at) {
                Mechanism::Derive
            } else if wired.contains(name.trim_start_matches('$'))
                || (family
                    && enclosing_macro_wires(&text, &macro_bodies, at, fidelity_macro, &name))
            {
                Mechanism::FidelityMacro
            } else {
                Mechanism::Neither
            };
            found.push(ElementDeclaration {
                file: file.clone(),
                line: text[..at].matches('\n').count() + 1,
                name,
                family,
                mechanism,
            });
        }
    }
    found
}

/// Whether the `macro_rules!` body containing `at` invokes `fidelity_macro` on the metavariable the
/// struct is declared under — `fidelity_element_impls!($name)` beside `$vis struct $name`.
fn enclosing_macro_wires(
    text: &str,
    macro_bodies: &[(usize, usize)],
    at: usize,
    fidelity_macro: &str,
    name: &str,
) -> bool {
    macro_bodies
        .iter()
        .filter(|(start, end)| at > *start && at < *end)
        .any(|(start, end)| text[*start..*end].contains(&format!("{fidelity_macro}!({name})")))
}

// ===============================================================================================
// The tests
// ===============================================================================================

/// Every scanner in this file is still matching, and the population of each mechanism is printed.
///
/// A count is what distinguishes *ran* from *skipped quietly*, so all of them are printed on success
/// and not only on failure.
#[test]
fn the_scanner_still_sees_both_mechanisms_in_all_three_crates() {
    for audited in UPPER_MARKUP {
        let declarations = element_declarations(audited.name, audited.fidelity_macro);
        let invocations = fidelity_macro_invocations(audited.name, audited.fidelity_macro);
        let derived = declarations
            .iter()
            .filter(|one| one.mechanism == Mechanism::Derive)
            .count();
        let families = declarations.iter().filter(|one| one.family).count();
        println!(
            "{}: {} element declarations ({derived} derived, {} on `{}!`, {families} of them \
             `macro_rules!` families), {} types wired by `{}!`, {} hand-written impls",
            audited.name,
            declarations.len(),
            declarations.len() - derived,
            audited.fidelity_macro,
            invocations.len(),
            audited.fidelity_macro,
            hand_written_impls(audited.name).len(),
        );
        assert!(
            derived >= audited.minimum_derived,
            "{}: only {derived} derived element declarations found — the derive scanner has stopped \
             matching",
            audited.name
        );
        assert!(
            invocations.len() >= audited.minimum_macro_invocations,
            "{}: only {} `{}!` invocations found — the macro scanner has stopped matching",
            audited.name,
            invocations.len(),
            audited.fidelity_macro
        );
        assert!(
            declarations.len() >= audited.minimum_element_declarations,
            "{}: only {} element declarations found — the struct scanner has stopped matching, and \
             it is the one that decides whether a type is audited at all",
            audited.name,
            declarations.len()
        );
    }
}

/// **The impl scanner reports zero for these three because there are none, not because it is
/// broken.**
///
/// A zero cannot carry a floor of its own, so the same scanner is pointed at the two crates that do
/// hand-write and required to find their populations. If it stops matching it reports zero
/// everywhere, and this fails before
/// [`no_type_in_the_upper_markup_serializes_itself_by_hand`] can report a clean bill it has not
/// earned.
#[test]
fn the_impl_scanner_is_calibrated_against_the_two_crates_that_do_hand_write() {
    for (krate, floor) in CALIBRATION {
        let found = hand_written_impls(krate);
        println!(
            "{krate}: {} hand-written impls (floor {floor})",
            found.len()
        );
        assert!(
            found.len() >= *floor,
            "{krate}: only {} hand-written impls found, and its own \
             crates/{krate}/tests/serialization_ledger.rs accounts for more than {floor} — the \
             scanner this file's zero depends on has stopped matching",
            found.len()
        );
    }
}

/// **No type in `mjx-chart`, `mjx-omml` or `mjx-vml` reads or rebuilds an element by hand.**
///
/// This is MJXOFF-218's question, answered for the last three crates it covers, and the answer is a
/// number: zero. A type that stops using the shared mechanism has to be looked at by a person and
/// written onto [`HAND_WRITTEN_IMPLS`] — which is the review MJXOFF-216 demonstrated nothing else
/// performs.
#[test]
fn no_type_in_the_upper_markup_serializes_itself_by_hand() {
    let ledger: BTreeSet<(&str, &str)> = HAND_WRITTEN_IMPLS
        .iter()
        .map(|row| (row.file, row.ty))
        .collect();
    let mut total = 0usize;
    for audited in UPPER_MARKUP {
        for found in hand_written_impls(audited.name) {
            total += 1;
            assert!(
                ledger.contains(&(found.file.as_str(), found.ty.as_str())),
                "crates/{}/src/{} hand-writes a `FromXml`/`ToXml` for `{}`. Every element type in \
                 rank 2.2 goes through `#[derive(FromXml, ToXml)]` or the crate's own \
                 `fidelity_*!` macro, so this is a new mechanism with no gate behind it — the shape \
                 MJXOFF-216 found on `mjx_dml::Picture::to_xml`. Say on HAND_WRITTEN_IMPLS what it \
                 keeps and why neither mechanism fits.",
                audited.name,
                found.file,
                found.ty
            );
        }
    }
    println!(
        "rank 2.2: {total} hand-written FromXml/ToXml impls, {} on the ledger",
        ledger.len()
    );
}

/// **Every type that models an element is on the derive or on its crate's fidelity macro.**
///
/// The complement of the test above: that one says nothing is hand-written, this one says nothing is
/// *unwired*. A struct carrying the framework fields but reaching XML through neither mechanism has
/// a serializer somewhere this gate is not looking, or none at all — and in the second case its
/// content is reachable only through whatever built it.
#[test]
fn every_element_modelling_type_is_on_one_of_the_two_mechanisms() {
    let allowed: BTreeSet<&str> = OFF_BOTH_MECHANISMS.iter().copied().collect();
    let mut total = 0usize;
    for audited in UPPER_MARKUP {
        for declaration in element_declarations(audited.name, audited.fidelity_macro) {
            total += 1;
            if declaration.mechanism != Mechanism::Neither
                || allowed.contains(declaration.name.as_str())
            {
                continue;
            }
            panic!(
                "crates/{}/src/{}:{} declares `{}`, which models an element but is on neither \
                 `#[derive(FromXml, ToXml)]` nor `{}!`. Put it on one of the two, or on \
                 OFF_BOTH_MECHANISMS with the reason.",
                audited.name,
                declaration.file,
                declaration.line,
                declaration.name,
                audited.fidelity_macro
            );
        }
    }
    println!(
        "rank 2.2: {total} element declarations, {} exempted on OFF_BOTH_MECHANISMS",
        allowed.len()
    );
}

/// **Each crate's fidelity macro still writes back every field it read.**
///
/// This is the one body check in the file, and it is the MJXOFF-216 shape stated generically: a
/// writer that captured a field on the way in and does not mention it on the way out has dropped
/// whatever that field held. `Picture::to_xml` handed `RawElement::rebuilt` a fresh `Vec::new()`
/// where its reader had captured the element's children, and every child of every picture in every
/// file went with it.
///
/// Three macros, three bodies, and between them they back every element type in rank 2.2 that is not
/// derived. Each is required to name, in its `to_xml`, every field its own `from_xml` captured — and
/// forbidden from constructing an empty collection, which is the only way the first requirement can
/// be met while still losing the content.
#[test]
fn each_fidelity_macro_writes_back_every_field_it_read() {
    /// The fields a fidelity reader may capture. A macro is held to the ones it actually uses, so a
    /// crate whose model keeps the element whole is checked on `element` and not on four fields it
    /// never had.
    const FIELDS: &[&str] = &["name", "attributes", "children", "empty", "element"];

    let mut checked = 0usize;
    for audited in UPPER_MARKUP {
        let macro_ = fidelity_macro_bodies(audited.name, audited.fidelity_macro);
        let reader = collapsed(&macro_.reader);
        let writer = collapsed(&macro_.writer);
        let captured: Vec<&str> = FIELDS
            .iter()
            .copied()
            .filter(|field| {
                reader.contains(&format!("{field}:"))
                    || reader.contains(&format!("element.{field}"))
            })
            .collect();
        assert!(
            !captured.is_empty(),
            "{}/{}: `{}!`'s reader captures none of {FIELDS:?} — the body reader has stopped \
             matching",
            audited.name,
            macro_.file,
            audited.fidelity_macro
        );
        for field in &captured {
            assert!(
                writer.contains(field),
                "{}/{}: `{}!`'s reader captures `{field}` and its writer never mentions it, so \
                 everything that field holds is dropped on the way out — MJXOFF-216's shape.",
                audited.name,
                macro_.file,
                audited.fidelity_macro
            );
        }
        assert!(
            !writer.contains("Vec::new()") && !writer.contains("vec![]"),
            "{}/{}: `{}!`'s writer builds a fresh empty collection. That is exactly how \
             `mjx_dml::Picture::to_xml` dropped every child of every picture (MJXOFF-216); a \
             rebuilt element carries the children it was read with.",
            audited.name,
            macro_.file,
            audited.fidelity_macro
        );
        println!(
            "{}/{}: `{}!` captures {captured:?} and writes all of them back",
            audited.name, macro_.file, audited.fidelity_macro
        );
        checked += 1;
    }
    assert_eq!(
        checked,
        UPPER_MARKUP.len(),
        "a crate's fidelity macro was not read at all"
    );
}
