//! The WordprocessingML corpus file: a long document (MJXOFF-147).
//!
//! `mjx-docx` has no model yet (Phase C) — `crates/mjx-docx/src/lib.rs` is a scaffold — so this
//! writes `word/document.xml` directly on [`mjx_opc::Package`], exactly the "open / tree-parse /
//! save" layer that exists today and that MJXOFF-90 will be measured against, rather than skipping
//! Word for want of a model (the trap MJXOFF-147 names explicitly).

use anyhow::{Context, Result};
use mjx_opc::{Package, PartName, Relationship, TargetMode};

use super::common::{REL_OFFICE_DOCUMENT, XML_DECLARATION};

/// The number of paragraphs the generated document carries. Each paragraph is `<w:p><w:r><w:rPr/>
/// <w:t>…</w:t></w:r></w:p>` — four elements — so this lands the document at ~80,000 elements,
/// deliberately the same order of magnitude as A7d's 80,004-element slide, so the two are read on
/// one scale.
pub const PARAGRAPH_COUNT: usize = 20_000;

const WORDPROCESSINGML_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const CONTENT_TYPE_DOCUMENT: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";

/// Builds a WordprocessingML package with [`PARAGRAPH_COUNT`] paragraphs.
///
/// # Errors
/// Returns an error if the package cannot be assembled or fails its own validation.
pub fn build_long_document() -> Result<Vec<u8>> {
    let document = PartName::new("/word/document.xml").context("document part name")?;
    let mut package = Package::empty();
    package
        .insert_part(&document, CONTENT_TYPE_DOCUMENT, document_bytes())
        .context("inserting word/document.xml")?;
    package
        .add_relationship(
            None,
            Relationship {
                id: "rId1".to_owned(),
                rel_type: REL_OFFICE_DOCUMENT.to_owned(),
                target: "word/document.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .context("wiring the officeDocument relationship")?;
    package.save().context("saving the generated long document")
}

/// The bytes of `word/document.xml`: [`PARAGRAPH_COUNT`] paragraphs plus the section properties
/// `CT_Body` requires. Built as a plain string, not a [`mjx_ooxml_core::RawElement`] tree — this is
/// the same choice A7d's `mjx248_measure` harness makes for its synthetic slide: assembling a tree
/// of hundreds of thousands of nodes just to serialize it immediately would cost the very memory and
/// time this corpus exists to let a *reader* measure, for no benefit to the generator.
fn document_bytes() -> Vec<u8> {
    let mut xml = String::with_capacity(PARAGRAPH_COUNT * 150 + 256);
    xml.push_str(XML_DECLARATION);
    xml.push_str("<w:document xmlns:w=\"");
    xml.push_str(WORDPROCESSINGML_NAMESPACE);
    xml.push_str("\">\r\n<w:body>\r\n");
    for i in 0..PARAGRAPH_COUNT {
        xml.push_str(&format!(
            "<w:p><w:r><w:rPr/><w:t xml:space=\"preserve\">Paragraph {i} — generated text padding \
             this run to a realistic sentence length for MJXOFF-147's corpus.</w:t></w:r></w:p>\r\n"
        ));
    }
    xml.push_str(
        "<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" \
         w:bottom=\"1440\" w:left=\"1440\" w:header=\"720\" w:footer=\"720\" w:gutter=\"0\"/>\
         </w:sectPr>\r\n",
    );
    xml.push_str("</w:body></w:document>\r\n");
    xml.into_bytes()
}
