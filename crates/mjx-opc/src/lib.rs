//! `mjx-opc` — the Open Packaging Conventions (OPC) layer for mjx-ooxml-rs.
//!
//! An OOXML file (`.pptx` / `.docx` / `.xlsx`) is a ZIP container of *parts* described by two kinds
//! of control stream: `[Content_Types].xml` (part → content-type) and `_rels/*.rels` (typed links
//! between parts). This crate models that container generically, with no knowledge of
//! WordprocessingML / SpreadsheetML / PresentationML.
//!
//! # Fidelity
//!
//! Every ZIP entry is retained verbatim, in order (see [`Package`]). Untouched entries re-serialize
//! to decompressed-byte-identical output. The round-trip contract is **per-part decompressed-payload
//! byte identity + structural container identity**, not identical ZIP bytes (deflate encodings vary).
//!
//! # Creating a package
//!
//! [`Package::empty`] builds a valid, empty container — `[Content_Types].xml` with the two `Default`
//! rules every package needs, plus an empty package-root `_rels/.rels`. It is format-agnostic: a
//! format layer turns it into a document by inserting parts and wiring relationships (see
//! `mjx_pptx::Presentation::blank`).
//!
//! # Example
//!
//! ```no_run
//! # fn main() -> Result<(), mjx_opc::OpcError> {
//! let bytes = std::fs::read("deck.pptx")?;
//! let pkg = mjx_opc::Package::open(&bytes)?;
//! for part in pkg.part_names() {
//!     if let Some(ct) = pkg.content_type_of(&part) {
//!         println!("{} -> {ct}", part.as_str());
//!     }
//! }
//! let re_saved = pkg.save()?;
//! # let _ = re_saved;
//! # Ok(())
//! # }
//! ```

mod content_types;
mod error;
mod media;
mod name;
mod package;
mod rels;
mod validate;

pub use content_types::{
    ContentTypes, Default, Override, CONTENT_TYPES_ZIP_NAME, CONTENT_TYPE_RELATIONSHIPS,
    CONTENT_TYPE_XML,
};
pub use error::OpcError;
pub use media::ImageFormat;
pub use name::PartName;
pub use package::{Package, PartBody, PartProvenance, ZipEntry};
pub use rels::{ExternalRelationship, Relationship, Relationships, RelationshipsPart, TargetMode};
pub use validate::PackageDefect;
