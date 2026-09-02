//! Failures, as real JavaScript `Error` objects.
//!
//! ```js
//! try {
//!   deck.shapeText(0, 99);
//! } catch (failure) {
//!   failure instanceof Error;        // true
//!   failure.name;                    // "OoxmlError"
//!   failure.code;                    // "IndexOutOfRange"
//!   failure.detail.surface.kind;     // "slide"
//!   failure.detail.shape.indices;    // [99]
//! }
//! ```
//!
//! # Why an `Error` and not a class of our own
//!
//! A `#[wasm_bindgen]` struct thrown as an exception is not an `Error`: `instanceof Error` is
//! false, it has no `stack`, and every logger, test runner and error reporter that special-cases
//! `Error` treats it as an opaque object. So a failure is a real [`js_sys::Error`] with
//! `name = "OoxmlError"` and two own properties added:
//!
//! * **`code`** — the stable classification, one of the eleven strings `mjx_ooxml::ErrorCode` names.
//!   `catch (e) { if (e.code === "IndexOutOfRange") … }` is the intended shape.
//! * **`detail`** — a plain object carrying whichever of `surface`, `shape`, `row`, `column` and
//!   `index` the failure had, and only those. A caller reads `detail.row ?? null`.
//!
//! The Python binding takes the opposite decision — eleven exception classes rather than one code —
//! because `except` selects on the class there and on nothing here. Both projections carry the same
//! information.

use wasm_bindgen::prelude::*;

use mjx_ooxml as ooxml;

use crate::address::{ShapePath, Surface};

/// The name every failure this library raises carries.
const ERROR_NAME: &str = "OoxmlError";

/// Projects a facade failure into the `Error` a caller catches.
///
/// # Panics
/// Never. Every step that could fail — setting a property on the error object — is ignored rather
/// than propagated, because losing a coordinate is better than losing the failure.
pub(crate) fn to_js_error(error: &ooxml::Error) -> JsValue {
    let raised = js_sys::Error::new(&error.to_string());
    raised.set_name(ERROR_NAME);
    set(&raised, "code", &JsValue::from_str(error.code().as_str()));
    set(&raised, "detail", &detail_of(error.detail()));
    raised.into()
}

/// The `detail` object: the coordinates the failure carried, and no keys for the ones it did not.
fn detail_of(detail: &ooxml::ErrorDetail) -> JsValue {
    let object = js_sys::Object::new();
    if let Some(surface) = detail.surface {
        set(&object, "surface", &Surface::from(surface).into());
    }
    if let Some(shape) = detail.shape.clone() {
        set(&object, "shape", &ShapePath::from(shape).into());
    }
    if let Some(row) = detail.row {
        set(&object, "row", &JsValue::from_f64(f64::from(row)));
    }
    if let Some(column) = detail.column {
        set(&object, "column", &JsValue::from_f64(f64::from(column)));
    }
    if let Some(index) = detail.index {
        set(&object, "index", &JsValue::from_f64(f64::from(index)));
    }
    object.into()
}

/// Sets one own property, ignoring a failure that cannot happen on an object we just made.
fn set(target: &JsValue, key: &str, value: &JsValue) {
    let _ = js_sys::Reflect::set(target, &JsValue::from_str(key), value);
}

/// The whole error projection, as a `Result` adapter.
pub(crate) fn map_error<T>(result: Result<T, ooxml::Error>) -> Result<T, JsValue> {
    result.map_err(|error| to_js_error(&error))
}

/// An `OoxmlError` this binding raises itself, for content the model can express and this build
/// cannot project.
pub(crate) fn unsupported_content(message: &str) -> JsValue {
    let raised = js_sys::Error::new(message);
    raised.set_name(ERROR_NAME);
    set(
        &raised,
        "code",
        &JsValue::from_str(ooxml::ErrorCode::UnsupportedContent.as_str()),
    );
    set(&raised, "detail", &js_sys::Object::new().into());
    raised.into()
}
