//! `Format`, `FormatFamily` and `detectFormat` — what a package *is*, read from its main part.
//!
//! # Why these five are functions rather than properties
//!
//! A `#[wasm_bindgen]` C-like enumeration is a **number** in JavaScript, not an object, so it can
//! carry no methods and no getters: `Format.Presentation` is `0`, and `(0).contentType` is
//! `undefined`. The five things a format knows about itself are therefore free functions taking a
//! format — `formatContentType(Format.Presentation)` — where the Python binding, whose enumerations
//! are real classes, spells them as properties.
//!
//! That is the second of the two shape differences between the bindings (the first is camelCase),
//! and like the first it is forced rather than chosen.
//!
//! Both enumerations are `#[non_exhaustive]` upstream, so the inbound projections are fallible in
//! the same way the open enumerations in [`crate::enums`] are; `tests/node/format.mjs` names every
//! member that exists today and checks the round trip, so the fallback arm is proved unreachable
//! rather than merely believed to be.

use wasm_bindgen::prelude::*;

use mjx_ooxml as ooxml;

use crate::errors::{map_error, unsupported_content};

/// The three markup languages ECMA-376 defines, and which this build can edit.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormatFamily {
    /// PresentationML — PowerPoint. Editable: this is what `Deck` opens.
    Presentation,
    /// WordprocessingML — Word. Detected, not yet editable.
    WordProcessing,
    /// SpreadsheetML — Excel. Detected, not yet editable.
    Spreadsheet,
}

impl FormatFamily {
    /// The model's family as this enumeration's member.
    fn from_model(family: ooxml::FormatFamily) -> Result<Self, JsValue> {
        Ok(match family {
            ooxml::FormatFamily::Presentation => Self::Presentation,
            ooxml::FormatFamily::WordProcessing => Self::WordProcessing,
            ooxml::FormatFamily::Spreadsheet => Self::Spreadsheet,
            _ => {
                return Err(unsupported_content(
                    "this build of the bindings does not project every markup family",
                ))
            }
        })
    }
}

/// What a package's main part says the document is.
///
/// One member per main-part content type, so a template is distinguishable from a presentation and
/// a macro-enabled file from a plain one — which is precisely what a filename check cannot do.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// A PowerPoint presentation (`.pptx`).
    Presentation,
    /// A macro-enabled PowerPoint presentation (`.pptm`).
    PresentationMacroEnabled,
    /// A PowerPoint template (`.potx`).
    PresentationTemplate,
    /// A macro-enabled PowerPoint template (`.potm`).
    PresentationTemplateMacroEnabled,
    /// A PowerPoint slide show (`.ppsx`).
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
    /// A binary Excel workbook (`.xlsb`).
    WorkbookBinary,
    /// An Excel template (`.xltx`).
    WorkbookTemplate,
    /// A macro-enabled Excel template (`.xltm`).
    WorkbookTemplateMacroEnabled,
}

impl Format {
    /// The model's format as this enumeration's member.
    pub(crate) fn from_model(format: ooxml::Format) -> Result<Self, JsValue> {
        Ok(match format {
            ooxml::Format::Presentation => Self::Presentation,
            ooxml::Format::PresentationMacroEnabled => Self::PresentationMacroEnabled,
            ooxml::Format::PresentationTemplate => Self::PresentationTemplate,
            ooxml::Format::PresentationTemplateMacroEnabled => {
                Self::PresentationTemplateMacroEnabled
            }
            ooxml::Format::PresentationSlideshow => Self::PresentationSlideshow,
            ooxml::Format::PresentationSlideshowMacroEnabled => {
                Self::PresentationSlideshowMacroEnabled
            }
            ooxml::Format::Document => Self::Document,
            ooxml::Format::DocumentMacroEnabled => Self::DocumentMacroEnabled,
            ooxml::Format::DocumentTemplate => Self::DocumentTemplate,
            ooxml::Format::DocumentTemplateMacroEnabled => Self::DocumentTemplateMacroEnabled,
            ooxml::Format::Workbook => Self::Workbook,
            ooxml::Format::WorkbookMacroEnabled => Self::WorkbookMacroEnabled,
            ooxml::Format::WorkbookBinary => Self::WorkbookBinary,
            ooxml::Format::WorkbookTemplate => Self::WorkbookTemplate,
            ooxml::Format::WorkbookTemplateMacroEnabled => Self::WorkbookTemplateMacroEnabled,
            _ => {
                return Err(unsupported_content(
                    "this build of the bindings does not project every document format",
                ))
            }
        })
    }

    /// This member as the model's format.
    fn to_model(self) -> ooxml::Format {
        match self {
            Self::Presentation => ooxml::Format::Presentation,
            Self::PresentationMacroEnabled => ooxml::Format::PresentationMacroEnabled,
            Self::PresentationTemplate => ooxml::Format::PresentationTemplate,
            Self::PresentationTemplateMacroEnabled => {
                ooxml::Format::PresentationTemplateMacroEnabled
            }
            Self::PresentationSlideshow => ooxml::Format::PresentationSlideshow,
            Self::PresentationSlideshowMacroEnabled => {
                ooxml::Format::PresentationSlideshowMacroEnabled
            }
            Self::Document => ooxml::Format::Document,
            Self::DocumentMacroEnabled => ooxml::Format::DocumentMacroEnabled,
            Self::DocumentTemplate => ooxml::Format::DocumentTemplate,
            Self::DocumentTemplateMacroEnabled => ooxml::Format::DocumentTemplateMacroEnabled,
            Self::Workbook => ooxml::Format::Workbook,
            Self::WorkbookMacroEnabled => ooxml::Format::WorkbookMacroEnabled,
            Self::WorkbookBinary => ooxml::Format::WorkbookBinary,
            Self::WorkbookTemplate => ooxml::Format::WorkbookTemplate,
            Self::WorkbookTemplateMacroEnabled => ooxml::Format::WorkbookTemplateMacroEnabled,
        }
    }
}

/// What these bytes are, read from the package's main part rather than from a filename.
///
/// Opens the container, follows the root `officeDocument` relationship, and maps that part's
/// content type — the same walk every conforming consumer makes, and the only answer that survives
/// a renamed file.
///
/// Throws an `OoxmlError` with code `Io` if the bytes are not a readable container, and
/// `MalformedDocument` if they are a container with no main part.
#[wasm_bindgen(js_name = "detectFormat")]
pub fn detect_format(data: &[u8]) -> Result<Format, JsValue> {
    Format::from_model(map_error(ooxml::detect_format(data))?)
}

/// The markup language a format belongs to.
#[wasm_bindgen(js_name = "formatFamily")]
pub fn format_family(format: Format) -> Result<FormatFamily, JsValue> {
    FormatFamily::from_model(format.to_model().family())
}

/// Whether a format carries macros (`.pptm`, `.docm`, `.xlsm`, and the template forms).
#[wasm_bindgen(js_name = "formatIsMacroEnabled")]
pub fn format_is_macro_enabled(format: Format) -> bool {
    format.to_model().is_macro_enabled()
}

/// A format's main-part content type, exactly as `[Content_Types].xml` states it.
#[wasm_bindgen(js_name = "formatContentType")]
pub fn format_content_type(format: Format) -> String {
    format.to_model().content_type().to_owned()
}

/// The extension a format conventionally carries — `"pptx"`, `"potm"`, `"xlsb"` — with no leading
/// dot.
#[wasm_bindgen(js_name = "formatConventionalExtension")]
pub fn format_conventional_extension(format: Format) -> String {
    format.to_model().conventional_extension().to_owned()
}

/// Whether `Deck.open` can edit a format. Word and Excel documents are detected before they are
/// editable, so a caller can say so precisely instead of reporting a parse failure.
#[wasm_bindgen(js_name = "formatIsEditable")]
pub fn format_is_editable(format: Format) -> bool {
    format.to_model().is_editable()
}

/// The two-by-two placeholder PNG every `replace…WithPlaceholder` call defaults to.
///
/// A function rather than a constant, because `wasm-bindgen` cannot export a byte constant and a
/// caller who mutated a shared `Uint8Array` would corrupt every later use of it.
#[wasm_bindgen(js_name = "defaultPlaceholderImage")]
pub fn default_placeholder_image() -> Vec<u8> {
    ooxml::DEFAULT_PLACEHOLDER_IMAGE.to_vec()
}
