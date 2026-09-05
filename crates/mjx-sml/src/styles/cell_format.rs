//! `CT_CellAlignment`, `CT_CellProtection` and `CT_NumFmt` — the three attribute-only types a `dxf`
//! and an `xf` both hold.
//!
//! # Why these are here rather than in [`super::differential`]
//!
//! `sml.xsd` reaches each of them from **two** places:
//!
//! | type | reached from |
//! |---|---|
//! | `CT_CellAlignment` (`:3402`) | `CT_Dxf/alignment` (this child) and `CT_Xf/alignment` (MJXOFF-108) |
//! | `CT_CellProtection` (`:3473`) | `CT_Dxf/protection` (this child) and `CT_Xf/protection` (MJXOFF-108) |
//! | `CT_NumFmt` (`:3583`) | `CT_Dxf/numFmt` (this child) and `CT_NumFmts/numFmt` (MJXOFF-108) |
//!
//! MJXOFF-105 is the child that reaches them first, and MJXOFF-108 is the one whose subject they
//! really are. Putting them in `differential.rs` would leave MJXOFF-108 either reaching into another
//! subject's module or writing a second copy — and **a copy would arrive with no executioner**,
//! which is the debt [`crate::font`] exists to have avoided once already. So they sit in a module
//! that belongs to neither: MJXOFF-108's `xf` reaches for these three rather than declaring them
//! again, exactly as this child reached for [`FontProperties`](crate::FontProperties).
//!
//! **A slot an `xf` needs and one of these lacks is a slot to add here.**
//!
//! # What this module is *not*
//!
//! It is not the number-format subject. [`NumberFormat`] is the two attributes of one `numFmt`
//! element; the `numFmts` table, the implied built-in format codes of Part 1 §18.8.30, and the
//! ladder that resolves a cell's `@s` through `cellXfs` into a format code are all MJXOFF-108's, and
//! `styles.xml`'s `numFmts` slot is held raw by [`super::stylesheet`] until it lands.

use mjx_ooxml_core::{Enumeration, Number, Text};
use mjx_ooxml_types::spreadsheetml::{HorizontalAlignment, TextRotation, VerticalAlignment};
use mjx_ooxml_types::support::OnOff;

use crate::leaf::attribute_bag;

attribute_bag! {
    /// `x:alignment` (`CT_CellAlignment`, `sml.xsd:3402`) — how a cell's content sits in it.
    ///
    /// `@vertical` carries the schema default `bottom`; the other eight attributes have none, so an
    /// absent one is genuinely absent — and inside a `dxf` that means *inherited*, not *default*.
    ///
    /// `@textRotation` is `ST_TextRotation`: `0..=180` degrees anticlockwise, **or** the sentinel
    /// `255`, which means vertical stacked text rather than a rotation of 255 degrees. It is
    /// reported as the number the file wrote; nothing here converts it.
    #[xml(attribute(local = "horizontal", codec = Enumeration<HorizontalAlignment>, accessor = horizontal_alignment))]
    #[xml(attribute(local = "vertical", codec = Enumeration<VerticalAlignment>, accessor = vertical_alignment, default = VerticalAlignment::Bottom))]
    #[xml(attribute(local = "textRotation", codec = Number<TextRotation>, accessor = text_rotation))]
    #[xml(attribute(local = "wrapText", codec = OnOff, accessor = wraps_text))]
    #[xml(attribute(local = "indent", codec = Number<u32>, accessor = indent))]
    #[xml(attribute(local = "relativeIndent", codec = Number<i32>, accessor = relative_indent))]
    #[xml(attribute(local = "justifyLastLine", codec = OnOff, accessor = justifies_last_line))]
    #[xml(attribute(local = "shrinkToFit", codec = OnOff, accessor = shrinks_to_fit))]
    #[xml(attribute(local = "readingOrder", codec = Number<u32>, accessor = reading_order))]
    CellAlignment, "alignment"
}

attribute_bag! {
    /// `x:protection` (`CT_CellProtection`, `sml.xsd:3473`) — whether a cell is locked, and whether
    /// its formula is hidden, **once the sheet itself is protected**.
    ///
    /// Neither attribute does anything on an unprotected sheet: `sheetProtection` is what turns them
    /// on, and that is MJXOFF-117's slot. Both are reported exactly as written.
    #[xml(attribute(local = "locked", codec = OnOff, accessor = locked))]
    #[xml(attribute(local = "hidden", codec = OnOff, accessor = formula_hidden))]
    CellProtection, "protection"
}

attribute_bag! {
    /// `x:numFmt` (`CT_NumFmt`, `sml.xsd:3583`) — one number format: its id and its format code.
    ///
    /// Both attributes are `use="required"` in the schema and both are optional here, for the reason
    /// [`SheetEntry`](crate::SheetEntry) gives: a file that omits one is reported as it stands
    /// rather than refused.
    ///
    /// **The id is not always defined here.** Part 1 §18.8.30 lists the ids whose format code is
    /// *implied* rather than written — `0` is `General`, `9` is `0%`, `14` is a date — and an `xf`
    /// may name one of those with no `numFmt` element anywhere in the part. Resolving an id to a
    /// format code is therefore MJXOFF-108's, and needs the built-in table as well as this element.
    #[xml(attribute(local = "numFmtId", codec = Number<u32>, accessor = number_format_id))]
    #[xml(attribute(local = "formatCode", codec = Text, accessor = format_code))]
    NumberFormat, "numFmt"
}
