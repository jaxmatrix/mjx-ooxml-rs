//! `xl/styles.xml`'s ninth slot: `tableStyles`, the table styles a workbook defines for itself — and
//! the 144 it does not, because they are Excel's.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_TableStyles` | 3689 | `x:tableStyles` (rank **8** of `CT_Stylesheet`) |
//! | `CT_TableStyle` | 3697 | `x:tableStyles/tableStyle` |
//! | `CT_TableStyleElement` | 3707 | `x:tableStyle/tableStyleElement` |
//!
//! # A built-in style name is not a missing style
//!
//! **This is the trap this module exists to keep out of the answer.** A table's
//! `tableStyleInfo@name` is very often `TableStyleMedium2` — and `TableStyleMedium2` is in no
//! `.xlsx` anywhere. Excel's presets are supplied by the *application*, not by the file, and
//! ECMA-376 Part 1 publishes all 144 of them as a separate artifact
//! (`OfficeOpenXML-SpreadsheetMLStyles/presetTableStyles.xml`, beside the schemas). §18.5.1.5 states
//! the consumer rule directly: *"If the style name does not correspond to the name of a table style
//! then the spreadsheet application should use default style."*
//!
//! So a lookup has **three** answers, not two, and [`TableStyleLookup`] carries all three:
//!
//! * **[`LocallyDefined`](TableStyleLookup::LocallyDefined)** — the workbook's own `tableStyles`
//!   defines it, and the definition is right there;
//! * **[`BuiltIn`](TableStyleLookup::BuiltIn)** — one of the 144 preset names. *Not in the file, and
//!   not missing.* Answering "style not found" here would be a confident wrong answer, which is the
//!   class of mistake this phase keeps refusing;
//! * **[`Undefined`](TableStyleLookup::Undefined)** — a name that is neither. That is the only case
//!   in which a consumer really does fall back to its default, and it is worth being able to say so.
//!
//! # Where the built-in names come from, and why they are six families rather than a list of 144
//!
//! `presetTableStyles.xml` names every preset as an *element*, and the 144 names fall into exactly
//! six contiguous families:
//!
//! | family | numbers |
//! |---|---|
//! | `TableStyleLight` | 1–21 |
//! | `TableStyleMedium` | 1–28 |
//! | `TableStyleDark` | 1–11 |
//! | `PivotStyleLight` | 1–28 |
//! | `PivotStyleMedium` | 1–28 |
//! | `PivotStyleDark` | 1–28 |
//!
//! Storing the bounds rather than the 144 strings is not a compression trick: the bounds are what the
//! artifact says, they were read off it rather than remembered, and a name outside them —
//! `TableStyleLight22`, `TableStyleMedium0` — is genuinely **not** a preset and must not be reported
//! as one. [`builtin_table_style_name`] is the parser, and its unit tests check every one of the 144
//! against the boundaries on both sides.
//!
//! This is the same discipline
//! [`builtin_cell_style_name`](crate::builtin_cell_style_name) follows for Annex G.2's built-in cell
//! styles: a table sourced from a published artifact, with the artifact named.
//!
//! # This module resolves no `dxf`
//!
//! A [`TableStyleRegion`] names a `@dxfId`, and that index is a **position** in
//! `xl/styles.xml`'s `dxfs`. Resolving it is [`DifferentialFormats`](crate::DifferentialFormats)'
//! job and appending to that table is
//! [`StylesheetPart::append_differential_format`](crate::StylesheetPart::append_differential_format)'s;
//! neither is repeated here, and nothing in this file inserts into, removes from or reorders the
//! `dxfs` table — an index handed out yesterday still names what it named.

use mjx_ooxml_core::{
    Enumeration, Interner, Number, RawAttribute, RawElement, RawName, RawNode, Text, ToXml,
};
use mjx_ooxml_types::spreadsheetml::TableStyleType;
use mjx_ooxml_types::support::OnOff;

use crate::leaf::attribute_bag;
use crate::worksheet::rebuild_element;

// -----------------------------------------------------------------------------------------------
// The preset table styles: six families, read off `presetTableStyles.xml`
// -----------------------------------------------------------------------------------------------

/// Which of the six preset families a built-in table style name belongs to.
///
/// The names are Excel's own — a table style and a *pivot* table style are separate galleries, which
/// is why `tableStyles` carries both `@defaultTableStyle` and `@defaultPivotStyle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltInTableStyleFamily {
    /// `TableStyleLight1` … `TableStyleLight21`.
    TableLight,
    /// `TableStyleMedium1` … `TableStyleMedium28`.
    TableMedium,
    /// `TableStyleDark1` … `TableStyleDark11`.
    TableDark,
    /// `PivotStyleLight1` … `PivotStyleLight28`.
    PivotLight,
    /// `PivotStyleMedium1` … `PivotStyleMedium28`.
    PivotMedium,
    /// `PivotStyleDark1` … `PivotStyleDark28`.
    PivotDark,
}

impl BuiltInTableStyleFamily {
    /// Every family, with its name prefix and the highest number it goes up to.
    ///
    /// Read off `OfficeOpenXML-SpreadsheetMLStyles/presetTableStyles.xml`, which ECMA-376 Part 1
    /// publishes beside the schemas: its 144 top-level elements are the 144 preset names, and they
    /// are contiguous from 1 within each family.
    const FAMILIES: &'static [(Self, &'static str, u32)] = &[
        // Longest prefixes are not an issue here — no family name is a prefix of another — but the
        // order is still the artifact's own grouping, table styles then pivot styles.
        (Self::TableLight, "TableStyleLight", 21),
        (Self::TableMedium, "TableStyleMedium", 28),
        (Self::TableDark, "TableStyleDark", 11),
        (Self::PivotLight, "PivotStyleLight", 28),
        (Self::PivotMedium, "PivotStyleMedium", 28),
        (Self::PivotDark, "PivotStyleDark", 28),
    ];

    /// The literal prefix every name in this family starts with.
    #[must_use]
    pub fn prefix(self) -> &'static str {
        Self::FAMILIES
            .iter()
            .find(|(family, _, _)| *family == self)
            .map_or("", |(_, prefix, _)| *prefix)
    }

    /// The highest number this family goes up to; the lowest is always 1.
    #[must_use]
    pub fn highest(self) -> u32 {
        Self::FAMILIES
            .iter()
            .find(|(family, _, _)| *family == self)
            .map_or(0, |(_, _, highest)| *highest)
    }

    /// Whether this family is one of the three *pivot* galleries.
    #[must_use]
    pub fn is_pivot(self) -> bool {
        matches!(self, Self::PivotLight | Self::PivotMedium | Self::PivotDark)
    }
}

/// One of ECMA-376's 144 preset table styles, by family and number.
///
/// Its [`Display`](core::fmt::Display) writes the name back exactly — `TableStyleMedium2` — so a
/// round trip through [`builtin_table_style_name`] is the identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BuiltInTableStyle {
    family: BuiltInTableStyleFamily,
    number: u32,
}

impl BuiltInTableStyle {
    /// Which gallery this style is from.
    #[must_use]
    pub fn family(self) -> BuiltInTableStyleFamily {
        self.family
    }

    /// Its number within that gallery, counting from 1.
    #[must_use]
    pub fn number(self) -> u32 {
        self.number
    }
}

impl core::fmt::Display for BuiltInTableStyle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}{}", self.family.prefix(), self.number)
    }
}

/// The preset table style `name` names, or `None` when it is not one of the 144.
///
/// `None` is not "no such style": a workbook is free to define a style of its own with any name at
/// all, and [`TableStyles::lookup`] is what puts the two questions together. See this module's own
/// documentation for why the distinction matters.
///
/// Leading zeroes are refused — `TableStyleMedium02` is not a name in the artifact — because
/// accepting one would make [`BuiltInTableStyle`]'s `Display` disagree with the string it was parsed
/// from, and a name that does not round-trip is a name this crate has invented.
#[must_use]
pub fn builtin_table_style_name(name: &str) -> Option<BuiltInTableStyle> {
    for (family, prefix, highest) in BuiltInTableStyleFamily::FAMILIES {
        let Some(digits) = name.strip_prefix(prefix) else {
            continue;
        };
        if digits.is_empty() || digits.starts_with('0') {
            return None;
        }
        let number = digits.parse::<u32>().ok()?;
        if number >= 1 && number <= *highest {
            return Some(BuiltInTableStyle {
                family: *family,
                number,
            });
        }
        return None;
    }
    None
}

/// What a workbook has to say about a table style name.
///
/// Returned by [`TableStyles::lookup`], and the reason a caller never has to guess whether a missing
/// definition is a defect. See this module's own documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableStyleLookup<'a> {
    /// The workbook defines this style itself, in `xl/styles.xml`'s `tableStyles`.
    ///
    /// A locally-defined style **shadows nothing**: a workbook may define `TableStyleMedium2` of its
    /// own, and this variant is then the right answer because the file's definition is what a
    /// consumer uses.
    LocallyDefined(&'a TableStyleDefinition),
    /// One of ECMA-376's 144 preset names, which no `.xlsx` contains. **Not missing.**
    BuiltIn(BuiltInTableStyle),
    /// Neither defined here nor a preset name — the one case in which §18.5.1.5's *"should use
    /// default style"* fallback applies.
    Undefined,
}

impl TableStyleLookup<'_> {
    /// The same three cases without the borrow — for a caller reporting across a part boundary, or
    /// across a language one.
    #[must_use]
    pub fn origin(&self) -> TableStyleOrigin {
        match self {
            Self::LocallyDefined(_) => TableStyleOrigin::LocallyDefined,
            Self::BuiltIn(_) => TableStyleOrigin::BuiltIn,
            Self::Undefined => TableStyleOrigin::Undefined,
        }
    }
}

/// [`TableStyleLookup`] with the definition dropped: three cases, `Copy`, no lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TableStyleOrigin {
    /// `xl/styles.xml` defines the style.
    LocallyDefined,
    /// It is one of ECMA-376's 144 presets, which live in the application rather than in the file.
    BuiltIn,
    /// Neither.
    Undefined,
}

// -----------------------------------------------------------------------------------------------
// `x:tableStyleElement` — one region of one style
// -----------------------------------------------------------------------------------------------

attribute_bag! {
    /// `x:tableStyleElement` (`CT_TableStyleElement`, `sml.xsd:3707`) — one region of a table style,
    /// and the differential format it paints there.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_TableStyleElement`. Wire element: `tableStyleElement`.
    ///
    /// `@type` is `ST_TableStyleType`, whose 28 values name the whole table, the header row, the
    /// totals row, the first and last column, the four stripes, the corner cells and the pivot-only
    /// regions. **Its generated variant names are read, never inferred** — the generator applies
    /// curated overrides — so a caller names
    /// [`TableStyleType`]'s variants rather than
    /// spelling a wire token.
    ///
    /// `@size` (default 1) is how many rows or columns wide the band is — a `firstRowStripe` with
    /// `size="2"` stripes two rows at a time. `@dxfId` is a **position** in `dxfs`; see this
    /// module's own documentation.
    #[xml(attribute(
        local = "type",
        codec = Enumeration<TableStyleType>,
        accessor = region,
        required
    ))]
    #[xml(attribute(local = "size", codec = Number<u32>, accessor = band_size, default = 1))]
    #[xml(attribute(local = "dxfId", codec = Number<u32>, accessor = differential_format_index))]
    TableStyleRegion, "tableStyleElement"
}

// -----------------------------------------------------------------------------------------------
// `x:tableStyle` — one style the workbook defines
// -----------------------------------------------------------------------------------------------

/// `x:tableStyle` (`CT_TableStyle`, `sml.xsd:3697`) — one table style this workbook defines for
/// itself.
///
/// **`ST_`/`CT_` symbol:** `CT_TableStyle`. Wire element: `tableStyle`.
///
/// `@name` is `use="required"` and is what a table's `tableStyleInfo@name` points at. `@pivot` and
/// `@table` both default to **`true`** and say which of the two galleries the style appears in — a
/// custom style is normally offered in both.
///
/// `@count` is a producer's cache over the regions. It is refreshed when this style is edited **and
/// the file declared one**, and never added to an element that wrote none.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "name", codec = Text, accessor = name, required))]
#[xml(attribute(local = "pivot", codec = OnOff, accessor = offered_for_pivot_tables, default = true))]
#[xml(attribute(local = "table", codec = OnOff, accessor = offered_for_tables, default = true))]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count))]
pub struct TableStyleDefinition {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "tableStyleElement", variant = Region, ty = TableStyleRegion))]
    content: Vec<TableStyleDefinitionContent>,
}

/// One child of [`TableStyleDefinition`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableStyleDefinitionContent {
    /// `x:tableStyleElement` — one region of the style.
    Region(TableStyleRegion),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl TableStyleDefinition {
    /// Builds an empty `x:tableStyle`, bound to `prefix` or to the default namespace.
    ///
    /// `@name` is `use="required"` and is not set here: a style name invented on a caller's behalf
    /// would be a style nothing points at.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "tableStyle"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[TableStyleDefinitionContent] {
        &self.content
    }

    /// Every `x:tableStyleElement`, in document order.
    pub fn regions(&self) -> impl Iterator<Item = &TableStyleRegion> + '_ {
        self.content.iter().filter_map(|item| match item {
            TableStyleDefinitionContent::Region(region) => Some(region),
            TableStyleDefinitionContent::Raw(_) => None,
        })
    }

    /// How many regions the style paints.
    #[must_use]
    pub fn len(&self) -> usize {
        self.regions().count()
    }

    /// Whether the style paints no region at all, which is valid markup and means it paints nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `index`-th `x:tableStyleElement`, mutably.
    pub fn region_mut(&mut self, index: usize) -> Option<&mut TableStyleRegion> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                TableStyleDefinitionContent::Region(region) => Some(region),
                TableStyleDefinitionContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends a region after the ones already present, refreshing `@count` when the file declared
    /// one.
    pub fn push(&mut self, interner: &mut Interner, region: TableStyleRegion) {
        self.content
            .push(TableStyleDefinitionContent::Region(region));
        self.empty = false;
        self.refresh_count(interner);
    }

    /// Writes `@count` from the regions actually present — but only onto an element that already
    /// declared one.
    fn refresh_count(&mut self, interner: &mut Interner) {
        if self.declared_count(interner).ok().flatten().is_some() {
            let count = u32::try_from(self.len()).unwrap_or(u32::MAX);
            self.set_declared_count(interner, Some(count));
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                TableStyleDefinitionContent::Region(region) => {
                    RawNode::Element(region.as_raw_element())
                }
                TableStyleDefinitionContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for TableStyleDefinition {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// `x:tableStyles` — the table
// -----------------------------------------------------------------------------------------------

/// `x:tableStyles` (`CT_TableStyles`, `sml.xsd:3689`) — every table style this workbook defines, and
/// the two defaults it prefers.
///
/// **`ST_`/`CT_` symbol:** `CT_TableStyles`. Wire element: `tableStyles`, rank **8** of
/// `CT_Stylesheet`.
///
/// `@defaultTableStyle` and `@defaultPivotStyle` are names, and almost always **preset** names: a
/// workbook whose `tableStyles` holds no `tableStyle` at all but says
/// `defaultTableStyle="TableStyleMedium2"` is entirely normal, and is the shape that makes
/// [`lookup`](Self::lookup) worth having.
///
/// `@count` is a producer's cache. It is refreshed when this table is edited **and the file declared
/// one**, and never added to an element that wrote none.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count))]
#[xml(attribute(local = "defaultTableStyle", codec = Text, accessor = default_table_style))]
#[xml(attribute(local = "defaultPivotStyle", codec = Text, accessor = default_pivot_style))]
pub struct TableStyles {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "tableStyle", variant = Style, ty = TableStyleDefinition))]
    content: Vec<TableStylesContent>,
}

/// One child of [`TableStyles`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableStylesContent {
    /// `x:tableStyle` — one style the workbook defines.
    Style(TableStyleDefinition),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl TableStyles {
    /// Builds an empty `x:tableStyles`, bound to `prefix` or to the default namespace.
    ///
    /// `tableStyle` is `minOccurs="0"`, so an empty element is valid markup and is what Excel writes
    /// for a workbook that defines no style of its own.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "tableStyles"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[TableStylesContent] {
        &self.content
    }

    /// Every `x:tableStyle`, in document order.
    pub fn styles(&self) -> impl Iterator<Item = &TableStyleDefinition> + '_ {
        self.content.iter().filter_map(|item| match item {
            TableStylesContent::Style(style) => Some(style),
            TableStylesContent::Raw(_) => None,
        })
    }

    /// How many styles the workbook defines for itself — which is **not** how many a consumer
    /// offers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.styles().count()
    }

    /// Whether the workbook defines no style of its own, which is the common case.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `index`-th `x:tableStyle`, mutably.
    pub fn style_mut(&mut self, index: usize) -> Option<&mut TableStyleDefinition> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                TableStylesContent::Style(style) => Some(style),
                TableStylesContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// What this workbook has to say about the style called `name`: defined here, a preset, or
    /// neither.
    ///
    /// **Never answers "not found" for a preset name.** See this module's own documentation for why
    /// that is the whole point of the three-way answer.
    ///
    /// Names are compared **exactly**, because §18.5.1.5 gives no case-folding rule for
    /// `tableStyleInfo@name` and inventing one would make this library answer a question the format
    /// does not ask. The first definition wins, which matters only for a file that already writes two
    /// styles with the same name.
    ///
    /// # Errors
    /// [`AttributeError`](mjx_ooxml_core::AttributeError) if a defined style's `@name` — which
    /// `CT_TableStyle` declares `use="required"` — is absent or will not decode. A style with no name
    /// is one nothing can point at, and skipping it silently would report a locally-defined style as
    /// a preset.
    pub fn lookup<'a>(
        &'a self,
        interner: &Interner,
        name: &str,
    ) -> Result<TableStyleLookup<'a>, mjx_ooxml_core::AttributeError> {
        for style in self.styles() {
            if style.name(interner)? == name {
                return Ok(TableStyleLookup::LocallyDefined(style));
            }
        }
        Ok(match builtin_table_style_name(name) {
            Some(builtin) => TableStyleLookup::BuiltIn(builtin),
            None => TableStyleLookup::Undefined,
        })
    }

    /// Appends a style after the ones already present, refreshing `@count` when the file declared
    /// one.
    ///
    /// Nothing checks that the name is free: a workbook that already defines the name is a workbook
    /// this library did not break, and refusing here would be refusing to write markup a producer is
    /// free to write.
    pub fn push(&mut self, interner: &mut Interner, style: TableStyleDefinition) {
        self.content.push(TableStylesContent::Style(style));
        self.empty = false;
        self.refresh_count(interner);
    }

    /// Writes `@count` from the styles actually present — but only onto an element that already
    /// declared one.
    fn refresh_count(&mut self, interner: &mut Interner) {
        if self.declared_count(interner).ok().flatten().is_some() {
            let count = u32::try_from(self.len()).unwrap_or(u32::MAX);
            self.set_declared_count(interner, Some(count));
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                TableStylesContent::Style(style) => RawNode::Element(style.as_raw_element()),
                TableStylesContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for TableStyles {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every one of the 144 names `presetTableStyles.xml` declares parses, and writes itself back.
    #[test]
    fn every_preset_name_parses_and_round_trips() {
        let mut seen = 0;
        for (family, prefix, highest) in BuiltInTableStyleFamily::FAMILIES {
            for number in 1..=*highest {
                let name = format!("{prefix}{number}");
                let parsed = builtin_table_style_name(&name)
                    .unwrap_or_else(|| panic!("{name} is one of the presets"));
                assert_eq!(parsed.family(), *family);
                assert_eq!(parsed.number(), number);
                assert_eq!(parsed.to_string(), name);
                seen += 1;
            }
        }
        assert_eq!(
            seen, 144,
            "the artifact declares 144 preset table styles; the families here cover {seen}"
        );
    }

    /// One past the top of each family is not a preset, and neither is zero.
    ///
    /// This is the boundary the "a built-in name is not a missing style" rule turns on in the other
    /// direction: reporting `TableStyleLight22` as built-in would be as wrong as reporting
    /// `TableStyleMedium2` as missing.
    #[test]
    fn a_name_outside_a_family_is_not_a_preset() {
        for (_, prefix, highest) in BuiltInTableStyleFamily::FAMILIES {
            assert!(
                builtin_table_style_name(&format!("{prefix}{}", highest + 1)).is_none(),
                "{prefix}{} is past the end of its family",
                highest + 1
            );
            assert!(builtin_table_style_name(&format!("{prefix}0")).is_none());
            assert!(builtin_table_style_name(prefix).is_none());
        }
        for name in [
            "",
            "TableStyleMedium",
            "TableStyleMedium02",
            "TableStyleMedium2 ",
            "tablestylemedium2",
            "AcmeBlue",
            "TableStyleHeavy1",
            "TableStyleMedium2x",
        ] {
            assert!(
                builtin_table_style_name(name).is_none(),
                "{name:?} must not be reported as a preset"
            );
        }
    }

    /// The three pivot families are pivot; the three table families are not.
    #[test]
    fn the_pivot_families_are_the_pivot_ones() {
        use BuiltInTableStyleFamily::{
            PivotDark, PivotLight, PivotMedium, TableDark, TableLight, TableMedium,
        };
        for family in [PivotLight, PivotMedium, PivotDark] {
            assert!(family.is_pivot());
        }
        for family in [TableLight, TableMedium, TableDark] {
            assert!(!family.is_pivot());
        }
    }
}
