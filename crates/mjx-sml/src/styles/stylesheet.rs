//! `xl/styles.xml` — `CT_Stylesheet` (`sml.xsd:3387`), the eleven-slot frame.
//!
//! # Five slots modelled, six held
//!
//! | rank | element | held as |
//! |---|---|---|
//! | 0 | `numFmts` | [`StylesheetContent::Raw`] — MJXOFF-108 (D09) |
//! | 1 | `fonts` | [`FontTable`] |
//! | 2 | `fills` | [`FillTable`] |
//! | 3 | `borders` | [`BorderTable`] |
//! | 4 | `cellStyleXfs` | [`StylesheetContent::Raw`] — MJXOFF-108 (D09) |
//! | 5 | `cellXfs` | [`StylesheetContent::Raw`] — MJXOFF-108 (D09) |
//! | 6 | `cellStyles` | [`StylesheetContent::Raw`] — MJXOFF-108 (D09) |
//! | 7 | `dxfs` | [`DifferentialFormats`] |
//! | 8 | `tableStyles` | [`StylesheetContent::Raw`] — MJXOFF-127 (D15) |
//! | 9 | `colors` | [`ColorTable`] |
//! | 10 | `extLst` | [`StylesheetContent::Raw`], on purpose and for good |
//!
//! The split is the part's own seam. MJXOFF-105 builds the **resource tables** a style index
//! resolves *into*; MJXOFF-108 builds the `xf` indirection that does the resolving. A `cellXfs` that
//! survives a round-trip today is proof the frame works, not proof an `xf` was modelled — and when
//! MJXOFF-108 lands, it replaces one `Raw` slot and changes nothing else here.
//!
//! The ranks above are never written down. Every placement goes through
//! [`mjx_ooxml_types::child_order::STYLESHEET`], generated from `sml.xsd` by
//! `cargo run -p xtask -- codegen`.
//!
//! # This model has never heard of a package, or of a theme part
//!
//! `styles.xml` names no relationship at all — it is the one major SpreadsheetML part with no `r:id`
//! anywhere in it. Its one outward reference is `<color theme="N"/>`, a position in the theme's
//! colour scheme, and resolving that needs the *theme part*, which is `mjx-xlsx`'s to fetch.
//! [`super::palette::resolve_color`] therefore takes an already-resolved
//! [`SchemeColors`](mjx_dml::SchemeColors) — `mjx-dml`'s interner-free bridge between two parts —
//! and this crate never learns where it came from.

use mjx_ooxml_core::{
    Enumeration, FromXml, Interner, RawAttribute, RawDocument, RawElement, RawName, RawNode,
};
use mjx_ooxml_types::child_order::STYLESHEET;
use mjx_ooxml_types::namespaces::SML;
use mjx_ooxml_types::shared::ConformanceClass;

use crate::error::SmlError;

use super::borders::BorderTable;
use super::colors::ColorTable;
use super::differential::DifferentialFormats;
use super::fills::FillTable;
use super::fonts::FontTable;

/// `x:styleSheet` (`CT_Stylesheet`, `sml.xsd:3387`) — the whole styles part.
///
/// See the [module documentation](self) for the eleven slots and for where the boundary with
/// MJXOFF-108 runs.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "conformance", codec = Enumeration<ConformanceClass>, accessor = conformance))]
pub struct StylesheetPart {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "fonts", variant = Fonts, ty = FontTable),
        child(local = "fills", variant = Fills, ty = FillTable),
        child(local = "borders", variant = Borders, ty = BorderTable),
        child(local = "dxfs", variant = DifferentialFormats, ty = DifferentialFormats),
        child(local = "colors", variant = Colors, ty = ColorTable)
    )]
    content: Vec<StylesheetContent>,
}

/// One child of [`StylesheetPart`]: five modelled slots, and everything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StylesheetContent {
    /// `x:fonts` (rank 1).
    Fonts(FontTable),
    /// `x:fills` (rank 2).
    Fills(FillTable),
    /// `x:borders` (rank 3).
    Borders(BorderTable),
    /// `x:dxfs` (rank 7).
    DifferentialFormats(DifferentialFormats),
    /// `x:colors` (rank 9).
    Colors(ColorTable),
    /// The six slots this child does not model — `numFmts`, `cellStyleXfs`, `cellXfs`,
    /// `cellStyles`, `tableStyles` and `extLst` — plus any foreign element, any
    /// `mc:AlternateContent`, and the text, comments and processing instructions between siblings.
    ///
    /// Preserved verbatim and in position: placement skips a node it cannot rank, so an unmodelled
    /// child never moves and never moves anything else.
    Raw(RawNode),
}

impl StylesheetContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    #[must_use]
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::Fonts(_) => "fonts",
            Self::Fills(_) => "fills",
            Self::Borders(_) => "borders",
            Self::DifferentialFormats(_) => "dxfs",
            Self::Colors(_) => "colors",
            Self::Raw(_) => return None,
        })
    }

    /// This child's rank in `CT_Stylesheet`'s `xsd:sequence`, from the generated table — **for an
    /// unmodelled element too**.
    ///
    /// This is the one place the styles frame cannot copy [`WorkbookPart`](crate::WorkbookPart) or
    /// [`WorksheetPart`](crate::WorksheetPart), and the reason is arithmetic rather than taste.
    /// Those two model a *prefix* of their sequence — ranks 0–17 of nineteen, and 0–6 of thirty-nine
    /// — so every slot they model ranks below every slot they hold raw, and a new child always
    /// belongs before all of them. This frame models ranks **1, 2, 3, 7 and 9** and holds **0, 4, 5,
    /// 6, 8 and 10** raw: the two sets interleave, so a `colors` inserted into a part that already
    /// writes `numFmts` and `cellXfs` has to land *after* both. Treating an unmodelled element as
    /// unranked would put it first.
    ///
    /// So a `Raw` element is ranked through the same generated table, by its own name, and only a
    /// node the table genuinely does not name — a foreign element, a comment, an
    /// `mc:AlternateContent` — stays unranked and is stepped over.
    #[must_use]
    fn rank(&self, interner: &Interner) -> Option<u16> {
        match self {
            Self::Raw(RawNode::Element(element)) => STYLESHEET.rank_of_element(element, interner),
            Self::Raw(_) => None,
            modelled => STYLESHEET.rank_of(None, modelled.local()?),
        }
    }
}

/// Declares one singleton slot: a borrowing getter, a mutable getter, and a setter that replaces the
/// existing child in place or inserts a new one at its rank in `CT_Stylesheet`'s sequence.
///
/// All five slots share these three bodies, and writing them out five times would be five chances to
/// reach for the wrong variant.
macro_rules! singleton_slot {
    ($getter:ident, $getter_mut:ident, $setter:ident, $variant:ident, $ty:ty, $local:literal, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<&$ty> {
            self.content.iter().find_map(|item| match item {
                StylesheetContent::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("`x:", $local, "`, mutably — `None` if the part writes none.")]
        #[must_use]
        pub fn $getter_mut(&mut self) -> Option<&mut $ty> {
            self.content.iter_mut().find_map(|item| match item {
                StylesheetContent::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("Sets `x:", $local, "`: `None` removes it; `Some(value)` replaces the \
            existing element **where it is**, or inserts a new one at its rank in \
            `CT_Stylesheet`'s `xsd:sequence`.\n\n\
            Takes the interner because placement has to rank the part's **unmodelled** slots too — \
            six of the eleven are held raw here, and they interleave with the five that are \
            modelled, so a raw element is ranked through the generated table by its own name. \
            Neither `WorkbookPart` nor `WorksheetPart` needs that: both model a prefix of their \
            sequence.")]
        pub fn $setter(&mut self, interner: &Interner, value: Option<$ty>) {
            let is_target =
                |item: &StylesheetContent| matches!(item, StylesheetContent::$variant(_));
            self.replace_or_insert(
                interner,
                $local,
                is_target,
                value.map(StylesheetContent::$variant),
            );
        }
    };
}

impl StylesheetPart {
    /// Reads a whole `xl/styles.xml` part.
    ///
    /// `Ok(None)` when the document's root is not an `x:styleSheet` — the caller handed over a
    /// different part, which is a question rather than an error, exactly as
    /// [`WorkbookPart::read_part`](crate::WorkbookPart::read_part) treats it.
    ///
    /// # Errors
    /// [`SmlError::Model`] if a modelled element does not match the shape its complex type declares.
    /// Nothing a well-formed file can *say* is refused.
    pub fn read_part(document: &RawDocument) -> Result<Option<Self>, SmlError> {
        Self::read_root(&document.root, &document.interner)
    }

    /// [`read_part`](Self::read_part) for a caller that holds the root element and the interner
    /// rather than the whole document — which is the shape an *editing* caller is in.
    ///
    /// # Errors
    /// As [`read_part`](Self::read_part).
    pub fn read_root(root: &RawElement, interner: &Interner) -> Result<Option<Self>, SmlError> {
        let namespace = root.name.namespace.map(|symbol| interner.resolve(symbol));
        let in_spreadsheetml =
            namespace == Some(SML.transitional) || (namespace.is_some() && namespace == SML.strict);
        if !in_spreadsheetml || interner.resolve(root.name.local) != "styleSheet" {
            return Ok(None);
        }
        Ok(Some(Self::from_xml(root, interner)?))
    }

    /// Builds an empty `x:styleSheet`, bound to `prefix` or to the default namespace.
    ///
    /// Declares no namespaces of its own: a part written from this has to bind at least the
    /// SpreadsheetML namespace. MJXOFF-112 (D10) is what writes whole parts from nothing; this
    /// exists so the model is constructible rather than only readable.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "styleSheet"),
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

    /// Every child, in document order, including the six slots this type does not model.
    #[must_use]
    pub fn content(&self) -> &[StylesheetContent] {
        &self.content
    }

    /// The local name of every **element** child, in document order.
    ///
    /// This is what an ordering assertion is written against: it says what the part *will emit*,
    /// which is the thing schema order is a property of. A modelled slot answers with the wire name
    /// its type is declared under; an unmodelled one answers with the name the file wrote.
    pub fn child_element_locals<'a>(
        &'a self,
        interner: &'a Interner,
    ) -> impl Iterator<Item = &'a str> + 'a {
        self.content.iter().filter_map(move |item| match item {
            StylesheetContent::Raw(RawNode::Element(element)) => {
                Some(interner.resolve(element.name.local))
            }
            StylesheetContent::Raw(_) => None,
            modelled => modelled.local(),
        })
    }

    singleton_slot!(
        fonts,
        fonts_mut,
        set_fonts,
        Fonts,
        FontTable,
        "fonts",
        "`x:fonts` — the font table an `xf`'s `@fontId` indexes. `None` if the part writes none, \
         which is legal and means every `@fontId` in the workbook dangles."
    );
    singleton_slot!(
        fills,
        fills_mut,
        set_fills,
        Fills,
        FillTable,
        "fills",
        "`x:fills` — the fill table an `xf`'s `@fillId` indexes."
    );
    singleton_slot!(
        borders,
        borders_mut,
        set_borders,
        Borders,
        BorderTable,
        "borders",
        "`x:borders` — the border table an `xf`'s `@borderId` indexes."
    );
    singleton_slot!(
        differential_formats,
        differential_formats_mut,
        set_differential_formats,
        DifferentialFormats,
        DifferentialFormats,
        "dxfs",
        "`x:dxfs` — the differential formats a conditional-formatting rule (MJXOFF-120) or a table \
         style (MJXOFF-127) names by `@dxfId`. Built here because it is a resource table like the \
         other three, even though its consumers arrive later."
    );
    singleton_slot!(
        colors,
        colors_mut,
        set_colors,
        Colors,
        ColorTable,
        "colors",
        "`x:colors` — the workbook's replacement indexed palette and its most-recently-used \
         colours. `None` means the **default** palette, not an empty one; see \
         [`super::palette`]."
    );

    /// Where a child named `local` belongs among the current children.
    fn insert_index(&self, interner: &Interner, local: &str) -> usize {
        STYLESHEET.insert_index_of_names(self.content.iter().map(|item| item.rank(interner)), local)
    }

    /// Replaces the first child `is_target` accepts, keeping its position; inserts at the schema
    /// rank when there is none; removes it when `value` is `None`.
    fn replace_or_insert(
        &mut self,
        interner: &Interner,
        local: &str,
        is_target: impl Fn(&StylesheetContent) -> bool,
        value: Option<StylesheetContent>,
    ) {
        let existing = self.content.iter().position(&is_target);
        match (existing, value) {
            (Some(at), Some(value)) => self.content[at] = value,
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(value)) => {
                let at = self.insert_index(interner, local);
                self.content.insert(at, value);
                self.empty = false;
            }
            (None, None) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every slot the generated table names is either modelled here or one of the six a later child
    /// owns — and the six are named, so a slot added to `sml.xsd` and regenerated fails here rather
    /// than being silently dropped into the unknown bucket.
    #[test]
    fn every_slot_of_the_generated_sequence_is_accounted_for() {
        assert_eq!(STYLESHEET.symbol, "CT_Stylesheet");
        assert_eq!(
            STYLESHEET.slots.len(),
            11,
            "CT_Stylesheet is an eleven-slot sequence"
        );
        let modelled = ["fonts", "fills", "borders", "dxfs", "colors"];
        let held = [
            "numFmts",
            "cellStyleXfs",
            "cellXfs",
            "cellStyles",
            "tableStyles",
            "extLst",
        ];
        for slot in STYLESHEET.slots {
            assert!(
                modelled.contains(&slot.local) || held.contains(&slot.local),
                "`{}` is a child of CT_Stylesheet that this frame neither models nor names as held",
                slot.local
            );
        }
        assert_eq!(modelled.len() + held.len(), STYLESHEET.slots.len());
    }

    /// A new table lands at its **schema** rank, not at the end, and not where a comment happens to
    /// be.
    #[test]
    fn an_inserted_table_lands_at_its_rank_among_unmodelled_neighbours() {
        let markup = concat!(
            r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
            r#"<numFmts count="0"/><!-- between --><cellXfs count="0"/><extLst/>"#,
            "</styleSheet>"
        );
        let mut document = mjx_xml::fidelity::parse(markup.as_bytes()).expect("the part parses");
        let mut part = StylesheetPart::read_part(&document)
            .expect("the part reads")
            .expect("the root is an x:styleSheet");

        let fonts = FontTable::new(&mut document.interner, None);
        part.set_fonts(&document.interner, Some(fonts));
        let colors = ColorTable::new(&mut document.interner, None);
        part.set_colors(&document.interner, Some(colors));

        let locals: Vec<&str> = part.child_element_locals(&document.interner).collect();
        assert_eq!(
            locals,
            vec!["numFmts", "fonts", "cellXfs", "colors", "extLst"],
            "`fonts` is rank 1 and `colors` rank 9, so both land among the slots already there"
        );
    }
}
