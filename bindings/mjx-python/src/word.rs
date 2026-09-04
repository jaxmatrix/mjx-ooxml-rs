//! Word-specific value classes: page geometry, effective-property summaries, fields, comments,
//! notes and revisions — everything [`crate::document::Document`]'s methods take or hand back that
//! is not already covered by [`crate::address`] (paths), [`crate::enums`] (payload-free
//! enumerations) or [`crate::errors`].
//!
//! # `Effective*` is a curated subset, not the whole ladder
//!
//! `mjx_ooxml::EffectiveCharacterProperties`/`EffectiveParagraphProperties` carry the full
//! `EG_RPrBase`/`CT_PPrBase` ladder result — dozens of fields, several of them themselves nested
//! resolved structs three and four types deep (indentation, spacing, frame properties,
//! conditional-formatting regions, …). Binding every one of those would multiply this module's own
//! size several times over for fields a caller doing ordinary formatting inspection rarely needs.
//! So [`EffectiveCharacterProperties`]/[`EffectiveParagraphProperties`] here expose the fields most
//! callers actually reach for — the toggle properties, size, colour and fonts for a run; alignment,
//! outline level and the paragraph-mark toggles for a paragraph — documented on each getter. The
//! full, untrimmed value is always one Rust call away through `mjx_ooxml::Document` directly; this
//! binding does not claim to be that escape hatch.

use pyo3::prelude::*;
use pyo3::types::PyModule;

use mjx_ooxml as ooxml;

use crate::enums::{FieldForm, Justification, PageOrientation, RevisionKind};

value_class! {
    /// A page's extent, orientation and (optional) paper-size code, in twips (1/1440 inch).
    PageSize(ooxml::PageSize), derive(Copy, PartialEq, Eq);

    /// A section's page margins, in twips.
    PageMargins(ooxml::PageMargins), derive(Copy, PartialEq, Eq);

    /// A resolved colour: `"auto"`, or a concrete `RRGGBB` hex value.
    EffectiveColor(ooxml::EffectiveColor), derive(PartialEq, Eq);

    /// A resolved font reference, per script slot.
    EffectiveFonts(ooxml::EffectiveFonts), derive(PartialEq, Eq);

    /// The curated subset of a run's effective character formatting — see this module's own doc
    /// comment for which fields, and why not all of them.
    EffectiveCharacterProperties(ooxml::EffectiveCharacterProperties), derive(PartialEq);

    /// The curated subset of a paragraph's effective layout — see this module's own doc comment.
    EffectiveParagraphProperties(ooxml::EffectiveParagraphProperties), derive(PartialEq);

    /// A resolved cell/table shading: the pattern's own colour and the background it draws over.
    EffectiveShading(ooxml::EffectiveShading), derive(PartialEq, Eq);

    /// A resolved cell/table border edge: its colour and width.
    EffectiveBorder(ooxml::EffectiveBorder), derive(PartialEq, Eq);

    /// One section: the paragraphs it governs, and its own page geometry.
    SectionSummary(ooxml::SectionSummary), derive(PartialEq, Eq);

    /// One comment: its id, author, initials and text.
    CommentSummary(ooxml::CommentSummary), derive(PartialEq, Eq);

    /// One user-visible footnote or endnote: its id and text.
    NoteSummary(ooxml::NoteSummary), derive(PartialEq, Eq);

    /// One tracked-change marker: its kind, author, date and id.
    RevisionInfo(ooxml::RevisionInfo), derive(PartialEq, Eq);

    /// A hyperlink's click target: an external URL, or an in-document bookmark anchor.
    HyperlinkTarget(ooxml::HyperlinkTarget), derive(PartialEq, Eq);

    /// One field (`w:fldSimple` or a `w:fldChar` sequence): its form, instruction, cached result
    /// and any fields nested inside it.
    Field(ooxml::Field), derive(PartialEq);

    /// One way a table's grid and its rows disagree with each other.
    GridDiscrepancy(ooxml::GridDiscrepancy), derive(PartialEq, Eq);
}

#[pymethods]
impl PageSize {
    /// ISO 216 A4, portrait: 210 x 297 mm.
    #[staticmethod]
    fn a4() -> Self {
        Self(ooxml::PageSize::a4())
    }

    /// US Letter, portrait: 8.5 x 11 in.
    #[staticmethod]
    fn us_letter() -> Self {
        Self(ooxml::PageSize::us_letter())
    }

    /// An arbitrary page extent, in twips, with the given orientation. Not checked here —
    /// `Document.blank` checks the result before writing anything.
    #[staticmethod]
    fn from_twips(width_twips: u32, height_twips: u32, orientation: PageOrientation) -> Self {
        Self(ooxml::PageSize::from_twips(
            width_twips,
            height_twips,
            orientation.into(),
        ))
    }

    /// The same physical page, rotated: width and height swapped, orientation set to landscape.
    fn landscape(&self) -> Self {
        Self(self.0.landscape())
    }

    /// The page width, in twips (1/1440 inch) — the larger dimension when landscape.
    #[getter]
    fn width_twips(&self) -> u32 {
        self.0.width_twips
    }

    /// The page height, in twips.
    #[getter]
    fn height_twips(&self) -> u32 {
        self.0.height_twips
    }

    /// The page's stated orientation.
    #[getter]
    fn orientation(&self) -> PyResult<PageOrientation> {
        PageOrientation::from_model(self.0.orientation)
    }

    fn __repr__(&self) -> String {
        format!(
            "PageSize.from_twips({}, {}, {:?})",
            self.0.width_twips, self.0.height_twips, self.0.orientation
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl PageMargins {
    /// Word's "Normal" template margins: 1 inch on every side, half an inch header/footer, no
    /// gutter.
    #[staticmethod]
    fn normal() -> Self {
        Self(ooxml::PageMargins::NORMAL)
    }

    /// The top margin, in twips — signed, so a negative value overlaps the header.
    #[getter]
    fn top(&self) -> i32 {
        self.0.top
    }

    /// The right margin, in twips.
    #[getter]
    fn right(&self) -> u32 {
        self.0.right
    }

    /// The bottom margin, in twips — signed, so a negative value overlaps the footer.
    #[getter]
    fn bottom(&self) -> i32 {
        self.0.bottom
    }

    /// The left margin, in twips.
    #[getter]
    fn left(&self) -> u32 {
        self.0.left
    }

    /// The header's distance from the page's top edge, in twips.
    #[getter]
    fn header(&self) -> u32 {
        self.0.header
    }

    /// The footer's distance from the page's bottom edge, in twips.
    #[getter]
    fn footer(&self) -> u32 {
        self.0.footer
    }

    /// Extra binding-side space added to the left margin, in twips.
    #[getter]
    fn gutter(&self) -> u32 {
        self.0.gutter
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl EffectiveColor {
    /// Whether the document leaves this colour to the renderer (`w:val="auto"`).
    #[getter]
    fn is_auto(&self) -> bool {
        matches!(self.0, ooxml::EffectiveColor::Auto)
    }

    /// The concrete `RRGGBB` hex value, uppercase, when this is not `auto`.
    #[getter]
    fn hex(&self) -> Option<&str> {
        match &self.0 {
            ooxml::EffectiveColor::Hex(hex) => Some(hex),
            ooxml::EffectiveColor::Auto => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl EffectiveFonts {
    /// The Latin/ASCII-range typeface.
    #[getter]
    fn ascii(&self) -> Option<&str> {
        self.0.ascii.as_deref()
    }

    /// The Latin "High ANSI" typeface.
    #[getter]
    fn high_ansi(&self) -> Option<&str> {
        self.0.high_ansi.as_deref()
    }

    /// The East Asian typeface.
    #[getter]
    fn east_asian(&self) -> Option<&str> {
        self.0.east_asian.as_deref()
    }

    /// The complex-script typeface.
    #[getter]
    fn complex_script(&self) -> Option<&str> {
        self.0.complex_script.as_deref()
    }
}

#[pymethods]
impl EffectiveCharacterProperties {
    /// Bold, resolved (XOR-combined across the style chain — see the guide).
    #[getter]
    fn bold(&self) -> Option<bool> {
        self.0.bold
    }

    /// Italic, resolved.
    #[getter]
    fn italic(&self) -> Option<bool> {
        self.0.italic
    }

    /// Single strikethrough, resolved.
    #[getter]
    fn strikethrough(&self) -> Option<bool> {
        self.0.strikethrough
    }

    /// Hidden text (`w:vanish`), resolved.
    #[getter]
    fn hidden(&self) -> Option<bool> {
        self.0.hidden
    }

    /// All capitals, resolved.
    #[getter]
    fn all_capitals(&self) -> Option<bool> {
        self.0.all_capitals
    }

    /// Small capitals, resolved.
    #[getter]
    fn small_caps(&self) -> Option<bool> {
        self.0.small_caps
    }

    /// The font size, in half-points, as the raw wire string (`ST_HpsMeasure` — an unsigned decimal
    /// or a universal measure, never renormalized).
    #[getter]
    fn font_size_half_points(&self) -> Option<String> {
        self.0
            .font_size
            .as_ref()
            .map(|value| value.to_wire().to_owned())
    }

    /// The resolved colour, its theme reference already baked to concrete `RRGGBB`.
    #[getter]
    fn color(&self) -> Option<EffectiveColor> {
        self.0.color.clone().map(EffectiveColor)
    }

    /// The resolved font reference, per script slot.
    #[getter]
    fn fonts(&self) -> Option<EffectiveFonts> {
        self.0.fonts.clone().map(EffectiveFonts)
    }
}

#[pymethods]
impl EffectiveParagraphProperties {
    /// `w:keepNext`, resolved.
    #[getter]
    fn keep_with_next(&self) -> Option<bool> {
        self.0.keep_with_next
    }

    /// `w:keepLines`, resolved.
    #[getter]
    fn keep_lines_together(&self) -> Option<bool> {
        self.0.keep_lines_together
    }

    /// `w:pageBreakBefore`, resolved.
    #[getter]
    fn page_break_before(&self) -> Option<bool> {
        self.0.page_break_before
    }

    /// `w:widowControl`, resolved.
    #[getter]
    fn widow_control(&self) -> Option<bool> {
        self.0.widow_control
    }

    /// The paragraph's resolved alignment (`w:jc`).
    #[getter]
    fn alignment(&self) -> PyResult<Option<Justification>> {
        match self.0.alignment {
            Some(value) => Justification::from_model(value).map(Some),
            None => Ok(None),
        }
    }

    /// The resolved outline level (`w:outlineLvl`), `0`-based; absent for body text.
    #[getter]
    fn outline_level(&self) -> Option<i64> {
        self.0.outline_level
    }
}

#[pymethods]
impl EffectiveShading {
    /// The shading pattern's own colour.
    #[getter]
    fn pattern_color(&self) -> Option<EffectiveColor> {
        self.0.pattern_color.clone().map(EffectiveColor)
    }

    /// The background colour the pattern draws over.
    #[getter]
    fn fill(&self) -> Option<EffectiveColor> {
        self.0.fill.clone().map(EffectiveColor)
    }
}

#[pymethods]
impl EffectiveBorder {
    /// The border's resolved colour.
    #[getter]
    fn color(&self) -> EffectiveColor {
        EffectiveColor(self.0.color.clone())
    }

    /// The border's width, in eighths of a point, if stated.
    #[getter]
    fn width_eighths_of_a_point(&self) -> Option<u64> {
        self.0.width_eighths_of_a_point
    }
}

#[pymethods]
impl SectionSummary {
    /// The first paragraph index this section governs.
    #[getter]
    fn first_paragraph(&self) -> u32 {
        self.0.first_paragraph
    }

    /// The last paragraph index this section governs, inclusive — `None` if it governs no
    /// paragraph at all.
    #[getter]
    fn last_paragraph(&self) -> Option<u32> {
        self.0.last_paragraph
    }

    /// This section's page size, if it states one.
    #[getter]
    fn page_size(&self) -> Option<PageSize> {
        self.0.page_size.map(PageSize)
    }

    /// This section's page margins, if it states one.
    #[getter]
    fn page_margins(&self) -> Option<PageMargins> {
        self.0.page_margins.map(PageMargins)
    }
}

#[pymethods]
impl CommentSummary {
    /// The comment's own id.
    #[getter]
    fn id(&self) -> i64 {
        self.0.id
    }

    /// The comment's author.
    #[getter]
    fn author(&self) -> &str {
        &self.0.author
    }

    /// The comment author's initials, if stated.
    #[getter]
    fn initials(&self) -> Option<&str> {
        self.0.initials.as_deref()
    }

    /// The comment's own text.
    #[getter]
    fn text(&self) -> &str {
        &self.0.text
    }
}

#[pymethods]
impl NoteSummary {
    /// The note's own id.
    #[getter]
    fn id(&self) -> i64 {
        self.0.id
    }

    /// The note's own text.
    #[getter]
    fn text(&self) -> &str {
        &self.0.text
    }
}

#[pymethods]
impl RevisionInfo {
    /// What kind of tracked change this is.
    #[getter]
    fn kind(&self) -> PyResult<RevisionKind> {
        RevisionKind::from_model(self.0.kind)
    }

    /// The author, if stated and well-formed.
    #[getter]
    fn author(&self) -> Option<&str> {
        self.0.author.as_deref()
    }

    /// The date/time stamp, the file's own wire string, unparsed.
    #[getter]
    fn date(&self) -> Option<&str> {
        self.0.date.as_deref()
    }

    /// The revision's own id, if stated and well-formed.
    #[getter]
    fn id(&self) -> Option<i64> {
        self.0.id
    }
}

#[pymethods]
impl HyperlinkTarget {
    /// An external URL target.
    #[staticmethod]
    fn url(url: &str) -> Self {
        Self(ooxml::HyperlinkTarget::Url(url.to_owned()))
    }

    /// An in-document bookmark anchor target.
    #[staticmethod]
    fn anchor(name: &str) -> Self {
        Self(ooxml::HyperlinkTarget::Anchor(name.to_owned()))
    }

    /// Whether this is an external URL target.
    #[getter]
    fn is_url(&self) -> bool {
        matches!(self.0, ooxml::HyperlinkTarget::Url(_))
    }

    /// The URL, when this is a `url` target.
    #[getter]
    fn url_value(&self) -> Option<&str> {
        match &self.0 {
            ooxml::HyperlinkTarget::Url(url) => Some(url),
            ooxml::HyperlinkTarget::Anchor(_) => None,
        }
    }

    /// The bookmark name, when this is an `anchor` target.
    #[getter]
    fn anchor_value(&self) -> Option<&str> {
        match &self.0 {
            ooxml::HyperlinkTarget::Anchor(name) => Some(name),
            ooxml::HyperlinkTarget::Url(_) => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Field {
    /// Which wire form this field was read from.
    #[getter]
    fn form(&self) -> PyResult<FieldForm> {
        FieldForm::from_model(self.0.form())
    }

    /// The field's instruction, verbatim, excluding any nested field's own instruction.
    #[getter]
    fn instruction(&self) -> &str {
        self.0.instruction()
    }

    /// The instruction's own field-type keyword (`"HYPERLINK"`, `"TOC"`, …), if recognizable.
    #[getter]
    fn field_name(&self) -> Option<&str> {
        self.0.field_name()
    }

    /// The instruction with the field-type keyword removed, verbatim.
    #[getter]
    fn arguments(&self) -> &str {
        self.0.arguments()
    }

    /// The field's cached result, excluding any nested field's own result — `None` only for a
    /// complex field with no `separate` marker (legal markup, not a missing value).
    #[getter]
    fn cached_result(&self) -> Option<&str> {
        self.0.cached_result()
    }

    /// Every field nested inside this one's own instruction or result zone, in document order.
    #[getter]
    fn nested_fields(&self) -> Vec<Field> {
        self.0.nested_fields().iter().cloned().map(Field).collect()
    }
}

#[pymethods]
impl GridDiscrepancy {
    /// Which kind of discrepancy this is: `"RowWidthMismatch"`, `"OrphanedVerticalMerge"` or
    /// `"EmptyRow"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.0 {
            ooxml::GridDiscrepancy::RowWidthMismatch { .. } => "RowWidthMismatch",
            ooxml::GridDiscrepancy::OrphanedVerticalMerge { .. } => "OrphanedVerticalMerge",
            ooxml::GridDiscrepancy::EmptyRow { .. } => "EmptyRow",
            // `GridDiscrepancy` is `#[non_exhaustive]`.
            _ => "Unknown",
        }
    }

    /// The row involved, for every kind.
    #[getter]
    fn row(&self) -> Option<u32> {
        match self.0 {
            ooxml::GridDiscrepancy::RowWidthMismatch { row, .. }
            | ooxml::GridDiscrepancy::OrphanedVerticalMerge { row, .. }
            | ooxml::GridDiscrepancy::EmptyRow { row } => u32::try_from(row).ok(),
            _ => None,
        }
    }

    /// The grid column involved, for `OrphanedVerticalMerge` only.
    #[getter]
    fn column(&self) -> Option<u32> {
        match self.0 {
            ooxml::GridDiscrepancy::OrphanedVerticalMerge { column, .. } => {
                u32::try_from(column).ok()
            }
            _ => None,
        }
    }

    /// The grid's declared column count, for `RowWidthMismatch` only.
    #[getter]
    fn declared_columns(&self) -> Option<u32> {
        match self.0 {
            ooxml::GridDiscrepancy::RowWidthMismatch {
                declared_columns, ..
            } => u32::try_from(declared_columns).ok(),
            _ => None,
        }
    }

    /// What the row's cells actually sum to, for `RowWidthMismatch` only.
    #[getter]
    fn spanned_columns(&self) -> Option<u32> {
        match self.0 {
            ooxml::GridDiscrepancy::RowWidthMismatch {
                spanned_columns, ..
            } => u32::try_from(spanned_columns).ok(),
            _ => None,
        }
    }
}

/// Adds every class in this module to the extension module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PageSize>()?;
    module.add_class::<PageMargins>()?;
    module.add_class::<EffectiveColor>()?;
    module.add_class::<EffectiveFonts>()?;
    module.add_class::<EffectiveCharacterProperties>()?;
    module.add_class::<EffectiveParagraphProperties>()?;
    module.add_class::<EffectiveShading>()?;
    module.add_class::<EffectiveBorder>()?;
    module.add_class::<SectionSummary>()?;
    module.add_class::<CommentSummary>()?;
    module.add_class::<NoteSummary>()?;
    module.add_class::<RevisionInfo>()?;
    module.add_class::<HyperlinkTarget>()?;
    module.add_class::<Field>()?;
    module.add_class::<GridDiscrepancy>()
}
