//! `Format`, `FormatFamily` and `detect_format` — what a package *is*, read from its main part.
//!
//! Both enumerations are `#[non_exhaustive]` upstream, so the inbound projections are fallible in
//! the same way the open enumerations in [`crate::enums`] are; `tests/test_format.py` names every
//! member that exists today and checks the round trip, so the fallback arm is proved unreachable
//! rather than merely believed to be.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule};

use mjx_ooxml as ooxml;

use crate::errors::{to_py_err, unsupported_content};

/// The three markup languages ECMA-376 defines, and which this build can edit.
#[pyclass(eq, eq_int, frozen, from_py_object, module = "mjx_ooxml")]
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
    /// The model's family as this class's member.
    fn from_model(family: ooxml::FormatFamily) -> PyResult<Self> {
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
#[pyclass(eq, eq_int, frozen, from_py_object, module = "mjx_ooxml")]
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
    /// The model's format as this class's member.
    pub(crate) fn from_model(format: ooxml::Format) -> PyResult<Self> {
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

#[pymethods]
impl Format {
    /// The markup language this format belongs to.
    #[getter]
    fn family(&self) -> PyResult<FormatFamily> {
        FormatFamily::from_model(self.to_model().family())
    }

    /// Whether this format carries macros (`.pptm`, `.docm`, `.xlsm`, and the template forms).
    #[getter]
    fn is_macro_enabled(&self) -> bool {
        self.to_model().is_macro_enabled()
    }

    /// The main part's content type, exactly as `[Content_Types].xml` states it.
    #[getter]
    fn content_type(&self) -> &'static str {
        self.to_model().content_type()
    }

    /// The extension this format conventionally carries — `"pptx"`, `"potm"`, `"xlsb"` — with no
    /// leading dot.
    #[getter]
    fn conventional_extension(&self) -> &'static str {
        self.to_model().conventional_extension()
    }

    /// Whether `Deck.open` can edit this format. Word and Excel documents are detected before they
    /// are editable, so a caller can say so precisely instead of reporting a parse failure.
    #[getter]
    fn is_editable(&self) -> bool {
        self.to_model().is_editable()
    }
}

/// What these bytes are, read from the package's main part rather than from a filename.
///
/// Opens the container, follows the root `officeDocument` relationship, and maps that part's
/// content type — the same walk every conforming consumer makes, and the only answer that survives
/// a renamed file.
///
/// Raises `IoError` if the bytes are not a readable container, and `MalformedDocumentError` if they
/// are a container with no main part.
#[pyfunction]
#[pyo3(name = "detect_format")]
pub(crate) fn detect_format(python: Python<'_>, data: &[u8]) -> PyResult<Format> {
    let format = python
        .detach(|| ooxml::detect_format(data))
        .map_err(to_py_err)?;
    Format::from_model(format)
}

/// Adds the two enumerations and `detect_format` to the extension module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Format>()?;
    module.add_class::<FormatFamily>()?;
    module.add_function(wrap_pyfunction!(detect_format, module)?)?;
    // The two-by-two placeholder PNG every `replace_*_with_placeholder` call defaults to.
    module.add(
        "DEFAULT_PLACEHOLDER_IMAGE",
        PyBytes::new(module.py(), ooxml::DEFAULT_PLACEHOLDER_IMAGE),
    )
}
