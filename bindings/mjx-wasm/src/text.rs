//! Text: run properties, paragraph properties, bullets, tabs, fonts, and the theme's font scheme.
//!
//! [`CharacterPropertiesSpec`] and [`ParagraphPropertiesSpec`] are the two classes most of a
//! deck's formatting goes through, and both mirror their Rust builders method for method — every
//! `with…` returns a **new** value rather than mutating, so a specification can be built once and
//! applied many times (and each intermediate value is its own wasm object; see `free()`):
//!
//! ```js
//! const heading = new CharacterPropertiesSpec()
//!   .withSizePoints(28)
//!   .withBold(true)
//!   .withColor(ColorSpec.srgb("1F3864"));
//! for (let slide = 0; slide < deck.slideCount(); slide += 1) {
//!   deck.setShapeRunProperties(slide, 0, heading);
//! }
//! heading.free();
//! ```

use wasm_bindgen::prelude::*;

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

#[wasm_bindgen]
impl TextFont {
    /// A typeface by name. `"+mj-lt"` and `"+mn-lt"` name the theme's major and minor Latin fonts.
    #[wasm_bindgen(constructor)]
    pub fn new(
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
    #[wasm_bindgen(getter, js_name = "typeface")]
    pub fn typeface(&self) -> String {
        self.0.typeface.clone()
    }

    /// The PANOSE classification, when the document states one.
    #[wasm_bindgen(getter, js_name = "panose")]
    pub fn panose(&self) -> Option<String> {
        self.0.panose.clone()
    }

    /// The pitch and family byte, when stated.
    #[wasm_bindgen(getter, js_name = "pitchFamily")]
    pub fn pitch_family(&self) -> Option<i32> {
        self.0.pitch_family
    }

    /// The character set byte, when stated.
    #[wasm_bindgen(getter, js_name = "charset")]
    pub fn charset(&self) -> Option<i32> {
        self.0.charset
    }

    /// Whether this names a theme font (`+mj-lt`, `+mn-ea`, …) rather than a typeface.
    #[wasm_bindgen(getter, js_name = "isThemeReference")]
    pub fn is_theme_reference(&self) -> bool {
        self.0.is_theme_reference()
    }

    /// Which theme font this names, when it names one.
    #[wasm_bindgen(getter, js_name = "themeReference")]
    pub fn theme_reference(&self) -> Option<ThemeFontReference> {
        self.0.theme_reference().map(ThemeFontReference)
    }
}

#[wasm_bindgen]
impl ThemeFontReference {
    /// The major or minor collection, and the script slot within it.
    #[wasm_bindgen(constructor)]
    pub fn new(collection: FontSchemeSlot, slot: FontSlot) -> Self {
        Self(ooxml::ThemeFontReference {
            collection: collection.into(),
            slot: slot.into(),
        })
    }

    /// Which collection — the major (heading) or minor (body) fonts.
    #[wasm_bindgen(getter, js_name = "collection")]
    pub fn collection(&self) -> Result<FontSchemeSlot, JsValue> {
        FontSchemeSlot::from_model(self.0.collection)
    }

    /// Which script slot within the collection.
    #[wasm_bindgen(getter, js_name = "slot")]
    pub fn slot(&self) -> Result<FontSlot, JsValue> {
        FontSlot::from_model(self.0.slot)
    }
}

#[wasm_bindgen]
impl SupplementalFont {
    /// The script this font covers, as the theme names it.
    #[wasm_bindgen(getter, js_name = "script")]
    pub fn script(&self) -> String {
        self.0.script().to_owned()
    }

    /// The typeface for that script.
    #[wasm_bindgen(getter, js_name = "typeface")]
    pub fn typeface(&self) -> String {
        self.0.typeface().to_owned()
    }
}

#[wasm_bindgen]
impl FontCollection {
    /// The font for one script slot, when the collection states one.
    pub fn font(&self, slot: FontSlot) -> Option<TextFont> {
        self.0.font(slot.into()).cloned().map(TextFont)
    }

    /// Every supplemental font this collection lists.
    #[wasm_bindgen(getter, js_name = "supplementalFonts")]
    pub fn supplemental_fonts(&self) -> Vec<SupplementalFont> {
        self.0
            .supplemental_fonts()
            .iter()
            .cloned()
            .map(SupplementalFont)
            .collect()
    }

    /// The supplemental font for one script, when the collection states one.
    #[wasm_bindgen(js_name = "supplementalFont")]
    pub fn supplemental_font(&self, script: &str) -> Option<SupplementalFont> {
        self.0
            .supplemental_font(script)
            .cloned()
            .map(SupplementalFont)
    }
}

#[wasm_bindgen]
impl FontScheme {
    /// The scheme's name, as the theme states it.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.0.name().to_owned()
    }

    /// The major (heading) collection.
    #[wasm_bindgen(getter, js_name = "major")]
    pub fn major(&self) -> FontCollection {
        FontCollection(self.0.major().clone())
    }

    /// The minor (body) collection.
    #[wasm_bindgen(getter, js_name = "minor")]
    pub fn minor(&self) -> FontCollection {
        FontCollection(self.0.minor().clone())
    }

    /// One of the two collections by name.
    pub fn collection(&self, slot: FontSchemeSlot) -> FontCollection {
        FontCollection(self.0.collection(slot.into()).clone())
    }

    /// The font a theme reference resolves to, when the scheme states one.
    pub fn font(&self, reference: &ThemeFontReference) -> Option<TextFont> {
        self.0.font(reference.0).cloned().map(TextFont)
    }

    /// The typeface a font resolves to: itself, unless it is a theme reference, in which case the
    /// font this scheme names for that slot.
    pub fn resolve(&self, font: &TextFont) -> Option<TextFont> {
        self.0.resolve(&font.0).cloned().map(TextFont)
    }
}

#[wasm_bindgen]
impl ThemeInfo {
    /// The colour a scheme slot resolves to in this theme, when it states one.
    pub fn color(&self, slot: ColorSchemeSlot) -> Option<ColorSpec> {
        self.0.color(slot.into()).cloned().map(ColorSpec)
    }

    /// Every slot the theme states a colour for, in order.
    ///
    /// Paired with `colorAt`: `wasm-bindgen` cannot project a list of tuples, and an array of
    /// two-element arrays would type as `any[][]`.
    #[wasm_bindgen(getter, js_name = "colorSlots")]
    pub fn color_slots(&self) -> Result<Vec<ColorSchemeSlot>, JsValue> {
        self.0
            .colors()
            .map(|(slot, _)| ColorSchemeSlot::from_model(slot))
            .collect()
    }

    /// The colour at one position of `colorSlots`.
    #[wasm_bindgen(js_name = "colorAt")]
    pub fn color_at(&self, index: u32) -> Option<ColorSpec> {
        self.0
            .colors()
            .nth(index as usize)
            .map(|(_, color)| ColorSpec(color.clone()))
    }

    /// The theme's font scheme, when it states one.
    #[wasm_bindgen(getter, js_name = "fontScheme")]
    pub fn font_scheme(&self) -> Option<FontScheme> {
        self.0.font_scheme().cloned().map(FontScheme)
    }

    /// The theme's fill style matrix, in order.
    #[wasm_bindgen(getter, js_name = "fillStyles")]
    pub fn fill_styles(&self) -> Vec<FillSpec> {
        self.0.fill_styles().iter().cloned().map(FillSpec).collect()
    }

    /// One fill style by index — the number a shape's `a:fillRef@idx` names, counting from one.
    #[wasm_bindgen(js_name = "fillStyle")]
    pub fn fill_style(&self, index: u32) -> Option<FillSpec> {
        self.0.fill_style(index).cloned().map(FillSpec)
    }

    /// The theme's line style matrix, in order.
    #[wasm_bindgen(getter, js_name = "lineStyles")]
    pub fn line_styles(&self) -> Vec<LineSpec> {
        self.0.line_styles().iter().cloned().map(LineSpec).collect()
    }

    /// One line style by index.
    #[wasm_bindgen(js_name = "lineStyle")]
    pub fn line_style(&self, index: u32) -> Option<LineSpec> {
        self.0.line_style(index).cloned().map(LineSpec)
    }

    /// One effect style by index.
    #[wasm_bindgen(js_name = "effectStyle")]
    pub fn effect_style(&self, index: u32) -> Option<EffectListSpec> {
        self.0.effect_style(index).cloned().map(EffectListSpec)
    }
}

// ---------------------------------------------------------------------------------------------
// Bullets, tabs and spacing
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl BulletCharacter {
    /// A literal bullet character, such as `"•"` or `"–"`.
    #[wasm_bindgen(constructor)]
    pub fn new(character: &str) -> Self {
        Self(ooxml::BulletCharacter::new(character))
    }

    /// The character.
    #[wasm_bindgen(getter, js_name = "character")]
    pub fn character(&self) -> String {
        self.0.character.clone()
    }
}

#[wasm_bindgen]
impl AutoNumberBullet {
    /// An automatically numbered bullet in the given scheme, starting at `start_at` (default `1`).
    #[wasm_bindgen(constructor)]
    pub fn new(scheme: AutonumberScheme, start_at: u32) -> Self {
        Self(ooxml::AutoNumberBullet::new(scheme.into()).starting_at(start_at))
    }

    /// Which of the forty-one numbering schemes.
    #[wasm_bindgen(getter, js_name = "scheme")]
    pub fn scheme(&self) -> Result<AutonumberScheme, JsValue> {
        AutonumberScheme::from_model(self.0.scheme)
    }

    /// The number the list starts at.
    #[wasm_bindgen(getter, js_name = "startAt")]
    pub fn start_at(&self) -> u32 {
        self.0.start_at
    }
}

#[wasm_bindgen]
impl BulletPicture {
    /// A picture bullet, named by the relationship id `Deck.add_image` hands back.
    #[wasm_bindgen(constructor)]
    pub fn new(image_rel_id: &str) -> Self {
        Self(ooxml::BulletPicture::new(image_rel_id))
    }

    /// The image's relationship id.
    #[wasm_bindgen(getter, js_name = "imageRelId")]
    pub fn image_rel_id(&self) -> String {
        self.0.image_rel_id.clone()
    }
}

#[wasm_bindgen]
impl Bullet {
    /// No bullet — `a:buNone`, which is how a paragraph turns one off that it would inherit.
    pub fn none() -> Self {
        Self(ooxml::Bullet::None)
    }

    /// A literal character.
    pub fn character(character: &BulletCharacter) -> Self {
        Self(ooxml::Bullet::Character(character.0.clone()))
    }

    /// An automatic number.
    #[wasm_bindgen(js_name = "autoNumber")]
    pub fn auto_number(bullet: &AutoNumberBullet) -> Self {
        Self(ooxml::Bullet::AutoNumber(bullet.0))
    }

    /// A picture.
    pub fn picture(picture: &BulletPicture) -> Self {
        Self(ooxml::Bullet::Picture(picture.0.clone()))
    }

    /// Which kind this is: `"none"`, `"character"`, `"auto_number"` or `"picture"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        match &self.0 {
            ooxml::Bullet::None => "none".to_owned(),
            ooxml::Bullet::Character(_) => "character".to_owned(),
            ooxml::Bullet::AutoNumber(_) => "auto_number".to_owned(),
            ooxml::Bullet::Picture(_) => "picture".to_owned(),
        }
    }

    /// The character, when this is a character bullet.
    #[wasm_bindgen(getter, js_name = "characterBullet")]
    pub fn character_bullet(&self) -> Option<BulletCharacter> {
        match &self.0 {
            ooxml::Bullet::Character(character) => Some(BulletCharacter(character.clone())),
            _ => None,
        }
    }

    /// The numbering, when this is an automatic number.
    #[wasm_bindgen(getter, js_name = "autoNumberBullet")]
    pub fn auto_number_bullet(&self) -> Option<AutoNumberBullet> {
        match &self.0 {
            ooxml::Bullet::AutoNumber(bullet) => Some(AutoNumberBullet(*bullet)),
            _ => None,
        }
    }

    /// The picture, when this is a picture bullet.
    #[wasm_bindgen(getter, js_name = "pictureBullet")]
    pub fn picture_bullet(&self) -> Option<BulletPicture> {
        match &self.0 {
            ooxml::Bullet::Picture(picture) => Some(BulletPicture(picture.clone())),
            _ => None,
        }
    }
}

#[wasm_bindgen]
impl BulletColor {
    /// The bullet takes the colour of the text it marks.
    #[wasm_bindgen(js_name = "followText")]
    pub fn follow_text() -> Self {
        Self(ooxml::BulletColor::FollowText)
    }

    /// The bullet is painted in a colour of its own.
    pub fn explicit(color: &ColorSpec) -> Self {
        Self(ooxml::BulletColor::Explicit(color.0.clone()))
    }

    /// Whether the bullet follows the text's colour.
    #[wasm_bindgen(getter, js_name = "followsText")]
    pub fn follows_text(&self) -> bool {
        matches!(self.0, ooxml::BulletColor::FollowText)
    }

    /// The bullet's own colour, when it has one.
    #[wasm_bindgen(getter, js_name = "color")]
    pub fn color(&self) -> Option<ColorSpec> {
        match &self.0 {
            ooxml::BulletColor::Explicit(color) => Some(ColorSpec(color.clone())),
            ooxml::BulletColor::FollowText => None,
        }
    }
}

#[wasm_bindgen]
impl BulletSize {
    /// The bullet takes the size of the text it marks.
    #[wasm_bindgen(js_name = "followText")]
    pub fn follow_text() -> Self {
        Self(ooxml::BulletSize::FollowText)
    }

    /// The bullet is a proportion of the text's size: `0.75` is three quarters.
    pub fn percentage(proportion: f64) -> Self {
        Self(ooxml::BulletSize::percentage(proportion))
    }

    /// The bullet is an absolute number of points.
    pub fn points(points: f64) -> Self {
        Self(ooxml::BulletSize::points(points))
    }

    /// Which kind this is: `"follow_text"`, `"percentage"` or `"points"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        match &self.0 {
            ooxml::BulletSize::FollowText => "follow_text".to_owned(),
            ooxml::BulletSize::Percentage(_) => "percentage".to_owned(),
            ooxml::BulletSize::Points(_) => "points".to_owned(),
        }
    }

    /// The proportion, when this is a proportional size.
    #[wasm_bindgen(getter, js_name = "proportion")]
    pub fn proportion(&self) -> Option<Fraction> {
        match &self.0 {
            ooxml::BulletSize::Percentage(fraction) => Some(Fraction(*fraction)),
            _ => None,
        }
    }

    /// The absolute size, when this is one.
    #[wasm_bindgen(getter, js_name = "size")]
    pub fn size(&self) -> Option<FontSize> {
        match &self.0 {
            ooxml::BulletSize::Points(size) => Some(FontSize(*size)),
            _ => None,
        }
    }
}

#[wasm_bindgen]
impl BulletTypeface {
    /// The bullet uses the typeface of the text it marks.
    #[wasm_bindgen(js_name = "followText")]
    pub fn follow_text() -> Self {
        Self(ooxml::BulletTypeface::FollowText)
    }

    /// The bullet uses a typeface of its own — `"Wingdings"`, typically.
    pub fn named(typeface: &str) -> Self {
        Self(ooxml::BulletTypeface::named(typeface))
    }

    /// Whether the bullet follows the text's typeface.
    #[wasm_bindgen(getter, js_name = "followsText")]
    pub fn follows_text(&self) -> bool {
        matches!(self.0, ooxml::BulletTypeface::FollowText)
    }

    /// The bullet's own font, when it has one.
    #[wasm_bindgen(getter, js_name = "font")]
    pub fn font(&self) -> Option<TextFont> {
        match &self.0 {
            ooxml::BulletTypeface::Explicit(font) => Some(TextFont(font.clone())),
            ooxml::BulletTypeface::FollowText => None,
        }
    }
}

#[wasm_bindgen]
impl TextSpacing {
    /// A proportion of the line's own height: `1.5` is one-and-a-half spacing.
    pub fn proportion(proportion: f64) -> Self {
        Self(ooxml::TextSpacing::proportion(proportion))
    }

    /// An absolute number of points.
    pub fn points(points: f64) -> Self {
        Self(ooxml::TextSpacing::points(points))
    }

    /// Which kind this is: `"percentage"` or `"points"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        match &self.0 {
            ooxml::TextSpacing::Percentage(_) => "percentage".to_owned(),
            ooxml::TextSpacing::Points(_) => "points".to_owned(),
        }
    }

    /// The proportion, when this is proportional spacing.
    #[wasm_bindgen(getter, js_name = "ratio")]
    pub fn ratio(&self) -> Option<Fraction> {
        match &self.0 {
            ooxml::TextSpacing::Percentage(fraction) => Some(Fraction(*fraction)),
            ooxml::TextSpacing::Points(_) => None,
        }
    }

    /// The absolute measure, when this is absolute spacing.
    #[wasm_bindgen(getter, js_name = "measure")]
    pub fn measure(&self) -> Option<TextPoint> {
        match &self.0 {
            ooxml::TextSpacing::Points(points) => Some(TextPoint(*points)),
            ooxml::TextSpacing::Percentage(_) => None,
        }
    }
}

#[wasm_bindgen]
impl UnderlineLine {
    /// The underline takes the run's own outline.
    #[wasm_bindgen(js_name = "followText")]
    pub fn follow_text() -> Self {
        Self(ooxml::UnderlineLine::FollowText)
    }

    /// The underline is drawn with a line of its own.
    pub fn explicit(line: &LineSpec) -> Self {
        Self(ooxml::UnderlineLine::Explicit(line.0.clone()))
    }

    /// Whether the underline follows the run's outline.
    #[wasm_bindgen(getter, js_name = "followsText")]
    pub fn follows_text(&self) -> bool {
        matches!(self.0, ooxml::UnderlineLine::FollowText)
    }

    /// The underline's own line, when it has one.
    #[wasm_bindgen(getter, js_name = "line")]
    pub fn line(&self) -> Option<LineSpec> {
        match &self.0 {
            ooxml::UnderlineLine::Explicit(line) => Some(LineSpec(line.clone())),
            ooxml::UnderlineLine::FollowText => None,
        }
    }
}

#[wasm_bindgen]
impl UnderlineFill {
    /// The underline takes the run's own fill.
    #[wasm_bindgen(js_name = "followText")]
    pub fn follow_text() -> Self {
        Self(ooxml::UnderlineFill::FollowText)
    }

    /// The underline is painted with a fill of its own.
    pub fn explicit(fill: &FillSpec) -> Self {
        Self(ooxml::UnderlineFill::Explicit(fill.0.clone()))
    }

    /// Whether the underline follows the run's fill.
    #[wasm_bindgen(getter, js_name = "followsText")]
    pub fn follows_text(&self) -> bool {
        matches!(self.0, ooxml::UnderlineFill::FollowText)
    }

    /// The underline's own fill, when it has one.
    #[wasm_bindgen(getter, js_name = "fill")]
    pub fn fill(&self) -> Option<FillSpec> {
        match &self.0 {
            ooxml::UnderlineFill::Explicit(fill) => Some(FillSpec(fill.clone())),
            ooxml::UnderlineFill::FollowText => None,
        }
    }
}

#[wasm_bindgen]
impl TabStop {
    /// A tab stop at the given number of points, with the given alignment.
    #[wasm_bindgen(js_name = "atPoints")]
    pub fn at_points(points: f64, alignment: TabAlignment) -> Self {
        Self(ooxml::TabStop::at_points(points, alignment.into()))
    }

    /// A tab stop at an absolute position, optionally with an alignment.
    #[wasm_bindgen(constructor)]
    pub fn new(position: &Emu, alignment: Option<TabAlignment>) -> Self {
        Self(ooxml::TabStop {
            position: position.0,
            alignment: alignment.map(Into::into),
        })
    }

    /// Where the stop is.
    #[wasm_bindgen(getter, js_name = "position")]
    pub fn position(&self) -> Emu {
        Emu(self.0.position)
    }

    /// The position in points.
    #[wasm_bindgen(getter, js_name = "positionPoints")]
    pub fn position_points(&self) -> f64 {
        self.0.position_points()
    }

    /// How text aligns at the stop, when stated.
    #[wasm_bindgen(getter, js_name = "alignment")]
    pub fn alignment(&self) -> Result<Option<TabAlignment>, JsValue> {
        self.0.alignment.map(TabAlignment::from_model).transpose()
    }
}

// ---------------------------------------------------------------------------------------------
// Run and paragraph properties
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl CharacterPropertiesSpec {
    /// A specification that states nothing. Everything it does not state is inherited.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self(ooxml::CharacterPropertiesSpec::new())
    }

    /// This specification at the given size in points.
    #[wasm_bindgen(js_name = "withSizePoints")]
    pub fn with_size_points(&self, points: f64) -> Self {
        Self(self.0.clone().with_size_points(points))
    }

    /// This specification at the given size.
    #[wasm_bindgen(js_name = "withSize")]
    pub fn with_size(&self, size: &FontSize) -> Self {
        Self(self.0.clone().with_size(size.0))
    }

    /// This specification, bold or not.
    #[wasm_bindgen(js_name = "withBold")]
    pub fn with_bold(&self, bold: bool) -> Self {
        Self(self.0.clone().with_bold(bold))
    }

    /// This specification, italic or not.
    #[wasm_bindgen(js_name = "withItalic")]
    pub fn with_italic(&self, italic: bool) -> Self {
        Self(self.0.clone().with_italic(italic))
    }

    /// This specification with the given underline style.
    #[wasm_bindgen(js_name = "withUnderline")]
    pub fn with_underline(&self, underline: TextUnderline) -> Self {
        Self(self.0.clone().with_underline(underline.into()))
    }

    /// This specification with the given strike-through.
    #[wasm_bindgen(js_name = "withStrike")]
    pub fn with_strike(&self, strike: TextStrike) -> Self {
        Self(self.0.clone().with_strike(strike.into()))
    }

    /// This specification with the given capitalisation.
    #[wasm_bindgen(js_name = "withCapitalization")]
    pub fn with_capitalization(&self, capitalization: TextCapitalization) -> Self {
        Self(self.0.clone().with_capitalization(capitalization.into()))
    }

    /// This specification with the given letter spacing, in points.
    #[wasm_bindgen(js_name = "withSpacingPoints")]
    pub fn with_spacing_points(&self, points: f64) -> Self {
        Self(self.0.clone().with_spacing_points(points))
    }

    /// This specification with the given kerning threshold, in points.
    #[wasm_bindgen(js_name = "withKerningPoints")]
    pub fn with_kerning_points(&self, points: f64) -> Self {
        Self(self.0.clone().with_kerning_points(points))
    }

    /// This specification raised or lowered by the given proportion of the font size.
    #[wasm_bindgen(js_name = "withBaseline")]
    pub fn with_baseline(&self, baseline: &Fraction) -> Self {
        Self(self.0.clone().with_baseline(baseline.0))
    }

    /// This specification tagged with the given language, such as `"en-GB"`.
    #[wasm_bindgen(js_name = "withLanguage")]
    pub fn with_language(&self, language: &str) -> Self {
        Self(self.0.clone().with_language(language))
    }

    /// This specification in the given colour — a solid fill, which is what a colour is here.
    #[wasm_bindgen(js_name = "withColor")]
    pub fn with_color(&self, color: &ColorSpec) -> Self {
        Self(self.0.clone().with_color(color.0.clone()))
    }

    /// This specification with the given fill, which may be a gradient or a picture.
    #[wasm_bindgen(js_name = "withFill")]
    pub fn with_fill(&self, fill: &FillSpec) -> Self {
        Self(self.0.clone().with_fill(fill.0.clone()))
    }

    /// This specification with the given text outline.
    #[wasm_bindgen(js_name = "withOutline")]
    pub fn with_outline(&self, outline: &LineSpec) -> Self {
        Self(self.0.clone().with_outline(outline.0.clone()))
    }

    /// This specification with the given text effects.
    #[wasm_bindgen(js_name = "withEffects")]
    pub fn with_effects(&self, effects: &EffectListSpec) -> Self {
        Self(self.0.clone().with_effects(effects.0.clone()))
    }

    /// This specification with the given highlight colour.
    #[wasm_bindgen(js_name = "withHighlight")]
    pub fn with_highlight(&self, highlight: &ColorSpec) -> Self {
        Self(self.0.clone().with_highlight(highlight.0.clone()))
    }

    /// This specification with the given underline line style.
    #[wasm_bindgen(js_name = "withUnderlineLine")]
    pub fn with_underline_line(&self, underline_line: &UnderlineLine) -> Self {
        Self(self.0.clone().with_underline_line(underline_line.0.clone()))
    }

    /// This specification with the given underline fill.
    #[wasm_bindgen(js_name = "withUnderlineFill")]
    pub fn with_underline_fill(&self, underline_fill: &UnderlineFill) -> Self {
        Self(self.0.clone().with_underline_fill(underline_fill.0.clone()))
    }

    /// This specification in the given typeface, for the Latin slot.
    #[wasm_bindgen(js_name = "withFont")]
    pub fn with_font(&self, typeface: &str) -> Self {
        Self(self.0.clone().with_font(typeface))
    }

    /// This specification with the given font for one script slot.
    #[wasm_bindgen(js_name = "withFontFor")]
    pub fn with_font_for(&self, slot: FontSlot, font: &TextFont) -> Self {
        Self(self.0.clone().with_font_for(slot.into(), font.0.clone()))
    }

    /// The size, when stated.
    #[wasm_bindgen(getter, js_name = "size")]
    pub fn size(&self) -> Option<FontSize> {
        self.0.size().map(FontSize)
    }

    /// The size in points, when stated.
    #[wasm_bindgen(getter, js_name = "sizePoints")]
    pub fn size_points(&self) -> Option<f64> {
        self.0.size_points()
    }

    /// Whether bold is stated, and what it says.
    #[wasm_bindgen(getter, js_name = "isBold")]
    pub fn is_bold(&self) -> Option<bool> {
        self.0.is_bold()
    }

    /// Whether italic is stated, and what it says.
    #[wasm_bindgen(getter, js_name = "isItalic")]
    pub fn is_italic(&self) -> Option<bool> {
        self.0.is_italic()
    }

    /// The underline style, when stated.
    #[wasm_bindgen(getter, js_name = "underline")]
    pub fn underline(&self) -> Result<Option<TextUnderline>, JsValue> {
        self.0
            .underline()
            .map(TextUnderline::from_model)
            .transpose()
    }

    /// The strike-through, when stated.
    #[wasm_bindgen(getter, js_name = "strike")]
    pub fn strike(&self) -> Result<Option<TextStrike>, JsValue> {
        self.0.strike().map(TextStrike::from_model).transpose()
    }

    /// The capitalisation, when stated.
    #[wasm_bindgen(getter, js_name = "capitalization")]
    pub fn capitalization(&self) -> Result<Option<TextCapitalization>, JsValue> {
        self.0
            .capitalization()
            .map(TextCapitalization::from_model)
            .transpose()
    }

    /// The letter spacing in points, when stated.
    #[wasm_bindgen(getter, js_name = "spacingPoints")]
    pub fn spacing_points(&self) -> Option<f64> {
        self.0.spacing_points()
    }

    /// The kerning threshold in points, when stated.
    #[wasm_bindgen(getter, js_name = "kerningPoints")]
    pub fn kerning_points(&self) -> Option<f64> {
        self.0.kerning_points()
    }

    /// The baseline offset, when stated.
    #[wasm_bindgen(getter, js_name = "baseline")]
    pub fn baseline(&self) -> Option<Fraction> {
        self.0.baseline().map(Fraction)
    }

    /// The language tag, when stated.
    #[wasm_bindgen(getter, js_name = "language")]
    pub fn language(&self) -> Option<String> {
        self.0.language().map(str::to_owned)
    }

    /// The fill, when stated.
    #[wasm_bindgen(getter, js_name = "fill")]
    pub fn fill(&self) -> Option<FillSpec> {
        self.0.fill().cloned().map(FillSpec)
    }

    /// The text outline, when stated.
    #[wasm_bindgen(getter, js_name = "outline")]
    pub fn outline(&self) -> Option<LineSpec> {
        self.0.outline().cloned().map(LineSpec)
    }

    /// The text effects, when stated.
    #[wasm_bindgen(getter, js_name = "effects")]
    pub fn effects(&self) -> Option<EffectListSpec> {
        self.0.effects().cloned().map(EffectListSpec)
    }

    /// The highlight colour, when stated.
    #[wasm_bindgen(getter, js_name = "highlight")]
    pub fn highlight(&self) -> Option<ColorSpec> {
        self.0.highlight().cloned().map(ColorSpec)
    }

    /// The underline line style, when stated.
    #[wasm_bindgen(getter, js_name = "underlineLine")]
    pub fn underline_line(&self) -> Option<UnderlineLine> {
        self.0.underline_line().cloned().map(UnderlineLine)
    }

    /// The underline fill, when stated.
    #[wasm_bindgen(getter, js_name = "underlineFill")]
    pub fn underline_fill(&self) -> Option<UnderlineFill> {
        self.0.underline_fill().cloned().map(UnderlineFill)
    }

    /// The font for one script slot, when stated.
    pub fn font(&self, slot: FontSlot) -> Option<TextFont> {
        self.0.font(slot.into()).cloned().map(TextFont)
    }

    /// This specification laid over `lower`: whatever this one states wins, and whatever it leaves
    /// unstated comes from `lower`. The same walk the `effective_…` readers make, one rung at a
    /// time.
    #[wasm_bindgen(js_name = "mergeUnder")]
    pub fn merge_under(&self, lower: &Self) -> Self {
        Self(self.0.clone().merge_under(&lower.0))
    }
}

#[wasm_bindgen]
impl ParagraphPropertiesSpec {
    /// A specification that states nothing. Everything it does not state is inherited.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self(ooxml::ParagraphPropertiesSpec::new())
    }

    /// This specification at the given list level.
    #[wasm_bindgen(js_name = "withLevel")]
    pub fn with_level(&self, level: &IndentLevel) -> Self {
        Self(self.0.clone().with_level(level.0))
    }

    /// This specification with the given alignment.
    #[wasm_bindgen(js_name = "withAlignment")]
    pub fn with_alignment(&self, alignment: TextAlignment) -> Self {
        Self(self.0.clone().with_alignment(alignment.into()))
    }

    /// This specification with the given left margin, in points.
    #[wasm_bindgen(js_name = "withLeftMarginPoints")]
    pub fn with_left_margin_points(&self, points: f64) -> Self {
        Self(self.0.clone().with_left_margin_points(points))
    }

    /// This specification with the given right margin, in points.
    #[wasm_bindgen(js_name = "withRightMarginPoints")]
    pub fn with_right_margin_points(&self, points: f64) -> Self {
        Self(self.0.clone().with_right_margin_points(points))
    }

    /// This specification with the given first-line indent, in points.
    #[wasm_bindgen(js_name = "withIndentPoints")]
    pub fn with_indent_points(&self, points: f64) -> Self {
        Self(self.0.clone().with_indent_points(points))
    }

    /// This specification with the given default tab size, in points.
    #[wasm_bindgen(js_name = "withDefaultTabSizePoints")]
    pub fn with_default_tab_size_points(&self, points: f64) -> Self {
        Self(self.0.clone().with_default_tab_size_points(points))
    }

    /// This specification, right-to-left or not.
    #[wasm_bindgen(js_name = "withRightToLeft")]
    pub fn with_right_to_left(&self, right_to_left: bool) -> Self {
        Self(self.0.clone().with_right_to_left(right_to_left))
    }

    /// This specification with the given font alignment within the line box.
    #[wasm_bindgen(js_name = "withFontAlignment")]
    pub fn with_font_alignment(&self, font_alignment: FontAlignment) -> Self {
        Self(self.0.clone().with_font_alignment(font_alignment.into()))
    }

    /// This specification with the given line spacing.
    #[wasm_bindgen(js_name = "withLineSpacing")]
    pub fn with_line_spacing(&self, spacing: &TextSpacing) -> Self {
        Self(self.0.clone().with_line_spacing(spacing.0))
    }

    /// This specification with the given space before the paragraph.
    #[wasm_bindgen(js_name = "withSpaceBefore")]
    pub fn with_space_before(&self, spacing: &TextSpacing) -> Self {
        Self(self.0.clone().with_space_before(spacing.0))
    }

    /// This specification with the given space after the paragraph.
    #[wasm_bindgen(js_name = "withSpaceAfter")]
    pub fn with_space_after(&self, spacing: &TextSpacing) -> Self {
        Self(self.0.clone().with_space_after(spacing.0))
    }

    /// This specification with the given bullet.
    #[wasm_bindgen(js_name = "withBullet")]
    pub fn with_bullet(&self, bullet: &Bullet) -> Self {
        Self(self.0.clone().with_bullet(bullet.0.clone()))
    }

    /// This specification bulleted with the given character.
    #[wasm_bindgen(js_name = "withBulletCharacter")]
    pub fn with_bullet_character(&self, character: &str) -> Self {
        Self(self.0.clone().with_bullet_character(character))
    }

    /// This specification with no bullet — `a:buNone`, which turns off an inherited one.
    #[wasm_bindgen(js_name = "withoutBullet")]
    pub fn without_bullet(&self) -> Self {
        Self(self.0.clone().without_bullet())
    }

    /// This specification with the given bullet colour.
    #[wasm_bindgen(js_name = "withBulletColor")]
    pub fn with_bullet_color(&self, color: &BulletColor) -> Self {
        Self(self.0.clone().with_bullet_color(color.0.clone()))
    }

    /// This specification with the given bullet size.
    #[wasm_bindgen(js_name = "withBulletSize")]
    pub fn with_bullet_size(&self, size: &BulletSize) -> Self {
        Self(self.0.clone().with_bullet_size(size.0))
    }

    /// This specification with the given bullet typeface.
    #[wasm_bindgen(js_name = "withBulletTypeface")]
    pub fn with_bullet_typeface(&self, typeface: &BulletTypeface) -> Self {
        Self(self.0.clone().with_bullet_typeface(typeface.0.clone()))
    }

    /// This specification with the given tab stops, replacing any it already had.
    #[wasm_bindgen(js_name = "withTabStops")]
    pub fn with_tab_stops(&self, stops: Vec<TabStop>) -> Self {
        Self(
            self.0
                .clone()
                .with_tab_stops(stops.into_iter().map(|stop| stop.0).collect()),
        )
    }

    /// This specification with the given default run properties — what the paragraph's own text
    /// inherits before any run states anything.
    #[wasm_bindgen(js_name = "withDefaultRunProperties")]
    pub fn with_default_run_properties(&self, properties: &CharacterPropertiesSpec) -> Self {
        Self(
            self.0
                .clone()
                .with_default_run_properties(properties.0.clone()),
        )
    }

    /// The list level, when stated.
    #[wasm_bindgen(getter, js_name = "level")]
    pub fn level(&self) -> Option<IndentLevel> {
        self.0.level().map(IndentLevel)
    }

    /// The alignment, when stated.
    #[wasm_bindgen(getter, js_name = "alignment")]
    pub fn alignment(&self) -> Result<Option<TextAlignment>, JsValue> {
        self.0
            .alignment()
            .map(TextAlignment::from_model)
            .transpose()
    }

    /// The left margin in points, when stated.
    #[wasm_bindgen(getter, js_name = "leftMarginPoints")]
    pub fn left_margin_points(&self) -> Option<f64> {
        self.0.left_margin_points()
    }

    /// The right margin in points, when stated.
    #[wasm_bindgen(getter, js_name = "rightMarginPoints")]
    pub fn right_margin_points(&self) -> Option<f64> {
        self.0.right_margin_points()
    }

    /// The first-line indent in points, when stated.
    #[wasm_bindgen(getter, js_name = "indentPoints")]
    pub fn indent_points(&self) -> Option<f64> {
        self.0.indent_points()
    }

    /// The default tab size in points, when stated.
    #[wasm_bindgen(getter, js_name = "defaultTabSizePoints")]
    pub fn default_tab_size_points(&self) -> Option<f64> {
        self.0.default_tab_size_points()
    }

    /// Whether right-to-left is stated, and what it says.
    #[wasm_bindgen(getter, js_name = "isRightToLeft")]
    pub fn is_right_to_left(&self) -> Option<bool> {
        self.0.is_right_to_left()
    }

    /// The font alignment, when stated.
    #[wasm_bindgen(getter, js_name = "fontAlignment")]
    pub fn font_alignment(&self) -> Result<Option<FontAlignment>, JsValue> {
        self.0
            .font_alignment()
            .map(FontAlignment::from_model)
            .transpose()
    }

    /// The line spacing, when stated.
    #[wasm_bindgen(getter, js_name = "lineSpacing")]
    pub fn line_spacing(&self) -> Option<TextSpacing> {
        self.0.line_spacing().map(TextSpacing)
    }

    /// The space before the paragraph, when stated.
    #[wasm_bindgen(getter, js_name = "spaceBefore")]
    pub fn space_before(&self) -> Option<TextSpacing> {
        self.0.space_before().map(TextSpacing)
    }

    /// The space after the paragraph, when stated.
    #[wasm_bindgen(getter, js_name = "spaceAfter")]
    pub fn space_after(&self) -> Option<TextSpacing> {
        self.0.space_after().map(TextSpacing)
    }

    /// The bullet, when stated.
    #[wasm_bindgen(getter, js_name = "bullet")]
    pub fn bullet(&self) -> Option<Bullet> {
        self.0.bullet().cloned().map(Bullet)
    }

    /// The bullet colour, when stated.
    #[wasm_bindgen(getter, js_name = "bulletColor")]
    pub fn bullet_color(&self) -> Option<BulletColor> {
        self.0.bullet_color().cloned().map(BulletColor)
    }

    /// The bullet size, when stated.
    #[wasm_bindgen(getter, js_name = "bulletSize")]
    pub fn bullet_size(&self) -> Option<BulletSize> {
        self.0.bullet_size().map(BulletSize)
    }

    /// The bullet typeface, when stated.
    #[wasm_bindgen(getter, js_name = "bulletTypeface")]
    pub fn bullet_typeface(&self) -> Option<BulletTypeface> {
        self.0.bullet_typeface().cloned().map(BulletTypeface)
    }

    /// The tab stops, in order.
    #[wasm_bindgen(getter, js_name = "tabStops")]
    pub fn tab_stops(&self) -> Vec<TabStop> {
        self.0.tab_stops().iter().copied().map(TabStop).collect()
    }

    /// The default run properties, when stated.
    #[wasm_bindgen(getter, js_name = "defaultRunProperties")]
    pub fn default_run_properties(&self) -> Option<CharacterPropertiesSpec> {
        self.0
            .default_run_properties()
            .cloned()
            .map(CharacterPropertiesSpec)
    }

    /// This specification laid over `lower`: whatever this one states wins.
    #[wasm_bindgen(js_name = "mergeUnder")]
    pub fn merge_under(&self, lower: &Self) -> Self {
        Self(self.0.clone().merge_under(&lower.0))
    }
}

impl Default for CharacterPropertiesSpec {
    /// The same value the no-argument constructor builds.
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ParagraphPropertiesSpec {
    /// The same value the no-argument constructor builds.
    fn default() -> Self {
        Self::new()
    }
}
