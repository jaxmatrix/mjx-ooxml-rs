//! `mjx-tokens` — the client platform's design tokens, typed for the Rust half of it.
//!
//! # Why this crate exists
//!
//! The chrome is HTML and the document canvas is Rust, and **a canvas cannot inherit a CSS custom
//! property**. Selection handles, alignment guides, rulers and marching ants are drawn by the
//! renderer, not by the browser, so they can only match the application around them if the same
//! token values reach both. One source — `docs/client-platform/data/tokens.json` — is therefore
//! generated into four artefacts by `cargo run -p xtask -- tokens`:
//!
//! | Artefact | Consumer |
//! |---|---|
//! | `ui/tokens/tokens.css` | the Web-Component chrome, as custom properties |
//! | `ui/tokens/derivations.css` | the cascade, as the `color-mix()` the derived tier came from |
//! | `ui/tokens/tokens.ts` | the shell's own logic, as typed constants |
//! | `crates/mjx-tokens/src/generated.rs` | this crate, as [`Tokens`] and [`Tokens::DEFAULTS`] |
//!
//! # Two tiers, and one `color-mix()`
//!
//! The source (MJXOFF-271) is layered rather than flat, and it is layered the way the application
//! this platform embeds into is: a handful of **seeds** and **knobs** that a host overrides, and a
//! **derived** tier mixed from them. Dark mode is the same expressions over different seeds, not a
//! second palette. [`color_mix`] is the only implementation of `color-mix(in srgb, …)` this project
//! owns; the browser's is the other party, and `ui/tokens/chromium-agreement.mjs` asserts that the
//! two agree for every derived token in both schemes rather than trusting that they do.
//!
//! The generated file is **committed**, never produced by a `build.rs` — the same doctrine
//! `mjx-ooxml-types` follows, and for the same reasons: a generated file nobody can read in review
//! or in a stack trace is a file nobody checks.
//!
//! # The resolution order
//!
//! [`resolve`] layers four steps, in this order:
//!
//! 1. **explicit configuration** — what the embedding application passed in;
//! 2. **host-supplied overrides** — the CSS custom properties read off the host element;
//! 3. **generated defaults** — [`Tokens::DEFAULTS`];
//! 4. **derivation** — every derived colour recomputed from whichever seeds and knobs won.
//!
//! That order is what makes *"if tokens are set, adopt them"* true. Dropping the platform into a
//! host that already declares `--color-paper` re-themes it with no code change, because the
//! generated names **are** that host's names: they were read from its stylesheet, not invented
//! here.
//!
//! ```
//! use mjx_tokens::{resolve, Color, TokenSource};
//!
//! // A host page that declares a different paper colour, plus a property that is not ours.
//! let host = [("--color-paper", "#fff8ec"), ("--not-a-token", "12px")];
//! let resolved = resolve([], host).expect("the host's values parse");
//!
//! assert_eq!(
//!     resolved.tokens().color.paper,
//!     Color { red: 0xff, green: 0xf8, blue: 0xec, alpha: 0xff }
//! );
//! assert_eq!(
//!     resolved.source_of("--color-paper"),
//!     Some(TokenSource::HostCustomProperty)
//! );
//! // Untouched tokens keep the generated default.
//! assert_eq!(
//!     resolved.source_of("--color-ink"),
//!     Some(TokenSource::GeneratedDefault)
//! );
//! ```
//!
//! # Contrast lives here, and the generator calls it
//!
//! `DESIGN_TOKENS.md` §2.2 measures `--color-green` at 3.39 : 1 on white — legal for a fill, not
//! for body text — and `--color-green-deep` at 5.34 : 1. Every colour token in the source declares
//! which of those it is through [`ColorUsage`], and a token tagged for text must reach
//! [`TEXT_CONTRAST_MINIMUM`] against its declared background. The measured ratio is recorded in
//! each token's docs, so the decision is auditable at the point of use.
//!
//! **The rule used to live in `xtask`'s generator alone, and MJXOFF-166's audit found the hole that
//! made.** The generator is not the only writer of `tokens.json`: the canvas harness's live token
//! editor writes back to it, and that harness may not depend on `xtask`. So a person could tweak a
//! text colour in the editor, get a green write-back, and watch `cargo run -p xtask -- tokens` go
//! red afterwards — the failure a long way from the keystroke that caused it, which is exactly what
//! the editor's validation exists to prevent, for the rule a design tweak is most likely to trip.
//!
//! The arithmetic is therefore **here**, in the one crate every writer can reach: [`contrast_ratio`],
//! [`check_usage`], and the [`ColorUsage`] and [`TokenIdentity::background`] metadata the generated
//! table now carries as *data* rather than as prose. `xtask`'s generator **calls** it rather than
//! keeping a copy; two statements of one rule is the divergence this whole pipeline exists to
//! prevent.

mod generated;

use std::borrow::Cow;
use std::fmt;

pub use generated::*;

// -------------------------------------------------------------------------------------------
// Value types
// -------------------------------------------------------------------------------------------

/// A colour: sRGB, eight bits a channel, with straight (un-premultiplied) alpha.
///
/// The channels are public because this is data — the whole crate is — and a renderer that has to
/// call four accessors to build a vertex colour is paying for nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    /// The red channel.
    pub red: u8,
    /// The green channel.
    pub green: u8,
    /// The blue channel.
    pub blue: u8,
    /// Opacity: `0xff` is opaque.
    pub alpha: u8,
}

impl Color {
    /// The colour as a non-premultiplied `[r, g, b, a]` in `0.0..=1.0`, which is the form every
    /// graphics API this platform targets wants.
    #[must_use]
    pub fn to_linear_bytes(self) -> [f32; 4] {
        [
            f32::from(self.red) / 255.0,
            f32::from(self.green) / 255.0,
            f32::from(self.blue) / 255.0,
            f32::from(self.alpha) / 255.0,
        ]
    }
}

impl fmt::Display for Color {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            red,
            green,
            blue,
            alpha,
        } = *self;
        if alpha == 0xff {
            write!(formatter, "#{red:02x}{green:02x}{blue:02x}")
        } else {
            write!(formatter, "#{red:02x}{green:02x}{blue:02x}{alpha:02x}")
        }
    }
}

/// A CSS length unit. Three, because the design-token source uses three.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LengthUnit {
    /// `px` — a CSS pixel.
    Pixels,
    /// `rem` — a multiple of the root element's font size.
    Rem,
    /// `em` — a multiple of the current element's font size.
    Em,
}

impl LengthUnit {
    /// The unit's CSS suffix.
    #[must_use]
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Pixels => "px",
            Self::Rem => "rem",
            Self::Em => "em",
        }
    }
}

/// A CSS length: a magnitude and the unit it is measured in.
///
/// The unit is kept rather than resolved to pixels because resolving it needs a root font size,
/// and the crate that has one is not this one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dimension {
    /// The magnitude, in [`Dimension::unit`].
    pub magnitude: f32,
    /// The unit the magnitude is in.
    pub unit: LengthUnit,
}

impl Dimension {
    /// The length in CSS pixels, given the font size one `rem` or `em` stands for.
    ///
    /// A `px` length ignores `font_size_in_pixels`; the parameter is taken unconditionally so a
    /// caller cannot resolve a relative length by accident while thinking it had an absolute one.
    #[must_use]
    pub fn to_pixels(self, font_size_in_pixels: f32) -> f32 {
        match self.unit {
            LengthUnit::Pixels => self.magnitude,
            LengthUnit::Rem | LengthUnit::Em => self.magnitude * font_size_in_pixels,
        }
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}{}",
            css_number(self.magnitude),
            self.unit.suffix()
        )
    }
}

/// A duration, held in milliseconds because that is the unit an animation clock counts in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Duration {
    /// The duration in milliseconds.
    pub milliseconds: f32,
}

impl fmt::Display for Duration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}ms", css_number(self.milliseconds))
    }
}

/// A percentage, held as the number CSS writes before the `%`.
///
/// The design-token source uses these for the **mix knobs** — `--theme-mix-chrome: 92%`,
/// `--theme-fill-primary-accent-mix: 16%` — the tier a host turns to re-shape every derived
/// surface at once. It is a type of its own rather than an `f32` because `92` and `92%` are
/// different CSS values, and a knob that reached a stylesheet without its unit would make the
/// `color-mix()` around it *invalid* rather than merely wrong, which is a failure that shows up as
/// an unstyled surface a long way from the token.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Percentage {
    /// The number before the `%`: `92` for `92%`.
    pub value: f32,
}

impl Percentage {
    /// The fraction this percentage is of one: `0.92` for `92%`.
    #[must_use]
    pub fn as_fraction(self) -> f64 {
        f64::from(self.value) / 100.0
    }
}

impl fmt::Display for Percentage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}%", css_number(self.value))
    }
}

/// A CSS `cubic-bezier` easing curve: the two control points, with `(0, 0)` and `(1, 1)` implied.
///
/// `x1`/`y1`/`x2`/`y2` are CSS's own names for them, which is the one place in this crate where a
/// short name is the self-explanatory one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier {
    /// The first control point's abscissa.
    pub x1: f32,
    /// The first control point's ordinate.
    pub y1: f32,
    /// The second control point's abscissa.
    pub x2: f32,
    /// The second control point's ordinate.
    pub y2: f32,
}

impl fmt::Display for CubicBezier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cubic-bezier({}, {}, {}, {})",
            css_number(self.x1),
            css_number(self.y1),
            css_number(self.x2),
            css_number(self.y2)
        )
    }
}

/// A font stack, held exactly as CSS spells it — `"Nunito Sans", system-ui, sans-serif`.
///
/// A `Cow` rather than a `&'static str` because a host may replace it at runtime, and a
/// `Vec<String>` rather than this would make [`Tokens::DEFAULTS`] impossible to write as a `const`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontStack(Cow<'static, str>);

impl FontStack {
    /// The generated defaults' constructor: a stack that is already `'static`.
    #[must_use]
    pub const fn from_static(css: &'static str) -> Self {
        Self(Cow::Borrowed(css))
    }

    /// A stack supplied at runtime.
    #[must_use]
    pub fn new(css: impl Into<String>) -> Self {
        Self(Cow::Owned(css.into()))
    }

    /// The stack as CSS text.
    #[must_use]
    pub fn css(&self) -> &str {
        &self.0
    }

    /// The family names in order, unquoted — what a text shaper wants, and what CSS text is not.
    pub fn faces(&self) -> impl Iterator<Item = &str> {
        split_css_list(&self.0).map(unquote)
    }
}

impl fmt::Display for FontStack {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A drop shadow, in the five parts CSS writes it in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    /// Horizontal offset; positive is to the right.
    pub offset_x: Dimension,
    /// Vertical offset; positive is downward.
    pub offset_y: Dimension,
    /// The blur radius.
    pub blur: Dimension,
    /// The spread radius; zero in every token this platform ships.
    pub spread: Dimension,
    /// The shadow's colour. In this platform it is always ink-tinted, never neutral black.
    pub color: Color,
}

impl fmt::Display for Shadow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A zero spread is CSS's default; writing it out adds a term a reader has to check against
        // the other four, and the struct keeps it either way.
        if self.spread.magnitude == 0.0 {
            write!(
                formatter,
                "{} {} {} {}",
                self.offset_x, self.offset_y, self.blur, self.color
            )
        } else {
            write!(
                formatter,
                "{} {} {} {} {}",
                self.offset_x, self.offset_y, self.blur, self.spread, self.color
            )
        }
    }
}

/// One token's value, whatever its type — what [`Tokens::custom_property`] answers with, and the
/// form in which a token can be compared across the artefacts.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenValue {
    /// A colour.
    Color(Color),
    /// A length.
    Dimension(Dimension),
    /// A duration.
    Duration(Duration),
    /// A unitless number, such as a line height.
    Number(f32),
    /// A font weight, `1..=1000`.
    FontWeight(u16),
    /// A font stack.
    FontStack(FontStack),
    /// An easing curve.
    CubicBezier(CubicBezier),
    /// A drop shadow.
    Shadow(Shadow),
    /// A percentage — one of the mix knobs a derivation reads.
    Percentage(Percentage),
}

impl fmt::Display for TokenValue {
    /// The value as CSS text — byte for byte what `tokens.css` declares for it, which is what
    /// makes the artefacts comparable at all.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Color(value) => value.fmt(formatter),
            Self::Dimension(value) => value.fmt(formatter),
            Self::Duration(value) => value.fmt(formatter),
            Self::Number(value) => formatter.write_str(&css_number(*value)),
            Self::FontWeight(value) => value.fmt(formatter),
            Self::FontStack(value) => value.fmt(formatter),
            Self::CubicBezier(value) => value.fmt(formatter),
            Self::Shadow(value) => value.fmt(formatter),
            Self::Percentage(value) => value.fmt(formatter),
        }
    }
}

/// One token's three names — its path in `tokens.json`, its CSS custom property, and its path in
/// `tokens.ts` — and, for a colour, the contrast metadata `DESIGN_TOKENS.md` §2.2 requires of it.
/// [`TOKENS`] is the whole table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenIdentity {
    /// The dotted path in the source and in the generated Rust, e.g. `color.ink-soft`.
    pub path: &'static str,
    /// The CSS custom property, e.g. `--color-ink-soft`.
    pub custom_property: &'static str,
    /// The path in `ui/tokens/tokens.ts`, e.g. `color.inkSoft`.
    pub typescript_path: &'static str,
    /// What this colour may be used for; `None` for every token that is not a colour.
    ///
    /// # Why this is a field rather than a sentence in the docs
    ///
    /// It was a sentence until MJXOFF-166's audit. The generated docs said
    /// `usage on-light-text · 11.75 : 1 on color.paper` for `--color-ink` and a *program* could not
    /// read it, so the only enforcement of §2.2 lived in `xtask`'s generator — which the canvas
    /// harness's live token editor may not depend on, and which therefore ran a build too late to
    /// stop a bad tweak. As data, [`check_usage`] is callable by whoever is doing the writing.
    pub usage: Option<ColorUsage>,
    /// The dotted path of the token this colour's contrast was measured against, e.g. `color.paper`.
    ///
    /// Present exactly when the source declares `$extensions.mjx.background`, which it must for
    /// every colour tagged for text — an unmeasured text colour is the rule stated rather than
    /// enforced. Resolve it with [`identity_at`].
    pub background: Option<&'static str>,
}

// -------------------------------------------------------------------------------------------
// Errors and parsing
// -------------------------------------------------------------------------------------------

/// What reading a token value from a host can fail with.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TokenError {
    /// The name is not one of the platform's tokens.
    #[error("`{custom_property}` is not a design token this platform defines")]
    UnknownCustomProperty {
        /// The name that was offered.
        custom_property: String,
    },
    /// The name is a token, but the text is not a value of its type.
    #[error("`{custom_property}` is {expected}, and `{text}` is not one: {detail}")]
    MalformedValue {
        /// The token whose value was malformed.
        custom_property: String,
        /// What that token's type is, in prose.
        expected: &'static str,
        /// The text that failed to parse.
        text: String,
        /// Why it failed.
        detail: String,
    },
}

/// A token value read from the CSS text a host declared it with.
///
/// Implemented for every type a token can have; [`parse_token`] is the entry point the generated
/// `set_custom_property` uses, which is why one impl per type is all this needs.
pub(crate) trait FromCss: Sized {
    /// What this type is, in prose, for the error message.
    const EXPECTED: &'static str;

    /// Parses the value, or says what was wrong with it.
    fn from_css(text: &str) -> Result<Self, String>;
}

/// Parses one token's value, attributing any failure to the token it belongs to.
pub(crate) fn parse_token<T: FromCss>(custom_property: &str, text: &str) -> Result<T, TokenError> {
    T::from_css(text.trim()).map_err(|detail| TokenError::MalformedValue {
        custom_property: custom_property.to_owned(),
        expected: T::EXPECTED,
        text: text.to_owned(),
        detail,
    })
}

impl FromCss for Color {
    const EXPECTED: &'static str = "a colour";

    fn from_css(text: &str) -> Result<Self, String> {
        let digits = text.strip_prefix('#').ok_or_else(|| {
            format!("`{text}` does not start with `#`; only hexadecimal colours are understood")
        })?;
        if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("`{digits}` is not hexadecimal"));
        }
        // A host's stylesheet legitimately uses the three- and four-digit shorthands, so they are
        // expanded rather than rejected. The generated defaults are always written out in full.
        let expanded: String = match digits.len() {
            3 | 4 => digits.chars().flat_map(|digit| [digit, digit]).collect(),
            6 | 8 => digits.to_owned(),
            other => {
                return Err(format!(
                    "expected 3, 4, 6 or 8 hexadecimal digits, found {other}"
                ))
            }
        };
        let channel = |at: usize| -> Result<u8, String> {
            u8::from_str_radix(&expanded[at..at + 2], 16).map_err(|error| error.to_string())
        };
        Ok(Self {
            red: channel(0)?,
            green: channel(2)?,
            blue: channel(4)?,
            alpha: if expanded.len() == 8 {
                channel(6)?
            } else {
                0xff
            },
        })
    }
}

impl FromCss for Dimension {
    const EXPECTED: &'static str = "a length";

    fn from_css(text: &str) -> Result<Self, String> {
        for unit in [LengthUnit::Rem, LengthUnit::Em, LengthUnit::Pixels] {
            if let Some(magnitude) = text.strip_suffix(unit.suffix()) {
                let magnitude = magnitude
                    .trim()
                    .parse()
                    .map_err(|_| format!("`{magnitude}` is not a number"))?;
                return Ok(Self { magnitude, unit });
            }
        }
        // `0` alone is a valid CSS length and a host may well write it.
        if text.trim() == "0" {
            return Ok(Self {
                magnitude: 0.0,
                unit: LengthUnit::Pixels,
            });
        }
        Err("it ends in none of `px`, `rem`, `em`".to_owned())
    }
}

impl FromCss for Duration {
    const EXPECTED: &'static str = "a duration";

    fn from_css(text: &str) -> Result<Self, String> {
        if let Some(milliseconds) = text.strip_suffix("ms") {
            return milliseconds
                .trim()
                .parse()
                .map(|milliseconds| Self { milliseconds })
                .map_err(|_| format!("`{milliseconds}` is not a number"));
        }
        if let Some(seconds) = text.strip_suffix('s') {
            let seconds: f32 = seconds
                .trim()
                .parse()
                .map_err(|_| format!("`{seconds}` is not a number"))?;
            return Ok(Self {
                milliseconds: seconds * 1000.0,
            });
        }
        Err("it ends in neither `ms` nor `s`".to_owned())
    }
}

impl FromCss for f32 {
    const EXPECTED: &'static str = "a number";

    fn from_css(text: &str) -> Result<Self, String> {
        text.parse().map_err(|_| "not a number".to_owned())
    }
}

impl FromCss for Percentage {
    const EXPECTED: &'static str = "a percentage";

    fn from_css(text: &str) -> Result<Self, String> {
        let number = text
            .strip_suffix('%')
            .ok_or_else(|| "it does not end in `%`".to_owned())?;
        let number = number.trim();
        Ok(Self {
            value: number
                .parse()
                .map_err(|_| format!("`{number}` is not a number"))?,
        })
    }
}

impl FromCss for u16 {
    const EXPECTED: &'static str = "a font weight";

    fn from_css(text: &str) -> Result<Self, String> {
        let weight: Self = text.parse().map_err(|_| "not a whole number".to_owned())?;
        if !(1..=1000).contains(&weight) {
            return Err(format!("{weight} is outside the CSS range 1..=1000"));
        }
        Ok(weight)
    }
}

impl FromCss for FontStack {
    const EXPECTED: &'static str = "a font stack";

    fn from_css(text: &str) -> Result<Self, String> {
        if split_css_list(text).next().is_none() {
            return Err("a font stack needs at least one family name".to_owned());
        }
        Ok(Self::new(text))
    }
}

impl FromCss for CubicBezier {
    const EXPECTED: &'static str = "an easing curve";

    fn from_css(text: &str) -> Result<Self, String> {
        let inside = text
            .strip_prefix("cubic-bezier(")
            .and_then(|rest| rest.strip_suffix(')'))
            .ok_or_else(|| "expected `cubic-bezier(a, b, c, d)`".to_owned())?;
        let mut points = [0.0_f32; 4];
        let mut written = 0;
        for part in inside.split(',') {
            let value: f32 = part
                .trim()
                .parse()
                .map_err(|_| format!("`{}` is not a number", part.trim()))?;
            *points
                .get_mut(written)
                .ok_or_else(|| "expected four control-point values".to_owned())? = value;
            written += 1;
        }
        if written != 4 {
            return Err(format!(
                "expected four control-point values, found {written}"
            ));
        }
        Ok(Self {
            x1: points[0],
            y1: points[1],
            x2: points[2],
            y2: points[3],
        })
    }
}

impl FromCss for Shadow {
    const EXPECTED: &'static str = "a drop shadow";

    fn from_css(text: &str) -> Result<Self, String> {
        let mut lengths = Vec::new();
        let mut color = None;
        for part in text.split_whitespace() {
            if part.starts_with('#') {
                if color.is_some() {
                    return Err("two colours".to_owned());
                }
                color = Some(Color::from_css(part)?);
            } else {
                lengths.push(Dimension::from_css(part)?);
            }
        }
        // Only hexadecimal colours are understood, here and in the source; a `rgba(…)` shadow is
        // rejected with this message rather than silently dropped.
        let color = color.ok_or_else(|| {
            "no `#rrggbb` colour; a shadow's colour must be written in hexadecimal".to_owned()
        })?;
        let zero = Dimension {
            magnitude: 0.0,
            unit: LengthUnit::Pixels,
        };
        match lengths.len() {
            3 => Ok(Self {
                offset_x: lengths[0],
                offset_y: lengths[1],
                blur: lengths[2],
                spread: zero,
                color,
            }),
            4 => Ok(Self {
                offset_x: lengths[0],
                offset_y: lengths[1],
                blur: lengths[2],
                spread: lengths[3],
                color,
            }),
            other => Err(format!(
                "expected three or four lengths and one colour, found {other} lengths"
            )),
        }
    }
}

// -------------------------------------------------------------------------------------------
// The resolver
// -------------------------------------------------------------------------------------------

/// Which layer of the resolution order a token's value came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenSource {
    /// The embedding application passed it in.
    ExplicitConfiguration,
    /// It was read off the host element as a CSS custom property.
    HostCustomProperty,
    /// Nobody set it, but a seed or a knob it is derived from *was*, so it was recomputed by
    /// [`Tokens::rederive`] and no longer holds its generated default.
    ///
    /// This is the step that makes a two-tier source worth having: a host that sets one seed
    /// re-themes every colour mixed from it, and this says which those were.
    Derivation,
    /// Nobody overrode it, so it is [`Tokens::DEFAULTS`].
    GeneratedDefault,
}

/// A resolved token set, and where each of its values came from.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolution {
    tokens: Tokens,
    /// Only the tokens that were overridden; everything absent is a generated default. Keyed by
    /// the `&'static` name out of [`TOKENS`], so resolving allocates nothing per token.
    overrides: Vec<(&'static str, TokenSource)>,
}

impl Resolution {
    /// The resolved values.
    #[must_use]
    pub fn tokens(&self) -> &Tokens {
        &self.tokens
    }

    /// The resolved values, taken by value — for a caller that is about to push them across a
    /// bridge and has no further use for the provenance.
    #[must_use]
    pub fn into_tokens(self) -> Tokens {
        self.tokens
    }

    /// Where one token's value came from, or `None` when the name is not a token at all.
    #[must_use]
    pub fn source_of(&self, custom_property: &str) -> Option<TokenSource> {
        if !TOKENS
            .iter()
            .any(|token| token.custom_property == custom_property)
        {
            return None;
        }
        Some(
            self.overrides
                .iter()
                .find(|(name, _)| *name == custom_property)
                .map_or(TokenSource::GeneratedDefault, |(_, source)| *source),
        )
    }

    /// Every token that was **not** left at its generated default, in the order it was applied.
    pub fn overrides(&self) -> impl Iterator<Item = (&'static str, TokenSource)> + '_ {
        self.overrides.iter().copied()
    }

    fn record(&mut self, custom_property: &'static str, source: TokenSource) {
        if let Some(slot) = self
            .overrides
            .iter_mut()
            .find(|(name, _)| *name == custom_property)
        {
            slot.1 = source;
        } else {
            self.overrides.push((custom_property, source));
        }
    }
}

impl Default for Tokens {
    fn default() -> Self {
        Self::DEFAULTS.clone()
    }
}

/// Resolves the platform's tokens in the order *explicit configuration → host-supplied overrides →
/// generated defaults*.
///
/// The two sources are treated differently on purpose, and the difference is the point:
///
/// - **A host element carries properties that are not ours.** An unknown name in
///   `host_custom_properties` is skipped, not an error.
/// - **Explicit configuration is the caller's own.** An unknown name there is a typo, and a typo
///   silently ignored is the classic theming bug — so it is [`TokenError::UnknownCustomProperty`].
///
/// A *malformed* value is an error from either source. A host that spells one of our own
/// properties wrongly should be told, and the caller can choose to carry on; a resolver that
/// quietly kept the default would leave a theme half-applied with nothing to point at.
///
/// When one source names the same property twice, the last one wins.
///
/// # Errors
///
/// [`TokenError::UnknownCustomProperty`] for a name in `explicit_configuration` that is not a
/// token, and [`TokenError::MalformedValue`] for text that is not a value of its token's type.
pub fn resolve<'a>(
    explicit_configuration: impl IntoIterator<Item = (&'a str, &'a str)>,
    host_custom_properties: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<Resolution, TokenError> {
    let mut resolution = Resolution {
        tokens: Tokens::DEFAULTS.clone(),
        overrides: Vec::new(),
    };

    // The host layer first, so explicit configuration lands on top of it.
    for (custom_property, css_text) in host_custom_properties {
        let Some(identity) = identity_of(custom_property) else {
            continue;
        };
        resolution
            .tokens
            .set_custom_property(custom_property, css_text)?;
        resolution.record(identity.custom_property, TokenSource::HostCustomProperty);
    }

    for (custom_property, css_text) in explicit_configuration {
        let identity =
            identity_of(custom_property).ok_or_else(|| TokenError::UnknownCustomProperty {
                custom_property: custom_property.to_owned(),
            })?;
        resolution
            .tokens
            .set_custom_property(custom_property, css_text)?;
        resolution.record(identity.custom_property, TokenSource::ExplicitConfiguration);
    }

    // The fourth step: every derived colour is recomputed from whichever seeds and knobs won
    // above — except the ones somebody set directly, which are an answer and not an input.
    let set_directly: Vec<&'static str> =
        resolution.overrides.iter().map(|(name, _)| *name).collect();
    let before = resolution.tokens.clone();
    resolution.tokens.rederive_except(&set_directly);
    for derivation in DERIVATIONS {
        if set_directly.contains(&derivation.custom_property) {
            continue;
        }
        // Recorded only when it actually moved: a derivation that lands back on its generated
        // default *is* its generated default, and saying otherwise would make `overrides()`
        // report the whole derived tier on every resolve.
        if before.custom_property(derivation.custom_property)
            != resolution
                .tokens
                .custom_property(derivation.custom_property)
        {
            resolution.record(derivation.custom_property, TokenSource::Derivation);
        }
    }

    Ok(resolution)
}

// -------------------------------------------------------------------------------------------
// Derivation — CSS `color-mix(in srgb, …)`, and the one implementation of it
// -------------------------------------------------------------------------------------------

/// **CSS `color-mix(in srgb, …)`, and the only implementation of it this project owns**
/// (MJXOFF-271).
///
/// The token source has two tiers: literal *seeds and knobs*, and *derived* colours expressed as a
/// mix of them. `ui/tokens/derivations.css` hands the derivation to the browser as a literal
/// `color-mix()` so that a host overriding a seed re-themes the chrome through the cascade with no
/// code; the Rust canvas cannot inherit a custom property, so it evaluates the same expression
/// here. Those are two *evaluators* — ours and Chromium's — and exactly one of them is ours.
/// `ui/tokens/chromium-agreement.mjs` asserts they agree for every derived token, in both schemes,
/// against Chromium's own `getComputedStyle`.
///
/// # The two behaviours that are easy to get wrong
///
/// Both were measured in Chromium before this was written, and both are in the CSS Color 5
/// definition rather than being quirks:
///
/// 1. **`color-mix(in srgb, C p%, transparent)` is an alpha operation, not a colour blend.**
///    `transparent` is `rgba(0, 0, 0, 0)`, and because mixing is done on *premultiplied* channels
///    its zero alpha contributes no colour at all — the result is `C`'s own channels at `p%`
///    alpha, never `C` darkened toward black. The token source uses that form constantly, and an
///    implementation that mixed un-premultiplied would darken every translucent stroke.
/// 2. **Percentages that do not sum to 100% renormalise.** The two weights are scaled to sum to
///    one, and if the *declared* sum was below 100% the result's alpha is scaled by that shortfall.
///    So `color-mix(in srgb, A 20%, B 20%)` is a half-and-half mix at 40% alpha, not a mix that
///    quietly leaves 60% of something behind.
///
/// # Precision, and why it is not a detail
///
/// A browser evaluates a whole nested expression in floating point and quantises once, when it
/// paints. An implementation that rounded to eight bits at every token boundary would differ from
/// it by a step on any expression more than one mix deep — which is not a rounding curiosity but a
/// visible seam where a chrome border meets a canvas border. So the arithmetic is
/// [`color_mix_exact`] over [`ExactColor`] and this is the one-mix convenience over it;
/// [`Tokens::rederive`] carries the exact value between derivations and quantises only what it
/// stores. `ui/tokens/chromium-agreement.mjs` caught exactly this, on five tokens, before it was
/// fixed.
///
/// Returns `None` only when both percentages are zero, which CSS calls invalid rather than
/// defining a colour for.
#[must_use]
pub fn color_mix(
    first: Color,
    first_percentage: Option<Percentage>,
    second: Color,
    second_percentage: Option<Percentage>,
) -> Option<Color> {
    color_mix_exact(
        first.to_exact(),
        first_percentage,
        second.to_exact(),
        second_percentage,
    )
    .map(Color::from_exact)
}

/// A colour part-way through a `color-mix()`: sRGB channels in `0.0..=255.0` and alpha in
/// `0.0..=1.0`, unrounded.
///
/// It exists so a nested expression is evaluated the way a browser evaluates it — once, in
/// floating point — rather than quantised at every step. See [`color_mix`].
pub type ExactColor = [f64; 4];

impl Color {
    /// This colour as an [`ExactColor`].
    #[must_use]
    pub fn to_exact(self) -> ExactColor {
        [
            f64::from(self.red),
            f64::from(self.green),
            f64::from(self.blue),
            f64::from(self.alpha) / 255.0,
        ]
    }

    /// An [`ExactColor`] quantised to eight bits a channel — what a painter, a stylesheet and the
    /// generated table all carry.
    #[must_use]
    pub fn from_exact(exact: ExactColor) -> Self {
        // Each channel is a convex combination of values in `0..=255`, so the rounded and clamped
        // result fits a `u8`; the clamp is belt and braces against a denormal reaching this.
        let byte = |value: f64| value.round().clamp(0.0, 255.0) as u8;
        Self {
            red: byte(exact[0]),
            green: byte(exact[1]),
            blue: byte(exact[2]),
            alpha: byte(exact[3] * 255.0),
        }
    }
}

/// [`color_mix`] without the quantisation — the arithmetic itself, and the only statement of it.
#[must_use]
pub fn color_mix_exact(
    first: ExactColor,
    first_percentage: Option<Percentage>,
    second: ExactColor,
    second_percentage: Option<Percentage>,
) -> Option<ExactColor> {
    let clamp = |percentage: Percentage| f64::from(percentage.value).clamp(0.0, 100.0);
    let (first_weight, second_weight) = match (first_percentage, second_percentage) {
        (None, None) => (50.0, 50.0),
        (Some(first), None) => (clamp(first), 100.0 - clamp(first)),
        (None, Some(second)) => (100.0 - clamp(second), clamp(second)),
        (Some(first), Some(second)) => (clamp(first), clamp(second)),
    };
    let sum = first_weight + second_weight;
    if sum <= 0.0 {
        return None;
    }
    // Below 100% the shortfall becomes transparency; at or above it, only the ratio matters.
    let alpha_multiplier = if sum < 100.0 { sum / 100.0 } else { 1.0 };
    let first_weight = first_weight / sum;
    let second_weight = second_weight / sum;

    let first_alpha = first[3];
    let second_alpha = second[3];
    let alpha = first_alpha.mul_add(first_weight, second_alpha * second_weight);

    let channel = |at: usize| -> f64 {
        if alpha == 0.0 {
            return 0.0;
        }
        // Premultiplied, which is what makes `transparent` an alpha operation rather than a blend
        // toward black.
        let premultiplied = first[at].mul_add(
            first_alpha * first_weight,
            second[at] * second_alpha * second_weight,
        );
        premultiplied / alpha
    };

    Some([channel(0), channel(1), channel(2), alpha * alpha_multiplier])
}

/// One side of a [`MixNode`]: what colour is being mixed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MixTerm {
    /// A colour written out in the source.
    Literal(Color),
    /// Another token, named by **its own** custom property — `--theme-light-midground`, never the
    /// scheme-relative alias.
    ///
    /// The alias is what `ui/tokens/derivations.css` writes, because a stylesheet has to work in
    /// whichever scheme is in force; this table is evaluated by a renderer that already knows which
    /// scheme it is painting, and a name that changed meaning underneath it would be a scheme bug
    /// with no symptom until the theme was switched. Both names come from one model in the
    /// generator, so they cannot describe different expressions.
    Token(&'static str),
    /// CSS's `transparent`, which is `rgba(0, 0, 0, 0)` — see [`color_mix`] for why that is an
    /// alpha operation and not a blend toward black.
    Transparent,
    /// A nested `color-mix()`, by index into the same [`DerivedFrom::Mix`] node table.
    Nested(usize),
}

/// One side's percentage: a number in the source, or one of the mix-knob tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MixPercentage {
    /// Written out in the source.
    Fixed(Percentage),
    /// A percentage token, by the custom property it is read through.
    Token(&'static str),
}

/// One `color-mix(in srgb, first p%, second q%)`.
///
/// There is no colour-space field: the source states `srgb` once, in this crate's documentation,
/// and a second space would be a second set of numbers for the four artefacts to disagree about.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MixNode {
    /// The first colour.
    pub first: MixTerm,
    /// Its percentage, or `None` when the source left it to CSS's default.
    pub first_percentage: Option<MixPercentage>,
    /// The second colour.
    pub second: MixTerm,
    /// Its percentage, or `None`.
    pub second_percentage: Option<MixPercentage>,
}

/// Where a derived token's value comes from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DerivedFrom {
    /// Another token, which is itself derived. A token that merely *names* a derived one stays a
    /// name rather than a second copy of the expression, so the two cannot come apart.
    Token(&'static str),
    /// A `color-mix()` expression tree; the **last** node is the root, because the reader appends
    /// operands before the node that consumes them.
    Mix(&'static [MixNode]),
}

/// One derived token: the custom property it produces and where its value comes from.
///
/// [`DERIVATIONS`] is the whole table, in the order the generator emitted it — which is a
/// topological order the generator *checks*, so [`Tokens::rederive`] can evaluate it in one
/// forward pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Derivation {
    /// The CSS custom property this derivation produces, e.g. `--theme-light-background`.
    pub custom_property: &'static str,
    /// Where its value comes from.
    pub source: DerivedFrom,
}

impl Tokens {
    /// Re-evaluates every derived token from whatever seeds and knobs this set now carries.
    ///
    /// This is the fourth step of the resolution order — *explicit configuration → host property →
    /// **derivation from whichever seeds won** → generated default*. [`resolve`] runs it for you;
    /// it is public because an application that mutates a seed directly needs the same step, and
    /// because leaving it implicit is how a host override would re-theme the chrome and not the
    /// canvas.
    ///
    /// Evaluation is one forward pass over [`DERIVATIONS`]: the generator refuses to emit a
    /// derivation that reads a token declared after it, so a derived token that feeds another has
    /// already been rewritten by the time the second is reached.
    pub fn rederive(&mut self) {
        self.rederive_except(&[]);
    }

    /// [`rederive`](Self::rederive), leaving the named custom properties alone.
    ///
    /// A host that overrode a derived colour *directly* meant that colour, not the expression it
    /// would otherwise have had — so re-deriving over it would silently discard the override, and
    /// the theme would be half-applied with nothing to point at.
    pub fn rederive_except(&mut self, keep: &[&str]) {
        // The exact result of every derivation evaluated in this pass, so that a derivation reading
        // another gets the browser's answer rather than a value already rounded to eight bits. See
        // `color_mix`: quantising at every token boundary put five tokens a step away from
        // Chromium, and a border is where that step shows.
        let mut exact: Vec<(&'static str, ExactColor)> = Vec::new();
        for derivation in DERIVATIONS {
            if keep.contains(&derivation.custom_property) {
                continue;
            }
            let evaluated = match derivation.source {
                DerivedFrom::Token(custom_property) => self.exact_at(&exact, custom_property),
                DerivedFrom::Mix(nodes) => self.evaluate_mix(&exact, nodes, nodes.len() - 1),
            };
            let Some(color) = evaluated else {
                continue;
            };
            exact.push((derivation.custom_property, color));
            // The generator only emits a derivation for a token whose value is a colour, so this
            // is a colour token and the write cannot fail on a type mismatch.
            let _ = self.set_custom_property(
                derivation.custom_property,
                &Color::from_exact(color).to_string(),
            );
        }
    }

    fn evaluate_mix(
        &self,
        exact: &[(&'static str, ExactColor)],
        nodes: &[MixNode],
        at: usize,
    ) -> Option<ExactColor> {
        let node = nodes.get(at)?;
        color_mix_exact(
            self.mix_term(exact, nodes, node.first)?,
            self.mix_percentage(node.first_percentage)?,
            self.mix_term(exact, nodes, node.second)?,
            self.mix_percentage(node.second_percentage)?,
        )
    }

    fn mix_term(
        &self,
        exact: &[(&'static str, ExactColor)],
        nodes: &[MixNode],
        term: MixTerm,
    ) -> Option<ExactColor> {
        match term {
            MixTerm::Literal(color) => Some(color.to_exact()),
            MixTerm::Transparent => Some([0.0, 0.0, 0.0, 0.0]),
            MixTerm::Nested(at) => self.evaluate_mix(exact, nodes, at),
            MixTerm::Token(custom_property) => self.exact_at(exact, custom_property),
        }
    }

    /// The exact value of one token: what this pass computed for it if it is itself derived, and
    /// the stored eight-bit colour otherwise.
    fn exact_at(
        &self,
        exact: &[(&'static str, ExactColor)],
        custom_property: &str,
    ) -> Option<ExactColor> {
        if let Some((_, value)) = exact.iter().find(|(name, _)| *name == custom_property) {
            return Some(*value);
        }
        self.color_at(custom_property).map(Color::to_exact)
    }

    /// The colour a token custom property holds, or `None` when it is not a colour token.
    fn color_at(&self, custom_property: &str) -> Option<Color> {
        match self.custom_property(custom_property)? {
            TokenValue::Color(color) => Some(color),
            _ => None,
        }
    }

    /// `None` means *the source wrote no percentage*, which CSS reads as a default rather than as
    /// a failure — so an absent percentage and an unreadable one must not be confused, and the
    /// outer `Option` is what keeps them apart.
    fn mix_percentage(&self, percentage: Option<MixPercentage>) -> Option<Option<Percentage>> {
        match percentage {
            None => Some(None),
            Some(MixPercentage::Fixed(value)) => Some(Some(value)),
            Some(MixPercentage::Token(custom_property)) => {
                match self.custom_property(custom_property)? {
                    TokenValue::Percentage(value) => Some(Some(value)),
                    _ => None,
                }
            }
        }
    }
}

// -------------------------------------------------------------------------------------------
// Contrast — `DESIGN_TOKENS.md` §2.2, as code every writer of the token source can reach
// -------------------------------------------------------------------------------------------

/// The WCAG 2.2 AA contrast minimum for body text.
///
/// Large text and user-interface components have lower minima and this crate deliberately does not
/// model them: a token tagged for *text* is tagged for the hardest case it will be put to, and a
/// second, laxer tier would only be a way to pass.
pub const TEXT_CONTRAST_MINIMUM: f64 = 4.5;

/// The relative luminance either side of which a surface counts as light or dark.
///
/// Used to check that an [`ColorUsage::OnLightText`] tag actually names a light background: a tag
/// that named the wrong surface would pass the ratio check against the wrong colour.
pub const LIGHT_SURFACE_LUMINANCE: f64 = 0.5;

/// What a colour token may be used for. **There is no default.**
///
/// `DESIGN_TOKENS.md` §2.2 calls colouring text with a fill-only accent *"the most likely
/// accessibility defect in the chrome"*, so an untagged colour is refused rather than defaulted to
/// [`ColorUsage::FillOnly`], which would make the omission invisible.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ColorUsage {
    /// Legal as text on the declared light background, which means it meets
    /// [`TEXT_CONTRAST_MINIMUM`] against it.
    OnLightText,
    /// Legal as text on the declared dark background.
    OnDarkText,
    /// Fills, borders, indicators and icons — never the colour of a glyph.
    FillOnly,
}

impl ColorUsage {
    /// Every usage, in the order a failure lists them.
    pub const ALL: [Self; 3] = [Self::OnLightText, Self::OnDarkText, Self::FillOnly];

    /// The spelling the token source uses, which is also what a failure quotes back.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OnLightText => "on-light-text",
            Self::OnDarkText => "on-dark-text",
            Self::FillOnly => "fill-only",
        }
    }

    /// Whether the token is allowed to colour text, which is what makes the contrast minimum
    /// binding.
    #[must_use]
    pub const fn colours_text(self) -> bool {
        matches!(self, Self::OnLightText | Self::OnDarkText)
    }

    /// The usage a source spelling names, or `None`.
    ///
    /// Deliberately not an error type: the *rule* lives in this crate and the *message* for an
    /// unreadable source file belongs to whoever is reading that file, which is the generator.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|usage| usage.as_str() == text)
    }
}

impl fmt::Display for ColorUsage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Why a colour tagged for text may not carry that tag.
///
/// Three refusals rather than one string, because they are three different mistakes: a translucent
/// text colour was measured against a surface it is never seen on; a tag naming the wrong surface
/// measured a correct ratio against the wrong colour; and a ratio below the minimum is the rule
/// itself. Only the third is fixed by choosing a different colour.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum ContrastError {
    /// The colour is tagged for text and is not opaque.
    #[error(
        "is tagged `{usage}` but is not opaque; text is drawn at full alpha, so a translucent text \
         colour is measured against a surface it is never seen on"
    )]
    Translucent {
        /// The usage the token declared.
        usage: ColorUsage,
    },
    /// The declared background is light where the tag says dark, or the other way round.
    #[error(
        "is tagged `{usage}`, but its declared background `{background_path}` ({background}) has a \
         relative luminance of {luminance:.3}, which is {surface}. The tag names the wrong surface"
    )]
    WrongSurface {
        /// The usage the token declared.
        usage: ColorUsage,
        /// The token path of the declared background.
        background_path: String,
        /// That background's CSS text.
        background: String,
        /// Its measured relative luminance.
        luminance: f64,
        /// What that luminance makes the background: `light` or `dark`.
        surface: &'static str,
    },
    /// The measured ratio is below [`TEXT_CONTRAST_MINIMUM`].
    #[error(
        "is tagged `{usage}`, but {colour} on `{background_path}` ({background}) measures \
         {ratio:.2} : 1, below the {} : 1 WCAG AA minimum for body text. DESIGN_TOKENS.md §2.2: \
         use this colour for fills, borders, indicators and icons, and the deep step of the same \
         ramp for accent-coloured text",
        TEXT_CONTRAST_MINIMUM
    )]
    BelowMinimum {
        /// The usage the token declared.
        usage: ColorUsage,
        /// The colour's CSS text.
        colour: String,
        /// The token path of the declared background.
        background_path: String,
        /// That background's CSS text.
        background: String,
        /// The measured ratio.
        ratio: f64,
    },
}

impl Color {
    /// This colour composited over `background` — what a partially transparent colour actually
    /// looks like, and therefore what its contrast has to be measured on.
    #[must_use]
    pub fn over(self, background: Self) -> Self {
        if self.alpha == 0xff {
            return self;
        }
        let alpha = f64::from(self.alpha) / 255.0;
        let mix = |over: u8, under: u8| -> u8 {
            let blended = f64::from(over).mul_add(alpha, f64::from(under) * (1.0 - alpha));
            // A convex combination of two values in `0..=255`, so the rounded result is in range
            // and fits a `u8`.
            blended.round().clamp(0.0, 255.0) as u8
        };
        Self {
            red: mix(self.red, background.red),
            green: mix(self.green, background.green),
            blue: mix(self.blue, background.blue),
            alpha: 0xff,
        }
    }

    /// WCAG 2.2 relative luminance.
    #[must_use]
    pub fn relative_luminance(self) -> f64 {
        let linear = |channel: u8| -> f64 {
            let value = f64::from(channel) / 255.0;
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126_f64.mul_add(
            linear(self.red),
            0.7152_f64.mul_add(linear(self.green), 0.0722 * linear(self.blue)),
        )
    }
}

/// The WCAG 2.2 contrast ratio between a foreground and a background.
///
/// The foreground is composited over the background first when it is not opaque, because a
/// translucent colour's contrast is the contrast of what is actually seen.
#[must_use]
pub fn contrast_ratio(foreground: Color, background: Color) -> f64 {
    let foreground = foreground.over(background).relative_luminance();
    let background = background.relative_luminance();
    let (lighter, darker) = if foreground >= background {
        (foreground, background)
    } else {
        (background, foreground)
    };
    (lighter + 0.05) / (darker + 0.05)
}

/// **The contrast rule itself.** `DESIGN_TOKENS.md` §2.2 in one function.
///
/// A [`ColorUsage::FillOnly`] colour is unconstrained and passes; a colour tagged for text must be
/// opaque, must name a background of the right lightness, and must reach [`TEXT_CONTRAST_MINIMUM`]
/// against it.
///
/// # Errors
///
/// [`ContrastError`], which quotes the measurement rather than only refusing.
pub fn check_usage(
    usage: ColorUsage,
    colour: Color,
    background: Color,
    background_path: &str,
) -> Result<(), ContrastError> {
    if !usage.colours_text() {
        return Ok(());
    }
    if colour.alpha != 0xff {
        return Err(ContrastError::Translucent { usage });
    }
    let luminance = background.relative_luminance();
    let background_is_light = luminance >= LIGHT_SURFACE_LUMINANCE;
    if background_is_light != (usage == ColorUsage::OnLightText) {
        return Err(ContrastError::WrongSurface {
            usage,
            background_path: background_path.to_owned(),
            background: background.to_string(),
            luminance,
            surface: if background_is_light { "light" } else { "dark" },
        });
    }
    let ratio = contrast_ratio(colour, background);
    if ratio < TEXT_CONTRAST_MINIMUM {
        return Err(ContrastError::BelowMinimum {
            usage,
            colour: colour.to_string(),
            background_path: background_path.to_owned(),
            background: background.to_string(),
            ratio,
        });
    }
    Ok(())
}

/// The table row for one custom property, if the platform defines it.
#[must_use]
pub fn identity_of(custom_property: &str) -> Option<&'static TokenIdentity> {
    TOKENS
        .iter()
        .find(|token| token.custom_property == custom_property)
}

/// The table row for one dotted token path, if the platform defines it.
///
/// The path is the spelling `tokens.json` and [`ContrastError`] use — a `background` is declared as
/// `{color.paper}`, never as a custom property — so a caller resolving a declared background needs
/// this direction of the index rather than [`identity_of`].
#[must_use]
pub fn identity_at(path: &str) -> Option<&'static TokenIdentity> {
    TOKENS.iter().find(|token| token.path == path)
}

/// One colour parsed from the CSS text a token source or a host declared it with.
///
/// Public because [`check_usage`] takes [`Color`]s and its callers read them out of text: the
/// generator out of `tokens.json`, the canvas harness's editor out of the same file. A second
/// hexadecimal parser beside this one is a second answer to *"what colour is `#4c7`"*.
///
/// # Errors
///
/// [`TokenError::MalformedValue`], attributed to `custom_property`.
pub fn parse_color(custom_property: &str, text: &str) -> Result<Color, TokenError> {
    parse_token::<Color>(custom_property, text)
}

// -------------------------------------------------------------------------------------------
// Shared rendering and parsing helpers
// -------------------------------------------------------------------------------------------

/// A number as CSS writes it, and as every artefact must agree it is written.
///
/// Rust's shortest round-tripping `f32` form is already CSS-legal (`1.25`, `0.875`, `18`); the one
/// case it gets wrong for a stylesheet is `-0`, which is a value no design token has and a
/// spelling no artefact should carry.
fn css_number(value: f32) -> String {
    let text = format!("{value}");
    if text == "-0" {
        "0".to_owned()
    } else {
        text
    }
}

/// Splits a CSS comma-separated list, ignoring commas inside quotes. A family name may contain one.
fn split_css_list(text: &str) -> impl Iterator<Item = &str> {
    let mut parts = Vec::new();
    let mut quote: Option<char> = None;
    let mut start = 0;
    for (at, character) in text.char_indices() {
        match (quote, character) {
            (Some(open), _) if character == open => quote = None,
            (None, '"' | '\'') => quote = Some(character),
            (None, ',') => {
                parts.push(text[start..at].trim());
                start = at + 1;
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts.into_iter().filter(|part| !part.is_empty())
}

/// Strips one matched pair of quotes, if the text is wrapped in them.
fn unquote(text: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = text
            .strip_prefix(quote)
            .and_then(|it| it.strip_suffix(quote))
        {
            return inner;
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_generated_defaults_carry_the_measured_allr_values() {
        let tokens = Tokens::DEFAULTS;
        // Re-seeded by MJXOFF-271 from hermes-universal's `allr` skin, which is the same brand as
        // DESIGN_TOKENS.md §1 measured from the marketing site and is the value the surrounding
        // application actually paints.
        assert_eq!(tokens.color.paper.to_string(), "#fbf8f2");
        assert_eq!(tokens.color.green.to_string(), "#2e9e63");
        assert_eq!(tokens.color.green_deep.to_string(), "#1e7a49");
        // ⚠ **The radii are the one group that is NO LONGER the measured value**, and that is a
        // design decision rather than drift. `allr` measured 16px here; the hand-audit pass reduced
        // the whole `radius` group — chip 8→4, control 10→6, card 16→10, panel 20→12, frame 22→14,
        // phone 36→24 — because the source's softness read as too round at editor density. The
        // group's own `$description` in `tokens.json` still claims the radii are large; it is the
        // sentence to fix when somebody disagrees with this one.
        assert_eq!(tokens.radius.card.to_string(), "10px");
        assert_eq!(tokens.spacing.to_string(), "0.25rem");
        assert_eq!(tokens.duration.transition.to_string(), "150ms");
        assert_eq!(tokens.ease.ink.to_string(), "cubic-bezier(0.45, 0, 0.2, 1)");
        assert_eq!(tokens.shadow.lift.to_string(), "0px 18px 48px #223b331c");
        assert_eq!(tokens.leading.body, 1.7);
        assert_eq!(tokens.font_weight.semibold, 600);
    }

    /// §2.3's whole point: the page is white paper in both schemes, and it is the *backdrop* that
    /// changes. A dark theme that darkened the page would change what the author sees relative to
    /// what they will print.
    #[test]
    fn the_page_stays_white_in_both_schemes_and_the_backdrop_does_not() {
        let document = &Tokens::DEFAULTS.document;
        let white = Color {
            red: 0xff,
            green: 0xff,
            blue: 0xff,
            alpha: 0xff,
        };
        assert_eq!(document.scheme(ColorScheme::Light).page, white);
        assert_eq!(document.scheme(ColorScheme::Dark).page, white);
        assert_ne!(
            document.scheme(ColorScheme::Light).backdrop,
            document.scheme(ColorScheme::Dark).backdrop
        );
    }

    #[test]
    fn a_font_stack_yields_its_faces_unquoted() {
        let tokens = Tokens::DEFAULTS;
        let faces: Vec<&str> = tokens.font.sans.faces().collect();
        assert_eq!(faces, ["Nunito Sans", "system-ui", "sans-serif"]);
    }

    #[test]
    fn explicit_configuration_beats_the_host_which_beats_the_defaults() {
        let resolved = resolve(
            [("--color-paper", "#111111")],
            [("--color-paper", "#222222"), ("--color-ink", "#333333")],
        )
        .expect("both parse");

        assert_eq!(resolved.tokens().color.paper.to_string(), "#111111");
        assert_eq!(resolved.tokens().color.ink.to_string(), "#333333");
        assert_eq!(
            resolved.tokens().color.line,
            Tokens::DEFAULTS.color.line,
            "an untouched token keeps the generated default"
        );
        assert_eq!(
            resolved.source_of("--color-paper"),
            Some(TokenSource::ExplicitConfiguration)
        );
        assert_eq!(
            resolved.source_of("--color-ink"),
            Some(TokenSource::HostCustomProperty)
        );
        assert_eq!(
            resolved.source_of("--color-line"),
            Some(TokenSource::GeneratedDefault)
        );
        assert_eq!(resolved.source_of("--not-a-token"), None);
    }

    #[test]
    fn a_host_property_that_is_not_ours_is_ignored_and_a_typo_in_configuration_is_not() {
        let resolved = resolve([], [("--tw-ring-offset-width", "0px")]).expect("ignored");
        assert_eq!(resolved.overrides().count(), 0);

        let error = resolve([("--colour-paper", "#ffffff")], []).expect_err("must be refused");
        assert_eq!(
            error,
            TokenError::UnknownCustomProperty {
                custom_property: "--colour-paper".to_owned()
            }
        );
    }

    #[test]
    fn a_malformed_value_names_the_token_it_belongs_to() {
        let error = resolve([], [("--radius-card", "sixteen")]).expect_err("must be refused");
        let message = error.to_string();
        assert!(message.contains("--radius-card"), "{message}");
        assert!(message.contains("a length"), "{message}");
    }

    /// A host's stylesheet is not written by this project, so the parsers accept the spellings CSS
    /// allows and the generator does not use.
    #[test]
    fn the_parsers_accept_the_spellings_a_host_would_write() {
        assert_eq!(
            Color::from_css("#fff"),
            Ok(Color {
                red: 0xff,
                green: 0xff,
                blue: 0xff,
                alpha: 0xff
            })
        );
        assert_eq!(
            Dimension::from_css("0"),
            Ok(Dimension {
                magnitude: 0.0,
                unit: LengthUnit::Pixels
            })
        );
        assert_eq!(
            Duration::from_css("0.15s"),
            Ok(Duration {
                milliseconds: 150.0
            })
        );
        assert_eq!(
            Shadow::from_css("0 2px 10px #223b3314").map(|shadow| shadow.to_string()),
            Ok("0px 2px 10px #223b3314".to_owned())
        );
    }

    /// Every token round-trips: the CSS text the crate renders parses back to the same value. If
    /// it did not, `tokens.css` and the Rust table could not be the same data.
    #[test]
    fn every_token_round_trips_through_its_css_text() {
        let mut tokens = Tokens::DEFAULTS.clone();
        for identity in TOKENS {
            let before = tokens
                .custom_property(identity.custom_property)
                .unwrap_or_else(|| panic!("`{}` has a value", identity.custom_property));
            tokens
                .set_custom_property(identity.custom_property, &before.to_string())
                .unwrap_or_else(|error| panic!("`{}`: {error}", identity.custom_property));
            let after = tokens
                .custom_property(identity.custom_property)
                .unwrap_or_else(|| panic!("`{}` has a value", identity.custom_property));
            assert_eq!(before, after, "`{}` did not round-trip", identity.path);
        }
    }

    /// The measurement the whole derived tier rests on: mixing toward `transparent` is an **alpha**
    /// operation, not a blend toward black.
    ///
    /// `color-mix(in srgb, C 18%, transparent)` is `rgba(C.rgb, 0.18)` in Chromium, and an
    /// implementation that mixed un-premultiplied would darken `C` toward black instead — which
    /// would be wrong for every translucent token in the source and wrong in a way that still looks
    /// like a colour.
    #[test]
    fn mixing_toward_transparent_changes_the_alpha_and_not_the_colour() {
        let green = Color {
            red: 0x2e,
            green: 0x9e,
            blue: 0x63,
            alpha: 0xff,
        };
        let transparent = Color {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0,
        };
        let mixed = color_mix(green, Some(Percentage { value: 18.0 }), transparent, None)
            .expect("18% and 82% is a legal mix");
        assert_eq!(mixed.red, green.red);
        assert_eq!(mixed.green, green.green);
        assert_eq!(mixed.blue, green.blue);
        assert_eq!(mixed.alpha, 0x2e, "0.18 × 255 rounds to 46");
    }

    /// The second measurement: percentages that do not sum to 100% renormalise, and the shortfall
    /// becomes transparency rather than being quietly dropped.
    #[test]
    fn percentages_that_do_not_sum_to_a_hundred_renormalise() {
        let black = Color {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0xff,
        };
        let white = Color {
            red: 0xff,
            green: 0xff,
            blue: 0xff,
            alpha: 0xff,
        };
        let mixed = color_mix(
            black,
            Some(Percentage { value: 20.0 }),
            white,
            Some(Percentage { value: 20.0 }),
        )
        .expect("40% in total is legal");
        assert_eq!(
            (mixed.red, mixed.green, mixed.blue),
            (128, 128, 128),
            "the two weights are scaled to a half each"
        );
        assert_eq!(
            mixed.alpha, 102,
            "and the 60% shortfall becomes transparency: 0.4 × 255"
        );
        assert_eq!(
            color_mix(
                black,
                Some(Percentage { value: 0.0 }),
                white,
                Some(Percentage { value: 0.0 })
            ),
            None,
            "two zero percentages is invalid in CSS rather than a colour"
        );
    }

    /// Every derived token in the committed table really is what its own derivation produces.
    ///
    /// This is the arithmetic half of the agreement gate: `ui/tokens/chromium-agreement.mjs` proves
    /// the derivations mean the same thing to a browser, and this proves the committed values were
    /// not written by hand beside them.
    #[test]
    fn rederiving_the_defaults_reproduces_the_committed_values() {
        assert!(
            DERIVATIONS.len() >= 10,
            "the derived tier has only {} tokens, which cannot be this source",
            DERIVATIONS.len()
        );
        let mut tokens = Tokens::DEFAULTS.clone();
        tokens.rederive();
        assert_eq!(
            tokens,
            Tokens::DEFAULTS,
            "re-deriving the shipped seeds must land exactly on the shipped values"
        );
    }

    /// And it can move: one overridden seed re-themes everything mixed from it, which is the whole
    /// reason the source has two tiers.
    #[test]
    fn overriding_one_seed_re_derives_everything_mixed_from_it() {
        let resolved = resolve([], [("--theme-light-midground", "#0000ff")]).expect("a colour");
        let moved: Vec<&str> = resolved
            .overrides()
            .filter(|(_, source)| *source == TokenSource::Derivation)
            .map(|(name, _)| name)
            .collect();
        assert!(
            moved.contains(&"--theme-light-border"),
            "the ring colour feeds the border: {moved:?}"
        );
        assert!(
            moved.contains(&"--document-light-page-border"),
            "and the page border follows the chrome's, which is the seam that matters: {moved:?}"
        );
        assert_ne!(
            resolved.tokens().theme.light.border,
            Tokens::DEFAULTS.theme.light.border
        );
        assert_eq!(
            resolved.tokens().theme.light.border,
            resolved.tokens().document.light.page_border,
            "chrome and canvas must not disagree about the one colour they share"
        );
        // A token nothing mixes from the overridden seed keeps its default.
        assert_eq!(
            resolved.tokens().theme.dark.border,
            Tokens::DEFAULTS.theme.dark.border
        );
    }

    /// A derived colour a host set **directly** is an answer, not an input — re-deriving over it
    /// would discard the override and leave the theme half-applied.
    #[test]
    fn a_directly_overridden_derived_colour_survives_re_derivation() {
        let resolved = resolve([("--theme-light-background", "#123456")], []).expect("a colour");
        assert_eq!(
            resolved.tokens().theme.light.background.to_string(),
            "#123456"
        );
        assert_eq!(
            resolved.source_of("--theme-light-background"),
            Some(TokenSource::ExplicitConfiguration)
        );
    }

    #[test]
    fn the_token_table_is_complete_and_has_no_duplicates() {
        assert!(TOKENS.len() > 50, "the table has {} rows", TOKENS.len());
        let mut names: Vec<&str> = TOKENS.iter().map(|token| token.custom_property).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "two tokens share a custom property");
    }
}
