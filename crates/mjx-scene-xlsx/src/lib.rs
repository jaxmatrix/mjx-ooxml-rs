//! Excel's companion to the box model: the trait `mjx-scene` cannot implement for itself.
//!
//! # Why this is a crate rather than a module
//!
//! [`mjx_scene::ResourceResolver`]'s own documentation says who implements it — *"the box model's
//! companion, the layer that issued the handles"* — and for Excel that layer is `mjx-layout-xlsx`.
//! It cannot be: `crates/mjx-layout-xlsx/tests/the_seam_holds.rs` refuses `mjx-scene` **by name**,
//! on the ground that *a box model that built a display list would have merged two stages the
//! architecture separates on purpose*. That gate is right, and the answer to it is a crate, not an
//! exemption. `mjx-scene-pptx` reached the same conclusion for PowerPoint first; this is not an
//! analogy to it but the same argument run again on the same seam.
//!
//! **Rank 3.7, the same rank as `mjx-scene-pptx`, and that is a decision rather than an accident.**
//! It must name the box model (3.6) for the handles and the display list (1.7) for what they resolve
//! into, so it sits above 3.6; it must stay below `mjx-view` (3.8) or a viewport would reach Excel
//! through it and stop being format-agnostic. Sharing the rank with PowerPoint's companion makes an
//! edge between the two **sideways**, which `xtask/tests/layering.rs` refuses by name — and that is
//! the edge that must never exist, exactly as it must not between the two box models at 3.6: a
//! spreadsheet's resolver has no business knowing what a slide is.
//!
//! **What the rank does not buy.** The layering gate refuses only an edge that points up or
//! sideways, so `mjx-scene-xlsx -> mjx-xlsx` (3.0), `-> mjx-session` (3.5) and `-> mjx-geometry`
//! (2.5) are legal *downward* edges and always will be. The two properties this crate has to hold —
//! **it never opens a package** and **it never paints** — are therefore held by
//! `tests/the_seam_holds.rs` and by nothing in the rank. `mjx-xlsx` is permitted in
//! `[dev-dependencies]` and nowhere else, because a suite that proves a real workbook's fills
//! resolve has to open a real workbook.
//!
//! # What a worksheet's scene is made of
//!
//! ```no_run
//! use mjx_layout::{BoxModel, PageIndex};
//! use mjx_layout_xlsx::{constraints_for, SheetBoxModel, SheetGrid};
//! use mjx_scene::{build_scene, SceneOptions};
//! use mjx_scene_xlsx::{SheetPalette, SheetResources};
//! use mjx_text::{FontResolver, GlyphAtlas};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let workbook = mjx_xlsx::Workbook::open(&std::fs::read("book.xlsx")?)?;
//! let grid = SheetGrid::read(&workbook, 0)?;
//! let constraints = constraints_for(mjx_layout::LayoutSize {
//!     width: mjx_ooxml_core::measure::Emu::from_inches(10.0),
//!     height: mjx_ooxml_core::measure::Emu::from_inches(7.0),
//! });
//!
//! let mut model = SheetBoxModel::new(FontResolver::builder().with_platform_fonts().build());
//! let page = model.layout_page(&grid, PageIndex::FIRST, &constraints, None)?;
//!
//! // The palette is the caller's, because the caller is the half that holds the package: the
//! // indexed table is in `styles.xml` and the theme is a part of its own.
//! let palette = SheetPalette::from_stylesheet(
//!     grid.formatting().stylesheet(),
//!     grid.formatting().resolver()?.formats().interner(),
//! );
//! let resources = SheetResources::new(model.catalogue().clone(), palette);
//!
//! let options = SceneOptions::new(constraints.page);
//! let mut atlas = GlyphAtlas::new();
//! let list = build_scene(
//!     page.fragments(),
//!     &resources,
//!     model.rasteriser_mut(),
//!     &mut atlas,
//!     &options,
//! )?;
//! println!("{} commands", list.commands().count());
//! # Ok(())
//! # }
//! ```
//!
//! Everything here is a **translation between two vocabularies** and nothing else. It resolves no
//! `xf` ladder — `mjx-xlsx` did that before the value reached the catalogue — and it invents no
//! geometry: a cell is a rectangle and so is a border band, which is why [`SheetGeometry`] answers
//! no handle at all.
//!
//! # ⚠ The MJXOFF-243 opacity loss is **not** on this path
//!
//! `mjx-scene-pptx` records that a DrawingML colour arrives opaque, because `mjx-dml`'s
//! `resolve_fill` bakes it to a six-digit hex triplet. **Excel's colours do not travel that road.**
//! A SpreadsheetML colour is `CT_Color` — one element with five attributes — its `@rgb` is
//! `AARRGGBB` with the alpha *first*, and `mjx_sml::styles::resolve_color` answers with a `ResolvedColor`
//! carrying that alpha as a `f64`. Nothing here discards it, and
//! `tests/the_alpha_survives.rs` asserts a half-transparent cell fill reaching the display list at
//! half opacity rather than describing that it does. See [`crate::colour`] for the one narrower
//! loss that does exist, in the theme part and one crate below.
//!
//! # ⚠ Three losses that are this path's own, all stated rather than hidden
//!
//! 1. **A border's dash.** A border band is a filled rectangle and a filled rectangle is solid, so
//!    `dashed`, `dotted` and the six other dashed styles draw as solid lines of the right weight and
//!    colour. `mjx_layout_xlsx::border` says what the fix costs;
//!    `tests/the_dash_is_lost_at_the_band.rs` asserts the loss.
//! 2. **`darkTrellis` and `lightTrellis` are drawn identically**, because DrawingML defines one
//!    trellis and SpreadsheetML two. See [`crate::fill`].
//! 3. **A font's `outline` and `shadow`.** `x:font` carries both as bare booleans, with no radius,
//!    colour or direction anywhere in the schema, so there is nothing to build an
//!    [`mjx_scene::EffectStyle`] out of. `mjx-scene-pptx` has a whole `effects` module because
//!    `a:effectLst` is a chain of eight fully-parameterised effects; SpreadsheetML has two flags,
//!    and this crate has no effects module rather than an empty one.
//!
//! None of the three is parity with Excel, and **nothing in this crate is described as such**.
//! Confirmation is a human sitting against real Microsoft Excel on Windows
//! (`docs/validation/07-the-reference-pack.md`); LibreOffice is a change detector and not a
//! reference, and the user has said its export of shades and gradients is not to be trusted at all.
//! Every reading this crate makes is marked `GUESS:` at the site that makes it.

#![forbid(unsafe_code)]

pub mod colour;
pub mod fill;
pub mod geometry;
pub mod resources;

pub use colour::{SheetPalette, SystemRole};
pub use fill::{fill_style, pattern_preset};
pub use geometry::SheetGeometry;
pub use resources::SheetResources;
