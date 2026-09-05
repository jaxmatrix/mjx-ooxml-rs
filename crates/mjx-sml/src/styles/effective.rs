//! **The resolver.** What formatting a cell actually carries, once the two-level indexed
//! indirection of `styles.xml` has been walked.
//!
//! # Excel resolves unlike anything else in this workspace
//!
//! PowerPoint inherits down a placeholder chain (`crates/mjx-pptx/src/presentation/effective.rs`);
//! Word inherits down a style ladder. Excel does neither. A cell carries an **index**, that index
//! names a record, and that record carries four more indices, a fifth into a *second* table of the
//! same records, and six flags that decide which of those two layers each aspect comes from.
//! Nothing is inherited; everything is dereferenced.
//!
//! ```text
//!   c@s ─────────────► cellXfs[s] ─── @numFmtId ──► numFmts / §18.8.30's implied table
//!    │  (or row@s      (the direct    ─── @fontId ────► fonts[…]
//!    │   with          layer)         ─── @fillId ────► fills[…]
//!    │   customFormat,                ─── @borderId ──► borders[…]
//!    │   or col@style,                ─── <alignment>, <protection>
//!    │   or 0)                        │
//!    │                                └── @xfId ──► cellStyleXfs[xfId]  (the layer beneath)
//!    │                                                 the same five, for every aspect whose
//!    └─ StyleIndexSource says which    applyX on the direct record is `applyX="0"`
//! ```
//!
//! §18.8.10 states the two-layer read normatively: *"A cell can have both direct formatting (e.g.,
//! bold) and a cell style (e.g., Explanatory) applied to it. Therefore, **both** the cell style xf
//! records and cell xf records shall be read to understand the full set of formatting applied to a
//! cell."*
//!
//! # The resolution order, exactly as implemented
//!
//! **1 — which `xf` to start from** ([`cell_style_index`]), in this order:
//!
//! | source | condition | index |
//! |---|---|---|
//! | [`StyleIndexSource::Cell`] | the cell writes `@s` | that `@s` |
//! | [`StyleIndexSource::Row`] | the row writes `customFormat="1"` | the row's `@s` |
//! | [`StyleIndexSource::Column`] | a `col` run covers the column | that run's `@style` |
//! | [`StyleIndexSource::Default`] | none of the above | `0` |
//!
//! The row's gate is normative: §18.3.1.73 defines `row@s` as *"Index to style record for the row
//! (only applied if `customFormat` attribute is '1')"*. The column's is `col@style`, *"Default style
//! for the affected column(s)"*.
//!
//! **2 — which layer supplies each aspect**, independently for all six of [`FormatAspect::ALL`]:
//!
//! 1. If the direct record's `applyX` [participates](ApplyFlag::participates) — that is, it is
//!    `Applied` **or** `Unstated` — the aspect comes from the direct record.
//!    ([`FormatLayer::Direct`])
//! 2. Otherwise, if the direct record names a `cellStyleXfs` record through `@xfId` and **that**
//!    record's own `applyX` participates, the aspect comes from it. ([`FormatLayer::CellStyle`])
//! 3. Otherwise nothing supplies it. ([`FormatLayer::Neither`])
//!
//! **3 — dereference.** The winning record's `@numFmtId`, `@fontId`, `@fillId` or `@borderId` is
//! reported as [`ResolvedAspect::resource_index`]; its `<alignment>` and `<protection>` are reached
//! through [`CellFormatResolver::alignment`] and [`CellFormatResolver::protection`].
//!
//! # Where that order is *prose* and where it is *reading*
//!
//! Step 1 and §18.8.10's two-layer requirement are normative sentences. Two things in step 2 are
//! not, and are marked as such in the comparison table this child hands to MJXOFF-122:
//!
//! * **`applyX` absent behaves as applied.** §18.8.45 defines each flag in one sentence and says
//!   nothing at all about absence. The reading comes from §18.8.9's worked example — *"the 0th
//!   record does not express any 'apply' attributes, while the other records do"* — where the record
//!   expressing none is `Normal` and is applied. See [`ApplyFlag`].
//! * **The `cellStyleXfs` record's own `applyX` is honoured too** (step 2's second clause).
//!   §18.8.9 says master formatting records *"also specify whether to apply or ignore particular
//!   aspects of formatting"* and its example is entirely about `cellStyleXfs` records that suppress,
//!   so honouring them is the faithful reading — but no sentence says what happens when both layers
//!   suppress, and [`FormatLayer::Neither`] is this crate's answer rather than the specification's.
//!
//! # A place for the `dxf` layer, which is not here
//!
//! A conditionally formatted cell has a `dxf` applied **on top of** everything above (§18.8.15: *"to
//! be applied on top of or in addition to any formatting already present"*). That is MJXOFF-120's,
//! and the seam it will sit at is exactly here: the `dxf` is a delta over an
//! [`EffectiveCellFormat`], evaluated after this resolver has produced one, and every member it does
//! not state is inherited from what this resolver answered. Nothing in this module has to change for
//! that layer to arrive — [`DifferentialFormat`](super::DifferentialFormat) is already built, and
//! this type is already the thing it deltas.
//!
//! # No rendering, ever
//!
//! [`CellFormatResolver::format_code`] reports the format code in force and stops. Applying it to a
//! value — turning `0.00` and `3.14159` into `"3.14"` — is a programme non-goal, not a gap.
//!
//! # Cost
//!
//! Resolution is called per cell over a sheet that may hold millions. So every `xf` in both tables
//! is decoded **once**, when the resolver is built, into a fixed-size record; after that
//! [`CellFormatResolver::effective_cell_format`] parses nothing, allocates nothing, and returns a
//! `Copy` value. The one place an allocation can happen is
//! [`format_code`](CellFormatResolver::format_code), and only for a format code the file spelled
//! with an entity reference — `Cow::Owned` is what decoding `&quot;` costs, and it is the only
//! honest answer.
//!
//! Reading takes `&self` throughout and cannot mark a part dirty. That is not a convention here: a
//! read that triggered a reserialise would break edit isolation for every caller, and
//! `crates/mjx-sml/tests/effective_cell_format.rs` asserts the package's bytes are untouched.

use std::borrow::Cow;

use mjx_ooxml_core::{AttributeError, FromXmlError, Interner};

use crate::cells::{Cell, Row};
use crate::error::SmlError;
use crate::worksheet::ColumnBlock;

use super::borders::Border;
use super::cell_format::{CellAlignment, CellProtection, NumberFormat};
use super::fills::Fill;
use super::fonts::Font;
use super::formats::{ApplyFlag, CellFormat, FormatAspect};
use super::named_styles::{NamedCellStyle, NamedCellStyles};
use super::number_formats::{
    builtin_format_code, builtin_format_code_in, NumberFormatLanguage, NumberFormatTable,
};
use super::stylesheet::StylesheetPart;

/// Which layer of the worksheet stated the style index a resolution started from.
///
/// §18.3.1.4 (`col@style`) and §18.3.1.73 (`row@s`, `row@customFormat`) are what make this a
/// *walk* rather than a single lookup: a cell with no `@s` is not unformatted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleIndexSource {
    /// The cell's own `@s`.
    Cell,
    /// The row's `@s`, because the row writes `customFormat="1"`.
    Row,
    /// A `col` run's `@style`, because a run covers the cell's column.
    Column,
    /// Nothing stated one, so the default record — `cellXfs[0]`.
    Default,
}

/// Which of the two `xf` tables supplied one aspect of a cell's format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormatLayer {
    /// `cellXfs[s]` — the direct cell format.
    Direct,
    /// `cellStyleXfs[xf@xfId]` — the named cell style beneath it.
    CellStyle,
    /// Neither: the direct record suppressed the aspect and no record beneath it supplies one.
    ///
    /// Reachable in three ways — the direct record's `@xfId` is absent, it names a record that does
    /// not exist, or that record suppresses the aspect too. See the [module documentation](self) for
    /// why this crate answers `Neither` rather than falling back to a default.
    Neither,
}

/// One of the six aspects of an [`EffectiveCellFormat`]: where it came from, and what it points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedAspect {
    /// The `applyX` attribute of the **direct** record, in all three of its states.
    ///
    /// Reported alongside the answer rather than consumed silently, because *absent* and *false*
    /// choose the same layer only by a reading of §18.8.9 rather than by a normative sentence — see
    /// the [module documentation](self).
    pub apply_flag: ApplyFlag,
    /// The direct record's `applyX` is what chose the layer; this is the flag of the record that
    /// actually supplied the aspect. Equal to `apply_flag` when `layer` is
    /// [`FormatLayer::Direct`].
    pub supplying_apply_flag: ApplyFlag,
    /// Which layer supplied the aspect.
    pub layer: FormatLayer,
    /// The index of the record that supplied it, within `layer`'s table. `None` for
    /// [`FormatLayer::Neither`].
    pub format_index: Option<u32>,
    /// For an index-valued aspect ([`FormatAspect::is_index`]) — a number format, a font, a fill or
    /// a border — the id the supplying record states. `None` when the record omits the attribute,
    /// and always `None` for `alignment` and `protection`, whose value is an element.
    pub resource_index: Option<u32>,
    /// Whether the supplying record states the aspect at all: the attribute is present for an
    /// index-valued aspect, or the child element is present for `alignment` / `protection`.
    ///
    /// False with a `layer` of [`FormatLayer::Direct`] is an ordinary, common state — `<xf/>`
    /// applies everything and states nothing.
    pub is_stated: bool,
}

impl ResolvedAspect {
    /// The aspect nothing supplies.
    const NEITHER: Self = Self {
        apply_flag: ApplyFlag::Suppressed,
        supplying_apply_flag: ApplyFlag::Suppressed,
        layer: FormatLayer::Neither,
        format_index: None,
        resource_index: None,
        is_stated: false,
    };
}

/// The formatting a cell actually carries — `sml::EffectiveCellFormat`.
///
/// `Copy`, and built without allocating: see the [module documentation](self) for why that is the
/// design and not a micro-optimisation. Resolving a value is
/// [`CellFormatResolver::effective_cell_format`]; turning its indices into fonts, fills, borders and
/// format codes is the rest of that type's surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveCellFormat {
    style_index: u32,
    style_index_source: StyleIndexSource,
    cell_style_format_index: Option<u32>,
    aspects: [ResolvedAspect; 6],
}

impl EffectiveCellFormat {
    /// The `cellXfs` index this resolution started from.
    #[must_use]
    pub const fn style_index(&self) -> u32 {
        self.style_index
    }

    /// Which layer of the worksheet stated that index.
    #[must_use]
    pub const fn style_index_source(&self) -> StyleIndexSource {
        self.style_index_source
    }

    /// The `cellStyleXfs` index the direct record names through its `@xfId`, or `None` when it names
    /// none.
    ///
    /// Reported whether or not any aspect came from that layer, so a caller can always reach the
    /// record beneath — which is what a consumer asking "what named style is this cell in?" needs.
    #[must_use]
    pub const fn cell_style_format_index(&self) -> Option<u32> {
        self.cell_style_format_index
    }

    /// How `aspect` was resolved.
    #[must_use]
    pub fn aspect(&self, aspect: FormatAspect) -> ResolvedAspect {
        self.aspects[aspect_slot(aspect)]
    }

    /// The number format: `@numFmtId` and where it came from.
    #[must_use]
    pub fn number_format(&self) -> ResolvedAspect {
        self.aspect(FormatAspect::NumberFormat)
    }

    /// The font: `@fontId` and where it came from.
    #[must_use]
    pub fn font(&self) -> ResolvedAspect {
        self.aspect(FormatAspect::Font)
    }

    /// The fill: `@fillId` and where it came from.
    #[must_use]
    pub fn fill(&self) -> ResolvedAspect {
        self.aspect(FormatAspect::Fill)
    }

    /// The border: `@borderId` and where it came from.
    #[must_use]
    pub fn border(&self) -> ResolvedAspect {
        self.aspect(FormatAspect::Border)
    }

    /// The alignment: which record's `x:alignment` is in force.
    #[must_use]
    pub fn alignment(&self) -> ResolvedAspect {
        self.aspect(FormatAspect::Alignment)
    }

    /// The protection flags: which record's `x:protection` is in force.
    #[must_use]
    pub fn protection(&self) -> ResolvedAspect {
        self.aspect(FormatAspect::Protection)
    }
}

/// The position of `aspect` in [`EffectiveCellFormat::aspects`], matching [`FormatAspect::ALL`].
const fn aspect_slot(aspect: FormatAspect) -> usize {
    match aspect {
        FormatAspect::NumberFormat => 0,
        FormatAspect::Font => 1,
        FormatAspect::Fill => 2,
        FormatAspect::Border => 3,
        FormatAspect::Alignment => 4,
        FormatAspect::Protection => 5,
    }
}

/// One `xf`, decoded once when the resolver is built.
///
/// Fixed size and `Copy`: this is what makes per-cell resolution parse nothing.
#[derive(Debug, Clone, Copy)]
struct DecodedFormat {
    /// The `applyX` flag for each aspect, in [`FormatAspect::ALL`] order.
    apply: [ApplyFlag; 6],
    /// The stated resource index for each aspect, in the same order. Always `None` for the two
    /// element-valued aspects.
    resource_index: [Option<u32>; 6],
    /// Whether the record states the aspect: the attribute is written, or the child is present.
    is_stated: [bool; 6],
    /// `@xfId`.
    cell_style_format_index: Option<u32>,
}

impl DecodedFormat {
    /// Decodes one `xf`, once.
    fn read(format: &CellFormat, interner: &Interner) -> Result<Self, AttributeError> {
        let mut apply = [ApplyFlag::Unstated; 6];
        let mut resource_index = [None; 6];
        let mut is_stated = [false; 6];
        for aspect in FormatAspect::ALL {
            let slot = aspect_slot(aspect);
            apply[slot] = format.apply_flag(interner, aspect)?;
            if aspect.is_index() {
                let index = format.resource_index(interner, aspect)?;
                resource_index[slot] = index;
                is_stated[slot] = index.is_some();
            } else {
                is_stated[slot] = match aspect {
                    FormatAspect::Alignment => format.alignment().is_some(),
                    _ => format.protection().is_some(),
                };
            }
        }
        Ok(Self {
            apply,
            resource_index,
            is_stated,
            cell_style_format_index: format.cell_style_format_index(interner)?,
        })
    }
}

/// Resolves cell formats against one `styles.xml`.
///
/// Built once per part and then asked per cell. See the [module documentation](self) for the
/// resolution order and for the cost model.
#[derive(Debug)]
pub struct CellFormatResolver<'a> {
    interner: &'a Interner,
    cell_formats: Vec<&'a CellFormat>,
    cell_style_formats: Vec<&'a CellFormat>,
    decoded_cell_formats: Vec<DecodedFormat>,
    decoded_cell_style_formats: Vec<DecodedFormat>,
    number_formats: Vec<(u32, &'a NumberFormat)>,
    fonts: Vec<&'a Font>,
    fills: Vec<&'a Fill>,
    borders: Vec<&'a Border>,
    named_styles: Option<&'a NamedCellStyles>,
}

impl<'a> CellFormatResolver<'a> {
    /// Builds a resolver over `stylesheet`, decoding every `xf` in both tables once.
    ///
    /// A part with no `cellXfs` at all is not an error — it is what a workbook that has never been
    /// formatted looks like — and every resolution against it reports
    /// [`SmlError::CellFormatIndexOutOfRange`], which is the truthful answer rather than an invented
    /// default record.
    ///
    /// # Errors
    /// [`SmlError::Model`] if an `xf` carries an `@numFmtId`, `@fontId`, `@fillId`, `@borderId`,
    /// `@xfId` or `applyX` whose value its declared type rejects. Decoding happens **here**, once,
    /// so a malformed record is reported when the resolver is built rather than on the cell that
    /// happens to use it.
    pub fn new(stylesheet: &'a StylesheetPart, interner: &'a Interner) -> Result<Self, SmlError> {
        let cell_formats: Vec<&'a CellFormat> = stylesheet
            .cell_formats()
            .map(|table| table.formats().collect())
            .unwrap_or_default();
        let cell_style_formats: Vec<&'a CellFormat> = stylesheet
            .cell_style_formats()
            .map(|table| table.formats().collect())
            .unwrap_or_default();
        let decoded_cell_formats = decode_all(&cell_formats, interner)?;
        let decoded_cell_style_formats = decode_all(&cell_style_formats, interner)?;

        let number_formats = match stylesheet.number_formats() {
            Some(table) => number_format_index(table, interner)?,
            None => Vec::new(),
        };

        Ok(Self {
            interner,
            cell_formats,
            cell_style_formats,
            decoded_cell_formats,
            decoded_cell_style_formats,
            number_formats,
            fonts: stylesheet
                .fonts()
                .map(|table| table.fonts().collect())
                .unwrap_or_default(),
            fills: stylesheet
                .fills()
                .map(|table| table.fills().collect())
                .unwrap_or_default(),
            borders: stylesheet
                .borders()
                .map(|table| table.borders().collect())
                .unwrap_or_default(),
            named_styles: stylesheet.named_styles(),
        })
    }

    /// How many records `cellXfs` holds.
    #[must_use]
    pub fn cell_format_count(&self) -> usize {
        self.cell_formats.len()
    }

    /// How many records `cellStyleXfs` holds.
    #[must_use]
    pub fn cell_style_format_count(&self) -> usize {
        self.cell_style_formats.len()
    }

    /// Resolves the format of a cell whose style index is `style_index`, stated by `source`.
    ///
    /// The whole of step 2 and step 3 of the [module documentation](self)'s order. Parses nothing
    /// and allocates nothing.
    ///
    /// # Errors
    /// [`SmlError::CellFormatIndexOutOfRange`] if `style_index` names no record in `cellXfs`. A
    /// dangling `@xfId` on the record that *is* found is **not** an error: it is a layer that does
    /// not exist, and the aspects that wanted it report [`FormatLayer::Neither`].
    pub fn resolve(
        &self,
        style_index: u32,
        source: StyleIndexSource,
    ) -> Result<EffectiveCellFormat, SmlError> {
        let direct_slot = usize::try_from(style_index).unwrap_or(usize::MAX);
        let direct = self.decoded_cell_formats.get(direct_slot).ok_or(
            SmlError::CellFormatIndexOutOfRange {
                index: style_index,
                table: "cellXfs",
                available: self.cell_formats.len(),
            },
        )?;

        let beneath = direct.cell_style_format_index.and_then(|index| {
            let slot = usize::try_from(index).unwrap_or(usize::MAX);
            self.decoded_cell_style_formats
                .get(slot)
                .map(|decoded| (index, decoded))
        });

        let mut aspects = [ResolvedAspect::NEITHER; 6];
        for aspect in FormatAspect::ALL {
            let slot = aspect_slot(aspect);
            let apply_flag = direct.apply[slot];
            aspects[slot] = if apply_flag.participates() {
                ResolvedAspect {
                    apply_flag,
                    supplying_apply_flag: apply_flag,
                    layer: FormatLayer::Direct,
                    format_index: Some(style_index),
                    resource_index: direct.resource_index[slot],
                    is_stated: direct.is_stated[slot],
                }
            } else if let Some((index, decoded)) =
                beneath.filter(|(_, decoded)| decoded.apply[slot].participates())
            {
                ResolvedAspect {
                    apply_flag,
                    supplying_apply_flag: decoded.apply[slot],
                    layer: FormatLayer::CellStyle,
                    format_index: Some(index),
                    resource_index: decoded.resource_index[slot],
                    is_stated: decoded.is_stated[slot],
                }
            } else {
                ResolvedAspect {
                    apply_flag,
                    ..ResolvedAspect::NEITHER
                }
            };
        }

        Ok(EffectiveCellFormat {
            style_index,
            style_index_source: source,
            cell_style_format_index: direct.cell_style_format_index,
            aspects,
        })
    }

    /// The effective format of `cell`, walking cell → row → column → the default record.
    ///
    /// `column_style` is what [`column_style_index`] answered for the cell's column; pass `None`
    /// when no `col` run covers it. `cell` is `None` for a position the sheet writes no `<c>` for,
    /// which still has a format.
    ///
    /// # Errors
    /// As [`resolve`](Self::resolve).
    pub fn effective_cell_format(
        &self,
        cell: Option<&Cell<'_>>,
        row: Option<&Row<'_>>,
        column_style: Option<u32>,
    ) -> Result<EffectiveCellFormat, SmlError> {
        let (index, source) = cell_style_index(cell, row, column_style);
        self.resolve(index, source)
    }

    /// The `x:font` `format`'s font aspect resolves to, or `None` when the aspect names no font or
    /// the font table has no such entry.
    #[must_use]
    pub fn font(&self, format: &EffectiveCellFormat) -> Option<&'a Font> {
        let index = format.font().resource_index?;
        self.fonts.get(usize::try_from(index).ok()?).copied()
    }

    /// The `x:fill` `format`'s fill aspect resolves to.
    #[must_use]
    pub fn fill(&self, format: &EffectiveCellFormat) -> Option<&'a Fill> {
        let index = format.fill().resource_index?;
        self.fills.get(usize::try_from(index).ok()?).copied()
    }

    /// The `x:border` `format`'s border aspect resolves to.
    #[must_use]
    pub fn border(&self, format: &EffectiveCellFormat) -> Option<&'a Border> {
        let index = format.border().resource_index?;
        self.borders.get(usize::try_from(index).ok()?).copied()
    }

    /// The `x:alignment` in force for `format`, from whichever layer supplied the aspect.
    #[must_use]
    pub fn alignment(&self, format: &EffectiveCellFormat) -> Option<&'a CellAlignment> {
        self.record(format.alignment())?.alignment()
    }

    /// The `x:protection` in force for `format`, from whichever layer supplied the aspect.
    #[must_use]
    pub fn protection(&self, format: &EffectiveCellFormat) -> Option<&'a CellProtection> {
        self.record(format.protection())?.protection()
    }

    /// The `x:numFmt` the workbook declares for `format`'s number-format id, or `None` when the id
    /// is one of §18.8.30's implied ones — or is not declared anywhere, which a file may do and
    /// which is that file's error rather than this one's.
    #[must_use]
    pub fn number_format(&self, format: &EffectiveCellFormat) -> Option<&'a NumberFormat> {
        let id = format.number_format().resource_index?;
        self.number_formats
            .iter()
            .find_map(|(declared, entry)| (*declared == id).then_some(*entry))
    }

    /// The **format code** in force for `format`.
    ///
    /// A declared `numFmt@formatCode` wins; failing that, §18.8.30's implied table for the ids it
    /// lists under *All Languages*. `None` means the id is locale-dependent — ask
    /// [`format_code_in`](Self::format_code_in) — or that neither the file nor the specification
    /// gives it a code.
    ///
    /// **Never normalized.** The code comes back exactly as the file spelled it, spaces, quoted
    /// literals, locale prefixes and escapes included. The only transformation is XML entity
    /// decoding, which is what turns `0.000&quot;m&quot;` back into `0.000"m"` — and that is the one
    /// case where the answer is `Cow::Owned`.
    ///
    /// # Errors
    /// [`SmlError::Model`] if the declared `@formatCode` is not valid UTF-8 or carries a reference
    /// that will not decode.
    pub fn format_code(
        &self,
        format: &EffectiveCellFormat,
    ) -> Result<Option<Cow<'a, str>>, SmlError> {
        self.format_code_from(format, |id| builtin_format_code(id).map(Cow::Borrowed))
    }

    /// [`format_code`](Self::format_code) for a consumer that knows its UI language, so that the
    /// locale-dependent ids of §18.8.30 — 27–36, 50–58 and the Thai block — can be answered too.
    ///
    /// # Errors
    /// As [`format_code`](Self::format_code).
    pub fn format_code_in(
        &self,
        format: &EffectiveCellFormat,
        language: NumberFormatLanguage,
    ) -> Result<Option<Cow<'a, str>>, SmlError> {
        self.format_code_from(format, |id| {
            builtin_format_code_in(id, language).map(Cow::Borrowed)
        })
    }

    /// The named cell style that names the `cellStyleXfs` record beneath `format`, if the workbook
    /// writes one.
    ///
    /// Not on the resolution path — a cell names an index, never a name — and offered because a
    /// consumer showing "this cell is in the *Explanatory Text* style" needs it. See
    /// [`super::named_styles`].
    #[must_use]
    pub fn named_style(&self, format: &EffectiveCellFormat) -> Option<&'a NamedCellStyle> {
        let index = format.cell_style_format_index()?;
        self.named_styles?
            .by_cell_style_format_index(self.interner, index)
    }

    /// The record `aspect` was supplied by.
    fn record(&self, aspect: ResolvedAspect) -> Option<&'a CellFormat> {
        let index = usize::try_from(aspect.format_index?).ok()?;
        match aspect.layer {
            FormatLayer::Direct => self.cell_formats.get(index).copied(),
            FormatLayer::CellStyle => self.cell_style_formats.get(index).copied(),
            FormatLayer::Neither => None,
        }
    }

    /// The shared body of the two format-code readers.
    fn format_code_from(
        &self,
        format: &EffectiveCellFormat,
        builtin: impl Fn(u32) -> Option<Cow<'a, str>>,
    ) -> Result<Option<Cow<'a, str>>, SmlError> {
        let Some(id) = format.number_format().resource_index else {
            return Ok(None);
        };
        if let Some(declared) = self.number_format(format) {
            if let Some(code) = declared
                .format_code(self.interner)
                .map_err(FromXmlError::from)?
            {
                return Ok(Some(code));
            }
        }
        Ok(builtin(id))
    }
}

/// Decodes a whole `xf` table, once.
fn decode_all(
    formats: &[&CellFormat],
    interner: &Interner,
) -> Result<Vec<DecodedFormat>, SmlError> {
    formats
        .iter()
        .map(|format| {
            DecodedFormat::read(format, interner).map_err(|error| SmlError::Model(error.into()))
        })
        .collect()
}

/// Pairs every declared `numFmt` with its `@numFmtId`, once. The first entry for an id wins, which
/// is [`NumberFormatTable::get`]'s rule.
fn number_format_index<'a>(
    table: &'a NumberFormatTable,
    interner: &Interner,
) -> Result<Vec<(u32, &'a NumberFormat)>, SmlError> {
    let mut pairs = Vec::new();
    for format in table.formats() {
        let id = format
            .number_format_id(interner)
            .map_err(FromXmlError::from)?;
        if let Some(id) = id {
            if !pairs.iter().any(|(declared, _)| *declared == id) {
                pairs.push((id, format));
            }
        }
    }
    Ok(pairs)
}

/// Step 1 of the resolution order: which `cellXfs` index is in force for a cell, and which layer of
/// the worksheet said so.
///
/// `cell` is `None` for a position the sheet writes no `<c>` for; `row` is `None` for a row it
/// writes no `<row>` for; `column_style` is [`column_style_index`]'s answer for the column.
///
/// The row layer is gated on `customFormat`, because §18.3.1.73 gates it: *"Index to style record
/// for the row (only applied if `customFormat` attribute is '1')"*. A row that writes `@s` without
/// `customFormat` states an index that is **not** applied, and reporting it would be wrong.
#[must_use]
pub fn cell_style_index(
    cell: Option<&Cell<'_>>,
    row: Option<&Row<'_>>,
    column_style: Option<u32>,
) -> (u32, StyleIndexSource) {
    if let Some(cell) = cell {
        if cell.has_written_style() {
            return (cell.style(), StyleIndexSource::Cell);
        }
    }
    if let Some(row) = row {
        if row.uses_custom_format() {
            return (row.style(), StyleIndexSource::Row);
        }
    }
    if let Some(index) = column_style {
        return (index, StyleIndexSource::Column);
    }
    (0, StyleIndexSource::Default)
}

/// A worksheet's `col@style` runs, decoded once.
///
/// `CT_Col` is a **run** — `<col min="1" max="16384" style="3"/>` is one element covering every
/// column — so nothing here expands a run into per-column records; [`ColumnStyles::style_index`]
/// scans the runs, of which a real worksheet has a handful.
///
/// Decoded once for the same reason [`CellFormatResolver`] decodes the `xf` tables once: the answer
/// is wanted per cell, and re-reading three attributes off every `col` element per cell would put
/// the parser on the hot path.
#[derive(Debug, Clone, Default)]
pub struct ColumnStyles {
    /// `(first column, last column, style index)`, one-based and inclusive, in document order.
    runs: Vec<(u32, u32, u32)>,
}

impl ColumnStyles {
    /// Reads every `col` of every `cols` block a worksheet wrote.
    ///
    /// `CT_Worksheet` declares `cols` `maxOccurs="unbounded"`, so this takes all the blocks and not
    /// one; see [`ColumnBlock`]'s own documentation for why merging them would change the file.
    ///
    /// # Errors
    /// [`SmlError::Model`] if a `col` omits its required `@min` or `@max`, or states one that is not
    /// an `xsd:unsignedInt`.
    pub fn read<'a>(
        blocks: impl IntoIterator<Item = &'a ColumnBlock>,
        interner: &Interner,
    ) -> Result<Self, SmlError> {
        let mut runs = Vec::new();
        for block in blocks {
            for run in block.runs() {
                let first = run.first_column(interner).map_err(FromXmlError::from)?;
                let last = run.last_column(interner).map_err(FromXmlError::from)?;
                let style = run.style_index(interner).map_err(FromXmlError::from)?;
                runs.push((first, last, style));
            }
        }
        Ok(Self { runs })
    }

    /// The `col@style` in force for the one-based `column`.
    ///
    /// `None` when no run covers the column. `Some(0)` when a run does and states index `0` — **and
    /// also when it states no `@style` at all**, because `col@style` carries the schema default
    /// `"0"`, so the two are the same statement and this crate does not invent a difference the
    /// schema denies.
    ///
    /// A file may write overlapping runs, which is malformed. The **last** covering run wins, so
    /// that the answer does not depend on which branch of a search happens to run first — the same
    /// rule [`super::palette`] applies to a `CT_Color` that spells itself twice.
    #[must_use]
    pub fn style_index(&self, column: u32) -> Option<u32> {
        self.runs
            .iter()
            .rev()
            .find_map(|(first, last, style)| (*first..=*last).contains(&column).then_some(*style))
    }

    /// How many `col` runs the worksheet wrote.
    #[must_use]
    pub fn len(&self) -> usize {
        self.runs.len()
    }

    /// Whether the worksheet wrote no `col` at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }
}

/// [`ColumnStyles::style_index`] for a caller resolving **one** cell and holding the blocks.
///
/// A caller resolving many should build a [`ColumnStyles`] once instead: this decodes every run on
/// every call.
///
/// # Errors
/// As [`ColumnStyles::read`].
pub fn column_style_index(
    blocks: &[ColumnBlock],
    interner: &Interner,
    column: u32,
) -> Result<Option<u32>, SmlError> {
    Ok(ColumnStyles::read(blocks, interner)?.style_index(column))
}

#[cfg(test)]
mod tests {
    use mjx_ooxml_core::RawDocument;

    use super::*;

    /// Parses a whole styles part and builds a resolver-ready model.
    fn styles(markup: &str) -> (RawDocument, StylesheetPart) {
        let document = mjx_xml::fidelity::parse(markup.as_bytes()).expect("the part parses");
        let part = StylesheetPart::read_part(&document)
            .expect("the part reads")
            .expect("the root is an x:styleSheet");
        (document, part)
    }

    /// A `cellXfs` record whose `@xfId` names nothing is not an error; the aspects that wanted the
    /// layer beneath report [`FormatLayer::Neither`].
    #[test]
    fn a_dangling_cell_style_index_is_a_missing_layer_rather_than_a_failure() {
        let (document, part) = styles(concat!(
            r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
            r#"<cellStyleXfs count="1"><xf numFmtId="0" fontId="0"/></cellStyleXfs>"#,
            r#"<cellXfs count="1"><xf fontId="4" xfId="9" applyFont="0"/></cellXfs>"#,
            "</styleSheet>"
        ));
        let resolver =
            CellFormatResolver::new(&part, &document.interner).expect("the resolver builds");
        let format = resolver
            .resolve(0, StyleIndexSource::Cell)
            .expect("index 0 exists");

        assert_eq!(format.cell_style_format_index(), Some(9));
        let font = format.font();
        assert_eq!(font.apply_flag, ApplyFlag::Suppressed);
        assert_eq!(font.layer, FormatLayer::Neither);
        assert_eq!(font.resource_index, None, "font 4 was suppressed");
        assert!(!font.is_stated);
        // Every other aspect still resolves through the direct record.
        assert_eq!(format.fill().layer, FormatLayer::Direct);
    }

    /// A style index past the end of `cellXfs` is refused rather than silently answered with 0.
    #[test]
    fn a_style_index_naming_no_record_is_refused() {
        let (document, part) = styles(concat!(
            r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
            r#"<cellXfs count="1"><xf/></cellXfs>"#,
            "</styleSheet>"
        ));
        let resolver =
            CellFormatResolver::new(&part, &document.interner).expect("the resolver builds");
        assert!(resolver.resolve(0, StyleIndexSource::Default).is_ok());
        let error = resolver
            .resolve(1, StyleIndexSource::Cell)
            .expect_err("index 1 names nothing");
        assert!(matches!(
            error,
            SmlError::CellFormatIndexOutOfRange {
                index: 1,
                table: "cellXfs",
                available: 1
            }
        ));
    }

    /// The last covering `col` run wins, and a column no run covers answers `None`.
    #[test]
    fn a_column_style_comes_from_the_last_run_that_covers_the_column() {
        let markup = concat!(
            r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
            r#"<cols><col min="1" max="3" style="5"/></cols>"#,
            r#"<cols><col min="3" max="4" style="9"/><col min="6" max="6"/></cols>"#,
            "</worksheet>"
        );
        let part = crate::WorksheetPart::read_part(markup.as_bytes())
            .expect("the part reads")
            .expect("the root is an x:worksheet");
        let blocks: Vec<ColumnBlock> = part.column_blocks().cloned().collect();
        let interner = part.interner();

        assert_eq!(
            column_style_index(&blocks, interner, 1).expect("it reads"),
            Some(5)
        );
        assert_eq!(
            column_style_index(&blocks, interner, 3).expect("it reads"),
            Some(9),
            "column 3 is covered twice; the later run wins"
        );
        assert_eq!(
            column_style_index(&blocks, interner, 5).expect("it reads"),
            None,
            "no run covers column 5"
        );
        assert_eq!(
            column_style_index(&blocks, interner, 6).expect("it reads"),
            Some(0),
            "a run with no @style states the schema default, which is 0 and not `no statement`"
        );
    }
}
