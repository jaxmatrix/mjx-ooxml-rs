//! Part classification and the document's part graph.
//!
//! `wml.xsd` declares exactly **14 global elements**; each is the root of its own OPC part, except
//! `w:txbxContent`, which is an *inline* root — it appears nested inside a drawing's text box, never
//! as the target of a relationship, so it is not one of [`PartKind`]'s variants (see the type's own
//! doc comment). [`DocumentParts::resolve`] is the part graph: starting from the package-root
//! `officeDocument` relationship, it enumerates the main document part's own relationships by type,
//! so a caller (or [`crate::Document`]) can reach `styles.xml`, `fontTable.xml`, and everything else
//! `tests/fixtures/sample.docx` and a richer document alike may carry.

use mjx_opc::{Package, PartName, Relationship, Relationships, TargetMode};

use crate::constants;
use crate::error::DocxError;

/// One of `wml.xsd`'s 14 global elements — a part kind, in the sense that each names both a
/// relationship type (how a part graph reaches it) and a content type (how `[Content_Types].xml`
/// registers it). The one exception, `w:txbxContent`, is deliberately absent: it is an *inline*
/// root nested inside a drawing's text box, never the target of a relationship or its own part, so
/// it has neither a relationship type nor a content type to give.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartKind {
    /// `w:document` — the main document part (§11.3.10).
    Document,
    /// `w:glossaryDocument` — the glossary document part (§11.3.8).
    GlossaryDocument,
    /// `w:styles` — the style definitions part (§11.3.12).
    Styles,
    /// `w:numbering` — the numbering definitions part (§11.3.11).
    Numbering,
    /// `w:settings` — the document settings part (§11.3.3).
    Settings,
    /// `w:webSettings` — the web settings part (§11.3.13).
    WebSettings,
    /// `w:fonts` — the font table part (§11.3.5).
    FontTable,
    /// `w:hdr` — a header part (§11.3.9). A document may relate to several.
    Header,
    /// `w:ftr` — a footer part (§11.3.6). A document may relate to several.
    Footer,
    /// `w:comments` — the comments part (§11.3.2).
    Comments,
    /// `w:footnotes` — the footnotes part (§11.3.7).
    Footnotes,
    /// `w:endnotes` — the endnotes part (§11.3.4).
    Endnotes,
    /// `w:recipients` — the Mail Merge Recipient Data part (§17.14.28; see
    /// [`constants::CONTENT_TYPE_MAIL_MERGE_RECIPIENT_DATA`] for what is and is not spec-confirmed
    /// about it).
    Recipients,
}

impl PartKind {
    /// The relationship type a part graph reaches this kind through — from the package root for
    /// [`Document`](Self::Document), from the main document part for everything else (including,
    /// per ECMA-376, [`GlossaryDocument`](Self::GlossaryDocument)).
    #[must_use]
    pub fn relationship_type(self) -> &'static str {
        match self {
            Self::Document => constants::REL_OFFICE_DOCUMENT,
            Self::GlossaryDocument => constants::REL_GLOSSARY_DOCUMENT,
            Self::Styles => constants::REL_STYLES,
            Self::Numbering => constants::REL_NUMBERING,
            Self::Settings => constants::REL_SETTINGS,
            Self::WebSettings => constants::REL_WEB_SETTINGS,
            Self::FontTable => constants::REL_FONT_TABLE,
            Self::Header => constants::REL_HEADER,
            Self::Footer => constants::REL_FOOTER,
            Self::Comments => constants::REL_COMMENTS,
            Self::Footnotes => constants::REL_FOOTNOTES,
            Self::Endnotes => constants::REL_ENDNOTES,
            Self::Recipients => constants::REL_MAIL_MERGE_RECIPIENT_DATA,
        }
    }

    /// This kind's registered content type.
    #[must_use]
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Document => constants::CONTENT_TYPE_DOCUMENT,
            Self::GlossaryDocument => constants::CONTENT_TYPE_GLOSSARY_DOCUMENT,
            Self::Styles => constants::CONTENT_TYPE_STYLES,
            Self::Numbering => constants::CONTENT_TYPE_NUMBERING,
            Self::Settings => constants::CONTENT_TYPE_SETTINGS,
            Self::WebSettings => constants::CONTENT_TYPE_WEB_SETTINGS,
            Self::FontTable => constants::CONTENT_TYPE_FONT_TABLE,
            Self::Header => constants::CONTENT_TYPE_HEADER,
            Self::Footer => constants::CONTENT_TYPE_FOOTER,
            Self::Comments => constants::CONTENT_TYPE_COMMENTS,
            Self::Footnotes => constants::CONTENT_TYPE_FOOTNOTES,
            Self::Endnotes => constants::CONTENT_TYPE_ENDNOTES,
            Self::Recipients => constants::CONTENT_TYPE_MAIL_MERGE_RECIPIENT_DATA,
        }
    }
}

/// The main document part's own part graph: every other part `tests/fixtures/sample.docx` — or a
/// richer document — relates to, resolved once when a [`crate::Document`] is opened.
///
/// A singular relationship (`styles`, `numbering`, `settings`, `webSettings`, `fontTable`, `theme`,
/// `footnotes`, `endnotes`, `comments`, `glossaryDocument`, `recipients`) keeps at most one target,
/// matching what ECMA-376 allows; `header`/`footer` keep every match, since a document relates to as
/// many as its sections use. `sample.docx` itself carries only `styles`, `fontTable`, `settings` and
/// `theme` — the minimum real-world shape — so every other field is `None`/empty on that fixture.
#[derive(Debug, Clone, Default)]
pub struct DocumentParts {
    /// `styles.xml`, if related.
    pub styles: Option<PartName>,
    /// `numbering.xml`, if related.
    pub numbering: Option<PartName>,
    /// `settings.xml`, if related.
    pub settings: Option<PartName>,
    /// `webSettings.xml`, if related.
    pub web_settings: Option<PartName>,
    /// `fontTable.xml`, if related.
    pub font_table: Option<PartName>,
    /// `theme/themeN.xml`, if related. Not a `wml.xsd` element (it is DrawingML) — resolved here
    /// because it is still part of the main document part's own part graph.
    pub theme: Option<PartName>,
    /// Every related header part, in relationship order (not reading order — headers are reached by
    /// `r:id` from individual `w:sectPr`s, which are not yet modeled).
    pub headers: Vec<PartName>,
    /// Every related footer part, in relationship order (see [`headers`](Self::headers)).
    pub footers: Vec<PartName>,
    /// `footnotes.xml`, if related.
    pub footnotes: Option<PartName>,
    /// `endnotes.xml`, if related.
    pub endnotes: Option<PartName>,
    /// `comments.xml`, if related.
    pub comments: Option<PartName>,
    /// `glossary/document.xml`, if related.
    pub glossary_document: Option<PartName>,
    /// `recipients.xml`, if related (`w:recipients` — MJXOFF-136's own part; C1 declared
    /// [`PartKind::Recipients`] but never resolved it, since nothing modelled the part yet).
    pub recipients: Option<PartName>,
}

impl DocumentParts {
    /// Resolves every relationship of `document_part` this crate currently classifies, by type.
    ///
    /// A relationship type this method does not ask for (an application-defined extension, a
    /// `customXml` part, …) is simply not visited — it stays untouched in the package, exactly as
    /// [`mjx_opc::Package::authored_xml_parts`] preserves whatever a caller's edits never dirty.
    ///
    /// # Errors
    /// Returns [`DocxError::ExternalTarget`] if a relationship this method resolves has
    /// `TargetMode::External` (WordprocessingML's own parts are always internal), or
    /// [`DocxError::TargetResolution`] if a target does not resolve to a valid part name.
    pub(crate) fn resolve(package: &Package, document_part: &PartName) -> Result<Self, DocxError> {
        let Some(rels) = package.relationships_for(Some(document_part)) else {
            return Ok(Self::default());
        };
        Ok(Self {
            styles: single(document_part, rels, PartKind::Styles.relationship_type())?,
            numbering: single(document_part, rels, PartKind::Numbering.relationship_type())?,
            settings: single(document_part, rels, PartKind::Settings.relationship_type())?,
            web_settings: single(
                document_part,
                rels,
                PartKind::WebSettings.relationship_type(),
            )?,
            font_table: single(document_part, rels, PartKind::FontTable.relationship_type())?,
            theme: single(document_part, rels, constants::REL_THEME)?,
            headers: many(document_part, rels, PartKind::Header.relationship_type())?,
            footers: many(document_part, rels, PartKind::Footer.relationship_type())?,
            footnotes: single(document_part, rels, PartKind::Footnotes.relationship_type())?,
            endnotes: single(document_part, rels, PartKind::Endnotes.relationship_type())?,
            comments: single(document_part, rels, PartKind::Comments.relationship_type())?,
            glossary_document: single(
                document_part,
                rels,
                PartKind::GlossaryDocument.relationship_type(),
            )?,
            recipients: single(document_part, rels, PartKind::Recipients.relationship_type())?,
        })
    }
}

/// The first relationship of `rel_type` from `source`'s own `.rels`, resolved to a part name —
/// `None` if there is none. ECMA-376 permits at most one of each singular relationship type per
/// part, so "first" and "only" coincide for a conformant document; a non-conformant duplicate is not
/// rejected here (this crate does not yet validate WordprocessingML-specific invariants — see
/// [`crate::Document::validate`]).
fn single(
    source: &PartName,
    rels: &Relationships,
    rel_type: &str,
) -> Result<Option<PartName>, DocxError> {
    let Some(rel) = rels.by_type(rel_type).next() else {
        return Ok(None);
    };
    Ok(Some(resolve_one(source, rel)?))
}

/// Every relationship of `rel_type` from `source`'s own `.rels`, resolved to part names, in
/// relationship order.
fn many(
    source: &PartName,
    rels: &Relationships,
    rel_type: &str,
) -> Result<Vec<PartName>, DocxError> {
    rels.by_type(rel_type)
        .map(|rel| resolve_one(source, rel))
        .collect()
}

/// Resolves one relationship's target to a part name, rejecting an external one.
fn resolve_one(source: &PartName, rel: &Relationship) -> Result<PartName, DocxError> {
    if rel.mode == TargetMode::External {
        return Err(DocxError::ExternalTarget {
            target: rel.target.clone(),
        });
    }
    source
        .resolve(&rel.target)
        .map_err(|err| target_error(err, &rel.target))
}

/// Resolves a relationship `target` relative to the package root (base directory `/`) — used for the
/// package-root `officeDocument` relationship, which has no source part.
pub(crate) fn resolve_from_root(target: &str) -> Result<PartName, DocxError> {
    PartName::resolve_from_root(target).map_err(|err| target_error(err, target))
}

/// Restates an OPC target-resolution failure as the WordprocessingML error naming the same target.
fn target_error(err: mjx_opc::OpcError, target: &str) -> DocxError {
    match err {
        mjx_opc::OpcError::ExternalTarget(_) => DocxError::ExternalTarget {
            target: target.to_owned(),
        },
        mjx_opc::OpcError::TargetResolution(_) | mjx_opc::OpcError::Malformed(_) => {
            DocxError::TargetResolution {
                target: target.to_owned(),
            }
        }
        other => DocxError::from(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_part_kind_pairs_a_relationship_type_with_a_content_type() {
        // Not much of an assertion on its own, but it forces every arm of both matches to compile
        // and run — a new PartKind variant with a missing arm in either match fails to build, and
        // that is the discriminating property: this test cannot pass while a variant is unhandled.
        for kind in [
            PartKind::Document,
            PartKind::GlossaryDocument,
            PartKind::Styles,
            PartKind::Numbering,
            PartKind::Settings,
            PartKind::WebSettings,
            PartKind::FontTable,
            PartKind::Header,
            PartKind::Footer,
            PartKind::Comments,
            PartKind::Footnotes,
            PartKind::Endnotes,
            PartKind::Recipients,
        ] {
            assert!(!kind.relationship_type().is_empty());
            assert!(kind.content_type().ends_with("+xml"));
        }
    }

    #[test]
    fn document_and_glossary_document_content_types_stay_disambiguated() {
        // The one place the relationship-type-suffix pattern this module's content types otherwise
        // follow deliberately breaks — see `constants::CONTENT_TYPE_MAIL_MERGE_RECIPIENT_DATA`'s doc
        // comment for the other eleven. A regression here would silently collide two part kinds.
        assert_eq!(
            PartKind::Document.content_type(),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
        );
        assert_eq!(
            PartKind::GlossaryDocument.content_type(),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document.glossary+xml"
        );
        assert_ne!(
            PartKind::Document.content_type(),
            PartKind::GlossaryDocument.content_type()
        );
    }
}
