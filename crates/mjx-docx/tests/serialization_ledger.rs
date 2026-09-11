//! **Every hand-written `FromXml`/`ToXml` impl in this crate is either the one body this crate
//! copies, character for character, or is on a ledger with an idiom the ledger can check**
//! (MJXOFF-218, closing the half MJXOFF-220 left).
//!
//! # The question here is not the question either earlier file answered
//!
//! `crates/mjx-dml/tests/serialization_ledger.rs` (MJXOFF-217) asks, of **eight** bespoke pairs,
//! whether each keeps what it does not model — eight different bodies, four of which were losing
//! content, so the answer had to be a reason per type.
//! `crates/mjx-sml/tests/serialization_ledger.rs` (MJXOFF-220) asks a different question again,
//! because that crate's writers are overwhelmingly `ToXml`-**only** one-line delegations and the
//! risk had moved one hop, into the `as_raw_element` behind them.
//!
//! Here the shape is a third thing. Every hand-written impl in this crate is a **pair**, and the
//! overwhelming majority of them are *the same body typed out again*: a reader that stores the
//! element's `name`, `attributes`, children and self-closing flag, and a writer that rebuilds from
//! exactly those four. Nothing is shared — no macro, no helper — so the risk is not "did somebody
//! design this type's preservation wrongly" but **"did somebody copy the body wrongly"**, a hundred
//! and forty-odd times over.
//!
//! A ledger with a row per type would be the wrong instrument for that: it would be the longest
//! list in the workspace, it would pass forever once written, and it would say nothing about the
//! one character that matters. So the primary arm of this file is a **character-for-character
//! comparison against the canonical body**, in both directions, with the bucket field required to
//! agree across the pair. A body that differs by so much as a `Vec::new()` is not the canonical body
//! any more and has to be looked at by a person and written down.
//!
//! # What that arm found
//!
//! Three types, all in `document/drawing.rs`, and all three the shapes MJXOFF-216 and MJXOFF-217
//! already named:
//!
//! * **`Control`** — its reader stored no children **at all** (the struct had no field for them) and
//!   its writer handed `RawElement::rebuilt` a fresh `Vec::new()` with the self-closing flag
//!   hard-coded `true`. `CT_Control` declares no content model, so this looked safe; the fidelity
//!   rule has no "the schema says this cannot happen" clause, and a foreign child, a comment or an
//!   `o:` extension between the tags was destroyed. `<w:control></w:control>` also came back
//!   `<w:control/>`.
//! * **`WordprocessingShape`** and **`TextboxInfo`** — both read `element.empty` into a field their
//!   writers then ignored in favour of a literal `false`, so `<wp:wsp/>` came back
//!   `<wp:wsp></wp:wsp>`. That is the loss MJXOFF-217 found twice in `mjx-dml`, twice more here.
//!
//! `crates/mjx-docx/tests/drawing_placement.rs`'s own MJXOFF-218 section is where those four losses
//! are proved against markup rather than against a source shape. Two more writers (`ObjectEmbed`,
//! `ObjectLink`) were correct but spelled differently; they were folded onto the canonical text, so
//! that the family really is one body and this file can compare rather than list.
//!
//! # Why three files and not one shared crate — the decision, on its third data point
//!
//! MJXOFF-220 recorded it: a shared gate would be a workspace member outside the layering graph (the
//! shape `mjx-fixtures` and `mjx-allocation-counter` have), it would carry the source scanner while
//! each crate still held its own ledger and its own idioms, "because the idioms *are* the finding
//! and they differ per crate".
//!
//! Writing the third file is the test of that claim, and it holds — more strongly than when it was
//! made. The three files share **the scanner**, sixty lines of brace matching, and share nothing
//! else: `mjx-dml` checks three idioms over eight bespoke pairs, `mjx-sml` checks a delegation and
//! the rebuilders behind it, and this file's primary arm — comparing a body to a canonical string —
//! exists in neither of the others and would be meaningless in `mjx-dml`, where no two bodies are
//! alike. A shared crate would hold the cheap half and leave the expensive half copied anyway.
//!
//! The one thing the duplication does cost is real and is worth naming: **a scanner improvement has
//! to be made three times.** It already happened once. MJXOFF-218's own census reported 5 `FromXml`
//! and 57 `ToXml` in `mjx-sml` where there are 6 and 58, because the scanner it used keyed on a bare
//! `impl ToXml for` and `font/color.rs` writes `impl mjx_ooxml_core::ToXml for ColorElement`.
//! MJXOFF-220 fixed that in its own file; this file uses the fixed scanner; MJXOFF-217's still had
//! the bare needle and is fixed in the same commit as this file, because a known hole in a gate is
//! not a thing to leave for a ticket.
//!
//! # This file asks what a pair **loses**, never what it **moves**
//!
//! Child order is outside every idiom here by construction, and MJXOFF-251 is what that cost: six
//! `mjx-docx` pairs re-ordered a child they never dropped, and all four ledgers stayed green.
//! `xtask/tests/child_order_census.rs` (MJXOFF-265) is where that question is answered, over every
//! function in every workspace member's `src/` rather than over impl bodies — the defect lived in a
//! free function both halves called, which is exactly what a ledger of impls cannot see.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ===============================================================================================
// The canonical body
// ===============================================================================================

/// The reader every preserving type in this crate writes out, with `{}` where the bucket field's
/// name goes.
///
/// Compared with whitespace collapsed, so `rustfmt` may re-wrap it; nothing else may differ.
const CANONICAL_READER: &str = "\
{ fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> { \
Ok(Self { name: element.name, attributes: element.attributes.clone(), \
{}: element.children.clone(), empty: element.empty, }) } }";

/// …and the writer that matches it. `empty` is `self.empty && children.is_empty()` rather than
/// `self.empty`: a bucket that has grown a child since it was read cannot be re-emitted
/// self-closing, and a bucket that has not re-emits exactly as it arrived.
const CANONICAL_WRITER: &str = "\
{ fn to_xml(&self, _interner: &mut Interner) -> RawElement { \
let children = self.{}.clone(); let empty = self.empty && children.is_empty(); \
RawElement::rebuilt(self.name, self.attributes.clone(), children, empty) } }";

/// The two names this crate spells the unknown bucket with — `extra` where the type also models
/// children, `children` where the type models none and the field *is* the content.
///
/// `CLAUDE.md`'s own "the field is spelled three ways" paragraph is what this list is a subset of;
/// the third spelling (a typed content vector with a `Raw` variant) is not a bucket a canonical
/// body can rebuild from, and every type that uses one is on [`BESPOKE`] below.
const BUCKET_FIELDS: [&str; 2] = ["extra", "children"];

// ===============================================================================================
// The ledger
// ===============================================================================================

/// How a hand-written pair that is **not** the canonical body keeps what it does not model.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Idiom {
    /// Holds the element itself — its name, its attributes and its self-closing flag — and types
    /// some of its children into a content vector with a `Raw` arm for the rest. Reads every child
    /// it was given and rebuilds from the element it was read from.
    Preserving,
    /// Splits one named properties child out of the element and hands **the rest** to a derived
    /// group type, then joins the two back on write. The children are never filtered: what is not
    /// the properties element is passed through untouched, and since MJXOFF-251 the index the
    /// properties child sat at travels with it, so the writer puts it back where the file had it
    /// rather than where `wml.xsd` would.
    PropertiesAndGroup,
    /// Constructs no element of its own in either direction — it delegates to a type that does, and
    /// so has nothing of its own to lose.
    Delegating,
}

/// One hand-written pair this crate is allowed to hold that is not the canonical body.
struct Entry {
    /// The source file, relative to `crates/mjx-docx/src/`.
    file: &'static str,
    /// The type the impls are written for.
    ty: &'static str,
    /// How it keeps what it does not model.
    idiom: Idiom,
    /// Why the canonical body does not fit. A reader who disagrees with this sentence has found a
    /// defect.
    reason: &'static str,
}

/// Every hand-written `FromXml`/`ToXml` pair in `mjx-docx` whose body is not the canonical one.
///
/// A new one fails [`every_hand_written_pair_is_the_canonical_body_or_on_the_ledger`] until somebody
/// writes down which idiom keeps its unmodelled content; a row that no longer names a pair fails
/// [`every_ledger_row_still_names_a_hand_written_pair`].
const BESPOKE: &[Entry] = &[
    Entry {
        file: "document/sections.rs",
        ty: "PageBorder",
        idiom: Idiom::Delegating,
        reason: "`CT_PageBorder` extends `CT_Border` with `@id`, and a page border is read and \
                 written as the `Border` inside it — attribute bag, unknown bucket and all. The \
                 wrapper adds one accessor and no state, so there is nothing for a canonical body \
                 to rebuild from.",
    },
    Entry {
        file: "document/sections.rs",
        ty: "TopPageBorder",
        idiom: Idiom::Delegating,
        reason: "`CT_TopPageBorder` — `CT_PageBorder` plus three `r:` header attributes, which \
                 reach a caller through the same `Border` attribute bag. Delegating, exactly as \
                 `PageBorder` above.",
    },
    Entry {
        file: "document/sections.rs",
        ty: "BottomPageBorder",
        idiom: Idiom::Delegating,
        reason: "`CT_BottomPageBorder` — the footer counterpart of `TopPageBorder`, and the same \
                 delegation for the same reason.",
    },
    Entry {
        file: "document/web_settings.rs",
        ty: "Frameset",
        idiom: Idiom::Preserving,
        reason: "`CT_Frameset` is recursive — a frameset holds framesets — and its six child kinds \
                 are an unordered choice rather than a sequence, so the position of each one has to \
                 survive. The typed content vector with a `Raw` arm is what keeps an unmodelled \
                 child exactly where the file put it, which a separated `extra` bucket cannot.",
    },
    Entry {
        file: "document/web_settings.rs",
        ty: "Div",
        idiom: Idiom::Preserving,
        reason: "`CT_Div`'s seven children are the HTML-division margins and borders a saved-as-web \
                 document carries, in the order the producer wrote them. Same typed content vector, \
                 same `Raw` arm, same reason as `Frameset` beside it.",
    },
    Entry {
        file: "document/font_table.rs",
        ty: "Font",
        idiom: Idiom::Preserving,
        reason: "`CT_Font` holds eleven optional children, four of which are the embedded-font \
                 relationship references a font table exists to carry. Their order is the \
                 producer's, and an unrecognised child between two of them keeps its place through \
                 the `Raw` arm rather than being swept to the end of a bucket.",
    },
    Entry {
        file: "document/structured_content.rs",
        ty: "Placeholder",
        idiom: Idiom::Preserving,
        reason: "`CT_Placeholder` declares exactly one child, `w:docPart`, so this type names it \
                 rather than building a content vocabulary for a vocabulary of one — and keeps \
                 every other child in an `extra` bucket beside it. The writer puts the named child \
                 back at the index the reader found it at, through the same shared worker the five \
                 `PropertiesAndGroup` rows below use — MJXOFF-251, which is why an `extra` bucket \
                 is enough here and a typed content vector is not needed.",
    },
    Entry {
        file: "document/structured_content.rs",
        ty: "CustomXmlBlock",
        idiom: Idiom::PropertiesAndGroup,
        reason: "`CT_CustomXmlBlock` is `w:customXmlPr?` followed by `EG_ContentBlockContent`, and \
                 that group is a derived type of its own. Reading is therefore: take the properties \
                 element out, hand every remaining child to the group's derive, and keep the \
                 element's own name, attributes and self-closing flag beside the two.",
    },
    Entry {
        file: "document/structured_content.rs",
        ty: "CustomXmlRun",
        idiom: Idiom::PropertiesAndGroup,
        reason: "`CT_CustomXmlRun` — the run-level member of the same family, over \
                 `EG_ContentRunContent` instead of the block group. Identical shape, identical \
                 reason.",
    },
    Entry {
        file: "document/structured_content.rs",
        ty: "CustomXmlRow",
        idiom: Idiom::PropertiesAndGroup,
        reason: "`CT_CustomXmlRow` — the table-row member of the same family, over \
                 `EG_ContentRowContent`. Identical shape, identical reason.",
    },
    Entry {
        file: "document/structured_content.rs",
        ty: "CustomXmlCell",
        idiom: Idiom::PropertiesAndGroup,
        reason: "`CT_CustomXmlCell` — the table-cell member of the same family, over \
                 `EG_ContentCellContent`. Identical shape, identical reason.",
    },
    Entry {
        file: "document/structured_content.rs",
        ty: "SmartTagRun",
        idiom: Idiom::PropertiesAndGroup,
        reason: "`CT_SmartTagRun` is the same shape with a different properties element \
                 (`w:smartTagPr`) and two more attributes: a smart tag is a custom-XML wrapper by \
                 another name, and shares the split-and-rejoin worker with the four above.",
    },
];

// ===============================================================================================
// The scanner
// ===============================================================================================

/// `crates/mjx-docx/src`.
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
        sources.len() > 20,
        "only {} source files found under {} — the walk has stopped walking",
        sources.len(),
        root.display()
    );
    sources
}

/// The `{ … }` block beginning at or after `from`, brace-balanced.
///
/// Adequate for this crate's sources, which hold no `{` inside a string or character literal in any
/// of these impl bodies; a body that grew one would over-run and fail loudly rather than pass
/// quietly, which is the right way round for a gate.
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
/// Only impls at **column zero**, which is where a free-standing impl sits and where neither the
/// derive's generated code (there is none in the source tree) nor an impl indented inside a
/// `macro_rules!` body can be found.
///
/// The trait may be written qualified — `impl mjx_ooxml_core::ToXml for T` — and a scanner keyed on
/// a bare `impl ToXml for` misses it, which is what MJXOFF-218's own census did in `mjx-sml`. This
/// is MJXOFF-220's scanner, unchanged, for that reason.
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

/// `text` with every run of whitespace collapsed to one space, for comparing a body to a shape.
fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The canonical reader and writer for a bucket named `bucket`.
fn canonical_pair(bucket: &str) -> (String, String) {
    (
        CANONICAL_READER.replace("{}", bucket),
        CANONICAL_WRITER.replace("{}", bucket),
    )
}

/// Every `RawElement::rebuilt(` call in `body`, each with the text that follows it, collapsed.
///
/// A rebuild is where this crate can meet MJXOFF-216's mistake directly: `Picture::to_xml`
/// synthesised its element's name and handed a fresh `Vec::new()` for the attributes, destroying a
/// foreign attribute, a foreign child and every `xmlns` declaration on the element.
fn rebuild_calls(body: &str) -> Vec<String> {
    const CALL: &str = "RawElement::rebuilt(";
    let collapsed = collapsed(body);
    collapsed
        .match_indices(CALL)
        .map(|(at, _)| {
            let from = at + CALL.len();
            let to = (from + 40).min(collapsed.len());
            collapsed[from..to].to_owned()
        })
        .collect()
}

// ===============================================================================================
// The anti-vacuity floors
// ===============================================================================================

/// The impl scanner must still be finding hand-written impls at all. Stated as *the scanner is
/// still matching*, never as *the crate holds exactly this many* — a floor pinned to the exact size
/// fires before the assertion it guards and hides the mutation that was meant to prove it.
const MINIMUM_HAND_WRITTEN_IMPLS: usize = 100;

/// …and the body comparison must still be matching bodies, which is this file's primary arm.
///
/// It guards the **silent** vacuity, not the loud one: a stale canonical string pushes every type
/// into the unexplained list and fails by name, but a scanner that finds no impls at all leaves that
/// list empty and would otherwise pass. Stated as *stopped matching* rather than as the exact count,
/// so a genuine mistyped copy fails by name here rather than being pre-empted by an off-by-one
/// floor.
const MINIMUM_CANONICAL_PAIRS: usize = 100;

/// The crate must still be seen to reach the derive, which reads the bulk of it and needs no ledger
/// at all: `mjx-derive`'s codegen emits the `Raw` fallthrough unconditionally and
/// `crates/mjx-derive/tests/derive.rs` backs every derived type at once.
const MINIMUM_DERIVED_TYPES: usize = 80;

// ===============================================================================================
// The tests
// ===============================================================================================

/// The scanner is alive, and the mechanism that needs no ledger is still the bulk of the crate.
#[test]
fn the_scanner_still_sees_both_serialization_mechanisms() {
    let hand_written = hand_written_impls();
    let mut derived = 0usize;
    for (_, text) in sources() {
        derived += text
            .split("#[derive(")
            .skip(1)
            .filter(|tail| {
                tail.find(")]")
                    .is_some_and(|end| tail[..end].contains("FromXml"))
            })
            .count();
    }
    let readers = hand_written
        .iter()
        .filter(|impl_| impl_.trait_name == "FromXml")
        .count();
    let types: BTreeSet<(&str, &str)> = hand_written
        .iter()
        .map(|impl_| (impl_.file.as_str(), impl_.ty.as_str()))
        .collect();
    println!(
        "mjx-docx serialization: {derived} `#[derive(FromXml)]`, {} hand-written impls over {} \
         types ({readers} FromXml, {} ToXml)",
        hand_written.len(),
        types.len(),
        hand_written.len() - readers,
    );
    assert!(
        derived >= MINIMUM_DERIVED_TYPES,
        "only {derived} `#[derive(FromXml)]` sites found — the derive scanner has stopped matching"
    );
    assert!(
        hand_written.len() >= MINIMUM_HAND_WRITTEN_IMPLS,
        "only {} hand-written impls found — the impl scanner has stopped matching",
        hand_written.len()
    );
}

/// Every hand-written impl in this crate comes in a pair — a reader and a writer for the same type.
///
/// The two directions are checked together below, and a type with only one of them would be checked
/// against half a shape. It would also be a new idiom this crate does not have: `mjx-dml` holds
/// read-only projections and `mjx-sml` holds `ToXml`-only writers, and neither shape exists here.
#[test]
fn every_hand_written_impl_is_half_of_a_pair() {
    let mut directions: BTreeMap<(String, String), BTreeSet<&'static str>> = BTreeMap::new();
    for impl_ in hand_written_impls() {
        directions
            .entry((impl_.file, impl_.ty))
            .or_default()
            .insert(impl_.trait_name);
    }
    let unpaired: Vec<String> = directions
        .iter()
        .filter(|(_, traits)| traits.len() != 2)
        .map(|((file, ty), traits)| {
            format!(
                "{file}: {ty} has only {:?}",
                traits.iter().collect::<Vec<_>>()
            )
        })
        .collect();
    assert!(
        unpaired.is_empty(),
        "hand-written impls with no counterpart in the other direction:\n  {}\n\nA read-only \
         projection or a write-only wrapper is an idiom this crate does not have; adding one means \
         adding an arm to this file, not a row to BESPOKE.",
        unpaired.join("\n  ")
    );
    assert!(
        directions.len() >= MINIMUM_CANONICAL_PAIRS,
        "only {} hand-written pairs found — the pairing scan has stopped matching",
        directions.len()
    );
}

/// **The primary arm.** Every hand-written pair is the canonical body character for character, in
/// both directions and over the same bucket field, or is on [`BESPOKE`] with a reason.
///
/// This is what a list of a hundred and forty-six names could not do. `Control` failed here as it
/// stood — its writer read `RawElement::rebuilt(self.name, self.attributes.clone(), Vec::new(),
/// true)`, which is not the canonical text and would have had to be explained; so did
/// `WordprocessingShape` and `TextboxInfo`, whose writers ended `self.children.clone(), false`.
#[test]
fn every_hand_written_pair_is_the_canonical_body_or_on_the_ledger() {
    let ledger: BTreeSet<(&str, &str)> =
        BESPOKE.iter().map(|entry| (entry.file, entry.ty)).collect();
    let mut by_type: BTreeMap<(String, String), BTreeMap<&'static str, String>> = BTreeMap::new();
    for impl_ in hand_written_impls() {
        by_type
            .entry((impl_.file, impl_.ty))
            .or_default()
            .insert(impl_.trait_name, collapsed(&impl_.body));
    }

    let mut canonical = 0usize;
    let mut per_bucket: BTreeMap<&str, usize> = BTreeMap::new();
    let mut unexplained = Vec::new();
    for ((file, ty), bodies) in &by_type {
        let (Some(reader), Some(writer)) = (bodies.get("FromXml"), bodies.get("ToXml")) else {
            continue; // `every_hand_written_impl_is_half_of_a_pair` owns this failure.
        };
        let matched = BUCKET_FIELDS.iter().find(|bucket| {
            let (expected_reader, expected_writer) = canonical_pair(bucket);
            *reader == expected_reader && *writer == expected_writer
        });
        match matched {
            Some(bucket) => {
                canonical += 1;
                *per_bucket.entry(bucket).or_default() += 1;
            }
            None if ledger.contains(&(file.as_str(), ty.as_str())) => {}
            None => {
                // Say which half diverged, so a mistyped copy names itself rather than sending a
                // reader to diff two hundred-character strings by eye.
                let halves: Vec<&str> = BUCKET_FIELDS
                    .iter()
                    .flat_map(|bucket| {
                        let (expected_reader, expected_writer) = canonical_pair(bucket);
                        [
                            (*reader == expected_reader).then_some("reader"),
                            (*writer == expected_writer).then_some("writer"),
                        ]
                    })
                    .flatten()
                    .collect();
                unexplained.push(format!(
                    "{file}: {ty} (canonical halves: {})\n      reader: {reader}\n      writer: \
                     {writer}",
                    if halves.is_empty() {
                        "neither".to_owned()
                    } else {
                        halves.join(" + ")
                    },
                ));
            }
        }
    }

    assert!(
        unexplained.is_empty(),
        "hand-written pairs that are neither the canonical body nor on this file's BESPOKE \
         ledger:\n  {}\n\nThis crate copies one reader and one writer out by hand for every \
         preserving type, and a copy that differs is outside every gate the workspace has: the \
         derive tests do not apply, the per-type suites name a subset, and the preservation gate is \
         per-fixture over a corpus in which these elements are canonical, so a fresh `Vec::new()` \
         reproduces an already-empty vector and the diff shows nothing. That is exactly how \
         MJXOFF-216 hid. Make it the canonical body, or add a BESPOKE row saying which idiom keeps \
         what it does not model.",
        unexplained.join("\n  ")
    );

    println!(
        "canonical pairs: {canonical} ({per_bucket:?}), bespoke: {}",
        ledger.len()
    );
    assert!(
        canonical >= MINIMUM_CANONICAL_PAIRS,
        "only {canonical} pairs matched the canonical body — the body comparison has stopped \
         matching, and it is this file's primary arm"
    );
}

/// The ledger's *claims* hold, which is what makes it more than a list.
#[test]
fn every_bespoke_row_matches_the_shape_of_the_impl_it_names() {
    let impls = hand_written_impls();
    let mut checked = 0usize;
    for entry in BESPOKE {
        for impl_ in impls
            .iter()
            .filter(|impl_| impl_.file == entry.file && impl_.ty == entry.ty)
        {
            checked += 1;
            let where_ = format!("{}: impl {} for {}", entry.file, impl_.trait_name, entry.ty);
            match entry.idiom {
                Idiom::Delegating => {
                    assert!(
                        !impl_.body.contains("RawElement::rebuilt")
                            && !impl_.body.contains("RawElement::new"),
                        "{where_} is on the ledger as Delegating but constructs an element of its \
                         own, so it has something to lose and needs a different idiom"
                    );
                }
                Idiom::Preserving | Idiom::PropertiesAndGroup => {
                    if impl_.trait_name == "FromXml" {
                        for marker in ["element.name", "element.attributes", "element.empty"] {
                            assert!(
                                impl_.body.contains(marker),
                                "{where_} is on the ledger as {:?} but never reads `{marker}` — a \
                                 reader that does not keep the element it was given cannot hand a \
                                 writer the element back",
                                entry.idiom
                            );
                        }
                        if entry.idiom == Idiom::Preserving {
                            assert!(
                                impl_.body.contains("element.children"),
                                "{where_} is on the ledger as Preserving but never looks at \
                                 `element.children` — that is `Control`'s defect exactly, a whole \
                                 element kept except for its content"
                            );
                        } else {
                            assert!(
                                impl_.body.contains("split_positioned_child"),
                                "{where_} is on the ledger as PropertiesAndGroup but does not \
                                 split its properties child out through the shared worker, so \
                                 nothing here says what happens to the children it did not name"
                            );
                        }
                    } else {
                        assert!(
                            impl_.body.contains("self.empty"),
                            "{where_} is on the ledger as {:?} but ignores `self.empty` — that \
                             re-emits `<x/>` as `<x></x>`, which is what `WordprocessingShape` and \
                             `TextboxInfo` did here and `mjx_dml::Inline` did in MJXOFF-217",
                            entry.idiom
                        );
                        let calls = rebuild_calls(&impl_.body);
                        assert!(
                            !calls.is_empty(),
                            "{where_} is on the ledger as {:?} but rebuilds no element at all — \
                             then it is Delegating, and the row is wrong",
                            entry.idiom
                        );
                        for call in calls {
                            assert!(
                                call.starts_with("self.name, self.attributes"),
                                "{where_} hands `RawElement::rebuilt` `{call}…` rather than the \
                                 element's own name and attributes — MJXOFF-216's exact shape"
                            );
                        }
                        if entry.idiom == Idiom::PropertiesAndGroup {
                            assert!(
                                impl_.body.contains("join_positioned_child"),
                                "{where_} is on the ledger as PropertiesAndGroup but does not join \
                                 its properties back through the shared worker"
                            );
                        }
                    }
                }
            }
        }
    }
    println!(
        "bespoke: {checked} impls checked against {} rows",
        BESPOKE.len()
    );
    assert_eq!(
        checked,
        BESPOKE.len() * 2,
        "each BESPOKE row names one reader and one writer — the matcher has stopped matching"
    );
}

/// …and in the other direction: a row that stopped naming anything is a row nobody is reading.
#[test]
fn every_ledger_row_still_names_a_hand_written_pair() {
    let found: BTreeSet<(String, String)> = hand_written_impls()
        .into_iter()
        .map(|impl_| (impl_.file, impl_.ty))
        .collect();
    let stale: Vec<String> = BESPOKE
        .iter()
        .filter(|entry| !found.contains(&(entry.file.to_owned(), entry.ty.to_owned())))
        .map(|entry| format!("{}: {}", entry.file, entry.ty))
        .collect();
    assert!(
        stale.is_empty(),
        "BESPOKE rows naming no hand-written impl (moved to the derive, renamed or deleted):\n  {}",
        stale.join("\n  ")
    );

    let rows: BTreeSet<(&str, &str)> = BESPOKE.iter().map(|entry| (entry.file, entry.ty)).collect();
    assert_eq!(
        rows.len(),
        BESPOKE.len(),
        "BESPOKE names the same pair twice — two reasons for one impl means one of them is not kept \
         true"
    );

    for entry in BESPOKE {
        assert!(
            entry.reason.len() > 60,
            "{}: {} — a ledger reason has to say something",
            entry.file,
            entry.ty
        );
    }
    println!("ledger: {} bespoke pairs, all still present", BESPOKE.len());
}
