//! [`SheetResources`] — what the handles in a worksheet's fragment tree resolve to.

use mjx_layout::{DecorationRef, ImageRef, SourceRef};
use mjx_layout_xlsx::{CellHit, PageCatalogue, ScaleBlend};
use mjx_scene::{Color, Decoration, FillStyle, Image, ResourceResolver};

use crate::colour::{SheetPalette, SystemRole};
use crate::fill::fill_style;

/// The resolver for one band of one worksheet.
///
/// It owns a [`PageCatalogue`] rather than borrowing one for the reason `mjx-scene-pptx` gives:
/// [`SheetBoxModel::catalogue`](mjx_layout_xlsx::SheetBoxModel::catalogue) is rebuilt on every
/// `layout_page`, and `build_scene` needs the rasteriser mutably at the same time — so a borrow of
/// the box model would have to be held across a call that takes another. A band's catalogue is a
/// handful of `Vec`s (one decoration per *distinct* effective format, not one per cell), so cloning
/// it is cheaper than the borrow it avoids.
#[derive(Clone, PartialEq, Debug)]
pub struct SheetResources {
    catalogue: PageCatalogue,
    palette: SheetPalette,
    /// `(row, column) -> decoration`, sorted, for [`ResourceResolver::text_decoration`].
    ///
    /// Built once at construction from the catalogue's own cell reports rather than looked up
    /// linearly per glyph run: a full screen is a couple of hundred cells and a couple of hundred
    /// runs, and the quadratic version of that is a scan the frame path does not need.
    text: Vec<((u32, u16), DecorationRef)>,
}

impl SheetResources {
    /// The resolver for the band `catalogue` describes, with colours resolved against `palette`.
    ///
    /// **No device scale, unlike PowerPoint's.** `mjx-scene-pptx` takes one because a DrawingML
    /// outline states its width in EMU and a `StrokeStyle` states it in device pixels. Nothing here
    /// answers a stroke at all: a cell's border is a **filled band**, positioned by the box model in
    /// the page's own coordinates (see [`mjx_layout_xlsx::border`]), so there is no length in this
    /// crate to convert. A scale parameter would be a value nothing reads.
    #[must_use]
    pub fn new(catalogue: PageCatalogue, palette: SheetPalette) -> Self {
        let mut text: Vec<((u32, u16), DecorationRef)> = catalogue
            .cells()
            .iter()
            .map(|report| ((report.row, report.column), report.decoration))
            .collect();
        text.sort_unstable_by_key(|(address, _)| *address);
        Self {
            catalogue,
            palette,
            text,
        }
    }

    /// The catalogue it answers from.
    #[must_use]
    pub fn catalogue(&self) -> &PageCatalogue {
        &self.catalogue
    }

    /// The palette its colours are resolved against.
    #[must_use]
    pub fn palette(&self) -> &SheetPalette {
        &self.palette
    }

    /// The decoration the cell at `row`, `column` carries, when it laid text out.
    ///
    /// `None` for a cell with no text — which is every cell that produces no glyph run, so the
    /// absence is never reached from [`ResourceResolver::text_decoration`].
    /// The solid colour a colour-scale rule interpolated, resolved.
    ///
    /// Both stops are resolved against this workbook's own palette and then mixed in **linear
    /// sRGB-component space**, channel by channel, alpha included.
    ///
    /// GUESS: that the mix is componentwise on the sRGB values rather than in a perceptual space.
    /// Excel's scales are visibly a straight ramp between the two swatches and nothing in ECMA-376
    /// describes the interpolation at all, so this is the reading that matches what a person sees;
    /// a perceptual blend would put a different colour in the middle of every three-stop scale.
    /// LibreOffice cannot settle it — the user has said its export of shades and gradients is not
    /// to be trusted — so this is one for the Windows sitting.
    #[must_use]
    pub fn scale_style(&self, blend: &ScaleBlend) -> Option<FillStyle> {
        let low = self.palette.resolve(&blend.low, SystemRole::Foreground)?;
        let high = self.palette.resolve(&blend.high, SystemRole::Foreground)?;
        let at = blend.fraction.clamp(0.0, 1.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let mix = |from: u8, to: u8| -> u8 {
            (f64::from(from) + at * (f64::from(to) - f64::from(from)))
                .round()
                .clamp(0.0, 255.0) as u8
        };
        Some(FillStyle::Solid(Color {
            red: mix(low.red, high.red),
            green: mix(low.green, high.green),
            blue: mix(low.blue, high.blue),
            alpha: mix(low.alpha, high.alpha),
        }))
    }

    /// The decoration the cell at `row`, `column` carries, when it laid text out.
    ///
    /// `None` for a cell with no text — which is every cell that produces no glyph run, so the
    /// absence is never reached from [`ResourceResolver::text_decoration`].
    #[must_use]
    pub fn cell_decoration(&self, row: u32, column: u16) -> Option<DecorationRef> {
        self.text
            .binary_search_by_key(&(row, column), |(address, _)| *address)
            .ok()
            .and_then(|at| self.text.get(at))
            .map(|(_, handle)| *handle)
    }
}

impl ResourceResolver for SheetResources {
    fn decoration(&self, reference: DecorationRef) -> Option<Decoration> {
        let entry = self.catalogue.decoration(reference)?;
        if let Some(band) = &entry.border_band {
            // A border band is a box the width of one line, filled with that line's colour. An edge
            // that states no `<color>` is drawn in the window's own foreground rather than dropped:
            // `<top style="hair"/>` with no colour is a border the file plainly asked for.
            //
            // **The band's style is not read here**, and that is the loss `mjx_layout_xlsx::border`
            // describes: a filled rectangle is solid, so a `dashed` edge draws as a solid line of
            // the right weight and colour. `tests/the_dash_is_lost_at_the_band.rs` asserts it.
            return Some(Decoration {
                fill: FillStyle::Solid(
                    self.palette
                        .resolve_or_system(band.colour.as_ref(), SystemRole::Foreground),
                ),
                stroke: None,
                opacity: 1.0,
                effects: Vec::new(),
            });
        }
        Some(Decoration {
            // ⚠ A colour scale **replaces** the cell's fill rather than tinting it, so it is asked
            // first. It arrives as two stops and a position between them — not as a colour —
            // because blending two `CT_Color`s needs the theme part and the workbook's
            // `indexedColors`, and a box model holds neither. This is where both halves are
            // present, which is why the interpolation happens here and nowhere else; see
            // `mjx_layout_xlsx::condfmt::graded`.
            fill: entry
                .scale_fill
                .as_ref()
                .and_then(|blend| self.scale_style(blend))
                .or_else(|| {
                    entry
                        .fill
                        .as_ref()
                        .map(|fill| fill_style(fill, &self.palette))
                })
                .unwrap_or(FillStyle::None),
            // A cell's four edges are four different lines and a decoration carries one stroke, so
            // the edges are their own fragments and this is `None` for every cell. Putting one of
            // them here would draw that edge on all four sides — which is the defect
            // `mjx-layout-pptx` names at its own `build_cell_borders`, and the reason
            // `mjx_layout_xlsx::border` exists.
            stroke: None,
            // SpreadsheetML has no per-cell opacity: a cell's transparency is the alpha of its own
            // colour, which reaches `fill` above with its alpha intact. Stating `1.0` is therefore
            // not a placeholder for a value that exists, and a `PushOpacity` this crate never emits
            // is a layer the painter never has to open.
            opacity: 1.0,
            // `x:font` carries `outline` and `shadow` booleans — Macintosh typography, two flags
            // with no radius, no colour and no direction anywhere in the schema. There is nothing to
            // build an `EffectStyle` out of, so nothing is built; see this crate's own
            // documentation.
            effects: Vec::new(),
        })
    }

    fn text_decoration(&self, source: &SourceRef) -> Option<Decoration> {
        // **This is the method PowerPoint's companion cannot answer**, and the difference is the
        // document model rather than the effort. A slide's run lives inside a paragraph inside a
        // shape and a `GlyphRunFragment` carries no handle, so `mjx-scene-pptx` has no way back to
        // the run's own `a:rPr`. A cell's text is *one string in one cell*, its fragments all carry
        // the cell's own two-segment path, and `CellHit::from_source` splits it — so the font that
        // the `xf` ladder already resolved is one lookup away.
        let hit = CellHit::from_source(source)?;
        let handle = self.cell_decoration(hit.row, hit.column)?;
        let entry = self.catalogue.decoration(handle)?;
        // ⚠ A number format's own colour wins over the font's, and it has to: `[Red]` is a statement
        // about *this value* — the negative section of `#,##0;[Red]#,##0` colours the negative cells
        // and nothing else — where `x:font/color` is a statement about the cell's style. A painter
        // that preferred the font would draw a red negative in the sheet's own black and lose the
        // one thing the format code was written for.
        let colour = match entry.text_colour {
            Some(index) => self.palette.resolve_indexed(index)?,
            None => {
                let font = entry.font.as_ref()?;
                self.palette
                    .resolve(font.color.as_ref()?, SystemRole::Foreground)?
            }
        };
        Some(Decoration {
            fill: FillStyle::Solid(colour),
            stroke: None,
            opacity: 1.0,
            effects: Vec::new(),
        })
    }

    fn image(&self, _reference: ImageRef) -> Option<Image> {
        // A worksheet's fragment tree carries no `Fragment::Image`: `mjx-layout-xlsx` lays out
        // cells and their text and issues no `ImageRef` at all, because a picture on a sheet is a
        // `xdr:twoCellAnchor` in a *drawing* part. MJXOFF-173 **places** those anchors — all three
        // modes, against the box model's own row heights and column widths — and lays out nothing
        // inside them, because a drawing's content is DrawingML and the crate that lays DrawingML
        // out sits at the same rank as the box model. So a drawing reaches the tree as a box with
        // no image handle, this is still never called, and `None` is still what it means.
        None
    }
}
