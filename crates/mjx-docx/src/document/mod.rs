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
//! - `body.rs` — `w:body` (`CT_Body`) and `w:background` (`CT_Background`), **seeded here as fidelity
//!   wrappers with no typed content**; MJXOFF-92 gives `Body` real fields (paragraphs, tables, the
//!   closing `w:sectPr`) rather than starting the file from nothing.
//!
//! Files later children are expected to add, one subject each (the same seam `presentation/` reads
//! in, chosen from the module list MJXOFF-90's ticket named for MJXOFF-92 through the rest of Phase
//! C): `run_properties.rs`, `paragraph_properties.rs`, `styles.rs`, `numbering.rs`, `effective.rs`,
//! `sections.rs`, `headers.rs`, `tables.rs`, `fields.rs`, `annotations.rs`, `revisions.rs`,
//! `drawing.rs`, `settings.rs`, `structured_content.rs`. A child that needs a subject not on this
//! list adds the file and a line here, the same way `presentation/`'s own list grew past A8.

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, RawAttribute, RawDocument, RawName, RawNode, ToXml,
};
use mjx_ooxml_types::namespaces::WML;
use mjx_ooxml_types::shared::ConformanceClass;
use mjx_opc::{Package, TargetMode};

use crate::error::DocxError;

mod body;
mod parts;

pub use body::{Background, Body};
pub use parts::{DocumentParts, PartKind};

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

    /// Resolves an already-loaded [`Package`] into a document: the `officeDocument` relationship,
    /// the main document part, and its part graph.
    ///
    /// This is the constructor for a caller who already holds the package, exactly as
    /// `Presentation::from_package` is for a deck — see that method's own doc comment for why a
    /// package built from nothing (once MJXOFF-98 adds `Document::blank`) will go through this same
    /// resolution rather than surviving as a special case.
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
}
