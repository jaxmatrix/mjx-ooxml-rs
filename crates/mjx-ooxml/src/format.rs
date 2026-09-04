//! [`Format`] and [`detect_format`] — what an OOXML file *is*, decided from the package rather than
//! from its name.
//!
//! # Why not the extension
//!
//! A filename is not evidence. `.pptm` and `.potx` are the same PresentationML markup as `.pptx`
//! with a different main-part content type; a `.docx` renamed to `.pptx` is still a Word document;
//! and a byte slice arriving from a browser upload or an HTTP body may carry no name at all. The
//! library does no file I/O, so it could not read one even if it wanted to.
//!
//! So detection reads the package: open it, follow the **root `officeDocument` relationship** to the
//! main part, and map that part's content type. That is the same walk every conforming consumer
//! makes, and it is the only answer that survives a renamed file.
//!
//! # Detection works before editing does
//!
//! Every family is recognized here before [`Deck::open`](crate::Deck::open) or
//! [`Document::open`](crate::Document::open) parses a single element: each refuses a package from
//! the *other* editable family (and Excel, not yet editable at all) with
//! [`ErrorCode::UnsupportedFormat`](crate::ErrorCode::UnsupportedFormat) rather than failing on some
//! Word-shaped or Excel-shaped element it does not recognize. That ordering is deliberate: a caller
//! who hands a `.docx` to `Deck::open` deserves to be told it is a Word document — and pointed at
//! `Document::open` — not that some part failed to parse.

use mjx_pptx::Package;

use crate::error::{Error, ErrorCode};

/// The root relationship every OOXML package carries, naming its main part.
const REL_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

/// The three markup languages ECMA-376 defines, and which this build can edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FormatFamily {
    /// PresentationML — PowerPoint. Editable: this is what [`Deck`](crate::Deck) opens.
    Presentation,
    /// WordprocessingML — Word. Detected, not yet editable.
    WordProcessing,
    /// SpreadsheetML — Excel. Detected, not yet editable.
    Spreadsheet,
}

/// What a package's main part says the document is.
///
/// One variant per main-part content type ECMA-376 and the Microsoft macro-enabled extensions
/// define, so nothing is lost in the classification: a template is distinguishable from a
/// presentation, and a macro-enabled file from a plain one, which is precisely what a filename check
/// cannot do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Format {
    /// A PowerPoint presentation (`.pptx`).
    Presentation,
    /// A macro-enabled PowerPoint presentation (`.pptm`).
    PresentationMacroEnabled,
    /// A PowerPoint template (`.potx`).
    PresentationTemplate,
    /// A macro-enabled PowerPoint template (`.potm`).
    PresentationTemplateMacroEnabled,
    /// A PowerPoint slide show (`.ppsx`) — the same markup, opened straight into presentation mode.
    PresentationSlideshow,
    /// A macro-enabled PowerPoint slide show (`.ppsm`).
    PresentationSlideshowMacroEnabled,
    /// A Word document (`.docx`).
    Document,
    /// A macro-enabled Word document (`.docm`).
    DocumentMacroEnabled,
    /// A Word template (`.dotx`).
    DocumentTemplate,
    /// A macro-enabled Word template (`.dotm`).
    DocumentTemplateMacroEnabled,
    /// An Excel workbook (`.xlsx`).
    Workbook,
    /// A macro-enabled Excel workbook (`.xlsm`).
    WorkbookMacroEnabled,
    /// A binary Excel workbook (`.xlsb`) — an OPC package whose main part is not XML at all.
    WorkbookBinary,
    /// An Excel template (`.xltx`).
    WorkbookTemplate,
    /// A macro-enabled Excel template (`.xltm`).
    WorkbookTemplateMacroEnabled,
}

/// Every main-part content type this build recognizes, paired with the format and the extension it
/// conventionally carries. The single source of truth for the three accessors below.
const CONTENT_TYPES: &[(Format, &str, &str)] = &[
    (
        Format::Presentation,
        "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml",
        "pptx",
    ),
    (
        Format::PresentationMacroEnabled,
        "application/vnd.ms-powerpoint.presentation.macroEnabled.main+xml",
        "pptm",
    ),
    (
        Format::PresentationTemplate,
        "application/vnd.openxmlformats-officedocument.presentationml.template.main+xml",
        "potx",
    ),
    (
        Format::PresentationTemplateMacroEnabled,
        "application/vnd.ms-powerpoint.template.macroEnabled.main+xml",
        "potm",
    ),
    (
        Format::PresentationSlideshow,
        "application/vnd.openxmlformats-officedocument.presentationml.slideshow.main+xml",
        "ppsx",
    ),
    (
        Format::PresentationSlideshowMacroEnabled,
        "application/vnd.ms-powerpoint.slideshow.macroEnabled.main+xml",
        "ppsm",
    ),
    (
        Format::Document,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
        "docx",
    ),
    (
        Format::DocumentMacroEnabled,
        "application/vnd.ms-word.document.macroEnabled.main+xml",
        "docm",
    ),
    (
        Format::DocumentTemplate,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.template.main+xml",
        "dotx",
    ),
    (
        Format::DocumentTemplateMacroEnabled,
        "application/vnd.ms-word.template.macroEnabled.main+xml",
        "dotm",
    ),
    (
        Format::Workbook,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
        "xlsx",
    ),
    (
        Format::WorkbookMacroEnabled,
        "application/vnd.ms-excel.sheet.macroEnabled.main+xml",
        "xlsm",
    ),
    (
        Format::WorkbookBinary,
        "application/vnd.ms-excel.sheet.binary.macroEnabled.main",
        "xlsb",
    ),
    (
        Format::WorkbookTemplate,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.template.main+xml",
        "xltx",
    ),
    (
        Format::WorkbookTemplateMacroEnabled,
        "application/vnd.ms-excel.template.macroEnabled.main+xml",
        "xltm",
    ),
];

impl Format {
    /// Which markup language this format is written in — the axis that decides whether this build
    /// can edit it.
    #[must_use]
    pub fn family(self) -> FormatFamily {
        match self {
            Self::Presentation
            | Self::PresentationMacroEnabled
            | Self::PresentationTemplate
            | Self::PresentationTemplateMacroEnabled
            | Self::PresentationSlideshow
            | Self::PresentationSlideshowMacroEnabled => FormatFamily::Presentation,
            Self::Document
            | Self::DocumentMacroEnabled
            | Self::DocumentTemplate
            | Self::DocumentTemplateMacroEnabled => FormatFamily::WordProcessing,
            Self::Workbook
            | Self::WorkbookMacroEnabled
            | Self::WorkbookBinary
            | Self::WorkbookTemplate
            | Self::WorkbookTemplateMacroEnabled => FormatFamily::Spreadsheet,
        }
    }

    /// Whether the format admits a VBA project — the `macroEnabled` half of every pair. A caller
    /// stripping macros, or refusing to open them, asks this.
    #[must_use]
    pub fn is_macro_enabled(self) -> bool {
        matches!(
            self,
            Self::PresentationMacroEnabled
                | Self::PresentationTemplateMacroEnabled
                | Self::PresentationSlideshowMacroEnabled
                | Self::DocumentMacroEnabled
                | Self::DocumentTemplateMacroEnabled
                | Self::WorkbookMacroEnabled
                | Self::WorkbookBinary
                | Self::WorkbookTemplateMacroEnabled
        )
    }

    /// The content type of the main part that identifies this format — what
    /// [`detect_format`] matched to arrive here.
    #[must_use]
    pub fn content_type(self) -> &'static str {
        CONTENT_TYPES
            .iter()
            .find_map(|(format, content_type, _)| (*format == self).then_some(*content_type))
            .unwrap_or("")
    }

    /// The file extension this format conventionally carries, without the dot — `"pptx"`, `"potm"`,
    /// `"xlsb"`. A *convention*, not evidence: it is what a caller should name a file it writes, and
    /// never what detection reads.
    #[must_use]
    pub fn conventional_extension(self) -> &'static str {
        CONTENT_TYPES
            .iter()
            .find_map(|(format, _, extension)| (*format == self).then_some(*extension))
            .unwrap_or("")
    }

    /// Whether this build can open the format for editing — true for the PresentationML and
    /// WordprocessingML families ([`crate::Deck`] and [`crate::Document`] respectively); Excel is
    /// detected but not yet editable.
    #[must_use]
    pub fn is_editable(self) -> bool {
        matches!(
            self.family(),
            FormatFamily::Presentation | FormatFamily::WordProcessing
        )
    }
}

/// What the OOXML package in `bytes` is, decided from its main part's content type.
///
/// ```no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let bytes = std::fs::read("deck.pptm")?;
/// let format = mjx_ooxml::detect_format(&bytes)?;
/// assert_eq!(format, mjx_ooxml::Format::PresentationMacroEnabled);
/// assert!(format.is_editable());
/// # Ok(())
/// # }
/// ```
///
/// # Errors
/// - [`ErrorCode::Io`] if the bytes are not a readable ZIP container.
/// - [`ErrorCode::MalformedDocument`] if the package has no root `officeDocument` relationship, or
///   its target does not resolve to a part the package holds.
/// - [`ErrorCode::UnsupportedFormat`] if the main part's content type is not one of the fifteen
///   ECMA-376 and macro-enabled types above — an OPC package that is not an Office document.
pub fn detect_format(bytes: &[u8]) -> Result<Format, Error> {
    let package = Package::open(bytes)?;
    format_of(&package)
}

/// The format of an already-open package — the half of [`detect_format`] that
/// [`Deck::open`](crate::Deck::open) reuses so a deck is parsed once, not twice.
pub(crate) fn format_of(package: &Package) -> Result<Format, Error> {
    let main = main_part_content_type(package)?;
    CONTENT_TYPES
        .iter()
        .find_map(|(format, content_type, _)| (*content_type == main).then_some(*format))
        .ok_or_else(|| {
            Error::new(
                ErrorCode::UnsupportedFormat,
                format!("main part content type {main} is not an Office document format"),
            )
        })
}

/// Follows the root `officeDocument` relationship and reports the content type of the part it names.
fn main_part_content_type(package: &Package) -> Result<&str, Error> {
    let root = package.relationships_for(None).ok_or_else(|| {
        Error::new(
            ErrorCode::MalformedDocument,
            "package has no root relationships part (/_rels/.rels)",
        )
    })?;
    let relationship = root.by_type(REL_OFFICE_DOCUMENT).next().ok_or_else(|| {
        Error::new(
            ErrorCode::MalformedDocument,
            "package has no officeDocument relationship",
        )
    })?;
    let part = mjx_pptx::PartName::resolve_from_root(&relationship.target)?;
    package.content_type_of(&part).ok_or_else(|| {
        Error::new(
            ErrorCode::MalformedDocument,
            format!(
                "the officeDocument relationship names {}, which the package does not hold",
                part.as_str()
            ),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every format must round-trip through its content type, or a lookup silently answers `""`.
    #[test]
    fn every_format_has_a_content_type_and_an_extension() {
        for (format, content_type, extension) in CONTENT_TYPES {
            assert_eq!(format.content_type(), *content_type);
            assert_eq!(format.conventional_extension(), *extension);
            assert!(!extension.is_empty());
        }
    }

    /// The table is a lookup key: two formats sharing a content type would make detection ambiguous.
    #[test]
    fn content_types_are_unique() {
        for (index, (_, content_type, _)) in CONTENT_TYPES.iter().enumerate() {
            assert!(
                !CONTENT_TYPES[index + 1..]
                    .iter()
                    .any(|(_, other, _)| other == content_type),
                "{content_type} appears twice"
            );
        }
    }

    #[test]
    fn presentationml_and_wordprocessingml_are_editable_spreadsheetml_is_not() {
        for (format, _, _) in CONTENT_TYPES {
            assert_eq!(
                format.is_editable(),
                matches!(
                    format.family(),
                    FormatFamily::Presentation | FormatFamily::WordProcessing
                )
            );
        }
        assert!(!Format::Workbook.is_editable());
    }
}
