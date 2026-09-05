//! `x:cellStyles` / `x:cellStyle` (`CT_CellStyles` at `sml.xsd:3618`, `CT_CellStyle` at `3624`) —
//! the **named** cell styles, and the built-in names of ECMA-376 Part 1 Annex G.2.
//!
//! # A named style is a name for a `cellStyleXfs` record
//!
//! "Normal", "Comma", "Heading 1", "20% - Accent1" — each is a `cellStyle` element carrying a
//! `@name` and an `@xfId`, and the `@xfId` is an index into `cellStyleXfs`. The style itself holds
//! no formatting at all; it names a record that does.
//!
//! That matters to the resolver in [`super::effective`] in one specific way and no other: **a cell
//! never names a style by name.** A cell's `@s` is an index into `cellXfs`, that record's `@xfId` is
//! an index into `cellStyleXfs`, and the `cellStyles` table is what gives *that* record a name for a
//! user interface to show. So this table is read by a caller asking "what is this cell's style
//! called?", and is not on the resolution path.
//!
//! # `builtinId` decides, not `name`
//!
//! §18.8.7: *"For all built-in cell styles, the `builtinId` determines the style, not the name."* A
//! file may write `<cellStyle name="Normale" xfId="0" builtinId="0"/>` in a localized producer, and
//! that is still the `Normal` style. [`builtin_cell_style_name`] is Annex G.2's table of the
//! **invariant** names, so a caller can recognise a style whose `@name` it does not know.
//!
//! Two of the fifty-one entries are not fixed strings: `builtinId` 1 and 2 are `RowLevel_` and
//! `ColLevel_` followed by the outline level, which the element carries separately in `@iLevel`.
//! [`BuiltInCellStyleName`] keeps that distinction rather than inventing a level to concatenate.
//!
//! # Six attributes, not three
//!
//! The ticket for this child named `xfId`, `builtinId` and `iLevel`. `CT_CellStyle` declares
//! **six** — `name`, `xfId`, `builtinId`, `iLevel`, `hidden` and `customBuiltin` — and the last two
//! are the ones that say whether a style is offered in the user interface and whether a built-in has
//! been edited. All six are here.

use mjx_ooxml_core::{Interner, Number, RawAttribute, RawName, RawNode, Text};
use mjx_ooxml_types::support::OnOff;

use crate::leaf::attribute_bag;

attribute_bag! {
    /// `x:cellStyle` (`CT_CellStyle`, `sml.xsd:3624`) — one named cell style.
    ///
    /// `@xfId` is declared **required** here as well as in the schema, for the reason
    /// [`ColumnRun::first_column`](crate::ColumnRun::first_column) is: a named style that names no
    /// `cellStyleXfs` record is not a style, and substituting `0` would assert that it names the
    /// `Normal` record when the file says nothing at all. Reading such a file still succeeds — an
    /// attribute bag decodes nothing until it is asked — and only the getter reports
    /// [`AttributeError::Missing`](mjx_ooxml_core::AttributeError::Missing).
    ///
    /// `@iLevel` is an outline level and is meaningful only for `builtinId` 1 and 2; see
    /// [`builtin_cell_style_name`].
    #[xml(attribute(local = "name", codec = Text, accessor = style_name))]
    #[xml(attribute(local = "xfId", codec = Number<u32>, accessor = cell_style_format_index, required))]
    #[xml(attribute(local = "builtinId", codec = Number<u32>, accessor = builtin_id))]
    #[xml(attribute(local = "iLevel", codec = Number<u32>, accessor = outline_level))]
    #[xml(attribute(local = "hidden", codec = OnOff, accessor = hidden_in_user_interface))]
    #[xml(attribute(local = "customBuiltin", codec = OnOff, accessor = builtin_is_customized))]
    NamedCellStyle, "cellStyle"
}

/// `x:cellStyles` (`CT_CellStyles`, `sml.xsd:3618`) — the workbook's named cell styles.
///
/// Unlike every other table in this part, a `cellStyle` is **not** addressed by its position:
/// nothing anywhere names a `cellStyles` index. It is addressed by `@builtinId`, by `@name`, or not
/// at all. So this type offers [`get`](Self::get) by position only as an ordinal accessor, and the
/// lookups a caller actually wants are [`by_builtin_id`](Self::by_builtin_id) and
/// [`by_name`](Self::by_name).
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count))]
pub struct NamedCellStyles {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "cellStyle", variant = Style, ty = NamedCellStyle))]
    content: Vec<NamedCellStylesContent>,
}

/// One child of [`NamedCellStyles`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamedCellStylesContent {
    /// `x:cellStyle`.
    Style(NamedCellStyle),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl NamedCellStyles {
    /// Builds an empty `x:cellStyles`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "cellStyles"),
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
    pub fn content(&self) -> &[NamedCellStylesContent] {
        &self.content
    }

    /// Every `x:cellStyle`, in document order.
    pub fn styles(&self) -> impl Iterator<Item = &NamedCellStyle> + '_ {
        self.content.iter().filter_map(|item| match item {
            NamedCellStylesContent::Style(style) => Some(style),
            NamedCellStylesContent::Raw(_) => None,
        })
    }

    /// The `index`-th `x:cellStyle` in document order.
    ///
    /// An ordinal accessor, not an address: see this type's own documentation.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&NamedCellStyle> {
        self.styles().nth(index)
    }

    /// The first style whose `@builtinId` is `id` — the lookup §18.8.7 says is authoritative.
    #[must_use]
    pub fn by_builtin_id(&self, interner: &Interner, id: u32) -> Option<&NamedCellStyle> {
        self.styles()
            .find(|style| style.builtin_id(interner).ok().flatten() == Some(id))
    }

    /// The first style whose `@name` is `name`, compared exactly.
    ///
    /// Not case-insensitively, and not against Annex G.2's invariant name: a localized producer
    /// writes a localized `@name`, which is precisely why `@builtinId` exists.
    #[must_use]
    pub fn by_name(&self, interner: &Interner, name: &str) -> Option<&NamedCellStyle> {
        self.styles()
            .find(|style| style.style_name(interner).ok().flatten().as_deref() == Some(name))
    }

    /// The style that names `cellStyleXfs` record `index` through its `@xfId`.
    ///
    /// This is the direction the resolver would travel if it needed a name for what it resolved
    /// through — from a `cellStyleXfs` index back to the style that names it.
    #[must_use]
    pub fn by_cell_style_format_index(
        &self,
        interner: &Interner,
        index: u32,
    ) -> Option<&NamedCellStyle> {
        self.styles()
            .find(|style| style.cell_style_format_index(interner).ok() == Some(index))
    }

    /// How many styles the table holds — counted, not read from `@count`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.styles().count()
    }

    /// Whether the table holds no style at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends `style` after the last one, and updates `@count` when the file declared one.
    pub fn push(&mut self, interner: &mut Interner, style: NamedCellStyle) {
        self.content.push(NamedCellStylesContent::Style(style));
        self.empty = false;
        if self.declared_count(interner).ok().flatten().is_some() {
            let count = u32::try_from(self.len()).unwrap_or(u32::MAX);
            self.set_declared_count(interner, Some(count));
        }
    }
}

/// The invariant name Annex G.2 gives a `@builtinId`.
///
/// Two of the fifty-one entries are a **prefix** rather than a name: Annex G.2 spells them
/// `RowLevel_ + level #` and `ColLevel_ + level #`, where the level comes from the element's own
/// `@iLevel`. Returning `"RowLevel_"` as though it were the whole name would be wrong, and
/// concatenating a level this function has not been given would be an invention, so the two are
/// their own variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltInCellStyleName {
    /// A fixed name — `Normal`, `Comma`, `Heading 1`, `20% - Accent1`, and forty-six more.
    Fixed(&'static str),
    /// `RowLevel_` followed by the style's own `@iLevel` (`builtinId` 1).
    RowOutlineLevel,
    /// `ColLevel_` followed by the style's own `@iLevel` (`builtinId` 2).
    ColumnOutlineLevel,
}

impl BuiltInCellStyleName {
    /// The fixed name, or `None` for the two outline-level prefixes.
    #[must_use]
    pub const fn fixed(self) -> Option<&'static str> {
        match self {
            Self::Fixed(name) => Some(name),
            Self::RowOutlineLevel | Self::ColumnOutlineLevel => None,
        }
    }

    /// The name with `level` appended, for the two outline-level entries; the fixed name otherwise.
    ///
    /// `level` is the style's `@iLevel`, and is ignored by every other entry.
    #[must_use]
    pub fn resolve(self, level: u32) -> std::borrow::Cow<'static, str> {
        match self {
            Self::Fixed(name) => std::borrow::Cow::Borrowed(name),
            Self::RowOutlineLevel => std::borrow::Cow::Owned(format!("RowLevel_{level}")),
            Self::ColumnOutlineLevel => std::borrow::Cow::Owned(format!("ColLevel_{level}")),
        }
    }
}

/// The invariant cell-style name ECMA-376 Part 1 Annex G.2 gives `builtin_id`.
///
/// `None` for an id Annex G.2 does not list — 12, 13 and 14 are gaps in the published table, and so
/// is everything above 53. §18.8.7 notes that additional values may be used but that
/// interoperability then rests on agreement between implementers, so answering `None` is the honest
/// result rather than a missing row.
///
/// The names are the invariant ones. A localized producer writes a localized `@name` beside the
/// same `@builtinId`, and §18.8.7 says the id is what determines the style.
#[must_use]
pub const fn builtin_cell_style_name(builtin_id: u32) -> Option<BuiltInCellStyleName> {
    use BuiltInCellStyleName::{ColumnOutlineLevel, Fixed, RowOutlineLevel};
    Some(match builtin_id {
        0 => Fixed("Normal"),
        1 => RowOutlineLevel,
        2 => ColumnOutlineLevel,
        3 => Fixed("Comma"),
        4 => Fixed("Currency"),
        5 => Fixed("Percent"),
        6 => Fixed("Comma [0]"),
        7 => Fixed("Currency [0]"),
        8 => Fixed("Hyperlink"),
        9 => Fixed("Followed Hyperlink"),
        10 => Fixed("Note"),
        11 => Fixed("Warning Text"),
        15 => Fixed("Title"),
        16 => Fixed("Heading 1"),
        17 => Fixed("Heading 2"),
        18 => Fixed("Heading 3"),
        19 => Fixed("Heading 4"),
        20 => Fixed("Input"),
        21 => Fixed("Output"),
        22 => Fixed("Calculation"),
        23 => Fixed("Check Cell"),
        24 => Fixed("Linked Cell"),
        25 => Fixed("Total"),
        26 => Fixed("Good"),
        27 => Fixed("Bad"),
        28 => Fixed("Neutral"),
        29 => Fixed("Accent1"),
        30 => Fixed("20% - Accent1"),
        31 => Fixed("40% - Accent1"),
        32 => Fixed("60% - Accent1"),
        33 => Fixed("Accent2"),
        34 => Fixed("20% - Accent2"),
        35 => Fixed("40% - Accent2"),
        36 => Fixed("60% - Accent2"),
        37 => Fixed("Accent3"),
        38 => Fixed("20% - Accent3"),
        39 => Fixed("40% - Accent3"),
        40 => Fixed("60% - Accent3"),
        41 => Fixed("Accent4"),
        42 => Fixed("20% - Accent4"),
        43 => Fixed("40% - Accent4"),
        44 => Fixed("60% - Accent4"),
        45 => Fixed("Accent5"),
        46 => Fixed("20% - Accent5"),
        47 => Fixed("40% - Accent5"),
        48 => Fixed("60% - Accent5"),
        49 => Fixed("Accent6"),
        50 => Fixed("20% - Accent6"),
        51 => Fixed("40% - Accent6"),
        52 => Fixed("60% - Accent6"),
        53 => Fixed("Explanatory Text"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use mjx_ooxml_core::FromXml;

    use super::*;

    /// Annex G.2's gaps are gaps here, and its two prefix entries are not fixed names.
    #[test]
    fn the_builtin_table_reproduces_annex_g2_including_its_holes() {
        assert_eq!(
            builtin_cell_style_name(0),
            Some(BuiltInCellStyleName::Fixed("Normal"))
        );
        assert_eq!(
            builtin_cell_style_name(53),
            Some(BuiltInCellStyleName::Fixed("Explanatory Text"))
        );
        for id in [12, 13, 14, 54, 100] {
            assert_eq!(
                builtin_cell_style_name(id),
                None,
                "Annex G.2 lists 0-11 and 15-53; {id} is not among them"
            );
        }
        let listed = (0..=11).chain(15..=53).count();
        assert_eq!(listed, 51);
        assert_eq!(
            (0..=200u32)
                .filter(|id| builtin_cell_style_name(*id).is_some())
                .count(),
            51
        );

        assert_eq!(
            builtin_cell_style_name(1)
                .expect("builtinId 1")
                .resolve(3)
                .as_ref(),
            "RowLevel_3"
        );
        assert_eq!(
            builtin_cell_style_name(2)
                .expect("builtinId 2")
                .resolve(7)
                .as_ref(),
            "ColLevel_7"
        );
        assert_eq!(
            builtin_cell_style_name(1).and_then(BuiltInCellStyleName::fixed),
            None
        );
        // A level is ignored by every fixed entry rather than appended to it.
        assert_eq!(
            builtin_cell_style_name(0)
                .expect("builtinId 0")
                .resolve(4)
                .as_ref(),
            "Normal"
        );
    }

    /// The three lookups answer about the same element, and `@name` is compared exactly.
    #[test]
    fn a_localized_name_is_still_found_by_its_builtin_id() {
        let markup = concat!(
            r#"<cellStyles xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="2">"#,
            r#"<cellStyle name="Normale" xfId="0" builtinId="0"/>"#,
            r#"<cellStyle name="Titre" xfId="4" builtinId="15" hidden="1" customBuiltin="1"/>"#,
            "</cellStyles>"
        );
        let document = mjx_xml::fidelity::parse(markup.as_bytes()).expect("the table parses");
        let table =
            NamedCellStyles::from_xml(&document.root, &document.interner).expect("it reads");

        assert_eq!(table.len(), 2);
        let title = table
            .by_builtin_id(&document.interner, 15)
            .expect("builtinId 15 is written");
        assert_eq!(
            title
                .style_name(&document.interner)
                .expect("the name decodes")
                .as_deref(),
            Some("Titre"),
            "the file's own name is reported, never Annex G.2's"
        );
        assert_eq!(
            builtin_cell_style_name(15).and_then(BuiltInCellStyleName::fixed),
            Some("Title"),
            "and the invariant name is available beside it"
        );
        assert_eq!(
            title
                .cell_style_format_index(&document.interner)
                .expect("@xfId is required and written"),
            4
        );
        assert_eq!(
            title
                .hidden_in_user_interface(&document.interner)
                .expect("the flag decodes"),
            Some(true)
        );
        assert_eq!(
            title
                .builtin_is_customized(&document.interner)
                .expect("the flag decodes"),
            Some(true)
        );

        assert!(table.by_name(&document.interner, "Title").is_none());
        assert!(table.by_name(&document.interner, "Titre").is_some());
        assert_eq!(
            table
                .by_cell_style_format_index(&document.interner, 4)
                .and_then(|style| style.builtin_id(&document.interner).ok().flatten()),
            Some(15)
        );
    }

    /// `@xfId` is required, so a style that omits it says so rather than answering `0`.
    #[test]
    fn a_style_with_no_format_index_reports_the_attribute_as_missing() {
        let markup = r#"<cellStyle xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" name="Broken"/>"#;
        let document = mjx_xml::fidelity::parse(markup.as_bytes()).expect("it parses");
        let style = NamedCellStyle::from_xml(&document.root, &document.interner)
            .expect("an attribute bag decodes nothing on read");
        assert!(matches!(
            style.cell_style_format_index(&document.interner),
            Err(mjx_ooxml_core::AttributeError::Missing { attribute: "xfId" })
        ));
    }
}
