//! `styles.xml`: the resource tables, the `xf` indirection and number formats.
//!
//! # The seam this directory is split along
//!
//! `CT_Stylesheet` (`sml.xsd:3387`) opens a cluster running to `sml.xsd:3818` — the second densest
//! in the schema after pivot tables — and it is also what makes SpreadsheetML structurally unlike
//! PowerPoint and Word: **a cell carries a style *index*, not a style**, and the index points into
//! tables that are themselves indexed by other tables.
//!
//! MJXOFF-105 (D08) builds the *resource tables* a style index resolves **into**; MJXOFF-108 (D09)
//! builds the `xf` indirection that does the resolving, and the number formats. The eleven slots of
//! [`STYLESHEET`](mjx_ooxml_types::child_order::STYLESHEET) divide along exactly that line, and
//! [`stylesheet`] holds all eleven either way.
//!
//! | Module | Subject | Filled by |
//! |---|---|---|
//! | `stylesheet.rs` | `CT_Stylesheet` itself: the eleven slots and their placement | MJXOFF-105 (D08) |
//! | `fonts.rs` | `fonts` / `font` | MJXOFF-105 (D08) |
//! | `fills.rs` | `fills` / `fill` / `patternFill` / `gradientFill` / `stop` | MJXOFF-105 (D08) |
//! | `borders.rs` | `borders` / `border` and its nine edges | MJXOFF-105 (D08) |
//! | `differential.rs` | `dxfs` / `dxf` — the *differential* formats | MJXOFF-105 (D08) |
//! | `colors.rs` | `colors` / `indexedColors` / `mruColors` / `rgbColor` | MJXOFF-105 (D08) |
//! | `palette.rs` | the legacy indexed palette, the theme position, the tint | MJXOFF-105 (D08) |
//! | `cell_format.rs` | `CT_CellAlignment`, `CT_CellProtection`, `CT_NumFmt` — shared with `CT_Xf` | MJXOFF-105 (D08), for MJXOFF-108 |
//! | `formats.rs` | `cellXfs` / `cellStyleXfs` / `xf` — the indirection, and the three-state `applyX` | MJXOFF-108 (D09) |
//! | `number_formats.rs` | `numFmts` / `numFmt`, and §18.8.30's **implied** format codes | MJXOFF-108 (D09) |
//! | `named_styles.rs` | `cellStyles` / `cellStyle`, and Annex G.2's built-in names | MJXOFF-108 (D09) |
//! | `effective.rs` | the resolver: cell → row → column, then direct `xf` → `cellStyleXfs` | MJXOFF-108 (D09) |
//!
//! It is a **directory** because those are independent vocabularies that happen to share a part: a
//! resource table is a list of values, and an `xf` is a pointer into four of them with its own
//! per-field "apply" flags. `mjx-pptx`'s `presentation.rs` reached 12,771 lines before MJXOFF-60
//! (A8) split it into subject modules; nothing here is allowed to start down that road.
//!
//! # Indices are identity
//!
//! Nothing in a workbook names a font, a fill, a border or a `dxf`. Each is addressed by its
//! **position** in its table — `fontId="3"` is the fourth `<font>` — so **reordering,
//! deduplicating or garbage-collecting a table silently repaints every cell that referred to
//! anything after the entry that moved**. Every table here therefore offers exactly three
//! operations: read one by index, iterate them, and *append*. `@count` moves with the append, and
//! only when the file declared one.
//!
//! Deduplication in particular is the "optimisation" a capable implementer reaches for, and
//! `tests/fixtures/style_resources.xlsx` writes two byte-identical `<font>` entries so that the
//! index-identity case has something real to fail on.
//!
//! # Absent is not default
//!
//! A [`DifferentialFormat`] states a *delta*: an absent member means **inherit**, never *take the
//! default*. `Option` is load-bearing on every one of its accessors, and
//! [`DifferentialFormat::inherits_everything`] is the state `<dxf/>` decodes to.
//!
//! # What MJXOFF-105 did **not** rewrite
//!
//! `CT_Font` — a font-table entry — is character for character the same fifteen font-property slots
//! as `CT_RPrElt`, a rich-text run's `rPr`, differing only in `rFont` vs `name` and in `family`'s
//! declared type. MJXOFF-97 modelled that family once, in [`crate::font`], deliberately outside both
//! subjects so that neither has to reach into the other:
//!
//! * [`FontProperties`](crate::FontProperties), with
//!   [`FontPropertyOwner::FontTableEntry`](crate::FontPropertyOwner::FontTableEntry) — which is what
//!   [`Font::properties`] decodes through, and what [`Font::from_properties`] authors from. **Not
//!   one slot had to be added**: `CT_Font`'s fifteen children and `CT_RPrElt`'s are the same list.
//! * [`Color`](crate::Color) — `CT_Color`, which every colour in this part uses: a font's, a pattern
//!   fill's `fgColor`/`bgColor`, a border edge's, a gradient stop's, an MRU entry's. It is **not**
//!   `mjx_dml::Color`, and `crates/mjx-sml/src/font/color.rs` says why in full — as does MJXOFF-105's
//!   own ticket, which instructed the opposite and was retracted.
//! * [`ColorElement`](crate::ColorElement) — that complex type as an *element*, one type for all
//!   five local names it stands under. MJXOFF-105 added it to [`crate::font`] rather than declaring
//!   a `fgColor` bag, a `bgColor` bag and a `color` bag beside MJXOFF-102's `tabColor` one, and
//!   moved `tabColor` onto it in the same change: four copies of one complex type is exactly the
//!   duplication this crate has a scheduled child to undo once already.
//!
//! The same rule now runs forward. `CT_CellAlignment`, `CT_CellProtection` and `CT_NumFmt` are
//! declared in [`cell_format`], a module belonging to neither subject, because `CT_Xf` needs all
//! three and `CT_Dxf` needed them first. **A slot MJXOFF-108's `xf` needs and one of them lacks is a
//! slot to add there.**

pub mod borders;
pub mod cell_format;
pub mod colors;
pub mod differential;
pub mod effective;
pub mod fills;
pub mod fonts;
pub mod formats;
pub mod named_styles;
pub mod number_formats;
pub mod palette;
pub mod stylesheet;

pub use borders::{
    Border, BorderContent, BorderEdge, BorderEdgeContent, BorderTable, BorderTableContent,
};
pub use cell_format::{CellAlignment, CellProtection, NumberFormat};
pub use colors::{
    ColorTable, ColorTableContent, IndexedColors, IndexedColorsContent, MruColors,
    MruColorsContent, RgbColor,
};
pub use differential::{
    DifferentialFormat, DifferentialFormatContent, DifferentialFormats, DifferentialFormatsContent,
};
pub use effective::{
    cell_style_index, column_style_index, CellFormatResolver, ColumnStyles, EffectiveCellFormat,
    FormatLayer, ResolvedAspect, StyleIndexSource,
};
pub use fills::{
    Fill, FillContent, FillTable, FillTableContent, GradientFill, GradientFillContent,
    GradientStop, GradientStopContent, PatternFill, PatternFillContent,
};
pub use fonts::{Font, FontTable, FontTableContent};
pub use formats::{
    ApplyFlag, CellFormat, CellFormatContent, CellFormatTable, CellFormatTableContent,
    CellFormatTableKind, FormatAspect,
};
pub use named_styles::{
    builtin_cell_style_name, BuiltInCellStyleName, NamedCellStyle, NamedCellStyles,
    NamedCellStylesContent,
};
pub use number_formats::{
    builtin_format_code, builtin_format_code_in, is_locale_dependent, NumberFormatLanguage,
    NumberFormatTable, NumberFormatTableContent,
};
pub use palette::{
    apply_tint, apply_tint_to_luminance, resolve_color, theme_color_slot, IndexedColor,
    IndexedColorPalette,
};
pub use stylesheet::{StylesheetContent, StylesheetPart};
