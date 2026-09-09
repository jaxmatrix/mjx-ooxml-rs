//! The facade's curation ledger: **what `mjx-ooxml` leaves behind is a list, not a number**
//! (MJXOFF-214).
//!
//! `mjx_ooxml::Deck`, `mjx_ooxml::Document` and `mjx_ooxml::Workbook` are each a *curated* view of
//! one crate below them. Each of the three module doc comments says which methods stay behind and
//! why — and until this file existed, each also said **how many**, in prose, from a count taken once
//! and never taken again. All three had rotted:
//!
//! | claim | where | measured at 0.0.136 |
//! |---|---|---|
//! | "Sixteen of the surface's methods are deliberately absent" | `crates/mjx-ooxml/src/deck.rs` | **18** |
//! | "`mjx_xlsx::Workbook` carries roughly seventy public methods" | `crates/mjx-ooxml/src/workbook.rs` | **165** |
//! | "sixty-five variants collapse to eleven codes" | `crates/mjx-ooxml/src/deck.rs` | **67** |
//! | "thirty-five variants collapse to eleven codes" | `crates/mjx-ooxml/src/document.rs` | **41** |
//!
//! That is MJXOFF-198 §4's *"every count is a fact that expires"* in its purest form, and the cure
//! is not a fresher number. A number in prose can only ever be right on the day it is written; a
//! **list** can be compared. So the numbers came out of the prose and the list went into this file,
//! where a method added to `mjx_pptx::Presentation`, `mjx_docx::Document` or `mjx_xlsx::Workbook`
//! and not projected onto the facade fails the build until somebody decides which it is: projected,
//! renamed, or deliberately left behind with a reason.
//!
//! # The trap this file is written against
//!
//! > *A ledger that lists what is absent is green precisely when the extractor stops finding
//! > anything.*
//!
//! An impl-block walk that matched nothing would make every surface empty, every difference empty,
//! and every assertion below trivially true. Three things are done about it:
//!
//! 1. **Both directions are asserted.** The difference must equal the ledger *and* the ledger must
//!    name nothing that is not in the difference — so a stale entry fails as loudly as a missing
//!    one.
//! 2. **Every surface carries an anti-vacuity floor**, stated as *the walk is still matching*
//!    rather than as *this surface is exactly this size*, the shape `doc_gate.rs` and
//!    `validation_index.rs` already settled on. A floor pinned to the exact count fires before the
//!    assertion it guards.
//! 3. **Every count is printed on success**, not only on failure, because a printed count is what
//!    distinguishes "ran" from "skipped quietly".
//!
//! # What is compared, and what is not
//!
//! Compared: the **names** of inherent `pub fn` items in `impl <Type> {` blocks, on both sides.
//! That is what a caller writes and what both bindings project, and it is what goes stale when a
//! crate below grows a method nobody carried up.
//!
//! Not compared, each for a stated reason:
//!
//! * **Signatures.** The facade's whole purpose is to change them — `usize` to `u32`, `impl
//!   Into<Surface>` to [`Surface`], a closure to a concrete return. `crates/mjx-ooxml/tests/
//!   delegate_wiring.rs` is what proves a delegate calls the method it is named after.
//! * **Anything behind a generic or a trait impl.** `impl<T> Foo for Bar` declares no inherent
//!   method a caller reaches by that name.
//! * **Methods the facade adds.** A facade-only method (`Deck::presentation_mut`, `Deck::format`)
//!   is not a curation decision about the crate below; it is this crate's own surface, already
//!   covered by `crates/mjx-ooxml/tests/public_paths.rs`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The workspace root — `xtask/`'s parent.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// Why a method of the crate below is not on the facade under that name.
///
/// The variants are the reasons the three module doc comments give, plus the two the audit found
/// unaccounted for. A new entry has to pick one, which is the point: "it is not there" is not a
/// reason, and a ledger that accepted it would be a list of everything with nothing said about it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Reason {
    /// Takes a closure over an interner-bound reference — the one shape a foreign function boundary
    /// cannot carry. The facade calls it internally where a concrete answer exists.
    Closure,
    /// Hands back a value borrowing the document for a caller-controlled lifetime, which neither
    /// PyO3 nor wasm-bindgen can express.
    BorrowedView,
    /// Takes or hands back an owned but **interner-bound** model — a `mjx_sml::WorksheetPart`, a
    /// `SheetMarkup`, a `SharedStringTable`. It borrows nothing and takes no closure, so it is
    /// neither of the two above; what it cannot do is cross a foreign function boundary, because
    /// every string in it is an index into an interner that stays behind.
    ///
    /// This variant exists because the audit found the facade's own documentation filing
    /// `worksheet_markup` and `write_worksheet_markup` under *the closure-taking markup doors*,
    /// which neither of them is (MJXOFF-214).
    InternerBoundModel,
    /// Takes an `mjx-sml` or `mjx-dml` spec *tree* rather than a flat struct; projecting it is a
    /// surface of its own. What it produces is readable through the facade.
    SpecTree,
    /// Hands out part-graph identity (a `PartName`, a relationship id) for content that is already
    /// addressable by index, or whose bytes are readable directly.
    PartHandle,
    /// Takes or returns an `mjx_opc::Package`, which this facade seals — see
    /// `Deck::presentation_mut`.
    SealedPackage,
    /// Present on the facade under a different, more specific name.
    RenamedTo(&'static str),
    /// Superseded by a different call that answers the same question in a shape a binding can
    /// carry.
    SupersededBy(&'static str),
    /// A document-wide, legacy or deep-tree cluster the curated surface deliberately does not
    /// carry — equations, mail merge, bookmarks, `altChunk`, the glossary document. Named in the
    /// facade module's own documentation, and reachable in full through the escape hatch.
    OutOfCuratedScope,
    /// Genuinely absent from all three languages, with nothing on the facade that answers it.
    /// **Every one of these is a gap this audit found, not a decision anyone recorded.**
    NoFacadeEquivalent,
}

impl Reason {
    /// The sentence a failure prints beside the method, so a reader meets the reason and not just
    /// the classification.
    fn describe(self) -> String {
        match self {
            Self::Closure => "takes a closure over an interner-bound reference".to_owned(),
            Self::BorrowedView => "hands back a borrowed view of the document".to_owned(),
            Self::InternerBoundModel => {
                "takes or hands back an interner-bound model that cannot cross the boundary"
                    .to_owned()
            }
            Self::SpecTree => "takes a spec tree rather than a flat struct".to_owned(),
            Self::PartHandle => "hands out part-graph identity".to_owned(),
            Self::SealedPackage => "takes or returns the sealed `mjx_opc::Package`".to_owned(),
            Self::RenamedTo(name) => format!("on the facade as `{name}`"),
            Self::SupersededBy(name) => format!("superseded by `{name}`"),
            Self::OutOfCuratedScope => {
                "a cluster the curated surface does not carry; reachable through the escape hatch"
                    .to_owned()
            }
            Self::NoFacadeEquivalent => "no facade equivalent — a gap, not a decision".to_owned(),
        }
    }
}

/// One of the three curated surfaces.
struct Surface {
    /// `Deck`, for messages.
    facade_type: &'static str,
    /// The facade's own source directory.
    facade_source: &'static str,
    /// `mjx_pptx::Presentation`, for messages.
    model_path: &'static str,
    /// The type name the crate below declares.
    model_type: &'static str,
    /// That crate's source directory.
    model_source: &'static str,
    /// The floor on the model's own surface, stated as *the walk is still matching*.
    model_floor: usize,
    /// The floor on the facade's surface, same reading.
    facade_floor: usize,
    /// Every model method absent from the facade under that name, with why.
    ledger: &'static [(&'static str, Reason)],
}

/// `Deck` against `mjx_pptx::Presentation`.
///
/// The four groups `crates/mjx-ooxml/src/deck.rs` names, plus the two the audit found it had never
/// named: `blank_with_properties` and `from_package`.
const DECK: Surface = Surface {
    facade_type: "Deck",
    facade_source: "crates/mjx-ooxml/src/deck",
    model_path: "mjx_pptx::Presentation",
    model_type: "Presentation",
    model_source: "crates/mjx-pptx/src/presentation",
    model_floor: 150,
    facade_floor: 150,
    ledger: &[
        ("activex_control_rel_id", Reason::PartHandle),
        ("activex_snapshot_rel_id", Reason::PartHandle),
        ("blank_with_properties", Reason::NoFacadeEquivalent),
        ("chart_rel_id", Reason::PartHandle),
        ("edit_vml_drawing", Reason::Closure),
        ("from_package", Reason::SealedPackage),
        ("layout_part", Reason::PartHandle),
        ("master_part", Reason::PartHandle),
        ("ole_object_rel_id", Reason::PartHandle),
        ("ole_snapshot_rel_id", Reason::PartHandle),
        ("picture_image_rel_id", Reason::PartHandle),
        ("presentation_part", Reason::PartHandle),
        ("shape", Reason::BorrowedView),
        ("slide_part", Reason::PartHandle),
        ("with_table_style", Reason::Closure),
        ("with_vml_drawing", Reason::Closure),
        ("with_vml_shape_for_activex_control", Reason::Closure),
        ("with_vml_shape_for_ole_object", Reason::Closure),
    ],
};

/// `Document` against `mjx_docx::Document`.
///
/// The five groups `crates/mjx-ooxml/src/document.rs` names — equations; mail merge, web settings,
/// the font table and recipients; bookmarks, move ranges, custom XML and `altChunk`; legacy form
/// fields; the closure doors — plus the four the audit found it had never named: the two
/// header/footer pairs whose narrower cover already exists, `parts`, and `blank_with_properties`.
const DOCUMENT: Surface = Surface {
    facade_type: "Document",
    facade_source: "crates/mjx-ooxml/src/document",
    model_path: "mjx_docx::Document",
    model_type: "Document",
    model_source: "crates/mjx-docx/src/document",
    model_floor: 90,
    facade_floor: 90,
    ledger: &[
        ("add_alt_chunk", Reason::OutOfCuratedScope),
        ("add_bookmark", Reason::OutOfCuratedScope),
        (
            "add_chart_placed",
            Reason::SupersededBy("add_floating_chart"),
        ),
        ("alt_chunk_parts", Reason::OutOfCuratedScope),
        ("alt_chunk_payload", Reason::OutOfCuratedScope),
        ("append_math", Reason::OutOfCuratedScope),
        ("blank_with_properties", Reason::NoFacadeEquivalent),
        ("comment_range", Reason::SupersededBy("comment_range_text")),
        ("create_footer", Reason::SupersededBy("set_footer_text")),
        ("create_header", Reason::SupersededBy("set_header_text")),
        ("custom_xml_del_range", Reason::OutOfCuratedScope),
        ("custom_xml_ins_range", Reason::OutOfCuratedScope),
        ("custom_xml_move_from_range", Reason::OutOfCuratedScope),
        ("custom_xml_move_to_range", Reason::OutOfCuratedScope),
        ("custom_xml_parts", Reason::OutOfCuratedScope),
        ("document_settings", Reason::Closure),
        ("edit_cell", Reason::Closure),
        ("edit_comments", Reason::Closure),
        ("edit_document_settings", Reason::Closure),
        ("edit_endnotes", Reason::Closure),
        ("edit_font_table", Reason::Closure),
        ("edit_footnotes", Reason::Closure),
        ("edit_form_field", Reason::Closure),
        ("edit_header_footer", Reason::Closure),
        ("edit_numbering", Reason::Closure),
        ("edit_recipients", Reason::Closure),
        ("edit_section_properties", Reason::Closure),
        ("edit_style_sheet", Reason::Closure),
        ("edit_table", Reason::Closure),
        ("edit_web_settings", Reason::Closure),
        ("font_table", Reason::Closure),
        ("form_field", Reason::Closure),
        ("from_package", Reason::SealedPackage),
        ("glossary_document", Reason::Closure),
        ("header_footer", Reason::Closure),
        ("header_footer_vml_drawings", Reason::PartHandle),
        ("insert_form_field", Reason::OutOfCuratedScope),
        ("move_from_range", Reason::OutOfCuratedScope),
        ("move_to_range", Reason::OutOfCuratedScope),
        ("numbering", Reason::Closure),
        ("paragraph_run_content", Reason::Closure),
        ("parts", Reason::BorrowedView),
        ("recipients", Reason::Closure),
        ("remove_bookmark", Reason::OutOfCuratedScope),
        ("resolve_bookmark", Reason::OutOfCuratedScope),
        ("resolve_data_binding", Reason::Closure),
        ("resolve_footer", Reason::SupersededBy("footer_text")),
        ("resolve_header", Reason::SupersededBy("header_text")),
        ("resolve_numbering", Reason::Closure),
        ("set_equation_run_text", Reason::OutOfCuratedScope),
        ("style_sheet", Reason::Closure),
        ("web_settings", Reason::Closure),
    ],
};

/// `Workbook` against `mjx_xlsx::Workbook`.
///
/// The four groups `crates/mjx-ooxml/src/workbook.rs` names, corrected in two places by the audit:
/// eight entries it filed as *closure doors* take or return an interner-bound model instead
/// ([`Reason::InternerBoundModel`]), and the comment and hyperlink calls are renames rather than
/// omissions — the facade spells them `add_cell_comment`, `cell_comment`, `set_cell_comment_text`,
/// `remove_cell_comment`, `set_cell_hyperlink_url` and `set_cell_hyperlink_location`.
const WORKBOOK: Surface = Surface {
    facade_type: "Workbook",
    facade_source: "crates/mjx-ooxml/src/workbook",
    model_path: "mjx_xlsx::Workbook",
    model_type: "Workbook",
    model_source: "crates/mjx-xlsx/src",
    model_floor: 120,
    facade_floor: 120,
    ledger: &[
        ("add_comment", Reason::RenamedTo("add_cell_comment")),
        ("add_conditional_formatting", Reason::SpecTree),
        ("add_data_validation", Reason::SpecTree),
        ("add_hyperlink_relationship", Reason::PartHandle),
        ("add_table", Reason::SpecTree),
        ("append_differential_format", Reason::SpecTree),
        ("auto_filter", Reason::Closure),
        ("blank_with_properties", Reason::NoFacadeEquivalent),
        ("calculation_chain", Reason::Closure),
        ("cell_text", Reason::SupersededBy("read_range")),
        ("comment_at", Reason::RenamedTo("cell_comment")),
        ("comments_markup", Reason::Closure),
        ("conditional_cell_format", Reason::BorrowedView),
        ("conditional_rules_for", Reason::Closure),
        ("data_validations", Reason::Closure),
        ("drawing_markup", Reason::Closure),
        ("edit_comments_markup", Reason::Closure),
        ("edit_drawing_markup", Reason::Closure),
        ("edit_table_markup", Reason::Closure),
        ("edit_vml_drawing_markup", Reason::Closure),
        ("edit_workbook_markup", Reason::Closure),
        ("from_package", Reason::SealedPackage),
        ("package", Reason::SealedPackage),
        ("part_inventory", Reason::SupersededBy("part_names")),
        ("parts", Reason::BorrowedView),
        ("remove_comment", Reason::RenamedTo("remove_cell_comment")),
        ("set_auto_filter", Reason::SpecTree),
        (
            "set_cell_hyperlink",
            Reason::RenamedTo("set_cell_hyperlink_url"),
        ),
        ("set_cell_value", Reason::SupersededBy("write_cells")),
        (
            "set_comment_text",
            Reason::RenamedTo("set_cell_comment_text"),
        ),
        ("shared_strings", Reason::InternerBoundModel),
        ("sheet_by_name", Reason::BorrowedView),
        ("sheet_formatting", Reason::InternerBoundModel),
        ("sheet_index_by_name", Reason::RenamedTo("sheet_index")),
        ("sheet_markup", Reason::InternerBoundModel),
        ("sheet_markup_of", Reason::InternerBoundModel),
        ("sheet_vml_drawing_part", Reason::PartHandle),
        ("styles_markup", Reason::InternerBoundModel),
        ("table_markup", Reason::Closure),
        ("visible_sheets", Reason::BorrowedView),
        ("vml_drawing_markup", Reason::Closure),
        ("with_vml_shape_for_comment", Reason::Closure),
        ("with_vml_shape_for_form_control", Reason::Closure),
        ("with_vml_shape_for_ole_object", Reason::Closure),
        ("workbook_markup", Reason::Closure),
        ("worksheet", Reason::BorrowedView),
        ("worksheet_by_name", Reason::BorrowedView),
        ("worksheet_markup", Reason::InternerBoundModel),
        ("worksheet_markup_of", Reason::InternerBoundModel),
        ("write_sheet_markup", Reason::InternerBoundModel),
        ("write_worksheet_markup", Reason::InternerBoundModel),
    ],
};

const SURFACES: &[&Surface] = &[&DECK, &DOCUMENT, &WORKBOOK];

/// Every `.rs` file under `directory`, plus `directory.rs` beside it — the two shapes a Rust module
/// tree takes in this workspace, of which `crates/mjx-ooxml/src/deck.rs` beside
/// `crates/mjx-ooxml/src/deck/` is the example this file walks.
fn sources(root: &Path, directory: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let sibling = root.join(format!("{directory}.rs"));
    if sibling.is_file() {
        files.push(sibling);
    }
    let mut stack = vec![root.join(directory)];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// The names of every inherent `pub fn` declared in an `impl <type> { … }` block for `type`.
///
/// The impl header must be exactly `impl <path> {` — no generics, no `for`, no `where` — so a trait
/// implementation and a generic one are both skipped, and the last `::` segment is what is matched,
/// because the facade writes `impl super::Document {` where `mjx-docx` writes `impl Document {`.
fn inherent_methods(files: &[PathBuf], type_name: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for file in files {
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let mut inside = false;
        for line in text.lines() {
            if let Some(target) = line
                .strip_prefix("impl ")
                .and_then(|rest| rest.strip_suffix(" {"))
            {
                inside = target.rsplit("::").next() == Some(type_name);
                continue;
            }
            if line == "}" {
                inside = false;
                continue;
            }
            if !inside {
                continue;
            }
            let Some(rest) = line.strip_prefix("    pub fn ") else {
                continue;
            };
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                names.insert(name);
            }
        }
    }
    names
}

#[test]
fn every_method_left_behind_by_the_facade_is_on_the_ledger() {
    let root = repository_root();
    let mut printed = Vec::new();

    for surface in SURFACES {
        let facade = inherent_methods(&sources(&root, surface.facade_source), surface.facade_type);
        let model = inherent_methods(&sources(&root, surface.model_source), surface.model_type);

        // ---- The floors, before either direction ---------------------------------------------
        // Stated as "the walk is still matching", never as "this surface is exactly this size": a
        // floor pinned to the exact count fires before the assertion it guards and hides the
        // mutation that was supposed to prove that assertion.
        assert!(
            facade.len() >= surface.facade_floor,
            "only {} inherent method(s) were found on `mjx_ooxml::{}`; the impl-block walk over \
             {} has stopped matching, and every comparison below would pass on almost nothing",
            facade.len(),
            surface.facade_type,
            surface.facade_source,
        );
        assert!(
            model.len() >= surface.model_floor,
            "only {} inherent method(s) were found on `{}`; the impl-block walk over {} has \
             stopped matching",
            model.len(),
            surface.model_path,
            surface.model_source,
        );

        let ledger: BTreeMap<&str, Reason> = surface.ledger.iter().copied().collect();
        assert_eq!(
            ledger.len(),
            surface.ledger.len(),
            "the `{}` ledger names the same method twice",
            surface.facade_type,
        );

        let absent: BTreeSet<&str> = model
            .iter()
            .map(String::as_str)
            .filter(|name| !facade.contains(*name))
            .collect();

        // ---- Direction 1: everything left behind is on the ledger ------------------------------
        let unrecorded: Vec<&str> = absent
            .iter()
            .copied()
            .filter(|name| !ledger.contains_key(name))
            .collect();
        assert!(
            unrecorded.is_empty(),
            "{} method(s) of `{}` are not on `mjx_ooxml::{}` and not on this file's ledger:\n  \
             {}\n\nEach is either a method to project onto the facade, a rename to record, or a \
             deliberate omission — say which here, and in `{}.rs`'s own module documentation.",
            unrecorded.len(),
            surface.model_path,
            surface.facade_type,
            unrecorded.join("\n  "),
            surface.facade_source,
        );

        // ---- Direction 2: the ledger names nothing that is not left behind ---------------------
        // The direction that catches a *stale* entry — a method since projected onto the facade, or
        // one deleted from the crate below. Without it the ledger would only ever grow.
        let stale: Vec<String> = ledger
            .keys()
            .filter(|name| !absent.contains(*name))
            .map(|name| {
                let verdict = if model.contains(*name) {
                    "is now on the facade under that name"
                } else {
                    "no longer exists on the model type"
                };
                format!("{name} ({verdict})")
            })
            .collect();
        assert!(
            stale.is_empty(),
            "the `{}` ledger names {} method(s) it should no longer name:\n  {}",
            surface.facade_type,
            stale.len(),
            stale.join("\n  "),
        );

        // ---- And every entry says something ----------------------------------------------------
        // A ledger row whose reason is the classification and nothing else is a row that proves the
        // method is absent and tells a reader nothing — the documentation form of a test with no
        // assertion. `Reason::describe` is what makes a failure readable, so it is exercised here
        // rather than only on the failure path.
        for (name, reason) in &ledger {
            assert!(
                reason.describe().len() > 10,
                "`{name}`'s reason describes itself as {:?}",
                reason.describe(),
            );
        }

        printed.push(format!(
            "  {:<9} {:>4} facade method(s) <- {:>4} on {:<22} {:>3} left behind, all on the ledger",
            surface.facade_type,
            facade.len(),
            model.len(),
            surface.model_path,
            absent.len(),
        ));
    }

    println!("facade curation ledger:\n{}", printed.join("\n"));
}

// ===============================================================================================
// The type-level half of the same question (MJXOFF-228)
// ===============================================================================================

/// The committed Python stub — the one machine-readable statement of the whole projected surface.
const PYTHON_STUB: &str = "bindings/mjx-python/python/mjx_ooxml/__init__.pyi";

/// Classes that are **not** obtainable from another call, with the reason.
///
/// The only category is the exception hierarchy. An exception is obtained by being *raised*, which
/// is a producer no signature mentions, and `bindings/mjx-python/tests/test_errors.py` is what
/// exercises them. Nothing else belongs here: a value class that cannot be obtained is the defect
/// this file is written against, not an entry to be added.
const UNOBTAINABLE_BY_DESIGN: &[(&str, &str)] = &[
    (
        "OoxmlError",
        "raised, never returned — the base of the hierarchy",
    ),
    ("IoError", "raised, never returned"),
    ("MalformedDocumentError", "raised, never returned"),
    ("InvalidDocumentError", "raised, never returned"),
    ("IndexOutOfRangeError", "raised, never returned"),
    ("WrongKindError", "raised, never returned"),
    ("NotFoundError", "raised, never returned"),
    ("NothingToReadError", "raised, never returned"),
    ("InvalidArgumentError", "raised, never returned"),
    ("StructureConflictError", "raised, never returned"),
    ("UnsupportedContentError", "raised, never returned"),
    ("UnsupportedFormatError", "raised, never returned"),
];

/// The floor on how many classes the stub declares, stated as *the walk is still matching*.
const STUB_CLASS_FLOOR: usize = 250;

/// The floor on how many non-documentation lines the walk reads.
const STUB_CODE_LINE_FLOOR: usize = 2_000;

/// **Is every exported value class obtainable from some other call?** — the type-level form of the
/// reachability rule (MJXOFF-228).
///
/// # The hole this closes
///
/// `every_method_left_behind_by_the_facade_is_on_the_ledger` above is a ledger of **methods**. A
/// type with no producer is a shape it was never asked to look for, and the audit found three:
/// `mjx_ooxml::ResolvedColor`, `mjx_ooxml::TableStyleFlags` and `mjx_ooxml::Backdrop` were exported
/// by the facade and by **both** bindings, and returned, taken and constructed by nothing in any of
/// the three — so a caller in three languages could name a type and never obtain a value of it.
/// MJXOFF-228 named the first two; `Backdrop` is this gate's own find, and is why it exists as a
/// sweep rather than as two assertions.
///
/// # Why it reads the Python stub, and why that is not the signature parser the projection gate
/// refused to write
///
/// `xtask/tests/binding_projection.rs`'s header says the type graph needs *"a signature parser over
/// two hand-written crates, and a signature parser that is subtly wrong is worse than none"*. That
/// is right, and it is avoided rather than argued with: **the signature file already exists.**
/// `bindings/mjx-python/python/mjx_ooxml/__init__.pyi` is committed, is checked against the compiled
/// module in both directions by `test_stub_parity.py`, and is checked by `mypy --strict`. Reading it
/// is reading a declaration, not reconstructing one.
///
/// And the reading is deliberately the weakest one that answers the question. It parses **nothing**
/// — no parentheses, no annotations, no line continuations, none of the 31 wrapped `def`s. It asks
/// only: *does this class's name appear anywhere in the stub outside a docstring and outside its own
/// `class` header?* A name that appears **nowhere** is exact — no signature can mention a name that
/// is absent from every line — and that is the direction this asserts, which is the same asymmetry
/// `binding_projection.rs` states about its own measure. A name that *does* appear is only evidence,
/// and the gate takes it at face value; the cost of that is a false green, never a false red.
///
/// The answer is the same in all three languages, which is why this sits here and not in a binding:
/// the Python mapping is the identity, and the wasm binding projects the same facade method by
/// method.
///
/// # What it still cannot see, stated rather than left to be rediscovered
///
/// * **Anything the Python binding does not project.** `crates/mjx-ooxml/src/lib.rs` exports 322
///   type-like names and the stub declares 300 classes; the 34 that are neither — `Presentation`
///   (the escape hatch), the error types (which arrive as Python exception classes), and the
///   measures and Word/Excel types the binding maps to a builtin — are invisible here. Closing that
///   would mean asking the question of the *facade's* own source, which is the signature parser this
///   design exists to avoid.
/// * **A class that is only ever an argument.** A name appearing in a parameter annotation counts as
///   evidence, so a type a caller can hand in and never get back reads as reachable. That was
///   `TableStyleFlags`'s exact half-state before MJXOFF-228: constructible in both bindings and
///   acceptable to nothing.
/// * **Whether the producer is itself reachable.** A method naming a class in its return annotation
///   satisfies this even if no caller can obtain the receiver it hangs off.
///
/// All three are false greens, never false reds, which is the direction this is built to fail in.
#[test]
fn every_exported_class_is_obtainable_from_some_other_call() {
    let stub = std::fs::read_to_string(repository_root().join(PYTHON_STUB))
        .unwrap_or_else(|error| panic!("reading {PYTHON_STUB}: {error}"));

    let mut declared: Vec<String> = Vec::new();
    let mut mentioned: BTreeSet<String> = BTreeSet::new();
    let mut code_lines = 0usize;
    let mut inside_docstring = false;

    for line in stub.lines() {
        let trimmed = line.trim();
        // Documentation is prose, and prose naming a class is not a way to obtain one.
        if inside_docstring {
            if trimmed.ends_with("\"\"\"") {
                inside_docstring = false;
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("\"\"\"") {
            // A one-line docstring opens and closes on the same line.
            if !rest.ends_with("\"\"\"") || rest.len() < 3 {
                inside_docstring = true;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        // A `class X:` header *declares*; it never produces, so it is not evidence for anything.
        if let Some(rest) = line.strip_prefix("class ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                declared.push(name);
            }
            continue;
        }
        code_lines += 1;
        for token in identifiers(line) {
            mentioned.insert(token);
        }
    }

    // ---- The floors, before either direction ---------------------------------------------------
    assert!(
        declared.len() >= STUB_CLASS_FLOOR,
        "only {} class(es) were found in {PYTHON_STUB}; the walk has stopped matching, and every \
         assertion below would pass on almost nothing",
        declared.len(),
    );
    assert!(
        code_lines >= STUB_CODE_LINE_FLOOR,
        "only {code_lines} non-documentation line(s) were read from {PYTHON_STUB}; the docstring \
         skip has swallowed the file, and every class would look unobtainable"
    );

    let ledger: BTreeMap<&str, &str> = UNOBTAINABLE_BY_DESIGN.iter().copied().collect();
    assert_eq!(
        ledger.len(),
        UNOBTAINABLE_BY_DESIGN.len(),
        "the ledger names the same class twice"
    );

    let orphans: Vec<&String> = declared
        .iter()
        .filter(|name| !mentioned.contains(*name))
        .collect();

    // ---- Direction 1: every orphan is on the ledger ---------------------------------------------
    let unrecorded: Vec<&str> = orphans
        .iter()
        .map(|name| name.as_str())
        .filter(|name| !ledger.contains_key(name))
        .collect();
    assert!(
        unrecorded.is_empty(),
        "{} exported class(es) are named by no signature anywhere, so a caller can name the type \
         and never obtain a value of it:\n  {}\n\nEach is either a type whose producer should be \
         projected onto the facade (and then onto both bindings), or an export to remove — which \
         is a breaking change to three surfaces and belongs in the `0.1.0` table. \
         `mjx_ooxml::ResolvedColor`, `TableStyleFlags` and `Backdrop` were the first three \
         (MJXOFF-228).",
        unrecorded.len(),
        unrecorded.join("\n  "),
    );

    // ---- Direction 2: the ledger names nothing that is no longer an orphan -----------------------
    let stale: Vec<&str> = ledger
        .keys()
        .copied()
        .filter(|name| !orphans.iter().any(|orphan| orphan.as_str() == *name))
        .collect();
    assert!(
        stale.is_empty(),
        "the ledger names {} class(es) that are now obtainable, or that the stub no longer \
         declares:\n  {}",
        stale.len(),
        stale.join("\n  "),
    );

    println!(
        "exported classes obtainable from some other call: {} of {} declared in {PYTHON_STUB} \
         ({} on the ledger, all of them raised rather than returned), over {code_lines} \
         non-documentation line(s)",
        declared.len() - orphans.len(),
        declared.len(),
        orphans.len(),
    );
}

/// Every Python identifier in `line`, which is every token the scan treats as evidence.
fn identifiers(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for character in line.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            current.push(character);
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}
