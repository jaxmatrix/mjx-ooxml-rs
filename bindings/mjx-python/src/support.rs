//! The three things every value class in this crate needs, in one place.
//!
//! * [`value_class!`] declares the `#[pyclass]` newtype and its two conversions. Every wrapper here
//!   is `NewType(mjx_ooxml::Thing)` — one field, no state of its own — so the declaration is the
//!   only part worth sharing; the methods are written out for each class, because that is where the
//!   design lives.
//! * [`RangeArg`] turns Python's own `range` into the `Range<u32>` the model takes.
//! * [`bytes_or_none`] and friends spell the few conversions that would otherwise be repeated
//!   two hundred times in `deck/`.

use pyo3::prelude::*;
use pyo3::types::PyRange;
use pyo3::{Borrowed, PyErr};

/// Declares a value class: a frozen `#[pyclass]` newtype over a `mjx_ooxml` value, plus the
/// conversions in both directions.
///
/// `frozen` is not incidental. These classes are values — a `FillSpec` is a description of a fill,
/// not a handle to one — and Python code that mutates a value it passed in and expects the document
/// to change is a bug this makes impossible to write. The builders return new instances, exactly as
/// the Rust builders do.
macro_rules! value_class {
    ($(
        $(#[$attribute:meta])*
        $name:ident($model:path) $(, derive($($extra:ident),+ $(,)?))?;
    )*) => {
        $(
            $(#[$attribute])*
            #[pyclass(frozen, from_py_object, module = "mjx_ooxml")]
            #[derive(Debug, Clone $($(, $extra)+)?)]
            pub struct $name(pub(crate) $model);

            impl From<$model> for $name {
                fn from(value: $model) -> Self {
                    Self(value)
                }
            }

            impl From<$name> for $model {
                fn from(value: $name) -> Self {
                    value.0
                }
            }
        )*
    };
}

/// A half-open index range, taken as Python's own `range`.
///
/// `deck.set_text_range_properties(0, 1, 0, range(4, 9), spec)` reads the way the same call reads in
/// Rust, and a `range` carries its own bounds check. A step other than `1` is refused rather than
/// silently ignored: `range(0, 10, 2)` does not describe a run of text.
pub(crate) struct RangeArg(pub(crate) core::ops::Range<u32>);

impl<'a, 'py> FromPyObject<'a, 'py> for RangeArg {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> Result<Self, PyErr> {
        let range = object.cast::<PyRange>().map_err(|_| {
            pyo3::exceptions::PyTypeError::new_err("a text range is a `range`, such as range(0, 5)")
        })?;
        let step: i64 = range.getattr("step")?.extract()?;
        if step != 1 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "a text range must have step 1: a range that skips characters does not describe a run",
            ));
        }
        let start: i64 = range.getattr("start")?.extract()?;
        let stop: i64 = range.getattr("stop")?.extract()?;
        if start < 0 || stop < 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "a text range cannot start or end before zero",
            ));
        }
        if stop < start {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "a text range cannot end before it starts",
            ));
        }
        let narrow = |value: i64| -> Result<u32, PyErr> {
            u32::try_from(value).map_err(|_| {
                pyo3::exceptions::PyValueError::new_err(
                    "a text range bound is larger than any document holds",
                )
            })
        };
        Ok(Self(narrow(start)?..narrow(stop)?))
    }
}

/// A list of borrowed strings, as the model's `&[&str]` argument shape.
///
/// The owned `Vec<String>` PyO3 hands back has to outlive the call, so the two-step is spelled here
/// once rather than at each of the four call sites.
pub(crate) fn as_str_slice(owned: &[String]) -> Vec<&str> {
    owned.iter().map(String::as_str).collect()
}
