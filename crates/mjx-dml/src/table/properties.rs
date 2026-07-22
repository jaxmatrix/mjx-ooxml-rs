//! `a:tblPr` (`CT_TableProperties`) — which parts of the table its style should emphasize, plus the
//! table's own fill and effects.

use mjx_ooxml_core::{FromXml as _, Interner, RawAttribute, RawName, RawNode};

use crate::build::{
    attr_bool, dml_child, dml_element, fidelity_element_impls, first_fill_child,
    replace_or_insert_child, set_attr,
};
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    #[must_use]
    pub fn part(&self, interner: &Interner, part: TablePart) -> Option<bool> {
        attr_bool(&self.attributes, interner, part.wire())
    }

    /// Whether the table has `part` **in effect** — the flag if stated, else the schema default.
    #[must_use]
    pub fn has_part(&self, interner: &Interner, part: TablePart) -> bool {
        self.part(interner, part).unwrap_or(false)
    }

    /// Turns `part` on or off. `false` **removes** the flag rather than writing `firstRow="0"`: the
    /// schema default is already false, so "off" is the absence of a claim.
    pub fn set_part(&mut self, interner: &mut Interner, part: TablePart, on: bool) {
        if on {
            set_attr(&mut self.attributes, interner, part.wire(), "1");
        } else {
            self.attributes.retain(|attribute| {
                attribute.name.prefix.is_some()
                    || interner.resolve(attribute.name.local) != part.wire()
            });
        }
    }

    /// Points the table at the table style with GUID `style_id` (`a:tableStyleId`), replacing any
    /// style reference it had. Inserted at its rank in `CT_TableProperties`'s sequence — after the
    /// fill and effects, before an `extLst`.
    pub fn set_table_style_id(&mut self, interner: &mut Interner, style_id: &str) {
        let text = RawNode::Text(Box::from(style_id.as_bytes()));
        let element = dml_element(interner, "tableStyleId", Vec::new(), vec![text]);
        replace_or_insert_child(
            &mut self.children,
            interner,
            element,
            |local| local == "tableStyleId" || local == "tableStyle",
            tblpr_child_rank,
        );
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
    /// it. A table may instead carry a whole `a:tableStyle` inline, which this does not report.
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
    pub fn set_attribute(&mut self, interner: &mut Interner, local: &str, value: &str) {
        set_attr(&mut self.attributes, interner, local, value);
    }
}

/// A child's rank in `CT_TableProperties`'s sequence: the fill group, the effects, the style choice
/// (`a:tableStyle` / `a:tableStyleId`), then the extension list. Order is validity, so a newly
/// inserted child is placed by this rather than appended.
fn tblpr_child_rank(local: &str) -> Option<usize> {
    if Fill::is_fill_local(local) {
        return Some(0);
    }
    match local {
        "effectLst" | "effectDag" => Some(1),
        "tableStyle" | "tableStyleId" => Some(2),
        "extLst" => Some(3),
        _ => None,
    }
}
