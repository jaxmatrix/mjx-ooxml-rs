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
//!
//! Files later children are expected to add, one subject each (the same seam `presentation/` reads
//! in, chosen from the module list MJXOFF-90's ticket named for MJXOFF-92 through the rest of Phase
//! C): `styles.rs`, `numbering.rs`, `effective.rs`, `sections.rs`, `headers.rs`, `tables.rs`,
//! `fields.rs`, `annotations.rs`, `revisions.rs`, `drawing.rs`, `settings.rs`,
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
mod paragraph_properties;
mod parts;
mod run_properties;

pub use body::{
    Background, BlockContent, Body, Break, Hyperlink, Paragraph, ParagraphContent,
    PermissionRangeEnd, PermissionRangeStart, PhoneticGuide, PhoneticGuideChild,
    PhoneticGuideContent, PhoneticGuideContentItem, PhoneticGuideProperties,
    PhoneticGuidePropertyContent, PhoneticGuideTextAlignment, PositionalTab, ProofingError,
    RelationshipReference, Run, RunInnerContent, ShortHex, Symbol, Text, Unmodeled,
    WhitespacePreservation,
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
