//! PowerPoint's companion to the box model: the two traits `mjx-scene` cannot implement for itself.
//!
//! # Why this is a crate rather than a module
//!
//! [`mjx_scene::ResourceResolver`]'s own documentation says who implements it — *"the box model's
//! companion, the layer that issued the handles"* — and for PowerPoint that layer is
//! `mjx-layout-pptx`. It cannot be: `crates/mjx-layout-pptx/tests/the_seam_holds.rs` refuses
//! `mjx-scene` **by name**, on the ground that *a box model that built a display list would have
//! merged two stages the architecture separates on purpose*. That gate is right, and the answer to
//! it is a crate, not an exemption.
//!
//! Rank **3.7** is the only place such a crate can sit. It has to name the box model (3.6) for the
//! handles, the display list (1.7) for the vocabulary they resolve into, and the geometry tables
//! (2.5) for the outlines — and 3.7 is above all three and below `mjx-view` (3.8), so a viewport
//! still reaches a format crate through nothing.
//!
//! # What it does, and what it refuses to do
//!
//! ```no_run
//! use mjx_layout::{BoxModel, PageIndex};
//! use mjx_layout_pptx::{constraints_for, SlideBoxModel, SlideDeck};
//! use mjx_pptx::Presentation;
//! use mjx_scene::{build_scene, SceneOptions};
//! use mjx_scene_pptx::{SlideGeometry, SlideResources};
//! use mjx_text::{FontResolver, GlyphAtlas};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut presentation = Presentation::open(&std::fs::read("deck.pptx")?)?;
//! let deck = SlideDeck::read(&mut presentation)?;
//! let constraints = constraints_for(&deck);
//!
//! let mut model = SlideBoxModel::new(FontResolver::builder().with_platform_fonts().build());
//! let page = model.layout_page(&deck, PageIndex::FIRST, &constraints, None)?;
//!
//! let options = SceneOptions::new(constraints.page);
//! let resources = SlideResources::new(model.catalogue().clone(), options.device_scale);
//! let mut geometry = SlideGeometry::new();
//! geometry.register_all(model.catalogue(), |_shape| None);
//!
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
//! inheritance ladder — `mjx-pptx` did that before the value reached the catalogue — and it invents
//! no geometry: [`SlideGeometry`] reads the document's own `a:prstGeom` and hands it to
//! `mjx-geometry`'s table.
//!
//! # ⚠ Two losses at the seam below, both stated rather than hidden
//!
//! 1. **Colour opacity.** `mjx-dml`'s `resolve_fill` / `resolve_line` / `resolve_effects` bake every
//!    colour to a `ColorSpec::Srgb` hex triplet, which has no alpha channel, and each says so in its
//!    own documentation. So an `<a:alpha val="63000"/>` — which the standard Office theme puts on
//!    the shadow of every shape — arrives here as opaque. Nothing in this crate can recover it, and
//!    nothing here pretends to:
//!    `crates/mjx-scene-pptx/tests/the_opacity_is_lost_at_the_spec_boundary.rs` asserts the loss, so
//!    that fixing the seam is a test going red and being deleted rather than a thing nobody
//!    remembers. See that file for what the fix costs and why it is not this crate's.
//! 2. **A picture's crop and its adjustments.** `a:srcRect`, `a:duotone`, `a:clrChange`,
//!    `a:alphaModFix` and `a:lum` are not modelled anywhere in this workspace — `mjx-dml`'s
//!    `PictureFill` preserves them as opaque `RawNode`s and exposes the relationship id and the
//!    tile/stretch mode alone — so a picture is drawn whole and unadjusted. That is a thing to
//!    *model*, in `mjx-dml`, before it can be consumed here.
//!
//! Neither is parity with PowerPoint, and nothing in this crate is described as such. Confirmation
//! is a human sitting against real Microsoft Office on Windows
//! (`docs/validation/07-the-reference-pack.md`); LibreOffice is a change detector and not a
//! reference, and the user has said its export of shades and gradients is not to be trusted at all.

#![forbid(unsafe_code)]

pub mod effects;
pub mod geometry;
pub mod paint;
pub mod resources;

pub use effects::effect_styles;
pub use geometry::SlideGeometry;
pub use paint::{fill_style, pattern_preset, stroke_style};
pub use resources::SlideResources;
