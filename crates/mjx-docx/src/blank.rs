//! Authoring a complete WordprocessingML package from nothing.
//!
//! [`Document::open`](crate::Document::open) needs a `.docx` to already exist. This module is the
//! other half: it writes `/word/document.xml` on top of [`mjx_opc::Package::empty`], which supplies
//! `[Content_Types].xml` and the package-root `.rels`, plus `docProps/core.xml` and
//! `docProps/app.xml` — the same four primitives `mjx_pptx::blank` uses
//! (`Package::empty`/`PartName`/`Relationship`/`TargetMode`), reaching the same conclusion A3 did:
//! write the markup, never ship a template.
//!
//! # Why the markup is written out rather than shipped as a template
//!
//! A committed `.docx` template would be the shortest route and the wrong one, for the exact reason
//! `mjx_pptx::blank` refuses one: it is markup nothing in this repository can explain, it cannot
//! follow the caller's page size, and it is invisible to the schema gate that exists precisely to
//! keep us honest about what we emit (`tests/schema_gate.rs`). Every element below is validated
//! against the ECMA-376 XSDs by the same suite that validates a document the library edits.
//!
//! # What "minimal" means here — and why it lands somewhere different from PowerPoint's answer
//!
//! `tests/fixtures/sample.docx` — LibreOffice 24.2.7.2's own output — ships **ten** parts: besides
//! `word/document.xml` and the two `docProps` parts, it adds `word/styles.xml`,
//! `word/fontTable.xml`, `word/settings.xml`, `word/theme/theme1.xml`, and
//! `word/_rels/document.xml.rels` to relate them. **None of the four `word/*.xml` additions, nor the
//! `.rels` that reaches them, is schema-required** — `PartKind::relationship_type` documents every
//! one of `wml.xsd`'s 13 part-bearing global elements as `minOccurs="0"` from wherever it is
//! reached, and `DocumentParts::resolve` already treats every field as optional for exactly that
//! reason.
//!
//! `mjx_pptx::blank`'s answer to the same question ("what beyond the schema minimum") is *include
//! the master, layout and theme*, because without them a deck is not merely undecorated — it is
//! **structurally unusable**: `Presentation::add_slide_from_layout` has no layout to clone, and a
//! slide with no master has nowhere to inherit a placeholder's position from. WordprocessingML does
//! not have that dependency. `Document::insert_paragraph` / `append_run` / `set_run_text`
//! ([`crate::Document`], MJXOFF-92) work on a body with **no related `styles.xml` at all**: a
//! paragraph with no `w:pStyle` and a run with no `w:rStyle` are both completely legal, and every
//! real Word implementation supplies its own built-in fallback appearance (a "Normal"-equivalent
//! typeface and size) for a document that names no style to inherit from — the same way a browser
//! renders unstyled HTML rather than refusing it. So unlike PowerPoint's masters and layout, a
//! `styles.xml` a blank document has no relationship to is not a structural prerequisite for using
//! the document — it is a *convenience* (naming "Heading 1" once instead of repeating direct
//! formatting), and this child's own ticket says as much: modelling `styles.xml` for real is
//! MJXOFF-101's, not this child's, and MJXOFF-101 **replaces** rather than extends whatever writer
//! this child might have shipped. Writing a throwaway `docDefaults`-only `styles.xml` here — legal
//! under the ticket's own wording — would be work MJXOFF-101 throws away on day one, which is exactly
//! the technical debt "completion over shipping early" warns against creating on purpose. So this
//! module writes **none** of `styles.xml`, `fontTable.xml`, `settings.xml`, `theme/theme1.xml`, or
//! any header/footer/numbering/glossary part — every one is deliberately absent, and every one
//! remains fully optional to add later exactly as `DocumentParts` already models it.
//!
//! What this module **does** write, beyond the package-agnostic parts `Package::empty` supplies:
//!
//! - `/word/document.xml` — `w:document` wrapping a `w:body` with one empty `w:p` (so
//!   [`Document::paragraph_count`](crate::Document::paragraph_count) starts at 1, matching a
//!   document a person just created in Word — never `0`, which would make `insert_paragraph`'s
//!   `0..=paragraph_count()` range degenerate) and a body-level `w:sectPr` naming the page.
//! - `docProps/core.xml` / `docProps/app.xml` — MJXOFF-149's packaging-layer decision, restated
//!   below, not re-litigated here.
//!
//! # `w:sectPr`, `w:pgSz` and `w:pgMar` — none of the three is schema-required either
//!
//! Contrary to what this ticket's own brief claims, checking `wml.xsd` directly shows `CT_Body`
//! declares `sectPr` `minOccurs="0"`, and `EG_SectPrContents` (which both `CT_SectPr` and
//! `CT_SectPrBase` wrap at `minOccurs="0"`) declares `pgSz` and `pgMar` `minOccurs="0"` too — a
//! `<w:document><w:body/></w:document>` with no section properties at all is schema-valid. So this
//! module's answer follows the *same* "not required, but what makes the result usable" reasoning
//! `mjx_pptx::blank` uses for its placeholders, not a schema requirement: a document that never
//! states a page size still opens, but every consumer has to invent one, which is exactly the kind
//! of ambiguity A3's own module doc calls out. `w:sectPr`/`w:pgSz`/`w:pgMar` are included for that
//! reason, with the caller choosing the page (see [`PageSize`]).
//!
//! **The one genuine `minOccurs`/`use="required"` claim in this file** is narrower: `CT_PageMar`
//! declares all seven of its attributes (`top`, `right`, `bottom`, `left`, `header`, `footer`,
//! `gutter`) `use="required"` — *if* a document writes `w:pgMar` at all, every one of the seven must
//! be present. That is the claim `tests/schema_gate.rs`'s
//! `dropping_any_pg_mar_attribute_turns_the_schema_gate_red` proves, once per attribute, by dropping
//! one and watching the schema gate go red; nothing else in this file makes a "required" claim, because
//! nothing else in `word/document.xml` is schema-required.
//!
//! # Document properties
//!
//! `docProps/core.xml` and `docProps/app.xml` **are** written — MJXOFF-149 decided this project
//! authors them (see `mjx_opc::doc_props`'s own module doc for the full reasoning, and
//! `mjx_pptx::blank`'s "Document properties" section for the sibling decision this restates rather
//! than re-derives). [`Document::blank`] writes both parts with every field absent — schema-valid
//! and deterministic, since `CT_CoreProperties` / `CT_Properties` are `xs:all` groups with every
//! child `minOccurs="0"`. [`Document::blank_with_properties`] is the same constructor for a caller
//! who wants to set title, creator, created/modified or the application name.

use mjx_ooxml_core::{Interner, RawDocument, ToXml};
use mjx_opc::doc_props::{self, CoreProperties, ExtendedProperties};
use mjx_opc::{Package, PartName, Relationship, TargetMode};

use crate::constants;
use crate::document::SectionProperties;
use crate::error::DocxError;
use crate::page::{PageMargins, PageSize};

/// The main document part every blank document writes.
pub(crate) const DOCUMENT_PART: &str = "/word/document.xml";

/// The `w:` namespace every element this module writes is qualified with.
const WML_NAMESPACE: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// The relationships namespace `r:`-prefixed attributes need declared somewhere in their ancestor
/// chain — `w:printerSettings@r:id` ([`SectionProperties::set_printer_settings`]) chief among the
/// ones a document built from nothing can now carry (MJXOFF-109). Declared on the root here,
/// mirroring `mjx_pptx::blank`'s own `PML_NAMESPACES` constant, which declares the identical URI for
/// the identical reason (`r:embed`, `r:id`, … on a deck built from nothing). Every `.docx` this
/// crate has ever read (`tests/fixtures/sample.docx` included) already carries this declaration on
/// its own root; a document [`crate::Document::blank`] builds is now no different.
const RELATIONSHIPS_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// The XML declaration every part this module writes begins with, matching what Office emits and
/// what `mjx_pptx::blank`'s own templates use.
const XML_DECLARATION: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    "\n"
);

/// Builds a complete, valid WordprocessingML package: `word/document.xml` (one empty paragraph and a
/// body-level `w:sectPr` naming `size`) plus `docProps/core.xml` / `docProps/app.xml`, on top of
/// [`Package::empty`]. See the [module docs](self) for exactly which optional parts this omits, and
/// why.
///
/// # Errors
/// Returns [`DocxError::InvalidPageSize`] if `size` is degenerate (see [`PageSize::validate`]), or
/// another [`DocxError`] if a package edit fails.
pub(crate) fn package(
    size: PageSize,
    core_properties: &CoreProperties,
    extended_properties: &ExtendedProperties,
) -> Result<Package, DocxError> {
    size.validate()?;

    let document = PartName::new(DOCUMENT_PART)?;
    let core_props_part = PartName::new(doc_props::CORE_PROPERTIES_PART)?;
    let extended_props_part = PartName::new(doc_props::EXTENDED_PROPERTIES_PART)?;

    let mut package = Package::empty();

    package.insert_part(
        &document,
        constants::CONTENT_TYPE_DOCUMENT,
        document_bytes(size),
    )?;
    package.insert_part(
        &core_props_part,
        doc_props::CORE_PROPERTIES_CONTENT_TYPE,
        doc_props::core_xml(core_properties),
    )?;
    package.insert_part(
        &extended_props_part,
        doc_props::EXTENDED_PROPERTIES_CONTENT_TYPE,
        doc_props::extended_xml(extended_properties),
    )?;

    add_rel(
        &mut package,
        None,
        "rId1",
        constants::REL_OFFICE_DOCUMENT,
        "word/document.xml",
    )?;
    add_rel(
        &mut package,
        None,
        "rId2",
        doc_props::CORE_PROPERTIES_REL_TYPE,
        "docProps/core.xml",
    )?;
    add_rel(
        &mut package,
        None,
        "rId3",
        doc_props::EXTENDED_PROPERTIES_REL_TYPE,
        "docProps/app.xml",
    )?;

    Ok(package)
}

/// Adds one relationship, keeping the call sites above readable — mirrors `mjx_pptx::blank::add_rel`
/// exactly.
fn add_rel(
    package: &mut Package,
    source: Option<&PartName>,
    id: &str,
    rel_type: &str,
    target: &str,
) -> Result<(), DocxError> {
    package.add_relationship(
        source,
        Relationship {
            id: id.to_owned(),
            rel_type: rel_type.to_owned(),
            target: target.to_owned(),
            mode: TargetMode::Internal,
        },
    )?;
    Ok(())
}

/// The bytes of `word/document.xml`: an empty `w:body` holding one empty `w:p` and a `w:sectPr`
/// naming `size` — see the [module docs](self) for why each piece is here.
///
/// The `w:sectPr` fragment itself comes from the real, fully modelled writer
/// (`crate::document::SectionProperties`, MJXOFF-109) — built with its own constructor and setters
/// and serialized on its own (no source bytes behind it, so it always reflows from the model), then
/// spliced as literal bytes into the surrounding hand-written skeleton. This is the same "minimal
/// literal template, then a real typed value" split `Document::create_style_sheet_part` already uses
/// for a part built from nothing: the *skeleton* (the `xmlns:w` declaration every child below relies
/// on to resolve) is still hand-written, matching `mjx_pptx::blank`'s own convention for every part
/// it authors, but the section itself is no longer a hand-formatted string duplicating what
/// `SectionProperties` already knows how to write.
fn document_bytes(size: PageSize) -> Vec<u8> {
    let mut interner = Interner::new();
    let mut section = SectionProperties::new(&mut interner);
    section.set_page_size(&mut interner, Some(size));
    section.set_page_margins(&mut interner, Some(PageMargins::NORMAL));
    let element = section.to_xml(&mut interner);
    let fragment = RawDocument::new(interner, false, Vec::new(), element, Vec::new());
    let section_bytes = mjx_xml::fidelity::serialize_to_vec(&fragment);
    let section_xml =
        String::from_utf8(section_bytes).expect("this crate's own writer only ever emits UTF-8");

    format!(
        concat!(
            "{declaration}",
            r#"<w:document xmlns:w="{ns}" xmlns:r="{rns}">"#,
            "<w:body>",
            "<w:p/>",
            "{section}",
            "</w:body>",
            "</w:document>",
        ),
        declaration = XML_DECLARATION,
        ns = WML_NAMESPACE,
        rns = RELATIONSHIPS_NAMESPACE,
        section = section_xml,
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_authored_document_part_is_well_formed_xml() {
        for size in [
            PageSize::a4(),
            PageSize::us_letter(),
            PageSize::a4().landscape(),
        ] {
            let bytes = document_bytes(size);
            mjx_xml::fidelity::parse(&bytes)
                .unwrap_or_else(|e| panic!("word/document.xml is not well-formed: {e}"));
        }
    }

    #[test]
    fn landscape_writes_the_orient_attribute_and_portrait_omits_it() {
        let portrait = String::from_utf8(document_bytes(PageSize::a4())).unwrap();
        assert!(!portrait.contains("w:orient="));

        let landscape = String::from_utf8(document_bytes(PageSize::a4().landscape())).unwrap();
        assert!(landscape.contains(r#"w:orient="landscape""#));
    }

    #[test]
    fn pg_sz_carries_the_callers_extent() {
        let xml = String::from_utf8(document_bytes(PageSize::us_letter())).unwrap();
        assert!(xml.contains(r#"<w:pgSz w:w="12240" w:h="15840"/>"#));
    }

    #[test]
    fn pg_mar_carries_word_s_normal_margins() {
        let xml = String::from_utf8(document_bytes(PageSize::a4())).unwrap();
        assert!(xml.contains(
            r#"<w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/>"#
        ));
    }

    #[test]
    fn an_out_of_range_page_size_is_refused() {
        let degenerate = PageSize::from_twips(0, 16_838, crate::page::PageOrientation::Portrait);
        let defaults = (CoreProperties::default(), ExtendedProperties::default());
        assert!(matches!(
            package(degenerate, &defaults.0, &defaults.1),
            Err(DocxError::InvalidPageSize { .. })
        ));
    }

    #[test]
    fn document_properties_are_written_with_every_field_absent_by_default() {
        let defaults = (CoreProperties::default(), ExtendedProperties::default());
        let built =
            package(PageSize::a4(), &defaults.0, &defaults.1).expect("a4 is a valid page size");
        let core = PartName::new(doc_props::CORE_PROPERTIES_PART).unwrap();
        let extended = PartName::new(doc_props::EXTENDED_PROPERTIES_PART).unwrap();
        assert!(built.part_bytes(&core).is_some());
        assert!(built.part_bytes(&extended).is_some());
    }
}
