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
//!
//! (This list previously named `styles.rs`, `numbering.rs`, `effective.rs` and `sections.rs` among
//! the files "later children are expected to add" — stale by the time MJXOFF-109 landed, all four
//! already existed. Fixed here rather than carried forward again.)
//!
//! Files later children are expected to add, one subject each (the same seam `presentation/` reads
//! in, chosen from the module list MJXOFF-90's ticket named for MJXOFF-92 through the rest of Phase
//! C): `fields.rs`, `annotations.rs`, `revisions.rs`, `drawing.rs`, `settings.rs`,
//! `structured_content.rs`. A child that needs a subject not on this list adds the file and a line
//! here, the same way `presentation/`'s own list grew past A8.

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, RawAttribute, RawDocument, RawName, RawNode, ToXml,
};
use mjx_ooxml_types::namespaces::WML;
use mjx_ooxml_types::shared::ConformanceClass;
use mjx_opc::{Package, TargetMode};

use crate::error::DocxError;

mod body;
mod effective;
mod headers;
mod numbering;
mod paragraph_properties;
mod parts;
mod property_macros;
mod run_properties;
mod sections;
mod styles;
mod tables;

pub use body::{
    Background, BlockContent, Body, Break, Hyperlink, Paragraph, ParagraphContent,
    PermissionRangeEnd, PermissionRangeStart, PhoneticGuide, PhoneticGuideChild,
    PhoneticGuideContent, PhoneticGuideContentItem, PhoneticGuideProperties,
    PhoneticGuidePropertyContent, PhoneticGuideTextAlignment, PositionalTab, ProofingError,
    RelationshipReference, Run, RunInnerContent, ShortHex, Symbol, Text, Unmodeled,
    WhitespacePreservation,
};
pub use effective::{
    EffectiveBorder, EffectiveCharacterProperties, EffectiveColor, EffectiveConditionalFormatting,
    EffectiveEastAsianLayout, EffectiveFonts, EffectiveFrameProperties, EffectiveIndentation,
    EffectiveLanguages, EffectiveManualRunWidth, EffectiveNumberingReference,
    EffectiveParagraphBorders, EffectiveParagraphProperties, EffectiveShading, EffectiveTabStop,
    EffectiveUnderline,
};
pub use headers::{HdrFtr, HeaderFooterType};
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
