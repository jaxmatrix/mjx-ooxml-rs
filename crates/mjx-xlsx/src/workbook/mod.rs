//! The [`Workbook`] entry point: open a container, read its part graph, save it back.
//!
//! # Why this is a directory on day one
//!
//! `mjx-pptx`'s `Presentation` reached **12,771 lines** in one file before A8 split it across
//! nineteen modules under `presentation/`, and `mjx-docx` started its equivalent split at MJXOFF-90
//! rather than repeat that. Excel is the larger of the three schemas — `sml.xsd` declares 367
//! complex types against `pml.xsd`'s 149 — so this crate gets the same treatment before the first
//! model lands, and each of the eighteen Phase D children after this one has a home named for it
//! rather than a god module to append to.
//!
//! # The split this crate is one half of
//!
//! **`mjx-sml` owns `sml.xsd` content; `mjx-xlsx` owns OPC structure.** A cell, a row, a shared
//! string, an `xf` — what they *are* — is `mjx-sml`'s, because an embedded workbook inside a `.pptx`
//! or a `.docx` is SpreadsheetML too and `mjx-chart` has to reach it. Parts, content types,
//! relationships, the ZIP and the [`Workbook`] a caller holds are this crate's. `mjx_sml::workbook`
//! and this module therefore share a name and share nothing else: that one models `CT_Workbook`,
//! this one is the surface a package is reached through.
//!
//! # The module tree, and the child that fills each file
//!
//! | Module | Filled by |
//! |---|---|
//! | `mod.rs` (this file) | MJXOFF-91 (D02) — [`Workbook`]: `open`/`from_package`/`save`/`save_unchecked`/`validate`/`parts`/`sheets`/`part_inventory` |
//! | [`sheets`](self::sheets) | MJXOFF-91 (D02) — the `x:sheets` graph entry ([`Sheet`], [`crate::SheetKind`]); MJXOFF-100 (D06) reads it through [`mjx_sml::WorkbookPart`] and resolves each `r:id` |
//! | [`views`](self::views) | MJXOFF-100 (D06) — [`WorkbookWindow`]: the active tab and the window geometry, decoded |
//! | [`properties`](self::properties) | MJXOFF-100 (D06) — [`DateSystem`] and [`CalculationSettings`], decoded |
//! | [`defined_names`](self::defined_names) | MJXOFF-100 (D06) — [`DefinedNameEntry`], with `@localSheetId` resolved against the sheet list |
//!
//! Beside this directory: [`crate::parts`] (the part graph), [`crate::preserve`] (what happens to a
//! part nobody models), [`crate::validate`] (the SpreadsheetML invariants), [`crate::worksheet`]
//! (the sheet-level graph and, from MJXOFF-102, the sheet itself), [`crate::blank`] (MJXOFF-112),
//! [`crate::error`], [`crate::guide`] and `nav.rs`. A child needing a subject none of them names
//! adds the file *and* a row here, the same way `mjx-pptx`'s own list grew past A8.
//!
//! # What MJXOFF-91 deliberately does not do
//!
//! It models nothing. There is no cell, no shared string and no style here, and reading a workbook
//! never re-serializes a part: the whole deliverable is that
//! `Workbook::open(bytes)?.save()?` reproduces every decompressed part byte for byte, and that the
//! part graph a later child needs is already resolved. See [`crate::preserve`] for that contract in
//! full.

pub(crate) mod defined_names;
pub(crate) mod properties;
pub(crate) mod sheets;
pub(crate) mod views;

use mjx_ooxml_core::{Interner, RawDocument, ToXml};
use mjx_ooxml_types::namespaces::SML;
use mjx_opc::{Package, PartName, TargetMode};
use mjx_sml::WorkbookPart;

use crate::error::XlsxError;
use crate::nav;
use crate::parts::{PartKind, WorkbookParts};
use crate::preserve::PartInventoryEntry;
use crate::worksheet::Worksheet;

pub use defined_names::{DefinedNameEntry, DefinedNameScope};
pub use properties::{CalculationSettings, DateSystem};
pub use sheets::Sheet;
pub use views::WorkbookWindow;

/// A SpreadsheetML workbook: an [`mjx_opc::Package`], the workbook part inside it, and the part
/// graph reached from there.
///
/// ```no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let bytes = std::fs::read("book.xlsx")?;
/// let workbook = mjx_xlsx::Workbook::open(&bytes)?;
/// for sheet in workbook.sheets() {
///     println!("{} -> {:?}", sheet.name, sheet.part);
/// }
/// let saved = workbook.save()?;
/// # let _ = saved;
/// # Ok(())
/// # }
/// ```
///
/// # Fidelity
///
/// Opening a workbook parses exactly one part — `xl/workbook.xml`, to read its sheet list — and
/// parsing is not mutating: the part keeps its container bytes and [`save`](Self::save) re-emits
/// them verbatim, as it does for every part nothing has touched. There is nothing in this type that
/// can dirty a part, which is why [`Workbook::open`] followed by [`Workbook::save`] is a byte-exact
/// round trip of the whole container.
#[derive(Debug)]
pub struct Workbook {
    package: Package,
    workbook_part: PartName,
    parts: WorkbookParts,
    sheets: Vec<Sheet>,
}

impl Workbook {
    /// Opens a workbook from its container bytes, resolving the workbook part and its part graph.
    ///
    /// # Errors
    /// Returns [`XlsxError`] if the package is unreadable, has no `officeDocument` relationship, its
    /// workbook part is missing, or that part's root element is not `x:workbook`.
    pub fn open(bytes: &[u8]) -> Result<Self, XlsxError> {
        Self::from_package(Package::open(bytes)?)
    }

    /// Resolves an already-loaded [`Package`] into a workbook.
    ///
    /// The constructor for a caller who already holds the package — and the one path every other
    /// constructor goes through, so that a workbook built from nothing (MJXOFF-112's
    /// `Workbook::blank`) is resolved by the same code that resolves one read off disk, rather than
    /// surviving as a special case nothing checks.
    ///
    /// # Errors
    /// Returns [`XlsxError::MissingOfficeDocument`] if the package root has no `officeDocument`
    /// relationship, [`XlsxError::ExternalTarget`] if that relationship points outside the package,
    /// [`XlsxError::MissingWorkbookPart`] if the part it names is absent, or
    /// [`XlsxError::MalformedWorkbook`] if that part's root element is not `x:workbook`.
    pub fn from_package(mut package: Package) -> Result<Self, XlsxError> {
        let workbook_part = {
            let root_rels = package
                .relationships_for(None)
                .ok_or(XlsxError::MissingOfficeDocument)?;
            let rel = root_rels
                .by_type(PartKind::Workbook.relationship_type())
                .next()
                .ok_or(XlsxError::MissingOfficeDocument)?;
            if rel.mode == TargetMode::External {
                return Err(XlsxError::ExternalTarget {
                    target: rel.target.clone(),
                });
            }
            nav::resolve_from_root(&rel.target)?
        };
        if !package.part_names().any(|part| part == workbook_part) {
            return Err(XlsxError::MissingWorkbookPart(
                workbook_part.as_str().to_owned(),
            ));
        }

        {
            // A read, never a mutation: `part_tree` keeps the part's original bytes and `save` still
            // re-emits them verbatim. The workbook part is identified by its **root element**, not
            // by its content type — which is what lets a macro-enabled workbook open even though
            // ECMA-376 declares no content type for one (see `crate::parts`).
            let doc = package.part_tree(&workbook_part)?;
            if !nav::name_is(&doc.root.name, &doc.interner, SML, "workbook") {
                return Err(XlsxError::MalformedWorkbook(
                    "root element is not x:workbook",
                ));
            }
        }

        let parts = WorkbookParts::resolve(&package, &workbook_part)?;
        let sheets = sheets::resolve(&mut package, &workbook_part)?;

        Ok(Self {
            package,
            workbook_part,
            parts,
            sheets,
        })
    }

    /// The workbook part's own name — `/xl/workbook.xml` in everything a real producer writes,
    /// though nothing in OPC requires that spelling.
    #[must_use]
    pub fn workbook_part(&self) -> &PartName {
        &self.workbook_part
    }

    /// The resolved part graph: styles, shared strings, theme, and every other part this crate
    /// classifies that the workbook relates to.
    #[must_use]
    pub fn parts(&self) -> &WorkbookParts {
        &self.parts
    }

    /// The workbook's tabs, in the order `x:sheets` lists them — see
    /// `crates/mjx-xlsx/src/workbook/sheets.rs`'s own module documentation for why that list, and
    /// not the relationship order, is the answer.
    #[must_use]
    pub fn sheets(&self) -> &[Sheet] {
        &self.sheets
    }

    /// The sheet at `index` in the `x:sheets` list, with its own part graph resolved.
    ///
    /// `None` if `index` is past the end, or if the entry's `r:id` reaches no part.
    ///
    /// # Errors
    /// Returns [`XlsxError`] if one of the sheet's own relationships has an unresolvable or external
    /// target.
    pub fn worksheet(&self, index: usize) -> Result<Option<Worksheet<'_>>, XlsxError> {
        let Some(sheet) = self.sheets.get(index) else {
            return Ok(None);
        };
        Worksheet::resolve(&self.package, sheet)
    }

    /// The index in the sheet list of the tab named exactly `name`, or `None`.
    ///
    /// Exact and case-sensitive. `@name` is `ST_Xstring`, ECMA-376 gives no case-folding rule for
    /// it, and a workbook may legally carry two tabs whose names differ only in case — so matching
    /// case-insensitively would be this library inventing a rule and then answering ambiguously.
    /// The **first** match wins, which matters only for a file that already broke §18.2.19's
    /// uniqueness requirement; [`Workbook::validate`](Self::validate) reports that separately.
    #[must_use]
    pub fn sheet_index_by_name(&self, name: &str) -> Option<usize> {
        self.sheets.iter().position(|sheet| sheet.name == name)
    }

    /// The tab named exactly `name`, or `None`. See [`sheet_index_by_name`](Self::sheet_index_by_name).
    #[must_use]
    pub fn sheet_by_name(&self, name: &str) -> Option<&Sheet> {
        self.sheets.iter().find(|sheet| sheet.name == name)
    }

    /// The sheet named exactly `name`, with its own part graph resolved.
    ///
    /// `None` if no tab has that name, or if its entry's `r:id` reaches no part.
    ///
    /// # Errors
    /// Returns [`XlsxError`] if one of the sheet's own relationships has an unresolvable or external
    /// target.
    pub fn worksheet_by_name(&self, name: &str) -> Result<Option<Worksheet<'_>>, XlsxError> {
        let Some(sheet) = self.sheet_by_name(name) else {
            return Ok(None);
        };
        Worksheet::resolve(&self.package, sheet)
    }

    /// The tabs a consumer shows, in tab order — those whose `@state` is `visible`.
    pub fn visible_sheets(&self) -> impl Iterator<Item = &Sheet> + '_ {
        self.sheets.iter().filter(|sheet| sheet.is_visible())
    }

    /// Reads the modelled `xl/workbook.xml`, handing `read` the parsed [`WorkbookPart`] together
    /// with the [`Interner`] it was parsed with.
    ///
    /// **This is not a mutation.** The part keeps its container bytes and
    /// [`save`](Self::save) still re-emits them verbatim; parsing a tree for reading is what
    /// [`mjx_opc::Package::part_tree`] is for. Mirrors `mjx_docx::Document::document_settings`
    /// exactly, which is the shape this workspace already uses for a whole-part model.
    ///
    /// # Errors
    /// Returns [`XlsxError`] if the workbook part cannot be read or is not well-formed, or
    /// [`XlsxError::MalformedWorkbook`] if its root is not `x:workbook` — which
    /// [`from_package`](Self::from_package) has already ruled out for a workbook that opened.
    pub fn workbook_markup<R>(
        &mut self,
        read: impl FnOnce(&WorkbookPart, &Interner) -> R,
    ) -> Result<R, XlsxError> {
        let part = self.workbook_part.clone();
        let document = self.package.part_tree(&part)?;
        let Some(markup) = WorkbookPart::read_part(document)? else {
            return Err(XlsxError::MalformedWorkbook(
                "root element is not x:workbook",
            ));
        };
        Ok(read(&markup, &document.interner))
    }

    /// Edits the modelled `xl/workbook.xml` and writes it back, keeping the verbatim bytes of every
    /// element the edit did not touch.
    ///
    /// The write-back goes through [`ToXml::write_back`], which restores the source range of every
    /// node the rebuild reproduced unchanged — so renaming one tab re-flows that one start tag and
    /// copies the rest of the part, extension list included.
    ///
    /// The resolved sheet list is refreshed afterwards, so [`sheets`](Self::sheets) never reports a
    /// tab name the part no longer holds.
    ///
    /// # Errors
    /// Returns [`XlsxError`] if the workbook part cannot be read or is not well-formed, or if the
    /// sheet list cannot be re-resolved afterwards.
    pub fn edit_workbook_markup<R>(
        &mut self,
        edit: impl FnOnce(&mut WorkbookPart, &mut Interner) -> R,
    ) -> Result<R, XlsxError> {
        let part = self.workbook_part.clone();
        let result = {
            let document = self.package.part_tree_mut(&part)?;
            let RawDocument { interner, root, .. } = document;
            let mut markup = match WorkbookPart::read_root(root, interner)? {
                Some(markup) => markup,
                None => {
                    return Err(XlsxError::MalformedWorkbook(
                        "root element is not x:workbook",
                    ))
                }
            };
            let result = edit(&mut markup, interner);
            markup.write_back(root, interner);
            result
        };
        self.sheets = sheets::resolve(&mut self.package, &part)?;
        Ok(result)
    }

    /// Renames the tab at `index`, leaving every other part — and every other element of
    /// `xl/workbook.xml` — byte-identical.
    ///
    /// The relationship the entry names is untouched, so the sheet still reaches the same part: a
    /// tab's name and the part behind it are independent, which is the whole point of
    /// `crates/mjx-xlsx/src/workbook/sheets.rs`'s first section.
    ///
    /// # Errors
    /// Returns [`XlsxError::NoSuchSheet`] if `index` names no tab, or [`XlsxError`] if the workbook
    /// part cannot be read or written.
    pub fn rename_sheet(&mut self, index: usize, name: &str) -> Result<(), XlsxError> {
        if index >= self.sheets.len() {
            return Err(XlsxError::NoSuchSheet {
                index,
                sheets: self.sheets.len(),
            });
        }
        self.edit_workbook_markup(|markup, interner| {
            if let Some(entry) = markup.sheets_mut().and_then(|list| list.entry_mut(index)) {
                entry.set_name(interner, Some(name));
            }
        })
    }

    /// The package this workbook was opened from.
    ///
    /// `&self`, so nothing reached through here can dirty a part. It is what
    /// `crates/mjx-xlsx/src/worksheet/grid.rs` reads a worksheet's **bytes** through, rather than
    /// asking for a tree the package would then cache.
    #[must_use]
    pub fn package(&self) -> &Package {
        &self.package
    }

    /// The package, mutably — for the one call that replaces a part's bytes.
    ///
    /// Deliberately `pub(crate)`: a caller reaching the package mutably could dirty any part of the
    /// container without this type knowing, and the resolved sheet list would then describe markup
    /// that is no longer there.
    pub(crate) fn package_mut(&mut self) -> &mut Package {
        &mut self.package
    }

    /// Every addressable part of the container, in container order, with its content type and what
    /// this crate made of it.
    ///
    /// A part reported [`Unclassified`](crate::PartClassification::Unclassified) is preserved,
    /// not rejected — see [`crate::preserve`].
    #[must_use]
    pub fn part_inventory(&self) -> Vec<PartInventoryEntry<'_>> {
        crate::preserve::inventory(&self.package)
    }

    /// Checks the packaging graph ([`mjx_opc::Package::validate`]) and this crate's own
    /// SpreadsheetML invariants, without writing anything.
    ///
    /// # Errors
    /// Returns [`XlsxError::Opc`] carrying the first packaging defect found, or
    /// [`XlsxError::InvalidWorkbook`] carrying the first SpreadsheetML one.
    pub fn validate(&self) -> Result<(), XlsxError> {
        self.package.validate().map_err(mjx_opc::OpcError::from)?;
        crate::validate::check(&self.package, &self.workbook_part)
    }

    /// Validates the workbook, then serializes it back to container bytes.
    ///
    /// # Errors
    /// Returns whatever [`validate`](Self::validate) returns, or [`XlsxError::Opc`] if the ZIP
    /// writer fails.
    pub fn save(&self) -> Result<Vec<u8>, XlsxError> {
        self.validate()?;
        self.save_unchecked()
    }

    /// Serializes the workbook back to container bytes **without** checking its invariants — the
    /// escape hatch for writing back a container that arrived broken.
    ///
    /// # Errors
    /// Returns [`XlsxError::Opc`] if the ZIP writer fails.
    pub fn save_unchecked(&self) -> Result<Vec<u8>, XlsxError> {
        Ok(self.package.save_unchecked()?)
    }
}
