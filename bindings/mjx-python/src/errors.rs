//! The exception hierarchy: one root, eleven subclasses, one per [`mjx_ooxml::ErrorCode`].
//!
//! ```python
//! try:
//!     deck.shape_text(0, 99)
//! except mjx_ooxml.IndexOutOfRangeError as failure:
//!     print(failure.code)     # "IndexOutOfRange"
//!     print(failure.surface)  # Surface.slide(0)
//!     print(failure.shape)    # ShapePath.of([99])
//! ```
//!
//! Three things make this worth eleven classes rather than one:
//!
//! * **`except` selects.** A caller who wants to retry a different address catches
//!   `IndexOutOfRangeError`; a caller who wants to give up on the file catches
//!   `MalformedDocumentError`. With one class both have to read `.code` and re-raise.
//! * **`IndexOutOfRangeError` is also a Python `IndexError`.** Code that already guards a lookup
//!   with `except IndexError` keeps working when the lookup is a slide index. It is the one place a
//!   second base class earns its keep, and it is built with `type()` because
//!   `PyErr::new_type` takes exactly one base.
//! * **The coordinates are attributes, not prose.** `.surface`, `.shape`, `.row`, `.column` and
//!   `.index` are the same values the caller passed in, in the same classes, so a handler can act on
//!   them instead of parsing the message.
//!
//! Every attribute is present on every instance — `None` where the failure carried no such
//! coordinate — so `failure.row` never raises `AttributeError`.

use pyo3::exceptions::{PyException, PyIndexError};
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyModule, PyTuple, PyType};
use pyo3::PyErr;

use mjx_ooxml as ooxml;

use crate::address::{path_out, surface_out};

/// The eleven concrete classes plus their root, created once per interpreter.
struct Exceptions {
    root: Py<PyType>,
    io: Py<PyType>,
    malformed_document: Py<PyType>,
    invalid_document: Py<PyType>,
    index_out_of_range: Py<PyType>,
    wrong_kind: Py<PyType>,
    not_found: Py<PyType>,
    nothing_to_read: Py<PyType>,
    invalid_argument: Py<PyType>,
    structure_conflict: Py<PyType>,
    unsupported_content: Py<PyType>,
    unsupported_format: Py<PyType>,
}

static EXCEPTIONS: PyOnceLock<Exceptions> = PyOnceLock::new();

/// Builds a subclass of `OoxmlError` with the given qualified name.
///
/// `dict` is deliberately `None`: the classes carry no class-level state, and their instance
/// attributes are set when an instance is raised.
fn subclass(
    py: Python<'_>,
    qualified_name: &std::ffi::CStr,
    documentation: &std::ffi::CStr,
    root: &Bound<'_, PyType>,
) -> PyResult<Py<PyType>> {
    PyErr::new_type(py, qualified_name, Some(documentation), Some(root), None)
}

/// The hierarchy, created on first use and thereafter reused.
fn exceptions(py: Python<'_>) -> PyResult<&Exceptions> {
    EXCEPTIONS.get_or_try_init(py, || {
        let root = PyErr::new_type(
            py,
            c"mjx_ooxml.OoxmlError",
            Some(c"Every failure this library reports. Carries `code` and the coordinates `surface`, `shape`, `row`, `column` and `index`."),
            Some(&py.get_type::<PyException>()),
            None,
        )?;
        let root_bound = root.bind(py).clone();

        // `IndexOutOfRangeError` alone has two bases, so it cannot come from `PyErr::new_type`.
        // Python's own `type(name, bases, namespace)` is the only constructor that takes several.
        let bases = PyTuple::new(
            py,
            [
                root_bound.clone().into_any(),
                py.get_type::<PyIndexError>().into_any(),
            ],
        )?;
        let namespace = pyo3::types::PyDict::new(py);
        namespace.set_item("__module__", "mjx_ooxml")?;
        namespace.set_item(
            "__doc__",
            "An index or range argument is outside what the document holds. Also an `IndexError`, so `except IndexError` catches it.",
        )?;
        let index_out_of_range = py
            .get_type::<PyType>()
            .call1(("IndexOutOfRangeError", bases, namespace))?
            .cast_into::<PyType>()
            .map_err(|_| {
                pyo3::exceptions::PyRuntimeError::new_err("type() did not return a class")
            })?
            .unbind();

        Ok::<_, PyErr>(Exceptions {
            io: subclass(py, c"mjx_ooxml.IoError", c"The container bytes could not be read or written. Nothing about the document was learned.", &root_bound)?,
            malformed_document: subclass(py, c"mjx_ooxml.MalformedDocumentError", c"The bytes are a package, but its markup is not what the schema requires.", &root_bound)?,
            invalid_document: subclass(py, c"mjx_ooxml.InvalidDocumentError", c"The document in memory breaks an invariant, so writing it was refused.", &root_bound)?,
            index_out_of_range,
            wrong_kind: subclass(py, c"mjx_ooxml.WrongKindError", c"The thing at that address is of a kind that cannot answer the call.", &root_bound)?,
            not_found: subclass(py, c"mjx_ooxml.NotFoundError", c"A name or identifier resolved to nothing.", &root_bound)?,
            nothing_to_read: subclass(py, c"mjx_ooxml.NothingToReadError", c"The target exists and is of the right kind, but states nothing for this call.", &root_bound)?,
            invalid_argument: subclass(py, c"mjx_ooxml.InvalidArgumentError", c"An argument is refused before anything is written.", &root_bound)?,
            structure_conflict: subclass(py, c"mjx_ooxml.StructureConflictError", c"The edit conflicts with the structure the document already has.", &root_bound)?,
            unsupported_content: subclass(py, c"mjx_ooxml.UnsupportedContentError", c"The document uses a construct this build does not model, or asks for one it cannot write.", &root_bound)?,
            unsupported_format: subclass(py, c"mjx_ooxml.UnsupportedFormatError", c"The file is a valid Office document of a format this build cannot open yet.", &root_bound)?,
            root,
        })
    })
}

/// The class a given code is raised as.
fn class_for<'py>(py: Python<'py>, code: ooxml::ErrorCode) -> PyResult<&'py Py<PyType>> {
    let classes = exceptions(py)?;
    Ok(match code {
        ooxml::ErrorCode::Io => &classes.io,
        ooxml::ErrorCode::MalformedDocument => &classes.malformed_document,
        ooxml::ErrorCode::InvalidDocument => &classes.invalid_document,
        ooxml::ErrorCode::IndexOutOfRange => &classes.index_out_of_range,
        ooxml::ErrorCode::WrongKind => &classes.wrong_kind,
        ooxml::ErrorCode::NotFound => &classes.not_found,
        ooxml::ErrorCode::NothingToRead => &classes.nothing_to_read,
        ooxml::ErrorCode::InvalidArgument => &classes.invalid_argument,
        ooxml::ErrorCode::StructureConflict => &classes.structure_conflict,
        ooxml::ErrorCode::UnsupportedContent => &classes.unsupported_content,
        ooxml::ErrorCode::UnsupportedFormat => &classes.unsupported_format,
        // `ErrorCode` is `#[non_exhaustive]`: a code this build does not know is still a failure,
        // and reporting it as the root class is strictly better than losing it.
        _ => &classes.root,
    })
}

/// Raises `error` as the Python exception its code selects, with its coordinates attached.
///
/// # Panics
/// Never. Every step that can fail — building the class, instantiating it, setting an attribute —
/// falls back to whatever exception that step produced, so this always returns a `PyErr` rather
/// than unwinding. That matters: a panic crossing the PyO3 boundary is a process-level abort in
/// anything that turns it into one, and this is the function every fallible method funnels through.
pub(crate) fn to_py_err(error: ooxml::Error) -> PyErr {
    Python::attach(|py| match build(py, &error) {
        Ok(instance) => PyErr::from_value(instance),
        Err(failure) => failure,
    })
}

/// Instantiates the exception and attaches the coordinates.
fn build<'py>(py: Python<'py>, error: &ooxml::Error) -> PyResult<Bound<'py, PyAny>> {
    let class = class_for(py, error.code())?.bind(py).clone();
    let instance = class.call1((error.message().to_owned(),))?;
    let detail = error.detail();
    instance.setattr("code", error.code().as_str())?;
    instance.setattr("surface", detail.surface.map(surface_out))?;
    instance.setattr("shape", detail.shape.clone().map(path_out))?;
    instance.setattr("row", detail.row)?;
    instance.setattr("column", detail.column)?;
    instance.setattr("index", detail.index)?;
    Ok(instance)
}

/// An `UnsupportedContentError` the bindings raise themselves, for content the model can express
/// and this build cannot project.
pub(crate) fn unsupported_content(message: &str) -> PyErr {
    Python::attach(|py| match raise_bare(py, message) {
        Ok(instance) => PyErr::from_value(instance),
        Err(failure) => failure,
    })
}

/// Instantiates an `UnsupportedContentError` with empty coordinates.
fn raise_bare<'py>(py: Python<'py>, message: &str) -> PyResult<Bound<'py, PyAny>> {
    let class = exceptions(py)?.unsupported_content.bind(py).clone();
    let instance = class.call1((message.to_owned(),))?;
    instance.setattr("code", ooxml::ErrorCode::UnsupportedContent.as_str())?;
    for empty in ["surface", "shape", "row", "column", "index"] {
        instance.setattr(empty, py.None())?;
    }
    Ok(instance)
}

/// Adds the twelve classes to the extension module under their Python names.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = module.py();
    let classes = exceptions(py)?;
    for (name, class) in [
        ("OoxmlError", &classes.root),
        ("IoError", &classes.io),
        ("MalformedDocumentError", &classes.malformed_document),
        ("InvalidDocumentError", &classes.invalid_document),
        ("IndexOutOfRangeError", &classes.index_out_of_range),
        ("WrongKindError", &classes.wrong_kind),
        ("NotFoundError", &classes.not_found),
        ("NothingToReadError", &classes.nothing_to_read),
        ("InvalidArgumentError", &classes.invalid_argument),
        ("StructureConflictError", &classes.structure_conflict),
        ("UnsupportedContentError", &classes.unsupported_content),
        ("UnsupportedFormatError", &classes.unsupported_format),
    ] {
        module.add(name, class.bind(py).clone())?;
    }
    Ok(())
}
