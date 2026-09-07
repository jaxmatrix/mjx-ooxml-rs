//! **Every hand-written `FromXml`/`ToXml` impl in this crate is accounted for, and the rebuilder
//! each one delegates to is held to the shape that keeps what it does not model** (MJXOFF-220,
//! closing `mjx-sml`'s half of MJXOFF-218).
//!
//! # The question here is not the question `mjx-dml` answered
//!
//! `crates/mjx-dml/tests/serialization_ledger.rs` (MJXOFF-217) asks, of eight hand-written
//! **pairs**, whether a type that reads an element by hand and rebuilds it by hand loses anything
//! in between. That was the right question there: MJXOFF-216 found four of the eight destroying a
//! foreign attribute, a foreign child or an `xmlns` declaration, and the four bodies were four
//! different mistakes.
//!
//! Applying it here unchanged would be **vacuous**, and the reason is worth writing down because it
//! is the shape MJXOFF-88 §7 warns about — *a gate phrased "X is covered and green" is green
//! precisely when X is skipped*:
//!
//! * This crate's hand-written impls are overwhelmingly `ToXml`-**only**. A `ToXml`-only type
//!   cannot lose what it never read: its reader is `#[derive(FromXml)]`, whose codegen emits the
//!   `Raw` fallthrough unconditionally and is backed once, for every derived type at once, by
//!   `crates/mjx-derive/tests/derive.rs`.
//! * And every one of those writers is the **same three lines** —
//!   `fn to_xml(&self, _interner: &mut Interner) -> RawElement { self.as_raw_element() }`. Under
//!   `mjx-dml`'s vocabulary each is a `Dispatcher`, and each would pass a `Dispatcher`'s check
//!   (*constructs no element of its own*) trivially, forever, whatever the rebuilder behind it did.
//!   A ledger of one reason repeated fifty-seven times is a list, and a list passes once it is
//!   written.
//!
//! **So the risk moved one hop, and this file follows it there.** What can lose content in
//! `mjx-sml` is the inherent `as_raw_element(&self)` the writer delegates to, and there are more of
//! those than there are `ToXml` impls. This file therefore checks three things rather than one:
//! that every hand-written writer really is that delegation (or is on a ledger saying why not),
//! that every rebuilder in the crate really does rebuild from the element's own name, attributes
//! and self-closing flag, and that every hand-written **reader** is on a ledger with a reason.
//!
//! # Why an `as_raw_element` at all, rather than the derive's `ToXml`
//!
//! [`ToXml::to_xml`](mjx_ooxml_core::ToXml::to_xml) takes `&mut Interner`, because a model that
//! authors an element name has to intern it. Nothing in this crate's worksheet family ever authors
//! one — every element keeps the [`RawName`](mjx_ooxml_core::RawName) it was read with — and
//! `mjx_sml::WorksheetPart`'s writer is a **`&self` byte writer**, which is what lets the
//! `sheetData` slot be MJXOFF-95's packed store instead of a subtree. A `&self` writer has no
//! mutable interner to lend, so the rebuild is an inherent method and `to_xml` is that method with
//! the parameter ignored. `crates/mjx-sml/src/worksheet/mod.rs`'s `rebuild_element` states the same
//! reason at the point it is acted on.
//!
//! # Scope, stated so the hole is deliberate rather than silent
//!
//! **This gate covers `mjx-sml` only**, as `mjx-dml`'s covers `mjx-dml` only. `mjx-docx` writes 158
//! hand-written pairs of the fully-preserving body, and classifying those is the other half of
//! MJXOFF-218.
//!
//! **Three files, not one shared crate, and that is a decision rather than an oversight.** A shared
//! gate would have to be a workspace member outside the layering graph (the shape `mjx-fixtures`
//! and `mjx-allocation-counter` have), and it would carry the source scanner — sixty lines — while
//! each crate would still hold its own ledger and its own idioms, because the idioms *are* the
//! finding and they differ per crate: `mjx-dml`'s are eight bespoke pairs, this crate's are one
//! delegation repeated and a family of rebuilders behind it, and `mjx-docx`'s are 158 copies of a
//! fourth thing. Sharing the cheap half and duplicating the expensive half buys nothing and adds a
//! crate to `CLAUDE.md`'s table.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ===============================================================================================
// The ledgers
// ===============================================================================================

/// A hand-written `ToXml` whose body is **not** this crate's one delegation.
///
/// A new one fails [`every_hand_written_writer_is_this_crate_s_idiom_or_on_the_ledger`] until
/// somebody adds a row saying what it keeps and why the delegation does not fit.
struct BespokeWriter {
    /// The source file, relative to `crates/mjx-sml/src/`.
    file: &'static str,
    /// The type the impl is written for.
    ty: &'static str,
    /// Why it is not the delegation. A reader who disagrees with this sentence has found a defect.
    reason: &'static str,
}

/// Every hand-written `ToXml` in this crate that is not `{ self.as_raw_element() }`.
const BESPOKE_WRITERS: &[BespokeWriter] = &[BespokeWriter {
    file: "workbook/defined_names.rs",
    ty: "DefinedName",
    reason:
        "`CT_DefinedName`'s content is character data, so the write path has two cases rather than \
         one: an untouched name replays the children the file held — an entity spelling, a CDATA \
         section, a comment between them — and one `set_definition` has reached writes a single \
         freshly escaped text node. The two formula leaves make the same choice through an \
         `as_raw_element`; this one is written inline because the type has no other caller for it. \
         It is still held to the preserving shape below.",
}];

/// A hand-written `FromXml` — a type that declines `mjx-derive`'s reader.
struct HandWrittenReader {
    /// The source file, relative to `crates/mjx-sml/src/`.
    file: &'static str,
    /// The type the impl is written for.
    ty: &'static str,
    /// Why the derive does not fit. A reader who disagrees with this sentence has found a defect.
    reason: &'static str,
}

/// Every hand-written `FromXml` in this crate.
///
/// Six, and they fall into three groups rather than one. The two formula leaves and `CommentText`
/// decline the derive's **text** arm, which reads only text and CDATA nodes and writes one
/// minimally-escaped text node; the other three are whole-element bags that keep every child
/// unexamined and therefore have no field list for the derive to generate.
const HAND_WRITTEN_READERS: &[HandWrittenReader] = &[
    HandWrittenReader {
        file: "comments.rs",
        ty: "CommentText",
        reason: "`CT_Rst` is rich text: a `t`, or a run of `r` elements each with its own `rPr`, \
                 or phonetic markup beside them. This reader keeps every child verbatim and \
                 decodes the concatenated text beside them, so `runs_markup` can hand back the run \
                 structure this type does not model. The derive has no arm that reads a subtree \
                 and a decoded string from the same element.",
    },
    HandWrittenReader {
        file: "font/color.rs",
        ty: "ColorElement",
        reason: "`CT_Color` is the same five attributes under four different element names \
                 (`color`, `fgColor`, `bgColor`, `tabColor`), so the wrapper keeps the name it was \
                 read with and every attribute unexamined, and `Color::read_attributes` decodes \
                 them on demand. Modelling the five attributes would make the element's own \
                 spelling — `ffff0000` against `FFFF0000` — a thing the writer chose.",
    },
    HandWrittenReader {
        file: "formula/element.rs",
        ty: "FormulaElement",
        reason: "The `ST_Formula` element three worksheet slots share. A text leaf, for the reason \
                 its `ToXml` row on BESPOKE_WRITERS gives: the derive's text arm would drop an \
                 entity spelling, a CDATA section and any comment between the children.",
    },
    HandWrittenReader {
        file: "features/tables.rs",
        ty: "TableFormula",
        reason: "A table column's calculated-column and totals-row formulas, the second \
                 `ST_Formula` leaf. The same text-arm reason as `FormulaElement`, in the file that \
                 owns worksheet tables rather than formulas.",
    },
    HandWrittenReader {
        file: "styles/fonts.rs",
        ty: "Font",
        reason: "`CT_Font` is an unordered `xsd:choice` of eighteen child elements, and a font is \
                 addressed by index rather than by shape: nothing in this crate reads a `b` out of \
                 one without going through `FontProperties`. So the type is the element, kept \
                 whole, and `from_properties` is the only path that authors one.",
    },
    HandWrittenReader {
        file: "workbook/defined_names.rs",
        ty: "DefinedName",
        reason: "`CT_DefinedName`'s content is its definition — a formula, as character data — and \
                 it is the leaf `crates/mjx-sml/tests/workbook_markup.rs`'s \
                 `an_entity_spelling_in_a_definition_survives_an_edit_elsewhere` pins. The derive's \
                 text arm would rewrite `&#38;` as `&amp;` in a name nobody edited.",
    },
];

// ===============================================================================================
// The scanner
// ===============================================================================================

/// `crates/mjx-sml/src`.
fn crate_source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.rs` file under `src/`, keyed by its path relative to it.
fn sources() -> BTreeMap<String, String> {
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
    let root = crate_source_root();
    let mut sources = BTreeMap::new();
    walk(&root, &root, &mut sources);
    assert!(
        sources.len() > 40,
        "only {} source files found under {} — the walk has stopped walking",
        sources.len(),
        root.display()
    );
    sources
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

/// One `impl FromXml for T` or `impl ToXml for T` written out in a source file.
struct HandWritten {
    /// The source file, relative to `src/`.
    file: String,
    /// `"FromXml"` or `"ToXml"`.
    trait_name: &'static str,
    /// The type it is written for.
    ty: String,
    /// The impl block, braces included.
    body: String,
}

/// Every hand-written `FromXml`/`ToXml` impl in this crate.
///
/// Only impls at **column zero**. An `impl` indented inside a `macro_rules!` body is one place
/// backing many types — this crate's counterpart to `mjx-dml`'s `fidelity_element_impls!` — and is
/// counted separately by [`the_scanner_still_sees_every_serialization_mechanism`], for the reason
/// that file gives: reading one macro body backs every type that invokes it, so it is not a
/// hand-written impl per type.
///
/// The trait may be written qualified. `font/color.rs` writes
/// `impl mjx_ooxml_core::ToXml for ColorElement`, and a scanner keyed on a bare `impl ToXml for`
/// misses it — which is exactly what MJXOFF-218's own census did, reporting 5 `FromXml` and 57
/// `ToXml` where there are 6 and 58.
fn hand_written_impls() -> Vec<HandWritten> {
    let mut found = Vec::new();
    for (file, text) in sources() {
        for trait_name in ["FromXml", "ToXml"] {
            for (at, _) in text.match_indices(&format!("{trait_name} for ")) {
                // The line must begin `impl `, at column zero, with only a path between the two.
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
                    "{file}: `impl {trait_name} for` with no type name after it"
                );
                found.push(HandWritten {
                    file: file.clone(),
                    trait_name,
                    ty,
                    body: balanced_block(&text, after),
                });
            }
        }
    }
    found
}

/// One inherent `fn as_raw_element(&self)` — the method a hand-written writer delegates to.
struct Rebuilder {
    /// The source file, relative to `src/`.
    file: String,
    /// The line it starts on, so a failure names a place a person can open.
    line: usize,
    /// Whether it answers `Option<RawElement>`, which is the content-enum dispatcher shape.
    dispatching: bool,
    /// The body, or `None` for the trait declaration in `sheets/frame.rs` that has none.
    body: Option<String>,
}

/// Every `fn as_raw_element(&self)` in this crate, macro bodies included.
///
/// A macro body is scanned exactly like a written-out one: it is the *one* place its rule lives, so
/// checking it once checks every type that invokes it. `leaf.rs`'s two shape macros and
/// `features/embedded.rs`'s `entry_list!` are all read here.
fn rebuilders() -> Vec<Rebuilder> {
    const SIGNATURE: &str = "fn as_raw_element(&self)";
    let mut found = Vec::new();
    for (file, text) in sources() {
        for (at, _) in text.match_indices(SIGNATURE) {
            let after = at + SIGNATURE.len();
            let terminator = text[after..]
                .find(['{', ';'])
                .map(|offset| after + offset)
                .expect("a signature is followed by a body or a semicolon");
            let signature = &text[after..terminator];
            found.push(Rebuilder {
                file: file.clone(),
                line: text[..at].matches('\n').count() + 1,
                dispatching: signature.contains("Option"),
                body: (text.as_bytes()[terminator] == b'{')
                    .then(|| balanced_block(&text, terminator)),
            });
        }
    }
    found
}

/// The delegation every hand-written writer in this crate is, with its whitespace collapsed.
const THE_DELEGATION: &str = "{ self.as_raw_element() }";

/// `text` with every run of whitespace collapsed to one space, for comparing a body to a shape.
fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ===============================================================================================
// The anti-vacuity floors
// ===============================================================================================

/// The impl scanner must still be finding hand-written impls at all. Stated as *the scanner is
/// still matching*, never as *the crate holds exactly this many* — a floor pinned to the exact size
/// fires before the assertion it guards and hides the mutation that was meant to prove it.
const MINIMUM_HAND_WRITTEN_IMPLS: usize = 30;

/// …and the rebuilder scanner, which is where this crate's risk actually lives.
const MINIMUM_REBUILDERS: usize = 30;

/// The crate must still be seen to reach the derive, which is what reads almost all of it.
const MINIMUM_DERIVED_TYPES: usize = 40;

/// …and the shape macros, the second mechanism that needs no ledger.
const MINIMUM_MACRO_INVOCATIONS: usize = 50;

// ===============================================================================================
// The tests
// ===============================================================================================

/// The scanners are alive, and the two mechanisms that need no ledger are still the bulk of the
/// crate.
#[test]
fn the_scanner_still_sees_every_serialization_mechanism() {
    let hand_written = hand_written_impls();
    let rebuilders = rebuilders();
    let mut derived = 0usize;
    let mut macro_invocations = 0usize;
    for (_, text) in sources() {
        derived += text
            .split("#[derive(")
            .skip(1)
            .filter(|tail| {
                tail.find(")]")
                    .is_some_and(|end| tail[..end].contains("FromXml"))
            })
            .count();
        for shape in [
            "attribute_bag!",
            "bag_without_declared_attributes!",
            "relationship_reference!",
            "character_data_shape!",
            "entry_list!",
        ] {
            macro_invocations += text.matches(shape).count().saturating_sub(
                text.matches(&format!("macro_rules! {}", &shape[..shape.len() - 1]))
                    .count(),
            );
        }
    }

    let readers = hand_written
        .iter()
        .filter(|impl_| impl_.trait_name == "FromXml")
        .count();
    let writers = hand_written.len() - readers;
    let types: BTreeSet<(&str, &str)> = hand_written
        .iter()
        .map(|impl_| (impl_.file.as_str(), impl_.ty.as_str()))
        .collect();
    println!(
        "mjx-sml serialization: {derived} `#[derive(FromXml)]`, {macro_invocations} shape-macro \
         invocations, {} hand-written impls over {} types ({readers} FromXml, {writers} ToXml, \
         {} ToXml-only), delegating to {} rebuilders and {} content-enum dispatchers",
        hand_written.len(),
        types.len(),
        writers - readers,
        rebuilders.iter().filter(|one| !one.dispatching).count(),
        rebuilders
            .iter()
            .filter(|one| one.dispatching && one.body.is_some())
            .count(),
    );

    assert!(
        derived >= MINIMUM_DERIVED_TYPES,
        "only {derived} `#[derive(FromXml)]` sites found — the derive scanner has stopped matching"
    );
    assert!(
        macro_invocations >= MINIMUM_MACRO_INVOCATIONS,
        "only {macro_invocations} shape-macro invocations found — that scanner has stopped matching"
    );
    assert!(
        hand_written.len() >= MINIMUM_HAND_WRITTEN_IMPLS,
        "only {} hand-written impls found — the impl scanner has stopped matching",
        hand_written.len()
    );
    assert!(
        rebuilders.len() >= MINIMUM_REBUILDERS,
        "only {} `as_raw_element` definitions found — the rebuilder scanner has stopped matching, \
         and it is the one that guards this crate's actual risk",
        rebuilders.len()
    );
}

/// **Every hand-written `ToXml` is the delegation, or is on the ledger with a reason.**
///
/// This is the arm a new writer meets. A body that does anything but hand the work to the type's
/// own rebuilder is outside the check below it — the rebuilder check cannot see what a writer does
/// *instead* of delegating — so it has to be looked at by a person and written down.
#[test]
fn every_hand_written_writer_is_this_crate_s_idiom_or_on_the_ledger() {
    let ledger: BTreeSet<(&str, &str)> = BESPOKE_WRITERS
        .iter()
        .map(|entry| (entry.file, entry.ty))
        .collect();
    let mut delegating = 0usize;
    let mut unexplained = Vec::new();
    for impl_ in hand_written_impls() {
        if impl_.trait_name != "ToXml" {
            continue;
        }
        if collapsed(&impl_.body).contains(THE_DELEGATION) {
            delegating += 1;
            continue;
        }
        if !ledger.contains(&(impl_.file.as_str(), impl_.ty.as_str())) {
            unexplained.push(format!("{}: impl ToXml for {}", impl_.file, impl_.ty));
            continue;
        }
        // A row on the ledger buys an exception from the delegation, never from the shape: a
        // bespoke writer builds its own element and is therefore the one place in this crate where
        // MJXOFF-216's mistake could be typed directly.
        let where_ = format!("{}: impl ToXml for {}", impl_.file, impl_.ty);
        for marker in ["self.name", "self.attributes", "self.empty"] {
            assert!(
                impl_.body.contains(marker),
                "{where_} is on BESPOKE_WRITERS but never touches `{marker}` — a writer that \
                 builds an element without the one it was read from is MJXOFF-216's exact shape"
            );
        }
    }
    assert!(
        unexplained.is_empty(),
        "hand-written `ToXml` impls that neither delegate to an `as_raw_element` nor appear on \
         BESPOKE_WRITERS:\n  {}\n\nA writer that builds its element itself is outside the \
         rebuilder check in this file, which is the only thing standing between this crate and \
         MJXOFF-216's shape. Delegate, or add a row saying what it keeps.",
        unexplained.join("\n  ")
    );
    assert!(
        delegating >= MINIMUM_HAND_WRITTEN_IMPLS,
        "only {delegating} delegating writers matched — the body comparison has stopped matching"
    );
    println!("writers: {delegating} delegating, {} bespoke", ledger.len());
}

/// **Every rebuilder keeps the element's own name, its attributes and its self-closing flag.**
///
/// This is MJXOFF-216's shape, checked where this crate can actually meet it. `Picture::to_xml`
/// synthesised the element's name and handed `RawElement::rebuilt` a fresh `Vec::new()` for the
/// attributes, destroying a foreign attribute, a foreign child and every `xmlns` declaration on the
/// element — and passed all three of `mjx-dml`'s gates at once. Here the equivalent body is
/// `as_raw_element`, and there are more of them than there are `ToXml` impls, because the frames
/// call them directly.
///
/// A dispatcher — `Option<RawElement>` over a content enum — is held to the opposite claim: it must
/// construct no element at all, so it has nothing to lose and the type it forwards to is checked
/// instead.
#[test]
fn every_rebuilder_keeps_what_its_type_does_not_model() {
    let mut preserving = 0usize;
    let mut dispatching = 0usize;
    for rebuilder in rebuilders() {
        let where_ = format!("{}:{}", rebuilder.file, rebuilder.line);
        let Some(body) = rebuilder.body.as_deref() else {
            assert!(
                rebuilder.dispatching,
                "{where_}: an `as_raw_element` with no body is the `SheetContent` trait \
                 declaration, which answers `Option<RawElement>`"
            );
            continue;
        };
        if rebuilder.dispatching {
            dispatching += 1;
            assert!(
                !body.contains("RawElement::rebuilt") && !body.contains("rebuild_element("),
                "{where_}: a dispatching `as_raw_element` builds an element of its own, so it has \
                 something to lose and is no longer merely forwarding"
            );
            continue;
        }
        preserving += 1;
        assert!(
            body.contains("self.name"),
            "{where_}: a rebuilder that never reads the element's own name is synthesising one — \
             MJXOFF-216's exact shape"
        );
        assert!(
            body.contains("self.attributes"),
            "{where_}: a rebuilder that never passes `self.attributes` is handing the rebuild an \
             attribute list it made up — MJXOFF-216's exact shape"
        );
        assert!(
            body.contains("self.empty"),
            "{where_}: a rebuilder that ignores `self.empty` re-emits `<x/>` as `<x></x>`, which \
             is the loss MJXOFF-217 found on `mjx_dml::wordprocessing_drawing::Inline`"
        );
    }
    println!("rebuilders: {preserving} preserving, {dispatching} dispatching");
    assert!(
        preserving >= MINIMUM_REBUILDERS,
        "only {preserving} rebuilders were checked — the scanner has stopped matching"
    );
}

/// **Every hand-written `FromXml` is on the reader ledger**, and every row still names one.
///
/// A hand-written reader sits outside `mjx-derive`'s codegen guarantee by definition. Six decline
/// the derive here and each declines for a stated reason; a seventh fails until somebody writes
/// down which.
#[test]
fn every_hand_written_reader_is_on_the_ledger() {
    let ledger: BTreeSet<(&str, &str)> = HAND_WRITTEN_READERS
        .iter()
        .map(|entry| (entry.file, entry.ty))
        .collect();
    let found: BTreeSet<(String, String)> = hand_written_impls()
        .into_iter()
        .filter(|impl_| impl_.trait_name == "FromXml")
        .map(|impl_| (impl_.file, impl_.ty))
        .collect();

    let unlisted: Vec<String> = found
        .iter()
        .filter(|(file, ty)| !ledger.contains(&(file.as_str(), ty.as_str())))
        .map(|(file, ty)| format!("{file}: impl FromXml for {ty}"))
        .collect();
    assert!(
        unlisted.is_empty(),
        "hand-written `FromXml` impls that are not on HAND_WRITTEN_READERS:\n  {}\n\nPut it on the \
         derive, or add a row saying what the derive would lose.",
        unlisted.join("\n  ")
    );

    let stale: Vec<String> = HAND_WRITTEN_READERS
        .iter()
        .filter(|entry| !found.contains(&(entry.file.to_owned(), entry.ty.to_owned())))
        .map(|entry| format!("{}: {}", entry.file, entry.ty))
        .collect();
    assert!(
        stale.is_empty(),
        "HAND_WRITTEN_READERS rows naming no hand-written impl (moved to the derive, renamed or \
         deleted):\n  {}",
        stale.join("\n  ")
    );

    assert_eq!(
        found.len(),
        ledger.len(),
        "the reader ledger and the readers found are the same size or one of the two has a \
         duplicate"
    );
    println!("readers: {} hand-written, all on the ledger", found.len());
}

/// Every ledger row says something, and still names an impl of the trait it claims.
#[test]
fn every_ledger_row_earns_its_place() {
    let impls = hand_written_impls();
    for entry in BESPOKE_WRITERS {
        assert!(
            entry.reason.len() > 60,
            "{}: {} — a ledger reason has to say something",
            entry.file,
            entry.ty
        );
        let matching: Vec<&HandWritten> = impls
            .iter()
            .filter(|impl_| {
                impl_.trait_name == "ToXml" && impl_.file == entry.file && impl_.ty == entry.ty
            })
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "{}: {} is on BESPOKE_WRITERS but there is no single `impl ToXml` for it",
            entry.file,
            entry.ty
        );
        assert!(
            !collapsed(&matching[0].body).contains(THE_DELEGATION),
            "{}: {} is on BESPOKE_WRITERS but its body is now the plain delegation — take the row \
             out rather than leaving a reason nobody has to keep true",
            entry.file,
            entry.ty
        );
    }
    for entry in HAND_WRITTEN_READERS {
        assert!(
            entry.reason.len() > 60,
            "{}: {} — a ledger reason has to say something",
            entry.file,
            entry.ty
        );
    }
    println!(
        "ledger: {} bespoke writers, {} hand-written readers",
        BESPOKE_WRITERS.len(),
        HAND_WRITTEN_READERS.len()
    );
}
