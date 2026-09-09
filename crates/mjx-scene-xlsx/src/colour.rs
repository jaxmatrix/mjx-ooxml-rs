//! [`SheetPalette`] — what a workbook's colours resolve against, and the one place a
//! [`mjx_sml::Color`] becomes a [`mjx_scene::Color`].
//!
//! # Why this is a table the caller supplies rather than something read here
//!
//! A SpreadsheetML colour is one element with five attributes — `auto`, `indexed`, `rgb`, `theme`,
//! `tint` — and three of the five are *indirect*. `theme="4"` is a position in
//! `xl/theme/theme1.xml`'s `<clrScheme>`; `indexed="8"` is a row of the legacy palette, which
//! `styles.xml` may replace whole; `auto="1"` is the consumer's own system colour. So a colour
//! cannot be resolved without two tables and a pair of system colours, and **none of the three is
//! in the catalogue the box model hands over** — a box model that resolved them would have opened
//! the package a second time.
//!
//! So the caller, which holds the workbook, builds this once and the resolver answers from it.
//! [`SheetPalette::from_stylesheet`] takes the indexed palette straight out of `styles.xml`;
//! the theme is a [`SchemeColors`], which is `mjx-dml`'s interner-free bridge and exactly what
//! `mjx_sml::resolve_color` wants.
//!
//! # ⚠ The MJXOFF-243 loss is **not** on this path, and that is worth stating precisely
//!
//! MJXOFF-243 records that `mjx-dml`'s `resolve_fill` / `resolve_line` / `resolve_effects` bake
//! every colour down to a `ColorSpec::Srgb` hex triplet, which has no alpha channel — so every
//! theme shadow in a `.pptx` currently renders solid. **Excel's colours do not travel that road.**
//! `mjx_sml::Color` keeps `@rgb` as the file's own eight hex digits, *alpha first*, and
//! `mjx_sml::styles::resolve_color` answers with a [`ResolvedColor`] whose `alpha` is a `f64` in
//! `0.0..=1.0`. Nothing in this crate discards it, and `tests/the_alpha_survives.rs` asserts that
//! a `rgb="80FF0000"` cell fill reaches the display list at half opacity.
//!
//! One narrower loss on the same subject **does** exist and is upstream of here:
//! [`SchemeColors::from_scheme`] resolves each theme slot and drops the slot's own alpha
//! (`let (rgb, _alpha) = …`), so a theme whose `<a:dk1>` carried an `<a:alpha>` would reach a cell
//! opaque. That is a theme-part transform rather than a cell's own colour, no workbook this project
//! has read writes one, and closing it is a change to `mjx-dml` rather than to this crate. It is
//! recorded here rather than fixed here for the same reason `mjx-scene-pptx` records its own.
//!
//! # ⚠ The legacy palette's alpha is `00`, and reading it as one would make Excel invisible
//!
//! ECMA-376 §18.8.27 prints the indexed palette with an ARGB alpha of `00` throughout — black is
//! `00000000` and red is `00FF0000` — and `mjx-sml` reports exactly what Part 1 prints rather than
//! "fixing" it, which is the right call for a model. It is the wrong number for a *painter*: read as
//! an opacity it makes every `indexed` colour in every workbook fully transparent, which is a border
//! that is not drawn and a font that is not there, with no error anywhere.
//!
//! So [`SheetPalette::resolve`] draws a colour reached through `@indexed` **opaque**. A file that
//! means transparent says so with `@rgb`, which is left exactly as written. This is the one place
//! in the crate where a value from below is deliberately overridden, and it is marked `GUESS:` at
//! the site.
//!
//! # ⚠ The system colours are a GUESS, and they are the caller's to override
//!
//! `auto="1"`, `indexed="64"` (*System Foreground*) and `indexed="65"` (*System Background*) all
//! mean *whatever the consumer's system colours are at render time*, and ECMA-376 prints no ARGB
//! for any of them. GUESS: black on white, which is what Excel's own default window is and what
//! every workbook that writes `bgColor indexed="64"` is assuming. A shell with a dark theme sets
//! its own through [`SheetPalette::with_system_colours`] rather than having one imposed on it —
//! which is the user's-document rule applied to the one value the document does not state.

use mjx_dml::{ResolvedColor, SchemeColors};
use mjx_scene::Color;
use mjx_sml::{Color as SheetColor, IndexedColorPalette};

/// Which system colour an unstated colour falls back to.
///
/// The distinction is not decoration: `auto="1"` on a font means the window's *text* colour and the
/// same attribute on a `bgColor` means the window's *background*, and answering black for both
/// would paint every cell black.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SystemRole {
    /// Text, a pattern's marks, a border — what the window draws *with*.
    Foreground,
    /// A cell's background — what the window draws *on*.
    Background,
}

/// The two tables and two system colours a worksheet's colours resolve against.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SheetPalette {
    theme: SchemeColors,
    indexed: IndexedColorPalette,
    foreground: Color,
    background: Color,
}

impl Default for SheetPalette {
    /// No theme, the default indexed palette, black on white.
    ///
    /// A workbook with no theme part resolves every `@theme` colour to nothing, which is honest:
    /// inventing a scheme would paint a cell in a colour no tier of the document states.
    fn default() -> Self {
        Self {
            theme: SchemeColors::default(),
            indexed: IndexedColorPalette::default_palette(),
            foreground: BLACK,
            background: WHITE,
        }
    }
}

/// GUESS: the system foreground — Excel's own default window text colour.
const BLACK: Color = Color {
    red: 0,
    green: 0,
    blue: 0,
    alpha: 0xff,
};

/// GUESS: the system background — Excel's own default window colour.
const WHITE: Color = Color {
    red: 0xff,
    green: 0xff,
    blue: 0xff,
    alpha: 0xff,
};

impl SheetPalette {
    /// The palette a workbook's theme and indexed table describe.
    #[must_use]
    pub fn new(theme: SchemeColors, indexed: IndexedColorPalette) -> Self {
        Self {
            theme,
            indexed,
            ..Self::default()
        }
    }

    /// The same palette resolving `@theme` positions against `theme`.
    ///
    /// Built by the caller with `mjx_dml::SchemeColors::from_scheme` over the workbook's own
    /// `xl/theme/theme1.xml`; this crate never opens a package.
    #[must_use]
    pub fn with_theme(mut self, theme: SchemeColors) -> Self {
        self.theme = theme;
        self
    }

    /// The same palette with the system colours a shell actually draws in.
    ///
    /// See the module documentation: black on white is a GUESS and not a fact, and a dark-themed
    /// shell states its own here rather than having ours imposed on it.
    #[must_use]
    pub fn with_system_colours(mut self, foreground: Color, background: Color) -> Self {
        self.foreground = foreground;
        self.background = background;
        self
    }

    /// The indexed palette a workbook's `styles.xml` declares, or the default when it declares
    /// none.
    ///
    /// ECMA-376 §18.8.27 requires an `indexedColors` block to be written **whole**, so a workbook
    /// that writes one replaces the table rather than patching it — which is what
    /// [`IndexedColorPalette::from_indexed_colors`] already does.
    #[must_use]
    pub fn from_stylesheet(
        stylesheet: &mjx_sml::StylesheetPart,
        interner: &mjx_ooxml_core::Interner,
    ) -> Self {
        let indexed = stylesheet
            .colors()
            .and_then(mjx_sml::ColorTable::indexed_colors)
            .map(|colors| IndexedColorPalette::from_indexed_colors(colors, interner))
            .unwrap_or_else(IndexedColorPalette::default_palette);
        Self {
            indexed,
            ..Self::default()
        }
    }

    /// The colour scheme it resolves `@theme` against.
    #[must_use]
    pub fn theme(&self) -> &SchemeColors {
        &self.theme
    }

    /// The indexed palette it resolves `@indexed` against.
    #[must_use]
    pub fn indexed(&self) -> &IndexedColorPalette {
        &self.indexed
    }

    /// What a system colour resolves to, in the role it is being asked for.
    #[must_use]
    pub fn system(&self, role: SystemRole) -> Color {
        match role {
            SystemRole::Foreground => self.foreground,
            SystemRole::Background => self.background,
        }
    }

    /// What `colour` paints, or `None` when the file states nothing this palette can read.
    ///
    /// `role` decides only what a *system* colour means — `auto="1"`, `indexed="64"` and
    /// `indexed="65"` — and has no effect on a colour the file states concretely.
    ///
    /// **The alpha survives.** See the module documentation: `@rgb` is `AARRGGBB` and the leading
    /// pair reaches [`mjx_scene::Color::alpha`] unchanged.
    #[must_use]
    pub fn resolve(&self, colour: &SheetColor, role: SystemRole) -> Option<Color> {
        if colour.automatic == Some(true) {
            return Some(self.system(role));
        }
        if let Some(resolved) = mjx_sml::styles::resolve_color(colour, &self.theme, &self.indexed) {
            let mut painted = scene_colour(resolved);
            if uses_the_indexed_palette(colour) {
                // GUESS, and one that has to be made *somewhere*: **every row of the legacy indexed
                // palette is printed with an alpha of `00`.** ECMA-376 §18.8.27's table gives black
                // as `00000000` and red as `00FF0000`, and `mjx-sml` reports what Part 1 prints
                // rather than "fixing" it — which is right for a model and wrong for a painter,
                // because reading those two zero nibbles as an opacity makes every legacy colour in
                // every workbook **invisible**.
                //
                // A file that means transparent says so with `@rgb`, which this leaves alone. The
                // palette's high byte is a BIFF artefact with no opacity in it, so a colour reached
                // through `@indexed` is drawn opaque. `tests/a_real_sheet_resolves.rs` asserts it
                // against `<left style="medium"><color indexed="8"/></left>`, which is a black
                // border that would otherwise not be there.
                painted.alpha = 0xff;
            }
            return Some(painted);
        }
        // `resolve_color` answers `None` for the two system rows as well as for a colour it cannot
        // read, and the two are different questions. Asking the palette directly is what tells them
        // apart: `bgColor indexed="64"` is on the second `<fill>` of practically every workbook
        // Excel has ever written, and treating it as unreadable would leave that fill unpainted.
        match colour.indexed.and_then(|index| self.indexed.lookup(index)) {
            Some(mjx_sml::IndexedColor::SystemForeground) => Some(self.system(role)),
            Some(mjx_sml::IndexedColor::SystemBackground) => Some(self.system(role)),
            _ => None,
        }
    }

    /// [`resolve`](Self::resolve), falling back to the role's system colour rather than to nothing.
    ///
    /// What a border band and a pattern's marks use: an edge the file drew but gave no colour is
    /// still an edge, and drawing it in the window's own foreground is what Excel does with a
    /// `<top style="hair"/>` that names no `<color>`. Compare [`resolve`](Self::resolve), which a
    /// *fill* uses, because a cell whose fill states no colour is not filled at all.
    #[must_use]
    pub fn resolve_or_system(&self, colour: Option<&SheetColor>, role: SystemRole) -> Color {
        colour
            .and_then(|colour| self.resolve(colour, role))
            .unwrap_or_else(|| self.system(role))
    }
}

/// Whether `colour` is reached through `@indexed` rather than through `@rgb` or `@theme`.
///
/// The order is `mjx_sml::styles::resolve_color`'s own — an explicit `rgb` beats a `theme` beats an
/// `indexed` — and it is restated rather than inferred, because "which spelling won" is not
/// something a [`ResolvedColor`] carries.
fn uses_the_indexed_palette(colour: &SheetColor) -> bool {
    colour.rgb.is_none() && colour.theme.is_none() && colour.indexed.is_some()
}

/// A resolved SpreadsheetML colour as the display list's own.
///
/// The alpha is a `f64` in `0.0..=1.0` on one side and a byte on the other; rounding rather than
/// truncating is what keeps `ff` opaque (`255.0 / 255.0 * 255.0` is not exactly `255.0` on every
/// path that produced it).
fn scene_colour(resolved: ResolvedColor) -> Color {
    Color {
        red: resolved.red,
        green: resolved.green,
        blue: resolved.blue,
        alpha: (resolved.alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    }
}
