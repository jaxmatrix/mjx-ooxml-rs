//! `Surface` and `ShapePath` — *which part* and *which shape* — and the three union types that let
//! a caller write a number instead.
//!
//! In Rust, `deck.shape_fill(slide.into(), 2.into())` works because both address types implement
//! `From<u32>`. `wasm-bindgen` gives no such thing for free: a class parameter accepts that class
//! and nothing else. So this module declares three **imported union types** — `Surface | number`,
//! `ShapePath | number | number[]`, and an array of the latter — and converts them by hand. The
//! `.d.ts` states the union, so TypeScript checks it:
//!
//! ```ts
//! deck.shapeFill(0, 2);                       // slide 0, top-level shape 2
//! deck.shapeFill(Surface.layout(1), [2, 1]);  // layout 1, member 1 of group 2
//! deck.shapeFill(Surface.notesMaster(), ShapePath.of([0]));
//! ```
//!
//! # `free()`
//!
//! `Surface` and `ShapePath` are wasm objects like every other class here, so they hold memory
//! until they are freed. A caller who passes numbers never creates one and has nothing to free —
//! which is the reason the union exists, and the reason the numeric spelling is the one the
//! examples use.

use wasm_bindgen::prelude::*;

use mjx_ooxml as ooxml;

use crate::support::invalid_argument;

/// The shape-bearing part a call is about: a slide, a layout, a master, a slide's notes, or the
/// single notes master.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Surface(pub(crate) ooxml::Surface);

#[wasm_bindgen]
impl Surface {
    /// The slide at this index, counting from zero.
    #[wasm_bindgen(js_name = "slide")]
    pub fn slide(index: u32) -> Self {
        Self(ooxml::Surface::Slide(index))
    }

    /// The slide layout at this index — one flat space across every master.
    #[wasm_bindgen(js_name = "layout")]
    pub fn layout(index: u32) -> Self {
        Self(ooxml::Surface::Layout(index))
    }

    /// The slide master at this index.
    #[wasm_bindgen(js_name = "master")]
    pub fn master(index: u32) -> Self {
        Self(ooxml::Surface::Master(index))
    }

    /// The notes slide belonging to the slide at this index.
    #[wasm_bindgen(js_name = "notes")]
    pub fn notes(slide_index: u32) -> Self {
        Self(ooxml::Surface::Notes(slide_index))
    }

    /// The single notes master every notes slide inherits from.
    #[wasm_bindgen(js_name = "notesMaster")]
    pub fn notes_master() -> Self {
        Self(ooxml::Surface::NotesMaster)
    }

    /// The index within this surface's own kind. The notes master is unique and reports `0`.
    #[wasm_bindgen(getter, js_name = "index")]
    pub fn index(&self) -> u32 {
        self.0.index()
    }

    /// The kind's name: `"slide"`, `"layout"`, `"master"`, `"notes"` or `"notes master"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        self.0.kind_name().to_owned()
    }

    /// Whether this stands at the head of its own inheritance chain — a slide master or the notes
    /// master, neither of which inherits from a further part.
    #[wasm_bindgen(getter, js_name = "isMasterLike")]
    pub fn is_master_like(&self) -> bool {
        self.0.is_master_like()
    }

    /// Whether this addresses the same part as `other`.
    #[wasm_bindgen(js_name = "equals")]
    pub fn equals(&self, other: &Surface) -> bool {
        self.0 == other.0
    }

    /// `slide 0`, `layout 1`, `notes master`.
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_display_string(&self) -> String {
        self.0.to_string().to_owned()
    }
}

impl From<ooxml::Surface> for Surface {
    fn from(surface: ooxml::Surface) -> Self {
        Self(surface)
    }
}

/// The address of a shape within a surface's shape tree: a top-level index, then the indices to
/// descend through nested groups.
#[wasm_bindgen]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShapePath(pub(crate) ooxml::ShapePath);

#[wasm_bindgen]
impl ShapePath {
    /// The top-level shape at this index.
    #[wasm_bindgen(js_name = "top")]
    pub fn top(index: u32) -> Self {
        Self(ooxml::ShapePath::from(index))
    }

    /// The shape at this address: `[2]` top-level, `[2, 1]` for member 1 of the group at index 2.
    #[wasm_bindgen(js_name = "of")]
    pub fn of(indices: Vec<u32>) -> Result<ShapePath, JsValue> {
        if indices.is_empty() {
            return Err(invalid_argument(
                "a shape address needs at least one index; the shape tree is not itself a shape",
            ));
        }
        Ok(Self(ooxml::ShapePath::from(indices)))
    }

    /// The address as an array of indices, outermost first.
    #[wasm_bindgen(getter, js_name = "indices")]
    pub fn indices(&self) -> Vec<u32> {
        self.0.indices().to_vec()
    }

    /// How deep the address reaches: `1` for a top-level shape, `2` for a member of a top-level
    /// group, and so on.
    #[wasm_bindgen(getter, js_name = "depth")]
    pub fn depth(&self) -> u32 {
        self.0.depth()
    }

    /// Whether this addresses a top-level shape — a single index, no group descent.
    #[wasm_bindgen(getter, js_name = "isTopLevel")]
    pub fn is_top_level(&self) -> bool {
        self.0.is_top_level()
    }

    /// The address of member `index` of the group this addresses — one step deeper.
    #[wasm_bindgen(js_name = "child")]
    pub fn child(&self, index: u32) -> Self {
        Self(self.0.child(index))
    }

    /// The address of the group this shape belongs to, or `undefined` for a top-level shape.
    #[wasm_bindgen(getter, js_name = "parent")]
    pub fn parent(&self) -> Option<ShapePath> {
        self.0.parent().map(Self)
    }

    /// Whether this addresses the same shape as `other`.
    #[wasm_bindgen(js_name = "equals")]
    pub fn equals(&self, other: &ShapePath) -> bool {
        self.0 == other.0
    }

    /// `2` for a top-level shape, `[2, 1]` for a group member.
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_display_string(&self) -> String {
        self.0.to_string().to_owned()
    }
}

impl From<ooxml::ShapePath> for ShapePath {
    fn from(path: ooxml::ShapePath) -> Self {
        Self(path)
    }
}

#[wasm_bindgen]
extern "C" {
    /// A surface argument: a `Surface`, or a number meaning that slide.
    #[wasm_bindgen(typescript_type = "Surface | number")]
    pub type SurfaceArg;

    /// A shape argument: a `ShapePath`, a number meaning that top-level shape, or an array of
    /// numbers descending through groups.
    #[wasm_bindgen(typescript_type = "ShapePath | number | number[]")]
    pub type ShapePathArg;

    /// Several shape arguments, each in any of the three spellings.
    #[wasm_bindgen(typescript_type = "(ShapePath | number | number[])[]")]
    pub type ShapePathListArg;
}

/// The model's surface, from whichever spelling the caller used.
pub(crate) fn surface_of(argument: &SurfaceArg) -> Result<ooxml::Surface, JsValue> {
    let value: &JsValue = argument.as_ref();
    if let Some(index) = as_index(value) {
        return Ok(ooxml::Surface::Slide(index?));
    }
    read_surface(value)
}

/// The model's shape address, from whichever spelling the caller used.
pub(crate) fn path_of(argument: &ShapePathArg) -> Result<ooxml::ShapePath, JsValue> {
    path_of_value(argument.as_ref())
}

/// The model's shape addresses, from an array of any of the three spellings.
pub(crate) fn paths_of(argument: &ShapePathListArg) -> Result<Vec<ooxml::ShapePath>, JsValue> {
    let value: &JsValue = argument.as_ref();
    let array = value
        .dyn_ref::<js_sys::Array>()
        .ok_or_else(|| invalid_argument("a list of shape addresses is an array"))?;
    array.iter().map(|entry| path_of_value(&entry)).collect()
}

/// The shared conversion behind [`path_of`] and [`paths_of`].
fn path_of_value(value: &JsValue) -> Result<ooxml::ShapePath, JsValue> {
    if let Some(index) = as_index(value) {
        return Ok(ooxml::ShapePath::from(index?));
    }
    if let Some(array) = value.dyn_ref::<js_sys::Array>() {
        return indices_of(array).map(ooxml::ShapePath::from);
    }
    read_path(value)
}

// -----------------------------------------------------------------------------------------------
// Reading an address class *without consuming it*
//
// `wasm-bindgen`'s own `JsValue` → exported-class conversion takes ownership: it unwraps the
// pointer and zeroes the JavaScript object's handle, so `deck.shapeCount(s)` would leave `s` freed
// and `deck.shapeKind(s, 0)` on the next line would throw. That is not an argument conversion
// anyone would want.
//
// So an address class is read the way JavaScript would read it — through the getters it publishes —
// and rebuilt. Reading a *freed* object throws inside wasm, which surfaces here as a `Reflect`
// failure and is reported as "not an address", which is exactly what it is.
// -----------------------------------------------------------------------------------------------

/// One property of a JavaScript object, or `None` if reading it failed or it was absent.
fn property(value: &JsValue, name: &str) -> Option<JsValue> {
    if !value.is_object() {
        return None;
    }
    js_sys::Reflect::get(value, &JsValue::from_str(name))
        .ok()
        .filter(|found| !found.is_undefined() && !found.is_null())
}

/// A `Surface`, rebuilt from the `kind` and `index` it publishes.
fn read_surface(value: &JsValue) -> Result<ooxml::Surface, JsValue> {
    let refuse = || invalid_argument("a surface is a Surface or a number (the slide index)");
    let kind = property(value, "kind")
        .and_then(|kind| kind.as_string())
        .ok_or_else(refuse)?;
    if kind == "notes master" {
        return Ok(ooxml::Surface::NotesMaster);
    }
    let index = property(value, "index")
        .and_then(|index| as_index(&index))
        .ok_or_else(refuse)??;
    match kind.as_str() {
        "slide" => Ok(ooxml::Surface::Slide(index)),
        "layout" => Ok(ooxml::Surface::Layout(index)),
        "master" => Ok(ooxml::Surface::Master(index)),
        "notes" => Ok(ooxml::Surface::Notes(index)),
        _ => Err(refuse()),
    }
}

/// A `ShapePath`, rebuilt from the `indices` it publishes.
fn read_path(value: &JsValue) -> Result<ooxml::ShapePath, JsValue> {
    let refuse =
        || invalid_argument("a shape address is a ShapePath, a number, or an array of numbers");
    let indices = property(value, "indices").ok_or_else(refuse)?;
    let array = js_sys::Uint32Array::new(&indices).to_vec();
    if array.is_empty() {
        return Err(refuse());
    }
    Ok(ooxml::ShapePath::from(array))
}

/// The whole, non-negative indices an array of numbers holds.
fn indices_of(array: &js_sys::Array) -> Result<Vec<u32>, JsValue> {
    if array.length() == 0 {
        return Err(invalid_argument(
            "a shape address needs at least one index; the shape tree is not itself a shape",
        ));
    }
    let mut indices = Vec::with_capacity(array.length() as usize);
    for entry in array.iter() {
        match as_index(&entry) {
            Some(index) => indices.push(index?),
            None => {
                return Err(invalid_argument(
                    "a shape address given as an array holds whole, non-negative numbers",
                ))
            }
        }
    }
    Ok(indices)
}

/// A JavaScript number as the whole, non-negative index an address is made of.
///
/// `None` means "not a number at all", so the caller can try the class instead; `Some(Err(…))` means
/// "a number, but not one an index can be" — which is a mistake worth naming rather than silently
/// truncating. JavaScript has one numeric type, so `2.5` and `-1` both arrive here looking plausible.
fn as_index(value: &JsValue) -> Option<Result<u32, JsValue>> {
    let number = value.as_f64()?;
    if number.fract() != 0.0 || number < 0.0 || number > f64::from(u32::MAX) {
        return Some(Err(invalid_argument(format!(
            "an index is a whole number between 0 and {}, not {number}",
            u32::MAX
        ))));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the range and integrality were just checked"
    )]
    Some(Ok(number as u32))
}
