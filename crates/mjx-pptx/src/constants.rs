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

/// The content type of the main presentation part.
pub const CONTENT_TYPE_PRESENTATION: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml";

/// The content type of a slide part.
pub const CONTENT_TYPE_SLIDE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.slide+xml";
