//! The shared machinery every value class in this crate uses.
//!
//! * [`value_class!`] declares the `#[wasm_bindgen]` newtype, its conversions, and the two methods
//!   JavaScript expects of a value — `equals` and `toString`. Every wrapper here is
//!   `NewType(mjx_ooxml::Thing)`: one field, no state of its own.
//! * [`to_bytes`] and [`str_list`] spell the two conversions that would otherwise be repeated at
//!   two hundred call sites.

use wasm_bindgen::prelude::*;

/// Declares a value class: a `#[wasm_bindgen]` newtype over a `mjx_ooxml` value, the conversions in
/// both directions, and `equals` / `toString`.
///
/// These classes are **values** — a `FillSpec` describes a fill, it is not a handle to one — so
/// every builder returns a new instance rather than mutating in place, exactly as the Rust builders
/// do. Each still owns memory on the wasm heap, so each still has `free()`; see the crate
/// documentation.
macro_rules! value_class {
    ($(
        $(#[$attribute:meta])*
        $name:ident($model:path) $(, derive($($extra:ident),+ $(,)?))?;
    )*) => {
        $(
            $(#[$attribute])*
            #[wasm_bindgen]
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

            #[wasm_bindgen]
            impl $name {
                /// Whether this value equals `other`.
                ///
                /// JavaScript's `===` compares wasm handles, so two separately-built descriptions
                /// of the same fill are never `===` even when they say the same thing. This is the
                /// comparison that means what a reader expects.
                #[wasm_bindgen(js_name = "equals")]
                pub fn equals(&self, other: &$name) -> bool {
                    self.0 == other.0
                }

                /// A description for logging and debugging. Not a stable format; do not parse it.
                #[wasm_bindgen(js_name = "toString")]
                pub fn to_display_string(&self) -> String {
                    format!("{:?}", self.0)
                }
            }
        )*
    };
}

/// A byte window as the `Uint8Array` the caller receives.
///
/// One copy, out of the wasm heap into JavaScript's. That is the whole cost of the boundary, and it
/// is why the library takes and returns whole payloads rather than streaming.
pub(crate) fn to_bytes(bytes: &[u8]) -> Vec<u8> {
    bytes.to_vec()
}

/// A list of owned strings as the model's `&[&str]` argument shape.
pub(crate) fn str_list(owned: &[String]) -> Vec<&str> {
    owned.iter().map(String::as_str).collect()
}

/// A `RangeError` for an argument this binding rejects before the model sees it.
///
/// Distinct from [`crate::errors::to_js_error`], which projects a failure the *library* reported:
/// this one is a mistake in the call itself.
pub(crate) fn invalid_argument(message: impl AsRef<str>) -> JsValue {
    let error = js_sys::RangeError::new(message.as_ref());
    error.into()
}
