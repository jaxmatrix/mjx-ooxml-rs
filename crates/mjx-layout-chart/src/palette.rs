//! Where a series' colour comes from when the file does not state one.
//!
//! # The rule this module exists to obey
//!
//! A `c:ser` with no `c:spPr` is the common case, not the exception: Office writes one only when a
//! reader has changed a series' colour by hand. Such a series takes its fill from the **document's**
//! theme — `accent1` for the first, `accent2` for the second, and round again after the sixth — and
//! that is the whole reason this crate never writes a literal colour into a series. A chart that
//! chose its own blue would render off-palette inside a customer's branded document, and would do it
//! silently, because it would look perfectly good in isolation.
//!
//! So the palette is an **input**. Each of the three box models reads the theme its own format
//! carries and hands the six accents over; a document with no theme part at all falls back to
//! [`ChartPalette::OFFICE`], which is the Office default theme's own six accents and is used only
//! where there is nothing to defer to.
//!
//! # Why the palette is six RGB triples and not a `SchemeColors`
//!
//! `mjx_dml::resolve::SchemeColors` is the natural type and it is the wrong one here, for a layering
//! reason worth writing down: `crates/mjx-layout-docx/tests/the_seam_holds.rs` refuses `mjx-dml` by
//! name, on the ground that `mjx-docx` resolves every theme reference before a value reaches the box
//! model. A palette typed in `mjx-dml` would force Word's box model to name a crate that gate exists
//! to keep out of it. Six triples cost nothing and all three box models can build one.

/// A colour, as the three bytes a theme resolves to.
pub type Rgb = [u8; 3];

/// The six accent colours a chart hands out to series that state no fill of their own, and the two
/// neutral colours its furniture is drawn in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ChartPalette {
    /// `a:accent1` … `a:accent6`, in order.
    pub accents: [Rgb; 6],
    /// What axis lines, tick marks and tick labels are drawn in — `a:tx1` in a real theme.
    pub text: Rgb,
    /// What gridlines are ruled in.
    pub gridline: Rgb,
}

impl ChartPalette {
    /// The Office default theme's accents, for a document that carries no theme part at all.
    ///
    /// **A fallback, never an override.** Every host passes its own document's accents when the
    /// document has them; this is what is left when there is nothing to defer to, and a chart drawn
    /// from it is a chart in a file that states no palette.
    pub const OFFICE: Self = Self {
        accents: [
            [0x44, 0x72, 0xC4],
            [0xED, 0x7D, 0x31],
            [0xA5, 0xA5, 0xA5],
            [0xFF, 0xC0, 0x00],
            [0x5B, 0x9B, 0xD5],
            [0x70, 0xAD, 0x47],
        ],
        text: [0x59, 0x59, 0x59],
        gridline: [0xD9, 0xD9, 0xD9],
    };

    /// A palette from a document's own six accents, keeping [`OFFICE`](Self::OFFICE)'s neutrals.
    #[must_use]
    pub const fn from_accents(accents: [Rgb; 6]) -> Self {
        Self {
            accents,
            ..Self::OFFICE
        }
    }

    /// The colour the `index`-th series takes when it states none: `accent1 … accent6`, then round
    /// again. A chart with seven series draws the seventh in `accent1`, which is what Office does.
    #[must_use]
    pub fn accent(&self, index: usize) -> Rgb {
        self.accents[index % self.accents.len()]
    }

    /// The colour the `index`-th *point* of a plot that varies its colours takes.
    ///
    /// The same cycle as [`accent`](Self::accent). A pie chart is one series of many points and
    /// `c:varyColors` is what makes each slice a different colour; without this a pie would be one
    /// colour, which is the "the chart renders" failure this crate is written against.
    #[must_use]
    pub fn point_accent(&self, index: usize) -> Rgb {
        self.accent(index)
    }
}

impl Default for ChartPalette {
    fn default() -> Self {
        Self::OFFICE
    }
}
