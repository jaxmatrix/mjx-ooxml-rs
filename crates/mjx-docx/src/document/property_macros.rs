//! Generic property-declaration macros shared by every rank-ordered container in this crate's WML
//! model: `RunProperties` (MJXOFF-94), `ParagraphProperties`/`ParagraphMarkRunProperties`
//! (MJXOFF-96), and `StyleParagraphProperties`/`StyleDefinition`/`TableStyleOverride` (MJXOFF-101).
//!
//! Extracted here rather than left as `paragraph_properties.rs`-local `macro_rules!` items so
//! MJXOFF-101 can invoke the same five macros over its own content enums instead of restating this
//! getter/setter logic a third time — the exact "consume, do not re-create" reuse this workspace's
//! own naming/process rules ask for.
//!
//! `macro_rules!` gives a bare type name written in a macro body (`Toggle`, `Border`,
//! `DecimalNumberValue`, `HalfPointMeasureValue`, `AttributeError`, `HalfPointMeasure`, `Interner` —
//! every one of them, below) **call-site** resolution, not definition-site: each module that invokes
//! one of these five macros (`paragraph_properties.rs`, `styles.rs`) must have the relevant types in
//! scope itself — confirmed directly (removing the equivalent imports from *this* file changes
//! nothing; it is each call site's own pre-existing `use` of `Toggle`/`Border`/… for its own leaf
//! types that satisfies the macro body's references), so nothing is imported here that only a macro
//! body would use.

/// Declares one `CT_OnOff`-shaped property on the container type the macro is invoked inside: a
/// tri-state getter and a whole-value setter.
macro_rules! toggle_property {
    ($enum_ty:ident, $getter:ident, $setter:ident, $variant:ident, $local:literal, $doc:literal) => {
        #[doc = $doc]
        pub fn $getter(&self, interner: &Interner) -> Result<Option<bool>, AttributeError> {
            self.content
                .iter()
                .find_map(|item| match item {
                    $enum_ty::$variant(toggle) => Some(toggle),
                    _ => None,
                })
                .map(|toggle| toggle.value(interner))
                .transpose()
        }

        #[doc = concat!("Sets `w:", $local, "`: `None` removes the element; `Some(value)` ensures it \
            is present with `w:val` written explicitly as `value`.")]
        pub fn $setter(&mut self, interner: &mut Interner, value: Option<bool>) {
            let is_target = |item: &$enum_ty| matches!(item, $enum_ty::$variant(_));
            match value {
                None => self.remove(is_target),
                Some(value) => {
                    let mut toggle = Toggle::new(interner, $local);
                    toggle.set_value(interner, Some(value));
                    self.set($local, is_target, Some($enum_ty::$variant(toggle)));
                }
            }
        }
    };
}

/// Declares one whole-value property: a borrowing getter and a replace-insert-or-remove setter,
/// generalized the same way as [`toggle_property!`].
macro_rules! value_property {
    ($enum_ty:ident, $getter:ident, $setter:ident, $variant:ident, $ty:ty, $local:literal, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<&$ty> {
            self.content.iter().find_map(|item| match item {
                $enum_ty::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("Sets `w:", $local, "`: `None` removes the element; `Some(value)` replaces \
            or inserts it at its schema rank.")]
        pub fn $setter(&mut self, value: Option<$ty>) {
            let is_target = |item: &$enum_ty| matches!(item, $enum_ty::$variant(_));
            self.set($local, is_target, value.map($enum_ty::$variant));
        }
    };
}

/// Declares one `CT_PBdr`-slot property: a borrowing getter and a setter that **renames** the
/// [`Border`] it is given to this slot's own wire local before storing it — see
/// `run_properties.rs`'s [`Border::renamed`] for why a plain [`value_property!`] setter would
/// silently emit the wrong element name here.
macro_rules! border_property {
    ($enum_ty:ident, $getter:ident, $setter:ident, $variant:ident, $local:literal, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<&Border> {
            self.content.iter().find_map(|item| match item {
                $enum_ty::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("Sets `w:", $local, "`: `None` removes the border; `Some(value)` replaces \
            or inserts it at its schema rank, renamed to `w:", $local, "` regardless of the name \
            `value` already carried.")]
        pub fn $setter(&mut self, interner: &mut Interner, value: Option<Border>) {
            let is_target = |item: &$enum_ty| matches!(item, $enum_ty::$variant(_));
            let value = value.map(|border| border.renamed(interner, $local));
            self.set($local, is_target, value.map($enum_ty::$variant));
        }
    };
}

/// Declares one `CT_DecimalNumber`-shaped property: a fallible flattened getter and a whole-value
/// setter that builds [`DecimalNumberValue`] under its own wire name.
macro_rules! decimal_number_property {
    ($enum_ty:ident, $getter:ident, $setter:ident, $variant:ident, $local:literal, $doc:literal) => {
        #[doc = $doc]
        pub fn $getter(&self, interner: &Interner) -> Result<Option<i64>, AttributeError> {
            self.content
                .iter()
                .find_map(|item| match item {
                    $enum_ty::$variant(value) => Some(value),
                    _ => None,
                })
                .map(|value| value.value(interner))
                .transpose()
        }

        #[doc = concat!("Sets `w:", $local, "`: `None` removes the element; `Some(value)` ensures it \
            is present with `w:val` written explicitly as `value`.")]
        pub fn $setter(&mut self, interner: &mut Interner, value: Option<i64>) {
            let is_target = |item: &$enum_ty| matches!(item, $enum_ty::$variant(_));
            match value {
                None => self.remove(is_target),
                Some(value) => {
                    let element = DecimalNumberValue::new(interner, $local, value);
                    self.set($local, is_target, Some($enum_ty::$variant(element)));
                }
            }
        }
    };
}

/// Declares one `CT_HpsMeasure`-shaped property (`sz`, `szCs`, `kern`): a fallible flattened getter
/// and a whole-value setter that builds [`HalfPointMeasureValue`] under its own wire name.
macro_rules! half_point_property {
    ($enum_ty:ident, $getter:ident, $setter:ident, $variant:ident, $local:literal, $doc:literal) => {
        #[doc = $doc]
        pub fn $getter(&self, interner: &Interner) -> Result<Option<HalfPointMeasure>, AttributeError> {
            self.content
                .iter()
                .find_map(|item| match item {
                    $enum_ty::$variant(value) => Some(value),
                    _ => None,
                })
                .map(|value| value.half_points(interner))
                .transpose()
        }

        #[doc = concat!("Sets `w:", $local, "`: `None` removes the element; `Some(value)` ensures it \
            is present with `w:val` written explicitly as `value` half-points.")]
        pub fn $setter(&mut self, interner: &mut Interner, value: Option<HalfPointMeasure>) {
            let is_target = |item: &$enum_ty| matches!(item, $enum_ty::$variant(_));
            match value {
                None => self.remove(is_target),
                Some(value) => {
                    let element = HalfPointMeasureValue::new(interner, $local, value);
                    self.set($local, is_target, Some($enum_ty::$variant(element)));
                }
            }
        }
    };
}

// Path-importable (Rust 2018 macro-by-path): a call site does `use
// super::property_macros::{toggle_property, ...};` then invokes `toggle_property!(...)` — no
// `#[macro_use]`, no crate-root pollution.
pub(crate) use border_property;
pub(crate) use decimal_number_property;
pub(crate) use half_point_property;
pub(crate) use toggle_property;
pub(crate) use value_property;
