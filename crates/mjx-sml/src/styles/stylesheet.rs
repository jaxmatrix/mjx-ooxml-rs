//! `xl/styles.xml` — `CT_Stylesheet` (`sml.xsd:3387`), the eleven-slot frame.
//!
//! # Ten slots modelled, one held
//!
//! | rank | element | held as |
//! |---|---|---|
//! | 0 | `numFmts` | [`NumberFormatTable`] |
//! | 1 | `fonts` | [`FontTable`] |
//! | 2 | `fills` | [`FillTable`] |
//! | 3 | `borders` | [`BorderTable`] |
//! | 4 | `cellStyleXfs` | [`CellFormatTable`] |
//! | 5 | `cellXfs` | [`CellFormatTable`] |
//! | 6 | `cellStyles` | [`NamedCellStyles`] |
//! | 7 | `dxfs` | [`DifferentialFormats`] |
//! | 8 | `tableStyles` | [`TableStyles`] |
//! | 9 | `colors` | [`ColorTable`] |
//! | 10 | `extLst` | [`StylesheetContent::Raw`], on purpose and for good |
//!
//! The split was the part's own seam. MJXOFF-105 built the **resource tables** a style index
//! resolves *into*; MJXOFF-108 builds the `xf` indirection that does the resolving, and it took the
//! four slots that child had held raw. MJXOFF-125 (D15) takes the last of them, `tableStyles`, and
//! what is left is `extLst`, which stays raw on purpose and for good.
//!
//! **`cellStyleXfs` and `cellXfs` are the same complex type in two slots.** Both are
//! [`CellFormatTable`]; only the local name they stand under and their meaning differ. See
//! [`super::formats`].
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
use super::formats::CellFormatTable;
use super::named_styles::NamedCellStyles;
use super::number_formats::NumberFormatTable;
use super::table_styles::TableStyles;

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
        child(local = "numFmts", variant = NumberFormats, ty = NumberFormatTable),
        child(local = "fonts", variant = Fonts, ty = FontTable),
        child(local = "fills", variant = Fills, ty = FillTable),
        child(local = "borders", variant = Borders, ty = BorderTable),
        child(local = "cellStyleXfs", variant = CellStyleFormats, ty = CellFormatTable),
        child(local = "cellXfs", variant = CellFormats, ty = CellFormatTable),
        child(local = "cellStyles", variant = NamedStyles, ty = NamedCellStyles),
        child(local = "dxfs", variant = DifferentialFormats, ty = DifferentialFormats),
        child(local = "tableStyles", variant = TableStyles, ty = TableStyles),
        child(local = "colors", variant = Colors, ty = ColorTable)
    )]
    content: Vec<StylesheetContent>,
}

/// One child of [`StylesheetPart`]: ten modelled slots, and everything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StylesheetContent {
    /// `x:numFmts` (rank 0).
    NumberFormats(NumberFormatTable),
    /// `x:fonts` (rank 1).
    Fonts(FontTable),
    /// `x:fills` (rank 2).
    Fills(FillTable),
    /// `x:borders` (rank 3).
    Borders(BorderTable),
    /// `x:cellStyleXfs` (rank 4) — the records the **named styles** are made of.
    CellStyleFormats(CellFormatTable),
    /// `x:cellXfs` (rank 5) — the records a cell's `@s` indexes.
    CellFormats(CellFormatTable),
    /// `x:cellStyles` (rank 6).
    NamedStyles(NamedCellStyles),
    /// `x:dxfs` (rank 7).
    DifferentialFormats(DifferentialFormats),
    /// `x:tableStyles` (rank 8) — the table styles the workbook defines, and the two it prefers.
    /// **Not** the 144 Excel supplies; see [`super::table_styles`].
    TableStyles(TableStyles),
    /// `x:colors` (rank 9).
    Colors(ColorTable),
    /// The one slot this frame does not model — `extLst` — plus any foreign element, any
    /// `mc:AlternateContent`, and the text, comments and processing instructions between
    /// siblings.
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
            Self::NumberFormats(_) => "numFmts",
            Self::Fonts(_) => "fonts",
            Self::Fills(_) => "fills",
            Self::Borders(_) => "borders",
            Self::CellStyleFormats(_) => "cellStyleXfs",
            Self::CellFormats(_) => "cellXfs",
            Self::NamedStyles(_) => "cellStyles",
            Self::DifferentialFormats(_) => "dxfs",
            Self::TableStyles(_) => "tableStyles",
            Self::Colors(_) => "colors",
            Self::Raw(_) => return None,
        })
    }

    /// This child's rank in `CT_Stylesheet`'s `xsd:sequence`, from the generated table — **for an
    /// unmodelled element too**.
    ///
    /// This frame models ranks **0–9** and holds rank **10** (`extLst`) raw, so a `colors` (rank 9)
    /// inserted into a part that already writes an `extLst` has to land *before* it. Treating an
    /// unmodelled element as unranked would put it first, and `colors` would come out ahead of
    /// `numFmts`.
    ///
    /// [`WorksheetPart`](crate::WorksheetPart) once avoided the question by modelling a *prefix* of
    /// its sequence, and no longer does: its held slots are ranks 15, 31, 32 and 38, interleaved
    /// with thirty-five modelled ones. Its `Slot::rank` states the same rule this method states, and
    /// `crates/mjx-sml/tests/sheet_grid.rs` pins the interleaved case there.
    ///
    /// MJXOFF-105 modelled 1, 2, 3, 7 and 9 and held 0, 4, 5, 6, 8 and 10; MJXOFF-108 took four of
    /// those six, and MJXOFF-125 took rank 8. **The interleaving survives even now that only
    /// `extLst` is held raw**: `extLst` is rank 10 and `colors` is rank 9, so a `colors` inserted
    /// into a part that already writes an `extLst` still has to land before it, which it does only
    /// because the `extLst` is ranked.
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
/// All ten modelled slots share these three bodies, and writing them out ten times would be ten
/// chances to reach for the wrong variant.
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
            Takes the interner because placement has to rank the part's **unmodelled** slots too: \
            a raw element is ranked through the generated table by its own name, so a modelled \
            child can never be placed on the wrong side of a held one.")]
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
         style (MJXOFF-125) names by `@dxfId`. Built here because it is a resource table like the \
         other three, even though its consumers arrived later."
    );
    singleton_slot!(
        table_styles,
        table_styles_mut,
        set_table_styles,
        TableStyles,
        TableStyles,
        "tableStyles",
        "`x:tableStyles` — the table styles this workbook defines for **itself**, plus its preferred \
         default table and pivot style names. `None` is the common case and does not mean the \
         workbook has no table styles: Excel's 144 presets are in no file at all. See \
         [`TableStyles::lookup`](super::table_styles::TableStyles::lookup)."
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
    singleton_slot!(
        number_formats,
        number_formats_mut,
        set_number_formats,
        NumberFormats,
        NumberFormatTable,
        "numFmts",
        "`x:numFmts` — the number formats this workbook writes down. `None` is the common case and \
         means every `@numFmtId` in the file is one of the **implied** ids of ECMA-376 Part 1 \
         §18.8.30; see [`super::number_formats`]."
    );
    singleton_slot!(
        cell_style_formats,
        cell_style_formats_mut,
        set_cell_style_formats,
        CellStyleFormats,
        CellFormatTable,
        "cellStyleXfs",
        "`x:cellStyleXfs` — the master records the **named** cell styles are made of, and the layer \
         a `cellXfs` record sits on top of through its `@xfId`."
    );
    singleton_slot!(
        cell_formats,
        cell_formats_mut,
        set_cell_formats,
        CellFormats,
        CellFormatTable,
        "cellXfs",
        "`x:cellXfs` — the master records a cell's `@s`, a row's `@s` and a column's `@style` index. \
         §18.8.10 calls these \"the starting point for determining the formatting for a cell\"."
    );
    singleton_slot!(
        named_styles,
        named_styles_mut,
        set_named_styles,
        NamedStyles,
        NamedCellStyles,
        "cellStyles",
        "`x:cellStyles` — the named styles (\"Normal\", \"Comma\", \"Heading 1\"), each naming a \
         `cellStyleXfs` record through its `@xfId`. Not on the resolution path: a cell names an \
         index, never a name."
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

    /// **Every slot of `CT_Stylesheet` is either modelled or held, and the two add up** — derived
    /// from the read path rather than from a list.
    ///
    /// This assertion is the one MJXOFF-88 §9 B2 held up as the thing `crates/mjx-sml/src/worksheet/frame.rs`
    /// lacked, and MJXOFF-220 gave the worksheet the stronger form of it: read a part holding one of
    /// every slot the generated table names, and ask the reader which of them it typed. That is what
    /// this test does now too, so a slot modelled here without a note anywhere flips a row rather
    /// than passing a length check.
    #[test]
    fn every_slot_of_the_generated_sequence_is_accounted_for() {
        assert_eq!(STYLESHEET.symbol, "CT_Stylesheet");

        let mut markup = String::from(
            r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
        );
        for slot in STYLESHEET.slots {
            markup.push('<');
            markup.push_str(slot.local);
            markup.push_str("/>");
        }
        markup.push_str("</styleSheet>");

        let document = mjx_xml::fidelity::parse(markup.as_bytes()).expect("the part parses");
        let part = StylesheetPart::read_part(&document)
            .expect("the part reads")
            .expect("the root is an x:styleSheet");
        assert_eq!(
            part.content.len(),
            STYLESHEET.slots.len(),
            "the frame read back a different number of children than the markup held"
        );

        let mut modelled = Vec::new();
        let mut held = Vec::new();
        for (item, declared) in part.content.iter().zip(STYLESHEET.slots) {
            match item.local() {
                Some(local) => {
                    assert_eq!(local, declared.local, "rank {} is misnamed", declared.rank);
                    modelled.push(declared.local);
                }
                None => held.push(declared.local),
            }
        }
        assert_eq!(modelled.len() + held.len(), STYLESHEET.slots.len());
        assert_eq!(
            held,
            vec!["extLst"],
            "the slots this frame holds raw have changed — every artefact that states the split has \
             to change with them, starting with this file's own module documentation"
        );
        println!(
            "CT_Stylesheet: {} slots, {} modelled, {} held",
            STYLESHEET.slots.len(),
            modelled.len(),
            held.len()
        );
    }

    /// A new table lands at its **schema** rank, not at the end, and not where a comment happens to
    /// be.
    ///
    /// Every slot but `extLst` is *modelled* as of MJXOFF-125, so the interleaving this exercises is
    /// the one that is left: `extLst` is rank 10, held raw, and `colors` at rank 9 has to land
    /// **before** it. The `tableStyles` in the markup is now a modelled slot and stands where the
    /// file put it, which is what says an insertion did not move it.
    #[test]
    fn an_inserted_table_lands_at_its_rank_among_unmodelled_neighbours() {
        let markup = concat!(
            r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
            r#"<numFmts count="0"/><!-- between --><tableStyles count="0"/><extLst/>"#,
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
            vec!["numFmts", "fonts", "tableStyles", "colors", "extLst"],
            "`fonts` is rank 1 and `colors` rank 9, so `colors` lands *before* the raw `extLst` at \
             rank 10 — which is what ranking an unmodelled element by its own name buys"
        );
    }

    /// The two `xf` tables are one type in two slots, and a setter must not reach for the other's
    /// variant.
    ///
    /// The mistake this is written against is a copy-paste in `singleton_slot!`: both invocations
    /// name `CellFormatTable`, so swapping `CellFormats` for `CellStyleFormats` still compiles, and
    /// every assertion about "the part has a cellXfs" still passes.
    #[test]
    fn the_two_xf_slots_are_told_apart_by_variant_and_not_by_type() {
        let markup =
            r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"/>"#;
        let mut document = mjx_xml::fidelity::parse(markup.as_bytes()).expect("the part parses");
        let mut part = StylesheetPart::read_part(&document)
            .expect("the part reads")
            .expect("the root is an x:styleSheet");

        let cell_formats = CellFormatTable::new(
            &mut document.interner,
            None,
            super::super::formats::CellFormatTableKind::CellFormats,
        );
        let style_formats = CellFormatTable::new(
            &mut document.interner,
            None,
            super::super::formats::CellFormatTableKind::CellStyleFormats,
        );
        // Set the *later* slot first: a setter that inserted at the end would pass anyway.
        part.set_cell_formats(&document.interner, Some(cell_formats));
        part.set_cell_style_formats(&document.interner, Some(style_formats));

        let locals: Vec<&str> = part.child_element_locals(&document.interner).collect();
        assert_eq!(locals, vec!["cellStyleXfs", "cellXfs"]);
        assert!(part.cell_formats().is_some());
        assert!(part.cell_style_formats().is_some());
        assert_eq!(
            document
                .interner
                .resolve(part.cell_formats().expect("cellXfs").element_name().local),
            "cellXfs"
        );
        assert_eq!(
            document.interner.resolve(
                part.cell_style_formats()
                    .expect("cellStyleXfs")
                    .element_name()
                    .local
            ),
            "cellStyleXfs"
        );
    }
}
