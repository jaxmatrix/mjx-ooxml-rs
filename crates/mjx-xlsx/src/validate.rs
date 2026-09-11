//! SpreadsheetML invariants — the checks [`Workbook::save`](crate::Workbook::save) runs on top of
//! the packaging ones.
//!
//! [`mjx_opc::Package::validate`] owns everything that is true of *any* OPC package: content-type
//! coverage, relationship targets, relationship-id uniqueness, and markup naming a relationship its
//! `.rels` never declares. What is left here is what only SpreadsheetML knows.
//!
//! # Two scopes, and why they differ
//!
//! `mjx-opc` draws the line and this module does not get to disagree with it (see
//! [`mjx_opc::Package::authored_xml_parts`]'s own doc comment). Restated in Excel's terms:
//!
//! * **Graph invariants** — [`WorkbookIsNotTheOfficeDocument`](SpreadsheetDefect::WorkbookIsNotTheOfficeDocument)
//!   and [`UnreachableSpreadsheetPart`](SpreadsheetDefect::UnreachableSpreadsheetPart) — are checked
//!   over the **whole package**, because they are properties of relationships and content types, not
//!   of anyone's markup, and an edit anywhere can break an edge the caller never looked at. This is
//!   the same scope `Package::validate` uses for its own relationship checks, which likewise refuse
//!   to save a container that arrived broken.
//! * **Markup invariants** — everything about the `x:sheets` list — are checked only over
//!   [`Package::authored_xml_parts`](mjx_opc::Package::authored_xml_parts), the parts whose bytes
//!   this library will write. A workbook opened and saved untouched is never faulted for markup it
//!   arrived with, and *reading* a sheet can never change whether a workbook saves.
//!
//! # The forward direction is one layer down
//!
//! "A `x:sheet` whose `r:id` no relationship declares" is **not** checked here. It is a dangling
//! relationship reference like any other, and `mjx-opc` already reports it as
//! [`PackageDefect::UndeclaredRelationshipReference`](mjx_opc::PackageDefect::UndeclaredRelationshipReference)
//! over the same set of parts; restating it would be a second, drifting implementation of one rule —
//! exactly the note `mjx_pptx::validate` carries for `p:sldId`. What is left is the direction
//! packaging cannot see: whether the relationship an entry names leads to a part of the *kind* a
//! sheet list is for, whether a sheet part is listed at all, and the two identifier spaces §18.2.19
//! requires to be unique.
//!
//! # A hyperlink and its relationship are one thing
//!
//! [`OrphanedHyperlinkRelationship`](SpreadsheetDefect::OrphanedHyperlinkRelationship) is the *other*
//! half of MJXOFF-127's two-halves rule, and the half `mjx-opc` cannot state. `mjx-opc` reports the
//! forward direction — markup naming a relationship its `.rels` never declares — and is explicit
//! that the reverse, a relationship nothing names, is legal: a `comments` relationship is found by
//! *type* and no markup ever names it, so a general "unreferenced relationship" rule would fault
//! every commented worksheet in existence.
//!
//! A **hyperlink** relationship is the exception, and narrowly so: it exists for no other purpose
//! than to be named by an `x:hyperlink@r:id`, so one that nothing names is an entry this library
//! removed without its other half. The check is therefore scoped to that one relationship type, and
//! to worksheets [`Package::authored_xml_parts`](mjx_opc::Package::authored_xml_parts) says this
//! library will write — the same asymmetry
//! [`TablePartTargetIsNotATable`](SpreadsheetDefect::TablePartTargetIsNotATable) has, and for the
//! same reason.
//!
//! # The orphan question, answered differently here than in OPC
//!
//! [`mjx_opc::Package::validate`] is explicit that an unreferenced part is legal, merely dead
//! weight, and never a defect. [`UnreachableSpreadsheetPart`](SpreadsheetDefect::UnreachableSpreadsheetPart)
//! narrows that for one family and one reason. A SpreadsheetML part is reached *only* through the
//! workbook's graph — there is no other consumer of an `xl/sharedStrings.xml` — so one the graph
//! cannot reach is not dead weight, it is a missing dependency: every `t="s"` cell in every
//! worksheet then indexes into a table nothing loads. The check is deliberately limited to the
//! `…spreadsheetml.*` content-type family, so a stray theme, image or OLE object stays legal, and
//! [`Workbook::save_unchecked`](crate::Workbook::save_unchecked) is the way to write a container
//! back exactly as it arrived regardless.

use std::collections::{HashMap, HashSet};

use mjx_ooxml_core::{FromXml, RawDocument};
use mjx_ooxml_types::namespaces::{SHARED_RELATIONSHIP_REFERENCE, SML};
use mjx_opc::{Package, PartName, TargetMode};
use mjx_sml::{CommentList, Comments};
use mjx_vml::Drawing;
use mjx_xml::fidelity;

use crate::error::XlsxError;
use crate::nav;
use crate::parts::{
    SheetKind, CONTENT_TYPE_CHARTSHEET, CONTENT_TYPE_DIALOGSHEET, CONTENT_TYPE_EXTERNAL_LINK,
    CONTENT_TYPE_PIVOT_CACHE_DEFINITION, CONTENT_TYPE_TABLE, CONTENT_TYPE_WORKSHEET, REL_COMMENTS,
    REL_HYPERLINK, REL_IMAGE, REL_OFFICE_DOCUMENT, REL_PRINTER_SETTINGS, REL_VML_DRAWING,
};
use crate::worksheet::comments;

/// The content-type prefix every SpreadsheetML part shares.
///
/// Used by [`SpreadsheetDefect::UnreachableSpreadsheetPart`]'s check to pick out the family whose
/// only consumer is the workbook graph. Matching on the prefix rather than on [`PartKind`]'s own
/// list is deliberate, and MJXOFF-133 did not change that: the enum now names every §12.3 part
/// type, but the prefix still covers a `.xlsm`'s macro-enabled workbook, a producer's own
/// `spreadsheetml.*` extension part, and anything a later edition of the specification adds — none
/// of which this crate classifies and all of which only the workbook can reach.
const SPREADSHEETML_CONTENT_TYPE_PREFIX: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.";

/// One broken SpreadsheetML invariant, named down to the part and the identifier at fault.
///
/// Returned — wrapped in [`XlsxError::InvalidWorkbook`](crate::XlsxError::InvalidWorkbook) — by
/// [`Workbook::validate`](crate::Workbook::validate) and [`Workbook::save`](crate::Workbook::save).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SpreadsheetDefect {
    /// The package-root `officeDocument` relationship no longer leads to the workbook part.
    ///
    /// ECMA-376 Part 1 §12.3.23: *"A package shall contain exactly one Workbook part, and that part
    /// shall be the target of a relationship in the package-relationship item."* A consumer finds
    /// the workbook through that one edge and nowhere else, so a package whose root relationship
    /// points at something else has no workbook at all, however much SpreadsheetML it contains.
    #[error(
        "the package-root officeDocument relationship targets {office_document_target:?}, not the \
         workbook part {workbook_part}"
    )]
    WorkbookIsNotTheOfficeDocument {
        /// The workbook part this [`Workbook`](crate::Workbook) was opened on.
        workbook_part: String,
        /// The `Target` the root relationship now names, exactly as written.
        office_document_target: String,
    },

    /// A SpreadsheetML part the container holds that no chain of relationships from the package root
    /// reaches.
    ///
    /// See this module's own documentation for why this one family is treated differently from
    /// `mjx-opc`'s "an unreferenced part is not a defect".
    #[error(
        "{part} is a SpreadsheetML part (content type {content_type}) that no relationship chain \
         from the package root reaches"
    )]
    UnreachableSpreadsheetPart {
        /// The unreachable part.
        part: String,
        /// Its content type.
        content_type: String,
    },

    /// A `x:pivotCache` or `x:externalReference` entry of the workbook part naming a relationship
    /// that leads to a part of the wrong kind.
    ///
    /// The direction packaging cannot see, for the two lists in `CT_Workbook`'s sequence that point
    /// *outward* at parts. A `pivotCaches/pivotCache@r:id` that leads to a shared string table is a
    /// workbook whose pivot tables will not load, and §12.3.12 is explicit about what the target of
    /// that relationship is; the forward direction — an `r:id` no relationship declares — is
    /// [`PackageDefect::UndeclaredRelationshipReference`](mjx_opc::PackageDefect::UndeclaredRelationshipReference),
    /// already reported over the same set of parts, and restating it here would be a second,
    /// drifting implementation of one rule.
    ///
    /// The parts themselves are preserved and not modelled — see [`crate::preserve`] — which is
    /// precisely why the *edges* to them are checked: nothing else in this library would ever
    /// notice one going stale.
    #[error(
        "{part}: {element} names relationship {relationship_id}, which targets {target_part} of \
         type {actual_content_type} — not {expected_content_type}"
    )]
    WorkbookReferenceTargetIsWrongKind {
        /// The part holding the list (the workbook part).
        part: String,
        /// The wire name of the entry element — `pivotCache` or `externalReference`.
        element: &'static str,
        /// The relationship the entry names.
        relationship_id: String,
        /// The part that relationship targets.
        target_part: String,
        /// The content type that target actually has.
        actual_content_type: String,
        /// The content type §12.3 requires for that entry's target.
        expected_content_type: &'static str,
    },

    /// A `x:sheet` entry naming a relationship that leads to something that is not a sheet.
    #[error(
        "{part}: x:sheets entry names relationship {relationship_id}, which targets {target_part} \
         of type {actual_content_type} — not a worksheet, chartsheet or dialogsheet"
    )]
    SheetEntryTargetIsNotASheet {
        /// The part holding the list (the workbook part).
        part: String,
        /// The relationship the entry names.
        relationship_id: String,
        /// The part that relationship targets.
        target_part: String,
        /// The content type that target actually has.
        actual_content_type: String,
    },

    /// A relationship leading to a sheet part that `x:sheets` never names — a sheet the workbook
    /// carries and no consumer will ever show.
    #[error("{part}: relationship {relationship_id} targets {target_part}, which x:sheets does not list")]
    UnlistedSheetRelationship {
        /// The part holding the list (the workbook part).
        part: String,
        /// The relationship no entry names.
        relationship_id: String,
        /// The part it targets.
        target_part: String,
    },

    /// Two `x:sheet` entries naming the same relationship — one sheet part listed as two tabs.
    #[error("{part}: x:sheets names relationship {relationship_id} more than once")]
    DuplicateSheetReference {
        /// The part holding the list.
        part: String,
        /// The relationship named twice.
        relationship_id: String,
    },

    /// Two `x:sheet` entries sharing a `@sheetId`.
    ///
    /// ECMA-376 Part 1 §18.2.19: *"Specifies the internal identifier for the sheet. This identifier
    /// shall be unique."*
    #[error("{part}: x:sheets has more than one entry with sheetId {sheet_id}")]
    DuplicateSheetId {
        /// The part holding the list.
        part: String,
        /// The repeated `@sheetId`.
        sheet_id: String,
    },

    /// Two `x:sheet` entries sharing a `@name`.
    ///
    /// ECMA-376 Part 1 §18.2.19: *"Specifies the name of the sheet. This name shall be unique."*
    /// Compared exactly, because that is the whole of what the clause says; a case-insensitive
    /// comparison would be a rule this project invented rather than one it read.
    #[error("{part}: x:sheets has more than one entry named {name:?}")]
    DuplicateSheetName {
        /// The part holding the list.
        part: String,
        /// The repeated `@name`.
        name: String,
    },

    /// A `x:tablePart` naming a relationship that leads to something that is not a table part.
    ///
    /// The direction packaging cannot see, and the table cluster's counterpart of
    /// [`SheetEntryTargetIsNotASheet`](Self::SheetEntryTargetIsNotASheet). A `tablePart@r:id` that
    /// names **no** relationship at all is not this: that is a dangling relationship reference like
    /// any other, and `mjx-opc` reports it as
    /// [`UndeclaredRelationshipReference`](mjx_opc::PackageDefect::UndeclaredRelationshipReference)
    /// over the same set of parts. Restating it here would be a second, drifting implementation of
    /// one rule.
    #[error(
        "{part}: x:tableParts names relationship {relationship_id}, which targets {target_part} of \
         type {actual_content_type} — not a table definition part"
    )]
    TablePartTargetIsNotATable {
        /// The worksheet part holding the list.
        part: String,
        /// The relationship the entry names.
        relationship_id: String,
        /// The part that relationship targets.
        target_part: String,
        /// The content type that target actually has.
        actual_content_type: String,
    },

    /// Two tables in the workbook sharing a `@id`.
    ///
    /// ECMA-376 Part 1 §18.5.1.2: *"A non zero integer representing the unique identifier for this
    /// table. Each table in the workbook shall have a unique id."* The scope is the **workbook**,
    /// not the sheet, so two tables on different sheets collide exactly as two on one sheet do —
    /// which is why [`Workbook::add_table`](crate::Workbook::add_table) allocates from every table
    /// part in the package rather than from the sheet's own.
    ///
    /// Nothing renumbers a table to make this go away: a table's id is what other records name it
    /// by, and moving one silently repoints whatever named it.
    /// [`Workbook::save_unchecked`](crate::Workbook::save_unchecked) writes a container back exactly
    /// as it arrived.
    #[error("{first_part} and {second_part} are both tables with id {table_id}, which §18.5.1.2 requires to be unique in the workbook")]
    DuplicateTableId {
        /// The first part carrying the id, in container order.
        first_part: String,
        /// The second.
        second_part: String,
        /// The repeated `@id`.
        table_id: String,
    },

    /// A `hyperlink` relationship on a worksheet **this library will write** that no `x:hyperlink`
    /// in that worksheet names.
    ///
    /// The half of MJXOFF-127's two-halves rule that packaging cannot state; see this module's own
    /// documentation for why a hyperlink relationship is the one kind an orphan is a defect for.
    ///
    /// **This is not a repair instruction.** Nothing removes the relationship to make the defect go
    /// away: [`Workbook::remove_cell_hyperlink`](crate::Workbook::remove_cell_hyperlink) removes
    /// both halves together and this reports when something did not, which is a different act.
    /// [`Workbook::save_unchecked`](crate::Workbook::save_unchecked) writes the container back
    /// regardless.
    #[error(
        "{part}: relationship {relationship_id} is a hyperlink to {target:?} that no x:hyperlink in \
         the sheet names"
    )]
    OrphanedHyperlinkRelationship {
        /// The worksheet part whose `.rels` declares it.
        part: String,
        /// The relationship nothing names.
        relationship_id: String,
        /// Its `Target`, exactly as written — never resolved and never fetched.
        target: String,
    },

    /// Two tables in the workbook sharing a `@displayName`.
    ///
    /// ECMA-376 Part 1 §18.5.1.2: *"This name shall not have any spaces in it, and it shall be
    /// unique amongst all other displayNames and definedNames in the workbook."* A formula
    /// references a table by this name, so two tables answering to it make every such formula
    /// ambiguous.
    ///
    /// **The check is narrower than the clause**, deliberately: it compares table display names
    /// against each other and **not** against the workbook's defined names, because a defined name
    /// and a table name colliding is a third thing to reason about (`_FilterDatabase` and the other
    /// built-in names among them) and reporting it wrongly would be worse than not reporting it.
    /// The gap is written down in the guide rather than hidden.
    #[error("{first_part} and {second_part} are both tables with displayName {display_name:?}, which §18.5.1.2 requires to be unique in the workbook")]
    DuplicateTableDisplayName {
        /// The first part carrying the name, in container order.
        first_part: String,
        /// The second.
        second_part: String,
        /// The repeated `@displayName`.
        display_name: String,
    },

    /// A comment whose box the sheet's legacy VML drawing does not hold (MJXOFF-114).
    ///
    /// Half a comment. In the Transitional flavour a cell comment is two parts — the text in
    /// `xl/commentsN.xml`, the pop-up box as a `v:shape` in `xl/drawings/vmlDrawingN.vml` — and
    /// neither half means anything alone. Excel opens a workbook with a comment and no box, reports
    /// it as damaged and repairs it by dropping the comment.
    ///
    /// This is the direction packaging cannot see at all: `mjx-opc` checks that a named relationship
    /// is declared, and *neither* half of a comment names the other by relationship. A box is
    /// matched to its comment by the comment's own `@shapeId`, or failing that by the `x:Row` and
    /// `x:Column` the shape states — both things the file says, neither inferred. See
    /// [`Workbook::with_vml_shape_for_comment`](crate::Workbook::with_vml_shape_for_comment).
    #[error(
        "{sheet_part}: the comment on {cell} in {comments_part} has no box — the sheet's legacy VML \
         drawing holds no shape for it"
    )]
    CommentWithoutABox {
        /// The worksheet both halves hang off.
        sheet_part: String,
        /// The comments part holding the text.
        comments_part: String,
        /// The cell the comment claims, from `x:comment@ref`.
        cell: String,
    },

    /// A comment box in the sheet's legacy VML drawing that no comment claims (MJXOFF-114).
    ///
    /// The other half of [`CommentWithoutABox`](Self::CommentWithoutABox), and the one a careless
    /// delete leaves behind: a `v:shape` whose `x:ClientData` says `ObjectType="Note"` is a comment's
    /// pop-up box and is drawn as one, so a box with no text is an empty tooltip on a cell nobody
    /// commented on.
    ///
    /// Only `Note` shapes are faulted. A `v:shape` drawing a form control or an OLE fallback lives in
    /// the same part and is claimed from a different list entirely.
    #[error(
        "{vml_part}: the comment box for {cell} (shape {shape}) is claimed by no comment in \
         {sheet_part}'s comments part"
    )]
    CommentBoxWithoutAComment {
        /// The worksheet both halves hang off.
        sheet_part: String,
        /// The legacy VML drawing part holding the box.
        vml_part: String,
        /// The shape's `@id`, exactly as the file wrote it.
        shape: String,
        /// The cell the box states it is attached to, or `?` when it states none.
        cell: String,
    },

    /// A sheet's `x:pageSetup` or `x:picture` naming a relationship of the wrong **type**.
    ///
    /// `CT_PageSetup`'s and `CT_CsPageSetup`'s `r:id` reaches a **printer settings** part
    /// (ECMA-376 Part 1 §15.2.13) and `CT_SheetBackgroundPicture`'s reaches an **image** part
    /// (§15.2.14). An `r:id` naming a declared relationship of some other type resolves to a part
    /// that is not the one the element means, and every consumer of it — including
    /// [`Workbook::sheet_printer_settings`](crate::Workbook::sheet_printer_settings) — is then
    /// answering with the wrong part rather than with nothing.
    ///
    /// # What this is *not*
    ///
    /// It is not the dangling reference. An `r:id` naming a relationship the `.rels` does not
    /// declare at all is `mjx_opc`'s
    /// [`UndeclaredRelationshipReference`](mjx_opc::PackageDefect::UndeclaredRelationshipReference),
    /// which covers every attribute in the relationship-reference namespace and needs no help from
    /// this crate. **This is the case that check cannot see**: the id is declared, so the package is
    /// internally consistent, and only a reader that knows what a `pageSetup` means can tell that it
    /// points at the wrong kind of thing.
    ///
    /// Reported only for a sheet part **this library will write** — the same scope
    /// [`OrphanedHyperlinkRelationship`](Self::OrphanedHyperlinkRelationship) has, and for the same
    /// reason: a workbook somebody else wrote is preserved, not corrected.
    #[error(
        "{part}: x:{element}/@r:id names relationship {relationship_id}, whose type is \
         {found_type:?} rather than {expected_type:?}"
    )]
    SheetReferenceHasTheWrongRelationshipType {
        /// The sheet part whose markup names it.
        part: String,
        /// The element carrying the reference: `pageSetup` or `picture`.
        element: &'static str,
        /// The relationship the element names.
        relationship_id: String,
        /// The relationship type the element requires.
        expected_type: &'static str,
        /// The type the `.rels` actually declares for it.
        found_type: String,
    },
}

/// Checks every SpreadsheetML invariant, in a deterministic order: the workbook edge, then
/// reachability, then the sheet list.
///
/// A pure function of the package: it takes `&Package`, parses nothing that is not already parsed
/// except the authored bytes it is about to check, and changes nothing.
pub(crate) fn check(package: &Package, workbook_part: &PartName) -> Result<(), XlsxError> {
    check_office_document_edge(package, workbook_part)?;
    check_spreadsheet_parts_are_reachable(package)?;
    check_sheet_list(package, workbook_part)?;
    check_workbook_reference_targets(package, workbook_part)?;
    check_table_part_targets(package)?;
    check_table_identity(package)?;
    check_hyperlink_relationships(package)?;
    check_sheet_print_references(package)?;
    check_comment_halves(package)?;
    Ok(())
}

/// A part's fidelity tree, borrowed from the package's cache when there is one and parsed for the
/// length of the call when there is not.
///
/// The two halves exist because an *edited* part has no bytes to parse — `mjx-opc` holds it as a
/// tree — while a part read off disk has bytes and no tree, and a check that only handled one of
/// them would silently skip the other. Skipping is the failure mode this whole file is written
/// against.
enum PartTree<'a> {
    Cached(&'a RawDocument),
    // Boxed because a `RawDocument` is 216 bytes against a reference's 8, and this value is only
    // ever a short-lived local — the allocation is one per part checked, and the alternative is an
    // enum every caller moves 216 bytes of.
    Parsed(Box<RawDocument>),
}

impl PartTree<'_> {
    fn get(&self) -> &RawDocument {
        match self {
            Self::Cached(tree) => tree,
            Self::Parsed(tree) => tree,
        }
    }
}

/// `part`'s tree, however the package is holding it, or `None` when it is absent or will not parse.
///
/// Well-formedness of an authored part is `mjx-opc`'s defect to report; if it will not parse there
/// is nothing here to check.
fn part_tree<'a>(package: &'a Package, part: &PartName) -> Option<PartTree<'a>> {
    if let Some((_, entry)) = package.authored_xml_parts().find(|(name, _)| name == part) {
        if let Some(tree) = entry.tree() {
            return Some(PartTree::Cached(tree));
        }
        let bytes = entry.bytes()?;
        return fidelity::parse(bytes)
            .ok()
            .map(|tree| PartTree::Parsed(Box::new(tree)));
    }
    let bytes = package.part_payload(part)?;
    fidelity::parse(&bytes)
        .ok()
        .map(|tree| PartTree::Parsed(Box::new(tree)))
}

/// The two lists in `CT_Workbook`'s sequence that point outward at parts —
/// `x:pivotCaches/pivotCache` (rank 12) and `x:externalReferences/externalReference` (rank 7) —
/// each name a relationship leading to a part of the kind §12.3 requires.
///
/// Scoped to [`Package::authored_xml_parts`](mjx_opc::Package::authored_xml_parts), like every other
/// markup check here: a workbook opened and saved untouched is never faulted for markup it arrived
/// with. `check_table_part_targets` is the same shape for a worksheet's `tableParts`, and this is
/// deliberately written as one loop over two entry kinds rather than as two nearly identical
/// functions.
fn check_workbook_reference_targets(
    package: &Package,
    workbook_part: &PartName,
) -> Result<(), XlsxError> {
    /// One list in `CT_Workbook`'s sequence: the wrapper, the entry, and what the entry must reach.
    const LISTS: &[(&str, &str, &str)] = &[
        (
            "pivotCaches",
            "pivotCache",
            CONTENT_TYPE_PIVOT_CACHE_DEFINITION,
        ),
        (
            "externalReferences",
            "externalReference",
            CONTENT_TYPE_EXTERNAL_LINK,
        ),
    ];

    if !package
        .authored_xml_parts()
        .any(|(part, _)| part == *workbook_part)
    {
        return Ok(()); // Container bytes: not ours to fault. See this module's own docs.
    }
    let Some(tree) = part_tree(package, workbook_part) else {
        return Ok(());
    };
    let tree = tree.get();
    let interner = &tree.interner;
    let Some(prefix) = nav::namespace_prefix(&tree.root, interner, SHARED_RELATIONSHIP_REFERENCE)
    else {
        // The part binds the relationship-reference namespace nowhere, so no entry in it can carry
        // an `r:id` at all. Both types declare one required; that is a schema defect the gate
        // reports and this check does not restate.
        return Ok(());
    };
    let Some(relationships) = package.relationships_for(Some(workbook_part)) else {
        return Ok(());
    };

    for (wrapper, element, expected_content_type) in LISTS {
        let Some(list) = nav::child(&tree.root, interner, SML, wrapper) else {
            continue;
        };
        for entry in nav::children(list, interner, SML, element) {
            let Some(reference) = nav::prefixed_attr_value(entry, interner, prefix, "id") else {
                continue;
            };
            let reference = reference?;
            let Some(rel) = relationships.by_id(&reference) else {
                continue; // Undeclared: reported one layer down, as a dangling reference.
            };
            if rel.mode == TargetMode::External {
                continue; // An external target has no content type to compare against.
            }
            let Ok(target) = nav::resolve_target(workbook_part, &rel.target) else {
                continue; // `mjx-opc`'s defect to report.
            };
            let Some(actual) = package.content_type_of(&target) else {
                continue; // A target with no content type is `mjx-opc`'s defect to report.
            };
            if actual == *expected_content_type {
                continue;
            }
            return Err(SpreadsheetDefect::WorkbookReferenceTargetIsWrongKind {
                part: workbook_part.as_str().to_owned(),
                element,
                relationship_id: reference,
                target_part: target.as_str().to_owned(),
                actual_content_type: actual.to_owned(),
                expected_content_type,
            }
            .into());
        }
    }
    Ok(())
}

/// Every `x:tablePart` of every worksheet **this library will write** names a relationship that
/// leads to a table definition part.
///
/// Scoped to [`Package::authored_xml_parts`](mjx_opc::Package::authored_xml_parts) for the reason
/// this module's own documentation gives: a workbook opened and saved untouched is never faulted for
/// markup it arrived with.
///
/// The *other* direction — a `tablePart@r:id` naming a relationship that does not exist — is
/// deliberately absent: it is a dangling relationship reference like any other, and
/// [`Package::validate`](mjx_opc::Package::validate) already reports it as
/// `UndeclaredRelationshipReference` over exactly this set of parts.
fn check_table_part_targets(package: &Package) -> Result<(), XlsxError> {
    let worksheets: Vec<PartName> = package
        .authored_xml_parts()
        .map(|(part, _)| part)
        .filter(|part| package.content_type_of(part) == Some(CONTENT_TYPE_WORKSHEET))
        .collect();

    for part in worksheets {
        let Some(tree) = part_tree(package, &part) else {
            continue;
        };
        let tree = tree.get();
        let interner = &tree.interner;
        let Some(list) = nav::child(&tree.root, interner, SML, "tableParts") else {
            continue;
        };
        let Some(prefix) =
            nav::namespace_prefix(&tree.root, interner, SHARED_RELATIONSHIP_REFERENCE)
        else {
            // The part binds the relationship-reference namespace nowhere, so no `tablePart` in it
            // can carry an `r:id` at all. `CT_TablePart` declares one required; that is a schema
            // defect, which the schema gate reports and this check does not restate.
            continue;
        };
        let Some(relationships) = package.relationships_for(Some(&part)) else {
            continue;
        };
        for entry in nav::children(list, interner, SML, "tablePart") {
            let Some(reference) = nav::prefixed_attr_value(entry, interner, prefix, "id") else {
                continue;
            };
            let reference = reference?;
            let Some(rel) = relationships.by_id(&reference) else {
                continue; // Undeclared: reported one layer down, as a dangling reference.
            };
            if rel.mode == TargetMode::External {
                continue; // An external target has no content type to compare against.
            }
            let Ok(target) = nav::resolve_target(&part, &rel.target) else {
                continue; // `mjx-opc`'s defect to report.
            };
            let Some(actual) = package.content_type_of(&target) else {
                continue; // A target with no content type is `mjx-opc`'s defect to report.
            };
            if actual == CONTENT_TYPE_TABLE {
                continue;
            }
            return Err(SpreadsheetDefect::TablePartTargetIsNotATable {
                part: part.as_str().to_owned(),
                relationship_id: reference,
                target_part: target.as_str().to_owned(),
                actual_content_type: actual.to_owned(),
            }
            .into());
        }
    }
    Ok(())
}

/// Every `hyperlink` relationship on a worksheet **this library will write** is named by an
/// `x:hyperlink` in that worksheet.
///
/// The reverse direction of `mjx-opc`'s dangling-reference check, restricted to the one relationship
/// type an orphan means something for. See this module's own documentation.
///
/// # Why this walks the tree rather than reading a `WorksheetPart`
///
/// [`mjx_sml::WorksheetPart::read_document`] builds MJXOFF-95's packed cell store, which for a
/// 300,000-cell sheet is the most expensive thing this crate can do. A check that needs one
/// element's attributes must not pay for that, and every other check in this file walks the same
/// [`RawDocument`] through [`crate::nav`] for the same reason.
///
/// The removal rule in `crate::worksheet::hyperlinks` asks the same question of a model it is
/// already holding, which is a different starting point and not a second implementation of this one:
/// this decides what a **saved package** says, that decides whether a relationship still has a user
/// during one edit.
fn check_hyperlink_relationships(package: &Package) -> Result<(), XlsxError> {
    let worksheets: Vec<PartName> = package
        .authored_xml_parts()
        .map(|(part, _)| part)
        .filter(|part| package.content_type_of(part) == Some(CONTENT_TYPE_WORKSHEET))
        .collect();

    for part in worksheets {
        let Some(relationships) = package.relationships_for(Some(&part)) else {
            continue;
        };
        let hyperlinks: Vec<(String, String)> = relationships
            .iter()
            .filter(|rel| rel.rel_type == REL_HYPERLINK)
            .map(|rel| (rel.id.clone(), rel.target.clone()))
            .collect();
        if hyperlinks.is_empty() {
            continue;
        }
        let Some(tree) = part_tree(package, &part) else {
            continue;
        };
        let tree = tree.get();
        let interner = &tree.interner;
        // Every `r:id` the sheet's `x:hyperlink` entries name. A part that binds the
        // relationship-reference namespace nowhere can spell no `r:id` at all, so every hyperlink
        // relationship on it is an orphan — which is the answer an empty set gives.
        let mut named: HashSet<String> = HashSet::new();
        if let Some(prefix) =
            nav::namespace_prefix(&tree.root, interner, SHARED_RELATIONSHIP_REFERENCE)
        {
            if let Some(list) = nav::child(&tree.root, interner, SML, "hyperlinks") {
                for entry in nav::children(list, interner, SML, "hyperlink") {
                    if let Some(reference) = nav::prefixed_attr_value(entry, interner, prefix, "id")
                    {
                        named.insert(reference?);
                    }
                }
            }
        }
        for (relationship_id, target) in hyperlinks {
            if named.contains(&relationship_id) {
                continue;
            }
            return Err(SpreadsheetDefect::OrphanedHyperlinkRelationship {
                part: part.as_str().to_owned(),
                relationship_id,
                target,
            }
            .into());
        }
    }
    Ok(())
}

/// Every `x:pageSetup` and `x:picture` on a sheet **this library will write** names a relationship
/// of the right type.
///
/// The half of "the reference and the part it names are one thing" that `mjx-opc` cannot answer: it
/// checks that a named relationship is *declared*, and this checks that the declared one is the
/// *kind* the element means. See
/// [`SpreadsheetDefect::SheetReferenceHasTheWrongRelationshipType`].
///
/// Walks the same [`RawDocument`] every other check in this file walks, through [`crate::nav`], for
/// the reason [`check_hyperlink_relationships`] states: reading a `WorksheetPart` would build
/// MJXOFF-95's packed cell store, and a check that needs two attributes must not pay for that.
///
/// All three sheet content types are swept, because all three declare a `pageSetup` — the
/// chartsheet's is `CT_CsPageSetup`, whose `r:id` means exactly the same thing.
fn check_sheet_print_references(package: &Package) -> Result<(), XlsxError> {
    const SHEET_CONTENT_TYPES: [&str; 3] = [
        CONTENT_TYPE_WORKSHEET,
        CONTENT_TYPE_CHARTSHEET,
        CONTENT_TYPE_DIALOGSHEET,
    ];
    // The two slots that carry an `r:id`, and the relationship type each one means.
    const REFERENCES: [(&str, &str); 2] =
        [("pageSetup", REL_PRINTER_SETTINGS), ("picture", REL_IMAGE)];

    let sheets: Vec<PartName> = package
        .authored_xml_parts()
        .map(|(part, _)| part)
        .filter(|part| {
            package
                .content_type_of(part)
                .is_some_and(|content_type| SHEET_CONTENT_TYPES.contains(&content_type))
        })
        .collect();

    for part in sheets {
        let Some(relationships) = package.relationships_for(Some(&part)) else {
            continue;
        };
        let Some(tree) = part_tree(package, &part) else {
            continue;
        };
        let tree = tree.get();
        let interner = &tree.interner;
        // A part that binds the relationship-reference namespace nowhere can spell no `r:id` at all.
        let Some(prefix) =
            nav::namespace_prefix(&tree.root, interner, SHARED_RELATIONSHIP_REFERENCE)
        else {
            continue;
        };
        for (element, expected_type) in REFERENCES {
            let Some(node) = nav::child(&tree.root, interner, SML, element) else {
                continue;
            };
            let Some(reference) = nav::prefixed_attr_value(node, interner, prefix, "id") else {
                continue;
            };
            let relationship_id = reference?;
            // An id that is declared nowhere is `mjx_opc`'s to report, not this check's: saying so
            // twice would make one defect two.
            let Some(relationship) = relationships.by_id(&relationship_id) else {
                continue;
            };
            if relationship.rel_type == expected_type {
                continue;
            }
            return Err(
                SpreadsheetDefect::SheetReferenceHasTheWrongRelationshipType {
                    part: part.as_str().to_owned(),
                    element,
                    relationship_id,
                    expected_type,
                    found_type: relationship.rel_type.clone(),
                }
                .into(),
            );
        }
    }
    Ok(())
}

/// No table **this library wrote** shares its `@id` or its `@displayName` with another table in the
/// workbook.
///
/// The scope is deliberately asymmetric, and it is the scope that makes the check both useful and
/// safe: the *offending* table has to be one this library will write, while the table it collides
/// with may be any table in the package. A workbook that arrived with two tables sharing an id
/// therefore still saves — its markup is not ours to fault — while a table
/// [`Workbook::add_table`](crate::Workbook::add_table) put there colliding with one on another sheet
/// is caught.
///
/// §18.5.1.2 requires both identifiers to be unique across the **workbook**, not the sheet.
fn check_table_identity(package: &Package) -> Result<(), XlsxError> {
    let authored: HashSet<String> = package
        .authored_xml_parts()
        .map(|(part, _)| part.as_str().to_owned())
        .collect();

    // Every table part in the package, in container order, with the two identifiers §18.5.1.2 makes
    // unique. `None` for either means the attribute is absent or will not decode, which is a schema
    // defect the gate reports rather than a collision.
    struct TableIdentity {
        part: String,
        authored: bool,
        id: Option<String>,
        display_name: Option<String>,
    }
    let mut tables: Vec<TableIdentity> = Vec::new();
    for part in package.part_names() {
        if package.content_type_of(&part) != Some(CONTENT_TYPE_TABLE) {
            continue;
        }
        let Some(tree) = part_tree(package, &part) else {
            continue;
        };
        let tree = tree.get();
        let interner = &tree.interner;
        if !nav::name_is(&tree.root.name, interner, SML, "table") {
            continue; // Registered as a table and rooted at something else: not this check's.
        }
        tables.push(TableIdentity {
            authored: authored.contains(part.as_str()),
            part: part.as_str().to_owned(),
            id: nav::attr_value(&tree.root, interner, "id").and_then(Result::ok),
            display_name: nav::attr_value(&tree.root, interner, "displayName").and_then(Result::ok),
        });
    }

    for (position, table) in tables.iter().enumerate() {
        if !table.authored {
            continue;
        }
        for (other_position, other) in tables.iter().enumerate() {
            if position == other_position {
                continue;
            }
            // The pair is named in container order, so the message does not depend on which of the
            // two happened to be the authored one.
            let (first, second) = if position < other_position {
                (&table.part, &other.part)
            } else {
                (&other.part, &table.part)
            };
            if let (Some(id), Some(other_id)) = (&table.id, &other.id) {
                if id == other_id {
                    return Err(SpreadsheetDefect::DuplicateTableId {
                        first_part: first.clone(),
                        second_part: second.clone(),
                        table_id: id.clone(),
                    }
                    .into());
                }
            }
            if let (Some(name), Some(other_name)) = (&table.display_name, &other.display_name) {
                if name == other_name {
                    return Err(SpreadsheetDefect::DuplicateTableDisplayName {
                        first_part: first.clone(),
                        second_part: second.clone(),
                        display_name: name.clone(),
                    }
                    .into());
                }
            }
        }
    }
    Ok(())
}

/// The package-root `officeDocument` relationship still leads to the workbook part.
fn check_office_document_edge(
    package: &Package,
    workbook_part: &PartName,
) -> Result<(), XlsxError> {
    let root = package
        .relationships_for(None)
        .ok_or(XlsxError::MissingOfficeDocument)?;
    let rel = root
        .by_type(REL_OFFICE_DOCUMENT)
        .next()
        .ok_or(XlsxError::MissingOfficeDocument)?;
    let names_the_workbook = rel.mode == TargetMode::Internal
        && nav::resolve_from_root(&rel.target).is_ok_and(|part| part == *workbook_part);
    if names_the_workbook {
        return Ok(());
    }
    Err(SpreadsheetDefect::WorkbookIsNotTheOfficeDocument {
        workbook_part: workbook_part.as_str().to_owned(),
        office_document_target: rel.target.clone(),
    }
    .into())
}

/// Every `…spreadsheetml.*` part is reachable from the package root by a chain of internal
/// relationships.
///
/// One breadth-first walk over the relationship graph, then one pass over the parts:
/// `O(parts + relationships)`, with no markup parsed at all.
fn check_spreadsheet_parts_are_reachable(package: &Package) -> Result<(), XlsxError> {
    let mut reached: HashSet<String> = HashSet::new();
    let mut frontier: Vec<Option<PartName>> = vec![None]; // `None` is the package root

    while let Some(source) = frontier.pop() {
        let Some(rels) = package.relationships_for(source.as_ref()) else {
            continue;
        };
        for rel in rels.iter() {
            if rel.mode == TargetMode::External {
                continue;
            }
            let resolved = match &source {
                Some(part) => nav::resolve_target(part, &rel.target),
                None => nav::resolve_from_root(&rel.target),
            };
            // An unresolvable or missing target is `mjx-opc`'s defect to report, and it reports it
            // before this pass runs; here it simply reaches nothing.
            let Ok(target) = resolved else { continue };
            if reached.insert(target.as_str().to_owned()) {
                frontier.push(Some(target));
            }
        }
    }

    for part in package.part_names() {
        let Some(content_type) = package.content_type_of(&part) else {
            continue; // `Package::validate` reports a part with no content type.
        };
        if !content_type.starts_with(SPREADSHEETML_CONTENT_TYPE_PREFIX) {
            continue;
        }
        if reached.contains(part.as_str()) {
            continue;
        }
        return Err(SpreadsheetDefect::UnreachableSpreadsheetPart {
            part: part.as_str().to_owned(),
            content_type: content_type.to_owned(),
        }
        .into());
    }
    Ok(())
}

/// A cell comment's two halves are both present, in both directions — MJXOFF-114's two-halves rule.
///
/// # Why this needs a validator at all
///
/// A worksheet table's four things agree because the sheet's `x:tableParts` names the part; a
/// drawing's six agree because `x:drawing@r:id` does. **A comment names nothing.** `CT_Worksheet`
/// has no `comments` slot: the comments part is found by relationship *type* alone, and this
/// module's own header records the consequence — a general "unreferenced relationship" rule "would
/// fault every commented worksheet in existence". So the only thing that can notice half a comment
/// is a rule that opens both parts and matches them, which is this.
///
/// # Scope: either half authored, not both
///
/// Every other markup check here is scoped to
/// [`Package::authored_xml_parts`](mjx_opc::Package::authored_xml_parts). This one asks whether
/// **any** of the three parts involved — the worksheet, its comments part, its VML drawing — is
/// authored, because that is what "we touched this comment" means. Deleting a box from a producer's
/// VML while leaving its producer-written comments part alone authors exactly one of the three, and
/// scoping to the comments part alone would let that pass. A workbook opened and saved with nothing
/// touched is still never faulted: none of the three is authored then.
///
/// The `.vml` content type is in `mjx-opc`'s `XML_CONTENT_TYPES_WITHOUT_SUFFIX`, so an edited VML
/// part really does appear in that set — which is what makes the widened scope work rather than
/// merely sound good.
///
/// # One implementation of the pairing rule
///
/// The comment-to-box match is [`crate::worksheet::comments::comment_box_shape`], the same function
/// `Workbook::sheet_comments` and `Workbook::with_vml_shape_for_comment` call. A second copy here
/// would be free to agree with itself while disagreeing with the reader — the shape of false green
/// this project keeps finding.
fn check_comment_halves(package: &Package) -> Result<(), XlsxError> {
    let authored: HashSet<String> = package
        .authored_xml_parts()
        .map(|(part, _)| part.as_str().to_owned())
        .collect();
    if authored.is_empty() {
        return Ok(());
    }
    let worksheets: Vec<PartName> = package
        .part_names()
        .filter(|part| package.content_type_of(part) == Some(CONTENT_TYPE_WORKSHEET))
        .collect();

    for sheet_part in worksheets {
        let Some(relationships) = package.relationships_for(Some(&sheet_part)) else {
            continue;
        };
        let related = |rel_type: &str| -> Option<PartName> {
            relationships
                .iter()
                .find(|relationship| relationship.rel_type == rel_type)
                .and_then(|relationship| {
                    nav::resolve_target(&sheet_part, &relationship.target).ok()
                })
        };
        let Some(comments_part) = related(REL_COMMENTS) else {
            continue;
        };
        let vml_part = related(REL_VML_DRAWING);
        let touched = authored.contains(sheet_part.as_str())
            || authored.contains(comments_part.as_str())
            || vml_part
                .as_ref()
                .is_some_and(|part| authored.contains(part.as_str()));
        if !touched {
            continue;
        }

        let Some(comments_tree) = part_tree(package, &comments_part) else {
            continue;
        };
        let comments_tree = comments_tree.get();
        let Ok(comments) = Comments::from_xml(&comments_tree.root, &comments_tree.interner) else {
            continue;
        };
        // A sheet with a comments part and no VML drawing at all: every comment in it is boxless,
        // and the first one names the defect.
        let vml_tree = vml_part.as_ref().and_then(|part| part_tree(package, part));
        let drawing = vml_tree.as_ref().and_then(|tree| {
            let tree = tree.get();
            Drawing::from_xml(&tree.root, &tree.interner)
                .ok()
                .map(|drawing| (drawing, &tree.interner))
        });

        let mut claimed: Vec<String> = Vec::new();
        for comment in comments.list().into_iter().flat_map(CommentList::comments) {
            let Ok(cell) = comment.cell(&comments_tree.interner) else {
                continue;
            };
            let shape_id = comment.shape_id(&comments_tree.interner).ok().flatten();
            let found = drawing.as_ref().and_then(|(drawing, interner)| {
                comments::comment_box_shape(drawing, interner, cell, shape_id)
                    .map(|shape| shape.identifier(interner).unwrap_or_default())
            });
            match found {
                Some(identifier) => claimed.push(identifier),
                None => {
                    return Err(SpreadsheetDefect::CommentWithoutABox {
                        sheet_part: sheet_part.as_str().to_owned(),
                        comments_part: comments_part.as_str().to_owned(),
                        cell: cell.to_string(),
                    }
                    .into())
                }
            }
        }

        let (Some((drawing, interner)), Some(vml_part)) = (drawing.as_ref(), vml_part.as_ref())
        else {
            continue;
        };
        for shape in drawing.all_shapes() {
            if !comments::is_comment_shape(shape, interner) {
                continue;
            }
            let identifier = shape.identifier(interner).unwrap_or_default();
            if claimed.contains(&identifier) {
                continue;
            }
            return Err(SpreadsheetDefect::CommentBoxWithoutAComment {
                sheet_part: sheet_part.as_str().to_owned(),
                vml_part: vml_part.as_str().to_owned(),
                shape: identifier,
                cell: comments::comment_box_cell(shape, interner)
                    .map_or_else(|| "?".to_owned(), |cell| cell.to_string()),
            }
            .into());
        }
    }
    Ok(())
}

/// The `x:sheets` list agrees with the workbook part's relationships, in both directions, and its
/// two identifier spaces are unique — checked only if this library will write the workbook's markup.
fn check_sheet_list(package: &Package, workbook_part: &PartName) -> Result<(), XlsxError> {
    let Some((_, entry)) = package
        .authored_xml_parts()
        .find(|(part, _)| part == workbook_part)
    else {
        return Ok(()); // Container bytes: not ours to fault. See this module's own docs.
    };

    match entry.tree() {
        Some(tree) => check_sheet_list_markup(package, workbook_part, tree),
        None => {
            let Some(bytes) = entry.bytes() else {
                return Ok(());
            };
            // Well-formedness of an authored part is `mjx-opc`'s defect to report; if it will not
            // parse there is nothing here to check.
            let Ok(tree) = fidelity::parse(bytes) else {
                return Ok(());
            };
            check_sheet_list_markup(package, workbook_part, &tree)
        }
    }
}

fn check_sheet_list_markup(
    package: &Package,
    workbook_part: &PartName,
    tree: &RawDocument,
) -> Result<(), XlsxError> {
    let interner = &tree.interner;
    let Some(sheets) = nav::child(&tree.root, interner, SML, "sheets") else {
        return Ok(());
    };
    let part = || workbook_part.as_str().to_owned();

    // What the list names, in order, and the two identifier spaces §18.2.19 requires to be unique.
    let mut listed: HashSet<String> = HashSet::new();
    let mut listed_order: Vec<String> = Vec::new();
    let mut sheet_ids: HashSet<String> = HashSet::new();
    let mut names: HashSet<String> = HashSet::new();
    let reference_prefix =
        nav::namespace_prefix(&tree.root, interner, SHARED_RELATIONSHIP_REFERENCE);

    for sheet in nav::children(sheets, interner, SML, "sheet") {
        if let Some(sheet_id) = nav::attr_value(sheet, interner, "sheetId") {
            let sheet_id = sheet_id?;
            if !sheet_ids.insert(sheet_id.clone()) {
                return Err(SpreadsheetDefect::DuplicateSheetId {
                    part: part(),
                    sheet_id,
                }
                .into());
            }
        }
        if let Some(name) = nav::attr_value(sheet, interner, "name") {
            let name = name?;
            if !names.insert(name.clone()) {
                return Err(SpreadsheetDefect::DuplicateSheetName { part: part(), name }.into());
            }
        }
        let Some(reference) = reference_prefix
            .and_then(|prefix| nav::prefixed_attr_value(sheet, interner, prefix, "id"))
        else {
            continue;
        };
        let reference = reference?;
        if !listed.insert(reference.clone()) {
            return Err(SpreadsheetDefect::DuplicateSheetReference {
                part: part(),
                relationship_id: reference,
            }
            .into());
        }
        listed_order.push(reference);
    }

    // What the relationships offer: every internal relationship whose target part carries one of
    // the three sheet content types. Matching on the *content type* rather than the relationship
    // type keeps this correct in both conformance worlds, whose relationship-type URIs differ while
    // their content types do not — the same reasoning `mjx_pptx::validate` states for `p:sldIdLst`.
    let Some(relationships) = package.relationships_for(Some(workbook_part)) else {
        return Ok(());
    };
    let mut sheet_relationships: HashMap<&str, PartName> = HashMap::new();
    for rel in relationships.iter() {
        if rel.mode == TargetMode::External {
            continue;
        }
        let Ok(target) = nav::resolve_target(workbook_part, &rel.target) else {
            continue; // `mjx-opc`'s defect to report.
        };
        if package
            .content_type_of(&target)
            .and_then(SheetKind::from_content_type)
            .is_some()
        {
            sheet_relationships.insert(rel.id.as_str(), target);
        }
    }

    // An entry naming a relationship that leads somewhere else.
    for reference in &listed_order {
        if sheet_relationships.contains_key(reference.as_str()) {
            continue;
        }
        let Some(rel) = relationships.by_id(reference) else {
            continue; // Undeclared: reported one layer down, as a dangling reference.
        };
        let Ok(target) = nav::resolve_target(workbook_part, &rel.target) else {
            continue;
        };
        let Some(actual) = package.content_type_of(&target) else {
            continue; // A target with no content type is `mjx-opc`'s defect to report.
        };
        return Err(SpreadsheetDefect::SheetEntryTargetIsNotASheet {
            part: part(),
            relationship_id: reference.clone(),
            target_part: target.as_str().to_owned(),
            actual_content_type: actual.to_owned(),
        }
        .into());
    }

    // …and the reverse: a sheet part the list never names.
    for (id, target) in &sheet_relationships {
        if listed.contains(*id) {
            continue;
        }
        return Err(SpreadsheetDefect::UnlistedSheetRelationship {
            part: part(),
            relationship_id: (*id).to_owned(),
            target_part: target.as_str().to_owned(),
        }
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parts::PartKind;

    /// Whether `kind` is one of the SpreadsheetML part kinds — the family
    /// [`SpreadsheetDefect::UnreachableSpreadsheetPart`] is about.
    ///
    /// A test helper, not part of the check: the check works from the content-type *prefix* so that
    /// a SpreadsheetML part this crate does not classify is covered too. This is what pins the two
    /// descriptions against each other.
    ///
    /// `.all()` on an empty slice is `true`, and no kind has an empty content-type list — every one
    /// of the twenty-eight declares at least one, which `every_kind_round_trips_through_its_own_content_types`
    /// asserts — so there is no kind this quietly answers `true` for by vacuity.
    fn is_spreadsheetml_part_kind(kind: PartKind) -> bool {
        kind.content_types()
            .iter()
            .all(|content_type| content_type.starts_with(SPREADSHEETML_CONTENT_TYPE_PREFIX))
    }

    use crate::parts::{
        CONTENT_TYPE_SHARED_STRINGS, CONTENT_TYPE_THEME, CONTENT_TYPE_WORKBOOK,
        CONTENT_TYPE_WORKSHEET, REL_SHARED_STRINGS, REL_THEME, REL_WORKSHEET,
    };
    use mjx_opc::Relationship;

    const WORKBOOK_MARKUP: &str = r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets></workbook>"#;

    /// A one-sheet workbook package, authored from nothing so the sheet-list checks are in scope.
    fn one_sheet_workbook(markup: &str) -> (Package, PartName) {
        let mut package = Package::empty();
        let workbook_part = PartName::new("/xl/workbook.xml").expect("a valid part name");
        package
            .insert_part(
                &workbook_part,
                CONTENT_TYPE_WORKBOOK,
                markup.as_bytes().to_vec(),
            )
            .expect("insert the workbook part");
        package
            .add_relationship(
                None,
                Relationship {
                    id: "rIdWb".to_owned(),
                    rel_type: REL_OFFICE_DOCUMENT.to_owned(),
                    target: "xl/workbook.xml".to_owned(),
                    mode: TargetMode::Internal,
                },
            )
            .expect("relate the workbook from the package root");
        let sheet = PartName::new("/xl/worksheets/sheet1.xml").expect("a valid part name");
        package
            .insert_part(&sheet, CONTENT_TYPE_WORKSHEET, b"<worksheet/>".to_vec())
            .expect("insert the worksheet part");
        package
            .add_relationship(
                Some(&workbook_part),
                Relationship {
                    id: "rId1".to_owned(),
                    rel_type: REL_WORKSHEET.to_owned(),
                    target: "worksheets/sheet1.xml".to_owned(),
                    mode: TargetMode::Internal,
                },
            )
            .expect("relate the worksheet");
        (package, workbook_part)
    }

    /// The baseline this whole module is measured against: a well-formed graph passes.
    ///
    /// Without it, every case below could be green because `check` rejects everything.
    #[test]
    fn a_well_formed_workbook_passes_every_check() {
        let (package, workbook_part) = one_sheet_workbook(WORKBOOK_MARKUP);
        check(&package, &workbook_part).expect("a correct workbook validates");
    }

    /// Retargeting the root relationship away from the workbook is refused, and the message names
    /// both the part and the target.
    #[test]
    fn the_office_document_relationship_must_still_name_the_workbook() {
        let (mut package, workbook_part) = one_sheet_workbook(WORKBOOK_MARKUP);
        package
            .retarget_relationship(
                None,
                "rIdWb",
                "xl/worksheets/sheet1.xml",
                TargetMode::Internal,
            )
            .expect("retarget the root relationship");
        let error =
            check(&package, &workbook_part).expect_err("the root edge no longer leads home");
        let text = error.to_string();
        assert!(text.contains("/xl/workbook.xml"), "{text}");
        assert!(text.contains("xl/worksheets/sheet1.xml"), "{text}");
    }

    /// A shared string table the graph cannot reach is refused, and the message names the part.
    ///
    /// This is the mutation the ticket names: drop the `sharedStrings` relationship and the save
    /// must stop. `Package::validate` alone cannot see it — an orphan is legal OPC — so this is the
    /// check that carries it, for the reason this module's own documentation gives.
    #[test]
    fn an_unreachable_shared_string_table_is_refused() {
        let (mut package, workbook_part) = one_sheet_workbook(WORKBOOK_MARKUP);
        let strings = PartName::new("/xl/sharedStrings.xml").expect("a valid part name");
        package
            .insert_part(&strings, CONTENT_TYPE_SHARED_STRINGS, b"<sst/>".to_vec())
            .expect("insert the shared string table");
        package
            .add_relationship(
                Some(&workbook_part),
                Relationship {
                    id: "rId9".to_owned(),
                    rel_type: REL_SHARED_STRINGS.to_owned(),
                    target: "sharedStrings.xml".to_owned(),
                    mode: TargetMode::Internal,
                },
            )
            .expect("relate it");
        check(&package, &workbook_part).expect("related, so reachable");

        package
            .remove_relationship(Some(&workbook_part), "rId9")
            .expect("drop the sharedStrings relationship");
        let error = check(&package, &workbook_part).expect_err("nothing reaches it now");
        assert!(
            error.to_string().contains("/xl/sharedStrings.xml"),
            "the defect must name the part: {error}"
        );
    }

    /// A stray *non*-SpreadsheetML part is left alone: the narrowing is one family wide, not a
    /// general orphan sweep that would contradict `mjx-opc`.
    ///
    /// The discriminating half of the case above — without it, "unreachable is refused" could be a
    /// blanket rule this crate has no business asserting.
    #[test]
    fn an_unreachable_theme_is_not_a_spreadsheetml_defect() {
        let (mut package, workbook_part) = one_sheet_workbook(WORKBOOK_MARKUP);
        let theme = PartName::new("/xl/theme/theme1.xml").expect("a valid part name");
        package
            .insert_part(&theme, CONTENT_TYPE_THEME, b"<a:theme/>".to_vec())
            .expect("insert an unrelated theme part");
        check(&package, &workbook_part)
            .expect("a theme nothing relates to is dead weight, not a broken workbook");

        // …and relating it changes nothing, which is the other half of "this rule is about one
        // family".
        package
            .add_relationship(
                Some(&workbook_part),
                Relationship {
                    id: "rId8".to_owned(),
                    rel_type: REL_THEME.to_owned(),
                    target: "theme/theme1.xml".to_owned(),
                    mode: TargetMode::Internal,
                },
            )
            .expect("relate it");
        check(&package, &workbook_part).expect("still fine");
    }

    /// An entry pointing at a part that is not a sheet is refused, naming the content type it found.
    #[test]
    fn a_sheet_entry_must_lead_to_a_sheet() {
        let (mut package, workbook_part) = one_sheet_workbook(WORKBOOK_MARKUP);
        // Point `rId1` — the one the sheet list names — at the styles part instead.
        let styles = PartName::new("/xl/styles.xml").expect("a valid part name");
        package
            .insert_part(
                &styles,
                crate::parts::CONTENT_TYPE_STYLES,
                b"<styleSheet/>".to_vec(),
            )
            .expect("insert the styles part");
        package
            .retarget_relationship(
                Some(&workbook_part),
                "rId1",
                "styles.xml",
                TargetMode::Internal,
            )
            .expect("retarget the sheet's relationship");
        // The worksheet part is now unreachable, so ask the sheet-list check directly: this case is
        // about the list, and the reachability case above is about reachability.
        let error = check_sheet_list(&package, &workbook_part).expect_err("rId1 is not a sheet");
        let text = error.to_string();
        assert!(text.contains("rId1"), "{text}");
        assert!(text.contains("/xl/styles.xml"), "{text}");
        assert!(text.contains("spreadsheetml.styles+xml"), "{text}");
    }

    /// A sheet part the workbook relates to but never lists is refused — a tab no consumer shows.
    #[test]
    fn a_sheet_part_the_list_never_names_is_refused() {
        let (mut package, workbook_part) = one_sheet_workbook(WORKBOOK_MARKUP);
        let second = PartName::new("/xl/worksheets/sheet2.xml").expect("a valid part name");
        package
            .insert_part(&second, CONTENT_TYPE_WORKSHEET, b"<worksheet/>".to_vec())
            .expect("insert a second worksheet");
        package
            .add_relationship(
                Some(&workbook_part),
                Relationship {
                    id: "rId2".to_owned(),
                    rel_type: REL_WORKSHEET.to_owned(),
                    target: "worksheets/sheet2.xml".to_owned(),
                    mode: TargetMode::Internal,
                },
            )
            .expect("relate it but never list it");
        let error = check(&package, &workbook_part).expect_err("sheet2 is related and unlisted");
        let text = error.to_string();
        assert!(text.contains("rId2"), "{text}");
        assert!(text.contains("/xl/worksheets/sheet2.xml"), "{text}");
    }

    /// The three identifier spaces §18.2.19 and the list itself require to be unique.
    #[test]
    fn the_sheet_lists_identifier_spaces_must_be_unique() {
        let cases: [(&str, &str); 3] = [
            (
                r#"<sheet name="A" sheetId="1" r:id="rId1"/><sheet name="B" sheetId="1" r:id="rId1"/>"#,
                "sheetId 1",
            ),
            (
                r#"<sheet name="A" sheetId="1" r:id="rId1"/><sheet name="A" sheetId="2" r:id="rId1"/>"#,
                r#"named "A""#,
            ),
            (
                r#"<sheet name="A" sheetId="1" r:id="rId1"/><sheet name="B" sheetId="2" r:id="rId1"/>"#,
                "names relationship rId1 more than once",
            ),
        ];
        for (entries, expected) in cases {
            let markup = format!(
                r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>{entries}</sheets></workbook>"#
            );
            let (package, workbook_part) = one_sheet_workbook(&markup);
            let error = check(&package, &workbook_part).expect_err("a repeated identifier");
            assert!(
                error.to_string().contains(expected),
                "expected a defect mentioning {expected}; got: {error}"
            );
        }
    }

    /// A workbook still holding its container bytes is never faulted for markup it arrived with.
    ///
    /// The scope rule, asserted rather than asserted-in-prose: the very markup that fails above is
    /// accepted when the bytes came from a container instead of from this library.
    #[test]
    fn markup_that_arrived_in_a_container_is_out_of_scope() {
        let duplicated = r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="A" sheetId="1" r:id="rId1"/><sheet name="A" sheetId="1" r:id="rId1"/></sheets></workbook>"#;
        let (package, workbook_part) = one_sheet_workbook(duplicated);
        check(&package, &workbook_part).expect_err("authored: in scope, and broken");

        // Round-trip the same package through a container, so every part is `FromContainer`.
        let bytes = package.save_unchecked().expect("write it out anyway");
        let reopened = Package::open(&bytes).expect("reopen");
        check_sheet_list(&reopened, &workbook_part)
            .expect("container bytes are not this library's markup to fault");
    }

    /// Every SpreadsheetML [`PartKind`] is inside the content-type family the reachability check
    /// uses, and the five outside it are outside it — the theme, the two kinds of drawing, the
    /// chart, and the XML map.
    ///
    /// Pins the prefix against the constants rather than restating it: a content type that drifted
    /// out of the family would silently stop being covered.
    ///
    /// [`PartKind::CustomXmlMappings`] is the last, and it is outside the family for a reason
    /// worth stating rather than patching around: §12.3.6 gives that part the content type
    /// `application/xml`, which is not a SpreadsheetML content type at all. So `xl/xmlMaps.xml` is
    /// **not** covered by [`SpreadsheetDefect::UnreachableSpreadsheetPart`] — an unreferenced one is
    /// dead weight rather than a missing dependency, exactly as `mjx-opc` says of any other
    /// unreferenced part. Its markup still validates against `sml.xsd`, and its bytes still survive
    /// a save; see `crates/mjx-xlsx/tests/preserved_parts.rs`.
    #[test]
    fn the_spreadsheetml_family_is_exactly_the_spreadsheetml_kinds() {
        for kind in PartKind::ALL {
            let expected = !matches!(
                kind,
                PartKind::Theme
                    | PartKind::Drawing
                    | PartKind::Chart
                    | PartKind::VmlDrawing
                    | PartKind::CustomXmlMappings
            );
            assert_eq!(
                is_spreadsheetml_part_kind(*kind),
                expected,
                "{kind:?} is on the wrong side of the SpreadsheetML content-type family"
            );
        }
    }
}
