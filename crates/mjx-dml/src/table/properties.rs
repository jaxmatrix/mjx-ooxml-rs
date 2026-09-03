//! `a:tblPr` (`CT_TableProperties`) — which parts of the table its style should emphasize, plus the
//! table's own fill and effects.

use mjx_ooxml_core::{FromXml as _, Interner, RawAttribute, RawName, RawNode, ToXml as _};
use mjx_ooxml_types::support::OnOff;

use super::style::TableStyle;
use mjx_ooxml_types::child_order::TABLE_PROPERTIES;

use crate::build::{dml_child, dml_element, dml_name, fidelity_element_impls, first_fill_child};
use crate::effect::EffectList;
use crate::fill::Fill;

/// A part of a table that its style may format differently — the seven `a:tblPr` flags.
///
/// These do not draw anything themselves. Each says *this table has such a part*, and the table
/// style then supplies the formatting for it: turning on [`FirstRow`](TablePart::FirstRow) is what
/// makes a header row look like a header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TablePart {
    /// `@firstRow` — the table has a header row.
    FirstRow,
    /// `@firstCol` — the table has a header column.
    FirstColumn,
    /// `@lastRow` — the table has a total row.
    LastRow,
    /// `@lastCol` — the table has a total column.
    LastColumn,
    /// `@bandRow` — rows alternate between two banded formats.
    BandedRows,
    /// `@bandCol` — columns alternate between two banded formats.
    BandedColumns,
    /// `@rtl` — the table's columns run right to left.
    RightToLeft,
}

impl TablePart {
    /// The attribute's name, without a prefix.
    #[must_use]
    pub fn wire(self) -> &'static str {
        match self {
            Self::FirstRow => "firstRow",
            Self::FirstColumn => "firstCol",
            Self::LastRow => "lastRow",
            Self::LastColumn => "lastCol",
            Self::BandedRows => "bandRow",
            Self::BandedColumns => "bandCol",
            Self::RightToLeft => "rtl",
        }
    }

    /// Every flag, for a caller reading or copying the whole set.
    #[must_use]
    pub fn all() -> [Self; 7] {
        [
            Self::FirstRow,
            Self::FirstColumn,
            Self::LastRow,
            Self::LastColumn,
            Self::BandedRows,
            Self::BandedColumns,
            Self::RightToLeft,
        ]
    }
}

/// `a:tblPr` (`CT_TableProperties`) — the table's banding flags, fill, effects and style reference.
///
/// A fidelity wrapper: the flags and the fill/effect children are exposed typed, while the style
/// choice (`a:tableStyle` / `a:tableStyleId`), `extLst` and anything unknown are preserved opaque.
/// Every flag defaults to `false`, so an unstated one is off.
///
/// The seven flags are declared here and reached through [`part`](Self::part) /
/// [`set_part`](Self::set_part), which take a [`TablePart`]: one method over seven attributes reads
/// better than seven near-identical ones, and a caller that wants to copy the whole set can iterate
/// [`TablePart::all`].
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "rtl", codec = OnOff, accessor = right_to_left))]
#[xml(attribute(local = "firstRow", codec = OnOff, accessor = first_row))]
#[xml(attribute(local = "firstCol", codec = OnOff, accessor = first_column))]
#[xml(attribute(local = "lastRow", codec = OnOff, accessor = last_row))]
#[xml(attribute(local = "lastCol", codec = OnOff, accessor = last_column))]
#[xml(attribute(local = "bandRow", codec = OnOff, accessor = banded_rows))]
#[xml(attribute(local = "bandCol", codec = OnOff, accessor = banded_columns))]
pub struct TableProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(TableProperties);

impl TableProperties {
    /// Whether the table declares `part`, or `None` if it does not state the flag at all.
    ///
    /// Unstated and `false` render identically — the schema default is `false` — but they are
    /// reported apart, because a writer should not add attributes a file never had.
    ///
    /// A value that is not an `ST_OnOff` is reported the same way an absent one is: a flag this
    /// model cannot read is a flag the table does not state, and the attribute round-trips verbatim
    /// either way.
    #[must_use]
    pub fn part(&self, interner: &Interner, part: TablePart) -> Option<bool> {
        match part {
            TablePart::FirstRow => self.first_row(interner),
            TablePart::FirstColumn => self.first_column(interner),
            TablePart::LastRow => self.last_row(interner),
            TablePart::LastColumn => self.last_column(interner),
            TablePart::BandedRows => self.banded_rows(interner),
            TablePart::BandedColumns => self.banded_columns(interner),
            TablePart::RightToLeft => self.right_to_left(interner),
        }
        .ok()
        .flatten()
    }

    /// Whether the table has `part` **in effect** — the flag if stated, else the schema default.
    #[must_use]
    pub fn has_part(&self, interner: &Interner, part: TablePart) -> bool {
        self.part(interner, part).unwrap_or(false)
    }

    /// Turns `part` on or off. `false` **removes** the flag rather than writing `firstRow="false"`:
    /// the schema default is already false, so "off" is the absence of a claim.
    ///
    /// `true` writes the one canonical `ST_OnOff` spelling this project emits, which is `true`.
    pub fn set_part(&mut self, interner: &mut Interner, part: TablePart, on: bool) {
        let on = on.then_some(true);
        match part {
            TablePart::FirstRow => self.set_first_row(interner, on),
            TablePart::FirstColumn => self.set_first_column(interner, on),
            TablePart::LastRow => self.set_last_row(interner, on),
            TablePart::LastColumn => self.set_last_column(interner, on),
            TablePart::BandedRows => self.set_banded_rows(interner, on),
            TablePart::BandedColumns => self.set_banded_columns(interner, on),
            TablePart::RightToLeft => self.set_right_to_left(interner, on),
        }
    }

    /// Points the table at the table style with GUID `style_id` (`a:tableStyleId`), replacing any
    /// style reference it had. Inserted at its rank in `CT_TableProperties`'s sequence — after the
    /// fill and effects, before an `extLst`.
    pub fn set_table_style_id(&mut self, interner: &mut Interner, style_id: &str) {
        let text = RawNode::Text(Box::from(style_id.as_bytes()));
        let element = dml_element(interner, "tableStyleId", Vec::new(), vec![text]);
        TABLE_PROPERTIES.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "tableStyleId" || local == "tableStyle"
        });
        self.empty = false;
    }

    /// The table's own fill (`EG_FillProperties`), or `None` if it declares none.
    #[must_use]
    pub fn fill(&self, interner: &Interner) -> Option<Fill> {
        first_fill_child(&self.children, interner)
            .and_then(|element| Fill::from_xml(element, interner).ok())
    }

    /// The table's effect list (`a:effectLst`), or `None` if it declares none.
    #[must_use]
    pub fn effects(&self, interner: &Interner) -> Option<EffectList> {
        dml_child(&self.children, interner, "effectLst")
            .and_then(|element| EffectList::from_xml(element, interner).ok())
    }

    /// The GUID of the table style this table uses (`a:tableStyleId`), or `None` if it names none.
    ///
    /// The style itself lives in the presentation's `tableStyles.xml` part (modeled by
    /// [`TableStyleList`](super::TableStyleList)); this reports the reference so a caller can resolve
    /// it. A table may instead carry a whole [`a:tableStyle` inline](Self::inline_style), which this
    /// does not report.
    #[must_use]
    pub fn table_style_id<'a>(&'a self, interner: &'a Interner) -> Option<&'a str> {
        dml_child(&self.children, interner, "tableStyleId").and_then(|element| {
            element.children.iter().find_map(|node| match node {
                RawNode::Text(bytes) | RawNode::CData(bytes) => {
                    std::str::from_utf8(bytes).ok().map(str::trim)
                }
                _ => None,
            })
        })
    }

    /// Writes `style` **inline** on the table (`a:tableStyle`), replacing any inline style or
    /// `a:tableStyleId` reference it had — the two are a choice, so setting one clears the other.
    ///
    /// `style` is a [`TableStyle`] built with the model's own constructors; it is emitted as
    /// `a:tableStyle` (the tag this choice uses) rather than the `a:tblStyle` a shared style is, since
    /// they are the same `CT_TableStyle` under different names. Placed at its rank in the sequence —
    /// after the fill and effects, before an `extLst`.
    pub fn set_inline_style(&mut self, interner: &mut Interner, style: &TableStyle) {
        let mut element = style.to_xml(interner);
        element.name = dml_name(interner, "tableStyle");
        TABLE_PROPERTIES.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "tableStyle" || local == "tableStyleId"
        });
        self.empty = false;
    }

    /// The [`TableStyle`] defined **inline** on the table (`a:tableStyle`), or `None` if the table
    /// names its style by GUID ([`table_style_id`](Self::table_style_id)) or none at all.
    ///
    /// The two are the `(tableStyle | tableStyleId)?` choice of `CT_TableProperties`: a table either
    /// points at a shared style in `tableStyles.xml` or spells one out here. An inline style is the
    /// same `CT_TableStyle` a shared one is, so the whole style model applies to it.
    #[must_use]
    pub fn inline_style(&self, interner: &Interner) -> Option<TableStyle> {
        dml_child(&self.children, interner, "tableStyle")
            .and_then(|element| TableStyle::from_xml(element, interner).ok())
    }

    /// The properties' attributes, verbatim.
    #[must_use]
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }

    /// The properties' children, verbatim.
    #[must_use]
    pub fn children(&self) -> &[RawNode] {
        &self.children
    }

    /// The properties' children, mutably.
    pub fn children_mut(&mut self) -> &mut Vec<RawNode> {
        &mut self.children
    }

    /// Sets an attribute, rewriting it in place when already present.
    ///
    /// The untyped escape hatch: `local` and `value` are whatever the caller says, so this reaches
    /// the attributes `CT_TableProperties` carries that this model does not name.
    pub fn set_attribute(&mut self, interner: &mut Interner, local: &str, value: &str) {
        mjx_xml::attribute::set(&mut self.attributes, interner, None, local, value);
    }
}
