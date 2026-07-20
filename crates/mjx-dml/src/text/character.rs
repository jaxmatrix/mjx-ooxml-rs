//! DrawingML character (run) properties: `CT_TextCharacterProperties` — how a run of text looks.
//!
//! [`CharacterProperties`] is a **fidelity wrapper** over the element (its name, attributes, children
//! and self-closing flag preserved verbatim), so one type serves every name this complex type appears
//! under — `a:rPr` on a run, `a:defRPr` inside paragraph properties, `a:endParaRPr` on a paragraph —
//! and re-emits each under its own tag.
//!
//! [`CharacterPropertiesSpec`] is the interner-free value the format-level API speaks. It is a
//! **builder**: a run has a dozen independent properties, and naming only the ones you mean is the
//! whole point.
//!
//! ```
//! use mjx_dml::{CharacterPropertiesSpec, ColorSpec, SchemeColor};
//!
//! let title = CharacterPropertiesSpec::new()
//!     .with_size_points(28.0)
//!     .with_bold(true)
//!     .with_color(ColorSpec::Scheme(SchemeColor::Accent1));
//!
//! assert_eq!(title.size_points(), Some(28.0));
//! assert_eq!(title.is_bold(), Some(true));
//! ```
//!
//! # Writing: merge, don't rebuild
//!
//! Unlike a fill or an outline — self-contained elements that can be rebuilt wholesale — `a:rPr`
//! mixes properties we model with state we do not: `lang`, `dirty`, `err`, `smtClean`, hyperlinks.
//! Office writes those on nearly every run. So [`CharacterProperties::apply`] **merges** a spec onto
//! an existing element, touching only what the spec names;
//! [`CharacterPropertiesSpec::to_properties`] builds a fresh element for a run that has none.

use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml};
use mjx_ooxml_types::support::on_off;

use crate::build::{
    attr_str, dml_child, fidelity_element_impls, first_color_child, first_fill_child,
    parse_percentage, replace_or_insert_child, set_attr,
};
use crate::color::{Color, ColorSpec};
use crate::effect::{EffectList, EffectListSpec};
use crate::fill::{Fill, FillSpec};
use crate::geometry::{FontSize, Fraction, TextPoint};
use crate::line::{LineProperties, LineSpec};
use crate::text::font::TextFont;

pub use mjx_ooxml_types::drawingml::{TextCapitalization, TextStrike, TextUnderline};

/// Which script a font applies to — the four typeface slots of `CT_TextCharacterProperties`.
///
/// A run may name a different font per script; `Latin` is the one a caller normally means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontSlot {
    /// `a:latin` — the latin-script font.
    Latin,
    /// `a:ea` — the East Asian font.
    EastAsian,
    /// `a:cs` — the complex-script font.
    ComplexScript,
    /// `a:sym` — the symbol font.
    Symbol,
}

impl FontSlot {
    /// The element's local name.
    pub(crate) fn local(self) -> &'static str {
        match self {
            Self::Latin => "latin",
            Self::EastAsian => "ea",
            Self::ComplexScript => "cs",
            Self::Symbol => "sym",
        }
    }

    /// Every slot, in `CT_TextCharacterProperties` schema order.
    #[must_use]
    pub fn all_slots() -> [Self; 4] {
        [
            Self::Latin,
            Self::EastAsian,
            Self::ComplexScript,
            Self::Symbol,
        ]
    }
}

// ---------------------------------------------------------------------------------------------
// CharacterProperties — the fidelity wrapper
// ---------------------------------------------------------------------------------------------

/// `CT_TextCharacterProperties` — the appearance of a run of text: size, weight, slant, underline,
/// strike, capitalization, spacing, and the fill/outline/effects/font it draws with.
///
/// A fidelity wrapper: the modeled properties are exposed by typed accessors, while everything else
/// — hyperlinks, the underline line/fill groups, `rtl`, `extLst`, and the housekeeping attributes
/// (`dirty`, `err`, `smtClean`, `altLang`, …) — is preserved verbatim so a run round-trips
/// byte-for-byte.
///
/// The element name is preserved too, so the same type reads and writes `a:rPr`, `a:defRPr` and
/// `a:endParaRPr` alike.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl CharacterProperties {
    /// The font size (`@sz`), or `None` if unset (an inherited size).
    #[must_use]
    pub fn size(&self, interner: &Interner) -> Option<FontSize> {
        attr_str(&self.attributes, interner, "sz")
            .and_then(|s| s.trim().parse::<i32>().ok())
            .map(FontSize::from_wire)
    }

    /// Whether the run is bold (`@b`), or `None` if unset (inherited).
    #[must_use]
    pub fn is_bold(&self, interner: &Interner) -> Option<bool> {
        attr_str(&self.attributes, interner, "b").and_then(on_off::from_wire)
    }

    /// Whether the run is italic (`@i`), or `None` if unset (inherited).
    #[must_use]
    pub fn is_italic(&self, interner: &Interner) -> Option<bool> {
        attr_str(&self.attributes, interner, "i").and_then(on_off::from_wire)
    }

    /// The underline style (`@u`), or `None` if unset. Note [`TextUnderline::None`] is an explicit
    /// "not underlined", which is not the same as unset: it overrides an inherited underline.
    #[must_use]
    pub fn underline(&self, interner: &Interner) -> Option<TextUnderline> {
        attr_str(&self.attributes, interner, "u").and_then(TextUnderline::from_wire)
    }

    /// The strikethrough style (`@strike`), or `None` if unset.
    #[must_use]
    pub fn strike(&self, interner: &Interner) -> Option<TextStrike> {
        attr_str(&self.attributes, interner, "strike").and_then(TextStrike::from_wire)
    }

    /// The capitalization applied to the run (`@cap`), or `None` if unset.
    #[must_use]
    pub fn capitalization(&self, interner: &Interner) -> Option<TextCapitalization> {
        attr_str(&self.attributes, interner, "cap").and_then(TextCapitalization::from_wire)
    }

    /// The character spacing (`@spc`) — negative tightens — or `None` if unset.
    #[must_use]
    pub fn spacing(&self, interner: &Interner) -> Option<TextPoint> {
        attr_str(&self.attributes, interner, "spc")
            .and_then(|s| s.trim().parse::<i32>().ok())
            .map(TextPoint::from_wire)
    }

    /// The size from which kerning applies (`@kern`), or `None` if unset.
    #[must_use]
    pub fn kerning(&self, interner: &Interner) -> Option<TextPoint> {
        attr_str(&self.attributes, interner, "kern")
            .and_then(|s| s.trim().parse::<i32>().ok())
            .map(TextPoint::from_wire)
    }

    /// The baseline offset (`@baseline`) as a fraction of the font size — positive raises
    /// (superscript), negative lowers (subscript) — or `None` if unset.
    #[must_use]
    pub fn baseline(&self, interner: &Interner) -> Option<Fraction> {
        attr_str(&self.attributes, interner, "baseline").and_then(parse_percentage)
    }

    /// The language of the run's text (`@lang`, e.g. `en-US`), or `None` if unset.
    #[must_use]
    pub fn language<'a>(&'a self, interner: &Interner) -> Option<&'a str> {
        attr_str(&self.attributes, interner, "lang")
    }

    /// The text fill (`EG_FillProperties`) — what the glyphs are painted with — or `None` if the run
    /// declares none.
    #[must_use]
    pub fn fill(&self, interner: &Interner) -> Option<Fill> {
        first_fill_child(&self.children, interner).and_then(|el| Fill::from_xml(el, interner).ok())
    }

    /// The glyph outline (`a:ln`), or `None` if the run declares none.
    #[must_use]
    pub fn outline(&self, interner: &Interner) -> Option<LineProperties> {
        dml_child(&self.children, interner, "ln")
            .and_then(|el| LineProperties::from_xml(el, interner).ok())
    }

    /// The text effects (`a:effectLst`), or `None` if the run declares none. `a:effectDag` (the
    /// alternative effect-container form) is preserved but not modeled.
    #[must_use]
    pub fn effects(&self, interner: &Interner) -> Option<EffectList> {
        dml_child(&self.children, interner, "effectLst")
            .and_then(|el| EffectList::from_xml(el, interner).ok())
    }

    /// The highlight (`a:highlight`) — the color drawn behind the glyphs — or `None` if unset.
    #[must_use]
    pub fn highlight(&self, interner: &Interner) -> Option<Color> {
        dml_child(&self.children, interner, "highlight")
            .and_then(|el| first_color_child(el, interner))
    }

    /// The typeface named for `slot`, or `None` if the run names none for it.
    #[must_use]
    pub fn font(&self, interner: &Interner, slot: FontSlot) -> Option<TextFont> {
        dml_child(&self.children, interner, slot.local()).map(|el| TextFont::read(el, interner))
    }

    /// The interner-free description of these properties. Colors are **not** resolved — a scheme color
    /// stays a scheme color; see `resolve_character_properties` for the resolved form.
    #[must_use]
    pub fn spec(&self, interner: &Interner) -> CharacterPropertiesSpec {
        CharacterPropertiesSpec {
            size: self.size(interner),
            bold: self.is_bold(interner),
            italic: self.is_italic(interner),
            underline: self.underline(interner),
            strike: self.strike(interner),
            capitalization: self.capitalization(interner),
            spacing: self.spacing(interner),
            kerning: self.kerning(interner),
            baseline: self.baseline(interner),
            language: self.language(interner).map(str::to_owned),
            fill: self.fill(interner).map(|fill| fill.spec(interner)),
            outline: self.outline(interner).map(|line| line.spec(interner)),
            effects: self.effects(interner).map(|fx| fx.spec(interner)),
            highlight: self.highlight(interner).map(|color| color.spec(interner)),
            fonts: FontSlot::all_slots()
                .into_iter()
                .filter_map(|slot| self.font(interner, slot).map(|font| (slot, font)))
                .collect(),
        }
    }

    /// Merges `spec` onto these properties **in place**, writing only what the spec names and leaving
    /// everything else — `lang`, `dirty`, hyperlinks, unmodeled children — exactly where it was.
    ///
    /// This is what makes bolding a run written by PowerPoint non-destructive. A property the spec
    /// leaves unset is *not* cleared: an unset field means "don't touch", not "remove". To remove a
    /// property, build a fresh element with
    /// [`to_properties`](CharacterPropertiesSpec::to_properties) instead.
    pub fn apply(&mut self, spec: &CharacterPropertiesSpec, interner: &mut Interner) {
        if let Some(size) = spec.size {
            set_attr(
                &mut self.attributes,
                interner,
                "sz",
                &size.to_wire().to_string(),
            );
        }
        if let Some(bold) = spec.bold {
            set_attr(&mut self.attributes, interner, "b", on_off::to_wire(bold));
        }
        if let Some(italic) = spec.italic {
            set_attr(&mut self.attributes, interner, "i", on_off::to_wire(italic));
        }
        if let Some(underline) = spec.underline {
            set_attr(&mut self.attributes, interner, "u", underline.to_wire());
        }
        if let Some(strike) = spec.strike {
            set_attr(&mut self.attributes, interner, "strike", strike.to_wire());
        }
        if let Some(caps) = spec.capitalization {
            set_attr(&mut self.attributes, interner, "cap", caps.to_wire());
        }
        if let Some(spacing) = spec.spacing {
            set_attr(
                &mut self.attributes,
                interner,
                "spc",
                &spacing.to_wire().to_string(),
            );
        }
        if let Some(kerning) = spec.kerning {
            set_attr(
                &mut self.attributes,
                interner,
                "kern",
                &kerning.to_wire().to_string(),
            );
        }
        if let Some(baseline) = spec.baseline {
            let native = (baseline.ratio() * 100_000.0).round() as i64;
            set_attr(
                &mut self.attributes,
                interner,
                "baseline",
                &native.to_string(),
            );
        }
        if let Some(language) = &spec.language {
            set_attr(&mut self.attributes, interner, "lang", language);
        }

        // Children are replaced as whole elements — each is self-contained, so there is no partial
        // state to preserve inside one — but only those the spec names, and in place so the schema
        // sequence is not disturbed.
        if let Some(fill) = &spec.fill {
            let element = fill.to_fill(interner).to_xml(interner);
            self.replace_child(interner, element, Fill::is_fill_local);
        }
        if let Some(outline) = &spec.outline {
            let element = outline.to_line(interner).to_xml(interner);
            self.replace_child(interner, element, |local| local == "ln");
        }
        if let Some(effects) = &spec.effects {
            let element = effects.to_effect_list(interner).to_xml(interner);
            self.replace_child(interner, element, |local| local == "effectLst");
        }
        if let Some(highlight) = &spec.highlight {
            if let Some(element) = build_highlight(interner, highlight) {
                self.replace_child(interner, element, |local| local == "highlight");
            }
        }
        for (slot, font) in &spec.fonts {
            let element = font.build(interner, slot.local());
            let local = slot.local();
            self.replace_child(interner, element, |candidate| candidate == local);
        }
        self.empty = self.empty && self.children.is_empty();
    }

    /// Replaces the first child element whose local name satisfies `matches` with `element`, keeping
    /// its position; inserts it in `CT_TextCharacterProperties` order when there is none.
    fn replace_child(
        &mut self,
        interner: &Interner,
        element: RawElement,
        matches: impl Fn(&str) -> bool,
    ) {
        replace_or_insert_child(&mut self.children, interner, element, matches, known_rank);
        self.empty = false;
    }
}

/// Children this model may write, in schema order. Anything else sorts last.
fn known_rank(local: &str) -> Option<usize> {
    let rank = match local {
        "ln" => 0,
        _ if Fill::is_fill_local(local) => 1,
        "effectLst" | "effectDag" => 2,
        "highlight" => 3,
        "uLnTx" | "uLn" => 4,
        "uFillTx" | "uFill" => 5,
        "latin" => 6,
        "ea" => 7,
        "cs" => 8,
        "sym" => 9,
        "hlinkClick" => 10,
        "hlinkMouseOver" => 11,
        "rtl" => 12,
        "extLst" => 13,
        _ => return None,
    };
    Some(rank)
}

fidelity_element_impls!(CharacterProperties);

/// Builds an `a:highlight` wrapping the given color, or `None` if the color cannot be rebuilt.
fn build_highlight(interner: &mut Interner, color: &ColorSpec) -> Option<RawElement> {
    let color = Color::from_spec(interner, color)?;
    let child = color.to_xml(interner);
    Some(crate::build::dml_element(
        interner,
        "highlight",
        Vec::new(),
        vec![RawNode::Element(child)],
    ))
}

// ---------------------------------------------------------------------------------------------
// CharacterPropertiesSpec — the interner-free builder
// ---------------------------------------------------------------------------------------------

/// An interner-free description of how a run of text looks — the value the format-level API reads and
/// writes.
///
/// Built by naming only the properties you mean; every one left unnamed **inherits** (from the
/// paragraph, the placeholder, the layout, the master, the theme). That is why each field is optional
/// and why unset never means "off": `TextUnderline::None` is how you say *not underlined* over an
/// inherited underline.
///
/// Sizes and spacing are in **points**, the unit type is measured in everywhere except the file
/// itself.
///
/// ```
/// use mjx_dml::{CharacterPropertiesSpec, TextUnderline};
///
/// let emphasis = CharacterPropertiesSpec::new()
///     .with_italic(true)
///     .with_underline(TextUnderline::Single)
///     .with_spacing_points(0.5);
/// assert_eq!(emphasis.is_italic(), Some(true));
/// assert_eq!(emphasis.spacing_points(), Some(0.5));
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CharacterPropertiesSpec {
    size: Option<FontSize>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<TextUnderline>,
    strike: Option<TextStrike>,
    capitalization: Option<TextCapitalization>,
    spacing: Option<TextPoint>,
    kerning: Option<TextPoint>,
    baseline: Option<Fraction>,
    language: Option<String>,
    fill: Option<FillSpec>,
    outline: Option<LineSpec>,
    effects: Option<EffectListSpec>,
    highlight: Option<ColorSpec>,
    fonts: Vec<(FontSlot, TextFont)>,
}

impl CharacterPropertiesSpec {
    /// Properties that name nothing — everything inherits. The same as [`Default`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the font size, in points.
    #[must_use]
    pub fn with_size_points(mut self, points: f64) -> Self {
        self.size = Some(FontSize::from_points(points));
        self
    }

    /// Sets the font size.
    #[must_use]
    pub fn with_size(mut self, size: FontSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Sets weight — `bold(false)` explicitly *un*-bolds text that would otherwise inherit bold.
    #[must_use]
    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = Some(bold);
        self
    }

    /// Sets slant — `italic(false)` explicitly un-italicizes inherited italic.
    #[must_use]
    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = Some(italic);
        self
    }

    /// Sets the underline style ([`TextUnderline::None`] to override an inherited underline).
    #[must_use]
    pub fn with_underline(mut self, underline: TextUnderline) -> Self {
        self.underline = Some(underline);
        self
    }

    /// Sets the strikethrough style.
    #[must_use]
    pub fn with_strike(mut self, strike: TextStrike) -> Self {
        self.strike = Some(strike);
        self
    }

    /// Sets the capitalization applied to the text.
    #[must_use]
    pub fn with_capitalization(mut self, capitalization: TextCapitalization) -> Self {
        self.capitalization = Some(capitalization);
        self
    }

    /// Sets character spacing, in points — negative tightens.
    #[must_use]
    pub fn with_spacing_points(mut self, points: f64) -> Self {
        self.spacing = Some(TextPoint::from_points(points));
        self
    }

    /// Sets the font size from which kerning applies, in points.
    #[must_use]
    pub fn with_kerning_points(mut self, points: f64) -> Self {
        self.kerning = Some(TextPoint::from_points(points));
        self
    }

    /// Sets the baseline offset as a fraction of the font size — positive raises (superscript),
    /// negative lowers (subscript).
    #[must_use]
    pub fn with_baseline(mut self, baseline: Fraction) -> Self {
        self.baseline = Some(baseline);
        self
    }

    /// Sets the language of the text (`en-US`, …).
    #[must_use]
    pub fn with_language(mut self, language: &str) -> Self {
        self.language = Some(language.to_owned());
        self
    }

    /// Paints the glyphs in a solid `color` — the common case of [`with_fill`](Self::with_fill).
    #[must_use]
    pub fn with_color(self, color: ColorSpec) -> Self {
        self.with_fill(FillSpec::Solid(color))
    }

    /// Sets the text fill (a gradient or pattern, where [`with_color`](Self::with_color) sets a flat color).
    #[must_use]
    pub fn with_fill(mut self, fill: FillSpec) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Sets the glyph outline.
    #[must_use]
    pub fn with_outline(mut self, outline: LineSpec) -> Self {
        self.outline = Some(outline);
        self
    }

    /// Sets the text effects.
    #[must_use]
    pub fn with_effects(mut self, effects: EffectListSpec) -> Self {
        self.effects = Some(effects);
        self
    }

    /// Sets the highlight color drawn behind the glyphs.
    #[must_use]
    pub fn with_highlight(mut self, highlight: ColorSpec) -> Self {
        self.highlight = Some(highlight);
        self
    }

    /// Names the latin-script font — `with_font("Calibri")`, or `with_font("+mj-lt")` for the theme's major
    /// font. For another script, use [`with_font_for`](Self::with_font_for).
    #[must_use]
    pub fn with_font(self, typeface: &str) -> Self {
        self.with_font_for(FontSlot::Latin, TextFont::named(typeface))
    }

    /// Names the font for one script slot.
    #[must_use]
    pub fn with_font_for(mut self, slot: FontSlot, font: TextFont) -> Self {
        self.fonts.retain(|(existing, _)| *existing != slot);
        self.fonts.push((slot, font));
        self
    }

    /// The font size, if set.
    #[must_use]
    pub fn size(&self) -> Option<FontSize> {
        self.size
    }

    /// The font size in points, if set.
    #[must_use]
    pub fn size_points(&self) -> Option<f64> {
        self.size.map(FontSize::points)
    }

    /// Whether the text is bold, if set.
    #[must_use]
    pub fn is_bold(&self) -> Option<bool> {
        self.bold
    }

    /// Whether the text is italic, if set.
    #[must_use]
    pub fn is_italic(&self) -> Option<bool> {
        self.italic
    }

    /// The underline style, if set.
    #[must_use]
    pub fn underline(&self) -> Option<TextUnderline> {
        self.underline
    }

    /// The strikethrough style, if set.
    #[must_use]
    pub fn strike(&self) -> Option<TextStrike> {
        self.strike
    }

    /// The capitalization, if set.
    #[must_use]
    pub fn capitalization(&self) -> Option<TextCapitalization> {
        self.capitalization
    }

    /// The character spacing in points, if set.
    #[must_use]
    pub fn spacing_points(&self) -> Option<f64> {
        self.spacing.map(TextPoint::points)
    }

    /// The kerning threshold in points, if set.
    #[must_use]
    pub fn kerning_points(&self) -> Option<f64> {
        self.kerning.map(TextPoint::points)
    }

    /// The baseline offset, if set.
    #[must_use]
    pub fn baseline(&self) -> Option<Fraction> {
        self.baseline
    }

    /// The language, if set.
    #[must_use]
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// The text fill, if set.
    #[must_use]
    pub fn fill(&self) -> Option<&FillSpec> {
        self.fill.as_ref()
    }

    /// The glyph outline, if set.
    #[must_use]
    pub fn outline(&self) -> Option<&LineSpec> {
        self.outline.as_ref()
    }

    /// The text effects, if set.
    #[must_use]
    pub fn effects(&self) -> Option<&EffectListSpec> {
        self.effects.as_ref()
    }

    /// The highlight color, if set.
    #[must_use]
    pub fn highlight(&self) -> Option<&ColorSpec> {
        self.highlight.as_ref()
    }

    /// The font named for one script slot, if set.
    #[must_use]
    pub fn font(&self, slot: FontSlot) -> Option<&TextFont> {
        self.fonts
            .iter()
            .find(|(existing, _)| *existing == slot)
            .map(|(_, font)| font)
    }

    /// Merges a **lower** inheritance tier under these properties: `self` wins wherever it names
    /// something, and `lower` supplies only what `self` leaves unset.
    ///
    /// This is one rung of the ladder a run's *effective* formatting is resolved along — the run's own
    /// `a:rPr`, then the paragraph's `a:defRPr`, then each list style up through the layout, the master
    /// and the presentation default. Fold from the top: `run.merge_under(&paragraph).merge_under(&shape)`.
    ///
    /// Every property merges as a whole value, so a tier that sets a fill supplies *that* fill rather
    /// than half of one, and an explicit "off" — `Fill::NoFill`, an empty effect list — is a present
    /// value that correctly blocks the tier below. Two fields are not a plain field-wise fallback:
    ///
    /// - **Fonts merge per script slot.** A tier naming only a latin font leaves a lower tier's East
    ///   Asian font standing, because `a:latin` and `a:ea` are separate elements, not one choice.
    /// - **Nothing else is special.** Sizes, weights, colors and the rest are single values.
    #[must_use]
    pub fn merge_under(mut self, lower: &Self) -> Self {
        self.size = self.size.or(lower.size);
        self.bold = self.bold.or(lower.bold);
        self.italic = self.italic.or(lower.italic);
        self.underline = self.underline.or(lower.underline);
        self.strike = self.strike.or(lower.strike);
        self.capitalization = self.capitalization.or(lower.capitalization);
        self.spacing = self.spacing.or(lower.spacing);
        self.kerning = self.kerning.or(lower.kerning);
        self.baseline = self.baseline.or(lower.baseline);
        self.language = self.language.or_else(|| lower.language.clone());
        self.fill = self.fill.or_else(|| lower.fill.clone());
        self.outline = self.outline.or_else(|| lower.outline.clone());
        self.effects = self.effects.or_else(|| lower.effects.clone());
        self.highlight = self.highlight.or_else(|| lower.highlight.clone());

        // Per slot, in schema order, so a merged spec's fonts read the same whichever tier set them.
        for slot in FontSlot::all_slots() {
            if self.font(slot).is_none() {
                if let Some(font) = lower.font(slot) {
                    self.fonts.push((slot, font.clone()));
                }
            }
        }
        self.fonts.sort_by_key(|(slot, _)| *slot as u8);
        self
    }

    /// Builds a **fresh** element for these properties under `local` (`rPr`, `defRPr` or
    /// `endParaRPr`), assembled in `CT_TextCharacterProperties` order: the attributes, then `a:ln` →
    /// fill → effects → `a:highlight` → the script fonts.
    ///
    /// Only what the spec names is written. To keep an existing element's unmodeled state, merge with
    /// [`CharacterProperties::apply`] instead.
    #[must_use]
    pub fn to_properties(&self, interner: &mut Interner, local: &str) -> CharacterProperties {
        let mut properties = CharacterProperties {
            name: crate::build::dml_name(interner, local),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        properties.apply(self, interner);
        properties.empty = properties.children.is_empty();
        properties
    }
}
