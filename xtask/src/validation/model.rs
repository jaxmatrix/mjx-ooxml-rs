//! The two ingest checks that build a typed element (MJXOFF-278).
//!
//! # The hole these fill
//!
//! Every other check in [`super::ingest`] is byte identity or laziness. `opens`, `round-trip` and
//! `package` are the OPC container; `xml tree` is `mjx-xml`'s untyped fidelity tree; `child order`
//! and `schema` are `mjx-schema-gate` over that same tree. And `facade` — the one that *looks* like
//! it reads the document — opens a [`Deck`], a [`Document`] or a [`Workbook`] and saves it back
//! **with no edit in between**, so part-level laziness re-emits every part from the raw bytes it
//! still holds and not one `FromXml` implementation ever runs. `facade` therefore reports `held` on
//! a file whose every reader in this workspace would refuse it, which is exactly what MJXOFF-278 was
//! filed about after the first ingest of a file Microsoft Office actually wrote.
//!
//! So there are two more, and both of them have to *build* something:
//!
//! * [`model_read`] — walks the whole document through the format model, resolves the inheritance
//!   ladders over markup nobody here wrote, and **prints what it read**. A model check that reads
//!   nothing passes trivially, which is the trap `the_corpus_is_walked_and_its_size_reported` was
//!   written against one layer up; the counts are the anti-vacuity, and they are in the report every
//!   run.
//! * [`typed_edit`] — makes the smallest edit the model offers, saves, and measures what moved. This
//!   is the strongest statement an ingest can make: *Office wrote this, we edited it, and every byte
//!   we did not mean to touch is still Office's.*
//!
//! # Three outcomes for a read, not two
//!
//! A model read that **errors** is this library's, and fails. But a file is allowed to carry markup
//! this project deliberately does not model, and the gaps pages — `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` and
//! its two siblings — are what decide which. Turning a documented non-goal into a red build teaches
//! nobody anything, so the walk splits an error three ways by its
//! [`ErrorCode`](mjx_ooxml::ErrorCode) and says which it saw:
//!
//! * [`ErrorCode::NothingToRead`] — the address exists and answers nothing: a slide with no notes
//!   slide, a picture with no text body, a deck with no notes master. **Counted, not a finding.**
//! * [`ErrorCode::UnsupportedContent`] — a construct this build does not model. **Named in the
//!   report, and it fails nothing**, because that is a gaps-page row rather than a defect.
//! * anything else — `MalformedDocument`, `IndexOutOfRange`, `WrongKind`, `NotFound`, … — is a
//!   reader that could not read what a reader of ours produced an address for. **This library's, and
//!   it fails.**
//!
//! # Why the edit is the sharpest thing here
//!
//! `set_shape_text` and `set_run_text` replace a text leaf and nothing else, and CLAUDE.md's
//! copy-on-write rule says what must follow: the edited text node denies its element and every
//! ancestor of it the verbatim source range, and every *other* subtree keeps its original bytes. So
//! the whole promise is visible in one number — **the bytes a save inserted into a part must be
//! exactly the bytes the caller set**. A writer that re-serialized the part wholesale, or moved a
//! child, or renormalized an attribute, or re-indented, changes that number and nothing else has to
//! be asserted to catch it.
//!
//! The three formats reach that leaf differently and the edit says so per format:
//!
//! | Format | The edit | What must be inserted |
//! |---|---|---|
//! | `.pptx` | one run's text per slide, `Deck::set_shape_text` | the marker text, verbatim |
//! | `.docx` | one run's text, `Document::set_run_text` | the marker text, verbatim |
//! | `.xlsx` | one numeric cell per sheet, `Workbook::write_cells` | the number's own spelling |
//!
//! A workbook's cell is a **number** and not a string on purpose: `CellInput::SharedText` writes
//! into `xl/sharedStrings.xml` as well as the sheet, and `CellInput::InlineText` rewrites the whole
//! `<c>` when the cell it lands in held a shared-string index. Neither is wrong — both are what
//! Excel itself does — but neither isolates the writer's granularity the way replacing one `<v>`'s
//! character data does, and isolating it is the entire point of the measurement. A cell carrying a
//! formula is skipped for the same reason: writing a value there removes the `<f>` beside it.
//!
//! # No file is written and no file is read
//!
//! Bytes in, findings out, exactly as the rest of [`super::ingest`]. The edit happens in memory and
//! is discarded with the report.

use std::fmt::Write as _;

use mjx_ooxml::{
    CellData, CellInput, CellWrite, Deck, Document, Error, ErrorCode, GraphicFrameKind, RunPath,
    ShapeKind, ShapePath, Surface, Workbook,
};
use mjx_opc::Package;

use super::ingest::{Finding, Verdict};
use super::ArtefactFormat;

/// The name the report prints for the walk.
const MODEL: &str = "model";

/// The name the report prints for the edit.
const EDIT: &str = "edit";

// -------------------------------------------------------------------------------------------
// What a walk could not read
// -------------------------------------------------------------------------------------------

/// Every address the walk asked for and did not get, split by what the refusal means.
///
/// The split is the verdict: see the module documentation for why an unmodelled construct and a
/// reader that fell over are not the same finding.
#[derive(Debug, Default)]
struct Notes {
    /// Addresses that exist and answer nothing. Counted rather than listed: a deck of a hundred
    /// pictures has a hundred shapes with no text body and naming each teaches nothing.
    nothing_to_read: usize,
    /// Constructs this build does not model, named in full. A gaps-page row, never a failure.
    unmodelled: Vec<String>,
    /// References the file makes that resolve to nothing inside it — a `numId` its
    /// `word/numbering.xml` does not define, a style id no style declares. **Reported**, for the
    /// same reason [`super::ingest`] reports a package defect a file *arrived* with: a dangling
    /// reference is a statement about the file, and `paragraph_properties.docx` proves a committed
    /// one exists. Named in full so a reader can decide which it is.
    dangling: Vec<String>,
    /// Everything else. **This library's.**
    faults: Vec<String>,
}

impl Notes {
    /// Records what `result` says about the address `at` names, and hands back the value if there
    /// was one.
    ///
    /// `at` is a closure so that the address is only formatted when there is something to say about
    /// it — the walk asks for hundreds of thousands of addresses on a large file and almost all of
    /// them answer.
    fn take<T>(&mut self, at: impl FnOnce() -> String, result: Result<T, Error>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                match error.code() {
                    ErrorCode::NothingToRead => self.nothing_to_read += 1,
                    ErrorCode::UnsupportedContent => {
                        self.unmodelled.push(format!("{}: {error}", at()));
                    }
                    ErrorCode::NotFound => self.dangling.push(format!("{}: {error}", at())),
                    _ => self.faults.push(format!("{}: {error}", at())),
                }
                None
            }
        }
    }

    /// The clauses the report appends to a walk's counts.
    fn clauses(&self) -> String {
        let mut text = String::new();
        if self.nothing_to_read > 0 {
            let _ = write!(
                text,
                "; {} address(es) held nothing to read, which is a shape without a text body or a \
                 slide without notes and not a finding",
                self.nothing_to_read
            );
        }
        if !self.unmodelled.is_empty() {
            let _ = write!(
                text,
                "; {} construct(s) this build does not model, which the gaps pages own and which \
                 fail nothing:\n    {}",
                self.unmodelled.len(),
                self.unmodelled.join("\n    ")
            );
        }
        if !self.dangling.is_empty() {
            let _ = write!(
                text,
                "; {} reference(s) the file makes that resolve to nothing inside it. That is the \
                 file's, not this library's — read them, then decide:\n    {}",
                self.dangling.len(),
                self.dangling.join("\n    ")
            );
        }
        text
    }
}

// -------------------------------------------------------------------------------------------
// The walk
// -------------------------------------------------------------------------------------------

/// Opens the file through its format model, walks all of it, and reports what it read.
///
/// The last clause is the one CLAUDE.md's laziness rule claims and nothing had measured: after a
/// walk that touched every surface, every paragraph and every inheritance ladder, saving the
/// document back must still produce every part's original bytes. **Reading must not dirty a part**,
/// and if it did, every byte comparison downstream of a read would be vacuous.
pub(super) fn model_read(format: ArtefactFormat, package: &Package, bytes: &[u8]) -> Finding {
    let mut notes = Notes::default();
    let read = match format {
        ArtefactFormat::Presentation => match Deck::open(bytes) {
            Err(error) => return refused(MODEL, &error),
            Ok(mut deck) => {
                let counts = walk_deck(&mut deck, &mut notes);
                let clean = unchanged_after(package, deck.save_unchecked());
                (counts.describe(), clean)
            }
        },
        ArtefactFormat::Document => match Document::open(bytes) {
            Err(error) => return refused(MODEL, &error),
            Ok(mut document) => {
                let counts = walk_document(&mut document, &mut notes);
                let clean = unchanged_after(package, document.save_unchecked());
                (counts.describe(), clean)
            }
        },
        ArtefactFormat::Workbook => match Workbook::open(bytes) {
            Err(error) => return refused(MODEL, &error),
            Ok(mut workbook) => {
                let counts = walk_workbook(&mut workbook, &mut notes);
                let clean = unchanged_after(package, workbook.save_unchecked());
                (counts.describe(), clean)
            }
        },
    };
    let (counts, dirtied) = read;

    if !notes.faults.is_empty() {
        return Finding {
            check: MODEL,
            verdict: Verdict::Failed,
            detail: format!(
                "the format model refused {} address(es) this library's own readers produced. \
                 Every one is a reader of ours meeting markup Office wrote, which is what an \
                 Office-authored file is for. Read after {counts}:\n    {}",
                notes.faults.len(),
                notes.faults.join("\n    ")
            ),
        };
    }
    match dirtied {
        Err(problem) => Finding {
            check: MODEL,
            verdict: Verdict::Failed,
            detail: format!(
                "read {counts}, and then {problem}. Reading must not dirty a part — CLAUDE.md's \
                 laziness rule is what every byte comparison downstream of a read depends on"
            ),
        },
        Ok(entries) => Finding {
            check: MODEL,
            verdict: if notes.dangling.is_empty() {
                Verdict::Held
            } else {
                Verdict::Reported
            },
            detail: format!(
                "{counts}; {entries} entry/entries still byte-identical afterwards, so reading \
                 dirtied nothing{}",
                notes.clauses()
            ),
        },
    }
}

/// The finding for a document the model would not open at all.
fn refused(check: &'static str, error: &Error) -> Finding {
    Finding {
        check,
        verdict: Verdict::Failed,
        detail: format!(
            "the format model refused to open it ({}): {error}",
            error.code().as_str()
        ),
    }
}

/// Whether `saved` still holds every entry of `package`, byte for byte — the laziness claim.
fn unchanged_after(package: &Package, saved: Result<Vec<u8>, Error>) -> Result<usize, String> {
    let saved = saved.map_err(|error| format!("saving it back refused: {error}"))?;
    let reopened =
        Package::open(&saved).map_err(|error| format!("what it wrote does not reopen: {error}"))?;
    let mut changed = Vec::new();
    for original in package.entries() {
        match reopened
            .entries()
            .iter()
            .find(|entry| entry.name == original.name)
        {
            None => changed.push(format!("{}: gone", original.name)),
            Some(written) if written.bytes() != original.bytes() => {
                changed.push(format!("{}: payload changed", original.name));
            }
            Some(_) => {}
        }
    }
    if changed.is_empty() {
        Ok(package.entries().len())
    } else {
        Err(format!(
            "{} entry/entries changed across a save that only read:\n    {}",
            changed.len(),
            changed.join("\n    ")
        ))
    }
}

// -------------------------------------------------------------------------------------------
// PresentationML
// -------------------------------------------------------------------------------------------

/// What a walk of a `Deck` read.
///
/// Table cells are counted apart from shapes on purpose. A cell's `a:txBody` is the *same*
/// `CT_TextBody` a shape's `p:txBody` is, so folding the two together would make "runs" a number
/// nobody could compare against a shape count.
#[derive(Debug, Default)]
struct DeckCounts {
    surfaces: usize,
    shapes: usize,
    with_text_body: usize,
    groups: usize,
    tables: usize,
    table_cells: usize,
    paragraphs: usize,
    runs: usize,
    characters: usize,
    cell_runs: usize,
    cell_characters: usize,
    run_properties: usize,
    shape_fills: usize,
}

impl DeckCounts {
    fn describe(&self) -> String {
        format!(
            "{} surface(s), {} shape(s) ({} of them with a text body, {} group(s) descended into), \
             {} table(s) of {} cell(s), {} paragraph(s), {} run(s), {} character(s), and {} run(s) \
             of {} character(s) inside those cells; {} effective_run_properties and {} \
             effective_shape_fill resolution(s)",
            self.surfaces,
            self.shapes,
            self.with_text_body,
            self.groups,
            self.tables,
            self.table_cells,
            self.paragraphs,
            self.runs,
            self.characters,
            self.cell_runs,
            self.cell_characters,
            self.run_properties,
            self.shape_fills
        )
    }
}

/// Every shape-bearing surface the deck holds, including the ones that may not be there.
///
/// A notes slide and a notes master are optional, and a deck that has neither is not defective — so
/// each is *probed* rather than assumed, and an [`ErrorCode::NothingToRead`] means absent.
///
/// The probe reads the shape list a second time, and deliberately: the part it needs is materialized
/// by then, the list is a few dozen small values, and the alternative — carrying every surface's
/// shapes out of here so the walk can reuse them — would hold every shape list in the deck alive at
/// once to save one cheap call per notes slide.
fn surfaces(deck: &mut Deck, notes: &mut Notes) -> Vec<Surface> {
    let mut all: Vec<Surface> = (0..deck.slide_count()).map(Surface::Slide).collect();
    all.extend((0..deck.layout_count()).map(Surface::Layout));
    all.extend((0..deck.master_count()).map(Surface::Master));
    for slide in 0..deck.slide_count() {
        let surface = Surface::Notes(slide);
        if present(notes, || surface.to_string(), deck.shapes(surface)) {
            all.push(surface);
        }
    }
    let master = Surface::NotesMaster;
    if present(notes, || master.to_string(), deck.shapes(master)) {
        all.push(master);
    }
    all
}

/// Whether an optional surface is there: `Ok` yes, [`ErrorCode::NothingToRead`] no, anything else a
/// fault recorded in `notes`.
fn present<T>(notes: &mut Notes, at: impl FnOnce() -> String, probe: Result<T, Error>) -> bool {
    match probe {
        Ok(_) => true,
        Err(error) if error.code() == ErrorCode::NothingToRead => false,
        Err(error) => {
            notes.faults.push(format!("{}: {error}", at()));
            false
        }
    }
}

fn walk_deck(deck: &mut Deck, notes: &mut Notes) -> DeckCounts {
    let mut counts = DeckCounts::default();
    for surface in surfaces(deck, notes) {
        counts.surfaces += 1;
        let Some(shapes) = notes.take(|| surface.to_string(), deck.shapes(surface)) else {
            continue;
        };
        for shape in shapes {
            walk_shape(
                deck,
                surface,
                &ShapePath::from(shape.index),
                shape.kind,
                &mut counts,
                notes,
            );
        }
    }
    counts
}

/// One shape, and — for a group — everything inside it.
fn walk_shape(
    deck: &mut Deck,
    surface: Surface,
    path: &ShapePath,
    kind: ShapeKind,
    counts: &mut DeckCounts,
    notes: &mut Notes,
) {
    counts.shapes += 1;
    let at = || format!("{surface} shape {path}");
    if notes
        .take(at, deck.effective_shape_fill(surface, path.clone()))
        .is_some()
    {
        counts.shape_fills += 1;
    }
    match kind {
        ShapeKind::GroupShape => {
            let Some(members) = notes.take(at, deck.shape_member_count(surface, path.clone()))
            else {
                return;
            };
            counts.groups += 1;
            for member in 0..members {
                let child = path.child(member);
                let Some(kind) = notes.take(
                    || format!("{surface} shape {child}"),
                    deck.shape_kind(surface, child.clone()),
                ) else {
                    continue;
                };
                walk_shape(deck, surface, &child, kind, counts, notes);
            }
        }
        ShapeKind::GraphicFrame => {
            if let Some(Some(GraphicFrameKind::Table)) =
                notes.take(at, deck.graphic_frame_kind(surface, path.clone()))
            {
                walk_table(deck, surface, path, counts, notes);
            }
        }
        // A `p:sp` is the only kind that can carry a `p:txBody`; asking a picture or a connector for
        // one answers `NothingToRead` and would inflate that counter with an absence the schema
        // already guarantees.
        ShapeKind::Shape => walk_text_body(deck, surface, path, counts, notes),
        // `ShapeKind` is `#[non_exhaustive]`, so this arm is the picture, the connector, the
        // content part — and whatever kind is added next, which will carry no `p:txBody` on the
        // day it appears either.
        _ => {}
    }
}

/// A shape's own text, and the run-property ladder over every run of it.
fn walk_text_body(
    deck: &mut Deck,
    surface: Surface,
    path: &ShapePath,
    counts: &mut DeckCounts,
    notes: &mut Notes,
) {
    let at = || format!("{surface} shape {path}");
    let Some(paragraphs) = notes.take(at, deck.paragraph_count(surface, path.clone())) else {
        return;
    };
    counts.with_text_body += 1;
    for paragraph in 0..paragraphs {
        counts.paragraphs += 1;
        let at = || format!("{surface} shape {path} paragraph {paragraph}");
        notes.take(
            at,
            deck.effective_paragraph_properties(surface, path.clone(), paragraph),
        );
        let Some(runs) = notes.take(at, deck.run_count(surface, path.clone(), paragraph)) else {
            continue;
        };
        for run in 0..runs {
            counts.runs += 1;
            let at = || format!("{surface} shape {path} paragraph {paragraph} run {run}");
            if let Some(text) = notes.take(at, deck.run_text(surface, path.clone(), paragraph, run))
            {
                counts.characters += text.chars().count();
            }
            if notes
                .take(
                    at,
                    deck.effective_run_properties(surface, path.clone(), paragraph, run),
                )
                .is_some()
            {
                counts.run_properties += 1;
            }
        }
    }
}

/// The table a graphic frame holds, cell by cell.
fn walk_table(
    deck: &mut Deck,
    surface: Surface,
    path: &ShapePath,
    counts: &mut DeckCounts,
    notes: &mut Notes,
) {
    let at = || format!("{surface} table {path}");
    let Some((rows, columns)) = notes.take(at, deck.table_dimensions(surface, path.clone())) else {
        return;
    };
    counts.tables += 1;
    for row in 0..rows {
        for column in 0..columns {
            counts.table_cells += 1;
            let at = || format!("{surface} table {path} cell r{row}c{column}");
            let Some(paragraphs) = notes.take(
                at,
                deck.cell_paragraph_count(surface, path.clone(), row, column),
            ) else {
                continue;
            };
            for paragraph in 0..paragraphs {
                let Some(runs) = notes.take(
                    at,
                    deck.cell_run_count(surface, path.clone(), row, column, paragraph),
                ) else {
                    continue;
                };
                for run in 0..runs {
                    counts.cell_runs += 1;
                    if let Some(text) = notes.take(
                        at,
                        deck.cell_run_text(surface, path.clone(), row, column, paragraph, run),
                    ) {
                        counts.cell_characters += text.chars().count();
                    }
                    notes.take(
                        at,
                        deck.effective_cell_run_properties(
                            surface,
                            path.clone(),
                            row,
                            column,
                            paragraph,
                            run,
                        ),
                    );
                }
            }
        }
    }
}

// -------------------------------------------------------------------------------------------
// WordprocessingML
// -------------------------------------------------------------------------------------------

/// What a walk of a `Document` read.
#[derive(Debug, Default)]
struct DocumentCounts {
    paragraphs: usize,
    runs: usize,
    characters: usize,
    containers: usize,
    tables: usize,
    table_cells: usize,
    cell_characters: usize,
    sections: usize,
    footnotes: usize,
    endnotes: usize,
    comments: usize,
    styles: usize,
    run_properties: usize,
    paragraph_properties: usize,
}

impl DocumentCounts {
    fn describe(&self) -> String {
        format!(
            "{} body paragraph(s), {} run(s) of {} character(s) ({} run container(s) descended \
             into), {} table(s) of {} cell(s) holding {} character(s), {} section(s), {} \
             footnote(s), {} endnote(s), {} comment(s), {} style(s); {} \
             effective_run_properties and {} effective_paragraph_properties resolution(s)",
            self.paragraphs,
            self.runs,
            self.characters,
            self.containers,
            self.tables,
            self.table_cells,
            self.cell_characters,
            self.sections,
            self.footnotes,
            self.endnotes,
            self.comments,
            self.styles,
            self.run_properties,
            self.paragraph_properties
        )
    }
}

fn walk_document(document: &mut Document, notes: &mut Notes) -> DocumentCounts {
    let mut counts = DocumentCounts::default();
    let paragraphs = notes
        .take(|| "the body".to_owned(), document.paragraph_count())
        .unwrap_or_default();
    for paragraph in 0..paragraphs {
        counts.paragraphs += 1;
        let at = || format!("paragraph {paragraph}");
        if notes
            .take(at, document.effective_paragraph_properties(paragraph))
            .is_some()
        {
            counts.paragraph_properties += 1;
        }
        let Some(slots) = notes.take(at, document.run_count(paragraph)) else {
            continue;
        };
        for slot in 0..slots {
            walk_run_slot(document, paragraph, slot, &mut counts, notes);
        }
    }

    let tables = notes
        .take(|| "the body".to_owned(), document.table_count())
        .unwrap_or_default();
    for table in 0..tables {
        let at = || format!("table {table}");
        let Some((rows, columns)) = notes.take(at, document.table_dimensions(table)) else {
            continue;
        };
        counts.tables += 1;
        for row in 0..rows {
            for column in 0..columns {
                counts.table_cells += 1;
                if let Some(text) = notes.take(
                    || format!("table {table} cell r{row}c{column}"),
                    document.cell_text(table, row, column),
                ) {
                    counts.cell_characters += text.chars().count();
                }
            }
        }
    }

    let at = || "the document".to_owned();
    counts.sections = notes.take(at, document.sections()).map_or(0, |s| s.len());
    counts.footnotes = notes.take(at, document.footnotes()).map_or(0, |n| n.len());
    counts.endnotes = notes.take(at, document.endnotes()).map_or(0, |n| n.len());
    counts.comments = notes.take(at, document.comments()).map_or(0, |c| c.len());
    counts.styles = notes.take(at, document.style_ids()).map_or(0, |s| s.len());
    counts
}

/// One run-or-hyperlink slot of a paragraph.
///
/// `run_count` counts *slots*, and a `w:hyperlink` is one — it holds runs rather than being one. The
/// model has no accessor that says which a slot is, so the walk asks for the text and, when the
/// address does not resolve to a run, descends into the slot until a child does not resolve either.
/// A slot that is neither a run nor a container of runs is left to [`Notes`] as a fault, which is
/// what it is.
fn walk_run_slot(
    document: &mut Document,
    paragraph: u32,
    slot: u32,
    counts: &mut DocumentCounts,
    notes: &mut Notes,
) {
    let path = RunPath::from(slot);
    match document.run_text(paragraph, path.clone()) {
        Ok(text) => read_run(document, paragraph, &path, text, counts, notes),
        // `run_count` counts run-*or-container* slots, and the model has no accessor that says
        // which a slot is: a `w:hyperlink`, a `w:sdt`, a `w:smartTag`, a `w:customXml`, a `w:dir`
        // and a `w:bdo` are each one slot holding runs of their own, and a `w:sdt` with no content
        // run holds none at all. So a slot whose bare address is not a run is descended into, and
        // finding no run inside it is a shape of the file rather than a reader that failed. The
        // code is `IndexOutOfRange` because `DocxError::AddressNotFound` maps to it, and every
        // index asked for here came from `run_count`, so it can only mean "that slot is not a run".
        Err(error) if error.code() == ErrorCode::IndexOutOfRange => {
            counts.containers += 1;
            let mut nested = 0u32;
            while let Ok(text) = document.run_text(paragraph, path.child(nested)) {
                read_run(
                    document,
                    paragraph,
                    &path.child(nested),
                    text,
                    counts,
                    notes,
                );
                nested += 1;
            }
        }
        Err(error) => notes
            .faults
            .push(format!("paragraph {paragraph} run {path}: {error}")),
    }
}

/// One run that resolved: its text, and the ladder above it.
fn read_run(
    document: &mut Document,
    paragraph: u32,
    path: &RunPath,
    text: String,
    counts: &mut DocumentCounts,
    notes: &mut Notes,
) {
    counts.runs += 1;
    counts.characters += text.chars().count();
    if notes
        .take(
            || format!("paragraph {paragraph} run {path}"),
            document.effective_run_properties(paragraph, path.clone()),
        )
        .is_some()
    {
        counts.run_properties += 1;
    }
}

// -------------------------------------------------------------------------------------------
// SpreadsheetML
// -------------------------------------------------------------------------------------------

/// What a walk of a `Workbook` read.
#[derive(Debug, Default)]
struct WorkbookCounts {
    sheets: usize,
    cells: usize,
    numbers: usize,
    strings: usize,
    booleans: usize,
    error_cells: usize,
    blanks: usize,
    /// Cells whose `<v>` states a token their own `c@t` cannot read. **A statement about the file**,
    /// like a dangling reference and unlike a fault: no schema admits the token, the file states it
    /// anyway, and MJXOFF-285 is where the facade stopped reporting such a cell as a blank.
    unreadable: usize,
    formulas: usize,
    characters: usize,
    merged_ranges: usize,
}

impl WorkbookCounts {
    fn describe(&self) -> String {
        format!(
            "{} sheet(s), {} cell(s) in their used ranges — {} number(s), {} string(s) of {} \
             character(s), {} boolean(s), {} error code(s), {} blank(s), {} value(s) the file \
             states that no cell type here can read — {} formula(s) and {} merged range(s)",
            self.sheets,
            self.cells,
            self.numbers,
            self.strings,
            self.characters,
            self.booleans,
            self.error_cells,
            self.blanks,
            self.unreadable,
            self.formulas,
            self.merged_ranges
        )
    }
}

fn walk_workbook(workbook: &mut Workbook, notes: &mut Notes) -> WorkbookCounts {
    let mut counts = WorkbookCounts::default();
    for sheet in 0..workbook.sheet_count() {
        counts.sheets += 1;
        let at = || format!("sheet {sheet}");
        notes.take(at, workbook.sheet(sheet));
        if let Some(ranges) = notes.take(at, workbook.merged_ranges(sheet)) {
            counts.merged_ranges += ranges.len();
        }
        let Some(block) = notes.take(at, workbook.read_sheet(sheet)) else {
            continue;
        };
        for row in 0..block.row_count() {
            for column in 0..block.column_count() {
                let at = || format!("sheet {sheet} cell r{row}c{column}");
                let Some(value) = notes.take(at, block.value(row, column)) else {
                    continue;
                };
                counts.cells += 1;
                match value {
                    CellData::Blank => counts.blanks += 1,
                    CellData::Number(_) => counts.numbers += 1,
                    CellData::Text(text) => {
                        counts.strings += 1;
                        counts.characters += text.chars().count();
                    }
                    CellData::Boolean(_) => counts.booleans += 1,
                    CellData::Error(_) => counts.error_cells += 1,
                    CellData::Unreadable(_) => counts.unreadable += 1,
                }
                if notes
                    .take(at, block.formula(row, column))
                    .flatten()
                    .is_some()
                {
                    counts.formulas += 1;
                }
            }
        }
    }
    counts
}

// -------------------------------------------------------------------------------------------
// The edit
// -------------------------------------------------------------------------------------------

/// One edit the check made, and the bytes a save must have inserted for it.
#[derive(Debug)]
struct Edit {
    /// Where it was made, as the report names it.
    at: String,
    /// What was there before, decoded — printed so a reader can see what the edit displaced.
    displaced: String,
    /// The bytes the save must insert.
    inserted: Vec<u8>,
    /// How tightly they are pinned.
    granularity: Granularity,
}

/// How much of a part a save is allowed to have rewritten for one edit — **which is not the same
/// question in all three formats**, and the check says so rather than holding two architectures to
/// one number.
///
/// The measurement stayed the same either way: what changed, and did anything else. What differs is
/// how small "the thing that changed" can be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Granularity {
    /// **The inserted bytes are the set text and nothing else.** A `p:txBody` and a `w:p` are
    /// `RawElement` trees, so replacing a text leaf denies the verbatim source range to that leaf's
    /// ancestors and to nothing else; every sibling subtree is copied out of the original bytes.
    /// Nothing weaker than exact equality is worth asserting there.
    TextLeaf,
    /// **The inserted bytes carry the value and stay inside one `<c>`.** A worksheet's cells live in
    /// `mjx-sml`'s packed store rather than in a `RawElement` tree, so writing one re-serializes
    /// that whole cell — `t="n"`, the schema default `sample.xlsx` spells out, is not written back.
    /// That is a cell wide and no wider, and the check holds it to exactly that: the region carries
    /// the number, and it crosses no cell or row boundary, so every other cell in the sheet kept
    /// its own bytes.
    Cell,
}

impl Granularity {
    /// Whether `inserted`, in place of `removed`, is what this granularity allows for `wanted`.
    fn accepts(self, wanted: &[u8], removed: &[u8], inserted: &[u8]) -> bool {
        match self {
            Self::TextLeaf => inserted == wanted,
            // Not a substring test on `wanted`: a minimal diff lets the common suffix absorb the
            // tail of the value when the digits happen to agree (`…0</v>` after `…0</v>`), so the
            // region is not obliged to hold the whole spelling. What the region *must* show is
            // locality, and `edit_workbook` proves the value itself by reading the saved cell back
            // through the model.
            Self::Cell => !crosses_a_cell(removed) && !crosses_a_cell(inserted),
        }
    }

    /// What the report says this granularity asked for.
    fn describe(self) -> &'static str {
        match self {
            Self::TextLeaf => "one contiguous region holding exactly the bytes that were set",
            Self::Cell => {
                "one contiguous region inside a single `<c>` element, holding the value \
                           that was written"
            }
        }
    }
}

/// Whether `haystack` holds `needle` — the substring search the `Cell` granularity needs.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.len() <= haystack.len()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

/// Whether a changed region reaches past the cell it should be inside.
fn crosses_a_cell(region: &[u8]) -> bool {
    [&b"<c "[..], b"<c/", b"<c>", b"</c>", b"<row", b"</row>"]
        .iter()
        .any(|boundary| contains(region, boundary))
}

/// The edits one format's planner made, and the bytes its save produced.
#[derive(Debug)]
struct EditedDocument {
    /// One entry per edit, in the order they were made.
    edits: Vec<Edit>,
    /// What `save` wrote.
    saved: Vec<u8>,
}

/// Makes the smallest edit the model offers, saves, and measures exactly what moved.
///
/// This is the fidelity contract stated against a real file rather than against our own output. See
/// the module documentation for the per-format table of what "the smallest edit" is and why a
/// workbook's is a number.
pub(super) fn typed_edit(format: ArtefactFormat, package: &Package, bytes: &[u8]) -> Finding {
    let planned = match format {
        ArtefactFormat::Presentation => edit_deck(bytes),
        ArtefactFormat::Document => edit_document(bytes),
        ArtefactFormat::Workbook => edit_workbook(bytes),
    };
    let EditedDocument { edits, saved } = match planned {
        Err(problem) => {
            return Finding {
                check: EDIT,
                verdict: Verdict::Failed,
                detail: problem,
            }
        }
        Ok(None) => {
            return Finding {
                check: EDIT,
                verdict: Verdict::Skipped,
                detail: format!(
                    "nothing in this .{} is editable through the typed model without changing more \
                     than one text leaf — a deck with no text run, a document with no run, or a \
                     workbook with no formula-free numeric cell. The measurement is not made, and \
                     it is not a pass",
                    format.extension()
                ),
            }
        }
        Ok(Some(made)) => made,
    };
    measure(package, &edits, &saved)
}

/// Compares the saved bytes against the original package and says whether the edit is all that
/// moved.
fn measure(package: &Package, edits: &[Edit], saved: &[u8]) -> Finding {
    let reopened = match Package::open(saved) {
        Ok(reopened) => reopened,
        Err(error) => {
            return Finding {
                check: EDIT,
                verdict: Verdict::Failed,
                detail: format!("what the edited document wrote does not reopen: {error}"),
            }
        }
    };

    let mut collateral = Vec::new();
    let mut matched = vec![false; edits.len()];
    let mut touched_bytes = 0usize;
    let mut changed = 0usize;
    for original in package.entries() {
        let Some(written) = reopened
            .entries()
            .iter()
            .find(|entry| entry.name == original.name)
        else {
            collateral.push(format!("{}: gone after an edit", original.name));
            continue;
        };
        let (Some(before), Some(after)) = (original.bytes(), written.bytes()) else {
            collateral.push(format!("{}: no materialized bytes", original.name));
            continue;
        };
        if before == after {
            continue;
        }
        changed += 1;
        touched_bytes += before.len();
        let (removed, inserted) = one_region(before, after);
        match edits
            .iter()
            .zip(&matched)
            .position(|(edit, landed)| {
                !*landed && edit.granularity.accepts(&edit.inserted, removed, inserted)
            })
        {
            Some(index) => matched[index] = true,
            None => collateral.push(format!(
                "{}: the save removed {} byte(s) and inserted {} in their place, and those are not \
                 the bytes any edit set. A part that re-serializes further than the one text leaf \
                 that changed is the collateral rewriting this check exists to find. Inserted: {:?}",
                original.name,
                removed.len(),
                inserted.len(),
                truncate(&String::from_utf8_lossy(inserted))
            )),
        }
    }
    for written in reopened.entries() {
        if !package
            .entries()
            .iter()
            .any(|entry| entry.name == written.name)
        {
            collateral.push(format!("{}: added by an edit", written.name));
        }
    }
    let unlanded: Vec<&Edit> = edits
        .iter()
        .zip(&matched)
        .filter(|(_, landed)| !**landed)
        .map(|(edit, _)| edit)
        .collect();

    if !collateral.is_empty() || !unlanded.is_empty() {
        let mut detail = format!(
            "{} edit(s) through the typed model changed {changed} entry/entries",
            edits.len()
        );
        if !unlanded.is_empty() {
            let _ = write!(
                detail,
                "; {} edit(s) left no part carrying exactly the bytes they set:\n    {}",
                unlanded.len(),
                unlanded
                    .iter()
                    .map(|edit| format!(
                        "{}: set {:?}",
                        edit.at,
                        String::from_utf8_lossy(&edit.inserted)
                    ))
                    .collect::<Vec<_>>()
                    .join("\n    ")
            );
        }
        if !collateral.is_empty() {
            let _ = write!(detail, ":\n    {}", collateral.join("\n    "));
        }
        return Finding {
            check: EDIT,
            verdict: Verdict::Failed,
            detail,
        };
    }

    Finding {
        check: EDIT,
        verdict: Verdict::Held,
        detail: format!(
            "{} edit(s) through the typed model changed exactly {changed} of {} entry/entries, and \
             across {touched_bytes} byte(s) of the markup they touched each changed part differs \
             from the original in {}. Edited: {}",
            edits.len(),
            package.entries().len(),
            edits
                .first()
                .map_or("no region at all", |edit| edit.granularity.describe()),
            edits
                .iter()
                .map(|edit| format!("{} (was {:?})", edit.at, truncate(&edit.displaced)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// The one contiguous region in which `after` differs from `before`: what was removed, and what was
/// put in its place.
///
/// Computed as the common prefix and the common suffix, so the pair it returns spans *every*
/// difference. That is what makes the equality test downstream a proof of contiguity rather than an
/// assumption: two separate edits would return a region with the untouched middle inside it, and a
/// middle is not the marker.
fn one_region<'a>(before: &'a [u8], after: &'a [u8]) -> (&'a [u8], &'a [u8]) {
    let prefix = before
        .iter()
        .zip(after)
        .take_while(|(left, right)| left == right)
        .count();
    let limit = before.len().min(after.len()) - prefix;
    let suffix = before
        .iter()
        .rev()
        .zip(after.iter().rev())
        .take_while(|(left, right)| left == right)
        .count()
        .min(limit);
    (
        &before[prefix..before.len() - suffix],
        &after[prefix..after.len() - suffix],
    )
}

/// A displaced string, short enough to sit on a report line.
fn truncate(text: &str) -> String {
    const LIMIT: usize = 40;
    if text.chars().count() <= LIMIT {
        return text.to_owned();
    }
    let head: String = text.chars().take(LIMIT).collect();
    format!("{head}…")
}

/// The text every edit writes, distinct per edit so no two can be confused for one another.
fn marker(ordinal: usize) -> String {
    format!("mjx typed model edit {ordinal}")
}

/// One run's text replaced on every slide that has one.
fn edit_deck(bytes: &[u8]) -> Result<Option<EditedDocument>, String> {
    let mut deck = Deck::open(bytes).map_err(|error| format!("Deck::open refused it: {error}"))?;
    let mut edits = Vec::new();
    for slide in 0..deck.slide_count() {
        let surface = Surface::Slide(slide);
        let Ok(shapes) = deck.shapes(surface) else {
            continue;
        };
        let Some((path, run, text)) = shapes
            .iter()
            .filter(|shape| shape.kind == ShapeKind::Shape)
            .find_map(|shape| {
                first_non_empty_run(&mut deck, surface, ShapePath::from(shape.index))
            })
        else {
            continue;
        };
        let inserted = marker(edits.len());
        deck.set_shape_text(surface, path.clone(), run, &inserted)
            .map_err(|error| {
                format!("set_shape_text on {surface} shape {path} refused: {error}")
            })?;
        edits.push(Edit {
            at: format!("{surface} shape {path} run {run}"),
            displaced: text,
            inserted: inserted.into_bytes(),
            granularity: Granularity::TextLeaf,
        });
    }
    if edits.is_empty() {
        return Ok(None);
    }
    let saved = deck
        .save()
        .map_err(|error| format!("saving the edited deck refused: {error}"))?;
    Ok(Some(EditedDocument { edits, saved }))
}

/// The first run of `path` whose text is not empty, as a flattened run index — what
/// `set_shape_text` addresses.
///
/// Empty is skipped for one reason: an `a:t` a producer wrote self-closing has no character data to
/// replace, so writing into it moves the tag as well as the text and the region would no longer be
/// the marker alone.
fn first_non_empty_run(
    deck: &mut Deck,
    surface: Surface,
    path: ShapePath,
) -> Option<(ShapePath, u32, String)> {
    let paragraphs = deck.paragraph_count(surface, path.clone()).ok()?;
    let mut flattened = 0u32;
    for paragraph in 0..paragraphs {
        let runs = deck.run_count(surface, path.clone(), paragraph).ok()?;
        for run in 0..runs {
            let text = deck
                .run_text(surface, path.clone(), paragraph, run)
                .ok()
                .unwrap_or_default();
            if !text.is_empty() {
                return Some((path, flattened, text));
            }
            flattened += 1;
        }
    }
    None
}

/// One run's text replaced in the document body.
fn edit_document(bytes: &[u8]) -> Result<Option<EditedDocument>, String> {
    let mut document =
        Document::open(bytes).map_err(|error| format!("Document::open refused it: {error}"))?;
    let paragraphs = document
        .paragraph_count()
        .map_err(|error| format!("reading the body refused: {error}"))?;
    let mut found = None;
    'search: for paragraph in 0..paragraphs {
        let Ok(slots) = document.run_count(paragraph) else {
            continue;
        };
        for slot in 0..slots {
            if let Ok(text) = document.run_text(paragraph, slot) {
                // `Run::set_text` maintains `xml:space="preserve"`: it adds the attribute when the
                // new text begins or ends in whitespace and removes it when it does not. That is
                // correct, and it is not what this check measures — a run whose text is bounded by
                // whitespace would move the attribute as well as the text and the region would no
                // longer be the marker alone. `mjx-dml`'s `Text::set_text` touches no attribute, so
                // the deck above needs no such rule.
                if !text.is_empty() && text.trim() == text {
                    found = Some((paragraph, slot, text));
                    break 'search;
                }
            }
        }
    }
    let Some((paragraph, run, displaced)) = found else {
        return Ok(None);
    };
    let inserted = marker(0);
    document
        .set_run_text(paragraph, run, &inserted)
        .map_err(|error| {
            format!("set_run_text on paragraph {paragraph} run {run} refused: {error}")
        })?;
    let saved = document
        .save()
        .map_err(|error| format!("saving the edited document refused: {error}"))?;
    Ok(Some(EditedDocument {
        edits: vec![Edit {
            at: format!("paragraph {paragraph} run {run}"),
            displaced,
            inserted: inserted.into_bytes(),
            granularity: Granularity::TextLeaf,
        }],
        saved,
    }))
}

/// One numeric cell rewritten on every sheet that has a formula-free one.
fn edit_workbook(bytes: &[u8]) -> Result<Option<EditedDocument>, String> {
    let mut workbook =
        Workbook::open(bytes).map_err(|error| format!("Workbook::open refused it: {error}"))?;
    let mut planned = Vec::new();
    for sheet in 0..workbook.sheet_count() {
        let Ok(block) = workbook.read_sheet(sheet) else {
            continue;
        };
        'cells: for row in 0..block.row_count() {
            for column in 0..block.column_count() {
                let Ok(CellData::Number(displaced)) = block.value(row, column) else {
                    continue;
                };
                if block.formula(row, column).ok().flatten().is_some() {
                    continue;
                }
                // Distinct per edit, integral so its shortest round-tripping spelling has no
                // exponent, and far from anything a spreadsheet is likely to hold.
                let value = 20_260_910.0 + f64::from(u32::try_from(planned.len()).unwrap_or(0));
                let reference = a1(block.first_row() + row, block.first_column() + column);
                planned.push((
                    sheet,
                    Edit {
                        at: format!("sheet {sheet} cell {reference}"),
                        displaced: displaced.to_string(),
                        inserted: value.to_string().into_bytes(),
                        granularity: Granularity::Cell,
                    },
                    CellWrite::new(reference, CellInput::Number(value)),
                    value,
                ));
                break 'cells;
            }
        }
    }
    if planned.is_empty() {
        return Ok(None);
    }
    let mut edits = Vec::with_capacity(planned.len());
    let mut written = Vec::with_capacity(planned.len());
    for (sheet, edit, write, value) in planned {
        workbook
            .write_cells(sheet, std::slice::from_ref(&write))
            .map_err(|error| format!("write_cells on {} refused: {error}", edit.at))?;
        written.push(WrittenCell {
            sheet,
            reference: write.reference.clone(),
            value,
            at: edit.at.clone(),
        });
        edits.push(edit);
    }
    let saved = workbook
        .save()
        .map_err(|error| format!("saving the edited workbook refused: {error}"))?;
    read_cells_back(&saved, &written)?;
    Ok(Some(EditedDocument { edits, saved }))
}

/// One cell the workbook edit wrote, kept so the value can be read back out of the saved bytes.
#[derive(Debug)]
struct WrittenCell {
    /// The sheet it is on.
    sheet: u32,
    /// Its A1 text.
    reference: String,
    /// The number written into it.
    value: f64,
    /// How the report names it.
    at: String,
}

/// Reads every edited cell back out of the saved bytes, through the model.
///
/// This is what proves the value landed where it was asked to. The byte comparison beside it proves
/// the *locality* — that nothing outside one `<c>` moved — and a minimal diff cannot always show the
/// whole spelling of a number, because a common suffix absorbs whatever digits happen to agree.
/// Together they are the pair `Granularity::TextLeaf` gets from byte equality alone.
fn read_cells_back(saved: &[u8], written: &[WrittenCell]) -> Result<(), String> {
    let workbook = Workbook::open(saved)
        .map_err(|error| format!("the edited workbook does not reopen: {error}"))?;
    for WrittenCell {
        sheet,
        reference,
        value,
        at,
    } in written
    {
        let block = workbook
            .read_range(*sheet, reference)
            .map_err(|error| format!("reading {at} back refused: {error}"))?;
        match block.value(0, 0) {
            Ok(CellData::Number(read)) if read == value => {}
            other => {
                return Err(format!(
                    "{at} was written as {value} and reads back as {other:?}. The bytes moved; the \
                     value did not arrive"
                ))
            }
        }
    }
    Ok(())
}

/// A zero-based row and column as the A1 text `write_cells` takes.
fn a1(row: u32, column: u32) -> String {
    let mut letters = Vec::new();
    let mut remaining = column;
    loop {
        letters.push(b'A' + u8::try_from(remaining % 26).unwrap_or(0));
        if remaining < 26 {
            break;
        }
        remaining = remaining / 26 - 1;
    }
    letters.reverse();
    format!(
        "{}{}",
        String::from_utf8_lossy(&letters),
        row.saturating_add(1)
    )
}
