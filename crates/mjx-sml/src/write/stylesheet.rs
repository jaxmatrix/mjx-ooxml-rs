//! `xl/styles.xml`, authored: the skeleton every workbook needs, and the four appends.
//!
//! # The rule this file is written to
//!
//! **A part authored on demand writes back a root that was *read*, never one freshly constructed.**
//! `crates/mjx-xlsx/src/blank.rs`'s module documentation states it in full, with the defect that
//! taught it: `mjx-docx`'s `create_footnotes_part` wrote a fresh `Footnotes::blank()` over a parsed
//! root, the fresh value had no ancestor to inherit `xmlns:w` from, the declaration was dropped, and
//! every footnote vanished on the next open — with a green gate throughout, because the gate
//! asserted on the model rather than on the file that came back.
//!
//! So [`AuthoredStylesheet::skeleton`] writes `<styleSheet xmlns="…"/>` as **bytes**, parses them,
//! and reads a [`StylesheetPart`] out of the parsed root. Every mutation after that is a mutation of
//! a model that came from a file, and [`AuthoredStylesheet::write_into`] returns it through
//! [`ToXml::write_back`] — which is also what keeps the namespace declaration, since it lives in the
//! root's attribute vector and an attribute vector is never rebuilt.
//!
//! # The skeleton, and what each entry is for
//!
//! Six tables, and none of them is decorative:
//!
//! | table | entries | why |
//! |---|---|---|
//! | `fonts` | 1 — 11pt Calibri, family 2 | `@fontId="0"` has to resolve, or every cell's font dangles |
//! | `fills` | 2 — `none`, then `gray125` | Excel writes both, always, and index 0 must be `none`; a workbook whose fill 0 is anything else repaints every unfilled cell |
//! | `borders` | 1 — five plain edges | `@borderId="0"` has to resolve |
//! | `cellStyleXfs` | 1 — `numFmtId=0 fontId=0 fillId=0 borderId=0` | the record the `Normal` named style points at |
//! | `cellXfs` | 1 — the same, plus `xfId="0"` | the record `c@s="0"` resolves to, and the format of every cell that states none |
//! | `cellStyles` | 1 — `Normal`, `builtinId="0"` | the name Excel shows in the style gallery for `cellStyleXfs[0]` |
//!
//! This is the same skeleton `mjx_chart::EmbeddedWorkbook`'s `build_styles()` emits — MJXOFF-112's
//! parity gate compares them table by table — except that it is built out of MJXOFF-105's and
//! MJXOFF-108's models rather than out of hand-assembled [`RawElement`](mjx_ooxml_core::RawElement)s.
//!
//! # `@count` is written here, and only here
//!
//! Every table's `push` updates `@count` **only when the file already declared one**, because adding
//! the attribute to a table that had none would author markup the producer chose not to write. A
//! table this module creates has no producer but this module, so the skeleton declares `@count` on
//! each of the six explicitly and every later append maintains it.

use mjx_ooxml_core::{Interner, RawDocument, ToXml};
use mjx_ooxml_types::namespaces::SML;
use mjx_ooxml_types::spreadsheetml::PatternType;

use crate::error::SmlError;
use crate::font::FontProperties;
use crate::styles::{
    BorderTable, CellFormatTable, CellFormatTableKind, FillTable, Font, FontTable, NamedCellStyle,
    NamedCellStyles, StylesheetPart,
};

use super::constants::XML_DECLARATION;
use super::style_specs::{BorderSpec, CellFormatSpec, PatternFillSpec};

/// Which of the two `xf` tables an append goes into.
///
/// They are the *same* complex type (`CT_Xf` under two element names), and that is exactly why the
/// distinction has to be a parameter rather than two methods that could drift: `cellXfs` is what a
/// cell's `@s` indexes, `cellStyleXfs` is what a named style's `@xfId` indexes, and putting a record
/// in the wrong one silently formats nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellFormatTarget {
    /// `x:cellXfs` — the records `c@s`, `row@s` and `col@style` index.
    CellFormats,
    /// `x:cellStyleXfs` — the records a `cellStyle`'s `@xfId` indexes.
    CellStyleFormats,
}

impl CellFormatTarget {
    /// The table kind [`CellFormatTable::new`] wants.
    fn table_kind(self) -> CellFormatTableKind {
        match self {
            Self::CellFormats => CellFormatTableKind::CellFormats,
            Self::CellStyleFormats => CellFormatTableKind::CellStyleFormats,
        }
    }
}

/// `xl/styles.xml` under construction: the parsed part, and the interner its names live in.
///
/// Owns a [`RawDocument`] rather than a bare [`Interner`] because the model is a *view* over that
/// document's root — see this module's own documentation for why that indirection is the point and
/// not an accident.
#[derive(Debug)]
pub struct AuthoredStylesheet {
    document: RawDocument,
    part: StylesheetPart,
}

impl AuthoredStylesheet {
    /// The bytes a styles part is seeded from: the declaration, and an empty `styleSheet` carrying
    /// the one namespace declaration everything in it inherits.
    fn seed_bytes() -> Vec<u8> {
        format!(
            r#"{XML_DECLARATION}<styleSheet xmlns="{}"/>"#,
            SML.transitional
        )
        .into_bytes()
    }

    /// An empty `styleSheet` — no table at all, which is legal and means every index in the workbook
    /// dangles.
    ///
    /// [`skeleton`](Self::skeleton) is what a workbook actually wants; this exists for a caller
    /// assembling the six tables itself.
    ///
    /// # Errors
    /// [`SmlError::Xml`] if the seed does not parse, [`SmlError::Model`] if it does not match
    /// `CT_Stylesheet`, or [`SmlError::AuthoredPartSeedRejected`] if its root is not a
    /// `styleSheet`. None is reachable — the seed is a literal in this file — and all three are
    /// returned rather than unwrapped because a library path does not panic on anything.
    pub fn empty() -> Result<Self, SmlError> {
        let document = mjx_xml::fidelity::parse(&Self::seed_bytes())?;
        let part = StylesheetPart::read_root(&document.root, &document.interner)?.ok_or(
            SmlError::AuthoredPartSeedRejected {
                part: super::constants::STYLES_PART,
            },
        )?;
        Ok(Self { document, part })
    }

    /// The skeleton a workbook needs to open: one font, the two fills Excel always writes, one
    /// border, one `cellStyleXfs` record, one `cellXfs` record and the `Normal` cell style.
    ///
    /// See this module's documentation for the table-by-table reason each entry is there.
    ///
    /// # Errors
    /// As [`empty`](Self::empty).
    pub fn skeleton() -> Result<Self, SmlError> {
        let mut styles = Self::empty()?;
        styles.install_skeleton()?;
        Ok(styles)
    }

    /// The modelled part, for a caller reading back what was authored.
    #[must_use]
    pub fn part(&self) -> &StylesheetPart {
        &self.part
    }

    /// The interner every name in [`part`](Self::part) is interned in.
    #[must_use]
    pub fn interner(&self) -> &Interner {
        &self.document.interner
    }

    /// Appends a font to `fonts` and answers its `@fontId`.
    ///
    /// Creating the table if the part has none, so a part built by [`empty`](Self::empty) grows one
    /// on first use rather than dropping the append.
    ///
    /// # Errors
    /// [`SmlError::Xml`] if an entry of `properties.extra` — the unknown bucket a caller may have
    /// filled by hand — is not well-formed XML.
    pub fn append_font(&mut self, properties: &FontProperties) -> Result<u32, SmlError> {
        let RawDocument { interner, .. } = &mut self.document;
        let font = Font::from_properties(interner, None, properties)?;
        let mut table = match self.part.fonts() {
            Some(table) => table.clone(),
            None => new_counted_font_table(interner),
        };
        table.push(interner, font);
        let index = index_of_last(table.len());
        self.part.set_fonts(interner, Some(table));
        Ok(index)
    }

    /// Appends a pattern fill to `fills` and answers its `@fillId`.
    pub fn append_pattern_fill(&mut self, spec: &PatternFillSpec) -> u32 {
        let RawDocument { interner, .. } = &mut self.document;
        let fill = spec.build(interner, None);
        let mut table = match self.part.fills() {
            Some(table) => table.clone(),
            None => new_counted_fill_table(interner),
        };
        table.push(interner, fill);
        let index = index_of_last(table.len());
        self.part.set_fills(interner, Some(table));
        index
    }

    /// Appends a border to `borders` and answers its `@borderId`.
    pub fn append_border(&mut self, spec: &BorderSpec) -> u32 {
        let RawDocument { interner, .. } = &mut self.document;
        let border = spec.build(interner, None);
        let mut table = match self.part.borders() {
            Some(table) => table.clone(),
            None => new_counted_border_table(interner),
        };
        table.push(interner, border);
        let index = index_of_last(table.len());
        self.part.set_borders(interner, Some(table));
        index
    }

    /// Appends an `xf` to `cellXfs` or to `cellStyleXfs` and answers its index in that table.
    pub fn append_cell_format(&mut self, target: CellFormatTarget, spec: &CellFormatSpec) -> u32 {
        let RawDocument { interner, .. } = &mut self.document;
        let format = spec.build(interner, None);
        let existing = match target {
            CellFormatTarget::CellFormats => self.part.cell_formats(),
            CellFormatTarget::CellStyleFormats => self.part.cell_style_formats(),
        };
        let mut table = match existing {
            Some(table) => table.clone(),
            None => new_counted_cell_format_table(interner, target),
        };
        table.push(interner, format);
        let index = index_of_last(table.len());
        match target {
            CellFormatTarget::CellFormats => self.part.set_cell_formats(interner, Some(table)),
            CellFormatTarget::CellStyleFormats => {
                self.part.set_cell_style_formats(interner, Some(table));
            }
        }
        index
    }

    /// How many records the named `xf` table holds.
    #[must_use]
    pub fn cell_format_count(&self, target: CellFormatTarget) -> usize {
        let table = match target {
            CellFormatTarget::CellFormats => self.part.cell_formats(),
            CellFormatTarget::CellStyleFormats => self.part.cell_style_formats(),
        };
        table.map_or(0, CellFormatTable::len)
    }

    /// The whole part as bytes, with the model written back over the root it was read from.
    ///
    /// `&mut self` because [`ToXml::write_back`] needs the interner mutably — a model that authors a
    /// name has to intern it — and because the root the model returns to is the one this type owns.
    /// Calling it twice produces identical bytes; nothing here consumes anything.
    pub fn write_into(&mut self, out: &mut Vec<u8>) {
        let RawDocument { interner, root, .. } = &mut self.document;
        self.part.write_back(root, interner);
        mjx_xml::fidelity::serialize(&self.document, out);
    }

    /// The whole part as bytes. See [`write_into`](Self::write_into).
    pub fn to_part_bytes(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        self.write_into(&mut out);
        out
    }

    /// Fills the six tables of the skeleton, in `CT_Stylesheet`'s own order.
    ///
    /// The order is not this method's doing — each setter places its slot at the rank the generated
    /// [`STYLESHEET`](mjx_ooxml_types::child_order::STYLESHEET) table gives it — but building them
    /// in schema order keeps the reading of this function and of the file it writes the same.
    fn install_skeleton(&mut self) -> Result<(), SmlError> {
        self.append_font(&FontProperties {
            font_name: Some("Calibri".to_owned()),
            family: Some(2),
            size_in_points: Some(11.0),
            ..FontProperties::default()
        })?;

        for pattern in [PatternType::None, PatternType::Gray12Point5Percent] {
            self.append_pattern_fill(&PatternFillSpec {
                pattern: Some(pattern),
                foreground: None,
                background: None,
            });
        }

        self.append_border(&BorderSpec::skeleton_border());

        self.append_cell_format(
            CellFormatTarget::CellStyleFormats,
            &CellFormatSpec::skeleton_cell_style_format(),
        );
        self.append_cell_format(
            CellFormatTarget::CellFormats,
            &CellFormatSpec::skeleton_cell_format(),
        );

        let RawDocument { interner, .. } = &mut self.document;
        let mut style = NamedCellStyle::new(interner, None);
        style.set_style_name(interner, Some("Normal"));
        style.set_cell_style_format_index(interner, 0);
        style.set_builtin_id(interner, Some(0));
        let mut styles = NamedCellStyles::new(interner, None);
        styles.set_declared_count(interner, Some(0));
        styles.push(interner, style);
        self.part.set_named_styles(interner, Some(styles));
        Ok(())
    }
}

/// The index the entry just appended to a table of `len` entries has.
fn index_of_last(len: usize) -> u32 {
    u32::try_from(len.saturating_sub(1)).unwrap_or(u32::MAX)
}

/// A `fonts` table that declares `@count`, so every later `push` maintains it.
fn new_counted_font_table(interner: &mut Interner) -> FontTable {
    let mut table = FontTable::new(interner, None);
    table.set_declared_count(interner, Some(0));
    table
}

/// A `fills` table that declares `@count`.
fn new_counted_fill_table(interner: &mut Interner) -> FillTable {
    let mut table = FillTable::new(interner, None);
    table.set_declared_count(interner, Some(0));
    table
}

/// A `borders` table that declares `@count`.
fn new_counted_border_table(interner: &mut Interner) -> BorderTable {
    let mut table = BorderTable::new(interner, None);
    table.set_declared_count(interner, Some(0));
    table
}

/// A `cellXfs` or `cellStyleXfs` table that declares `@count`.
fn new_counted_cell_format_table(
    interner: &mut Interner,
    target: CellFormatTarget,
) -> CellFormatTable {
    let mut table = CellFormatTable::new(interner, None, target.table_kind());
    table.set_declared_count(interner, Some(0));
    table
}
