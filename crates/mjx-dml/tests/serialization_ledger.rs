//! **Every hand-written `FromXml`/`ToXml` impl in this crate is on a ledger, with a reason and a
//! shape the ledger can check** (MJXOFF-217, closing MJXOFF-216's fourth clause).
//!
//! # Why this file exists
//!
//! Unknown-child and unknown-attribute preservation is guaranteed three ways in this workspace, and
//! two of them are guaranteed *once*:
//!
//! * **`#[derive(FromXml, ToXml)]`** — `mjx-derive`'s codegen emits the `Raw` fallthrough
//!   unconditionally, with no way to invoke the arm without it, so
//!   `crates/mjx-derive/tests/derive.rs` backs every derived type at once.
//! * **`crate::build::fidelity_element_impls!`** — one macro body storing `name`, `attributes`,
//!   `children` and `empty` verbatim, so one reading of it backs every type that invokes it.
//! * **A hand-written pair** — backed by nothing but the care of whoever wrote it.
//!
//! MJXOFF-216 is what the third kind costs. `Picture` and `PictureNonVisual` read three children by
//! local name, discarded every other one, and rebuilt with a synthesised element name and
//! `Vec::new()` for the attributes — destroying a foreign attribute, a foreign child and every
//! `xmlns` declaration on the element. It passed all three gates at once: the derive tests did not
//! apply, `in_context_roundtrip.rs` did not name the type, and the preservation gate is per-fixture
//! over a corpus in which every `pic:pic` is canonical, so `Vec::new()` reproduced an already-empty
//! vector and the diff showed nothing.
//!
//! MJXOFF-217 counted the class: of the **eight** hand-written pairs this crate held at 0.0.137,
//! four lost content and two more lost the self-closing flag. All six are fixed. This file is what
//! stops the seventh arriving unnoticed — because the next one will arrive looking like a feature.
//!
//! # What it checks, and why it is not merely a list
//!
//! A ledger of names would pass forever once written. So each entry declares an **idiom**, and the
//! idiom is checked against the impl's own source:
//!
//! * [`Idiom::FullyPreserving`] — the body must read `self.name` and `self.attributes` and must not
//!   construct a fresh attribute vector. That is exactly the shape MJXOFF-216 violated, so this arm
//!   would have failed on `Picture::to_xml` as it stood.
//! * [`Idiom::Dispatcher`] — the body must not build a `RawElement` at all: it delegates to a type
//!   that does, and adds nothing of its own to lose.
//! * [`Idiom::ReadOnlyProjection`] — the type must have **no `ToXml` impl anywhere in the crate**.
//!   Nothing is ever written back through it, so nothing can be lost; the entry is checked to still
//!   be true rather than trusted.
//!
//! # Scope, stated so the hole is deliberate rather than silent
//!
//! **This gate covers `mjx-dml` only.** `mjx-docx` writes the fully-preserving body out by hand for
//! 158 types and `mjx-sml` for 57 more, and classifying those is a unit of its own rather than a
//! paragraph of this one; it is tracked separately. Extending this file to them means widening
//! [`crate_root`] and growing [`LEDGER`], not rewriting anything here.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

// ===============================================================================================
// The ledger
// ===============================================================================================

/// How a hand-written impl keeps what it does not model.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Idiom {
    /// Stores the element's own `name`, `attributes`, `children` and `empty`, and rebuilds from
    /// them. Preserves strictly more than an `extra` bucket, because nothing was separated out.
    FullyPreserving,
    /// Delegates to another type's impl and constructs no element of its own.
    Dispatcher,
    /// `FromXml` with no `ToXml` at all — a view over an element, never a wrapper around one.
    ReadOnlyProjection,
}

/// One hand-written `FromXml`/`ToXml` impl this crate is allowed to hold.
struct Entry {
    /// The source file, relative to `crates/mjx-dml/src/`. Two different types are named `Anchor`,
    /// so the file is part of the key.
    file: &'static str,
    /// The type the impl is written for.
    ty: &'static str,
    /// How it keeps what it does not model.
    idiom: Idiom,
    /// Why it is not on the derive. A reader who disagrees with this sentence has found a defect.
    reason: &'static str,
}

/// Every hand-written `FromXml`/`ToXml` impl in `mjx-dml`, at 0.0.138.
///
/// Adding an impl without adding a row here fails
/// [`every_hand_written_impl_is_on_the_ledger`]; a row that no longer names one fails
/// [`every_ledger_row_still_names_a_hand_written_impl`].
const LEDGER: &[Entry] = &[
    Entry {
        file: "theme.rs",
        ty: "ColorScheme",
        idiom: Idiom::ReadOnlyProjection,
        reason:
            "A view over `a:clrScheme` that keeps only slot→colour pairs. The theme is authored \
                 as a whole document by `default_theme_xml`, never serialized from this view.",
    },
    Entry {
        file: "theme.rs",
        ty: "Theme",
        idiom: Idiom::ReadOnlyProjection,
        reason: "A view over `a:theme` that keeps the colour scheme, the font scheme and the two \
                 style matrices and drops the rest, because resolving a fill never needs them. \
                 Serializing it would emit a theme with holes in it — see `default_theme_xml`.",
    },
    Entry {
        file: "style.rs",
        ty: "StyleMatrixReference",
        idiom: Idiom::ReadOnlyProjection,
        reason:
            "A projection of two facts out of an `a:fillRef`/`a:lnRef`/`a:effectRef`: the index \
                 and the substituted colour. It does not retain the element's attributes and says \
                 so on itself.",
    },
    Entry {
        file: "wordprocessing_drawing.rs",
        ty: "Inline",
        idiom: Idiom::FullyPreserving,
        reason: "`wp:inline`'s children are read on demand by nine accessors rather than typed \
                 into a content vector, because `mjx-docx` parses its own payload out of them. \
                 Holding the raw list is what makes a `pic:pic` inside a `w:drawing` survive.",
    },
    Entry {
        file: "wordprocessing_drawing.rs",
        ty: "Anchor",
        idiom: Idiom::FullyPreserving,
        reason: "`wp:anchor`, for the same reason as `Inline` beside it.",
    },
    Entry {
        file: "fill.rs",
        ty: "Fill",
        idiom: Idiom::Dispatcher,
        reason: "The `EG_FillProperties` choice: the element's local name is the discriminant, \
                 which the container derive does not model. Each of the six variants is a fidelity \
                 wrapper, and an unrecognised local name lands in `Group` rather than erroring.",
    },
    Entry {
        file: "geometry/guide.rs",
        ty: "GeometryGuide",
        idiom: Idiom::FullyPreserving,
        reason: "`a:gd` is an attribute-only leaf: the derive models element children, and this \
                 type has none to model. It still keeps any unexpected child it meets.",
    },
    Entry {
        file: "spreadsheet_drawing.rs",
        ty: "AnchoredObject",
        idiom: Idiom::Dispatcher,
        reason: "The six things an anchor can hold, told apart by element name. Reading is \
                 `AnchoredObject::read`, not `FromXml`, because a name that is none of the six is \
                 `None` rather than an error.",
    },
    Entry {
        file: "spreadsheet_drawing.rs",
        ty: "Anchor",
        idiom: Idiom::Dispatcher,
        reason: "The three `EG_Anchor` element kinds, told apart by element name, exactly as \
                 `AnchoredObject` above.",
    },
];

// ===============================================================================================
// Reading the crate's own source
// ===============================================================================================

/// `crates/mjx-dml/src`.
fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.rs` file under [`crate_root`], with its path relative to that root.
fn sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
        let entries = std::fs::read_dir(dir).expect("mjx-dml's own source tree is readable");
        for entry in entries {
            let path = entry.expect("a readable directory entry").path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let relative = path
                    .strip_prefix(root)
                    .expect("a path under the root")
                    .to_string_lossy()
                    .replace('\\', "/");
                let text = std::fs::read_to_string(&path).expect("a readable source file");
                out.push((relative, text));
            }
        }
    }
    let root = crate_root();
    let mut out = Vec::new();
    walk(&root, &root, &mut out);
    out.sort();
    out
}

/// One hand-written impl, with the source of its body.
struct HandWritten {
    file: String,
    trait_name: &'static str,
    ty: String,
    body: String,
}

/// The `{ … }` block that starts at the first `{` at or after `from`, brace-matched.
///
/// Adequate for this crate's sources, which contain no `{` inside a string or character literal in
/// any of these impl bodies; a body that grew one would over-run and fail loudly rather than pass
/// quietly, which is the right way round for a gate.
fn balanced_block(text: &str, from: usize) -> String {
    let bytes = text.as_bytes();
    let mut index = from;
    while index < bytes.len() && bytes[index] != b'{' {
        index += 1;
    }
    let start = index;
    let mut depth = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return text[start..=index].to_owned();
                }
            }
            _ => {}
        }
        index += 1;
    }
    panic!("unbalanced braces after byte {from} — the impl-body scanner has stopped matching");
}

/// Every hand-written `impl FromXml for T` / `impl ToXml for T` in this crate's sources.
///
/// Matched at the start of a line, which is where a free-standing impl sits and where neither the
/// derive's generated code (there is none in the source tree) nor `fidelity_element_impls!`'s body
/// (indented inside the macro, and fully qualified as `::mjx_ooxml_core::FromXml`) can be found.
fn hand_written_impls() -> Vec<HandWritten> {
    let mut found = Vec::new();
    for (file, text) in sources() {
        for trait_name in ["FromXml", "ToXml"] {
            let needle = format!("\nimpl {trait_name} for ");
            let mut cursor = 0usize;
            while let Some(offset) = text[cursor..].find(&needle) {
                let at = cursor + offset + needle.len();
                let ty: String = text[at..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                assert!(
                    !ty.is_empty(),
                    "{file}: `impl {trait_name} for` with no type name after it"
                );
                found.push(HandWritten {
                    file: file.clone(),
                    trait_name,
                    body: balanced_block(&text, at),
                    ty,
                });
                cursor = at;
            }
        }
    }
    found
}

// ===============================================================================================
// The anti-vacuity floors
// ===============================================================================================

/// The extractor must still be finding hand-written impls at all. Stated as *the scanner is still
/// matching*, never as *the crate holds exactly this many* — a floor pinned to the exact size fires
/// before the assertion it guards and hides the mutation that was meant to prove it.
const MINIMUM_HAND_WRITTEN_IMPLS: usize = 8;

/// The crate must still be seen to reach the derive. If this walk lost the crate, every assertion
/// below would pass over an empty set.
const MINIMUM_DERIVED_TYPES: usize = 25;

/// …and the shared fidelity macro.
const MINIMUM_MACRO_TYPES: usize = 40;

// ===============================================================================================
// The tests
// ===============================================================================================

/// The scanner is alive, and the two mechanisms that need no ledger are still the bulk of the crate.
#[test]
fn the_scanner_still_sees_all_three_serialization_mechanisms() {
    let hand_written = hand_written_impls();
    let mut derived = 0usize;
    let mut macro_invoked = 0usize;
    for (_, text) in sources() {
        // A `#[derive(..)]` list naming `FromXml`, however it was imported — most of this crate
        // writes `use mjx_derive::{FromXml, ToXml}` and then a bare `FromXml` in the list.
        derived += text
            .split("#[derive(")
            .skip(1)
            .filter(|tail| {
                tail.find(")]")
                    .is_some_and(|end| tail[..end].contains("FromXml"))
            })
            .count();
        macro_invoked += text
            .matches("fidelity_element_impls!(")
            .count()
            .saturating_sub(text.matches("fidelity_element_impls!($ty)").count());
    }
    println!(
        "mjx-dml serialization: {} derived, {macro_invoked} via fidelity_element_impls!, \
         {} hand-written impls over {} types",
        derived,
        hand_written.len(),
        hand_written
            .iter()
            .map(|impl_| (impl_.file.as_str(), impl_.ty.as_str()))
            .collect::<BTreeSet<_>>()
            .len(),
    );
    assert!(
        derived >= MINIMUM_DERIVED_TYPES,
        "only {derived} `#[derive(FromXml)]` sites found — the derive scanner has stopped matching"
    );
    assert!(
        macro_invoked >= MINIMUM_MACRO_TYPES,
        "only {macro_invoked} `fidelity_element_impls!` sites found — that scanner has stopped \
         matching"
    );
    assert!(
        hand_written.len() >= MINIMUM_HAND_WRITTEN_IMPLS,
        "only {} hand-written impls found — the impl scanner has stopped matching",
        hand_written.len()
    );
}

/// Every hand-written impl is on the ledger. **A new one fails here until somebody writes down why
/// it is not on the derive.**
#[test]
fn every_hand_written_impl_is_on_the_ledger() {
    let ledger: BTreeSet<(&str, &str)> = LEDGER.iter().map(|e| (e.file, e.ty)).collect();
    let mut unlisted = Vec::new();
    for impl_ in hand_written_impls() {
        if !ledger.contains(&(impl_.file.as_str(), impl_.ty.as_str())) {
            unlisted.push(format!(
                "{}: impl {} for {}",
                impl_.file, impl_.trait_name, impl_.ty
            ));
        }
    }
    assert!(
        unlisted.is_empty(),
        "hand-written `FromXml`/`ToXml` impls that are not on this file's LEDGER:\n  {}\n\n\
         A hand-written pair sits outside `mjx-derive`'s codegen guarantee by definition, which is \
         how MJXOFF-216 hid from all three gates at once. Put it on the derive, or add a row \
         saying which idiom keeps what it does not model and why the derive does not fit.",
        unlisted.join("\n  ")
    );
}

/// …and in the other direction: a row that stopped naming anything is a row nobody is reading.
#[test]
fn every_ledger_row_still_names_a_hand_written_impl() {
    let found: BTreeSet<(String, String)> = hand_written_impls()
        .into_iter()
        .map(|impl_| (impl_.file, impl_.ty))
        .collect();
    let stale: Vec<String> = LEDGER
        .iter()
        .filter(|entry| !found.contains(&(entry.file.to_owned(), entry.ty.to_owned())))
        .map(|entry| format!("{}: {}", entry.file, entry.ty))
        .collect();
    assert!(
        stale.is_empty(),
        "LEDGER rows naming no hand-written impl (moved to the derive, renamed or deleted):\n  {}",
        stale.join("\n  ")
    );

    for entry in LEDGER {
        assert!(
            entry.reason.len() > 40,
            "{}: {} — a ledger reason has to say something",
            entry.file,
            entry.ty
        );
    }
}

/// The ledger's *claims* hold, which is what makes it more than a list.
///
/// This is the arm that would have failed on `Picture::to_xml` as MJXOFF-216 found it: a
/// [`Idiom::FullyPreserving`] writer that synthesises its element name and hands
/// `RawElement::rebuilt` a fresh `Vec::new()` is exactly what "fully preserving" is not.
#[test]
fn every_ledger_row_matches_the_shape_of_the_impl_it_names() {
    let impls = hand_written_impls();
    let mut checked = 0usize;
    for entry in LEDGER {
        for impl_ in impls
            .iter()
            .filter(|i| i.file == entry.file && i.ty == entry.ty)
        {
            checked += 1;
            let where_ = format!("{}: impl {} for {}", entry.file, impl_.trait_name, entry.ty);
            match entry.idiom {
                Idiom::FullyPreserving => {
                    assert!(
                        impl_.body.contains("self.name") || impl_.body.contains("element.name"),
                        "{where_} is on the ledger as FullyPreserving but never touches the \
                         element's own name"
                    );
                    assert!(
                        impl_.body.contains("attributes"),
                        "{where_} is on the ledger as FullyPreserving but never touches the \
                         attribute vector"
                    );
                    assert!(
                        !impl_.body.contains("Vec::new()"),
                        "{where_} is on the ledger as FullyPreserving but builds a fresh vector — \
                         that is MJXOFF-216's exact shape"
                    );
                }
                Idiom::Dispatcher => assert!(
                    !impl_.body.contains("RawElement::rebuilt")
                        && !impl_.body.contains("RawElement::new"),
                    "{where_} is on the ledger as a Dispatcher but constructs an element of its \
                     own, so it has something to lose and needs a different idiom"
                ),
                Idiom::ReadOnlyProjection => {
                    assert_eq!(
                        impl_.trait_name, "FromXml",
                        "{where_} is on the ledger as a ReadOnlyProjection, but here is a ToXml \
                         for it — something is now written back through a view that keeps only \
                         part of its element"
                    );
                }
            }
        }
    }
    println!(
        "serialization ledger: {checked} impls checked against {} rows",
        LEDGER.len()
    );
    assert!(
        checked >= MINIMUM_HAND_WRITTEN_IMPLS,
        "only {checked} impls were matched to a ledger row — the matcher has stopped matching"
    );
}
