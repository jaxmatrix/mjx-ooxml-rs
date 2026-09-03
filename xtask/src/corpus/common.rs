//! Constants shared by the corpus generators (MJXOFF-147).

/// The XML declaration every generated part begins with, matching what the rest of this workspace's
/// templates use (`crates/mjx-pptx/src/blank.rs`).
pub const XML_DECLARATION: &str =
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n";

/// The one relationship type every package root wires to its main part.
pub const REL_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
