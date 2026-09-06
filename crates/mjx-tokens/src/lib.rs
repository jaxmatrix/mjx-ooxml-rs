//! `mjx-tokens` — the client platform's design tokens, typed for the Rust half of it.
//!
//! # Why this crate exists
//!
//! The chrome is HTML and the document canvas is Rust, and **a canvas cannot inherit a CSS custom
//! property**. Selection handles, alignment guides, rulers and marching ants are drawn by the
//! renderer, not by the browser, so they can only match the application around them if the same
//! token values reach both. One source — `docs/client-platform/data/tokens.json` — is therefore
//! generated into three artefacts by `cargo run -p xtask -- tokens`:
//!
//! | Artefact | Consumer |
//! |---|---|
//! | `ui/tokens/tokens.css` | the Web-Component chrome, as custom properties |
//! | `ui/tokens/tokens.ts` | the shell's own logic, as typed constants |
//! | `crates/mjx-tokens/src/generated.rs` | this crate, as [`Tokens`] and [`Tokens::DEFAULTS`] |
//!
//! The generated file is **committed**, never produced by a `build.rs` — the same doctrine
//! `mjx-ooxml-types` follows, and for the same reasons: a generated file nobody can read in review
//! or in a stack trace is a file nobody checks.
//!
//! # The resolution order
//!
//! [`resolve`] layers three sources, in this order:
//!
//! 1. **explicit configuration** — what the embedding application passed in;
//! 2. **host-supplied overrides** — the CSS custom properties read off the host element;
//! 3. **generated defaults** — [`Tokens::DEFAULTS`].
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
//! # Contrast is enforced in the generator, not here
//!
//! `DESIGN_TOKENS.md` §2.2 measures `--color-green` at 3.39 : 1 on white — legal for a fill, not
//! for body text — and `--color-green-deep` at 5.34 : 1. Every colour token in the source declares
//! which of those it is, and `xtask`'s generator **fails** if one tagged for text does not reach
//! 4.5 : 1 against its declared background. The measured ratio is recorded in each token's docs
//! here, so the decision is auditable at the point of use.

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
/// form in which a token can be compared across the three artefacts.
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
}

impl fmt::Display for TokenValue {
    /// The value as CSS text — byte for byte what `tokens.css` declares for it, which is what
    /// makes the three artefacts comparable at all.
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
        }
    }
}

/// One token's three names: its path in `tokens.json`, its CSS custom property, and its path in
/// `tokens.ts`. [`TOKENS`] is the whole table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenIdentity {
    /// The dotted path in the source and in the generated Rust, e.g. `color.ink-soft`.
    pub path: &'static str,
    /// The CSS custom property, e.g. `--color-ink-soft`.
    pub custom_property: &'static str,
    /// The path in `ui/tokens/tokens.ts`, e.g. `color.inkSoft`.
    pub typescript_path: &'static str,
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

    Ok(resolution)
}

/// The table row for one custom property, if the platform defines it.
#[must_use]
pub fn identity_of(custom_property: &str) -> Option<&'static TokenIdentity> {
    TOKENS
        .iter()
        .find(|token| token.custom_property == custom_property)
}

// -------------------------------------------------------------------------------------------
// Shared rendering and parsing helpers
// -------------------------------------------------------------------------------------------

/// A number as CSS writes it, and as all three artefacts must agree it is written.
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
        assert_eq!(tokens.color.paper.to_string(), "#fdfcf9");
        assert_eq!(tokens.color.green.to_string(), "#2e9e63");
        assert_eq!(tokens.color.green_deep.to_string(), "#1e7a49");
        assert_eq!(tokens.radius.card.to_string(), "16px");
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
