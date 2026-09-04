//! The [`Document`] entry point: open, read/set the conformance class, save.
//!
//! `mjx-pptx`'s `Presentation` reached 12,771 lines and 266 public methods before A8 split it across
//! 19 files under `presentation/`. This module starts the equivalent split for Word **on day one**,
//! so the next nineteen children add to a plan instead of to a god object. Files this module already
//! has:
//!
//! - `mod.rs` (this file) — the [`Document`] facade (`open`/`save`/`save_unchecked`/`validate`), and
//!   [`MainDocument`] / the `w:document` root's own skeleton (`CT_Document`).
//! - `parts.rs` — [`PartKind`], the fourteen `wml.xsd` global elements classified as parts (thirteen
//!   of them; `w:txbxContent` is an inline root, not a part — see that module), and
//!   [`DocumentParts`], the resolved part graph.
//! - `body.rs` — `w:body` (`CT_Body`) and the whole block content model MJXOFF-92 gives it:
//!   paragraphs (`w:p`), runs (`w:r`), text (`w:t`) and the rest of `EG_RunInnerContent`'s 33
//!   members. `w:background` (`CT_Background`) stays the fidelity-wrapper skeleton C1 seeded it as —
//!   nobody has claimed its own content yet.
//! - `run_properties.rs` — `w:rPr` (`CT_RPr`) and `EG_RPrBase`'s 39 members, MJXOFF-94's own file:
//!   [`RunProperties`], reached off [`Run::run_properties`].
//! - `paragraph_properties.rs` — `w:pPr` (`CT_PPr`) and `CT_PPrBase`'s 33 members, MJXOFF-96's own
//!   file: [`ParagraphProperties`], reached off [`Paragraph::properties`], and
//!   [`ParagraphMarkRunProperties`] (`w:pPr/w:rPr`, the pilcrow's own formatting — never a run's).
//! - `styles.rs` — `word/styles.xml` (`CT_Styles`), MJXOFF-101's own file: [`StyleSheet`],
//!   `w:basedOn` chain resolution, `w:latentStyles`.
//! - `numbering.rs` — `word/numbering.xml` (`CT_Numbering`), MJXOFF-104's own file: the two-hop
//!   `w:numPr` → `w:num` → `w:abstractNum`/`w:lvl` resolution ([`NumberingIndex`]).
//! - `effective.rs` — the effective-properties ladder (`docDefaults` → table style → numbering →
//!   paragraph style → character style → direct), MJXOFF-106's own file — see
//!   [the guide](crate::effective_properties) for the full account.
//! - `sections.rs` — `w:sectPr` (`CT_SectPr`) and section addressing, MJXOFF-109's own file:
//!   [`SectionProperties`], [`SectionSpan`], and [`HeaderFooterReference`] (`EG_HdrFtrReferences`'
//!   structural half — *which* one applies is `headers.rs`'s).
//! - `headers.rs` — `w:hdr`/`w:ftr` (`CT_HdrFtr`) and the legacy VML they carry, MJXOFF-113's own
//!   file: [`HdrFtr`] (reusing MJXOFF-92's block-content addressing, generalized here — see that
//!   module's own doc comment), and variant resolution (`Document::resolve_header`/`resolve_footer`)
//!   against the ECMA-376 Part 1 prose this module's own doc comment quotes.
//! - `tables.rs` — `w:tbl`/`w:tr`/`w:tc` (`CT_Tbl`/`CT_Row`/`CT_Tc`) and the grid, MJXOFF-116's own
//!   file: [`Table`], [`Row`], [`Cell`] and the `(row, column)` merge-aware addressing and structural
//!   edits (insert/remove row/column) built on WordprocessingML's own *continuation* merge model —
//!   see that module's own doc comment for how it differs from `mjx-pptx`'s span model. A cell is
//!   `body.rs`'s block-content generalization's **third** container, after `Body` and `HdrFtr`.
//! - `fields.rs` / `hyperlinks.rs` — `w:fldSimple`/`w:fldChar` (`CT_SimpleField`/`CT_FldChar`) and
//!   `w:hyperlink`'s own attributes, MJXOFF-121's own files: [`Field`], [`FieldCharacter`],
//!   [`FormFieldData`] and hyperlink target resolution.
//! - `ranges.rs` — the range-marker mechanism `EG_RangeMarkupElements` needs (`CT_Markup`/
//!   `CT_MarkupRange`/`CT_Bookmark`, and [`ranges::RangeIndex`], the id-paired start/end resolver),
//!   MJXOFF-124's own file — see that module's own doc comment for why pairing is by id, never a
//!   stack, and what MJXOFF-126 should call.
//! - `annotations.rs` — `w:comments`/`w:footnotes`/`w:endnotes` (`CT_Comments`/`CT_Footnotes`/
//!   `CT_Endnotes`) and the section-level `w:footnotePr`/`w:endnotePr` C9 left opaque, MJXOFF-124's
//!   own file: [`Comments`], [`Footnotes`], [`Endnotes`] and their own content types.
//!
//! (This list previously named `styles.rs`, `numbering.rs`, `effective.rs` and `sections.rs` among
//! the files "later children are expected to add" — stale by the time MJXOFF-109 landed, all four
//! already existed. Fixed here rather than carried forward again.)
//!
//! Files later children are expected to add, one subject each (the same seam `presentation/` reads
//! in, chosen from the module list MJXOFF-90's ticket named for MJXOFF-92 through the rest of Phase
//! C): `revisions.rs`, `drawing.rs`, `settings.rs`, `structured_content.rs`. A child that needs a
//! subject not on this list adds the file and a line here, the same way `presentation/`'s own list
//! grew past A8.

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, RawAttribute, RawDocument, RawName, RawNode, ToXml,
};
use mjx_ooxml_types::namespaces::WML;
use mjx_ooxml_types::shared::ConformanceClass;
use mjx_opc::{Package, TargetMode};

use crate::error::DocxError;

mod annotations;
mod body;
mod drawing;
mod effective;
mod fields;
mod headers;
mod hyperlinks;
mod numbering;
mod paragraph_properties;
mod parts;
mod property_macros;
mod ranges;
mod revisions;
mod run_properties;
mod sections;
mod styles;
mod table_properties;
mod table_regions;
mod tables;

pub use annotations::{
    Comment, Comments, CommentsContent, EndnotePositionElement, EndnoteProperties,
    EndnotePropertyContent, Endnotes, EndnotesContent, FootnoteEndnote, FootnoteEndnoteReference,
    FootnotePositionElement, FootnoteProperties, FootnotePropertyContent, Footnotes,
    FootnotesContent, NumberFormatElement, NumberRestartElement,
};
pub use body::{
    Background, BlockContent, Body, Break, Hyperlink, Paragraph, ParagraphContent,
    PermissionRangeEnd, PermissionRangeStart, PhoneticGuide, PhoneticGuideChild,
    PhoneticGuideContent, PhoneticGuideContentItem, PhoneticGuideProperties,
    PhoneticGuidePropertyContent, PhoneticGuideTextAlignment, PositionalTab, ProofingError,
    RelationshipReference, Run, RunInnerContent, ShortHex, Symbol, Text, Unmodeled,
    WhitespacePreservation,
};
pub use drawing::{
    Control, Drawing, DrawingContent, EmbeddedObject, EmbeddedObjectContent, ObjectEmbed,
    ObjectLink, TextBoxContent, TextboxInfo, WordprocessingShape, WordprocessingShapeContent,
};
pub use effective::{
    EffectiveBorder, EffectiveCharacterProperties, EffectiveColor, EffectiveConditionalFormatting,
    EffectiveEastAsianLayout, EffectiveFonts, EffectiveFrameProperties, EffectiveIndentation,
    EffectiveLanguages, EffectiveManualRunWidth, EffectiveNumberingReference,
    EffectiveParagraphBorders, EffectiveParagraphProperties, EffectiveShading, EffectiveTabStop,
    EffectiveUnderline,
};
pub use fields::{
    Field, FieldCharacter, FieldCharacterContent, FieldForm, FieldPath, FormFieldCheckBox,
    FormFieldCheckBoxContent, FormFieldData, FormFieldDataContent, FormFieldDropDownList,
    FormFieldDropDownListContent, FormFieldHelpTextElement, FormFieldNameElement,
    FormFieldStatusTextElement, FormFieldTextInput, FormFieldTextInputContent,
    FormFieldTextTypeElement, MacroNameElement, SimpleField, StringElement,
    UnsignedDecimalNumberValue,
};
pub use headers::{HdrFtr, HeaderFooterType};
pub use hyperlinks::HyperlinkTarget;
pub use numbering::{
    AbstractNumbering, AbstractNumberingContent, HexIdentifier, LevelLegacyFormatting,
    LevelNumberFormat, LevelSuffix, LevelTextSegment, LevelTextTemplate, MultiLevelKind, Numbering,
    NumberingContent, NumberingIndex, NumberingInstance, NumberingInstanceContent, NumberingLevel,
    NumberingLevelContent, NumberingLevelOverride, NumberingLevelOverrideContent, NumberingLookup,
    NumberingPictureBullet, NumberingPictureBulletContent, NumberingResolution,
    MAX_NUM_STYLE_LINK_DEPTH,
};
pub use paragraph_properties::{
    ConditionalFormatting, ConditionalFormattingBits, DecimalNumberValue, FrameProperties,
    Indentation, LineSpacing, NumberingProperties, NumberingPropertyContent, ParagraphAlignment,
    ParagraphBorderContent, ParagraphBorders, ParagraphMarkRunProperties,
    ParagraphMarkRunPropertyContent, ParagraphProperties, ParagraphPropertyContent, ParagraphStyle,
    ParagraphTextFlowDirection, Spacing, TabStop, TabStopContent, TabStops,
    TextBoxTightWrapSetting, VerticalCharacterAlignment,
};
pub use parts::{DocumentParts, PartKind};
pub use ranges::{
    covered_text, paragraphs_spanned, Bookmark, BookmarkResolution, MarkerLocation, Markup,
    MarkupRange, RangeIndex, RangeResolution,
};
pub use revisions::{
    CellMergeTrackChange, CellPropertiesChange, CellPropertiesChangeContent, MoveBookmark,
    ParagraphMarkPropertiesChange, ParagraphMarkPropertiesChangeContent, ParagraphPropertiesChange,
    ParagraphPropertiesChangeContent, RevisionInfo, RevisionKind, RowPropertiesChange,
    RowPropertiesChangeContent, RunPropertiesChange, RunPropertiesChangeContent, RunTrackChange,
    SectionPropertiesChange, SectionPropertiesChangeContent, TableExceptionPropertiesChange,
    TableExceptionPropertiesChangeContent, TableGridChange, TableGridChangeContent,
    TablePropertiesChange, TablePropertiesChangeContent, TrackChangeMarker, TrackChangeNumbering,
};
pub use run_properties::{
    Border, CharacterStyle, Color, EastAsianLayout, Emphasis, Fonts, HalfPoint,
    HalfPointMeasureValue, HexColor, Highlight, Lang, Languages, ManualRunWidth, RunProperties,
    RunPropertyContent, Scale, Shading, SignedHalfPoint, SignedHalfPointMeasureValue, SignedTwips,
    SignedTwipsMeasureValue, TextEffect, TextScaleValue, ThemeHexDigit, Toggle, Twips, Underline,
    VerticalAlignment,
};
pub use sections::{
    BottomPageBorder, Column, Columns, ColumnsContent, DocumentGrid, HeaderFooterReference,
    LineNumbering, PageBorder, PageBorderSet, PageBorderSetContent, PageNumbering,
    PageVerticalAlignment, PaperSource, SectionLocation, SectionProperties, SectionPropertyContent,
    SectionSpan, SectionType, TopPageBorder,
};
pub use styles::{
    DefaultParagraphProperties, DefaultParagraphPropertyContent, DefaultRunProperties,
    DefaultRunPropertyContent, DocumentDefaults, DocumentDefaultsContent, LatentStyleContent,
    LatentStyleException, LatentStyles, LinkedStyleResolution, LongHex, RevisionSaveId,
    StyleDefinition, StyleDefinitionContent, StyleIndex, StyleParagraphProperties,
    StyleParagraphPropertyContent, StyleSheet, StyleSheetContent, StyleString, TableStyleOverride,
    TableStyleOverrideContent, MAX_BASED_ON_CHAIN_DEPTH,
};
pub use table_properties::{
    CellBorderContent, CellBorders, CellHeaderReferences, CellMargins, CellTextDirection,
    CellVerticalAlignment, FloatingTableOverlap, FloatingTablePosition, HeaderReferenceContent,
    MarginContent, RowHeight, RowProperties, RowPropertyContent, TableAlignment,
    TableBorderContent, TableBorders, TableCellMargins, TableExceptionProperties,
    TableExceptionPropertyContent, TableLayout, TableLook, TableProperties, TablePropertyContent,
    TableStringValue, TableWidth, TableWidthMeasure,
};
pub use table_regions::{
    applicable_regions, CellBorderEdge, ConditionalFormatRegion, TableLookFlags,
};
pub use tables::{
    Cell, CellProperties, CellPropertiesContent, Grid, GridColumn, GridContent, GridDiscrepancy,
    MergeMarker, MergedCellType, Row, RowContent, Table, TableContent,
};

use crate::address::{BlockPath, RunPath};

use parts::resolve_from_root;

/// An open WordprocessingML document: an OPC [`Package`] plus the resolved main document part and
/// its part graph.
///
/// Mirrors `mjx_pptx::Presentation` deliberately — same constructor names, same
/// `save`/`save_unchecked`/`validate` split — so a reviewer who knows the deck API can guess the
/// document one. `open`/`from_package` resolve only the *part graph* (which relationship points
/// where); no paragraph, run, or style is parsed until a later child's accessor asks for one.
#[derive(Debug)]
pub struct Document {
    package: Package,
    document_part: mjx_opc::PartName,
    parts: DocumentParts,
}

impl Document {
    /// Opens a document from its container bytes, resolving the main document part and its part
    /// graph.
    ///
    /// # Errors
    /// Returns [`DocxError`] if the package is unreadable, has no `officeDocument` relationship, its
    /// main document part is missing, or `word/document.xml`'s root is not `w:document`.
    pub fn open(bytes: &[u8]) -> Result<Self, DocxError> {
        Self::from_package(Package::open(bytes)?)
    }

    /// Creates a blank document: one empty paragraph and a body-level `w:sectPr` naming `size`, no
    /// styles, settings, fonts or theme related to it.
    ///
    /// Every part is authored from code (see the `blank` module) rather than unpacked from a
    /// committed template, so the markup is markup this project can explain and the same schema
    /// gate that validates an edited document validates this one. See that module's own doc comment
    /// for exactly which optional parts a blank document gets, and why it lands somewhere different
    /// from [`mjx_pptx::Presentation::blank`](https://docs.rs/mjx-pptx)'s answer to the same
    /// question.
    ///
    /// ```
    /// # fn main() -> Result<(), mjx_docx::DocxError> {
    /// use mjx_docx::{Document, PageSize};
    ///
    /// let mut document = Document::blank(PageSize::a4())?;
    /// document.append_paragraph()?;
    /// document.append_run(1, "Hello, document.")?;
    /// let bytes = document.save()?;
    /// # let _ = bytes;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns [`DocxError::InvalidPageSize`] if `size` is degenerate (see
    /// [`PageSize::validate`](crate::PageSize)'s doc comment — `word/document.xml`'s
    /// `w:pgSz`/`w:pgMar` are checked against a physically meaningful condition, not a numeric range
    /// `ST_TwipsMeasure` does not declare), or another [`DocxError`] if building the package fails.
    pub fn blank(size: crate::PageSize) -> Result<Self, DocxError> {
        Self::blank_with_properties(
            size,
            &mjx_opc::doc_props::CoreProperties::default(),
            &mjx_opc::doc_props::ExtendedProperties::default(),
        )
    }

    /// [`blank`](Self::blank) with document properties (`docProps/core.xml` / `docProps/app.xml`)
    /// set from `core_properties` / `extended_properties` rather than left absent.
    ///
    /// Both parts are always written — MJXOFF-149's packaging-layer decision, restated in
    /// `crate::blank`'s own module doc — this constructor only chooses what goes in them. Every
    /// field of both is optional, so `&CoreProperties::default()` / `&ExtendedProperties::default()`
    /// produce the same childless parts [`blank`](Self::blank) does.
    ///
    /// ```
    /// # fn main() -> Result<(), mjx_docx::DocxError> {
    /// use mjx_opc::doc_props::{CoreProperties, DocumentTimestamp, ExtendedProperties};
    /// use mjx_docx::{Document, PageSize};
    ///
    /// let created = DocumentTimestamp::new(2024, 1, 1, 0, 0, 0)?;
    /// let document = Document::blank_with_properties(
    ///     PageSize::us_letter(),
    ///     &CoreProperties {
    ///         title: Some("Quarterly Review".to_owned()),
    ///         creator: Some("mjx-ooxml-rs".to_owned()),
    ///         created: Some(created),
    ///         modified: Some(created),
    ///     },
    ///     &ExtendedProperties {
    ///         application: Some("mjx-ooxml-rs".to_owned()),
    ///     },
    /// )?;
    /// let bytes = document.save()?;
    /// # let _ = bytes;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns [`DocxError::InvalidPageSize`] if `size` is degenerate, or another [`DocxError`] if
    /// building the package fails.
    pub fn blank_with_properties(
        size: crate::PageSize,
        core_properties: &mjx_opc::doc_props::CoreProperties,
        extended_properties: &mjx_opc::doc_props::ExtendedProperties,
    ) -> Result<Self, DocxError> {
        Self::from_package(crate::blank::package(
            size,
            core_properties,
            extended_properties,
        )?)
    }

    /// Resolves an already-loaded [`Package`] into a document: the `officeDocument` relationship,
    /// the main document part, and its part graph.
    ///
    /// This is the constructor for a caller who already holds the package, exactly as
    /// `Presentation::from_package` is for a deck — see that method's own doc comment for why a
    /// package built from nothing ([`Document::blank`]) goes through this same resolution rather
    /// than surviving as a special case.
    ///
    /// # Errors
    /// Returns [`DocxError`] if the package has no `officeDocument` relationship, its main document
    /// part is missing, or `word/document.xml`'s root is not `w:document`.
    pub fn from_package(mut package: Package) -> Result<Self, DocxError> {
        let document_part = {
            let root_rels = package
                .relationships_for(None)
                .ok_or(DocxError::MissingOfficeDocument)?;
            let rel = root_rels
                .by_type(PartKind::Document.relationship_type())
                .next()
                .ok_or(DocxError::MissingOfficeDocument)?;
            if rel.mode == TargetMode::External {
                return Err(DocxError::ExternalTarget {
                    target: rel.target.clone(),
                });
            }
            resolve_from_root(&rel.target)?
        };
        if package.part_bytes(&document_part).is_none() {
            return Err(DocxError::MissingDocumentPart(
                document_part.as_str().to_owned(),
            ));
        }

        {
            let doc = package.part_tree(&document_part)?;
            let root_local = doc.interner.resolve(doc.root.name.local);
            let root_namespace = doc.root.name.namespace.map(|s| doc.interner.resolve(s));
            let is_document = root_local == "document"
                && (root_namespace == Some(WML.transitional) || root_namespace == WML.strict);
            if !is_document {
                return Err(DocxError::MalformedDocument(
                    "root element is not w:document",
                ));
            }
        }

        let parts = DocumentParts::resolve(&package, &document_part)?;

        Ok(Self {
            package,
            document_part,
            parts,
        })
    }

    /// The resolved part graph: `styles.xml`, `fontTable.xml`, and every other part this crate
    /// currently classifies that the document relates to.
    #[must_use]
    pub fn parts(&self) -> &DocumentParts {
        &self.parts
    }

    /// Reads this document's `word/styles.xml`, handing `read` the parsed [`StyleSheet`] together
    /// with the [`mjx_ooxml_core::Interner`] it was parsed with — every accessor on the returned
    /// model (a `styleId`, a `w:name`, …) needs that specific interner to resolve, so the two are
    /// never handed back separately, mirroring [`Document::edit_style_sheet`]'s own shape exactly
    /// (read-only rather than parse/mutate/write-back).
    ///
    /// Returns `None` — `read` is never called — if this document relates to no `word/styles.xml`
    /// at all (a [`Document::blank`] document, for one; see `blank.rs`'s own doc comment for why a
    /// blank document deliberately starts with no `styles.xml`).
    ///
    /// # Errors
    /// Returns [`DocxError`] if `word/styles.xml` is related but cannot be read, is not
    /// well-formed, or its root is not `w:styles`.
    pub fn style_sheet<R>(
        &mut self,
        read: impl FnOnce(&StyleSheet, &mjx_ooxml_core::Interner) -> R,
    ) -> Result<Option<R>, DocxError> {
        let Some(styles_part) = self.parts.styles.clone() else {
            return Ok(None);
        };
        let doc = self.package.part_tree(&styles_part)?;
        let sheet = StyleSheet::from_xml(&doc.root, &doc.interner)?;
        Ok(Some(read(&sheet, &doc.interner)))
    }

    /// Edits this document's style sheet, creating `word/styles.xml` — with its content-type
    /// registration and its `styles` relationship from the main document part — first if the
    /// document does not relate to one yet.
    ///
    /// `edit` receives the current (or freshly created, empty) [`StyleSheet`] and the package's
    /// [`mjx_ooxml_core::Interner`]; every [`StyleSheet`]/[`StyleDefinition`]/… setter needs both,
    /// exactly as every other typed edit in this crate does. This one primitive covers every
    /// authoring shape this child's ticket names — adding a style, modifying one (read it via the
    /// `StyleSheet` the closure receives, mutate it with its own setters), and starting a
    /// `styles.xml` a document does not yet have — without a separate `Document`-level method for
    /// each of the dozens of properties a style can carry, mirroring how `set_run_text` and
    /// `insert_paragraph` are themselves thin wrappers around exactly this parse/mutate/write-back
    /// shape for `word/document.xml`.
    ///
    /// Only `word/styles.xml` (and, the first time, `[Content_Types].xml` and
    /// `word/_rels/document.xml.rels`) is ever dirtied — every other part, and every other style
    /// inside `word/styles.xml` `edit` does not touch, keeps its original bytes (see
    /// [`mjx_ooxml_core::ToXml::write_back`]).
    ///
    /// # Errors
    /// Returns [`DocxError`] if `word/styles.xml` is related but cannot be read, or if creating a
    /// missing `word/styles.xml` fails (a malformed existing `[Content_Types].xml`/`.rels`, or a
    /// part-name collision).
    pub fn edit_style_sheet<R>(
        &mut self,
        edit: impl FnOnce(&mut StyleSheet, &mut mjx_ooxml_core::Interner) -> R,
    ) -> Result<R, DocxError> {
        let styles_part = match &self.parts.styles {
            Some(part) => part.clone(),
            None => self.create_style_sheet_part()?,
        };
        let doc = self.package.part_tree_mut(&styles_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut sheet = if root.name.local == interner.intern("styles") {
            StyleSheet::from_xml(root, interner)?
        } else {
            return Err(DocxError::MalformedDocument(
                "word/styles.xml root is not w:styles",
            ));
        };
        let result = edit(&mut sheet, interner);
        sheet.write_back(root, interner);
        Ok(result)
    }

    /// Creates an empty `word/styles.xml`, registers its content type, and relates it from the main
    /// document part — the "a document that has none" case [`Document::edit_style_sheet`] needs
    /// before it can parse anything.
    ///
    /// Written as a minimal XML string template, exactly `blank.rs`'s own `document_bytes` is —
    /// **not** through [`ToXml::to_xml`] — because a freshly built [`StyleSheet`] value has no
    /// ancestor to inherit an `xmlns:w` declaration from the way every *parsed* WML element does;
    /// [`Document::edit_style_sheet`]'s very next step re-parses these bytes through the normal
    /// `part_tree_mut`/`FromXml` path, so the typed model only ever mutates a tree it actually read.
    fn create_style_sheet_part(&mut self) -> Result<mjx_opc::PartName, DocxError> {
        let styles_part =
            self.document_part
                .resolve("styles.xml")
                .map_err(|_| DocxError::TargetResolution {
                    target: "styles.xml".to_owned(),
                })?;
        const WML_NAMESPACE: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
        let bytes = format!(
            concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                "\n",
                r#"<w:styles xmlns:w="{ns}"/>"#,
            ),
            ns = WML_NAMESPACE,
        )
        .into_bytes();
        self.package
            .insert_part(&styles_part, crate::constants::CONTENT_TYPE_STYLES, bytes)?;
        let rid = self.next_rid_for(&self.document_part.clone());
        self.package.add_relationship(
            Some(&self.document_part),
            mjx_opc::Relationship {
                id: rid,
                rel_type: crate::constants::REL_STYLES.to_owned(),
                target: "styles.xml".to_owned(),
                mode: mjx_opc::TargetMode::Internal,
            },
        )?;
        self.parts.styles = Some(styles_part.clone());
        Ok(styles_part)
    }

    /// Reads this document's `word/numbering.xml`, handing `read` the parsed [`Numbering`] together
    /// with the [`mjx_ooxml_core::Interner`] it was parsed with — mirrors
    /// [`Document::style_sheet`]'s own shape exactly.
    ///
    /// Returns `None` — `read` is never called — if this document relates to no
    /// `word/numbering.xml` at all.
    ///
    /// # Errors
    /// Returns [`DocxError`] if `word/numbering.xml` is related but cannot be read, is not
    /// well-formed, or its root is not `w:numbering`.
    pub fn numbering<R>(
        &mut self,
        read: impl FnOnce(&Numbering, &mjx_ooxml_core::Interner) -> R,
    ) -> Result<Option<R>, DocxError> {
        let Some(numbering_part) = self.parts.numbering.clone() else {
            return Ok(None);
        };
        let doc = self.package.part_tree(&numbering_part)?;
        let numbering = Numbering::from_xml(&doc.root, &doc.interner)?;
        Ok(Some(read(&numbering, &doc.interner)))
    }

    /// Edits this document's numbering definitions, creating `word/numbering.xml` — with its
    /// content-type registration and its `numbering` relationship from the main document part —
    /// first if the document does not relate to one yet. Mirrors
    /// [`Document::edit_style_sheet`]'s own shape exactly.
    ///
    /// # Errors
    /// Returns [`DocxError`] if `word/numbering.xml` is related but cannot be read, or if creating a
    /// missing `word/numbering.xml` fails (a malformed existing `[Content_Types].xml`/`.rels`, or a
    /// part-name collision).
    pub fn edit_numbering<R>(
        &mut self,
        edit: impl FnOnce(&mut Numbering, &mut mjx_ooxml_core::Interner) -> R,
    ) -> Result<R, DocxError> {
        let numbering_part = match &self.parts.numbering {
            Some(part) => part.clone(),
            None => self.create_numbering_part()?,
        };
        let doc = self.package.part_tree_mut(&numbering_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut numbering = if root.name.local == interner.intern("numbering") {
            Numbering::from_xml(root, interner)?
        } else {
            return Err(DocxError::MalformedDocument(
                "word/numbering.xml root is not w:numbering",
            ));
        };
        let result = edit(&mut numbering, interner);
        numbering.write_back(root, interner);
        Ok(result)
    }

    /// Creates an empty `word/numbering.xml`, registers its content type, and relates it from the
    /// main document part — mirrors [`Document::create_style_sheet_part`] exactly.
    fn create_numbering_part(&mut self) -> Result<mjx_opc::PartName, DocxError> {
        let numbering_part = self.document_part.resolve("numbering.xml").map_err(|_| {
            DocxError::TargetResolution {
                target: "numbering.xml".to_owned(),
            }
        })?;
        const WML_NAMESPACE: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
        let bytes = format!(
            concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                "\n",
                r#"<w:numbering xmlns:w="{ns}"/>"#,
            ),
            ns = WML_NAMESPACE,
        )
        .into_bytes();
        self.package.insert_part(
            &numbering_part,
            crate::constants::CONTENT_TYPE_NUMBERING,
            bytes,
        )?;
        let rid = self.next_rid_for(&self.document_part.clone());
        self.package.add_relationship(
            Some(&self.document_part),
            mjx_opc::Relationship {
                id: rid,
                rel_type: crate::constants::REL_NUMBERING.to_owned(),
                target: "numbering.xml".to_owned(),
                mode: mjx_opc::TargetMode::Internal,
            },
        )?;
        self.parts.numbering = Some(numbering_part.clone());
        Ok(numbering_part)
    }

    /// Resolves `numbering_id`/`level` through both indirection hops — see `numbering.rs`'s own
    /// module doc — including a `w:numStyleLink` redirect through `word/styles.xml` when the
    /// resolved abstract definition carries one. `read` receives the [`NumberingLookup`] together
    /// with the [`mjx_ooxml_core::Interner`] its borrowed data (when any) was parsed with.
    ///
    /// `numbering_id = 0` always resolves to [`NumberingLookup::None`] — checked before this method
    /// looks at whether `word/numbering.xml` is even related, since `0` is "no numbering" regardless
    /// (see the module's own doc comment). Any other `numbering_id`, when the document relates to no
    /// `word/numbering.xml` at all, is [`DocxError::UnknownNumberingId`] — the same error a `numId`
    /// unresolvable *within* an existing part reports, since from a caller's perspective both mean
    /// exactly the same thing: the referenced list definition cannot be found.
    ///
    /// Each redirect hop parses `word/numbering.xml` and, if needed, `word/styles.xml` in turn —
    /// never two parts' fidelity trees at once (each [`mjx_opc::Package::part_tree`] borrow ends
    /// before the next begins; [`mjx_ooxml_core::Interner`]s are per-part and are not merged across
    /// this boundary) — bounded by [`MAX_NUM_STYLE_LINK_DEPTH`] hops.
    ///
    /// # Errors
    /// Returns [`DocxError::UnknownNumberingId`], [`DocxError::UnknownAbstractNumberingId`],
    /// [`DocxError::MissingAbstractNumberingReference`] (see [`NumberingIndex::resolve`] for all
    /// three), [`DocxError::NumberingStyleLinkTargetMissing`],
    /// [`DocxError::NumberingStyleLinkWrongKind`], [`DocxError::NumberingStyleLinkHasNoNumbering`] or
    /// [`DocxError::NumberingStyleLinkTooDeep`] for a `w:numStyleLink` redirect that cannot be
    /// followed, or another [`DocxError`] if a related part cannot be read.
    pub fn resolve_numbering<R>(
        &mut self,
        numbering_id: i64,
        level: i64,
        read: impl FnOnce(&NumberingLookup<'_>, &mjx_ooxml_core::Interner) -> R,
    ) -> Result<R, DocxError> {
        if numbering_id == 0 {
            let doc = self.package.part_tree(&self.document_part)?;
            return Ok(read(&NumberingLookup::None, &doc.interner));
        }

        let mut current_id = numbering_id;
        let mut hops = 0usize;
        loop {
            if hops > MAX_NUM_STYLE_LINK_DEPTH {
                return Err(DocxError::NumberingStyleLinkTooDeep {
                    numbering_id,
                    limit: MAX_NUM_STYLE_LINK_DEPTH,
                });
            }
            let Some(numbering_part) = self.parts.numbering.clone() else {
                return Err(DocxError::UnknownNumberingId(current_id));
            };
            let doc = self.package.part_tree(&numbering_part)?;
            let numbering = Numbering::from_xml(&doc.root, &doc.interner)?;
            let index = NumberingIndex::build(&numbering, &doc.interner)?;
            let lookup = index.resolve(current_id, level, &doc.interner)?;

            let redirect_style_id = match &lookup {
                NumberingLookup::None => None,
                NumberingLookup::Resolved(resolution) => resolution
                    .abstract_definition()
                    .numbering_style_link()
                    .map(|link| link.value(&doc.interner))
                    .transpose()
                    .map_err(FromXmlError::from)?
                    .map(std::borrow::Cow::into_owned),
            };
            let Some(style_id) = redirect_style_id else {
                return Ok(read(&lookup, &doc.interner));
            };

            let Some(styles_part) = self.parts.styles.clone() else {
                return Err(DocxError::NumberingStyleLinkTargetMissing { style_id });
            };
            let doc = self.package.part_tree(&styles_part)?;
            let sheet = StyleSheet::from_xml(&doc.root, &doc.interner)?;
            let style_index = StyleIndex::build(&sheet, &doc.interner)?;
            let style = style_index.style_by_id(&style_id).ok_or_else(|| {
                DocxError::NumberingStyleLinkTargetMissing {
                    style_id: style_id.clone(),
                }
            })?;
            let kind = style.kind(&doc.interner).map_err(FromXmlError::from)?;
            if kind != Some(mjx_ooxml_types::wordprocessingml::StyleType::Numbering) {
                return Err(DocxError::NumberingStyleLinkWrongKind {
                    style_id,
                    found: kind,
                });
            }
            let next_id = style
                .paragraph_properties()
                .and_then(StyleParagraphProperties::numbering)
                .map(|reference| reference.numbering_id(&doc.interner))
                .transpose()
                .map_err(FromXmlError::from)?
                .flatten()
                .ok_or_else(|| DocxError::NumberingStyleLinkHasNoNumbering {
                    style_id: style_id.clone(),
                })?;
            current_id = next_id;
            hops += 1;
        }
    }

    /// Attaches the paragraph at `paragraph` to the numbering instance `numbering_id` at level
    /// `level` (`w:numPr/w:numId` and `w:numPr/w:ilvl`), replacing any numbering reference it already
    /// carried.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if `paragraph` does not address a paragraph.
    pub fn attach_paragraph_to_list(
        &mut self,
        paragraph: impl Into<BlockPath>,
        numbering_id: i64,
        level: i64,
    ) -> Result<(), DocxError> {
        let paragraph_path = paragraph.into();
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        let properties = paragraph.properties_or_insert(interner);
        let mut numbering = NumberingProperties::new(interner);
        numbering.set_level(interner, Some(level));
        numbering.set_numbering_id(interner, Some(numbering_id));
        properties.set_numbering(Some(numbering));
        main.write_back(root, interner);
        Ok(())
    }

    /// Removes the paragraph at `paragraph`'s own `w:numPr`, if it carries one (a no-op otherwise).
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if `paragraph` does not address a paragraph.
    pub fn detach_paragraph_from_list(
        &mut self,
        paragraph: impl Into<BlockPath>,
    ) -> Result<(), DocxError> {
        let paragraph_path = paragraph.into();
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        if let Some(properties) = paragraph.properties_mut() {
            properties.set_numbering(None);
        }
        main.write_back(root, interner);
        Ok(())
    }

    /// Every section this document has, in document order — see [`SectionSpan`]'s own doc comment
    /// for why a section's properties live at the *end* of the range it governs, not the start.
    /// `read` receives the spans together with the [`mjx_ooxml_core::Interner`] every
    /// [`SectionProperties`] accessor among them needs, mirroring [`Document::style_sheet`]'s own
    /// shape exactly.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or another [`DocxError`] if
    /// the main document part cannot be read.
    pub fn sections<R>(
        &mut self,
        read: impl FnOnce(&[SectionSpan], &mjx_ooxml_core::Interner) -> R,
    ) -> Result<R, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let spans = sections::sections_in(body);
        Ok(read(&spans, &doc.interner))
    }

    /// Edits the `w:sectPr` at `location`, creating an empty one first if it does not already exist
    /// — the one primitive behind both "change an existing section's properties" (call on a
    /// [`SectionLocation`] that already carries a `w:sectPr`) and "split the document into a new
    /// section" (call on a paragraph that carries none yet: the new `w:sectPr` lands inside *that*
    /// paragraph's own `w:pPr`, ending a section there, exactly where MJXOFF-109's own ticket
    /// requires it — never appended to the body).
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if [`SectionLocation::Paragraph`] does not address a
    /// paragraph.
    pub fn edit_section_properties<R>(
        &mut self,
        location: SectionLocation,
        edit: impl FnOnce(&mut SectionProperties, &mut mjx_ooxml_core::Interner) -> R,
    ) -> Result<R, DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let result = match location {
            SectionLocation::Body => {
                let properties = body.section_properties_or_insert(interner);
                edit(properties, interner)
            }
            SectionLocation::Paragraph(path) => {
                let paragraph = body
                    .paragraph_mut(&path)
                    .ok_or_else(|| DocxError::AddressNotFound(format!("no paragraph at {path}")))?;
                let properties = paragraph
                    .properties_or_insert(interner)
                    .section_properties_or_insert(interner);
                edit(properties, interner)
            }
        };
        main.write_back(root, interner);
        Ok(result)
    }

    /// Removes the `w:sectPr` at `location`, if it carries one (a no-op otherwise) — "removing a
    /// section": a paragraph's own section break disappears and its former range joins whatever
    /// section follows it (see [`SectionSpan`]'s own doc comment).
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if [`SectionLocation::Paragraph`] does not address a
    /// paragraph.
    pub fn remove_section_properties(
        &mut self,
        location: SectionLocation,
    ) -> Result<(), DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        match location {
            SectionLocation::Body => body.set_section_properties(None),
            SectionLocation::Paragraph(path) => {
                let paragraph = body
                    .paragraph_mut(&path)
                    .ok_or_else(|| DocxError::AddressNotFound(format!("no paragraph at {path}")))?;
                if let Some(properties) = paragraph.properties_mut() {
                    properties.set_section_properties(None);
                }
            }
        }
        main.write_back(root, interner);
        Ok(())
    }

    // -------------------------------------------------------------------------------------------
    // Headers and footers (MJXOFF-113) — variant resolution, reading/editing their own content, and
    // creating/removing them on demand.
    // -------------------------------------------------------------------------------------------

    /// Whether this document's sections use different headers/footers for even and odd pages
    /// (`w:settings/w:evenAndOddHeaders`), read directly from `word/settings.xml` — MJXOFF-136 models
    /// that part as a whole; this crate reads only the one flag [`Document::resolve_header`]/
    /// `resolve_footer` need, exactly as this child's own ticket asks. `false` (the schema default)
    /// if the document relates to no `word/settings.xml`, or if that part carries no
    /// `w:evenAndOddHeaders` at all.
    ///
    /// # Errors
    /// Returns [`DocxError`] if `word/settings.xml` is related but cannot be read.
    pub fn even_and_odd_headers(&mut self) -> Result<bool, DocxError> {
        let Some(settings_part) = self.parts.settings.clone() else {
            return Ok(false);
        };
        let doc = self.package.part_tree(&settings_part)?;
        let found = doc.root.children.iter().find_map(|node| match node {
            RawNode::Element(element)
                if is_wml_element(element, &doc.interner, "evenAndOddHeaders") =>
            {
                Some(element)
            }
            _ => None,
        });
        let Some(element) = found else {
            return Ok(false);
        };
        let toggle = Toggle::from_xml(element, &doc.interner)?;
        Ok(toggle.value(&doc.interner).map_err(FromXmlError::from)?)
    }

    /// Resolves which header part actually applies to `section_index`'s pages of variant `kind` —
    /// see `crate::document::headers`'s own doc comment for the ECMA-376 Part 1 rules this
    /// implements (`w:titlePg`, `w:evenAndOddHeaders`, and inheritance from the previous section).
    /// `None` when no section from `section_index` back to the document's first states a reference of
    /// the resolved variant (real Word would create a blank one; this crate does not fabricate one on
    /// a read — see [`Document::create_header`]).
    ///
    /// # Errors
    /// Returns [`DocxError::SectionOutOfRange`] if `section_index` names no section,
    /// [`DocxError::NoBody`] if the document declares no body, or another [`DocxError`] if a related
    /// part cannot be read or a reference this walk reads is malformed.
    pub fn resolve_header(
        &mut self,
        section_index: usize,
        kind: HeaderFooterType,
    ) -> Result<Option<mjx_opc::PartName>, DocxError> {
        self.resolve_header_footer(section_index, kind, true)
    }

    /// As [`Document::resolve_header`], for footers (§17.10.2, identical rules).
    ///
    /// # Errors
    /// See [`Document::resolve_header`].
    pub fn resolve_footer(
        &mut self,
        section_index: usize,
        kind: HeaderFooterType,
    ) -> Result<Option<mjx_opc::PartName>, DocxError> {
        self.resolve_header_footer(section_index, kind, false)
    }

    fn resolve_header_footer(
        &mut self,
        section_index: usize,
        kind: HeaderFooterType,
        is_header: bool,
    ) -> Result<Option<mjx_opc::PartName>, DocxError> {
        let even_and_odd_headers = self.even_and_odd_headers()?;
        let rel_id = self.sections(|spans, interner| {
            headers::resolve_reference(
                spans,
                section_index,
                kind,
                even_and_odd_headers,
                interner,
                is_header,
            )
        })??;
        match rel_id {
            Some(rel_id) => self.part_for_document_rel(&rel_id).map(Some),
            None => Ok(None),
        }
    }

    /// Resolves relationship `id` in the main document part's own `.rels` to a part name.
    ///
    /// # Errors
    /// Returns [`DocxError::ExternalTarget`] if the relationship targets outside the package, or
    /// [`DocxError::TargetResolution`] if `id` names no relationship or its target does not resolve.
    fn part_for_document_rel(&self, id: &str) -> Result<mjx_opc::PartName, DocxError> {
        let rel = self
            .package
            .relationships_for(Some(&self.document_part))
            .and_then(|rels| rels.by_id(id))
            .ok_or_else(|| DocxError::TargetResolution {
                target: id.to_owned(),
            })?;
        if rel.mode == TargetMode::External {
            return Err(DocxError::ExternalTarget {
                target: rel.target.clone(),
            });
        }
        self.document_part
            .resolve(&rel.target)
            .map_err(|_| DocxError::TargetResolution {
                target: rel.target.clone(),
            })
    }

    /// Reads a header or footer part's content, handing `read` the parsed [`HdrFtr`] together with
    /// the [`mjx_ooxml_core::Interner`] it was parsed with — mirrors [`Document::style_sheet`]'s own
    /// shape. `part` is one of [`DocumentParts::headers`]/`footers`, or a part
    /// [`Document::resolve_header`]/`resolve_footer`/`create_header`/`create_footer` named.
    ///
    /// This is how MJXOFF-92's paragraph/run model, MJXOFF-94/96's properties and MJXOFF-106's
    /// effective-property ladder reach inside a header or footer: none of the three reaches into
    /// `Body`/`HdrFtr` themselves, only into the [`Paragraph`]/[`Run`] a caller already holds — so
    /// `read`'s closure uses [`HdrFtr::paragraph`]/[`Paragraph::run`]/[`Paragraph::properties`]/…
    /// exactly as it would against a [`Document::sections`]-obtained paragraph.
    ///
    /// # Errors
    /// Returns [`DocxError`] if `part` cannot be read, is not well-formed, or its root is not
    /// `w:hdr`/`w:ftr`.
    pub fn header_footer<R>(
        &mut self,
        part: &mjx_opc::PartName,
        read: impl FnOnce(&HdrFtr, &mjx_ooxml_core::Interner) -> R,
    ) -> Result<R, DocxError> {
        let doc = self.package.part_tree(part)?;
        check_header_footer_root(&doc.root, &doc.interner)?;
        let content = HdrFtr::from_xml(&doc.root, &doc.interner)?;
        Ok(read(&content, &doc.interner))
    }

    /// Edits a header or footer part's content in place — mirrors [`Document::edit_style_sheet`]'s
    /// own shape. Unlike style sheets and numbering definitions, this never creates the part itself:
    /// use [`Document::create_header`]/[`Document::create_footer`] first.
    ///
    /// # Errors
    /// As [`Document::header_footer`].
    pub fn edit_header_footer<R>(
        &mut self,
        part: &mjx_opc::PartName,
        edit: impl FnOnce(&mut HdrFtr, &mut mjx_ooxml_core::Interner) -> R,
    ) -> Result<R, DocxError> {
        let doc = self.package.part_tree_mut(part)?;
        let RawDocument { interner, root, .. } = doc;
        check_header_footer_root(root, interner)?;
        let mut content = HdrFtr::from_xml(root, interner)?;
        let result = edit(&mut content, interner);
        content.write_back(root, interner);
        Ok(result)
    }

    /// The VML content of every `w:pict` a header or footer part carries — see
    /// `crate::document::headers::vml_drawings_in`'s own doc comment for how `mc:AlternateContent` is
    /// resolved (non-mutatingly, via `mjx-mce`) and why `w:pict` itself is read directly rather than
    /// through [`super::RunInnerContent`]. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`DocxError`] if `part` cannot be read, [`DocxError::Mce`] if its `mc:AlternateContent`
    /// markup is malformed, or [`DocxError::Vml`] if a `w:pict` this walk finds does not parse as VML.
    pub fn header_footer_vml_drawings(
        &mut self,
        part: &mjx_opc::PartName,
    ) -> Result<Vec<mjx_vml::Drawing>, DocxError> {
        let doc = self.package.part_tree(part)?;
        headers::vml_drawings_in(doc)
    }

    /// Creates a new header part of `kind` for the section at `location`, wiring
    /// `w:headerReference` into that section's `w:sectPr` at its schema rank — creating an empty
    /// `w:sectPr` first if the section carries none, exactly as [`Document::edit_section_properties`]
    /// does. The new part holds one empty paragraph; edit it with [`Document::edit_header_footer`].
    ///
    /// If the section already names a header of this `kind`, the old reference is replaced by the
    /// new one; the old part and its own relationship are left in the package (call
    /// [`Document::remove_header`] first if they should not survive) — this method never removes
    /// content a caller has not asked it to remove.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body,
    /// [`DocxError::AddressNotFound`] if [`SectionLocation::Paragraph`] does not address a paragraph,
    /// or another [`DocxError`] if the package edit fails.
    pub fn create_header(
        &mut self,
        location: SectionLocation,
        kind: HeaderFooterType,
    ) -> Result<mjx_opc::PartName, DocxError> {
        self.create_header_footer(location, kind, true)
    }

    /// As [`Document::create_header`], for footers.
    ///
    /// # Errors
    /// See [`Document::create_header`].
    pub fn create_footer(
        &mut self,
        location: SectionLocation,
        kind: HeaderFooterType,
    ) -> Result<mjx_opc::PartName, DocxError> {
        self.create_header_footer(location, kind, false)
    }

    fn create_header_footer(
        &mut self,
        location: SectionLocation,
        kind: HeaderFooterType,
        is_header: bool,
    ) -> Result<mjx_opc::PartName, DocxError> {
        let (part_kind, local, stem) = if is_header {
            (PartKind::Header, "hdr", "header")
        } else {
            (PartKind::Footer, "ftr", "footer")
        };
        let (part, target) = self.next_header_footer_part(stem)?;
        self.package.insert_part(
            &part,
            part_kind.content_type(),
            headers::initial_bytes(local),
        )?;
        let rid = self.next_rid_for(&self.document_part.clone());
        self.package.add_relationship(
            Some(&self.document_part),
            mjx_opc::Relationship {
                id: rid.clone(),
                rel_type: part_kind.relationship_type().to_owned(),
                target,
                mode: mjx_opc::TargetMode::Internal,
            },
        )?;
        if is_header {
            self.parts.headers.push(part.clone());
        } else {
            self.parts.footers.push(part.clone());
        }

        let element_local = if is_header {
            "headerReference"
        } else {
            "footerReference"
        };
        self.edit_section_properties(
            location,
            |properties, interner| -> Result<(), FromXmlError> {
                if is_header {
                    properties.remove_header_reference(kind, interner)?;
                } else {
                    properties.remove_footer_reference(kind, interner)?;
                }
                let reference = HeaderFooterReference::new(interner, element_local, &rid, kind);
                if is_header {
                    properties.push_header_reference(reference);
                } else {
                    properties.push_footer_reference(reference);
                }
                Ok(())
            },
        )??;

        Ok(part)
    }

    /// The `word/{stem}N.xml` part name one past the highest `N` already in the package (so a package
    /// with `header1.xml` and `header3.xml` gets `header4.xml`, never colliding with either), and its
    /// relationship target relative to the main document part.
    fn next_header_footer_part(
        &self,
        stem: &str,
    ) -> Result<(mjx_opc::PartName, String), DocxError> {
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            let file_name = part.as_str().rsplit('/').next().unwrap_or("");
            if let Some(digits) = file_name
                .strip_prefix(stem)
                .and_then(|rest| rest.strip_suffix(".xml"))
            {
                if let Ok(n) = digits.parse::<u32>() {
                    max_n = max_n.max(n);
                }
            }
        }
        let target = format!("{stem}{}.xml", max_n + 1);
        let part =
            self.document_part
                .resolve(&target)
                .map_err(|_| DocxError::TargetResolution {
                    target: target.clone(),
                })?;
        Ok((part, target))
    }

    /// Removes the section at `location`'s own `kind` header reference, if it states one (a no-op
    /// otherwise), and — unless another `w:headerReference` anywhere in the document still names the
    /// same part — sweeps the now-unreferenced part and its relationship
    /// ([`mjx_opc::Package::remove_unreferenced_parts`]).
    ///
    /// Unlike [`Document::create_header`], this never creates a `w:sectPr` the section did not
    /// already have: removing a reference from a section with none (or from one that carries a
    /// `w:sectPr` naming no reference of this `kind`) is simply nothing to do.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body,
    /// [`DocxError::AddressNotFound`] if [`SectionLocation::Paragraph`] does not address a paragraph,
    /// or another [`DocxError`] if the package edit fails.
    pub fn remove_header(
        &mut self,
        location: SectionLocation,
        kind: HeaderFooterType,
    ) -> Result<(), DocxError> {
        self.remove_header_footer(location, kind, true)
    }

    /// As [`Document::remove_header`], for footers.
    ///
    /// # Errors
    /// See [`Document::remove_header`].
    pub fn remove_footer(
        &mut self,
        location: SectionLocation,
        kind: HeaderFooterType,
    ) -> Result<(), DocxError> {
        self.remove_header_footer(location, kind, false)
    }

    fn remove_header_footer(
        &mut self,
        location: SectionLocation,
        kind: HeaderFooterType,
        is_header: bool,
    ) -> Result<(), DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let properties = match &location {
            SectionLocation::Body => body.section_properties_mut(),
            SectionLocation::Paragraph(path) => {
                let paragraph = body
                    .paragraph_mut(path)
                    .ok_or_else(|| DocxError::AddressNotFound(format!("no paragraph at {path}")))?;
                paragraph
                    .properties_mut()
                    .and_then(paragraph_properties::ParagraphProperties::section_properties_mut)
            }
        };
        let removed = match properties {
            Some(properties) if is_header => properties
                .remove_header_reference(kind, interner)
                .map_err(FromXmlError::from)?,
            Some(properties) => properties
                .remove_footer_reference(kind, interner)
                .map_err(FromXmlError::from)?,
            None => None,
        };
        main.write_back(root, interner);
        let Some(reference) = removed else {
            return Ok(());
        };
        let rel_id = reference
            .relationship_id(interner)
            .map_err(FromXmlError::from)?
            .into_owned();
        self.package
            .remove_relationship(Some(&self.document_part), &rel_id)
            .map_err(DocxError::from)?;
        self.package
            .remove_unreferenced_parts()
            .map_err(DocxError::from)?;
        self.parts = parts::DocumentParts::resolve(&self.package, &self.document_part)?;
        Ok(())
    }

    /// The next free relationship id (`rId{N}`) in `part`'s `.rels`, one past the current maximum —
    /// `rId1` when the part has no relationships yet.
    fn next_rid_for(&self, part: &mjx_opc::PartName) -> String {
        let mut max_n = 0u32;
        if let Some(rels) = self.package.relationships_for(Some(part)) {
            for rel in rels.iter() {
                if let Some(n) = rel
                    .id
                    .strip_prefix("rId")
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    max_n = max_n.max(n);
                }
            }
        }
        format!("rId{}", max_n + 1)
    }

    /// The document's conformance class (`w:document/@conformance`) — `Strict` or `Transitional`, or
    /// `None` if the attribute is absent (every fixture in this workspace is Transitional and omits
    /// it, which is legal: absence is not a claim either way).
    ///
    /// # Errors
    /// Returns [`DocxError::Opc`] if the main document part cannot be read, or
    /// [`DocxError::Model`] if `@conformance` is present but not a value `ST_ConformanceClass`
    /// recognizes.
    pub fn conformance(&mut self) -> Result<Option<ConformanceClass>, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        Ok(main
            .conformance(&doc.interner)
            .map_err(FromXmlError::from)?)
    }

    /// Sets (or, given `None`, removes) `w:document/@conformance`.
    ///
    /// # Errors
    /// Returns [`DocxError`] if the main document part cannot be read.
    pub fn set_conformance(&mut self, value: Option<ConformanceClass>) -> Result<(), DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        main.set_conformance(interner, value);
        main.write_back(root, interner);
        Ok(())
    }

    /// How many paragraphs `w:body` holds, or `0` if the document declares no body.
    ///
    /// # Errors
    /// Returns [`DocxError`] if the main document part cannot be read.
    pub fn paragraph_count(&mut self) -> Result<usize, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        Ok(main.body().map_or(0, Body::paragraph_count))
    }

    /// How many run-or-hyperlink slots the paragraph at `path` holds at its top level — see
    /// [`Paragraph::run_count`] for what that counts.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if `path` does not address a paragraph.
    pub fn run_count(&mut self, paragraph: impl Into<BlockPath>) -> Result<usize, DocxError> {
        Ok(self.resolve_paragraph(paragraph.into())?.run_count())
    }

    /// The whole text of the paragraph at `path` — every run reachable from it, including runs
    /// nested inside a `w:hyperlink`, concatenated in document order.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if `path` does not address a paragraph.
    pub fn paragraph_text(&mut self, paragraph: impl Into<BlockPath>) -> Result<String, DocxError> {
        Ok(self.resolve_paragraph(paragraph.into())?.text())
    }

    /// The text of the run at `run` within the paragraph at `paragraph` — the concatenation of every
    /// `w:t` the run holds (see [`Run::text`] for why `w:delText`/`w:instrText` are not included).
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if either address does not resolve.
    pub fn run_text(
        &mut self,
        paragraph: impl Into<BlockPath>,
        run: impl Into<RunPath>,
    ) -> Result<String, DocxError> {
        let run_path = run.into();
        let paragraph = self.resolve_paragraph(paragraph.into())?;
        let run = paragraph
            .run(&run_path)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no run at {run_path}")))?;
        Ok(run.text())
    }

    /// Sets the text of the run at `run` within the paragraph at `paragraph` (see [`Run::set_text`]
    /// for the `xml:space` rule this applies). Only `word/document.xml` is dirtied, and — because
    /// this goes through [`ToXml::write_back`] — only the byte range containing the edited run
    /// actually re-serializes; every sibling paragraph and run keeps its original bytes.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if either address does not resolve.
    pub fn set_run_text(
        &mut self,
        paragraph: impl Into<BlockPath>,
        run: impl Into<RunPath>,
        text: &str,
    ) -> Result<(), DocxError> {
        let paragraph_path = paragraph.into();
        let run_path = run.into();
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        let run = paragraph
            .run_mut(&run_path)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no run at {run_path}")))?;
        run.set_text(interner, text);
        main.write_back(root, interner);
        Ok(())
    }

    /// Inserts a new, empty paragraph so it becomes the paragraph at `at`, shifting every paragraph
    /// at or after that position one place later. `at` must address an existing paragraph or the one
    /// past the last (`0..=paragraph_count()`).
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if `at` is out of range.
    pub fn insert_paragraph(&mut self, at: impl Into<BlockPath>) -> Result<(), DocxError> {
        let at = at.into();
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let paragraph = Paragraph::new(interner);
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        if !body.insert_paragraph(&at, paragraph) {
            return Err(DocxError::AddressNotFound(format!(
                "no paragraph slot at {at}"
            )));
        }
        main.write_back(root, interner);
        Ok(())
    }

    /// Appends a new, empty paragraph as the body's new last paragraph (before `w:sectPr`, when the
    /// body has one).
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body.
    pub fn append_paragraph(&mut self) -> Result<(), DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let paragraph = Paragraph::new(interner);
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        body.append_paragraph(paragraph);
        main.write_back(root, interner);
        Ok(())
    }

    /// Removes the paragraph at `at`.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if `at` does not address a paragraph.
    pub fn remove_paragraph(&mut self, at: impl Into<BlockPath>) -> Result<(), DocxError> {
        let at = at.into();
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        if body.remove_paragraph(&at).is_none() {
            return Err(DocxError::AddressNotFound(format!("no paragraph at {at}")));
        }
        main.write_back(root, interner);
        Ok(())
    }

    /// Inserts a new run holding `text` so it becomes the top-level run-or-hyperlink slot `at`
    /// within the paragraph at `paragraph`, shifting every slot at or after that position one place
    /// later. `at` must address an existing slot or the one past the last (`0..=run_count()`).
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if either address is out of range.
    pub fn insert_run(
        &mut self,
        paragraph: impl Into<BlockPath>,
        at: impl Into<RunPath>,
        text: &str,
    ) -> Result<(), DocxError> {
        let paragraph_path = paragraph.into();
        let at = at.into();
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let run = Run::with_text(interner, text);
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        if !paragraph.insert_run(&at, run) {
            return Err(DocxError::AddressNotFound(format!("no run slot at {at}")));
        }
        main.write_back(root, interner);
        Ok(())
    }

    /// Appends a new run holding `text` as the paragraph's new last top-level run.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if `paragraph` does not address a paragraph.
    pub fn append_run(
        &mut self,
        paragraph: impl Into<BlockPath>,
        text: &str,
    ) -> Result<(), DocxError> {
        let paragraph_path = paragraph.into();
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let run = Run::with_text(interner, text);
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        paragraph.append_run(run);
        main.write_back(root, interner);
        Ok(())
    }

    /// Removes the run at `run` within the paragraph at `paragraph`.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if either address does not resolve.
    pub fn remove_run(
        &mut self,
        paragraph: impl Into<BlockPath>,
        run: impl Into<RunPath>,
    ) -> Result<(), DocxError> {
        let paragraph_path = paragraph.into();
        let run_path = run.into();
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        if paragraph.remove_run(&run_path).is_none() {
            return Err(DocxError::AddressNotFound(format!("no run at {run_path}")));
        }
        main.write_back(root, interner);
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Tables (MJXOFF-116) — a top-level table is addressed by a plain index (independent of the
    // paragraph index space: a table interleaved between two paragraphs shifts neither). `(row,
    // column)` addressing inside a table mirrors `mjx_pptx::Presentation`'s own naming, argument
    // order and return shape (`crates/mjx-pptx/src/presentation/tables.rs`) — see `tables.rs`'s own
    // doc comment for how the underlying markup differs.
    // -----------------------------------------------------------------------------------------

    /// How many top-level tables `w:body` holds, or `0` if the document declares no body.
    ///
    /// # Errors
    /// Returns [`DocxError`] if the main document part cannot be read.
    pub fn table_count(&mut self) -> Result<usize, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        Ok(main.body().map_or(0, Body::table_count))
    }

    /// The shape of the table at top-level index `table`, as `(rows, columns)`. The column count
    /// comes from the table's `w:tblGrid`, not from counting some row's cells.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if `table` does not address a table.
    pub fn table_dimensions(&mut self, table: usize) -> Result<(usize, usize), DocxError> {
        self.with_table(table, |table, _interner| {
            Ok((table.row_count(), table.column_count()))
        })
    }

    /// How many rows and columns the cell at `(row, column)` of table `table` spans, as `(rows,
    /// columns)` — see [`tables::Table::cell_span`] for the full contract.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`]/[`DocxError::AddressNotFound`] as [`table_dimensions`
    /// ](Self::table_dimensions), or [`DocxError::TableCellOutOfRange`] if `(row, column)` is out of
    /// range.
    pub fn cell_span(
        &mut self,
        table: usize,
        row: usize,
        column: usize,
    ) -> Result<(usize, usize), DocxError> {
        self.with_table(table, |table, interner| {
            table
                .cell_span(interner, row, column)
                .ok_or(DocxError::TableCellOutOfRange {
                    row,
                    column,
                    rows: table.row_count(),
                    columns: table.column_count(),
                })
        })
    }

    /// Which cell actually renders at `(row, column)` of table `table` — see
    /// [`tables::Table::merge_anchor`] for the full contract, including the malformed-grid case.
    ///
    /// # Errors
    /// As [`cell_span`](Self::cell_span).
    pub fn merged_cell_anchor(
        &mut self,
        table: usize,
        row: usize,
        column: usize,
    ) -> Result<(usize, usize), DocxError> {
        self.with_table(table, |table, interner| {
            table
                .merge_anchor(interner, row, column)
                .ok_or(DocxError::TableCellOutOfRange {
                    row,
                    column,
                    rows: table.row_count(),
                    columns: table.column_count(),
                })
        })
    }

    /// Every grid discrepancy table `table` currently has — see
    /// [`tables::Table::grid_discrepancies`].
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions).
    pub fn table_grid_discrepancies(
        &mut self,
        table: usize,
    ) -> Result<Vec<tables::GridDiscrepancy>, DocxError> {
        self.with_table(table, |table, interner| {
            Ok(table.grid_discrepancies(interner))
        })
    }

    /// The text of the cell at `(row, column)` of table `table` — its direct paragraphs' text,
    /// joined by a newline.
    ///
    /// # Errors
    /// As [`cell_span`](Self::cell_span).
    pub fn cell_text(
        &mut self,
        table: usize,
        row: usize,
        column: usize,
    ) -> Result<String, DocxError> {
        self.with_table(table, |table, interner| {
            table.cell(interner, row, column).map(Cell::text).ok_or(
                DocxError::TableCellOutOfRange {
                    row,
                    column,
                    rows: table.row_count(),
                    columns: table.column_count(),
                },
            )
        })
    }

    /// Sets the text of the cell at `(row, column)` of table `table`: replaces its first direct
    /// paragraph's runs with a single run holding `text` (appending a fresh paragraph first if the
    /// cell holds none). Only `word/document.xml` is dirtied, and only the edited cell's own byte
    /// range re-serializes — every other row and cell keeps its original bytes.
    ///
    /// # Errors
    /// As [`cell_span`](Self::cell_span).
    pub fn set_cell_text(
        &mut self,
        table: usize,
        row: usize,
        column: usize,
        text: &str,
    ) -> Result<(), DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let (rows, columns) = body
            .table(table)
            .map(|table| (table.row_count(), table.column_count()))
            .ok_or_else(|| DocxError::AddressNotFound(format!("no table at index {table}")))?;
        let table_ref = body
            .table_mut(table)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no table at index {table}")))?;
        let cell =
            table_ref
                .cell_mut(interner, row, column)
                .ok_or(DocxError::TableCellOutOfRange {
                    row,
                    column,
                    rows,
                    columns,
                })?;
        if cell.paragraph_count() == 0 {
            cell.append_paragraph(Paragraph::new(interner));
        }
        let paragraph = match cell.paragraph_mut(0) {
            Some(paragraph) => paragraph,
            None => unreachable!("just ensured at least one paragraph above"),
        };
        while paragraph.run_count() > 0 {
            paragraph.remove_run(paragraph.run_count() - 1);
        }
        paragraph.append_run(Run::with_text(interner, text));
        main.write_back(root, interner);
        Ok(())
    }

    /// Sets (or, given `None`/`Some(1)`, removes) the `w:gridSpan` of the cell at `(row, column)` of
    /// table `table` — how many grid columns it covers. `(row, column)` addresses the cell *before*
    /// the change takes effect; growing or shrinking a span shifts which physical cell later queries
    /// at `column + 1` resolve to, exactly as authoring a merge does in Word itself.
    ///
    /// # Errors
    /// As [`cell_span`](Self::cell_span).
    pub fn set_cell_span(
        &mut self,
        table: usize,
        row: usize,
        column: usize,
        span: Option<usize>,
    ) -> Result<(), DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let (rows, columns) = body
            .table(table)
            .map(|table| (table.row_count(), table.column_count()))
            .ok_or_else(|| DocxError::AddressNotFound(format!("no table at index {table}")))?;
        let table_ref = body
            .table_mut(table)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no table at index {table}")))?;
        let cell =
            table_ref
                .cell_mut(interner, row, column)
                .ok_or(DocxError::TableCellOutOfRange {
                    row,
                    column,
                    rows,
                    columns,
                })?;
        cell.set_column_span(interner, span);
        main.write_back(root, interner);
        Ok(())
    }

    /// Sets (or, given `None`, removes) the `w:vMerge` of the cell at `(row, column)` of table
    /// `table`. `Some(MergedCellType::Restart)` starts (or restarts) a vertical merge;
    /// `Some(MergedCellType::Continue)` marks the cell as continuing the region above it (the caller
    /// is responsible for there being a `restart` reachable above, per ECMA-376 Part 1 §17.4.84 —
    /// see `tables.rs`'s own doc comment); `None` removes the cell from any vertical merge.
    ///
    /// # Errors
    /// As [`cell_span`](Self::cell_span).
    pub fn set_cell_vertical_merge(
        &mut self,
        table: usize,
        row: usize,
        column: usize,
        kind: Option<tables::MergedCellType>,
    ) -> Result<(), DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let (rows, columns) = body
            .table(table)
            .map(|table| (table.row_count(), table.column_count()))
            .ok_or_else(|| DocxError::AddressNotFound(format!("no table at index {table}")))?;
        let table_ref = body
            .table_mut(table)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no table at index {table}")))?;
        let cell =
            table_ref
                .cell_mut(interner, row, column)
                .ok_or(DocxError::TableCellOutOfRange {
                    row,
                    column,
                    rows,
                    columns,
                })?;
        cell.set_vertical_merge(interner, kind);
        main.write_back(root, interner);
        Ok(())
    }

    /// Reaches the cell at `(row, column)` of table `table` and hands it, with the part's interner,
    /// to `edit` — the general escape hatch behind every narrower cell-editing method above, and how
    /// a table is authored **into a table cell**: `edit`'s own body calls
    /// [`tables::Cell::append_table`] with a fresh [`tables::Table::new`], exactly as
    /// [`append_table`](Self::append_table) does at the document's own top level. Only
    /// `word/document.xml` is dirtied.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::TableCellOutOfRange`] if `(row, column)` is out of range.
    pub fn edit_cell<R>(
        &mut self,
        table: usize,
        row: usize,
        column: usize,
        edit: impl FnOnce(&mut tables::Cell, &mut mjx_ooxml_core::Interner) -> R,
    ) -> Result<R, DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let (rows, columns) = body
            .table(table)
            .map(|table| (table.row_count(), table.column_count()))
            .ok_or_else(|| DocxError::AddressNotFound(format!("no table at index {table}")))?;
        let table_ref = body
            .table_mut(table)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no table at index {table}")))?;
        let cell =
            table_ref
                .cell_mut(interner, row, column)
                .ok_or(DocxError::TableCellOutOfRange {
                    row,
                    column,
                    rows,
                    columns,
                })?;
        let result = edit(cell, interner);
        main.write_back(root, interner);
        Ok(result)
    }

    /// Reaches the table at `table` itself and hands it, with the part's interner, to `edit` — the
    /// general escape hatch for setting the table's own `w:tblPr` (style reference, `w:tblLook`,
    /// band sizes, …) or a row's `w:tblPrEx`/`w:trPr`, none of which any narrower method above
    /// exposes. Only `word/document.xml` is dirtied.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if `table` does not address a table.
    pub fn edit_table<R>(
        &mut self,
        table: usize,
        edit: impl FnOnce(&mut Table, &mut mjx_ooxml_core::Interner) -> R,
    ) -> Result<R, DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let table_ref = body
            .table_mut(table)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no table at index {table}")))?;
        let result = edit(table_ref, interner);
        main.write_back(root, interner);
        Ok(result)
    }

    /// Appends a new `rows` x `columns` table as the body's new last top-level table (before
    /// `w:sectPr`, when the body has one), and returns its new index. Every cell starts with one
    /// empty paragraph.
    ///
    /// # Errors
    /// Returns [`DocxError::InvalidTableSize`] if either dimension is zero, or
    /// [`DocxError::NoBody`] if the document declares no body.
    pub fn append_table(&mut self, rows: usize, columns: usize) -> Result<usize, DocxError> {
        if rows == 0 || columns == 0 {
            return Err(DocxError::InvalidTableSize { rows, columns });
        }
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let table = Table::new(interner, rows, columns);
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let index = body.append_table(table);
        main.write_back(root, interner);
        Ok(index)
    }

    /// Removes the top-level table at `index`.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if `index` does not address a table.
    pub fn remove_table(&mut self, index: usize) -> Result<(), DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        if body.remove_table(index).is_none() {
            return Err(DocxError::AddressNotFound(format!(
                "no table at index {index}"
            )));
        }
        main.write_back(root, interner);
        Ok(())
    }

    /// Inserts a row into table `table` so it becomes row `at`; `at` equal to the current row count
    /// appends. A vertical merge the new row falls inside grows to include it — see
    /// [`tables::Table::insert_row`].
    ///
    /// # Errors
    /// Returns [`DocxError::TableCellOutOfRange`] if `at` is past the end, plus the errors of
    /// [`table_dimensions`](Self::table_dimensions).
    pub fn insert_row(&mut self, table: usize, at: usize) -> Result<(), DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let table_ref = body
            .table_mut(table)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no table at index {table}")))?;
        let (rows, columns) = (table_ref.row_count(), table_ref.column_count());
        if at > rows {
            return Err(DocxError::TableCellOutOfRange {
                row: at,
                column: 0,
                rows,
                columns,
            });
        }
        table_ref.insert_row(interner, at, |interner| {
            Cell::new(interner).to_xml(interner)
        })?;
        main.write_back(root, interner);
        Ok(())
    }

    /// Removes row `at` from table `table`. A vertical merge the row lies inside shrinks; a merge
    /// anchored in the row promotes the cell below it — see [`tables::Table::remove_row`].
    ///
    /// # Errors
    /// Returns [`DocxError::InvalidTableSize`] if `at` is the table's only row,
    /// [`DocxError::TableCellOutOfRange`] if `at` is out of range, plus the errors of
    /// [`table_dimensions`](Self::table_dimensions).
    pub fn remove_row(&mut self, table: usize, at: usize) -> Result<(), DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let table_ref = body
            .table_mut(table)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no table at index {table}")))?;
        let (rows, columns) = (table_ref.row_count(), table_ref.column_count());
        if at >= rows {
            return Err(DocxError::TableCellOutOfRange {
                row: at,
                column: 0,
                rows,
                columns,
            });
        }
        if rows == 1 {
            return Err(DocxError::InvalidTableSize { rows: 0, columns });
        }
        table_ref.remove_row(interner, at);
        main.write_back(root, interner);
        Ok(())
    }

    /// Inserts a column into table `table` so it becomes column `at`; `at` equal to the current
    /// column count appends. A horizontal merge the new column falls inside grows to include it —
    /// see [`tables::Table::insert_column`].
    ///
    /// # Errors
    /// Returns [`DocxError::TableCellOutOfRange`] if `at` is past the end, plus the errors of
    /// [`table_dimensions`](Self::table_dimensions).
    pub fn insert_column(&mut self, table: usize, at: usize) -> Result<(), DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let table_ref = body
            .table_mut(table)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no table at index {table}")))?;
        let (rows, columns) = (table_ref.row_count(), table_ref.column_count());
        if at > columns {
            return Err(DocxError::TableCellOutOfRange {
                row: 0,
                column: at,
                rows,
                columns,
            });
        }
        table_ref.insert_column(interner, at, |interner| {
            Cell::new(interner).to_xml(interner)
        })?;
        main.write_back(root, interner);
        Ok(())
    }

    /// Removes column `at` from table `table`: its `w:gridCol` and one cell from every row. A
    /// horizontal merge the column lies inside shrinks; a merge anchored in the column promotes the
    /// cell to its right — see [`tables::Table::remove_column`].
    ///
    /// # Errors
    /// Returns [`DocxError::InvalidTableSize`] if `at` is the table's only column,
    /// [`DocxError::TableCellOutOfRange`] if `at` is out of range, plus the errors of
    /// [`table_dimensions`](Self::table_dimensions).
    pub fn remove_column(&mut self, table: usize, at: usize) -> Result<(), DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let table_ref = body
            .table_mut(table)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no table at index {table}")))?;
        let (rows, columns) = (table_ref.row_count(), table_ref.column_count());
        if at >= columns {
            return Err(DocxError::TableCellOutOfRange {
                row: 0,
                column: at,
                rows,
                columns,
            });
        }
        if columns == 1 {
            return Err(DocxError::InvalidTableSize { rows, columns: 0 });
        }
        table_ref.remove_column(interner, at);
        main.write_back(root, interner);
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Fields (MJXOFF-121) — see `fields.rs`'s own doc comment for the read model and the
    // marker-pairing/nesting design. `FieldPath` addresses a field the way `BlockPath`/`RunPath`
    // address a paragraph/run: a top-level index, then indices descending through
    // `Field::nested_fields`.
    // -----------------------------------------------------------------------------------------

    /// Every field the paragraph at `paragraph` holds, at its own top level and (recursively)
    /// nested inside one of those, in document order.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body,
    /// [`DocxError::AddressNotFound`] if `paragraph` does not address a paragraph, or
    /// [`DocxError::UnbalancedField`] if a `w:fldChar` marker sequence in that paragraph's own
    /// content does not balance.
    pub fn fields(&mut self, paragraph: impl Into<BlockPath>) -> Result<Vec<Field>, DocxError> {
        let paragraph_path = paragraph.into();
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let paragraph_ref = body.paragraph(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        paragraph_ref.fields(&doc.interner)
    }

    /// Sets the field at `field` (within the paragraph at `paragraph`)'s own instruction, leaving
    /// its cached result — and every other field, and every other part — byte-identical. See
    /// `fields.rs`'s own doc comment for exactly what this collapses and what it never touches.
    ///
    /// # Errors
    /// [`DocxError::NoBody`]/[`DocxError::AddressNotFound`] as [`Document::fields`];
    /// [`DocxError::UnbalancedField`] if the paragraph's own markers do not balance;
    /// [`DocxError::FieldNotFound`] if `field` does not address a field within that paragraph's
    /// fields; [`DocxError::FieldHasNestedContent`] if the instruction zone itself holds a nested
    /// field (collapsing it would destroy that field's own markup).
    pub fn set_field_instruction(
        &mut self,
        paragraph: impl Into<BlockPath>,
        field: impl Into<FieldPath>,
        text: &str,
    ) -> Result<(), DocxError> {
        let paragraph_path = paragraph.into();
        let field_path = field.into();
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        fields::set_instruction(
            paragraph.content_mut(),
            field_path.indices(),
            text,
            interner,
        )?;
        main.write_back(root, interner);
        Ok(())
    }

    /// Sets the field at `field`'s own cached result, leaving its instruction — and every other
    /// field, and every other part — byte-identical.
    ///
    /// # Errors
    /// As [`Document::set_field_instruction`], plus [`DocxError::FieldHasNoCachedResult`] if the
    /// field's complex `w:fldChar` form carries no `separate` marker (a field with no `separate` has
    /// no cached-result zone to edit — see [`Field::cached_result`]'s own doc comment).
    pub fn set_field_cached_result_text(
        &mut self,
        paragraph: impl Into<BlockPath>,
        field: impl Into<FieldPath>,
        text: &str,
    ) -> Result<(), DocxError> {
        let paragraph_path = paragraph.into();
        let field_path = field.into();
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        fields::set_cached_result_text(
            paragraph.content_mut(),
            field_path.indices(),
            text,
            interner,
        )?;
        main.write_back(root, interner);
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Hyperlinks (MJXOFF-121) — `w:hyperlink` wraps the runs it links; see `hyperlinks.rs`'s own
    // doc comment. Scoped to the main document body's own top-level run-or-hyperlink slots — a
    // caller reaching a header/footer through `edit_header_footer` still reads/writes `Hyperlink`'s
    // own typed attributes directly (MJXOFF-92's addressing already recurses into one); only this
    // convenience pair, which also manages the relationship, is body-only.
    // -----------------------------------------------------------------------------------------

    /// The click target of the hyperlink at top-level run-or-hyperlink slot `at` within the
    /// paragraph at `paragraph`, resolved against the main document part's own relationships —
    /// `r:id` wins over `anchor` when both are present (§17.16.22). `None` if `at` does not land on
    /// a hyperlink, or the hyperlink resolves to neither a relationship nor an anchor.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if `paragraph` does not address a paragraph.
    pub fn hyperlink_target(
        &mut self,
        paragraph: impl Into<BlockPath>,
        at: impl Into<RunPath>,
    ) -> Result<Option<HyperlinkTarget>, DocxError> {
        let paragraph_path = paragraph.into();
        let at = at.into();
        let (rel_id, anchor) = {
            let doc = self.package.part_tree(&self.document_part)?;
            let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
            let body = main.body().ok_or(DocxError::NoBody)?;
            let paragraph_ref = body.paragraph(&paragraph_path).ok_or_else(|| {
                DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
            })?;
            let Some(hyperlink) = paragraph_ref.hyperlink_at(&at) else {
                return Ok(None);
            };
            let rel_id = hyperlink
                .relationship_id(&doc.interner)
                .ok()
                .flatten()
                .map(|cow| cow.into_owned());
            let anchor = hyperlink
                .anchor(&doc.interner)
                .ok()
                .flatten()
                .map(|cow| cow.into_owned());
            (rel_id, anchor)
        };
        let rels = self.package.relationships_for(Some(&self.document_part));
        Ok(hyperlinks::resolve_target(
            rel_id.as_deref(),
            anchor.as_deref(),
            rels,
        ))
    }

    /// Inserts a new hyperlink wrapping one run of `text` at top-level run-or-hyperlink slot `at`
    /// within the paragraph at `paragraph`, shifting every slot at or after that position one place
    /// later — adding the external relationship [`HyperlinkTarget::Url`] needs first (an
    /// [`HyperlinkTarget::Anchor`] names no relationship: it targets a bookmark in this same
    /// document). `at` must address an existing slot or the one past the last (`0..=run_count()`),
    /// checked *before* any relationship is added, so a bad call leaves the package untouched.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body,
    /// [`DocxError::AddressNotFound`] if either address is out of range, or another
    /// [`DocxError`] if adding the relationship fails.
    pub fn insert_hyperlink(
        &mut self,
        paragraph: impl Into<BlockPath>,
        at: impl Into<RunPath>,
        text: &str,
        target: &HyperlinkTarget,
    ) -> Result<(), DocxError> {
        let paragraph_path = paragraph.into();
        let at = at.into();
        let [index] = at.indices() else {
            return Err(DocxError::AddressNotFound(format!(
                "hyperlink address {at} is not top-level"
            )));
        };
        let paragraph_ref = self.resolve_paragraph(paragraph_path.clone())?;
        if *index > paragraph_ref.run_count() {
            return Err(DocxError::AddressNotFound(format!("no run slot at {at}")));
        }

        let rel_id = if let HyperlinkTarget::Url(url) = target {
            let rid = self.next_rid_for(&self.document_part.clone());
            self.package.add_relationship(
                Some(&self.document_part),
                mjx_opc::Relationship {
                    id: rid.clone(),
                    rel_type: crate::constants::REL_HYPERLINK.to_owned(),
                    target: url.clone(),
                    mode: mjx_opc::TargetMode::External,
                },
            )?;
            Some(rid)
        } else {
            None
        };

        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let mut hyperlink = Hyperlink::new(interner);
        if let Some(rid) = &rel_id {
            hyperlink.set_relationship_id(interner, Some(rid.as_str()));
        }
        if let HyperlinkTarget::Anchor(name) = target {
            hyperlink.set_anchor(interner, Some(name.as_str()));
        }
        hyperlink.append_run(Run::with_text(interner, text));
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph_mut = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        let inserted = paragraph_mut.insert_hyperlink(&at, hyperlink);
        debug_assert!(inserted, "the slot was validated as in-range above");
        main.write_back(root, interner);
        Ok(())
    }

    /// Removes the hyperlink at top-level run-or-hyperlink slot `at` within the paragraph at
    /// `paragraph` — together with every run it wraps (a caller who wants to keep the wrapped text
    /// un-linked reads it first, e.g. with [`Document::paragraph_text`], and reinserts plain runs
    /// with [`Document::insert_run`]) — and the relationship it named, unless some other
    /// `w:hyperlink` anywhere in the main document part still names the same relationship.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body,
    /// [`DocxError::AddressNotFound`] if either address does not resolve to a hyperlink, or another
    /// [`DocxError`] if the package edit fails.
    pub fn remove_hyperlink(
        &mut self,
        paragraph: impl Into<BlockPath>,
        at: impl Into<RunPath>,
    ) -> Result<(), DocxError> {
        let paragraph_path = paragraph.into();
        let at = at.into();
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph_mut = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        let removed = paragraph_mut
            .remove_hyperlink(&at)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no hyperlink at {at}")))?;
        let rel_id = removed
            .relationship_id(interner)
            .ok()
            .flatten()
            .map(|id| id.into_owned());
        main.write_back(root, interner);

        let Some(rel_id) = rel_id else {
            return Ok(());
        };
        let still_used = {
            let doc = self.package.part_tree(&self.document_part)?;
            let mut ids = Vec::new();
            hyperlinks::collect_hyperlink_rel_ids(&doc.root, &doc.interner, &mut ids);
            ids.contains(&rel_id.as_str())
        };
        if !still_used {
            self.package
                .remove_relationship(Some(&self.document_part), &rel_id)?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Form fields (MJXOFF-121) — `w:ffData`, carried on the `begin` `w:fldChar` of a complex field.
    // Addressed by the run holding that `w:fldChar` (the same `RunPath` `Document::run_text` uses),
    // never by `FieldPath`: a form field is a property of one specific marker run, not a field in
    // its own right the way a `TOC`/`PAGEREF` is.
    // -----------------------------------------------------------------------------------------

    /// Reads the `w:ffData` the `w:fldChar` at run `field_run` (within the paragraph at
    /// `paragraph`) carries, handing it — `None` if that run holds no `w:fldChar`, or its
    /// `w:fldChar` carries no `w:ffData` (every marker except a form field's own `begin`) — to
    /// `read` together with the part's own [`mjx_ooxml_core::Interner`], mirroring
    /// [`Document::style_sheet`]'s own shape exactly: every [`FormFieldData`] accessor needs the
    /// same interner the value was parsed with, so the two are never handed back separately.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if either address does not resolve to a run.
    pub fn form_field<R>(
        &mut self,
        paragraph: impl Into<BlockPath>,
        field_run: impl Into<RunPath>,
        read: impl FnOnce(Option<&FormFieldData>, &mjx_ooxml_core::Interner) -> R,
    ) -> Result<R, DocxError> {
        let paragraph_path = paragraph.into();
        let field_run = field_run.into();
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let paragraph_ref = body.paragraph(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        let run = paragraph_ref
            .run(&field_run)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no run at {field_run}")))?;
        let data = run.content().iter().find_map(|item| match item {
            RunInnerContent::ComplexFieldCharacter(field_char) => field_char.form_field_data(),
            _ => None,
        });
        Ok(read(data, &doc.interner))
    }

    /// Reaches the `w:ffData` of the `w:fldChar` at run `field_run` (within the paragraph at
    /// `paragraph`) — creating an empty one first if that marker carries none yet — and hands it,
    /// with the part's interner, to `edit`. The general escape hatch for authoring or changing a
    /// form field's own name, help/status text, macros, enabled/calc-on-exit flags and
    /// checkbox/drop-down/text-input definition; [`Document::insert_form_field`] is the shortcut for
    /// building a whole new form field from nothing.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body,
    /// [`DocxError::AddressNotFound`] if either address does not resolve to a run, or
    /// [`DocxError::AddressNotFound`] if that run holds no `w:fldChar` at all.
    pub fn edit_form_field<R>(
        &mut self,
        paragraph: impl Into<BlockPath>,
        field_run: impl Into<RunPath>,
        edit: impl FnOnce(&mut FormFieldData, &mut mjx_ooxml_core::Interner) -> R,
    ) -> Result<R, DocxError> {
        let paragraph_path = paragraph.into();
        let field_run = field_run.into();
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        let run = paragraph
            .run_mut(&field_run)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no run at {field_run}")))?;
        let field_char = run
            .content_mut()
            .iter_mut()
            .find_map(|item| match item {
                RunInnerContent::ComplexFieldCharacter(field_char) => Some(field_char),
                _ => None,
            })
            .ok_or_else(|| {
                DocxError::AddressNotFound(format!("run at {field_run} carries no w:fldChar"))
            })?;
        if field_char.form_field_data().is_none() {
            field_char.set_form_field_data(Some(FormFieldData::new(interner)));
        }
        let data = match field_char.form_field_data_mut() {
            Some(data) => data,
            None => unreachable!("just inserted above"),
        };
        let result = edit(data, interner);
        main.write_back(root, interner);
        Ok(result)
    }

    /// Inserts a whole new form field — a fresh `begin`/`separate`/`end` `w:fldChar` triple, with
    /// `instruction` as the `begin` run's own `w:instrText` and `display_text` as the cached-result
    /// run between `separate` and `end` — as five consecutive runs starting at top-level
    /// run-or-hyperlink slot `at` within the paragraph at `paragraph`. The `begin` marker's own
    /// `w:ffData` starts empty; populate it afterward with [`Document::edit_form_field`] on this
    /// same `(paragraph, at)` address — every `FormFieldData`/`FormFieldCheckBox`/… value must be
    /// built with *this* document's own [`mjx_ooxml_core::Interner`], which only a method that hands
    /// the caller that interner (as `edit_form_field`'s closure does) can guarantee; a `data`
    /// parameter built by the caller from a throwaway interner of its own would embed symbols that
    /// resolve to the wrong strings once written back with this document's interner instead.
    ///
    /// Real Word writes ` FORMCHECKBOX `, ` FORMDROPDOWN ` or ` FORMTEXT ` as `instruction` for the
    /// three form-field kinds respectively; this crate does not parse or supply field-code text (see
    /// `fields.rs`'s own doc comment), so the caller states it.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or
    /// [`DocxError::AddressNotFound`] if either address is out of range.
    pub fn insert_form_field(
        &mut self,
        paragraph: impl Into<BlockPath>,
        at: impl Into<RunPath>,
        instruction: &str,
        display_text: &str,
    ) -> Result<(), DocxError> {
        let paragraph_path = paragraph.into();
        let at = at.into();
        let [index] = at.indices() else {
            return Err(DocxError::AddressNotFound(format!(
                "form field address {at} is not top-level"
            )));
        };
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        let count = paragraph.run_count();
        if *index > count {
            return Err(DocxError::AddressNotFound(format!("no run slot at {at}")));
        }

        let begin = fields::marker_run(
            interner,
            mjx_ooxml_types::wordprocessingml::FieldCharacterType::Begin,
        );
        let five = vec![
            begin,
            Run::with_field_code(interner, instruction),
            fields::marker_run(
                interner,
                mjx_ooxml_types::wordprocessingml::FieldCharacterType::Separate,
            ),
            Run::with_text(interner, display_text),
            fields::marker_run(
                interner,
                mjx_ooxml_types::wordprocessingml::FieldCharacterType::End,
            ),
        ];

        let content = paragraph.content_mut();
        let at_slot = content
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                matches!(
                    item,
                    ParagraphContent::Run(_) | ParagraphContent::Hyperlink(_)
                )
            })
            .nth(*index)
            .map(|(slot, _)| slot)
            .unwrap_or(content.len());
        for (offset, run) in five.into_iter().enumerate() {
            content.insert(at_slot + offset, ParagraphContent::Run(run));
        }
        main.write_back(root, interner);
        Ok(())
    }

    /// Reads the top-level table at `index` and hands it, with the part's interner, to `read`.
    /// Does not dirty the part.
    fn with_table<R>(
        &mut self,
        index: usize,
        read: impl FnOnce(&Table, &mjx_ooxml_core::Interner) -> Result<R, DocxError>,
    ) -> Result<R, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let table = body
            .table(index)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no table at index {index}")))?;
        read(table, &doc.interner)
    }

    /// Reads the paragraph at `path`, without dirtying the part.
    fn resolve_paragraph(&mut self, path: BlockPath) -> Result<Paragraph, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        body.paragraph(&path)
            .cloned()
            .ok_or_else(|| DocxError::AddressNotFound(format!("no paragraph at {path}")))
    }

    // ---------------------------------------------------------------------------------------------
    // Comments (`word/comments.xml`, MJXOFF-124)
    // ---------------------------------------------------------------------------------------------

    /// Reads this document's `word/comments.xml`, handing `read` the parsed [`Comments`] together
    /// with the [`mjx_ooxml_core::Interner`] it was parsed with — mirrors [`Document::style_sheet`]'s
    /// own shape exactly. `None` — `read` is never called — if this document relates to no
    /// `word/comments.xml` at all.
    ///
    /// # Errors
    /// Returns [`DocxError`] if `word/comments.xml` is related but cannot be read, is not
    /// well-formed, or its root is not `w:comments`.
    pub fn comments<R>(
        &mut self,
        read: impl FnOnce(&Comments, &mjx_ooxml_core::Interner) -> R,
    ) -> Result<Option<R>, DocxError> {
        let Some(comments_part) = self.parts.comments.clone() else {
            return Ok(None);
        };
        let doc = self.package.part_tree(&comments_part)?;
        let comments = Comments::from_xml(&doc.root, &doc.interner)?;
        Ok(Some(read(&comments, &doc.interner)))
    }

    /// Edits this document's comments, creating `word/comments.xml` — with its content-type
    /// registration and its `comments` relationship from the main document part — first if it does
    /// not already have one.
    ///
    /// # Errors
    /// Returns [`DocxError`] if `word/comments.xml` is related but cannot be read, or if creating a
    /// missing one fails.
    pub fn edit_comments<R>(
        &mut self,
        edit: impl FnOnce(&mut Comments, &mut mjx_ooxml_core::Interner) -> R,
    ) -> Result<R, DocxError> {
        let comments_part = match &self.parts.comments {
            Some(part) => part.clone(),
            None => self.create_comments_part()?,
        };
        let doc = self.package.part_tree_mut(&comments_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut comments = if root.name.local == interner.intern("comments") {
            Comments::from_xml(root, interner)?
        } else {
            return Err(DocxError::MalformedDocument(
                "word/comments.xml root is not w:comments",
            ));
        };
        let result = edit(&mut comments, interner);
        comments.write_back(root, interner);
        Ok(result)
    }

    /// Creates an empty `word/comments.xml`, registers its content type, and relates it from the main
    /// document part.
    fn create_comments_part(&mut self) -> Result<mjx_opc::PartName, DocxError> {
        let comments_part = self.document_part.resolve("comments.xml").map_err(|_| {
            DocxError::TargetResolution {
                target: "comments.xml".to_owned(),
            }
        })?;
        const WML_NAMESPACE: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
        let bytes = format!(
            concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                "\n",
                r#"<w:comments xmlns:w="{ns}"/>"#,
            ),
            ns = WML_NAMESPACE,
        )
        .into_bytes();
        self.package.insert_part(
            &comments_part,
            crate::constants::CONTENT_TYPE_COMMENTS,
            bytes,
        )?;
        let rid = self.next_rid_for(&self.document_part.clone());
        self.package.add_relationship(
            Some(&self.document_part),
            mjx_opc::Relationship {
                id: rid,
                rel_type: crate::constants::REL_COMMENTS.to_owned(),
                target: "comments.xml".to_owned(),
                mode: mjx_opc::TargetMode::Internal,
            },
        )?;
        self.parts.comments = Some(comments_part.clone());
        Ok(comments_part)
    }

    /// Adds a new comment on the **whole** paragraph at `paragraph`: wraps it in
    /// `w:commentRangeStart`/`w:commentRangeEnd`, appends a run holding `w:commentReference` right
    /// after the range end, and appends the [`Comment`] itself (`author`, optional `initials`, `text`
    /// as its own paragraph) to `word/comments.xml` — creating that part, its content type and its
    /// relationship first if the document has none. Returns the comment's own freshly assigned id
    /// (one past the highest id already in `word/comments.xml`, reserved or not).
    ///
    /// This wraps the **entire** paragraph, never an arbitrary run range within it — the
    /// range-resolution mechanism this crate *reads* (`ranges.rs`) resolves an arbitrary span exactly
    /// as this ticket's own trap requires, but this *writer* only ever authors the one shape the
    /// ticket's own "Done when" needs: "a comment on this paragraph." A caller wanting a narrower
    /// range builds `w:commentRangeStart`/`w:commentRangeEnd` at a specific slot directly.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body,
    /// [`DocxError::AddressNotFound`] if `paragraph` does not address a paragraph, or another
    /// [`DocxError`] if the package edit fails.
    pub fn add_comment(
        &mut self,
        paragraph: impl Into<BlockPath>,
        author: &str,
        initials: Option<&str>,
        text: &str,
    ) -> Result<i64, DocxError> {
        let paragraph_path = paragraph.into();
        {
            let doc = self.package.part_tree(&self.document_part)?;
            let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
            let body = main.body().ok_or(DocxError::NoBody)?;
            body.paragraph(&paragraph_path).ok_or_else(|| {
                DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
            })?;
        }

        let comments_part = match &self.parts.comments {
            Some(part) => part.clone(),
            None => self.create_comments_part()?,
        };
        let id = {
            let doc = self.package.part_tree_mut(&comments_part)?;
            let RawDocument { interner, root, .. } = doc;
            let mut comments = if root.name.local == interner.intern("comments") {
                Comments::from_xml(root, interner)?
            } else {
                return Err(DocxError::MalformedDocument(
                    "word/comments.xml root is not w:comments",
                ));
            };
            let id = comments.next_id(interner);
            let mut comment = Comment::new(interner, id, author);
            if let Some(initials) = initials {
                comment.set_raw_initials(interner, Some(initials));
            }
            if let Some(p) = comment.paragraph_mut(0) {
                p.append_run(Run::with_text(interner, text));
            }
            comments.push(comment);
            comments.write_back(root, interner);
            id
        };

        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph_mut = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        let content = paragraph_mut.content_mut();
        content.insert(
            0,
            ParagraphContent::CommentRangeStart(ranges::MarkupRange::new(
                interner,
                "commentRangeStart",
                id,
            )),
        );
        content.push(ParagraphContent::CommentRangeEnd(ranges::MarkupRange::new(
            interner,
            "commentRangeEnd",
            id,
        )));
        let reference = ranges::Markup::new(interner, "commentReference", id);
        content.push(ParagraphContent::Run(Run::with_inner_content(
            interner,
            RunInnerContent::CommentReference(reference),
        )));
        main.write_back(root, interner);
        Ok(id)
    }

    /// Removes the comment with `id`: every `w:commentRangeStart`/`w:commentRangeEnd`/
    /// `w:commentReference` naming it anywhere in the body (recursing into every table cell — see
    /// `ranges::remove_matching`'s own doc comment for the one documented gap, markers nested inside
    /// a `w:hyperlink`), and the [`Comment`] entry itself from `word/comments.xml` — deleting that
    /// part and its relationship when it was the last comment, so [`mjx_opc::Package::validate`] never
    /// reports an orphan.
    ///
    /// Not an error if `id` names no comment at all — a no-op, matching
    /// [`Document::remove_paragraph`]'s own "already gone" leniency.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or another [`DocxError`] if the
    /// package edit fails.
    pub fn remove_comment(&mut self, id: i64) -> Result<(), DocxError> {
        {
            let doc = self.package.part_tree_mut(&self.document_part)?;
            let RawDocument { interner, root, .. } = doc;
            let mut main = MainDocument::from_xml(root, interner)?;
            let body = main.body_mut().ok_or(DocxError::NoBody)?;
            ranges::remove_matching(
                body.content_mut(),
                &|item: &ParagraphContent| match item {
                    ParagraphContent::CommentRangeStart(marker) => marker.id(interner) == Ok(id),
                    ParagraphContent::CommentRangeEnd(marker) => marker.id(interner) == Ok(id),
                    _ => false,
                },
                &|item: &RunInnerContent| match item {
                    RunInnerContent::CommentReference(marker) => marker.id(interner) == Ok(id),
                    _ => false,
                },
            );
            main.write_back(root, interner);
        }

        let Some(comments_part) = self.parts.comments.clone() else {
            return Ok(());
        };
        let now_empty = {
            let doc = self.package.part_tree_mut(&comments_part)?;
            let RawDocument { interner, root, .. } = doc;
            let mut comments = if root.name.local == interner.intern("comments") {
                Comments::from_xml(root, interner)?
            } else {
                return Err(DocxError::MalformedDocument(
                    "word/comments.xml root is not w:comments",
                ));
            };
            comments.remove(interner, id);
            let empty = comments.comments().next().is_none();
            comments.write_back(root, interner);
            empty
        };
        if now_empty {
            let rel_id = self
                .package
                .relationships_for(Some(&self.document_part))
                .and_then(|rels| rels.by_type(crate::constants::REL_COMMENTS).next())
                .map(|rel| rel.id.clone());
            if let Some(rel_id) = rel_id {
                self.package
                    .remove_relationship(Some(&self.document_part), &rel_id)?;
            }
            self.package.remove_unreferenced_parts()?;
            self.parts.comments = None;
        }
        Ok(())
    }

    /// This comment's own resolved range: whether both `w:commentRangeStart`/`w:commentRangeEnd`
    /// naming `id` were found, and — when both were — the text between them and how many paragraphs
    /// it spans. `None` if neither marker names `id` at all.
    ///
    /// This is [`ranges::RangeIndex`]/[`ranges::covered_text`] applied to comment ranges specifically
    /// — the same mechanism [`Document::resolve_bookmark`] applies to bookmarks, and the one
    /// MJXOFF-126 reuses for its own move-range markers (see `ranges.rs`'s own doc comment).
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or another [`DocxError`] if the
    /// main document part cannot be read.
    pub fn comment_range(&mut self, id: i64) -> Result<Option<RangeResolution>, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let index = RangeIndex::build(
            body.content(),
            &doc.interner,
            ranges::classify_comment_range,
        );
        Ok(index.get(id))
    }

    /// [`Document::comment_range`]'s own resolved text, or `None` if `id` names no resolved (both
    /// markers found) comment range.
    ///
    /// # Errors
    /// See [`Document::comment_range`].
    pub fn comment_range_text(&mut self, id: i64) -> Result<Option<String>, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let index = RangeIndex::build(
            body.content(),
            &doc.interner,
            ranges::classify_comment_range,
        );
        Ok(match index.get(id) {
            Some(RangeResolution::Resolved { start, end }) => {
                Some(covered_text(body.content(), start, end))
            }
            _ => None,
        })
    }

    /// This move range's own resolved extent: whether both `w:moveFromRangeStart`/
    /// `w:moveFromRangeEnd` naming `id` were found. `None` if neither marker names `id` at all.
    /// [`Document::move_to_range`] is the `w:moveToRangeStart`/`End` counterpart — the two are
    /// separate id spaces (see `crate::document::revisions`'s own doc comment on
    /// `revisions::classify_move_from_range`).
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or another [`DocxError`] if
    /// the main document part cannot be read.
    pub fn move_from_range(&mut self, id: i64) -> Result<Option<RangeResolution>, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let index = RangeIndex::build(
            body.content(),
            &doc.interner,
            revisions::classify_move_from_range,
        );
        Ok(index.get(id))
    }

    /// [`Document::move_from_range`]'s own `w:moveToRangeStart`/`End` counterpart.
    ///
    /// # Errors
    /// See [`Document::move_from_range`].
    pub fn move_to_range(&mut self, id: i64) -> Result<Option<RangeResolution>, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let index = RangeIndex::build(
            body.content(),
            &doc.interner,
            revisions::classify_move_to_range,
        );
        Ok(index.get(id))
    }

    /// This `w:customXmlInsRangeStart`/`w:customXmlInsRangeEnd` range's own resolved extent, keyed
    /// by `id`. `None` if neither marker names `id` at all. The four `customXml*Range` kinds
    /// (`Ins`/`Del`/`MoveFrom`/`MoveTo`, this method and its three siblings below) are four separate
    /// id spaces, exactly like `move_from_range`/`move_to_range` above.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or another [`DocxError`] if
    /// the main document part cannot be read.
    pub fn custom_xml_ins_range(&mut self, id: i64) -> Result<Option<RangeResolution>, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let index = RangeIndex::build(
            body.content(),
            &doc.interner,
            revisions::classify_custom_xml_ins_range,
        );
        Ok(index.get(id))
    }

    /// [`Document::custom_xml_ins_range`]'s own `customXmlDelRange*` counterpart.
    ///
    /// # Errors
    /// See [`Document::custom_xml_ins_range`].
    pub fn custom_xml_del_range(&mut self, id: i64) -> Result<Option<RangeResolution>, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let index = RangeIndex::build(
            body.content(),
            &doc.interner,
            revisions::classify_custom_xml_del_range,
        );
        Ok(index.get(id))
    }

    /// [`Document::custom_xml_ins_range`]'s own `customXmlMoveFromRange*` counterpart.
    ///
    /// # Errors
    /// See [`Document::custom_xml_ins_range`].
    pub fn custom_xml_move_from_range(
        &mut self,
        id: i64,
    ) -> Result<Option<RangeResolution>, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let index = RangeIndex::build(
            body.content(),
            &doc.interner,
            revisions::classify_custom_xml_move_from_range,
        );
        Ok(index.get(id))
    }

    /// [`Document::custom_xml_ins_range`]'s own `customXmlMoveToRange*` counterpart.
    ///
    /// # Errors
    /// See [`Document::custom_xml_ins_range`].
    pub fn custom_xml_move_to_range(
        &mut self,
        id: i64,
    ) -> Result<Option<RangeResolution>, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let index = RangeIndex::build(
            body.content(),
            &doc.interner,
            revisions::classify_custom_xml_move_to_range,
        );
        Ok(index.get(id))
    }

    /// Every tracked change in the document's own body (MJXOFF-126): every `w:ins`/`w:del`/
    /// `w:moveFrom`/`w:moveTo` (recursing into nested revisions — an insertion nested inside a
    /// deletion reports both), every `*Change` property wrapper, every bare tracked marker
    /// (`w:cellIns`/`w:cellDel`/`w:numPr/w:ins`/the paragraph mark's own `w:ins`/`w:del`/
    /// `w:moveFrom`/`w:moveTo`) and every tracked cell merge, each with its own author, date and id
    /// exactly as the file states them. Headers, footers, comments, footnotes and endnotes are
    /// separate parts this method does not walk — see this crate's own per-part accessors
    /// (`headers`/`footers`/`comments`/`footnotes`/`endnotes`) for those.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or another [`DocxError`] if
    /// the main document part cannot be read.
    pub fn revisions(&mut self) -> Result<Vec<RevisionInfo>, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let mut out = Vec::new();
        revisions::collect_revisions(body.content(), &doc.interner, &mut out);
        Ok(out)
    }

    /// The document body's own text with every tracked insertion kept and every tracked deletion
    /// dropped (`w:moveFrom`/`w:moveTo` content excluded from both this and
    /// [`Document::text_with_revisions_rejected`] — see `crate::document::revisions`'s own doc
    /// comment on why an in-place move resolution is not part of this read-only computation).
    /// Paragraphs are joined with `\n`, matching [`Paragraph::text`]'s own convention.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or another [`DocxError`] if
    /// the main document part cannot be read.
    pub fn text_with_revisions_accepted(&mut self) -> Result<String, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        Ok(revisions::text_with_accepted(body.content()))
    }

    /// [`Document::text_with_revisions_accepted`]'s own rejected-text counterpart: tracked deletions
    /// kept, tracked insertions dropped.
    ///
    /// # Errors
    /// See [`Document::text_with_revisions_accepted`].
    pub fn text_with_revisions_rejected(&mut self) -> Result<String, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        Ok(revisions::text_with_rejected(body.content()))
    }

    // ---------------------------------------------------------------------------------------------
    // Footnotes (`word/footnotes.xml`, MJXOFF-124)
    // ---------------------------------------------------------------------------------------------

    /// Reads this document's `word/footnotes.xml`, handing `read` the parsed [`Footnotes`] together
    /// with the [`mjx_ooxml_core::Interner`] it was parsed with. `None` if this document relates to
    /// no `word/footnotes.xml` at all.
    ///
    /// # Errors
    /// Returns [`DocxError`] if `word/footnotes.xml` is related but cannot be read, is not
    /// well-formed, or its root is not `w:footnotes`.
    pub fn footnotes<R>(
        &mut self,
        read: impl FnOnce(&Footnotes, &mjx_ooxml_core::Interner) -> R,
    ) -> Result<Option<R>, DocxError> {
        let Some(footnotes_part) = self.parts.footnotes.clone() else {
            return Ok(None);
        };
        let doc = self.package.part_tree(&footnotes_part)?;
        let footnotes = Footnotes::from_xml(&doc.root, &doc.interner)?;
        Ok(Some(read(&footnotes, &doc.interner)))
    }

    /// Edits this document's footnotes, creating `word/footnotes.xml` — with the two reserved
    /// separator entries every footnotes part needs (see `annotations.rs`'s own doc comment), its
    /// content-type registration and its `footnotes` relationship from the main document part — first
    /// if it does not already have one.
    ///
    /// # Errors
    /// Returns [`DocxError`] if `word/footnotes.xml` is related but cannot be read, or if creating a
    /// missing one fails.
    pub fn edit_footnotes<R>(
        &mut self,
        edit: impl FnOnce(&mut Footnotes, &mut mjx_ooxml_core::Interner) -> R,
    ) -> Result<R, DocxError> {
        let footnotes_part = match &self.parts.footnotes {
            Some(part) => part.clone(),
            None => self.create_footnotes_part()?,
        };
        let doc = self.package.part_tree_mut(&footnotes_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut footnotes = if root.name.local == interner.intern("footnotes") {
            Footnotes::from_xml(root, interner)?
        } else {
            return Err(DocxError::MalformedDocument(
                "word/footnotes.xml root is not w:footnotes",
            ));
        };
        let result = edit(&mut footnotes, interner);
        footnotes.write_back(root, interner);
        Ok(result)
    }

    /// Creates `word/footnotes.xml` carrying only the two reserved separator entries, registers its
    /// content type, and relates it from the main document part.
    fn create_footnotes_part(&mut self) -> Result<mjx_opc::PartName, DocxError> {
        let footnotes_part = self.document_part.resolve("footnotes.xml").map_err(|_| {
            DocxError::TargetResolution {
                target: "footnotes.xml".to_owned(),
            }
        })?;
        const WML_NAMESPACE: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
        let bytes = format!(
            concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                "\n",
                r#"<w:footnotes xmlns:w="{ns}"/>"#,
            ),
            ns = WML_NAMESPACE,
        )
        .into_bytes();
        self.package.insert_part(
            &footnotes_part,
            crate::constants::CONTENT_TYPE_FOOTNOTES,
            bytes,
        )?;
        let rid = self.next_rid_for(&self.document_part.clone());
        self.package.add_relationship(
            Some(&self.document_part),
            mjx_opc::Relationship {
                id: rid,
                rel_type: crate::constants::REL_FOOTNOTES.to_owned(),
                target: "footnotes.xml".to_owned(),
                mode: mjx_opc::TargetMode::Internal,
            },
        )?;
        self.parts.footnotes = Some(footnotes_part.clone());
        // Populate the two reserved entries through the normal part_tree_mut/FromXml path (the typed
        // model only ever mutates a tree it actually read — see `headers::initial_bytes`'s own doc
        // comment for why this is a second round trip rather than writing the reserved entries into
        // the literal bytes above directly).
        {
            let doc = self.package.part_tree_mut(&footnotes_part)?;
            let RawDocument { interner, root, .. } = doc;
            let blank = annotations::Footnotes::blank(interner);
            blank.write_back(root, interner);
        }
        Ok(footnotes_part)
    }

    /// Adds a new user footnote, appending a `w:footnoteReference` run to the **end** of the paragraph
    /// at `paragraph` and a new [`FootnoteEndnote`] holding `text` as its own paragraph to
    /// `word/footnotes.xml` — creating that part first if the document has none. Returns the
    /// footnote's own freshly assigned id (one past the highest id already in the part, reserved
    /// entries included — see `annotations.rs`'s own doc comment for why reserved ids are never
    /// reused either).
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body,
    /// [`DocxError::AddressNotFound`] if `paragraph` does not address a paragraph, or another
    /// [`DocxError`] if the package edit fails.
    pub fn add_footnote(
        &mut self,
        paragraph: impl Into<BlockPath>,
        text: &str,
    ) -> Result<i64, DocxError> {
        let paragraph_path = paragraph.into();
        {
            let doc = self.package.part_tree(&self.document_part)?;
            let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
            let body = main.body().ok_or(DocxError::NoBody)?;
            body.paragraph(&paragraph_path).ok_or_else(|| {
                DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
            })?;
        }

        let footnotes_part = match &self.parts.footnotes {
            Some(part) => part.clone(),
            None => self.create_footnotes_part()?,
        };
        let id = {
            let doc = self.package.part_tree_mut(&footnotes_part)?;
            let RawDocument { interner, root, .. } = doc;
            let mut footnotes = if root.name.local == interner.intern("footnotes") {
                Footnotes::from_xml(root, interner)?
            } else {
                return Err(DocxError::MalformedDocument(
                    "word/footnotes.xml root is not w:footnotes",
                ));
            };
            let id = footnotes.next_user_id(interner);
            let mut entry = FootnoteEndnote::new(interner, "footnote", id);
            if let Some(p) = entry.paragraph_mut(0) {
                p.append_run(Run::with_text(interner, text));
            }
            footnotes.push(entry);
            footnotes.write_back(root, interner);
            id
        };

        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph_mut = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        let reference = FootnoteEndnoteReference::new(interner, "footnoteReference", id);
        paragraph_mut.append_run(Run::with_inner_content(
            interner,
            RunInnerContent::FootnoteReference(reference),
        ));
        main.write_back(root, interner);
        Ok(id)
    }

    /// Removes the footnote with `id`: every `w:footnoteReference` naming it anywhere in the body
    /// (recursing into every table cell), and the [`FootnoteEndnote`] entry itself from
    /// `word/footnotes.xml`. **Never** removes the part itself, even when no user footnote remains —
    /// the two reserved separator entries still must be there (`annotations.rs`'s own doc comment);
    /// unlike [`Document::remove_comment`], "the last one" for footnotes only ever means the last
    /// *user* footnote, and the part they share with the reserved entries stays.
    ///
    /// Not an error if `id` names no footnote at all, or names a reserved entry (refused silently
    /// rather than corrupting the part Word itself would repair — a caller has no business removing
    /// `separator`/`continuationSeparator`/`continuationNotice`).
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or another [`DocxError`] if the
    /// package edit fails.
    pub fn remove_footnote(&mut self, id: i64) -> Result<(), DocxError> {
        let Some(footnotes_part) = self.parts.footnotes.clone() else {
            return Ok(());
        };
        let is_user = {
            let doc = self.package.part_tree(&footnotes_part)?;
            let footnotes = Footnotes::from_xml(&doc.root, &doc.interner)?;
            footnotes
                .footnote(&doc.interner, id)
                .is_some_and(|footnote| footnote.is_user_visible(&doc.interner))
        };
        if !is_user {
            return Ok(());
        }

        {
            let doc = self.package.part_tree_mut(&self.document_part)?;
            let RawDocument { interner, root, .. } = doc;
            let mut main = MainDocument::from_xml(root, interner)?;
            let body = main.body_mut().ok_or(DocxError::NoBody)?;
            ranges::remove_matching(
                body.content_mut(),
                &|_: &ParagraphContent| false,
                &|item: &RunInnerContent| match item {
                    RunInnerContent::FootnoteReference(marker) => marker.id(interner) == Ok(id),
                    _ => false,
                },
            );
            main.write_back(root, interner);
        }

        let doc = self.package.part_tree_mut(&footnotes_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut footnotes = if root.name.local == interner.intern("footnotes") {
            Footnotes::from_xml(root, interner)?
        } else {
            return Err(DocxError::MalformedDocument(
                "word/footnotes.xml root is not w:footnotes",
            ));
        };
        footnotes.remove(interner, id);
        footnotes.write_back(root, interner);
        Ok(())
    }

    // ---------------------------------------------------------------------------------------------
    // Endnotes (`word/endnotes.xml`, MJXOFF-124) — the same shape as footnotes, above.
    // ---------------------------------------------------------------------------------------------

    /// As [`Document::footnotes`], for endnotes.
    ///
    /// # Errors
    /// Returns [`DocxError`] if `word/endnotes.xml` is related but cannot be read, is not
    /// well-formed, or its root is not `w:endnotes`.
    pub fn endnotes<R>(
        &mut self,
        read: impl FnOnce(&Endnotes, &mjx_ooxml_core::Interner) -> R,
    ) -> Result<Option<R>, DocxError> {
        let Some(endnotes_part) = self.parts.endnotes.clone() else {
            return Ok(None);
        };
        let doc = self.package.part_tree(&endnotes_part)?;
        let endnotes = Endnotes::from_xml(&doc.root, &doc.interner)?;
        Ok(Some(read(&endnotes, &doc.interner)))
    }

    /// As [`Document::edit_footnotes`], for endnotes.
    ///
    /// # Errors
    /// Returns [`DocxError`] if `word/endnotes.xml` is related but cannot be read, or if creating a
    /// missing one fails.
    pub fn edit_endnotes<R>(
        &mut self,
        edit: impl FnOnce(&mut Endnotes, &mut mjx_ooxml_core::Interner) -> R,
    ) -> Result<R, DocxError> {
        let endnotes_part = match &self.parts.endnotes {
            Some(part) => part.clone(),
            None => self.create_endnotes_part()?,
        };
        let doc = self.package.part_tree_mut(&endnotes_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut endnotes = if root.name.local == interner.intern("endnotes") {
            Endnotes::from_xml(root, interner)?
        } else {
            return Err(DocxError::MalformedDocument(
                "word/endnotes.xml root is not w:endnotes",
            ));
        };
        let result = edit(&mut endnotes, interner);
        endnotes.write_back(root, interner);
        Ok(result)
    }

    /// As [`Document::create_footnotes_part`], for endnotes.
    fn create_endnotes_part(&mut self) -> Result<mjx_opc::PartName, DocxError> {
        let endnotes_part = self.document_part.resolve("endnotes.xml").map_err(|_| {
            DocxError::TargetResolution {
                target: "endnotes.xml".to_owned(),
            }
        })?;
        const WML_NAMESPACE: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
        let bytes = format!(
            concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                "\n",
                r#"<w:endnotes xmlns:w="{ns}"/>"#,
            ),
            ns = WML_NAMESPACE,
        )
        .into_bytes();
        self.package.insert_part(
            &endnotes_part,
            crate::constants::CONTENT_TYPE_ENDNOTES,
            bytes,
        )?;
        let rid = self.next_rid_for(&self.document_part.clone());
        self.package.add_relationship(
            Some(&self.document_part),
            mjx_opc::Relationship {
                id: rid,
                rel_type: crate::constants::REL_ENDNOTES.to_owned(),
                target: "endnotes.xml".to_owned(),
                mode: mjx_opc::TargetMode::Internal,
            },
        )?;
        self.parts.endnotes = Some(endnotes_part.clone());
        {
            let doc = self.package.part_tree_mut(&endnotes_part)?;
            let RawDocument { interner, root, .. } = doc;
            let blank = annotations::Endnotes::blank(interner);
            blank.write_back(root, interner);
        }
        Ok(endnotes_part)
    }

    /// As [`Document::add_footnote`], for endnotes.
    ///
    /// # Errors
    /// See [`Document::add_footnote`].
    pub fn add_endnote(
        &mut self,
        paragraph: impl Into<BlockPath>,
        text: &str,
    ) -> Result<i64, DocxError> {
        let paragraph_path = paragraph.into();
        {
            let doc = self.package.part_tree(&self.document_part)?;
            let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
            let body = main.body().ok_or(DocxError::NoBody)?;
            body.paragraph(&paragraph_path).ok_or_else(|| {
                DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
            })?;
        }

        let endnotes_part = match &self.parts.endnotes {
            Some(part) => part.clone(),
            None => self.create_endnotes_part()?,
        };
        let id = {
            let doc = self.package.part_tree_mut(&endnotes_part)?;
            let RawDocument { interner, root, .. } = doc;
            let mut endnotes = if root.name.local == interner.intern("endnotes") {
                Endnotes::from_xml(root, interner)?
            } else {
                return Err(DocxError::MalformedDocument(
                    "word/endnotes.xml root is not w:endnotes",
                ));
            };
            let id = endnotes.next_user_id(interner);
            let mut entry = FootnoteEndnote::new(interner, "endnote", id);
            if let Some(p) = entry.paragraph_mut(0) {
                p.append_run(Run::with_text(interner, text));
            }
            endnotes.push(entry);
            endnotes.write_back(root, interner);
            id
        };

        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph_mut = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        let reference = FootnoteEndnoteReference::new(interner, "endnoteReference", id);
        paragraph_mut.append_run(Run::with_inner_content(
            interner,
            RunInnerContent::EndnoteReference(reference),
        ));
        main.write_back(root, interner);
        Ok(id)
    }

    /// As [`Document::remove_footnote`], for endnotes.
    ///
    /// # Errors
    /// See [`Document::remove_footnote`].
    pub fn remove_endnote(&mut self, id: i64) -> Result<(), DocxError> {
        let Some(endnotes_part) = self.parts.endnotes.clone() else {
            return Ok(());
        };
        let is_user = {
            let doc = self.package.part_tree(&endnotes_part)?;
            let endnotes = Endnotes::from_xml(&doc.root, &doc.interner)?;
            endnotes
                .endnote(&doc.interner, id)
                .is_some_and(|endnote| endnote.is_user_visible(&doc.interner))
        };
        if !is_user {
            return Ok(());
        }

        {
            let doc = self.package.part_tree_mut(&self.document_part)?;
            let RawDocument { interner, root, .. } = doc;
            let mut main = MainDocument::from_xml(root, interner)?;
            let body = main.body_mut().ok_or(DocxError::NoBody)?;
            ranges::remove_matching(
                body.content_mut(),
                &|_: &ParagraphContent| false,
                &|item: &RunInnerContent| match item {
                    RunInnerContent::EndnoteReference(marker) => marker.id(interner) == Ok(id),
                    _ => false,
                },
            );
            main.write_back(root, interner);
        }

        let doc = self.package.part_tree_mut(&endnotes_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut endnotes = if root.name.local == interner.intern("endnotes") {
            Endnotes::from_xml(root, interner)?
        } else {
            return Err(DocxError::MalformedDocument(
                "word/endnotes.xml root is not w:endnotes",
            ));
        };
        endnotes.remove(interner, id);
        endnotes.write_back(root, interner);
        Ok(())
    }

    // ---------------------------------------------------------------------------------------------
    // Bookmarks (`w:bookmarkStart`/`w:bookmarkEnd`, MJXOFF-124) — the range mechanism applied to a
    // marker pair with no second part behind it, and the resolution target for MJXOFF-121's own
    // `Hyperlink::anchor` (see `hyperlinks.rs`'s own doc comment).
    // ---------------------------------------------------------------------------------------------

    /// Adds a bookmark named `name` around the **whole** paragraph at `paragraph` — see
    /// [`Document::add_comment`]'s own doc comment for why this writer only ever authors that one
    /// shape. Returns the bookmark's own freshly assigned id (one past the highest bookmark id
    /// already in the body).
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body,
    /// [`DocxError::AddressNotFound`] if `paragraph` does not address a paragraph, or
    /// [`DocxError::BookmarkNameInUse`] if another bookmark anywhere in the body already carries
    /// `name` — refused here (an already-over-long/duplicate value read from an untrusted file is
    /// still read, never rejected; only a caller's own *new* value is, the same fidelity-versus-
    /// validity split `fields.rs`'s own module doc documents for its four length-bounded strings) so
    /// that [`Document::resolve_bookmark`] never has to guess which of two same-named bookmarks a
    /// `w:hyperlink w:anchor` meant.
    pub fn add_bookmark(
        &mut self,
        paragraph: impl Into<BlockPath>,
        name: &str,
    ) -> Result<i64, DocxError> {
        let paragraph_path = paragraph.into();
        let id = {
            let doc = self.package.part_tree(&self.document_part)?;
            let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
            let body = main.body().ok_or(DocxError::NoBody)?;
            body.paragraph(&paragraph_path).ok_or_else(|| {
                DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
            })?;
            let already_used = ranges::flatten_paragraphs(body.content())
                .into_iter()
                .any(|p| {
                    p.content().iter().any(|item| match item {
                        ParagraphContent::BookmarkStart(bookmark) => {
                            bookmark.name(&doc.interner).as_deref() == Some(name)
                        }
                        _ => false,
                    })
                });
            if already_used {
                return Err(DocxError::BookmarkNameInUse(name.to_owned()));
            }
            let index = RangeIndex::build(body.content(), &doc.interner, ranges::classify_bookmark);
            index.max_id().map_or(1, |max| max + 1)
        };

        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let paragraph_mut = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        let content = paragraph_mut.content_mut();
        content.insert(
            0,
            ParagraphContent::BookmarkStart(Bookmark::new(interner, id, name)),
        );
        content.push(ParagraphContent::BookmarkEnd(MarkupRange::new(
            interner,
            "bookmarkEnd",
            id,
        )));
        main.write_back(root, interner);
        Ok(id)
    }

    /// Removes every `w:bookmarkStart`/`w:bookmarkEnd` naming `id` anywhere in the body (recursing
    /// into every table cell — see `ranges::remove_matching`'s own doc comment for the one
    /// documented gap). Not an error if `id` names no bookmark at all.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or another [`DocxError`] if the
    /// package edit fails.
    pub fn remove_bookmark(&mut self, id: i64) -> Result<(), DocxError> {
        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        ranges::remove_matching(
            body.content_mut(),
            &|item: &ParagraphContent| match item {
                ParagraphContent::BookmarkStart(bookmark) => bookmark.id(interner) == Ok(id),
                ParagraphContent::BookmarkEnd(marker) => marker.id(interner) == Ok(id),
                _ => false,
            },
            &|_: &RunInnerContent| false,
        );
        main.write_back(root, interner);
        Ok(())
    }

    /// Resolves a bookmark by `name` against the body's own bookmark index: `None` if no
    /// `w:bookmarkStart` anywhere in the body carries this name, [`BookmarkResolution::UnmatchedStart`]
    /// if one does but no `w:bookmarkEnd` shares its id (ECMA-376 Part 1 §17.13.6.2 calls this
    /// non-conformant; real files have it anyway), or [`BookmarkResolution::Resolved`] with the
    /// bookmark's own id and the text it covers.
    ///
    /// **This is the seam MJXOFF-121's own `Hyperlink::anchor` was left unresolved for** — that type's
    /// own doc comment says so directly: *"`anchor` naming a bookmark is not resolved against a
    /// bookmark index here — that index is MJXOFF-124's own."* A caller who reads
    /// [`crate::HyperlinkTarget::Anchor`] hands the raw name it carries straight to this method to
    /// finish the resolution:
    ///
    /// ```
    /// # fn main() -> Result<(), mjx_docx::DocxError> {
    /// use mjx_docx::{BookmarkResolution, Document, HyperlinkTarget, PageSize};
    ///
    /// // `Document::blank` already starts with one (empty) paragraph, at index 0.
    /// let mut document = Document::blank(PageSize::a4())?;
    /// document.add_bookmark(0, "Target")?;
    /// document.append_paragraph()?;
    /// document.insert_hyperlink(1, 0, "jump", &HyperlinkTarget::Anchor("Target".to_owned()))?;
    ///
    /// let HyperlinkTarget::Anchor(name) = document.hyperlink_target(1, 0)?.unwrap() else {
    ///     panic!("this hyperlink names an anchor")
    /// };
    /// match document.resolve_bookmark(&name)?.unwrap() {
    ///     BookmarkResolution::Resolved { id, text } => {
    ///         assert_eq!(text, "");
    ///         let _ = id;
    ///     }
    ///     BookmarkResolution::UnmatchedStart { .. } => panic!("this bookmark is well-formed"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or another [`DocxError`] if the
    /// main document part cannot be read.
    pub fn resolve_bookmark(
        &mut self,
        name: &str,
    ) -> Result<Option<BookmarkResolution>, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let target_id = ranges::flatten_paragraphs(body.content())
            .into_iter()
            .find_map(|p| {
                p.content().iter().find_map(|item| match item {
                    ParagraphContent::BookmarkStart(bookmark)
                        if bookmark.name(&doc.interner).as_deref() == Some(name) =>
                    {
                        bookmark.id(&doc.interner).ok()
                    }
                    _ => None,
                })
            });
        let Some(id) = target_id else {
            return Ok(None);
        };
        let index = RangeIndex::build(body.content(), &doc.interner, ranges::classify_bookmark);
        Ok(Some(match index.get(id) {
            Some(RangeResolution::Resolved { start, end }) => BookmarkResolution::Resolved {
                id,
                text: covered_text(body.content(), start, end),
            },
            _ => BookmarkResolution::UnmatchedStart { id },
        }))
    }

    /// Validates the document, then serializes it back to container bytes (only edited parts
    /// re-serialize).
    ///
    /// Mirrors `Presentation::save`: the check is not optional here either, for the same reason —
    /// [`save_unchecked`](Self::save_unchecked) is the deliberate escape hatch. Today's check is the
    /// packaging graph alone ([`Package::validate`]); a WordprocessingML-specific invariant checker
    /// is not this child's to build (nothing in this crate authors markup yet that could violate
    /// one).
    ///
    /// # Errors
    /// Returns [`DocxError::Opc`] (carrying an [`mjx_opc::OpcError::Invalid`]) if the package
    /// violates an invariant, or another [`DocxError`] if the ZIP writer fails.
    pub fn save(&self) -> Result<Vec<u8>, DocxError> {
        self.validate()?;
        self.save_unchecked()
    }

    /// Serializes the document back to container bytes **without** checking its invariants.
    ///
    /// # Errors
    /// Returns [`DocxError`] if the ZIP writer fails.
    pub fn save_unchecked(&self) -> Result<Vec<u8>, DocxError> {
        Ok(self.package.save_unchecked()?)
    }

    /// Checks the packaging graph ([`Package::validate`]), without writing anything.
    ///
    /// # Errors
    /// Returns [`DocxError::Opc`] carrying the first packaging defect found.
    pub fn validate(&self) -> Result<(), DocxError> {
        self.package.validate().map_err(mjx_opc::OpcError::from)?;
        Ok(())
    }
}

/// `CT_Document` — the `w:document` root's own content: the optional page background it extends
/// from `CT_DocumentBase` (see [`Background`]'s doc comment for why that base type has no struct of
/// its own), then the optional body, then `@conformance`.
///
/// Only `@conformance` and the two typed children are modeled; every other attribute (the eleven
/// namespace declarations and `mc:Ignorable="w14 wp14 w15"` `tests/fixtures/sample.docx`'s root
/// carries) passes straight through `attributes`, verbatim, in position — the same fidelity
/// `mjx-dml`'s `TextBody` documents for what it does not model.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "conformance", codec = Enumeration<ConformanceClass>, accessor = conformance))]
pub struct MainDocument {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "background", variant = Background, ty = Background),
        child(local = "body", variant = Body, ty = Body)
    )]
    content: Vec<MainDocumentContent>,
}

/// One ordered child of a [`MainDocument`]: a typed [`Background`] or [`Body`], or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MainDocumentContent {
    /// `w:background` (`CT_Background`).
    Background(Background),
    /// `w:body` (`CT_Body`).
    Body(Body),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

impl MainDocument {
    /// The document's body (`w:body`), or `None` if it declares none (legal: `CT_Document`'s `body`
    /// is `minOccurs="0"`).
    #[must_use]
    pub fn body(&self) -> Option<&Body> {
        self.content.iter().find_map(|item| match item {
            MainDocumentContent::Body(body) => Some(body),
            _ => None,
        })
    }

    /// The document's page background (`w:background`), or `None` if it declares none.
    #[must_use]
    pub fn background(&self) -> Option<&Background> {
        self.content.iter().find_map(|item| match item {
            MainDocumentContent::Background(background) => Some(background),
            _ => None,
        })
    }

    /// The document's body (`w:body`), mutably, or `None` if it declares none.
    pub fn body_mut(&mut self) -> Option<&mut Body> {
        self.content.iter_mut().find_map(|item| match item {
            MainDocumentContent::Body(body) => Some(body),
            _ => None,
        })
    }
}

/// Whether `element` is `local` in the WordprocessingML namespace (Transitional or Strict) — the
/// same permissive namespace check [`Document::from_package`] already applies to the document root,
/// reused here for `w:evenAndOddHeaders` ([`Document::even_and_odd_headers`]) and a header/footer
/// part's own root ([`check_header_footer_root`]).
fn is_wml_element(
    element: &mjx_ooxml_core::RawElement,
    interner: &mjx_ooxml_core::Interner,
    local: &str,
) -> bool {
    let element_local = interner.resolve(element.name.local);
    let namespace = element.name.namespace.map(|s| interner.resolve(s));
    element_local == local && (namespace == Some(WML.transitional) || namespace == WML.strict)
}

/// Rejects a part whose root is not `w:hdr`/`w:ftr` — the same "cannot hand a part to the wrong
/// model" defensiveness [`Document::style_sheet`]/[`Document::edit_numbering`] already apply to
/// theirs.
fn check_header_footer_root(
    root: &mjx_ooxml_core::RawElement,
    interner: &mjx_ooxml_core::Interner,
) -> Result<(), DocxError> {
    if is_wml_element(root, interner, "hdr") || is_wml_element(root, interner, "ftr") {
        Ok(())
    } else {
        Err(DocxError::MalformedDocument(
            "part root is not w:hdr or w:ftr",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_fixtures::fixture;
    use mjx_ooxml_core::RawElement;

    /// The retained [`RawElement::source_span`] of the second `<w:p>` under `<w:body>` — `None` if
    /// the document has been reflowed rather than copied verbatim at that point in the tree.
    ///
    /// Same-crate access to `Document::package`/`document_part` is what makes this test possible at
    /// all: the span is an internal fact about the live in-memory tree, not something the public API
    /// exposes (nor should it — a caller does not need to know which bytes were copied, only that
    /// they were).
    fn sibling_paragraph_span(document: &mut Document) -> Option<std::ops::Range<u32>> {
        let doc = document
            .package
            .part_tree(&document.document_part)
            .expect("read word/document.xml");
        let body = doc.root.children.iter().find_map(|node| match node {
            RawNode::Element(element) if doc.interner.resolve(element.name.local) == "body" => {
                Some(element)
            }
            _ => None,
        })?;
        body.children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(element) if doc.interner.resolve(element.name.local) == "p" => {
                    Some(element)
                }
                _ => None,
            })
            .nth(1)
            .and_then(RawElement::source_span)
    }

    /// The retained [`RawElement::source_span`] of the `index`-th `<w:tr>` under the first `<w:tbl>`
    /// under `<w:body>` of `ragged_table.docx` (MJXOFF-116's own fixture — see
    /// `crates/mjx-docx/tests/tables.rs`'s own module doc comment for its geometry) — `None` if that
    /// row has been reflowed rather than copied verbatim. Same-crate access, exactly as
    /// [`sibling_paragraph_span`] above.
    fn table_row_span(document: &mut Document, index: usize) -> Option<std::ops::Range<u32>> {
        let doc = document
            .package
            .part_tree(&document.document_part)
            .expect("read word/document.xml");
        let body = doc.root.children.iter().find_map(|node| match node {
            RawNode::Element(element) if doc.interner.resolve(element.name.local) == "body" => {
                Some(element)
            }
            _ => None,
        })?;
        let table = body.children.iter().find_map(|node| match node {
            RawNode::Element(element) if doc.interner.resolve(element.name.local) == "tbl" => {
                Some(element)
            }
            _ => None,
        })?;
        table
            .children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(element) if doc.interner.resolve(element.name.local) == "tr" => {
                    Some(element)
                }
                _ => None,
            })
            .nth(index)
            .and_then(RawElement::source_span)
    }

    /// Row-level copy-on-write, proved the same way [`editing_one_run_retains_the_untouched_sibling_
    /// paragraphs_source_span`] proves it for a paragraph: editing one cell's text
    /// ([`Document::set_cell_text`]) must leave every other row's retained
    /// [`RawElement::source_span`] untouched — not merely byte-equal (which a complete reflow could
    /// still coincidentally reproduce), but the *same, retained* span, proving the untouched rows'
    /// bytes were copied verbatim rather than rebuilt from the model.
    ///
    /// Confirmed by hand: neutralising the run-replacement in [`Document::set_cell_text`] (so the
    /// method touches nothing) turns this red at its own `assert_ne!` —
    /// `left: Some(614..815), right: Some(614..815)` — because a no-op edit cannot be distinguished
    /// from span retention; restored by re-editing, not `git checkout --`.
    #[test]
    fn editing_one_cells_text_retains_every_other_rows_source_span() {
        let mut document =
            Document::open(&fixture("ragged_table.docx")).expect("open ragged_table.docx");

        let before: Vec<Option<std::ops::Range<u32>>> = (0..4)
            .map(|row| table_row_span(&mut document, row))
            .collect();
        assert!(
            before.iter().all(Option::is_some),
            "every freshly parsed, never-touched row always has a span"
        );

        document
            .set_cell_text(0, 1, 0, "edited")
            .expect("edit row 1's own first cell");

        let after: Vec<Option<std::ops::Range<u32>>> = (0..4)
            .map(|row| table_row_span(&mut document, row))
            .collect();
        assert_ne!(
            before[1], after[1],
            "the edited row's own span must change — otherwise this test could not distinguish \
             span retention from a coincidence"
        );
        for row in [0, 2, 3] {
            assert_eq!(
                before[row], after[row],
                "editing row 1's cell must not disturb row {row}'s retained source span"
            );
        }
    }

    /// Edit isolation, proved at the mechanism `sample.docx`'s whole-part byte identity cannot
    /// distinguish from a lucky coincidence: `sample.docx` has no incidental whitespace or unusual
    /// attribute formatting, so a *complete* reflow from the model (bypassing
    /// [`ToXml::write_back`]'s span-preserving restore entirely) still happens to reproduce
    /// byte-identical output for this fixture — confirmed by hand while developing this test, and
    /// the reason this test checks [`RawElement::source_span`] directly rather than only comparing
    /// bytes. A element that has been reflowed from the model, rather than copied verbatim, carries
    /// no span at all (`None`), regardless of whether its bytes happen to still match.
    #[test]
    fn editing_one_run_retains_the_untouched_sibling_paragraphs_source_span() {
        let mut document = Document::open(&fixture("sample.docx")).expect("open sample.docx");

        let before = sibling_paragraph_span(&mut document);
        assert!(
            before.is_some(),
            "a freshly parsed, never-touched element always has a span"
        );

        document
            .set_run_text(0, 0, "Edited text.")
            .expect("edit paragraph 0's run");

        let after = sibling_paragraph_span(&mut document);
        assert_eq!(
            before, after,
            "editing paragraph 0's run must not disturb paragraph 1's retained source span"
        );
    }
}
