//! PresentationML relationship-type and content-type URI constants.
//!
//! These are the *transitional* (Office-emitted) URIs, which the fixtures use. Relationship lookup
//! (`Relationships::by_type`) matches the exact string.

/// The relationship type from the package root to the main presentation part.
pub const REL_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

/// The relationship type from the presentation part to a slide part.
pub const REL_SLIDE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";

/// The relationship type from a slide part to its slide layout.
pub const REL_SLIDE_LAYOUT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout";

/// The relationship type from a slide layout to its slide master.
pub const REL_SLIDE_MASTER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster";

/// The relationship type from a slide master (or the presentation) to its theme.
pub const REL_THEME: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme";

/// The relationship type from a part to an external hyperlink target (a URL). Always
/// `TargetMode="External"`.
pub const REL_HYPERLINK: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";

/// The relationship type from a slide to its notes slide.
pub const REL_NOTES_SLIDE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide";

/// The relationship type from the presentation (or a notes slide) to the notes master.
pub const REL_NOTES_MASTER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesMaster";

/// The relationship type from a part (e.g. a slide) to an embedded image part.
pub const REL_IMAGE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";

/// The relationship type from the presentation part to its `tableStyles.xml`.
pub const REL_TABLE_STYLES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/tableStyles";

/// The content type of the main presentation part.
pub const CONTENT_TYPE_PRESENTATION: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml";

/// The content type of a slide part.
pub const CONTENT_TYPE_SLIDE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.slide+xml";

/// The content type of a theme part.
pub const CONTENT_TYPE_THEME: &str = "application/vnd.openxmlformats-officedocument.theme+xml";

/// The content type of a notes slide part.
pub const CONTENT_TYPE_NOTES_SLIDE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml";

/// The content type of a notes master part.
pub const CONTENT_TYPE_NOTES_MASTER: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.notesMaster+xml";

/// The content type of the `tableStyles.xml` part. Shares the `xml` extension with every other part,
/// so it is registered as a per-part Override, not a Default.
pub const CONTENT_TYPE_TABLE_STYLES: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml";
