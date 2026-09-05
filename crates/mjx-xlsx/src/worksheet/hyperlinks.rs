//! Hyperlinks at the package tier: the half `mjx-sml` cannot see, which is the relationship.
//!
//! | half | where it lives |
//! |---|---|
//! | the entry | `xl/worksheets/sheetN.xml`, `x:hyperlinks/hyperlink` — [`mjx_sml::Hyperlink`] |
//! | the target of an external one | `xl/worksheets/_rels/sheetN.xml.rels`, [`REL_HYPERLINK`](crate::parts::REL_HYPERLINK) |
//!
//! # A hyperlink and its relationship are one thing
//!
//! An external link is **two** records that have to agree, in two parts, and either one alone is a
//! file Excel offers to repair. So the two are written and removed together, in one call:
//!
//! * [`Workbook::set_cell_hyperlink`] with a [`HyperlinkTarget::Url`] adds the entry *and* an
//!   `External` relationship, and takes the previous entry's relationship out if nothing else in the
//!   sheet still names it;
//! * [`Workbook::remove_cell_hyperlink`] removes the entry *and* the relationship it named, under
//!   the same "unless something else still names it" rule;
//! * a relationship this library nonetheless leaves behind is reported by
//!   [`Workbook::validate`](crate::Workbook::validate) as
//!   [`SpreadsheetDefect::OrphanedHyperlinkRelationship`](crate::SpreadsheetDefect::OrphanedHyperlinkRelationship),
//!   rather than shipped for Excel to complain about.
//!
//! The vocabulary is `mjx-pptx`'s, deliberately. [`HyperlinkTarget::Url`] is
//! [`mjx_pptx::Hyperlink::Url`](https://docs.rs/mjx-pptx) down to the variant name, and
//! [`add_hyperlink_relationship`](Workbook::add_hyperlink_relationship) is
//! `Presentation::add_hyperlink_rel` with the same shape and the same "remove it once unreferenced"
//! companion. A second vocabulary for the same idea, one surface later, would be two things for a
//! caller to learn and two things for this project to keep in step.
//!
//! # The second kind is not a relationship at all
//!
//! PowerPoint's internal jump is a relationship to the target slide part. **Excel's is not.** An
//! internal link writes `@location` — `Sheet2!A1`, or a defined name — and names no relationship,
//! because there is no part to reach: the destination is a cell in this same workbook.
//! [`HyperlinkTarget::Location`] is therefore a string and not an index, and setting one adds
//! nothing to the `.rels`.
//!
//! # Never repaired, and never followed
//!
//! Two rules, both about not being helpful, and both stated again here because this is the tier
//! where it would be tempting:
//!
//! * **an entry carrying both an `@r:id` and a `@location` is not a defect.** It is a real shape
//!   Excel writes. [`SheetHyperlink`] reports it as
//!   [`HyperlinkKind::ExternalWithLocation`] and nothing normalises it away: no call here drops the
//!   relationship reference because a location is present, or the location because a relationship
//!   is;
//! * **an external target is an untrusted URI.** It is carried exactly as the `.rels` wrote it —
//!   never percent-normalised, never resolved against a base, never made absolute, and above all
//!   never fetched. `mjx-ooxml-rs` performs no network or filesystem access on a workbook's behalf,
//!   and a hyperlink is the most obvious place a reader might assume otherwise.

use mjx_ooxml_core::Interner;
use mjx_opc::{PartName, Relationship, TargetMode};
use mjx_sml::{CellRange, CellReference, Hyperlink};

use crate::error::XlsxError;
use crate::parts::REL_HYPERLINK;
use crate::workbook::Workbook;

/// Where a hyperlink points, in the form a caller writes one.
///
/// The authoring vocabulary, and deliberately only two cases — the two `CT_Hyperlink` can express
/// on its own. [`SheetHyperlink`] is the richer *report* that comes back out, because a file may say
/// more than a caller ever asks for.
///
/// Named for the target rather than for the link because `mjx_sml::Hyperlink` is already the markup
/// element; the [`Url`](Self::Url) variant keeps `mjx_pptx::Hyperlink::Url`'s name exactly, so the
/// two surfaces read the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HyperlinkTarget {
    /// An external target — a web URL, a `mailto:` address, or a file path — followed as-is by a
    /// consumer. Stored as an `External` relationship whose `Target` is this string, and the entry's
    /// `@r:id` names it.
    ///
    /// **Carried verbatim.** Nothing here parses, normalises, resolves or fetches it.
    Url(String),
    /// A jump inside this workbook: a cell reference such as `Sheet2!A1`, or a defined name.
    ///
    /// Stored as the entry's `@location` and **no relationship at all** — unlike PowerPoint's slide
    /// jump, which is an internal relationship. See the
    /// [*Hyperlinks* guide page](crate::guide::hyperlinks).
    Location(String),
}

/// Which of `CT_Hyperlink`'s shapes an entry actually is.
///
/// Four cases rather than two, because a file may write combinations the authoring vocabulary does
/// not offer, and reporting one of those as though it were one of the two would be repairing it in
/// the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyperlinkKind {
    /// `@r:id` and no `@location` — a plain external link.
    External,
    /// `@location` and no `@r:id` — a plain internal jump.
    Internal,
    /// **Both.** A real shape Excel writes, not a defect: an external target with a fragment, or a
    /// link repointed inside the workbook whose relationship was left in place. Reported, preserved,
    /// and never normalised into either of the other two.
    ExternalWithLocation,
    /// Neither. The entry names a range and nothing to go to — valid markup that points nowhere.
    Unresolved,
}

/// One `x:hyperlink` on a sheet, resolved against the sheet's relationships and decoded.
///
/// What [`Workbook::sheet_hyperlinks`] answers with: owned, borrowing nothing, holding no interner —
/// the shape [`SheetTable`](crate::SheetTable) and [`DefinedNameEntry`](crate::DefinedNameEntry)
/// take, and the shape a binding can project.
///
/// Every field is what the file *says*. [`target`](Self::target) is the relationship's `Target`
/// exactly as the `.rels` wrote it, and it is `None` both when the entry names no relationship and
/// when it names one the sheet does not declare — [`relationship_id`](Self::relationship_id) tells
/// those two apart, and the dangling case is reported by
/// [`Package::validate`](mjx_opc::Package::validate) rather than repaired here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetHyperlink {
    /// `@ref` — the range the link covers. A **range**, which for a single-cell link is a
    /// [`CellRange::Cell`].
    pub range: CellRange,
    /// `@r:id`, exactly as written, or `None` when the entry names no relationship.
    pub relationship_id: Option<String>,
    /// The `Target` of the relationship `@r:id` names, exactly as the `.rels` wrote it.
    ///
    /// `None` when there is no `@r:id`, **or** when there is one the sheet's `.rels` does not
    /// declare. Never resolved, rewritten, or fetched.
    pub target: Option<String>,
    /// The `TargetMode` of that relationship — `External` for every hyperlink Excel writes, and
    /// reported rather than assumed because a file may say otherwise.
    pub target_mode: Option<TargetMode>,
    /// `@location` — a cell reference or a defined name inside this workbook.
    pub location: Option<String>,
    /// `@tooltip` — the hover text.
    pub tooltip: Option<String>,
    /// `@display` — the text a consumer shows. Never kept in step with the cell's own value.
    pub display: Option<String>,
}

impl SheetHyperlink {
    /// Which of `CT_Hyperlink`'s four shapes this entry is.
    ///
    /// Decided on what the **element** says — whether it carries an `@r:id` and whether it carries a
    /// `@location` — and not on whether the relationship resolves. An entry with a dangling `@r:id`
    /// is still [`External`](HyperlinkKind::External): it is a link whose relationship is missing,
    /// which is a different fault from a link that names none.
    #[must_use]
    pub fn kind(&self) -> HyperlinkKind {
        match (self.relationship_id.is_some(), self.location.is_some()) {
            (true, true) => HyperlinkKind::ExternalWithLocation,
            (true, false) => HyperlinkKind::External,
            (false, true) => HyperlinkKind::Internal,
            (false, false) => HyperlinkKind::Unresolved,
        }
    }
}

impl Workbook {
    /// Every hyperlink on the tab at `index`, in the order `x:hyperlinks` lists them, resolved
    /// against the sheet's own relationships.
    ///
    /// Reading does not dirty the package: the worksheet is read from its bytes into a model that
    /// lives for the length of the call, and [`save`](Workbook::save) still re-emits it verbatim.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab; [`XlsxError::Sml`] or [`XlsxError::Xml`]
    /// if the worksheet part is unreadable, or if an entry's `@ref` is absent or will not parse —
    /// `@ref` is `use="required"` and a list keyed on it cannot be answered around.
    pub fn sheet_hyperlinks(&self, index: usize) -> Result<Vec<SheetHyperlink>, XlsxError> {
        let Some(markup) = self.worksheet_markup(index)? else {
            return Ok(Vec::new());
        };
        let sheet_part = self.sheet_part(index)?;
        let Some(links) = markup.hyperlinks() else {
            return Ok(Vec::new());
        };
        let prefix = markup.relationship_prefix();
        let interner = markup.interner();

        let mut out = Vec::with_capacity(links.len());
        for link in links.links() {
            out.push(self.decode(link, interner, prefix, &sheet_part)?);
        }
        Ok(out)
    }

    /// The first hyperlink covering `reference` on the tab at `index`, or `None`.
    ///
    /// **Document order, not precedence order**, for the reason
    /// [`mjx_sml::Hyperlinks::position_covering`] gives: two entries may cover one cell, and nothing
    /// here decides which one a consumer would follow.
    ///
    /// # Errors
    /// As [`sheet_hyperlinks`](Self::sheet_hyperlinks).
    pub fn cell_hyperlink(
        &self,
        index: usize,
        reference: CellReference,
    ) -> Result<Option<SheetHyperlink>, XlsxError> {
        let Some(markup) = self.worksheet_markup(index)? else {
            return Ok(None);
        };
        let sheet_part = self.sheet_part(index)?;
        let Some(links) = markup.hyperlinks() else {
            return Ok(None);
        };
        let prefix = markup.relationship_prefix();
        let interner = markup.interner();
        let Some(at) = links.position_covering(interner, reference) else {
            return Ok(None);
        };
        let Some(link) = links.links().nth(at) else {
            return Ok(None);
        };
        Ok(Some(self.decode(link, interner, prefix, &sheet_part)?))
    }

    /// Links `range` on the tab at `index` to `target`, replacing whatever entry already covered
    /// `range`'s first cell.
    ///
    /// **Both halves, in one call.** A [`HyperlinkTarget::Url`] adds an `External`
    /// [`REL_HYPERLINK`](crate::parts::REL_HYPERLINK) relationship on the **sheet** part and points
    /// the new entry's `@r:id` at it; a [`HyperlinkTarget::Location`] adds no relationship at all
    /// and writes `@location`. If an entry was replaced and it named a relationship, that
    /// relationship is removed once nothing else in the sheet still names it — the rule
    /// `mjx_pptx::Presentation::set_run_hyperlink` follows, which is what lets two cells share one
    /// link without the first removal breaking the second.
    ///
    /// A worksheet that binds no prefix to the relationship-reference namespace gains one, exactly
    /// as [`add_table`](Workbook::add_table) makes it: an `@r:id` cannot be spelled otherwise, and a
    /// sheet this library authored declares only the SpreadsheetML namespace. Nothing overwrites a
    /// binding the file made.
    ///
    /// **No cell is touched.** `@display` is not written from the cell's value and the cell's value
    /// is not written from `target`; making them agree is the caller's, and doing it silently would
    /// overwrite data nobody asked to lose.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, [`XlsxError::MissingWorkbookPart`] if it
    /// reaches no worksheet part, or [`XlsxError`] if the package refuses the relationship.
    pub fn set_cell_hyperlink(
        &mut self,
        index: usize,
        range: CellRange,
        target: &HyperlinkTarget,
    ) -> Result<(), XlsxError> {
        let sheet_part = self.sheet_part(index)?;
        // The range's top-left cell is what "already linked" is decided on: one entry may cover many
        // cells, and asking about every cell of a whole-column `@ref` would be a scan of a million.
        let bounds = range.normalized_bounds();
        let anchor = CellReference::relative(bounds.first_column(), bounds.first_row())
            .map_err(mjx_sml::SmlError::from)?;

        let mut markup = self
            .worksheet_markup(index)?
            .ok_or_else(|| XlsxError::MissingWorkbookPart(format!("sheet {index}")))?;

        // Everything that can fail without touching the package happens first, so a refusal leaves
        // the workbook exactly as it was.
        let displaced = markup
            .hyperlink_position_covering(anchor)
            .and_then(|at| {
                markup
                    .hyperlinks()
                    .and_then(|links| links.links().nth(at))
                    .map(|link| (at, link))
            })
            .map(|(at, link)| {
                link.relationship_id(markup.interner(), markup.relationship_prefix())
                    .map(|id| (at, id))
            })
            .transpose()
            .map_err(mjx_ooxml_core::FromXmlError::from)?;

        let relationship_id = match target {
            HyperlinkTarget::Url(url) => {
                let prefix = markup.bind_relationship_prefix();
                let id = self.next_sheet_relationship_id(&sheet_part);
                self.package_mut().add_relationship(
                    Some(&sheet_part),
                    Relationship {
                        id: id.clone(),
                        rel_type: REL_HYPERLINK.to_owned(),
                        // The URI the caller gave, byte for byte. Never normalised, never resolved.
                        target: url.clone(),
                        mode: TargetMode::External,
                    },
                )?;
                Some((prefix, id))
            }
            HyperlinkTarget::Location(_) => None,
        };

        {
            let element_prefix = markup.element_prefix().map(str::to_owned);
            let interner = markup.interner_mut();
            let mut entry = Hyperlink::new(interner, element_prefix.as_deref());
            entry.set_range(interner, range);
            match target {
                HyperlinkTarget::Url(_) => {
                    let (prefix, id) = relationship_id
                        .as_ref()
                        .expect("a Url target allocated a relationship above");
                    entry.set_relationship_id(interner, prefix, id);
                }
                HyperlinkTarget::Location(location) => {
                    entry.set_location(interner, Some(location.as_str()));
                }
            }
            if let Some((at, _)) = displaced.as_ref() {
                markup.remove_hyperlink(*at);
            }
            markup.add_hyperlink(entry);
        }
        self.write_worksheet_markup(index, &markup)?;

        if let Some((_, Some(previous))) = displaced {
            self.remove_hyperlink_relationship_if_unreferenced(&sheet_part, &previous)?;
        }
        Ok(())
    }

    /// Removes the first hyperlink covering `reference` on the tab at `index`, and the relationship
    /// it named, reporting whether there was one.
    ///
    /// **Both halves, in one call**, which is the whole point of this method existing beside
    /// [`mjx_sml::WorksheetPart::remove_hyperlink`]: the markup tier can take the entry out and
    /// cannot take the relationship out, and an entry removed without its relationship leaves an
    /// orphan [`Workbook::validate`](Workbook::validate) then reports.
    ///
    /// The relationship survives if some other entry in the same sheet still names it — two cells
    /// sharing one link is a shape Excel writes, and removing the first must not break the second.
    ///
    /// When the last entry goes, so does the `x:hyperlinks` element: `hyperlink` is
    /// `minOccurs="1"`.
    ///
    /// # Errors
    /// As [`set_cell_hyperlink`](Self::set_cell_hyperlink).
    pub fn remove_cell_hyperlink(
        &mut self,
        index: usize,
        reference: CellReference,
    ) -> Result<bool, XlsxError> {
        let sheet_part = self.sheet_part(index)?;
        let mut markup = self
            .worksheet_markup(index)?
            .ok_or_else(|| XlsxError::MissingWorkbookPart(format!("sheet {index}")))?;
        let Some(at) = markup.hyperlink_position_covering(reference) else {
            return Ok(false);
        };
        let prefix = markup.relationship_prefix().map(str::to_owned);
        let Some(removed) = markup.remove_hyperlink(at) else {
            return Ok(false);
        };
        let relationship_id = removed
            .relationship_id(markup.interner(), prefix.as_deref())
            .map_err(mjx_ooxml_core::FromXmlError::from)?;
        self.write_worksheet_markup(index, &markup)?;
        if let Some(relationship_id) = relationship_id {
            self.remove_hyperlink_relationship_if_unreferenced(&sheet_part, &relationship_id)?;
        }
        Ok(true)
    }

    /// Adds the relationship a [`HyperlinkTarget::Url`] needs to `part`, and answers its id.
    ///
    /// Public because [`set_cell_hyperlink`](Self::set_cell_hyperlink) is not the only thing that
    /// will ever need it — a comment box and an OLE object reach parts the same way — and because
    /// naming the one place a hyperlink relationship is created makes the two-halves rule checkable.
    /// `mjx_pptx::Presentation::add_hyperlink_rel` is the same method one surface earlier.
    ///
    /// **`url` is written verbatim.** Nothing normalises, resolves or validates it.
    ///
    /// # Errors
    /// [`XlsxError::Opc`] if the package refuses the relationship.
    pub fn add_hyperlink_relationship(
        &mut self,
        part: &PartName,
        url: &str,
    ) -> Result<String, XlsxError> {
        let id = self.next_sheet_relationship_id(part);
        self.package_mut().add_relationship(
            Some(part),
            Relationship {
                id: id.clone(),
                rel_type: REL_HYPERLINK.to_owned(),
                target: url.to_owned(),
                mode: TargetMode::External,
            },
        )?;
        Ok(id)
    }

    /// Removes hyperlink relationship `relationship_id` from `part` **unless** some `x:hyperlink`
    /// still in the part names it.
    ///
    /// The counterpart of `mjx_pptx::Presentation::remove_hyperlink_rel_if_unreferenced`, and the
    /// reason two cells can share one link: the relationship survives until its last user is gone.
    ///
    /// # Errors
    /// [`XlsxError`] if the worksheet cannot be read or the package refuses the removal.
    fn remove_hyperlink_relationship_if_unreferenced(
        &mut self,
        part: &PartName,
        relationship_id: &str,
    ) -> Result<(), XlsxError> {
        let still_used = match self.worksheet_markup_of(part)? {
            Some(markup) => {
                let prefix = markup.relationship_prefix();
                let interner = markup.interner();
                let mut used = false;
                if let Some(links) = markup.hyperlinks() {
                    for link in links.links() {
                        if link
                            .relationship_id(interner, prefix)
                            .map_err(mjx_ooxml_core::FromXmlError::from)?
                            .as_deref()
                            == Some(relationship_id)
                        {
                            used = true;
                            break;
                        }
                    }
                }
                used
            }
            None => false,
        };
        if !still_used {
            self.package_mut()
                .remove_relationship(Some(part), relationship_id)?;
        }
        Ok(())
    }

    /// The worksheet part behind the tab at `index`.
    fn sheet_part(&self, index: usize) -> Result<PartName, XlsxError> {
        let sheets = self.sheets().len();
        self.sheets()
            .get(index)
            .ok_or(XlsxError::NoSuchSheet { index, sheets })?
            .part
            .clone()
            .ok_or_else(|| XlsxError::MissingWorkbookPart(format!("sheet {index}")))
    }

    /// Decodes one entry into the owned report [`sheet_hyperlinks`](Self::sheet_hyperlinks) answers
    /// with.
    fn decode(
        &self,
        link: &Hyperlink,
        interner: &Interner,
        prefix: Option<&str>,
        sheet_part: &PartName,
    ) -> Result<SheetHyperlink, XlsxError> {
        use mjx_ooxml_core::FromXmlError;

        let relationship_id = link
            .relationship_id(interner, prefix)
            .map_err(FromXmlError::from)?;
        let resolved = relationship_id.as_deref().and_then(|id| {
            self.package()
                .relationships_for(Some(sheet_part))
                .and_then(|rels| rels.by_id(id))
                .map(|rel| (rel.target.clone(), rel.mode))
        });
        Ok(SheetHyperlink {
            range: link.range(interner).map_err(FromXmlError::from)?,
            relationship_id,
            target: resolved.as_ref().map(|(target, _)| target.clone()),
            target_mode: resolved.as_ref().map(|(_, mode)| *mode),
            location: link
                .location(interner)
                .map_err(FromXmlError::from)?
                .map(std::borrow::Cow::into_owned),
            tooltip: link
                .tooltip(interner)
                .map_err(FromXmlError::from)?
                .map(std::borrow::Cow::into_owned),
            display: link
                .display(interner)
                .map_err(FromXmlError::from)?
                .map(std::borrow::Cow::into_owned),
        })
    }
}
