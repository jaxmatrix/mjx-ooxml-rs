//! `Surface` and `ShapePath` — *which part* and *which shape* — and the two hand-written
//! conversions that make a bare `int` mean what it means in Rust.
//!
//! In Rust, `deck.shape_fill(slide.into(), 2.into())` works because `Surface` and `ShapePath` both
//! implement `From<u32>`. PyO3 gives that for free for nothing: a `#[pyclass]` argument accepts an
//! instance of that class and nothing else. So this module writes the two [`FromPyObject`]
//! implementations by hand — `SurfaceArg` and `ShapePathArg` — and every `Deck` method takes
//! those rather than the classes. The result is the Rust ergonomics exactly:
//!
//! ```python
//! deck.shape_fill(0, 2)                                  # slide 0, top-level shape 2
//! deck.shape_fill(Surface.layout(1), [2, 1])             # layout 1, member 1 of group 2
//! deck.shape_fill(Surface.notes_master(), ShapePath.of([0]))
//! ```
//!
//! Both classes are immutable and hashable, so they work as dictionary keys and in sets — which is
//! how a caller keeps a map from an address to whatever it is tracking.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use pyo3::prelude::*;
use pyo3::types::{PyModule, PySequence, PyString};
use pyo3::{Borrowed, PyErr};

use mjx_ooxml as ooxml;

/// The shape-bearing part a call is about.
///
/// A bare integer means a slide wherever a surface is expected, so `deck.shape_count(0)` and
/// `deck.shape_count(Surface.slide(0))` are the same call. The other four kinds have to be named.
#[pyclass(frozen, from_py_object, module = "mjx_ooxml")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Surface(pub(crate) ooxml::Surface);

#[pymethods]
impl Surface {
    /// The slide at this index, counting from zero.
    #[staticmethod]
    fn slide(index: u32) -> Self {
        Self(ooxml::Surface::Slide(index))
    }

    /// The slide layout at this index — one flat space across every master.
    #[staticmethod]
    fn layout(index: u32) -> Self {
        Self(ooxml::Surface::Layout(index))
    }

    /// The slide master at this index.
    #[staticmethod]
    fn master(index: u32) -> Self {
        Self(ooxml::Surface::Master(index))
    }

    /// The notes slide belonging to the slide at this index.
    #[staticmethod]
    fn notes(slide_index: u32) -> Self {
        Self(ooxml::Surface::Notes(slide_index))
    }

    /// The single notes master every notes slide inherits from.
    #[staticmethod]
    fn notes_master() -> Self {
        Self(ooxml::Surface::NotesMaster)
    }

    /// The index within this surface's own kind. The notes master is unique and reports `0`.
    #[getter]
    fn index(&self) -> u32 {
        self.0.index()
    }

    /// The kind's name: `"slide"`, `"layout"`, `"master"`, `"notes"` or `"notes master"`.
    #[getter]
    fn kind(&self) -> &'static str {
        self.0.kind_name()
    }

    /// Whether this stands at the head of its own inheritance chain — a slide master or the notes
    /// master, neither of which inherits from a further part.
    #[getter]
    fn is_master_like(&self) -> bool {
        self.0.is_master_like()
    }

    fn __repr__(&self) -> String {
        match self.0 {
            ooxml::Surface::Slide(index) => format!("Surface.slide({index})"),
            ooxml::Surface::Layout(index) => format!("Surface.layout({index})"),
            ooxml::Surface::Master(index) => format!("Surface.master({index})"),
            ooxml::Surface::Notes(index) => format!("Surface.notes({index})"),
            ooxml::Surface::NotesMaster => "Surface.notes_master()".to_owned(),
        }
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        hasher.finish()
    }
}

/// The address of a shape within a surface's shape tree.
///
/// A bare integer means a top-level shape wherever a path is expected, and a list of integers means
/// a descent through nested groups, so `deck.shape_kind(0, 2)` and `deck.shape_kind(0, [2, 1])`
/// both work without naming this class.
#[pyclass(frozen, from_py_object, module = "mjx_ooxml")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShapePath(pub(crate) ooxml::ShapePath);

#[pymethods]
impl ShapePath {
    /// The top-level shape at this index.
    #[staticmethod]
    fn top(index: u32) -> Self {
        Self(ooxml::ShapePath::from(index))
    }

    /// The shape at this address: `[2]` top-level, `[2, 1]` for member 1 of the group at index 2.
    #[staticmethod]
    fn of(indices: Vec<u32>) -> Self {
        Self(ooxml::ShapePath::from(indices))
    }

    /// The address as a list of indices, outermost first.
    #[getter]
    fn indices(&self) -> Vec<u32> {
        self.0.indices().to_vec()
    }

    /// How deep the address reaches: `1` for a top-level shape, `2` for a member of a top-level
    /// group, and so on.
    #[getter]
    fn depth(&self) -> u32 {
        self.0.depth()
    }

    /// Whether this addresses a top-level shape — a single index, no group descent.
    #[getter]
    fn is_top_level(&self) -> bool {
        self.0.is_top_level()
    }

    /// The address of member `index` of the group this addresses — one step deeper.
    fn child(&self, index: u32) -> Self {
        Self(self.0.child(index))
    }

    /// The address of the group this shape belongs to, or `None` for a top-level shape.
    #[getter]
    fn parent(&self) -> Option<Self> {
        self.0.parent().map(Self)
    }

    fn __repr__(&self) -> String {
        format!("ShapePath.of({:?})", self.0.indices())
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        hasher.finish()
    }
}

/// A surface argument: an `int` (meaning a slide) or a [`Surface`].
///
/// This is the hand-written half of the ergonomics — the reason `deck.shape_count(0)` is legal.
pub(crate) struct SurfaceArg(pub(crate) ooxml::Surface);

impl<'a, 'py> FromPyObject<'a, 'py> for SurfaceArg {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> Result<Self, PyErr> {
        // A `bool` is an `int` in Python, and `deck.shape_count(True)` is never what anyone meant.
        if object.is_instance_of::<pyo3::types::PyBool>() {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "a surface is an int (the slide index) or a Surface, not a bool",
            ));
        }
        if let Ok(index) = object.extract::<u32>() {
            return Ok(Self(ooxml::Surface::Slide(index)));
        }
        match object.extract::<Surface>() {
            Ok(surface) => Ok(Self(surface.0)),
            Err(_) => Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "a surface is an int (the slide index) or a Surface, not {}",
                type_name(&object)
            ))),
        }
    }
}

/// A shape argument: an `int` (a top-level shape), a sequence of `int` (a descent through groups),
/// or a [`ShapePath`].
pub(crate) struct ShapePathArg(pub(crate) ooxml::ShapePath);

impl<'a, 'py> FromPyObject<'a, 'py> for ShapePathArg {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> Result<Self, PyErr> {
        if object.is_instance_of::<pyo3::types::PyBool>() {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "a shape address is an int, a sequence of int, or a ShapePath, not a bool",
            ));
        }
        if let Ok(index) = object.extract::<u32>() {
            return Ok(Self(ooxml::ShapePath::from(index)));
        }
        if let Ok(path) = object.extract::<ShapePath>() {
            return Ok(Self(path.0));
        }
        // A `str` is a sequence, and extracting one as `Vec<u32>` would fail per element with a
        // message about the element rather than about the argument. Refuse it up front.
        if object.is_instance_of::<PyString>() {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "a shape address is an int, a sequence of int, or a ShapePath, not a str",
            ));
        }
        if object.cast::<PySequence>().is_ok() {
            if let Ok(indices) = object.extract::<Vec<u32>>() {
                if indices.is_empty() {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "a shape address needs at least one index; the shape tree is not itself a shape",
                    ));
                }
                return Ok(Self(ooxml::ShapePath::from(indices)));
            }
        }
        Err(pyo3::exceptions::PyTypeError::new_err(format!(
            "a shape address is an int, a sequence of int, or a ShapePath, not {}",
            type_name(&object)
        )))
    }
}

/// The name of an argument's type, for a message that says what was passed.
fn type_name(object: &Borrowed<'_, '_, PyAny>) -> String {
    object
        .get_type()
        .name()
        .map(|name| name.to_string())
        .unwrap_or_else(|_| "an object of unknown type".to_owned())
}

/// The facade's surface as the class a caller receives back.
pub(crate) fn surface_out(surface: ooxml::Surface) -> Surface {
    Surface(surface)
}

/// The facade's path as the class a caller receives back.
pub(crate) fn path_out(path: ooxml::ShapePath) -> ShapePath {
    ShapePath(path)
}

/// Adds both classes to the extension module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Surface>()?;
    module.add_class::<ShapePath>()
}
