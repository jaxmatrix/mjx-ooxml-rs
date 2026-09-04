//! WordprocessingML relationship-type and content-type URI constants.
//!
//! These are the *transitional* (Office-emitted) URIs — the ones `tests/fixtures/sample.docx` and
//! every other fixture actually carry — matching the convention `mjx-pptx::constants` already
//! settled on. [`crate::document::PartKind`] is what most callers want; these are its raw strings,
//! kept here (rather than inlined into that match) so a relationship-type typo shows up as a one-line
//! diff instead of being buried in a match arm.
//!
//! Sourced from ECMA-376 Part 1 (5th edition), §11.3 "WordprocessingML Reference Material": each
//! `REL_*`/`CONTENT_TYPE_*` pair below (other than [`REL_MAIL_MERGE_RECIPIENT_DATA`] /
//! [`CONTENT_TYPE_MAIL_MERGE_RECIPIENT_DATA`], see that constant's own doc comment) restates a
//! numbered subclause's stated content type and the relationship type named in its "Relationships"
//! paragraph — §11.3.2 Comments Part, §11.3.3 Document Settings Part, §11.3.4 Endnotes Part, §11.3.5
//! Font Table Part, §11.3.6 Footer Part, §11.3.7 Footnotes Part, §11.3.8 Glossary Document Part,
//! §11.3.9 Header Part, §11.3.10 Main Document Part, §11.3.11 Numbering Definitions Part, §11.3.12
//! Style Definitions Part, §11.3.13 Web Settings Part. The spec states each relationship type in its
//! *Strict* alternate form (`http://purl.oclc.org/ooxml/officeDocument/relationships/...`); every
//! fixture in this workspace is Transitional, so the prefix is substituted for
//! `http://schemas.openxmlformats.org/officeDocument/2006/relationships/...` — confirmed against
//! `tests/fixtures/sample.docx`'s own `word/_rels/document.xml.rels`, which carries exactly this
//! Transitional form for `styles`/`fontTable`/`settings`/`theme`.

/// The relationship type from the package root to the main document part (§11.3.10).
pub const REL_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

/// The relationship type from the main document part to the glossary document part (§11.3.8).
pub const REL_GLOSSARY_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/glossaryDocument";

/// The relationship type from a document part to its style definitions part (§11.3.12).
pub const REL_STYLES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";

/// The relationship type from a document part to its numbering definitions part (§11.3.11).
pub const REL_NUMBERING: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering";

/// The relationship type from a document part to its document settings part (§11.3.3).
pub const REL_SETTINGS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings";

/// The relationship type from a document part to its web settings part (§11.3.13).
pub const REL_WEB_SETTINGS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/webSettings";

/// The relationship type from a document part to its font table part (§11.3.5).
pub const REL_FONT_TABLE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/fontTable";

/// The relationship type from a document part (or a header/footer) to a header part (§11.3.9).
pub const REL_HEADER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/header";

/// The relationship type from a document part (or a header/footer) to a footer part (§11.3.6).
pub const REL_FOOTER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer";

/// The relationship type from a document part to its comments part (§11.3.2).
pub const REL_COMMENTS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments";

/// The relationship type from a document part to its footnotes part (§11.3.7).
pub const REL_FOOTNOTES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes";

/// The relationship type from a document part to its endnotes part (§11.3.4).
pub const REL_ENDNOTES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/endnotes";

/// The relationship type from a document settings part to a Mail Merge Recipient Data part
/// (`w:recipientData@r:id`, ECMA-376 Part 1 §17.14.28) — confirmed directly from Part 1's own
/// worked example (the clause quotes `Type="http://purl.oclc.org/ooxml/officeDocument/relationships/\
/// mailMergeRecipientData"` verbatim). Unlike every other constant in this module, Part 1 never
/// states this part's **content type** in prose (it has no numbered `§11.3.x` "Part" subclause of its
/// own — the Mail Merge Recipient Data part is documented only through the relationship that reaches
/// it). [`CONTENT_TYPE_MAIL_MERGE_RECIPIENT_DATA`] is therefore the one string in this module inferred
/// by pattern from the other twelve confirmed pairs rather than read directly off the page; see its
/// own doc comment.
pub const REL_MAIL_MERGE_RECIPIENT_DATA: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/mailMergeRecipientData";

/// The relationship type from a document part to a Printer Settings part (`w:printerSettings@r:id`,
/// §17.6.14 "printerSettings (Reference to Printer Settings Data)"). Confirmed directly against
/// Part 1's own worked example for that clause, which quotes the Transitional form of this URI
/// verbatim (`Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/\
/// printerSettings"` once substituted for the Strict `purl.oclc.org` form every other constant in
/// this module already substitutes — see this module's own doc comment for why).
pub const REL_PRINTER_SETTINGS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/printerSettings";

/// The content type of a Printer Settings part — a binary part
/// ([`SectionProperties::printer_settings`](crate::SectionProperties)'s target), never XML, so it is
/// registered by extension (`Default Extension="bin"`) rather than by an `Override` on a specific
/// part name in real Office output; kept here anyway as the one string a caller authoring a fresh
/// Printer Settings part (or a test fixture — see `crates/mjx-docx/tests/sections.rs`) needs.
pub const CONTENT_TYPE_PRINTER_SETTINGS: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.printerSettings";

/// The relationship type from a document, header or footer part to a theme part. Shared with
/// `mjx-pptx`'s `REL_THEME` (same URI, same OPC concept — DrawingML, not WordprocessingML — so it is
/// declared again here rather than reached across a sideways crate edge).
pub const REL_THEME: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme";

/// The relationship type from a document, header or footer part to a `w:hyperlink`'s external
/// target — always `TargetMode="External"`. Confirmed against ECMA-376 Part 1 §17.16.22
/// ("hyperlink (Hyperlink)"), whose own worked example quotes this Transitional URI verbatim.
/// Shared with `mjx-pptx`'s `REL_HYPERLINK` (same URI, same OPC concept) — declared again here for
/// the same reason [`REL_THEME`] is.
pub const REL_HYPERLINK: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";

/// The content type of the main document part (§11.3.10).
pub const CONTENT_TYPE_DOCUMENT: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";

/// The content type of the glossary document part (§11.3.8).
pub const CONTENT_TYPE_GLOSSARY_DOCUMENT: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.glossary+xml";

/// The content type of a style definitions part (§11.3.12).
pub const CONTENT_TYPE_STYLES: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml";

/// The content type of a numbering definitions part (§11.3.11).
pub const CONTENT_TYPE_NUMBERING: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml";

/// The content type of a document settings part (§11.3.3).
pub const CONTENT_TYPE_SETTINGS: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml";

/// The content type of a web settings part (§11.3.13).
pub const CONTENT_TYPE_WEB_SETTINGS: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.webSettings+xml";

/// The content type of a font table part (§11.3.5).
pub const CONTENT_TYPE_FONT_TABLE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.fontTable+xml";

/// The content type of a header part (§11.3.9).
pub const CONTENT_TYPE_HEADER: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml";

/// The content type of a footer part (§11.3.6).
pub const CONTENT_TYPE_FOOTER: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml";

/// The content type of a comments part (§11.3.2).
pub const CONTENT_TYPE_COMMENTS: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml";

/// The content type of a footnotes part (§11.3.7).
pub const CONTENT_TYPE_FOOTNOTES: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml";

/// The content type of an endnotes part (§11.3.4).
pub const CONTENT_TYPE_ENDNOTES: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.endnotes+xml";

/// The content type of a Mail Merge Recipient Data part — **inferred, not spec-confirmed**. Every
/// other content type in this module restates ECMA-376 Part 1's own stated string for that part; this
/// one does not exist anywhere in Part 1's prose (see [`REL_MAIL_MERGE_RECIPIENT_DATA`]). It is
/// built from the pattern the other twelve confirmed pairs establish —
/// `application/vnd.openxmlformats-officedocument.wordprocessingml.<relationship-type-suffix>+xml` —
/// which holds exactly for ten of them (`comments`, `settings`→`endnotes`→`fontTable`→`footer`→
/// `footnotes`→`header`→`numbering`→`styles`→`webSettings` all match their own relationship-type
/// suffix character-for-character) and diverges only for the two parts whose relationship type is the
/// generic word `document`/`glossaryDocument` and needed `document.main`/`document.glossary` to stay
/// unambiguous — a pressure `mailMergeRecipientData` does not have. No fixture in this workspace
/// carries this part and nothing here validates it against a schema, so a wrong guess here changes no
/// test's outcome; flagged so a later child sourcing an Office-authored mail-merge document (MJXOFF-130)
/// double-checks it against a real file before relying on it.
pub const CONTENT_TYPE_MAIL_MERGE_RECIPIENT_DATA: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.mailMergeRecipientData+xml";

/// The content type of a theme part. Shared with `mjx-pptx`'s `CONTENT_TYPE_THEME` — see
/// [`REL_THEME`] for why it is declared again rather than reached across a sideways crate edge.
pub const CONTENT_TYPE_THEME: &str = "application/vnd.openxmlformats-officedocument.theme+xml";

/// The relationship type from a document part to an image (media) part (§13.3.1, Table 13-2, the OPC
/// "image" relationship — used identically across every format this workspace models; shared with
/// `mjx-pptx`'s own `REL_IMAGE`, declared again here for the same sideways-edge reason
/// [`REL_THEME`]'s own doc comment gives, since `mjx-docx` cannot depend on `mjx-pptx`).
pub const REL_IMAGE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
