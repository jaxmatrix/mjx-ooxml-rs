//! Text: run properties, paragraph properties, bullets, tabs, fonts, and the theme's font scheme.
//!
//! [`CharacterPropertiesSpec`] and [`ParagraphPropertiesSpec`] are the two classes most of a
//! deck's formatting goes through, and both mirror their Rust builders method for method — every
//! `with_…` returns a new value, so a specification can be built once and applied many times:
//!
//! ```python
//! heading = (CharacterPropertiesSpec()
//!            .with_size_points(28)
//!            .with_bold(True)
//!            .with_color(ColorSpec.srgb("1F3864")))
//! for slide in range(deck.slide_count):
//!     deck.set_shape_run_properties(slide, 0, heading)
//! ```

use pyo3::prelude::*;
use pyo3::types::PyModule;

use mjx_ooxml as ooxml;

use crate::enums::{
    AutonumberScheme, ColorSchemeSlot, FontAlignment, FontSchemeSlot, FontSlot, TabAlignment,
    TextAlignment, TextCapitalization, TextStrike, TextUnderline,
};
use crate::measures::{Emu, FontSize, Fraction, IndentLevel, TextPoint};
use crate::paint::{ColorSpec, EffectListSpec, FillSpec, LineSpec};

value_class! {
    /// Everything a run of text can state about itself: size, weight, colour, underline, the fonts
    /// for each script.
    CharacterPropertiesSpec(ooxml::CharacterPropertiesSpec), derive(PartialEq);

    /// Everything a paragraph can state: alignment, margins, spacing, bullet, tab stops, and the
    /// run properties its own text inherits.
    ParagraphPropertiesSpec(ooxml::ParagraphPropertiesSpec), derive(PartialEq);

    /// A typeface reference: the name, and the classification attributes that let a consumer
    /// substitute when the font is missing.
    TextFont(ooxml::TextFont), derive(PartialEq);

    /// A theme font reference — which collection (major or minor) and which script slot.
    ThemeFontReference(ooxml::ThemeFontReference), derive(Copy, PartialEq);

    /// One tab stop: where it is, and how text aligns at it.
    TabStop(ooxml::TabStop), derive(Copy, PartialEq);

    /// What marks a paragraph: nothing, a character, an automatic number, or a picture.
    Bullet(ooxml::Bullet), derive(PartialEq);

    /// A literal bullet character.
    BulletCharacter(ooxml::BulletCharacter), derive(PartialEq);

    /// An automatically numbered bullet: which numbering scheme, and what it starts at.
    AutoNumberBullet(ooxml::AutoNumberBullet), derive(Copy, PartialEq);

    /// A picture bullet, named by the relationship id of its image.
    BulletPicture(ooxml::BulletPicture), derive(PartialEq);

    /// The bullet's colour: the text's, or one of its own.
    BulletColor(ooxml::BulletColor), derive(PartialEq);

    /// The bullet's size: the text's, a proportion of it, or an absolute size.
    BulletSize(ooxml::BulletSize), derive(Copy, PartialEq);

    /// The bullet's typeface: the text's, or one of its own.
    BulletTypeface(ooxml::BulletTypeface), derive(PartialEq);

    /// A spacing measure: a proportion of the line, or an absolute number of points.
    TextSpacing(ooxml::TextSpacing), derive(Copy, PartialEq);

    /// The underline's line style: the text's, or one of its own.
    UnderlineLine(ooxml::UnderlineLine), derive(PartialEq);

    /// The underline's fill: the text's, or one of its own.
    UnderlineFill(ooxml::UnderlineFill), derive(PartialEq);

    /// A theme's font for one script beyond the three main slots.
    SupplementalFont(ooxml::SupplementalFont), derive(PartialEq);

    /// One half of a theme's font scheme — the fonts for the Latin, East Asian and complex-script
    /// slots, plus the supplemental fonts.
    FontCollection(ooxml::FontCollection), derive(PartialEq);

    /// A theme's font scheme: its name, and its major and minor collections.
    FontScheme(ooxml::FontScheme), derive(PartialEq);

    /// What a theme states, interner-free: its colours, its fonts, and its style matrices.
    ThemeInfo(ooxml::ThemeInfo), derive(PartialEq);
}

// ---------------------------------------------------------------------------------------------
// Fonts
// ---------------------------------------------------------------------------------------------

#[pymethods]
impl TextFont {
    /// A typeface by name. `"+mj-lt"` and `"+mn-lt"` name the theme's major and minor Latin fonts.
    #[new]
    #[pyo3(signature = (typeface, panose = None, pitch_family = None, charset = None))]
    fn new(
        typeface: &str,
        panose: Option<String>,
        pitch_family: Option<i32>,
        charset: Option<i32>,
    ) -> Self {
        Self(ooxml::TextFont {
            typeface: typeface.to_owned(),
            panose,
            pitch_family,
            charset,
        })
    }

    /// The typeface name, exactly as written.
    #[getter]
    fn typeface(&self) -> &str {
        &self.0.typeface
    }

    /// The PANOSE classification, when the document states one.
    #[getter]
    fn panose(&self) -> Option<&str> {
        self.0.panose.as_deref()
    }

    /// The pitch and family byte, when stated.
    #[getter]
    fn pitch_family(&self) -> Option<i32> {
        self.0.pitch_family
    }

    /// The character set byte, when stated.
    #[getter]
    fn charset(&self) -> Option<i32> {
        self.0.charset
    }

    /// Whether this names a theme font (`+mj-lt`, `+mn-ea`, …) rather than a typeface.
    #[getter]
    fn is_theme_reference(&self) -> bool {
        self.0.is_theme_reference()
    }

    /// Which theme font this names, when it names one.
    #[getter]
    fn theme_reference(&self) -> Option<ThemeFontReference> {
        self.0.theme_reference().map(ThemeFontReference)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ThemeFontReference {
    /// The major or minor collection, and the script slot within it.
    #[new]
    fn new(collection: FontSchemeSlot, slot: FontSlot) -> Self {
        Self(ooxml::ThemeFontReference {
            collection: collection.into(),
            slot: slot.into(),
        })
    }

    /// Which collection — the major (heading) or minor (body) fonts.
    #[getter]
    fn collection(&self) -> PyResult<FontSchemeSlot> {
        FontSchemeSlot::from_model(self.0.collection)
    }

    /// Which script slot within the collection.
    #[getter]
    fn slot(&self) -> PyResult<FontSlot> {
        FontSlot::from_model(self.0.slot)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl SupplementalFont {
    /// The script this font covers, as the theme names it.
    #[getter]
    fn script(&self) -> &str {
        self.0.script()
    }

    /// The typeface for that script.
    #[getter]
    fn typeface(&self) -> &str {
        self.0.typeface()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl FontCollection {
    /// The font for one script slot, when the collection states one.
    fn font(&self, slot: FontSlot) -> Option<TextFont> {
        self.0.font(slot.into()).cloned().map(TextFont)
    }

    /// Every supplemental font this collection lists.
    #[getter]
    fn supplemental_fonts(&self) -> Vec<SupplementalFont> {
        self.0
            .supplemental_fonts()
            .iter()
            .cloned()
            .map(SupplementalFont)
            .collect()
    }

    /// The supplemental font for one script, when the collection states one.
    fn supplemental_font(&self, script: &str) -> Option<SupplementalFont> {
        self.0
            .supplemental_font(script)
            .cloned()
            .map(SupplementalFont)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl FontScheme {
    /// The scheme's name, as the theme states it.
    #[getter]
    fn name(&self) -> &str {
        self.0.name()
    }

    /// The major (heading) collection.
    #[getter]
    fn major(&self) -> FontCollection {
        FontCollection(self.0.major().clone())
    }

    /// The minor (body) collection.
    #[getter]
    fn minor(&self) -> FontCollection {
        FontCollection(self.0.minor().clone())
    }

    /// One of the two collections by name.
    fn collection(&self, slot: FontSchemeSlot) -> FontCollection {
        FontCollection(self.0.collection(slot.into()).clone())
    }

    /// The font a theme reference resolves to, when the scheme states one.
    fn font(&self, reference: ThemeFontReference) -> Option<TextFont> {
        self.0.font(reference.0).cloned().map(TextFont)
    }

    /// The typeface a font resolves to: itself, unless it is a theme reference, in which case the
    /// font this scheme names for that slot.
    fn resolve(&self, font: &TextFont) -> Option<TextFont> {
        self.0.resolve(&font.0).cloned().map(TextFont)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ThemeInfo {
    /// The colour a scheme slot resolves to in this theme, when it states one.
    fn color(&self, slot: ColorSchemeSlot) -> Option<ColorSpec> {
        self.0.color(slot.into()).cloned().map(ColorSpec)
    }

    /// Every slot the theme states a colour for, paired with that colour.
    #[getter]
    fn colors(&self) -> PyResult<Vec<(ColorSchemeSlot, ColorSpec)>> {
        self.0
            .colors()
            .map(|(slot, color)| Ok((ColorSchemeSlot::from_model(slot)?, ColorSpec(color.clone()))))
            .collect()
    }

    /// The theme's font scheme, when it states one.
    #[getter]
    fn font_scheme(&self) -> Option<FontScheme> {
        self.0.font_scheme().cloned().map(FontScheme)
    }

    /// The theme's fill style matrix, in order.
    #[getter]
    fn fill_styles(&self) -> Vec<FillSpec> {
        self.0.fill_styles().iter().cloned().map(FillSpec).collect()
    }

    /// One fill style by index — the number a shape's `a:fillRef@idx` names, counting from one.
    fn fill_style(&self, index: u32) -> Option<FillSpec> {
        self.0.fill_style(index).cloned().map(FillSpec)
    }

    /// The theme's line style matrix, in order.
    #[getter]
    fn line_styles(&self) -> Vec<LineSpec> {
        self.0.line_styles().iter().cloned().map(LineSpec).collect()
    }

    /// One line style by index.
    fn line_style(&self, index: u32) -> Option<LineSpec> {
        self.0.line_style(index).cloned().map(LineSpec)
    }

    /// One effect style by index.
    fn effect_style(&self, index: u32) -> Option<EffectListSpec> {
        self.0.effect_style(index).cloned().map(EffectListSpec)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

// ---------------------------------------------------------------------------------------------
// Bullets, tabs and spacing
// ---------------------------------------------------------------------------------------------

#[pymethods]
impl BulletCharacter {
    /// A literal bullet character, such as `"•"` or `"–"`.
    #[new]
    fn new(character: &str) -> Self {
        Self(ooxml::BulletCharacter::new(character))
    }

    /// The character.
    #[getter]
    fn character(&self) -> &str {
        &self.0.character
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl AutoNumberBullet {
    /// An automatically numbered bullet in the given scheme, starting at `start_at` (default `1`).
    #[new]
    #[pyo3(signature = (scheme, start_at = 1))]
    fn new(scheme: AutonumberScheme, start_at: u32) -> Self {
        Self(ooxml::AutoNumberBullet::new(scheme.into()).starting_at(start_at))
    }

    /// Which of the forty-one numbering schemes.
    #[getter]
    fn scheme(&self) -> PyResult<AutonumberScheme> {
        AutonumberScheme::from_model(self.0.scheme)
    }

    /// The number the list starts at.
    #[getter]
    fn start_at(&self) -> u32 {
        self.0.start_at
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl BulletPicture {
    /// A picture bullet, named by the relationship id `Deck.add_image` hands back.
    #[new]
    fn new(image_rel_id: &str) -> Self {
        Self(ooxml::BulletPicture::new(image_rel_id))
    }

    /// The image's relationship id.
    #[getter]
    fn image_rel_id(&self) -> &str {
        &self.0.image_rel_id
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Bullet {
    /// No bullet — `a:buNone`, which is how a paragraph turns one off that it would inherit.
    #[staticmethod]
    fn none() -> Self {
        Self(ooxml::Bullet::None)
    }

    /// A literal character.
    #[staticmethod]
    fn character(character: BulletCharacter) -> Self {
        Self(ooxml::Bullet::Character(character.0))
    }

    /// An automatic number.
    #[staticmethod]
    fn auto_number(bullet: AutoNumberBullet) -> Self {
        Self(ooxml::Bullet::AutoNumber(bullet.0))
    }

    /// A picture.
    #[staticmethod]
    fn picture(picture: BulletPicture) -> Self {
        Self(ooxml::Bullet::Picture(picture.0))
    }

    /// Which kind this is: `"none"`, `"character"`, `"auto_number"` or `"picture"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match &self.0 {
            ooxml::Bullet::None => "none",
            ooxml::Bullet::Character(_) => "character",
            ooxml::Bullet::AutoNumber(_) => "auto_number",
            ooxml::Bullet::Picture(_) => "picture",
        }
    }

    /// The character, when this is a character bullet.
    #[getter]
    fn character_bullet(&self) -> Option<BulletCharacter> {
        match &self.0 {
            ooxml::Bullet::Character(character) => Some(BulletCharacter(character.clone())),
            _ => None,
        }
    }

    /// The numbering, when this is an automatic number.
    #[getter]
    fn auto_number_bullet(&self) -> Option<AutoNumberBullet> {
        match &self.0 {
            ooxml::Bullet::AutoNumber(bullet) => Some(AutoNumberBullet(*bullet)),
            _ => None,
        }
    }

    /// The picture, when this is a picture bullet.
    #[getter]
    fn picture_bullet(&self) -> Option<BulletPicture> {
        match &self.0 {
            ooxml::Bullet::Picture(picture) => Some(BulletPicture(picture.clone())),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl BulletColor {
    /// The bullet takes the colour of the text it marks.
    #[staticmethod]
    fn follow_text() -> Self {
        Self(ooxml::BulletColor::FollowText)
    }

    /// The bullet is painted in a colour of its own.
    #[staticmethod]
    fn explicit(color: ColorSpec) -> Self {
        Self(ooxml::BulletColor::Explicit(color.0))
    }

    /// Whether the bullet follows the text's colour.
    #[getter]
    fn follows_text(&self) -> bool {
        matches!(self.0, ooxml::BulletColor::FollowText)
    }

    /// The bullet's own colour, when it has one.
    #[getter]
    fn color(&self) -> Option<ColorSpec> {
        match &self.0 {
            ooxml::BulletColor::Explicit(color) => Some(ColorSpec(color.clone())),
            ooxml::BulletColor::FollowText => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl BulletSize {
    /// The bullet takes the size of the text it marks.
    #[staticmethod]
    fn follow_text() -> Self {
        Self(ooxml::BulletSize::FollowText)
    }

    /// The bullet is a proportion of the text's size: `0.75` is three quarters.
    #[staticmethod]
    fn percentage(proportion: f64) -> Self {
        Self(ooxml::BulletSize::percentage(proportion))
    }

    /// The bullet is an absolute number of points.
    #[staticmethod]
    fn points(points: f64) -> Self {
        Self(ooxml::BulletSize::points(points))
    }

    /// Which kind this is: `"follow_text"`, `"percentage"` or `"points"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match &self.0 {
            ooxml::BulletSize::FollowText => "follow_text",
            ooxml::BulletSize::Percentage(_) => "percentage",
            ooxml::BulletSize::Points(_) => "points",
        }
    }

    /// The proportion, when this is a proportional size.
    #[getter]
    fn proportion(&self) -> Option<Fraction> {
        match &self.0 {
            ooxml::BulletSize::Percentage(fraction) => Some(Fraction(*fraction)),
            _ => None,
        }
    }

    /// The absolute size, when this is one.
    #[getter]
    fn size(&self) -> Option<FontSize> {
        match &self.0 {
            ooxml::BulletSize::Points(size) => Some(FontSize(*size)),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl BulletTypeface {
    /// The bullet uses the typeface of the text it marks.
    #[staticmethod]
    fn follow_text() -> Self {
        Self(ooxml::BulletTypeface::FollowText)
    }

    /// The bullet uses a typeface of its own — `"Wingdings"`, typically.
    #[staticmethod]
    fn named(typeface: &str) -> Self {
        Self(ooxml::BulletTypeface::named(typeface))
    }

    /// Whether the bullet follows the text's typeface.
    #[getter]
    fn follows_text(&self) -> bool {
        matches!(self.0, ooxml::BulletTypeface::FollowText)
    }

    /// The bullet's own font, when it has one.
    #[getter]
    fn font(&self) -> Option<TextFont> {
        match &self.0 {
            ooxml::BulletTypeface::Explicit(font) => Some(TextFont(font.clone())),
            ooxml::BulletTypeface::FollowText => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl TextSpacing {
    /// A proportion of the line's own height: `1.5` is one-and-a-half spacing.
    #[staticmethod]
    fn proportion(proportion: f64) -> Self {
        Self(ooxml::TextSpacing::proportion(proportion))
    }

    /// An absolute number of points.
    #[staticmethod]
    fn points(points: f64) -> Self {
        Self(ooxml::TextSpacing::points(points))
    }

    /// Which kind this is: `"percentage"` or `"points"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match &self.0 {
            ooxml::TextSpacing::Percentage(_) => "percentage",
            ooxml::TextSpacing::Points(_) => "points",
        }
    }

    /// The proportion, when this is proportional spacing.
    #[getter]
    fn ratio(&self) -> Option<Fraction> {
        match &self.0 {
            ooxml::TextSpacing::Percentage(fraction) => Some(Fraction(*fraction)),
            ooxml::TextSpacing::Points(_) => None,
        }
    }

    /// The absolute measure, when this is absolute spacing.
    #[getter]
    fn measure(&self) -> Option<TextPoint> {
        match &self.0 {
            ooxml::TextSpacing::Points(points) => Some(TextPoint(*points)),
            ooxml::TextSpacing::Percentage(_) => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl UnderlineLine {
    /// The underline takes the run's own outline.
    #[staticmethod]
    fn follow_text() -> Self {
        Self(ooxml::UnderlineLine::FollowText)
    }

    /// The underline is drawn with a line of its own.
    #[staticmethod]
    fn explicit(line: LineSpec) -> Self {
        Self(ooxml::UnderlineLine::Explicit(line.0))
    }

    /// Whether the underline follows the run's outline.
    #[getter]
    fn follows_text(&self) -> bool {
        matches!(self.0, ooxml::UnderlineLine::FollowText)
    }

    /// The underline's own line, when it has one.
    #[getter]
    fn line(&self) -> Option<LineSpec> {
        match &self.0 {
            ooxml::UnderlineLine::Explicit(line) => Some(LineSpec(line.clone())),
            ooxml::UnderlineLine::FollowText => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl UnderlineFill {
    /// The underline takes the run's own fill.
    #[staticmethod]
    fn follow_text() -> Self {
        Self(ooxml::UnderlineFill::FollowText)
    }

    /// The underline is painted with a fill of its own.
    #[staticmethod]
    fn explicit(fill: FillSpec) -> Self {
        Self(ooxml::UnderlineFill::Explicit(fill.0))
    }

    /// Whether the underline follows the run's fill.
    #[getter]
    fn follows_text(&self) -> bool {
        matches!(self.0, ooxml::UnderlineFill::FollowText)
    }

    /// The underline's own fill, when it has one.
    #[getter]
    fn fill(&self) -> Option<FillSpec> {
        match &self.0 {
            ooxml::UnderlineFill::Explicit(fill) => Some(FillSpec(fill.clone())),
            ooxml::UnderlineFill::FollowText => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl TabStop {
    /// A tab stop at the given number of points, with the given alignment.
    #[staticmethod]
    fn at_points(points: f64, alignment: TabAlignment) -> Self {
        Self(ooxml::TabStop::at_points(points, alignment.into()))
    }

    /// A tab stop at an absolute position, optionally with an alignment.
    #[new]
    #[pyo3(signature = (position, alignment = None))]
    fn new(position: Emu, alignment: Option<TabAlignment>) -> Self {
        Self(ooxml::TabStop {
            position: position.0,
            alignment: alignment.map(Into::into),
        })
    }

    /// Where the stop is.
    #[getter]
    fn position(&self) -> Emu {
        Emu(self.0.position)
    }

    /// The position in points.
    #[getter]
    fn position_points(&self) -> f64 {
        self.0.position_points()
    }

    /// How text aligns at the stop, when stated.
    #[getter]
    fn alignment(&self) -> PyResult<Option<TabAlignment>> {
        self.0.alignment.map(TabAlignment::from_model).transpose()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

// ---------------------------------------------------------------------------------------------
// Run and paragraph properties
// ---------------------------------------------------------------------------------------------

#[pymethods]
impl CharacterPropertiesSpec {
    /// A specification that states nothing. Everything it does not state is inherited.
    #[new]
    fn new() -> Self {
        Self(ooxml::CharacterPropertiesSpec::new())
    }

    /// This specification at the given size in points.
    fn with_size_points(&self, points: f64) -> Self {
        Self(self.0.clone().with_size_points(points))
    }

    /// This specification at the given size.
    fn with_size(&self, size: FontSize) -> Self {
        Self(self.0.clone().with_size(size.0))
    }

    /// This specification, bold or not.
    fn with_bold(&self, bold: bool) -> Self {
        Self(self.0.clone().with_bold(bold))
    }

    /// This specification, italic or not.
    fn with_italic(&self, italic: bool) -> Self {
        Self(self.0.clone().with_italic(italic))
    }

    /// This specification with the given underline style.
    fn with_underline(&self, underline: TextUnderline) -> Self {
        Self(self.0.clone().with_underline(underline.into()))
    }

    /// This specification with the given strike-through.
    fn with_strike(&self, strike: TextStrike) -> Self {
        Self(self.0.clone().with_strike(strike.into()))
    }

    /// This specification with the given capitalisation.
    fn with_capitalization(&self, capitalization: TextCapitalization) -> Self {
        Self(self.0.clone().with_capitalization(capitalization.into()))
    }

    /// This specification with the given letter spacing, in points.
    fn with_spacing_points(&self, points: f64) -> Self {
        Self(self.0.clone().with_spacing_points(points))
    }

    /// This specification with the given kerning threshold, in points.
    fn with_kerning_points(&self, points: f64) -> Self {
        Self(self.0.clone().with_kerning_points(points))
    }

    /// This specification raised or lowered by the given proportion of the font size.
    fn with_baseline(&self, baseline: Fraction) -> Self {
        Self(self.0.clone().with_baseline(baseline.0))
    }

    /// This specification tagged with the given language, such as `"en-GB"`.
    fn with_language(&self, language: &str) -> Self {
        Self(self.0.clone().with_language(language))
    }

    /// This specification in the given colour — a solid fill, which is what a colour is here.
    fn with_color(&self, color: ColorSpec) -> Self {
        Self(self.0.clone().with_color(color.0))
    }

    /// This specification with the given fill, which may be a gradient or a picture.
    fn with_fill(&self, fill: FillSpec) -> Self {
        Self(self.0.clone().with_fill(fill.0))
    }

    /// This specification with the given text outline.
    fn with_outline(&self, outline: LineSpec) -> Self {
        Self(self.0.clone().with_outline(outline.0))
    }

    /// This specification with the given text effects.
    fn with_effects(&self, effects: EffectListSpec) -> Self {
        Self(self.0.clone().with_effects(effects.0))
    }

    /// This specification with the given highlight colour.
    fn with_highlight(&self, highlight: ColorSpec) -> Self {
        Self(self.0.clone().with_highlight(highlight.0))
    }

    /// This specification with the given underline line style.
    fn with_underline_line(&self, underline_line: UnderlineLine) -> Self {
        Self(self.0.clone().with_underline_line(underline_line.0))
    }

    /// This specification with the given underline fill.
    fn with_underline_fill(&self, underline_fill: UnderlineFill) -> Self {
        Self(self.0.clone().with_underline_fill(underline_fill.0))
    }

    /// This specification in the given typeface, for the Latin slot.
    fn with_font(&self, typeface: &str) -> Self {
        Self(self.0.clone().with_font(typeface))
    }

    /// This specification with the given font for one script slot.
    fn with_font_for(&self, slot: FontSlot, font: TextFont) -> Self {
        Self(self.0.clone().with_font_for(slot.into(), font.0))
    }

    /// The size, when stated.
    #[getter]
    fn size(&self) -> Option<FontSize> {
        self.0.size().map(FontSize)
    }

    /// The size in points, when stated.
    #[getter]
    fn size_points(&self) -> Option<f64> {
        self.0.size_points()
    }

    /// Whether bold is stated, and what it says.
    #[getter]
    fn is_bold(&self) -> Option<bool> {
        self.0.is_bold()
    }

    /// Whether italic is stated, and what it says.
    #[getter]
    fn is_italic(&self) -> Option<bool> {
        self.0.is_italic()
    }

    /// The underline style, when stated.
    #[getter]
    fn underline(&self) -> PyResult<Option<TextUnderline>> {
        self.0
            .underline()
            .map(TextUnderline::from_model)
            .transpose()
    }

    /// The strike-through, when stated.
    #[getter]
    fn strike(&self) -> PyResult<Option<TextStrike>> {
        self.0.strike().map(TextStrike::from_model).transpose()
    }

    /// The capitalisation, when stated.
    #[getter]
    fn capitalization(&self) -> PyResult<Option<TextCapitalization>> {
        self.0
            .capitalization()
            .map(TextCapitalization::from_model)
            .transpose()
    }

    /// The letter spacing in points, when stated.
    #[getter]
    fn spacing_points(&self) -> Option<f64> {
        self.0.spacing_points()
    }

    /// The kerning threshold in points, when stated.
    #[getter]
    fn kerning_points(&self) -> Option<f64> {
        self.0.kerning_points()
    }

    /// The baseline offset, when stated.
    #[getter]
    fn baseline(&self) -> Option<Fraction> {
        self.0.baseline().map(Fraction)
    }

    /// The language tag, when stated.
    #[getter]
    fn language(&self) -> Option<&str> {
        self.0.language()
    }

    /// The fill, when stated.
    #[getter]
    fn fill(&self) -> Option<FillSpec> {
        self.0.fill().cloned().map(FillSpec)
    }

    /// The text outline, when stated.
    #[getter]
    fn outline(&self) -> Option<LineSpec> {
        self.0.outline().cloned().map(LineSpec)
    }

    /// The text effects, when stated.
    #[getter]
    fn effects(&self) -> Option<EffectListSpec> {
        self.0.effects().cloned().map(EffectListSpec)
    }

    /// The highlight colour, when stated.
    #[getter]
    fn highlight(&self) -> Option<ColorSpec> {
        self.0.highlight().cloned().map(ColorSpec)
    }

    /// The underline line style, when stated.
    #[getter]
    fn underline_line(&self) -> Option<UnderlineLine> {
        self.0.underline_line().cloned().map(UnderlineLine)
    }

    /// The underline fill, when stated.
    #[getter]
    fn underline_fill(&self) -> Option<UnderlineFill> {
        self.0.underline_fill().cloned().map(UnderlineFill)
    }

    /// The font for one script slot, when stated.
    fn font(&self, slot: FontSlot) -> Option<TextFont> {
        self.0.font(slot.into()).cloned().map(TextFont)
    }

    /// This specification laid over `lower`: whatever this one states wins, and whatever it leaves
    /// unstated comes from `lower`. The same walk the `effective_…` readers make, one rung at a
    /// time.
    fn merge_under(&self, lower: &Self) -> Self {
        Self(self.0.clone().merge_under(&lower.0))
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ParagraphPropertiesSpec {
    /// A specification that states nothing. Everything it does not state is inherited.
    #[new]
    fn new() -> Self {
        Self(ooxml::ParagraphPropertiesSpec::new())
    }

    /// This specification at the given list level.
    fn with_level(&self, level: IndentLevel) -> Self {
        Self(self.0.clone().with_level(level.0))
    }

    /// This specification with the given alignment.
    fn with_alignment(&self, alignment: TextAlignment) -> Self {
        Self(self.0.clone().with_alignment(alignment.into()))
    }

    /// This specification with the given left margin, in points.
    fn with_left_margin_points(&self, points: f64) -> Self {
        Self(self.0.clone().with_left_margin_points(points))
    }

    /// This specification with the given right margin, in points.
    fn with_right_margin_points(&self, points: f64) -> Self {
        Self(self.0.clone().with_right_margin_points(points))
    }

    /// This specification with the given first-line indent, in points.
    fn with_indent_points(&self, points: f64) -> Self {
        Self(self.0.clone().with_indent_points(points))
    }

    /// This specification with the given default tab size, in points.
    fn with_default_tab_size_points(&self, points: f64) -> Self {
        Self(self.0.clone().with_default_tab_size_points(points))
    }

    /// This specification, right-to-left or not.
    fn with_right_to_left(&self, right_to_left: bool) -> Self {
        Self(self.0.clone().with_right_to_left(right_to_left))
    }

    /// This specification with the given font alignment within the line box.
    fn with_font_alignment(&self, font_alignment: FontAlignment) -> Self {
        Self(self.0.clone().with_font_alignment(font_alignment.into()))
    }

    /// This specification with the given line spacing.
    fn with_line_spacing(&self, spacing: TextSpacing) -> Self {
        Self(self.0.clone().with_line_spacing(spacing.0))
    }

    /// This specification with the given space before the paragraph.
    fn with_space_before(&self, spacing: TextSpacing) -> Self {
        Self(self.0.clone().with_space_before(spacing.0))
    }

    /// This specification with the given space after the paragraph.
    fn with_space_after(&self, spacing: TextSpacing) -> Self {
        Self(self.0.clone().with_space_after(spacing.0))
    }

    /// This specification with the given bullet.
    fn with_bullet(&self, bullet: Bullet) -> Self {
        Self(self.0.clone().with_bullet(bullet.0))
    }

    /// This specification bulleted with the given character.
    fn with_bullet_character(&self, character: &str) -> Self {
        Self(self.0.clone().with_bullet_character(character))
    }

    /// This specification with no bullet — `a:buNone`, which turns off an inherited one.
    fn without_bullet(&self) -> Self {
        Self(self.0.clone().without_bullet())
    }

    /// This specification with the given bullet colour.
    fn with_bullet_color(&self, color: BulletColor) -> Self {
        Self(self.0.clone().with_bullet_color(color.0))
    }

    /// This specification with the given bullet size.
    fn with_bullet_size(&self, size: BulletSize) -> Self {
        Self(self.0.clone().with_bullet_size(size.0))
    }

    /// This specification with the given bullet typeface.
    fn with_bullet_typeface(&self, typeface: BulletTypeface) -> Self {
        Self(self.0.clone().with_bullet_typeface(typeface.0))
    }

    /// This specification with the given tab stops, replacing any it already had.
    fn with_tab_stops(&self, stops: Vec<TabStop>) -> Self {
        Self(
            self.0
                .clone()
                .with_tab_stops(stops.into_iter().map(|stop| stop.0).collect()),
        )
    }

    /// This specification with the given default run properties — what the paragraph's own text
    /// inherits before any run states anything.
    fn with_default_run_properties(&self, properties: CharacterPropertiesSpec) -> Self {
        Self(self.0.clone().with_default_run_properties(properties.0))
    }

    /// The list level, when stated.
    #[getter]
    fn level(&self) -> Option<IndentLevel> {
        self.0.level().map(IndentLevel)
    }

    /// The alignment, when stated.
    #[getter]
    fn alignment(&self) -> PyResult<Option<TextAlignment>> {
        self.0
            .alignment()
            .map(TextAlignment::from_model)
            .transpose()
    }

    /// The left margin in points, when stated.
    #[getter]
    fn left_margin_points(&self) -> Option<f64> {
        self.0.left_margin_points()
    }

    /// The right margin in points, when stated.
    #[getter]
    fn right_margin_points(&self) -> Option<f64> {
        self.0.right_margin_points()
    }

    /// The first-line indent in points, when stated.
    #[getter]
    fn indent_points(&self) -> Option<f64> {
        self.0.indent_points()
    }

    /// The default tab size in points, when stated.
    #[getter]
    fn default_tab_size_points(&self) -> Option<f64> {
        self.0.default_tab_size_points()
    }

    /// Whether right-to-left is stated, and what it says.
    #[getter]
    fn is_right_to_left(&self) -> Option<bool> {
        self.0.is_right_to_left()
    }

    /// The font alignment, when stated.
    #[getter]
    fn font_alignment(&self) -> PyResult<Option<FontAlignment>> {
        self.0
            .font_alignment()
            .map(FontAlignment::from_model)
            .transpose()
    }

    /// The line spacing, when stated.
    #[getter]
    fn line_spacing(&self) -> Option<TextSpacing> {
        self.0.line_spacing().map(TextSpacing)
    }

    /// The space before the paragraph, when stated.
    #[getter]
    fn space_before(&self) -> Option<TextSpacing> {
        self.0.space_before().map(TextSpacing)
    }

    /// The space after the paragraph, when stated.
    #[getter]
    fn space_after(&self) -> Option<TextSpacing> {
        self.0.space_after().map(TextSpacing)
    }

    /// The bullet, when stated.
    #[getter]
    fn bullet(&self) -> Option<Bullet> {
        self.0.bullet().cloned().map(Bullet)
    }

    /// The bullet colour, when stated.
    #[getter]
    fn bullet_color(&self) -> Option<BulletColor> {
        self.0.bullet_color().cloned().map(BulletColor)
    }

    /// The bullet size, when stated.
    #[getter]
    fn bullet_size(&self) -> Option<BulletSize> {
        self.0.bullet_size().map(BulletSize)
    }

    /// The bullet typeface, when stated.
    #[getter]
    fn bullet_typeface(&self) -> Option<BulletTypeface> {
        self.0.bullet_typeface().cloned().map(BulletTypeface)
    }

    /// The tab stops, in order.
    #[getter]
    fn tab_stops(&self) -> Vec<TabStop> {
        self.0.tab_stops().iter().copied().map(TabStop).collect()
    }

    /// The default run properties, when stated.
    #[getter]
    fn default_run_properties(&self) -> Option<CharacterPropertiesSpec> {
        self.0
            .default_run_properties()
            .cloned()
            .map(CharacterPropertiesSpec)
    }

    /// This specification laid over `lower`: whatever this one states wins.
    fn merge_under(&self, lower: &Self) -> Self {
        Self(self.0.clone().merge_under(&lower.0))
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// Adds every class in this module to the extension module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<CharacterPropertiesSpec>()?;
    module.add_class::<ParagraphPropertiesSpec>()?;
    module.add_class::<TextFont>()?;
    module.add_class::<ThemeFontReference>()?;
    module.add_class::<TabStop>()?;
    module.add_class::<Bullet>()?;
    module.add_class::<BulletCharacter>()?;
    module.add_class::<AutoNumberBullet>()?;
    module.add_class::<BulletPicture>()?;
    module.add_class::<BulletColor>()?;
    module.add_class::<BulletSize>()?;
    module.add_class::<BulletTypeface>()?;
    module.add_class::<TextSpacing>()?;
    module.add_class::<UnderlineLine>()?;
    module.add_class::<UnderlineFill>()?;
    module.add_class::<SupplementalFont>()?;
    module.add_class::<FontCollection>()?;
    module.add_class::<FontScheme>()?;
    module.add_class::<ThemeInfo>()
}
