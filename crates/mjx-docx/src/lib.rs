//! `mjx-docx` — WordprocessingML: document body, styles, numbering, sections, headers/footers.
//!
//! The entry point is [`Document`]: open a `.docx`'s container bytes with [`Document::open`], read
//! its part graph with [`Document::parts`], and save with [`Document::save`]. It owns an
//! [`mjx_opc::Package`] and resolves the part graph `tests/fixtures/sample.docx` and a richer
//! document alike carry; everything it does not yet model is preserved verbatim by the OPC
//! copy-on-write layer.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let bytes = std::fs::read("document.docx")?;
//! let mut document = mjx_docx::Document::open(&bytes)?;
//! println!("{:?}", document.conformance()?);
//! let saved = document.save()?;
//! # let _ = saved;
//! # Ok(())
//! # }
//! ```
//!
//! See `crates/mjx-docx/src/document/mod.rs`'s own doc comment for the module layout this crate is
//! built on and the plan for the files later children add.

mod address;
pub mod constants;
mod document;
mod error;

pub use address::{BlockPath, RunPath};
pub use document::{
    Background, BlockContent, Body, Border, Break, CharacterStyle, Color, Document, DocumentParts,
    EastAsianLayout, Emphasis, Fonts, HalfPoint, HalfPointMeasureValue, HexColor, Highlight,
    Hyperlink, Lang, Languages, MainDocument, MainDocumentContent, ManualRunWidth, Paragraph,
    ParagraphContent, PartKind, PermissionRangeEnd, PermissionRangeStart, PhoneticGuide,
    PhoneticGuideChild, PhoneticGuideContent, PhoneticGuideContentItem, PhoneticGuideProperties,
    PhoneticGuidePropertyContent, PhoneticGuideTextAlignment, PositionalTab, ProofingError,
    RelationshipReference, Run, RunInnerContent, RunProperties, RunPropertyContent, Scale, Shading,
    SignedHalfPoint, SignedHalfPointMeasureValue, SignedTwips, SignedTwipsMeasureValue, Symbol,
    Text, TextEffect, TextScaleValue, ThemeHexDigit, Toggle, Twips, Underline, Unmodeled,
    VerticalAlignment,
};
pub use error::DocxError;
// The OPC vocabulary a caller of this crate's own signatures must be able to name: the package
// `Document::from_package` takes, and the part names `DocumentParts` hands back. Re-exported so
// nothing downstream has to depend on `mjx-opc` to state a parameter type this crate chose — the
// same reasoning `mjx_pptx` documents for its own re-export of the same vocabulary.
pub use mjx_opc::{OpcError, Package, PartName, TargetMode};
