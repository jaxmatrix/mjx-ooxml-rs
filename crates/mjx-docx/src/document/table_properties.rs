//! `CT_TblPrBase`/`CT_TblPrExBase`/`CT_TrPrBase`/`CT_TcPrBase` — the four formatting-property bases
//! MJXOFF-116 left opaque: a table's own `w:tblPr`, a row's exception set `w:tblPrEx`, a row's own
//! `w:trPr`, and the eleven members `w:tc/w:tcPr` (`tables.rs`) does not yet type.
//!
//! # One set of leaf types, four containers, no parallel model
//!
//! `CT_TblStylePr` (`w:style/w:tblStylePr`, MJXOFF-101's [`super::styles::TableStyleOverride`]) has
//! its own `tblPr`/`trPr`/`tcPr` children — but they are `CT_TblPrBase` and literally `CT_TrPr`/
//! `CT_TcPr` (verified against `wml.xsd` directly, not assumed), the *exact same* complex types this
//! module and `tables.rs` already model for a live table's own properties. [`TableProperties`],
//! [`RowProperties`] and `tables.rs`'s own [`super::tables::CellProperties`] are therefore reused
//! **directly** for both — `styles.rs` is updated to stop wrapping them in `Unmodeled` — rather than
//! this module inventing a second, parallel set of table-property types. The one type in this module
//! *not* shared with a table style is [`TableExceptionProperties`] (`CT_TblPrExBase`, `w:tblPrEx`):
//! `CT_TblStylePr` has no exception-set child at all.
//!
//! # `w:tblLook`/`w:cnfStyle`'s `val` is a legacy, undocumented artifact — never consulted
//!
//! Both `CT_TblLook` and `CT_Cnf` (the latter already modelled by MJXOFF-106 as
//! [`super::paragraph_properties::ConditionalFormatting`]) carry **two** representations of the same
//! information in `wml.xsd`'s Transitional schema: twelve (`CT_Cnf`) or six (`CT_TblLook`) named
//! `ST_OnOff` attributes, *and* a `val` attribute (a 12-character `[01]*` bitmask for `CT_Cnf`, a
//! `ST_ShortHexNumber` for `CT_TblLook`). ECMA-376 Part 1's own prose for both elements — §17.3.1.8
//! (`cnfStyle`), §17.4.7/§17.4.8 (the row/cell variants) and §17.4.55 (`tblLook`) — documents **only
//! the named attributes**; `val` is never mentioned, has no documented bit-position-to-region mapping,
//! and every worked example in the spec writes the named attributes directly (`w:firstRow="true"`,
//! never `w:val="1000...`"`). [`TableLook`] therefore reads and writes `val` for round-trip fidelity
//! alone ([`TableLook::legacy_bitmask`]) and **never derives a region from it** — region resolution
//! (`table_regions.rs`) reads the six named flags exclusively. This is the opposite of what an earlier
//! dispatch note for this child assumed (that `val`'s bit positions were the authority to establish
//! from the prose); the prose itself settles it the other way. See `table_regions.rs`'s own doc
//! comment for the full account.
//!
//! # `w:tblW`'s width is never exposed without its unit
//!
//! `CT_TblWidth`'s `w` attribute (`ST_MeasurementOrPercent`) means twentieths of a point, fiftieths of
//! a percent, `0` or "automatic", depending on the sibling `type` attribute (`ST_TblWidth`) — the same
//! shape of trap as `w:spacing/@line` needing `@lineRule` (MJXOFF-96's
//! [`super::paragraph_properties::Spacing::line_spacing`]). [`TableWidth::measure`] is therefore the
//! *only* way to read either half, always returning both together as one [`TableWidthMeasure`], with
//! ECMA-376 Part 1 §17.4.87's own stated defaults applied when either attribute is absent (`type` →
//! `dxa`, `w` → `"0"`) — the default is *returned*, never *written*.

use std::borrow::Cow;

use mjx_ooxml_core::{
    AttributeCodec, AttributeError, Enumeration, FromXml, FromXmlError, Interner,
    InvalidAttributeValue, RawAttribute, RawElement, RawName, RawNode, Text as TextCodec, ToXml,
};
use mjx_ooxml_types::child_order::{
    CELL_BORDERS, CELL_MARGINS, TABLE_BORDERS, TABLE_CELL_MARGINS, TABLE_EXCEPTION_PROPERTIES_BASE,
    TABLE_PROPERTIES_BASE, TABLE_ROW_PROPERTIES_BASE,
};
use mjx_ooxml_types::shared::{RelativeHorizontalAlignment, RelativeVerticalAlignment};
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::wordprocessingml::{
    FourDigitHexadecimalNumber, HeightRule, HorizontalAnchor, MeasurementOrPercentage,
    TableJustification, TableLayoutType, TableOverlap, TableWidthUnit, TextFlowDirection,
    VerticalAnchor, VerticalJustification,
};

use super::body::wml_name;
use super::paragraph_properties::{ConditionalFormatting, DecimalNumberValue};
use super::property_macros::{
    border_property, decimal_number_property, toggle_property, value_property,
};
use super::run_properties::{Border, Shading, SignedTwips, Toggle, Twips};

// -------------------------------------------------------------------------------------------
// Attribute codecs this module needs beyond what `run_properties.rs`/`paragraph_properties.rs`
// already declared — both raw wire-string passthroughs, in the same shape as `run_properties.rs`'s
// own `Scale`/`Lang`.
// -------------------------------------------------------------------------------------------

/// `ST_MeasurementOrPercent` (`w:tblW`'s `w` and its five siblings' own `w`) as an attribute value —
/// the wire string itself, preserved exactly; see this module's own doc comment for why it is never
/// parsed further here.
#[derive(Debug)]
struct MeasurementOrPercent;

impl AttributeCodec for MeasurementOrPercent {
    type Value<'a> = MeasurementOrPercentage;
    type Input<'a> = MeasurementOrPercentage;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<MeasurementOrPercentage, InvalidAttributeValue> {
        Ok(MeasurementOrPercentage::from_wire(&raw))
    }

    fn encode<'a>(value: MeasurementOrPercentage) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

/// `ST_ShortHexNumber` (`w:tblLook`'s legacy `val`) as an attribute value.
#[derive(Debug)]
struct ShortHex;

impl AttributeCodec for ShortHex {
    type Value<'a> = FourDigitHexadecimalNumber;
    type Input<'a> = FourDigitHexadecimalNumber;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<FourDigitHexadecimalNumber, InvalidAttributeValue> {
        Ok(FourDigitHexadecimalNumber::from_wire(&raw))
    }

    fn encode<'a>(value: FourDigitHexadecimalNumber) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

// -------------------------------------------------------------------------------------------
// CT_String, reused under four names (`DecimalNumberValue`'s own "one wire shape, several names"
// pattern, `paragraph_properties.rs`).
// -------------------------------------------------------------------------------------------

/// `CT_String` — a required `val` string. Reused for `w:tblStyle` (a table-style reference),
/// `w:tblCaption`, `w:tblDescription` and `w:header` (inside [`CellHeaderReferences`]).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = value, required))]
pub struct TableStringValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl TableStringValue {
    /// Builds a new `local` element (`"tblStyle"`, `"tblCaption"`, `"tblDescription"` or `"header"`)
    /// of `value`.
    #[must_use]
    pub(crate) fn new(interner: &mut Interner, local: &str, value: &str) -> Self {
        let mut item = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for TableStringValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for TableStringValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_TblWidth (w:tblW and five siblings) — the line/lineRule-shaped trap.
// -------------------------------------------------------------------------------------------

/// `w:tblW`'s `type` paired with its `w` — the only way [`TableWidth::measure`] returns either. See
/// this module's own doc comment for why a caller never sees `w` without `type`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableWidthMeasure {
    /// `type` (`ST_TblWidth`) — which of `auto`/`dxa`/`nil`/`pct` `value` means.
    pub unit: TableWidthUnit,
    /// `w` (`ST_MeasurementOrPercent`), in the unit `unit` names — the raw wire value, since the
    /// union has no single numeric shape (a plain integer for `dxa`/`pct`, a `ST_UniversalMeasure`
    /// string schema-legal but never written by Word for this element).
    pub value: MeasurementOrPercentage,
}

/// `CT_TblWidth` (`w:tblW`, `w:tblCellSpacing`, `w:tblInd`, `w:wBefore`, `w:wAfter`, `w:tcW`, and
/// each of [`TableCellMargins`]/[`CellMargins`]'s six sides — "Table Measurement", §17.4.87) — a
/// measurement whose unit is a sibling attribute. One Rust type under many wire names, exactly as
/// [`Border`] is; `TableWidth::renamed` (crate-private) is how a setter corrects the name before
/// storing, the same shape `Border::renamed` already established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableWidth {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl TableWidth {
    /// Builds a new width with no stated measure — a placeholder name that whichever `set_*` a
    /// caller passes this to (`TableProperties::set_width`, `CellMargins::set_top`, …) renames to
    /// its own slot; see this type's own doc comment.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "tblW"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }

    /// Renames this width to `local`, keeping every attribute — see this type's own doc comment.
    #[must_use]
    pub(crate) fn renamed(mut self, interner: &mut Interner, local: &str) -> Self {
        self.name = wml_name(interner, local);
        self
    }

    /// Reads `type` and `w` together. ECMA-376 Part 1 §17.4.87's own stated defaults are applied when
    /// either is absent (`type` → `dxa`, `w` → `"0"`) — the default is *returned*, never written.
    ///
    /// # Errors
    /// An [`AttributeError`] if `type` is present but not one of `ST_TblWidth`'s four tokens.
    pub fn measure(&self, interner: &Interner) -> Result<TableWidthMeasure, AttributeError> {
        let unit = mjx_xml::attribute::read::<Enumeration<TableWidthUnit>>(
            &self.attributes,
            interner,
            Some("w"),
            "type",
            "w:type",
        )?
        .unwrap_or(TableWidthUnit::Twips);
        let value = mjx_xml::attribute::read::<MeasurementOrPercent>(
            &self.attributes,
            interner,
            Some("w"),
            "w",
            "w:w",
        )?
        .unwrap_or_else(|| MeasurementOrPercentage::from_wire("0"));
        Ok(TableWidthMeasure { unit, value })
    }

    /// Writes `type` and `w` together: `None` removes both; `Some` writes both explicitly, so a
    /// later read is never ambiguous about which unit `value` is in.
    pub fn set_measure(&mut self, interner: &mut Interner, measure: Option<TableWidthMeasure>) {
        match measure {
            None => {
                mjx_xml::attribute::write::<Enumeration<TableWidthUnit>>(
                    &mut self.attributes,
                    interner,
                    Some("w"),
                    "type",
                    None,
                );
                mjx_xml::attribute::write::<MeasurementOrPercent>(
                    &mut self.attributes,
                    interner,
                    Some("w"),
                    "w",
                    None,
                );
            }
            Some(TableWidthMeasure { unit, value }) => {
                mjx_xml::attribute::write::<Enumeration<TableWidthUnit>>(
                    &mut self.attributes,
                    interner,
                    Some("w"),
                    "type",
                    Some(unit),
                );
                mjx_xml::attribute::write::<MeasurementOrPercent>(
                    &mut self.attributes,
                    interner,
                    Some("w"),
                    "w",
                    Some(value),
                );
                self.empty = false;
            }
        }
    }
}

impl FromXml for TableWidth {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for TableWidth {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// Declares one [`TableWidth`]-shaped property, renaming on write — the [`TableWidth`] counterpart of
/// `property_macros.rs`'s own `border_property!`, needed for the same reason: this one Rust type is
/// reused under several wire names within a single container, so a plain `value_property!` setter
/// could silently store a width under the wrong element name.
macro_rules! table_width_property {
    ($enum_ty:ident, $getter:ident, $setter:ident, $variant:ident, $local:literal, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<&TableWidth> {
            self.content.iter().find_map(|item| match item {
                $enum_ty::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("Sets `w:", $local, "`: `None` removes it; `Some(value)` replaces or \
            inserts it at its schema rank, renamed to `w:", $local, "` regardless of the name \
            `value` already carried.")]
        pub fn $setter(&mut self, interner: &mut Interner, value: Option<TableWidth>) {
            let is_target = |item: &$enum_ty| matches!(item, $enum_ty::$variant(_));
            let value = value.map(|width| width.renamed(interner, $local));
            self.set($local, is_target, value.map($enum_ty::$variant));
        }
    };
}

// `tables.rs`'s `CellProperties` (MJXOFF-116) extends `CT_TcPrBase` with `w:tcW`, so it reuses this
// macro too — path-importable the same way `property_macros.rs`'s own macros are.
pub(crate) use table_width_property;

// -------------------------------------------------------------------------------------------
// CT_TblLook (w:tblLook)
// -------------------------------------------------------------------------------------------

/// `CT_TblLook` (`w:tblLook`, "Table Style Conditional Formatting Settings", §17.4.55) — which of a
/// table style's conditional formats this table actually applies. Six named flags, unstated meaning
/// `false` (ECMA-376 Part 1's own stated default: row/column banding on, every edge off); `val` is a
/// legacy `ST_ShortHexNumber` this type preserves for round-trip but never consults — see this
/// module's own doc comment.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "firstRow", prefix = "w", codec = OnOff, accessor = first_row))]
#[xml(attribute(local = "lastRow", prefix = "w", codec = OnOff, accessor = last_row))]
#[xml(attribute(local = "firstColumn", prefix = "w", codec = OnOff, accessor = first_column))]
#[xml(attribute(local = "lastColumn", prefix = "w", codec = OnOff, accessor = last_column))]
#[xml(attribute(local = "noHBand", prefix = "w", codec = OnOff, accessor = no_horizontal_band))]
#[xml(attribute(local = "noVBand", prefix = "w", codec = OnOff, accessor = no_vertical_band))]
#[xml(attribute(local = "val", prefix = "w", codec = ShortHex, accessor = legacy_bitmask))]
pub struct TableLook {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl TableLook {
    /// A fresh, empty `w:tblLook` — every flag absent (so every edge reads as off, banding as on)
    /// until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "tblLook"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for TableLook {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for TableLook {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_JcTable (w:jc, inside a table/row/exception context — always local "jc")
// -------------------------------------------------------------------------------------------

/// `CT_JcTable` (`w:jc`, "Table Alignment", within `w:tblPr`/`w:tblPrEx`/`w:trPr`) — a required table
/// justification. A distinct type from [`super::paragraph_properties::ParagraphAlignment`] (`CT_Jc`)
/// even though both wrap a single required enum `val`: the two `ST_*` enums differ (`ST_JcTable` has
/// no `both`/`distribute`/…), so this is not the same wire shape under a new name.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<TableJustification>, accessor = value, required))]
pub struct TableAlignment {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl TableAlignment {
    /// Builds a new `w:jc` of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, value: TableJustification) -> Self {
        let mut item = Self {
            name: wml_name(interner, "jc"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for TableAlignment {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for TableAlignment {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_TblPPr (w:tblpPr) — floating-table position
// -------------------------------------------------------------------------------------------

/// `CT_TblPPr` (`w:tblpPr`, "Floating Table Positioning", §17.4.59) — where a floating table sits
/// relative to text, in the same independent-attribute shape as
/// [`super::paragraph_properties::FrameProperties`]'s own `x`/`xAlign` pair.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "leftFromText", prefix = "w", codec = Twips, accessor = left_from_text))]
#[xml(attribute(local = "rightFromText", prefix = "w", codec = Twips, accessor = right_from_text))]
#[xml(attribute(local = "topFromText", prefix = "w", codec = Twips, accessor = top_from_text))]
#[xml(attribute(local = "bottomFromText", prefix = "w", codec = Twips, accessor = bottom_from_text))]
#[xml(attribute(local = "vertAnchor", prefix = "w", codec = Enumeration<VerticalAnchor>, accessor = vertical_anchor))]
#[xml(attribute(local = "horzAnchor", prefix = "w", codec = Enumeration<HorizontalAnchor>, accessor = horizontal_anchor))]
#[xml(attribute(local = "tblpXSpec", prefix = "w", codec = Enumeration<RelativeHorizontalAlignment>, accessor = x_alignment))]
#[xml(attribute(local = "tblpX", prefix = "w", codec = SignedTwips, accessor = x))]
#[xml(attribute(local = "tblpYSpec", prefix = "w", codec = Enumeration<RelativeVerticalAlignment>, accessor = y_alignment))]
#[xml(attribute(local = "tblpY", prefix = "w", codec = SignedTwips, accessor = y))]
pub struct FloatingTablePosition {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FloatingTablePosition {
    /// A fresh, empty `w:tblpPr` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "tblpPr"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for FloatingTablePosition {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for FloatingTablePosition {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_TblOverlap (w:tblOverlap) and CT_TblLayoutType (w:tblLayout)
// -------------------------------------------------------------------------------------------

/// `CT_TblOverlap` (`w:tblOverlap`, "Floating Table Allows Other Tables to Overlap", §17.4.56) — a
/// required overlap setting. Named `FloatingTableOverlap` (never bare `TblOverlap`) to avoid colliding
/// with the generated value enum [`TableOverlap`] it wraps.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<TableOverlap>, accessor = value, required))]
pub struct FloatingTableOverlap {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FloatingTableOverlap {
    /// Builds a new `w:tblOverlap` of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, value: TableOverlap) -> Self {
        let mut item = Self {
            name: wml_name(interner, "tblOverlap"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for FloatingTableOverlap {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for FloatingTableOverlap {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_TblLayoutType` (`w:tblLayout`, "Table Layout", §17.4.52) — `fixed` versus `autofit`, on the
/// optional `type` attribute (not `val`).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "type", prefix = "w", codec = Enumeration<TableLayoutType>, accessor = layout))]
pub struct TableLayout {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl TableLayout {
    /// Builds a new `w:tblLayout`; `layout` is written only when given.
    #[must_use]
    pub fn new(interner: &mut Interner, layout: Option<TableLayoutType>) -> Self {
        let mut item = Self {
            name: wml_name(interner, "tblLayout"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_layout(interner, layout);
        item
    }
}

impl FromXml for TableLayout {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for TableLayout {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_TblBorders (w:tblBorders) and CT_TcBorders (w:tcBorders) — reusing `Border` (MJXOFF-94).
// -------------------------------------------------------------------------------------------

/// One ordered child of [`TableBorders`]: `CT_TblBorders`'s sequence is `top, start, left, bottom,
/// end, right, insideH, insideV` — every one a [`Border`] (MJXOFF-94's own type, reused rather than
/// restated, as `ParagraphBorders` (`paragraph_properties.rs`) already does for `w:pBdr`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableBorderContent {
    /// `w:top`.
    Top(Border),
    /// `w:start` (logical, Strict-preferred spelling of the left/leading edge).
    Start(Border),
    /// `w:left` (physical, Transitional spelling).
    Left(Border),
    /// `w:bottom`.
    Bottom(Border),
    /// `w:end` (logical spelling of the right/trailing edge).
    End(Border),
    /// `w:right` (physical spelling).
    Right(Border),
    /// `w:insideH` — the borders between rows.
    InsideHorizontal(Border),
    /// `w:insideV` — the borders between columns.
    InsideVertical(Border),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_TblBorders` (`w:tblBorders`, "Table Borders", §17.4.39) — the eight borders a table (or a
/// table style's `w:tblPr`/`w:tblPrEx`) can carry.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct TableBorders {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "top", variant = Top, ty = Border),
        child(local = "start", variant = Start, ty = Border),
        child(local = "left", variant = Left, ty = Border),
        child(local = "bottom", variant = Bottom, ty = Border),
        child(local = "end", variant = End, ty = Border),
        child(local = "right", variant = Right, ty = Border),
        child(local = "insideH", variant = InsideHorizontal, ty = Border),
        child(local = "insideV", variant = InsideVertical, ty = Border)
    )]
    content: Vec<TableBorderContent>,
}

impl TableBorders {
    /// Builds a new, empty `w:tblBorders`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "tblBorders"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &TableBorderContent) -> Option<u16> {
        let local = match item {
            TableBorderContent::Top(_) => "top",
            TableBorderContent::Start(_) => "start",
            TableBorderContent::Left(_) => "left",
            TableBorderContent::Bottom(_) => "bottom",
            TableBorderContent::End(_) => "end",
            TableBorderContent::Right(_) => "right",
            TableBorderContent::InsideHorizontal(_) => "insideH",
            TableBorderContent::InsideVertical(_) => "insideV",
            TableBorderContent::Raw(_) => return None,
        };
        TABLE_BORDERS.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&TableBorderContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: TableBorderContent) {
        let at = TABLE_BORDERS.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&TableBorderContent) -> bool,
        value: Option<TableBorderContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    border_property!(TableBorderContent, top, set_top, Top, "top", "`w:top`.");
    border_property!(
        TableBorderContent,
        start,
        set_start,
        Start,
        "start",
        "`w:start`."
    );
    border_property!(
        TableBorderContent,
        left,
        set_left,
        Left,
        "left",
        "`w:left`."
    );
    border_property!(
        TableBorderContent,
        bottom,
        set_bottom,
        Bottom,
        "bottom",
        "`w:bottom`."
    );
    border_property!(TableBorderContent, end, set_end, End, "end", "`w:end`.");
    border_property!(
        TableBorderContent,
        right,
        set_right,
        Right,
        "right",
        "`w:right`."
    );
    border_property!(
        TableBorderContent,
        inside_horizontal,
        set_inside_horizontal,
        InsideHorizontal,
        "insideH",
        "`w:insideH` — the borders between rows."
    );
    border_property!(
        TableBorderContent,
        inside_vertical,
        set_inside_vertical,
        InsideVertical,
        "insideV",
        "`w:insideV` — the borders between columns."
    );
}

/// One ordered child of [`CellBorders`]: `CT_TcBorders`'s sequence is [`TableBorderContent`]'s eight
/// plus the two diagonals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellBorderContent {
    /// `w:top`.
    Top(Border),
    /// `w:start`.
    Start(Border),
    /// `w:left`.
    Left(Border),
    /// `w:bottom`.
    Bottom(Border),
    /// `w:end`.
    End(Border),
    /// `w:right`.
    Right(Border),
    /// `w:insideH`.
    InsideHorizontal(Border),
    /// `w:insideV`.
    InsideVertical(Border),
    /// `w:tl2br` — the diagonal from top-left to bottom-right.
    TopLeftToBottomRight(Border),
    /// `w:tr2bl` — the diagonal from top-right to bottom-left.
    TopRightToBottomLeft(Border),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_TcBorders` (`w:tcBorders`, "Table Cell Borders", §17.4.66) — a cell's own borders (or a table
/// style's `w:tcPr`), [`TableBorders`]'s eight plus the two diagonals.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct CellBorders {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "top", variant = Top, ty = Border),
        child(local = "start", variant = Start, ty = Border),
        child(local = "left", variant = Left, ty = Border),
        child(local = "bottom", variant = Bottom, ty = Border),
        child(local = "end", variant = End, ty = Border),
        child(local = "right", variant = Right, ty = Border),
        child(local = "insideH", variant = InsideHorizontal, ty = Border),
        child(local = "insideV", variant = InsideVertical, ty = Border),
        child(local = "tl2br", variant = TopLeftToBottomRight, ty = Border),
        child(local = "tr2bl", variant = TopRightToBottomLeft, ty = Border)
    )]
    content: Vec<CellBorderContent>,
}

impl CellBorders {
    /// Builds a new, empty `w:tcBorders`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "tcBorders"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &CellBorderContent) -> Option<u16> {
        let local = match item {
            CellBorderContent::Top(_) => "top",
            CellBorderContent::Start(_) => "start",
            CellBorderContent::Left(_) => "left",
            CellBorderContent::Bottom(_) => "bottom",
            CellBorderContent::End(_) => "end",
            CellBorderContent::Right(_) => "right",
            CellBorderContent::InsideHorizontal(_) => "insideH",
            CellBorderContent::InsideVertical(_) => "insideV",
            CellBorderContent::TopLeftToBottomRight(_) => "tl2br",
            CellBorderContent::TopRightToBottomLeft(_) => "tr2bl",
            CellBorderContent::Raw(_) => return None,
        };
        CELL_BORDERS.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&CellBorderContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: CellBorderContent) {
        let at = CELL_BORDERS.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&CellBorderContent) -> bool,
        value: Option<CellBorderContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    border_property!(CellBorderContent, top, set_top, Top, "top", "`w:top`.");
    border_property!(
        CellBorderContent,
        start,
        set_start,
        Start,
        "start",
        "`w:start`."
    );
    border_property!(CellBorderContent, left, set_left, Left, "left", "`w:left`.");
    border_property!(
        CellBorderContent,
        bottom,
        set_bottom,
        Bottom,
        "bottom",
        "`w:bottom`."
    );
    border_property!(CellBorderContent, end, set_end, End, "end", "`w:end`.");
    border_property!(
        CellBorderContent,
        right,
        set_right,
        Right,
        "right",
        "`w:right`."
    );
    border_property!(
        CellBorderContent,
        inside_horizontal,
        set_inside_horizontal,
        InsideHorizontal,
        "insideH",
        "`w:insideH`."
    );
    border_property!(
        CellBorderContent,
        inside_vertical,
        set_inside_vertical,
        InsideVertical,
        "insideV",
        "`w:insideV`."
    );
    border_property!(
        CellBorderContent,
        top_left_to_bottom_right,
        set_top_left_to_bottom_right,
        TopLeftToBottomRight,
        "tl2br",
        "`w:tl2br` — the diagonal from top-left to bottom-right."
    );
    border_property!(
        CellBorderContent,
        top_right_to_bottom_left,
        set_top_right_to_bottom_left,
        TopRightToBottomLeft,
        "tr2bl",
        "`w:tr2bl` — the diagonal from top-right to bottom-left."
    );
}

// -------------------------------------------------------------------------------------------
// CT_TblCellMar (w:tblCellMar) and CT_TcMar (w:tcMar) — six `TableWidth` sides each.
// -------------------------------------------------------------------------------------------

/// One ordered child of [`TableCellMargins`]/[`CellMargins`]: `CT_TblCellMar`/`CT_TcMar`'s sequence
/// is `top, start, left, bottom, end, right` — every one a [`TableWidth`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarginContent {
    /// `w:top`.
    Top(TableWidth),
    /// `w:start`.
    Start(TableWidth),
    /// `w:left`.
    Left(TableWidth),
    /// `w:bottom`.
    Bottom(TableWidth),
    /// `w:end`.
    End(TableWidth),
    /// `w:right`.
    Right(TableWidth),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_TblCellMar` (`w:tblCellMar`, "Table Cell Margin Defaults", §17.4.43) — a table's default cell
/// margins, overridable per cell by [`CellMargins`].
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct TableCellMargins {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "top", variant = Top, ty = TableWidth),
        child(local = "start", variant = Start, ty = TableWidth),
        child(local = "left", variant = Left, ty = TableWidth),
        child(local = "bottom", variant = Bottom, ty = TableWidth),
        child(local = "end", variant = End, ty = TableWidth),
        child(local = "right", variant = Right, ty = TableWidth)
    )]
    content: Vec<MarginContent>,
}

impl TableCellMargins {
    /// Builds a new, empty `w:tblCellMar`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "tblCellMar"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &MarginContent) -> Option<u16> {
        let local = match item {
            MarginContent::Top(_) => "top",
            MarginContent::Start(_) => "start",
            MarginContent::Left(_) => "left",
            MarginContent::Bottom(_) => "bottom",
            MarginContent::End(_) => "end",
            MarginContent::Right(_) => "right",
            MarginContent::Raw(_) => return None,
        };
        TABLE_CELL_MARGINS.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&MarginContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: MarginContent) {
        let at =
            TABLE_CELL_MARGINS.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&MarginContent) -> bool,
        value: Option<MarginContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    table_width_property!(MarginContent, top, set_top, Top, "top", "`w:top`.");
    table_width_property!(
        MarginContent,
        start,
        set_start,
        Start,
        "start",
        "`w:start`."
    );
    table_width_property!(MarginContent, left, set_left, Left, "left", "`w:left`.");
    table_width_property!(
        MarginContent,
        bottom,
        set_bottom,
        Bottom,
        "bottom",
        "`w:bottom`."
    );
    table_width_property!(MarginContent, end, set_end, End, "end", "`w:end`.");
    table_width_property!(
        MarginContent,
        right,
        set_right,
        Right,
        "right",
        "`w:right`."
    );
}

/// `CT_TcMar` (`w:tcMar`, "Single Table Cell Margins", §17.4.68) — one cell's own margins, overriding
/// [`TableCellMargins`] for that cell alone.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct CellMargins {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "top", variant = Top, ty = TableWidth),
        child(local = "start", variant = Start, ty = TableWidth),
        child(local = "left", variant = Left, ty = TableWidth),
        child(local = "bottom", variant = Bottom, ty = TableWidth),
        child(local = "end", variant = End, ty = TableWidth),
        child(local = "right", variant = Right, ty = TableWidth)
    )]
    content: Vec<MarginContent>,
}

impl CellMargins {
    /// Builds a new, empty `w:tcMar`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "tcMar"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &MarginContent) -> Option<u16> {
        let local = match item {
            MarginContent::Top(_) => "top",
            MarginContent::Start(_) => "start",
            MarginContent::Left(_) => "left",
            MarginContent::Bottom(_) => "bottom",
            MarginContent::End(_) => "end",
            MarginContent::Right(_) => "right",
            MarginContent::Raw(_) => return None,
        };
        CELL_MARGINS.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&MarginContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: MarginContent) {
        let at = CELL_MARGINS.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&MarginContent) -> bool,
        value: Option<MarginContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    table_width_property!(MarginContent, top, set_top, Top, "top", "`w:top`.");
    table_width_property!(
        MarginContent,
        start,
        set_start,
        Start,
        "start",
        "`w:start`."
    );
    table_width_property!(MarginContent, left, set_left, Left, "left", "`w:left`.");
    table_width_property!(
        MarginContent,
        bottom,
        set_bottom,
        Bottom,
        "bottom",
        "`w:bottom`."
    );
    table_width_property!(MarginContent, end, set_end, End, "end", "`w:end`.");
    table_width_property!(
        MarginContent,
        right,
        set_right,
        Right,
        "right",
        "`w:right`."
    );
}

// -------------------------------------------------------------------------------------------
// CT_Height (w:trHeight)
// -------------------------------------------------------------------------------------------

/// `CT_Height` (`w:trHeight`, "Table Row Height", §17.4.80) — a row's preferred height and the rule
/// (`ST_HeightRule`) governing whether it is exact, a minimum, or automatic.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Twips, accessor = height))]
#[xml(attribute(local = "hRule", prefix = "w", codec = Enumeration<HeightRule>, accessor = rule))]
pub struct RowHeight {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl RowHeight {
    /// A fresh, empty `w:trHeight` — both attributes absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "trHeight"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for RowHeight {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for RowHeight {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_TextDirection (w:textDirection, inside w:tcPr) and CT_VerticalJc (w:vAlign)
// -------------------------------------------------------------------------------------------

/// `CT_TextDirection` (`w:textDirection`, inside `w:tcPr`, "Table Cell Text Direction", §17.4.75) — a
/// cell's own text flow direction. A distinct type from
/// [`super::paragraph_properties::ParagraphTextFlowDirection`] for the same reason that type is
/// distinct from a bare `TextFlowDirection`: the two live in different containers even though the
/// wire shape (one optional `val`) coincides.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<TextFlowDirection>, accessor = value))]
pub struct CellTextDirection {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl CellTextDirection {
    /// Builds a new `w:textDirection`; `value` is written only when given.
    #[must_use]
    pub fn new(interner: &mut Interner, value: Option<TextFlowDirection>) -> Self {
        let mut item = Self {
            name: wml_name(interner, "textDirection"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for CellTextDirection {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for CellTextDirection {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_VerticalJc` (`w:vAlign`, inside `w:tcPr`, "Table Cell Vertical Alignment", §17.4.83) — a
/// required vertical alignment for a cell's content.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<VerticalJustification>, accessor = value, required))]
pub struct CellVerticalAlignment {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl CellVerticalAlignment {
    /// Builds a new `w:vAlign` of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, value: VerticalJustification) -> Self {
        let mut item = Self {
            name: wml_name(interner, "vAlign"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for CellVerticalAlignment {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for CellVerticalAlignment {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_Headers (w:headers) — a cell's `w:headers/w:header*` accessibility references.
// -------------------------------------------------------------------------------------------

/// `CT_Headers` (`w:headers`, "Table Header Cell References", §17.4.28) — a data cell's references
/// to the header cells that describe it, each a [`TableStringValue`] (`w:header`, `CT_String`). The
/// list is a plain repeatable sequence (no other element type ever appears here), so — unlike this
/// module's rank-ordered containers — insertion is always a straightforward push/remove; no schema
/// child-order table is needed.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct CellHeaderReferences {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "header", variant = Header, ty = TableStringValue))]
    content: Vec<HeaderReferenceContent>,
}

/// [`CellHeaderReferences`]'s one child shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderReferenceContent {
    /// `w:header` (`CT_String`).
    Header(TableStringValue),
    /// Any other child — preserved verbatim (`CT_Headers` names none, but a foreign extension is
    /// still round-tripped).
    Raw(RawNode),
}

impl CellHeaderReferences {
    /// Builds a new `w:headers` naming `ids`, in order.
    #[must_use]
    pub fn new(interner: &mut Interner, ids: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let content = ids
            .into_iter()
            .map(|id| {
                HeaderReferenceContent::Header(TableStringValue::new(
                    interner,
                    "header",
                    id.as_ref(),
                ))
            })
            .collect::<Vec<_>>();
        Self {
            name: wml_name(interner, "headers"),
            attributes: Vec::new(),
            empty: content.is_empty(),
            content,
        }
    }

    /// The header-cell ids this cell names, in order (opaque children skipped).
    pub fn ids(&self) -> impl Iterator<Item = &TableStringValue> {
        self.content.iter().filter_map(|item| match item {
            HeaderReferenceContent::Header(value) => Some(value),
            _ => None,
        })
    }

    /// Appends a new `w:header` naming `id`.
    pub fn push(&mut self, interner: &mut Interner, id: &str) {
        self.content
            .push(HeaderReferenceContent::Header(TableStringValue::new(
                interner, "header", id,
            )));
        self.empty = false;
    }
}

// -------------------------------------------------------------------------------------------
// CT_TblPrBase (w:tblPr) — the table-level properties, shared verbatim with
// `w:tblStylePr/w:tblPr` (CT_TblPrBase exactly, per `wml.xsd`).
// -------------------------------------------------------------------------------------------

/// One ordered child of [`TableProperties`]: `CT_TblPrBase`'s own seventeen members, or an opaque
/// node — `w:tblPrChange` (`CT_TblPr`'s own extension, revision tracking, MJXOFF-126's scope) falls
/// to [`TablePropertyContent::Raw`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TablePropertyContent {
    /// `w:tblStyle` (`CT_String`) — the table style this table references.
    Style(TableStringValue),
    /// `w:tblpPr` — floating-table position.
    FloatingPosition(FloatingTablePosition),
    /// `w:tblOverlap`.
    Overlap(FloatingTableOverlap),
    /// `w:bidiVisual`.
    BidiVisual(Toggle),
    /// `w:tblStyleRowBandSize` (`CT_DecimalNumber`) — rows per row band; ECMA-376 Part 1 §17.7.6.7's
    /// own default of `1` applies when absent.
    RowBandSize(DecimalNumberValue),
    /// `w:tblStyleColBandSize` (`CT_DecimalNumber`) — columns per column band; §17.7.6.5's own
    /// default of `1` applies when absent.
    ColumnBandSize(DecimalNumberValue),
    /// `w:tblW` — the table's preferred width.
    Width(TableWidth),
    /// `w:jc` — the table's own alignment on the page.
    Justification(TableAlignment),
    /// `w:tblCellSpacing`.
    CellSpacing(TableWidth),
    /// `w:tblInd`.
    Indent(TableWidth),
    /// `w:tblBorders`.
    Borders(TableBorders),
    /// `w:shd`.
    Shading(Shading),
    /// `w:tblLayout`.
    Layout(TableLayout),
    /// `w:tblCellMar`.
    CellMargins(TableCellMargins),
    /// `w:tblLook`.
    Look(TableLook),
    /// `w:tblCaption`.
    Caption(TableStringValue),
    /// `w:tblDescription`.
    Description(TableStringValue),
    /// `w:tblPrChange` (`CT_TblPrChange`) — the tracked-change wrapper around a previous `w:tblPr`.
    /// `CT_TblPrBase`'s own sequence has no member of this name — it is `CT_TblPr`'s own trailing
    /// extension — so it is always placed last (see `TableProperties::rank`'s own doc comment).
    Change(super::revisions::TablePropertiesChange),
    /// Any other child — preserved verbatim, in position.
    Raw(RawNode),
}

/// `CT_TblPrBase` (`w:tblPr`, "Table Properties", §17.4.60, minus its `w:tblPrChange` extension) — a
/// table's own formatting properties. Reused **directly** for `w:style/w:tblStylePr/w:tblPr`
/// ([`super::styles::TableStyleOverride::table_properties`]), which is literally this same complex
/// type — not a parallel model.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct TableProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "tblStyle", variant = Style, ty = TableStringValue),
        child(local = "tblpPr", variant = FloatingPosition, ty = FloatingTablePosition),
        child(local = "tblOverlap", variant = Overlap, ty = FloatingTableOverlap),
        child(local = "bidiVisual", variant = BidiVisual, ty = Toggle),
        child(local = "tblStyleRowBandSize", variant = RowBandSize, ty = DecimalNumberValue),
        child(local = "tblStyleColBandSize", variant = ColumnBandSize, ty = DecimalNumberValue),
        child(local = "tblW", variant = Width, ty = TableWidth),
        child(local = "jc", variant = Justification, ty = TableAlignment),
        child(local = "tblCellSpacing", variant = CellSpacing, ty = TableWidth),
        child(local = "tblInd", variant = Indent, ty = TableWidth),
        child(local = "tblBorders", variant = Borders, ty = TableBorders),
        child(local = "shd", variant = Shading, ty = Shading),
        child(local = "tblLayout", variant = Layout, ty = TableLayout),
        child(local = "tblCellMar", variant = CellMargins, ty = TableCellMargins),
        child(local = "tblLook", variant = Look, ty = TableLook),
        child(local = "tblCaption", variant = Caption, ty = TableStringValue),
        child(local = "tblDescription", variant = Description, ty = TableStringValue),
        child(local = "tblPrChange", variant = Change, ty = super::revisions::TablePropertiesChange)
    )]
    content: Vec<TablePropertyContent>,
}

impl TableProperties {
    /// Builds a new, empty `w:tblPr`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "tblPr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The content, verbatim — MJXOFF-126's `w:tblPrChange` reader reaches it this way.
    #[must_use]
    pub fn content(&self) -> &[TablePropertyContent] {
        &self.content
    }

    fn rank(item: &TablePropertyContent) -> Option<u16> {
        let local = match item {
            TablePropertyContent::Style(_) => "tblStyle",
            TablePropertyContent::FloatingPosition(_) => "tblpPr",
            TablePropertyContent::Overlap(_) => "tblOverlap",
            TablePropertyContent::BidiVisual(_) => "bidiVisual",
            TablePropertyContent::RowBandSize(_) => "tblStyleRowBandSize",
            TablePropertyContent::ColumnBandSize(_) => "tblStyleColBandSize",
            TablePropertyContent::Width(_) => "tblW",
            TablePropertyContent::Justification(_) => "jc",
            TablePropertyContent::CellSpacing(_) => "tblCellSpacing",
            TablePropertyContent::Indent(_) => "tblInd",
            TablePropertyContent::Borders(_) => "tblBorders",
            TablePropertyContent::Shading(_) => "shd",
            TablePropertyContent::Layout(_) => "tblLayout",
            TablePropertyContent::CellMargins(_) => "tblCellMar",
            TablePropertyContent::Look(_) => "tblLook",
            TablePropertyContent::Caption(_) => "tblCaption",
            TablePropertyContent::Description(_) => "tblDescription",
            // `w:tblPrChange` is `CT_TblPr`'s own trailing extension over `CT_TblPrBase`
            // (`TABLE_PROPERTIES_BASE` has no such member — confirmed directly against `wml.xsd`,
            // where `tblPrChange` is the sole, always-last child `CT_TblPr` adds), so it is treated
            // exactly like `Raw`: unranked, which `insert`'s own `ChildOrder::insert_index_of_names`
            // (see that method's own doc comment) resolves by skipping it during the ranking scan
            // and appending a *new* item at the physical end when `local` itself does not resolve —
            // together, this always keeps `w:tblPrChange` last, which is the only legal position.
            TablePropertyContent::Change(_) | TablePropertyContent::Raw(_) => return None,
        };
        TABLE_PROPERTIES_BASE.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&TablePropertyContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: TablePropertyContent) {
        let at =
            TABLE_PROPERTIES_BASE.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&TablePropertyContent) -> bool,
        value: Option<TablePropertyContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    /// The tracked-change wrapper around a previous `w:tblPr` (`w:tblPrChange`), or `None` if this
    /// `w:tblPr` carries none. Real only on a live `w:tbl/w:tblPr`; schema-illegal (but harmlessly
    /// typed, never authored by this crate there) on `w:tblStylePr/w:tblPr` — see
    /// [`TablePropertyContent::Change`]'s own doc comment.
    #[must_use]
    pub fn change(&self) -> Option<&super::revisions::TablePropertiesChange> {
        self.content.iter().find_map(|item| match item {
            TablePropertyContent::Change(change) => Some(change),
            _ => None,
        })
    }

    /// [`TableProperties::change`], mutably.
    pub fn change_mut(&mut self) -> Option<&mut super::revisions::TablePropertiesChange> {
        self.content.iter_mut().find_map(|item| match item {
            TablePropertyContent::Change(change) => Some(change),
            _ => None,
        })
    }

    /// The table style this table references (`w:tblStyle/@val`), or `None`.
    ///
    /// # Errors
    /// An [`AttributeError`] if `w:tblStyle/@val` is present but malformed.
    pub fn style_id(&self, interner: &Interner) -> Result<Option<String>, AttributeError> {
        self.content
            .iter()
            .find_map(|item| match item {
                TablePropertyContent::Style(value) => Some(value),
                _ => None,
            })
            .map(|value| value.value(interner).map(Cow::into_owned))
            .transpose()
    }

    /// Sets (or, given `None`, removes) `w:tblStyle`.
    pub fn set_style_id(&mut self, interner: &mut Interner, style_id: Option<&str>) {
        let is_target =
            |item: &TablePropertyContent| matches!(item, TablePropertyContent::Style(_));
        match style_id {
            None => self.remove(is_target),
            Some(style_id) => {
                let value = TableStringValue::new(interner, "tblStyle", style_id);
                self.set(
                    "tblStyle",
                    is_target,
                    Some(TablePropertyContent::Style(value)),
                );
            }
        }
    }

    value_property!(
        TablePropertyContent,
        floating_position,
        set_floating_position,
        FloatingPosition,
        FloatingTablePosition,
        "tblpPr",
        "`w:tblpPr` — this table's floating position, when it is not inline."
    );
    value_property!(
        TablePropertyContent,
        overlap,
        set_overlap,
        Overlap,
        FloatingTableOverlap,
        "tblOverlap",
        "`w:tblOverlap` — whether other floating tables may overlap this one."
    );
    toggle_property!(
        TablePropertyContent,
        bidi_visual,
        set_bidi_visual,
        BidiVisual,
        "bidiVisual",
        "`w:bidiVisual` — whether this table's columns present right-to-left."
    );
    decimal_number_property!(
        TablePropertyContent,
        row_band_size,
        set_row_band_size,
        RowBandSize,
        "tblStyleRowBandSize",
        "`w:tblStyleRowBandSize` — rows per row band; the effective size is [`Self::effective_row_band_size`]."
    );
    decimal_number_property!(
        TablePropertyContent,
        column_band_size,
        set_column_band_size,
        ColumnBandSize,
        "tblStyleColBandSize",
        "`w:tblStyleColBandSize` — columns per column band; the effective size is [`Self::effective_column_band_size`]."
    );

    /// `w:tblStyleRowBandSize`'s stated value, or ECMA-376 Part 1 §17.7.6.7's own default of `1` when
    /// absent, unreadable, or not positive.
    ///
    /// # Errors
    /// An [`AttributeError`] if `w:tblStyleRowBandSize/@w:val` is present but malformed.
    pub fn effective_row_band_size(&self, interner: &Interner) -> Result<usize, AttributeError> {
        Ok(self
            .row_band_size(interner)?
            .filter(|value| *value > 0)
            .map_or(1, |value| value as usize))
    }

    /// `w:tblStyleColBandSize`'s stated value, or ECMA-376 Part 1 §17.7.6.5's own default of `1` when
    /// absent, unreadable, or not positive.
    ///
    /// # Errors
    /// An [`AttributeError`] if `w:tblStyleColBandSize/@w:val` is present but malformed.
    pub fn effective_column_band_size(&self, interner: &Interner) -> Result<usize, AttributeError> {
        Ok(self
            .column_band_size(interner)?
            .filter(|value| *value > 0)
            .map_or(1, |value| value as usize))
    }

    table_width_property!(
        TablePropertyContent,
        width,
        set_width,
        Width,
        "tblW",
        "`w:tblW` — the table's preferred width."
    );
    value_property!(
        TablePropertyContent,
        justification,
        set_justification,
        Justification,
        TableAlignment,
        "jc",
        "`w:jc` — the table's own alignment on the page."
    );
    table_width_property!(
        TablePropertyContent,
        cell_spacing,
        set_cell_spacing,
        CellSpacing,
        "tblCellSpacing",
        "`w:tblCellSpacing`."
    );
    table_width_property!(
        TablePropertyContent,
        indent,
        set_indent,
        Indent,
        "tblInd",
        "`w:tblInd`."
    );
    value_property!(
        TablePropertyContent,
        borders,
        set_borders,
        Borders,
        TableBorders,
        "tblBorders",
        "`w:tblBorders`."
    );
    value_property!(
        TablePropertyContent,
        shading,
        set_shading,
        Shading,
        Shading,
        "shd",
        "`w:shd` — the table's own background shading."
    );
    value_property!(
        TablePropertyContent,
        layout,
        set_layout,
        Layout,
        TableLayout,
        "tblLayout",
        "`w:tblLayout` — `fixed` versus `autofit`."
    );
    value_property!(
        TablePropertyContent,
        cell_margins,
        set_cell_margins,
        CellMargins,
        TableCellMargins,
        "tblCellMar",
        "`w:tblCellMar` — this table's default cell margins."
    );
    value_property!(
        TablePropertyContent,
        look,
        set_look,
        Look,
        TableLook,
        "tblLook",
        "`w:tblLook` — which of the table style's conditional formats this table applies."
    );

    /// This table's caption (`w:tblCaption/@val`), or `None`.
    #[must_use]
    pub fn caption(&self) -> Option<&TableStringValue> {
        self.content.iter().find_map(|item| match item {
            TablePropertyContent::Caption(value) => Some(value),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:tblCaption`.
    pub fn set_caption(&mut self, interner: &mut Interner, caption: Option<&str>) {
        let is_target =
            |item: &TablePropertyContent| matches!(item, TablePropertyContent::Caption(_));
        match caption {
            None => self.remove(is_target),
            Some(caption) => {
                let value = TableStringValue::new(interner, "tblCaption", caption);
                self.set(
                    "tblCaption",
                    is_target,
                    Some(TablePropertyContent::Caption(value)),
                );
            }
        }
    }

    /// This table's description (`w:tblDescription/@val`), or `None`.
    #[must_use]
    pub fn description(&self) -> Option<&TableStringValue> {
        self.content.iter().find_map(|item| match item {
            TablePropertyContent::Description(value) => Some(value),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:tblDescription`.
    pub fn set_description(&mut self, interner: &mut Interner, description: Option<&str>) {
        let is_target =
            |item: &TablePropertyContent| matches!(item, TablePropertyContent::Description(_));
        match description {
            None => self.remove(is_target),
            Some(description) => {
                let value = TableStringValue::new(interner, "tblDescription", description);
                self.set(
                    "tblDescription",
                    is_target,
                    Some(TablePropertyContent::Description(value)),
                );
            }
        }
    }
}

// -------------------------------------------------------------------------------------------
// CT_TblPrExBase (w:tblPrEx) — the row-level exception set, a subset of CT_TblPrBase's members.
// -------------------------------------------------------------------------------------------

/// One ordered child of [`TableExceptionProperties`]: `CT_TblPrExBase`'s own nine members, or an
/// opaque node (`w:tblPrExChange`, MJXOFF-126's scope).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableExceptionPropertyContent {
    /// `w:tblW`.
    Width(TableWidth),
    /// `w:jc`.
    Justification(TableAlignment),
    /// `w:tblCellSpacing`.
    CellSpacing(TableWidth),
    /// `w:tblInd`.
    Indent(TableWidth),
    /// `w:tblBorders`.
    Borders(TableBorders),
    /// `w:shd`.
    Shading(Shading),
    /// `w:tblLayout`.
    Layout(TableLayout),
    /// `w:tblCellMar`.
    CellMargins(TableCellMargins),
    /// `w:tblLook`.
    Look(TableLook),
    /// `w:tblPrExChange` (`CT_TblPrExChange`) — the tracked-change wrapper around a previous
    /// `w:tblPrEx`. Always last (see `TableProperties::rank`'s own doc comment — the identical
    /// "trailing extension member" reasoning applies here).
    Change(super::revisions::TableExceptionPropertiesChange),
    /// Any other child — preserved verbatim, in position.
    Raw(RawNode),
}

/// `CT_TblPrExBase` (`w:tblPrEx`, "Table Properties Exceptions", §17.4.61, minus its
/// `w:tblPrExChange` extension) — a row's own override of its table's properties. See this module's
/// own doc comment: a row assembled from a differently-formatted original states this so its own
/// properties win for that row alone, without touching the table's `w:tblPr`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct TableExceptionProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "tblW", variant = Width, ty = TableWidth),
        child(local = "jc", variant = Justification, ty = TableAlignment),
        child(local = "tblCellSpacing", variant = CellSpacing, ty = TableWidth),
        child(local = "tblInd", variant = Indent, ty = TableWidth),
        child(local = "tblBorders", variant = Borders, ty = TableBorders),
        child(local = "shd", variant = Shading, ty = Shading),
        child(local = "tblLayout", variant = Layout, ty = TableLayout),
        child(local = "tblCellMar", variant = CellMargins, ty = TableCellMargins),
        child(local = "tblLook", variant = Look, ty = TableLook),
        child(local = "tblPrExChange", variant = Change, ty = super::revisions::TableExceptionPropertiesChange)
    )]
    content: Vec<TableExceptionPropertyContent>,
}

impl TableExceptionProperties {
    /// Builds a new, empty `w:tblPrEx`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "tblPrEx"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Whether this `w:tblPrEx` states nothing at all — the "may as well not be here" state a caller
    /// removing its last stated member can leave behind.
    #[must_use]
    pub fn is_fully_empty(&self) -> bool {
        self.content.is_empty()
    }

    fn rank(item: &TableExceptionPropertyContent) -> Option<u16> {
        let local = match item {
            TableExceptionPropertyContent::Width(_) => "tblW",
            TableExceptionPropertyContent::Justification(_) => "jc",
            TableExceptionPropertyContent::CellSpacing(_) => "tblCellSpacing",
            TableExceptionPropertyContent::Indent(_) => "tblInd",
            TableExceptionPropertyContent::Borders(_) => "tblBorders",
            TableExceptionPropertyContent::Shading(_) => "shd",
            TableExceptionPropertyContent::Layout(_) => "tblLayout",
            TableExceptionPropertyContent::CellMargins(_) => "tblCellMar",
            TableExceptionPropertyContent::Look(_) => "tblLook",
            // `w:tblPrExChange` is `CT_TblPrEx`'s own trailing extension over `CT_TblPrExBase` —
            // see `TableProperties::rank`'s own doc comment for the identical reasoning.
            TableExceptionPropertyContent::Change(_) | TableExceptionPropertyContent::Raw(_) => {
                return None
            }
        };
        TABLE_EXCEPTION_PROPERTIES_BASE.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&TableExceptionPropertyContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: TableExceptionPropertyContent) {
        let at = TABLE_EXCEPTION_PROPERTIES_BASE
            .insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&TableExceptionPropertyContent) -> bool,
        value: Option<TableExceptionPropertyContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    table_width_property!(
        TableExceptionPropertyContent,
        width,
        set_width,
        Width,
        "tblW",
        "`w:tblW`."
    );
    value_property!(
        TableExceptionPropertyContent,
        justification,
        set_justification,
        Justification,
        TableAlignment,
        "jc",
        "`w:jc`."
    );
    table_width_property!(
        TableExceptionPropertyContent,
        cell_spacing,
        set_cell_spacing,
        CellSpacing,
        "tblCellSpacing",
        "`w:tblCellSpacing`."
    );
    table_width_property!(
        TableExceptionPropertyContent,
        indent,
        set_indent,
        Indent,
        "tblInd",
        "`w:tblInd`."
    );
    value_property!(
        TableExceptionPropertyContent,
        borders,
        set_borders,
        Borders,
        TableBorders,
        "tblBorders",
        "`w:tblBorders`."
    );
    value_property!(
        TableExceptionPropertyContent,
        shading,
        set_shading,
        Shading,
        Shading,
        "shd",
        "`w:shd`."
    );
    value_property!(
        TableExceptionPropertyContent,
        layout,
        set_layout,
        Layout,
        TableLayout,
        "tblLayout",
        "`w:tblLayout`."
    );
    value_property!(
        TableExceptionPropertyContent,
        cell_margins,
        set_cell_margins,
        CellMargins,
        TableCellMargins,
        "tblCellMar",
        "`w:tblCellMar`."
    );
    value_property!(
        TableExceptionPropertyContent,
        look,
        set_look,
        Look,
        TableLook,
        "tblLook",
        "`w:tblLook`."
    );

    /// The tracked-change wrapper around a previous `w:tblPrEx` (`w:tblPrExChange`), or `None` if
    /// this `w:tblPrEx` carries none.
    #[must_use]
    pub fn change(&self) -> Option<&super::revisions::TableExceptionPropertiesChange> {
        self.content.iter().find_map(|item| match item {
            TableExceptionPropertyContent::Change(change) => Some(change),
            _ => None,
        })
    }

    /// [`TableExceptionProperties::change`], mutably.
    pub fn change_mut(&mut self) -> Option<&mut super::revisions::TableExceptionPropertiesChange> {
        self.content.iter_mut().find_map(|item| match item {
            TableExceptionPropertyContent::Change(change) => Some(change),
            _ => None,
        })
    }
}

// -------------------------------------------------------------------------------------------
// CT_TrPrBase (w:trPr) — a row's own properties, shared verbatim with `w:tblStylePr/w:trPr`
// (literally `CT_TrPr` in both places, per `wml.xsd`).
// -------------------------------------------------------------------------------------------

/// One ordered child of [`RowProperties`]: `CT_TrPrBase`'s own twelve members, or an opaque node
/// (`w:ins`/`w:del`/`w:trPrChange`, `CT_TrPr`'s own extension — MJXOFF-126's scope).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowPropertyContent {
    /// `w:cnfStyle` — reused directly from [`ConditionalFormatting`] (MJXOFF-106's own type for
    /// `w:pPr/w:cnfStyle`); the wire shape is `CT_Cnf` in both places.
    ConditionalFormatting(ConditionalFormatting),
    /// `w:divId`.
    DivisionId(DecimalNumberValue),
    /// `w:gridBefore` — grid columns this row leaves empty before its first cell.
    GridBefore(DecimalNumberValue),
    /// `w:gridAfter` — grid columns this row leaves empty after its last cell.
    GridAfter(DecimalNumberValue),
    /// `w:wBefore` — the width of the empty space `w:gridBefore` names.
    WidthBefore(TableWidth),
    /// `w:wAfter` — the width of the empty space `w:gridAfter` names.
    WidthAfter(TableWidth),
    /// `w:cantSplit`.
    CantSplit(Toggle),
    /// `w:trHeight`.
    Height(RowHeight),
    /// `w:tblHeader` — whether this row repeats as a heading on every page the table spans.
    TableHeader(Toggle),
    /// `w:tblCellSpacing`.
    CellSpacing(TableWidth),
    /// `w:jc`.
    Justification(TableAlignment),
    /// `w:hidden`.
    Hidden(Toggle),
    /// `w:ins` (`CT_TrackChange`) — marks the whole row as tracked-inserted. `CT_TrPr`'s own
    /// trailing extension over `CT_TrPrBase`, always ordered before [`RowPropertyContent::Deleted`]/
    /// [`RowPropertyContent::Change`] (see `RowProperties::insert_trailing`'s own doc comment).
    Inserted(super::revisions::TrackChangeMarker),
    /// `w:del` (`CT_TrackChange`) — marks the whole row as tracked-deleted.
    Deleted(super::revisions::TrackChangeMarker),
    /// `w:trPrChange` (`CT_TrPrChange`) — the tracked-change wrapper around a previous `w:trPr`.
    Change(super::revisions::RowPropertiesChange),
    /// Any other child — preserved verbatim, in position.
    Raw(RawNode),
}

/// `CT_TrPrBase` (`w:trPr`, "Table Row Properties", §17.4.81, minus `CT_TrPr`'s own `w:ins`/`w:del`/
/// `w:trPrChange` extension) — a row's own formatting properties. Reused **directly** for
/// `w:style/w:tblStylePr/w:trPr` ([`super::styles::TableStyleOverride::row_properties`]).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct RowProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "cnfStyle", variant = ConditionalFormatting, ty = ConditionalFormatting),
        child(local = "divId", variant = DivisionId, ty = DecimalNumberValue),
        child(local = "gridBefore", variant = GridBefore, ty = DecimalNumberValue),
        child(local = "gridAfter", variant = GridAfter, ty = DecimalNumberValue),
        child(local = "wBefore", variant = WidthBefore, ty = TableWidth),
        child(local = "wAfter", variant = WidthAfter, ty = TableWidth),
        child(local = "cantSplit", variant = CantSplit, ty = Toggle),
        child(local = "trHeight", variant = Height, ty = RowHeight),
        child(local = "tblHeader", variant = TableHeader, ty = Toggle),
        child(local = "tblCellSpacing", variant = CellSpacing, ty = TableWidth),
        child(local = "jc", variant = Justification, ty = TableAlignment),
        child(local = "hidden", variant = Hidden, ty = Toggle),
        child(local = "ins", variant = Inserted, ty = super::revisions::TrackChangeMarker),
        child(local = "del", variant = Deleted, ty = super::revisions::TrackChangeMarker),
        child(local = "trPrChange", variant = Change, ty = super::revisions::RowPropertiesChange)
    )]
    content: Vec<RowPropertyContent>,
}

impl RowProperties {
    /// Builds a new, empty `w:trPr`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "trPr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Whether this `w:trPr` states nothing at all.
    #[must_use]
    pub fn is_fully_empty(&self) -> bool {
        self.content.is_empty()
    }

    fn rank(item: &RowPropertyContent) -> Option<u16> {
        let local = match item {
            RowPropertyContent::ConditionalFormatting(_) => "cnfStyle",
            RowPropertyContent::DivisionId(_) => "divId",
            RowPropertyContent::GridBefore(_) => "gridBefore",
            RowPropertyContent::GridAfter(_) => "gridAfter",
            RowPropertyContent::WidthBefore(_) => "wBefore",
            RowPropertyContent::WidthAfter(_) => "wAfter",
            RowPropertyContent::CantSplit(_) => "cantSplit",
            RowPropertyContent::Height(_) => "trHeight",
            RowPropertyContent::TableHeader(_) => "tblHeader",
            RowPropertyContent::CellSpacing(_) => "tblCellSpacing",
            RowPropertyContent::Justification(_) => "jc",
            RowPropertyContent::Hidden(_) => "hidden",
            // `ins`/`del`/`trPrChange` are `CT_TrPr`'s own trailing extension over `CT_TrPrBase`
            // (`TABLE_ROW_PROPERTIES_BASE` has no member of any of these three names) — unranked
            // here for the same reason `TableProperties::rank` treats `w:tblPrChange` as unranked,
            // but placing *these three* correctly relative to each other additionally needs
            // [`RowProperties::insert_trailing`], since there are three of them with their own fixed
            // relative order, not one.
            RowPropertyContent::Inserted(_)
            | RowPropertyContent::Deleted(_)
            | RowPropertyContent::Change(_)
            | RowPropertyContent::Raw(_) => return None,
        };
        TABLE_ROW_PROPERTIES_BASE.rank_of(None, local)
    }

    /// The fixed relative order of `CT_TrPr`'s three trailing extension members — `ins, del,
    /// trPrChange`, confirmed directly against `wml.xsd`. `None` for every base member and `Raw`.
    fn trailing_rank(item: &RowPropertyContent) -> Option<u8> {
        match item {
            RowPropertyContent::Inserted(_) => Some(0),
            RowPropertyContent::Deleted(_) => Some(1),
            RowPropertyContent::Change(_) => Some(2),
            _ => None,
        }
    }

    fn remove(&mut self, is_target: impl Fn(&RowPropertyContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: RowPropertyContent) {
        let at = TABLE_ROW_PROPERTIES_BASE
            .insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    /// Inserts one of the three trailing extension members (`ins`/`del`/`trPrChange`) at its own
    /// fixed relative position among whichever of the other two are already present — unlike
    /// [`RowProperties::insert`], which (correctly, for every *base* member) always appends an
    /// unranked new item at the absolute end, which would put e.g. a freshly-set `w:ins` *after* an
    /// already-present `w:trPrChange`, violating `CT_TrPr`'s own fixed order. Finds the first
    /// existing trailing member whose own [`RowProperties::trailing_rank`] is `>=` the new item's —
    /// inserting immediately before it — or appends at the very end when none is found (also
    /// correct: nothing may legally follow these three).
    fn insert_trailing(&mut self, item: RowPropertyContent) {
        let priority = Self::trailing_rank(&item).unwrap_or_else(|| {
            unreachable!("insert_trailing is only called with a trailing variant")
        });
        let at = self
            .content
            .iter()
            .position(|existing| {
                Self::trailing_rank(existing).is_some_and(|other| other >= priority)
            })
            .unwrap_or(self.content.len());
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&RowPropertyContent) -> bool,
        value: Option<RowPropertyContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    fn set_trailing(
        &mut self,
        is_target: impl Fn(&RowPropertyContent) -> bool,
        value: Option<RowPropertyContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert_trailing(value);
        }
    }

    /// The row's own tracked-insertion marker (`w:ins`), or `None` if absent.
    #[must_use]
    pub fn inserted(&self) -> Option<&super::revisions::TrackChangeMarker> {
        self.content.iter().find_map(|item| match item {
            RowPropertyContent::Inserted(marker) => Some(marker),
            _ => None,
        })
    }

    /// The row's own tracked-deletion marker (`w:del`), or `None` if absent.
    #[must_use]
    pub fn deleted(&self) -> Option<&super::revisions::TrackChangeMarker> {
        self.content.iter().find_map(|item| match item {
            RowPropertyContent::Deleted(marker) => Some(marker),
            _ => None,
        })
    }

    /// The tracked-change wrapper around a previous `w:trPr` (`w:trPrChange`), or `None` if this
    /// `w:trPr` carries none.
    #[must_use]
    pub fn change(&self) -> Option<&super::revisions::RowPropertiesChange> {
        self.content.iter().find_map(|item| match item {
            RowPropertyContent::Change(change) => Some(change),
            _ => None,
        })
    }

    /// [`RowProperties::change`], mutably.
    pub fn change_mut(&mut self) -> Option<&mut super::revisions::RowPropertiesChange> {
        self.content.iter_mut().find_map(|item| match item {
            RowPropertyContent::Change(change) => Some(change),
            _ => None,
        })
    }

    /// Sets (or clears) this row's own tracked-insertion marker.
    pub fn set_inserted(&mut self, marker: Option<super::revisions::TrackChangeMarker>) {
        self.set_trailing(
            |item| matches!(item, RowPropertyContent::Inserted(_)),
            marker.map(RowPropertyContent::Inserted),
        );
    }

    /// Sets (or clears) this row's own tracked-deletion marker.
    pub fn set_deleted(&mut self, marker: Option<super::revisions::TrackChangeMarker>) {
        self.set_trailing(
            |item| matches!(item, RowPropertyContent::Deleted(_)),
            marker.map(RowPropertyContent::Deleted),
        );
    }

    value_property!(
        RowPropertyContent,
        conditional_formatting,
        set_conditional_formatting,
        ConditionalFormatting,
        ConditionalFormatting,
        "cnfStyle",
        "`w:cnfStyle` — this row's own conditional-formatting region flags."
    );
    decimal_number_property!(
        RowPropertyContent,
        division_id,
        set_division_id,
        DivisionId,
        "divId",
        "`w:divId`."
    );
    decimal_number_property!(
        RowPropertyContent,
        grid_before,
        set_grid_before,
        GridBefore,
        "gridBefore",
        "`w:gridBefore` — grid columns this row leaves empty before its first cell."
    );
    decimal_number_property!(
        RowPropertyContent,
        grid_after,
        set_grid_after,
        GridAfter,
        "gridAfter",
        "`w:gridAfter` — grid columns this row leaves empty after its last cell."
    );
    table_width_property!(
        RowPropertyContent,
        width_before,
        set_width_before,
        WidthBefore,
        "wBefore",
        "`w:wBefore`."
    );
    table_width_property!(
        RowPropertyContent,
        width_after,
        set_width_after,
        WidthAfter,
        "wAfter",
        "`w:wAfter`."
    );
    toggle_property!(
        RowPropertyContent,
        cant_split,
        set_cant_split,
        CantSplit,
        "cantSplit",
        "`w:cantSplit` — whether this row must not be split across pages."
    );
    value_property!(
        RowPropertyContent,
        height,
        set_height,
        Height,
        RowHeight,
        "trHeight",
        "`w:trHeight`."
    );
    toggle_property!(
        RowPropertyContent,
        table_header,
        set_table_header,
        TableHeader,
        "tblHeader",
        "`w:tblHeader` — whether this row repeats as a heading on every page the table spans."
    );
    table_width_property!(
        RowPropertyContent,
        cell_spacing,
        set_cell_spacing,
        CellSpacing,
        "tblCellSpacing",
        "`w:tblCellSpacing`."
    );
    value_property!(
        RowPropertyContent,
        justification,
        set_justification,
        Justification,
        TableAlignment,
        "jc",
        "`w:jc`."
    );
    toggle_property!(
        RowPropertyContent,
        hidden,
        set_hidden,
        Hidden,
        "hidden",
        "`w:hidden`."
    );
}
