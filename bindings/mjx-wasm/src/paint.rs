//! Colour, fill, line and effect — everything that decides what a shape looks like.
//!
//! Two shapes of class live here, and the difference is not arbitrary:
//!
//! * The Rust types that are **enumerations with payloads** — `ColorSpec`, `FillSpec`, `LineDash`,
//!   `LineJoin` — become classes with static constructors, one per variant, and a `kind` property
//!   that says which one a value is. A TypeScript `enum` cannot carry per-member data, and a plain
//!   object would give up every check.
//! * The Rust types that are **structs with public fields** — `LineSpec`, `LineEnd`, the eight
//!   effects — become classes with a constructor in field order (trailing arguments optional) and
//!   read-only getters, plus the same `with…` methods the Rust builders have. Both spellings work,
//!   exactly as they do one layer down.

use wasm_bindgen::prelude::*;

use mjx_ooxml as ooxml;

use crate::enums::{
    BlendMode, ColorKind, ColorSchemeSlot, CompoundLine, LineCap, LineEndLength, LineEndType,
    LineEndWidth, PatternType, PenAlignment, PictureFillMode, PresetLineDash, PresetShadow,
    RectangleAlignment, SchemeColor,
};
use crate::measures::{Angle, Emu, Fraction, LineWidth};

value_class! {
    /// A colour, as the document states it: six hex digits, a theme slot, or one of the other
    /// colour elements DrawingML defines.
    ColorSpec(ooxml::ColorSpec), derive(PartialEq);

    /// One stop on a gradient: where it sits, and what colour it is there.
    GradientStopSpec(ooxml::GradientStopSpec), derive(PartialEq);

    /// How a shape, cell, run or chart element is filled.
    FillSpec(ooxml::FillSpec), derive(PartialEq);

    /// The dash pattern of a line: one of the eleven presets, or a custom pattern the document
    /// spells out (which this build preserves but does not model).
    LineDash(ooxml::LineDash), derive(PartialEq);

    /// How two segments of a line meet.
    LineJoin(ooxml::LineJoin), derive(PartialEq);

    /// The head or tail decoration of a line — an arrowhead, and how big it is.
    LineEnd(ooxml::LineEnd), derive(PartialEq);

    /// An outline: width, cap, dash, join, ends, and the fill that paints it.
    LineSpec(ooxml::LineSpec), derive(PartialEq);

    /// A Gaussian blur over whatever is behind it.
    BlurEffect(ooxml::BlurEffect), derive(PartialEq);

    /// A fill painted over the shape in a blend mode.
    FillOverlayEffect(ooxml::FillOverlayEffect), derive(PartialEq);

    /// A coloured halo outside the shape's edge.
    GlowEffect(ooxml::GlowEffect), derive(PartialEq);

    /// A shadow cast inside the shape's edge.
    InnerShadowEffect(ooxml::InnerShadowEffect), derive(PartialEq);

    /// A shadow cast outside the shape's edge, with its own scale, skew and alignment.
    OuterShadowEffect(ooxml::OuterShadowEffect), derive(PartialEq);

    /// One of the twenty shadows the specification names, in a colour of your choosing.
    PresetShadowEffect(ooxml::PresetShadowEffect), derive(PartialEq);

    /// A mirrored, fading copy of the shape below it.
    ReflectionEffect(ooxml::ReflectionEffect), derive(PartialEq);

    /// A feathered edge that fades the shape out over a radius.
    SoftEdgeEffect(ooxml::SoftEdgeEffect), derive(PartialEq);

    /// The eight effects a shape can carry, in the order the markup writes them.
    EffectListSpec(ooxml::EffectListSpec), derive(PartialEq);

    /// A theme's twelve-slot colour mapping: which scheme colour each named slot resolves to.
    ColorMap(ooxml::ColorMap), derive(PartialEq);

    /// A colour resolved all the way to channels — what a renderer would actually paint.
    ResolvedColor(ooxml::ResolvedColor), derive(Copy, PartialEq);
}

// ---------------------------------------------------------------------------------------------
// Colour
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl ColorSpec {
    /// A literal colour, six hexadecimal digits with no leading `#`: `ColorSpec.srgb("1F3864")`.
    pub fn srgb(hex: &str) -> Self {
        Self(ooxml::ColorSpec::Srgb(hex.to_owned()))
    }

    /// A theme colour, resolved through the surface's colour map at render time.
    pub fn scheme(color: SchemeColor) -> Self {
        Self(ooxml::ColorSpec::Scheme(color.into()))
    }

    /// One of the other colour elements — `hslClr`, `scrgbClr`, `sysClr`, `prstClr` — kept exactly
    /// as written so it round-trips, and reported here so a caller knows what it is looking at.
    pub fn other(kind: ColorKind, value: Option<String>) -> Self {
        Self(ooxml::ColorSpec::Other {
            kind: kind.into(),
            value,
        })
    }

    /// Which kind of colour element this is.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<ColorKind, JsValue> {
        ColorKind::from_model(match &self.0 {
            ooxml::ColorSpec::Srgb(_) => ooxml::ColorKind::Srgb,
            ooxml::ColorSpec::Scheme(_) => ooxml::ColorKind::Scheme,
            ooxml::ColorSpec::Other { kind, .. } => *kind,
        })
    }

    /// The six hex digits, when this is a literal colour.
    #[wasm_bindgen(getter, js_name = "srgbValue")]
    pub fn srgb_value(&self) -> Option<String> {
        match &self.0 {
            ooxml::ColorSpec::Srgb(hex) => Some(hex.clone()),
            _ => None,
        }
    }

    /// The theme slot, when this is a theme colour.
    #[wasm_bindgen(getter, js_name = "schemeColor")]
    pub fn scheme_color(&self) -> Result<Option<SchemeColor>, JsValue> {
        match &self.0 {
            ooxml::ColorSpec::Scheme(color) => SchemeColor::from_model(*color).map(Some),
            _ => Ok(None),
        }
    }

    /// The raw value of one of the other colour elements, when the document stated one.
    #[wasm_bindgen(getter, js_name = "value")]
    pub fn value(&self) -> Option<String> {
        match &self.0 {
            ooxml::ColorSpec::Other { value, .. } => value.clone(),
            _ => None,
        }
    }
}

#[wasm_bindgen]
impl GradientStopSpec {
    /// A stop at `position` along the gradient, painted `color`.
    #[wasm_bindgen(constructor)]
    pub fn new(position: &Fraction, color: &ColorSpec) -> Self {
        Self(ooxml::GradientStopSpec {
            position: position.0,
            color: color.0.clone(),
        })
    }

    /// Where the stop sits, as a proportion of the gradient's length.
    #[wasm_bindgen(getter, js_name = "position")]
    pub fn position(&self) -> Fraction {
        Fraction(self.0.position)
    }

    /// The colour at this stop.
    #[wasm_bindgen(getter, js_name = "color")]
    pub fn color(&self) -> ColorSpec {
        ColorSpec(self.0.color.clone())
    }
}

#[wasm_bindgen]
impl ColorMap {
    /// The mapping that sends every slot to itself — what a theme means when it states no `clrMap`.
    pub fn identity() -> Self {
        Self(ooxml::ColorMap::identity())
    }

    /// Which scheme colour a named slot resolves to, or `None` for a colour that is not mapped.
    pub fn resolve(&self, color: SchemeColor) -> Result<Option<ColorSchemeSlot>, JsValue> {
        match self.0.resolve(color.into()) {
            Some(slot) => ColorSchemeSlot::from_model(slot).map(Some),
            None => Ok(None),
        }
    }

    /// The slot `bg1` maps to.
    #[wasm_bindgen(getter, js_name = "background1")]
    pub fn background1(&self) -> Result<ColorSchemeSlot, JsValue> {
        ColorSchemeSlot::from_model(self.0.background1)
    }

    /// The slot `tx1` maps to.
    #[wasm_bindgen(getter, js_name = "text1")]
    pub fn text1(&self) -> Result<ColorSchemeSlot, JsValue> {
        ColorSchemeSlot::from_model(self.0.text1)
    }

    /// The slot `bg2` maps to.
    #[wasm_bindgen(getter, js_name = "background2")]
    pub fn background2(&self) -> Result<ColorSchemeSlot, JsValue> {
        ColorSchemeSlot::from_model(self.0.background2)
    }

    /// The slot `tx2` maps to.
    #[wasm_bindgen(getter, js_name = "text2")]
    pub fn text2(&self) -> Result<ColorSchemeSlot, JsValue> {
        ColorSchemeSlot::from_model(self.0.text2)
    }

    /// The six accent slots, in order.
    #[wasm_bindgen(getter, js_name = "accents")]
    pub fn accents(&self) -> Result<Vec<ColorSchemeSlot>, JsValue> {
        [
            self.0.accent1,
            self.0.accent2,
            self.0.accent3,
            self.0.accent4,
            self.0.accent5,
            self.0.accent6,
        ]
        .into_iter()
        .map(ColorSchemeSlot::from_model)
        .collect()
    }

    /// The slot `hlink` maps to.
    #[wasm_bindgen(getter, js_name = "hyperlink")]
    pub fn hyperlink(&self) -> Result<ColorSchemeSlot, JsValue> {
        ColorSchemeSlot::from_model(self.0.hyperlink)
    }

    /// The slot `folHlink` maps to.
    #[wasm_bindgen(getter, js_name = "followedHyperlink")]
    pub fn followed_hyperlink(&self) -> Result<ColorSchemeSlot, JsValue> {
        ColorSchemeSlot::from_model(self.0.followed_hyperlink)
    }
}

#[wasm_bindgen]
impl ResolvedColor {
    /// The red channel, `0`–`255`.
    #[wasm_bindgen(getter, js_name = "red")]
    pub fn red(&self) -> u8 {
        self.0.red
    }

    /// The green channel, `0`–`255`.
    #[wasm_bindgen(getter, js_name = "green")]
    pub fn green(&self) -> u8 {
        self.0.green
    }

    /// The blue channel, `0`–`255`.
    #[wasm_bindgen(getter, js_name = "blue")]
    pub fn blue(&self) -> u8 {
        self.0.blue
    }

    /// The alpha channel as a proportion of one, `1.0` being fully opaque.
    #[wasm_bindgen(getter, js_name = "alpha")]
    pub fn alpha(&self) -> f64 {
        self.0.alpha
    }

    /// The colour as six hexadecimal digits.
    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> String {
        self.0.to_hex().to_owned()
    }
}

// ---------------------------------------------------------------------------------------------
// Fill
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl FillSpec {
    /// No fill at all — `a:noFill`, which is not the same as stating nothing.
    pub fn none() -> Self {
        Self(ooxml::FillSpec::None)
    }

    /// One flat colour.
    pub fn solid(color: &ColorSpec) -> Self {
        Self(ooxml::FillSpec::Solid(color.0.clone()))
    }

    /// A linear gradient through the given stops, at the given angle.
    pub fn gradient(stops: Vec<GradientStopSpec>, angle: Option<Angle>) -> Self {
        Self(ooxml::FillSpec::Gradient {
            stops: stops.into_iter().map(|stop| stop.0).collect(),
            angle: angle.map(|angle| angle.0),
        })
    }

    /// An image, named by the relationship id `Deck.add_image` hands back.
    pub fn picture(rel_id: &str, mode: PictureFillMode) -> Self {
        Self(ooxml::FillSpec::Picture {
            rel_id: rel_id.to_owned(),
            mode: mode.into(),
        })
    }

    /// One of the fifty-four hatch patterns, in a foreground and background colour.
    pub fn pattern(
        preset: Option<PatternType>,
        foreground: Option<ColorSpec>,
        background: Option<ColorSpec>,
    ) -> Self {
        Self(ooxml::FillSpec::Pattern {
            preset: preset.map(Into::into),
            foreground: foreground.map(|color| color.0),
            background: background.map(|color| color.0),
        })
    }

    /// Inherit the enclosing group's fill — `a:grpFill`.
    pub fn group() -> Self {
        Self(ooxml::FillSpec::Group)
    }

    /// Which kind of fill this is: `"none"`, `"solid"`, `"gradient"`, `"picture"`, `"pattern"` or
    /// `"group"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        match &self.0 {
            ooxml::FillSpec::None => "none".to_owned(),
            ooxml::FillSpec::Solid(_) => "solid".to_owned(),
            ooxml::FillSpec::Gradient { .. } => "gradient".to_owned(),
            ooxml::FillSpec::Picture { .. } => "picture".to_owned(),
            ooxml::FillSpec::Pattern { .. } => "pattern".to_owned(),
            ooxml::FillSpec::Group => "group".to_owned(),
        }
    }

    /// The colour, when this is a solid fill.
    #[wasm_bindgen(getter, js_name = "color")]
    pub fn color(&self) -> Option<ColorSpec> {
        match &self.0 {
            ooxml::FillSpec::Solid(color) => Some(ColorSpec(color.clone())),
            _ => None,
        }
    }

    /// The stops, when this is a gradient; an empty list otherwise.
    #[wasm_bindgen(getter, js_name = "stops")]
    pub fn stops(&self) -> Vec<GradientStopSpec> {
        match &self.0 {
            ooxml::FillSpec::Gradient { stops, .. } => {
                stops.iter().cloned().map(GradientStopSpec).collect()
            }
            _ => Vec::new(),
        }
    }

    /// The gradient's angle, when it states one.
    #[wasm_bindgen(getter, js_name = "angle")]
    pub fn angle(&self) -> Option<Angle> {
        match &self.0 {
            ooxml::FillSpec::Gradient { angle, .. } => angle.map(Angle),
            _ => None,
        }
    }

    /// The image relationship id, when this is a picture fill.
    #[wasm_bindgen(getter, js_name = "relId")]
    pub fn rel_id(&self) -> Option<String> {
        match &self.0 {
            ooxml::FillSpec::Picture { rel_id, .. } => Some(rel_id.clone()),
            _ => None,
        }
    }

    /// How the image is laid into the shape, when this is a picture fill.
    #[wasm_bindgen(getter, js_name = "pictureMode")]
    pub fn picture_mode(&self) -> Result<Option<PictureFillMode>, JsValue> {
        match &self.0 {
            ooxml::FillSpec::Picture { mode, .. } => PictureFillMode::from_model(*mode).map(Some),
            _ => Ok(None),
        }
    }

    /// The hatch pattern, when this is a pattern fill and it names one.
    #[wasm_bindgen(getter, js_name = "patternPreset")]
    pub fn pattern_preset(&self) -> Result<Option<PatternType>, JsValue> {
        match &self.0 {
            ooxml::FillSpec::Pattern {
                preset: Some(preset),
                ..
            } => PatternType::from_model(*preset).map(Some),
            _ => Ok(None),
        }
    }

    /// The pattern's foreground colour, when it states one.
    #[wasm_bindgen(getter, js_name = "foreground")]
    pub fn foreground(&self) -> Option<ColorSpec> {
        match &self.0 {
            ooxml::FillSpec::Pattern { foreground, .. } => foreground.clone().map(ColorSpec),
            _ => None,
        }
    }

    /// The pattern's background colour, when it states one.
    #[wasm_bindgen(getter, js_name = "background")]
    pub fn background(&self) -> Option<ColorSpec> {
        match &self.0 {
            ooxml::FillSpec::Pattern { background, .. } => background.clone().map(ColorSpec),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Line
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl LineDash {
    /// One of the eleven dash patterns the specification names.
    pub fn preset(dash: PresetLineDash) -> Self {
        Self(ooxml::LineDash::Preset(dash.into()))
    }

    /// A custom dash pattern. The document's own `a:custDash` stops are preserved on write; this
    /// build does not model the individual dash and space lengths.
    pub fn custom() -> Self {
        Self(ooxml::LineDash::Custom)
    }

    /// The named pattern, when this is a preset.
    #[wasm_bindgen(getter, js_name = "presetDash")]
    pub fn preset_dash(&self) -> Result<Option<PresetLineDash>, JsValue> {
        match &self.0 {
            ooxml::LineDash::Preset(dash) => PresetLineDash::from_model(*dash).map(Some),
            ooxml::LineDash::Custom => Ok(None),
        }
    }

    /// Whether the document spelled the pattern out rather than naming one.
    #[wasm_bindgen(getter, js_name = "isCustom")]
    pub fn is_custom(&self) -> bool {
        matches!(self.0, ooxml::LineDash::Custom)
    }
}

#[wasm_bindgen]
impl LineJoin {
    /// A rounded corner.
    pub fn round() -> Self {
        Self(ooxml::LineJoin::Round)
    }

    /// A flattened corner.
    pub fn bevel() -> Self {
        Self(ooxml::LineJoin::Bevel)
    }

    /// A pointed corner, optionally limited so that a very sharp angle does not run away.
    pub fn miter(limit: Option<Fraction>) -> Self {
        Self(ooxml::LineJoin::Miter {
            limit: limit.map(|limit| limit.0),
        })
    }

    /// Which join this is: `"round"`, `"bevel"` or `"miter"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        match &self.0 {
            ooxml::LineJoin::Round => "round".to_owned(),
            ooxml::LineJoin::Bevel => "bevel".to_owned(),
            ooxml::LineJoin::Miter { .. } => "miter".to_owned(),
        }
    }

    /// The mitre limit, when this is a mitre join that states one.
    #[wasm_bindgen(getter, js_name = "miterLimit")]
    pub fn miter_limit(&self) -> Option<Fraction> {
        match &self.0 {
            ooxml::LineJoin::Miter { limit } => limit.map(Fraction),
            _ => None,
        }
    }
}

#[wasm_bindgen]
impl LineEnd {
    /// An end decoration: which arrowhead, how wide, how long. Every part is optional, and an
    /// unstated one is inherited.
    #[wasm_bindgen(constructor)]
    pub fn new(
        kind: Option<LineEndType>,
        width: Option<LineEndWidth>,
        length: Option<LineEndLength>,
    ) -> Self {
        Self(ooxml::LineEnd {
            kind: kind.map(Into::into),
            width: width.map(Into::into),
            length: length.map(Into::into),
        })
    }

    /// Which arrowhead, when the line states one.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<Option<LineEndType>, JsValue> {
        self.0.kind.map(LineEndType::from_model).transpose()
    }

    /// How wide the arrowhead is, when the line states it.
    #[wasm_bindgen(getter, js_name = "width")]
    pub fn width(&self) -> Result<Option<LineEndWidth>, JsValue> {
        self.0.width.map(LineEndWidth::from_model).transpose()
    }

    /// How long the arrowhead is, when the line states it.
    #[wasm_bindgen(getter, js_name = "length")]
    pub fn length(&self) -> Result<Option<LineEndLength>, JsValue> {
        self.0.length.map(LineEndLength::from_model).transpose()
    }
}

#[wasm_bindgen]
impl LineSpec {
    /// An outline that states nothing. Add to it with the `with_…` methods.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self(ooxml::LineSpec::new())
    }

    /// The common case: a solid line of one width and one colour.
    pub fn solid(width: &LineWidth, color: &ColorSpec) -> Self {
        Self(ooxml::LineSpec::solid(width.0, color.0.clone()))
    }

    /// This outline with the given width.
    #[wasm_bindgen(js_name = "withWidth")]
    pub fn with_width(&self, width: &LineWidth) -> Self {
        let mut line = self.0.clone();
        line.width = Some(width.0);
        Self(line)
    }

    /// This outline with the given end cap.
    #[wasm_bindgen(js_name = "withCap")]
    pub fn with_cap(&self, cap: LineCap) -> Self {
        let mut line = self.0.clone();
        line.cap = Some(cap.into());
        Self(line)
    }

    /// This outline drawn as a compound (double, triple, thick-thin) line.
    #[wasm_bindgen(js_name = "withCompound")]
    pub fn with_compound(&self, compound: CompoundLine) -> Self {
        let mut line = self.0.clone();
        line.compound = Some(compound.into());
        Self(line)
    }

    /// This outline centred on, or inset from, the shape's edge.
    #[wasm_bindgen(js_name = "withPenAlignment")]
    pub fn with_pen_alignment(&self, alignment: PenAlignment) -> Self {
        let mut line = self.0.clone();
        line.pen_alignment = Some(alignment.into());
        Self(line)
    }

    /// This outline painted with the given fill — which is how a line gets a gradient.
    #[wasm_bindgen(js_name = "withFill")]
    pub fn with_fill(&self, fill: &FillSpec) -> Self {
        let mut line = self.0.clone();
        line.fill = Some(fill.0.clone());
        Self(line)
    }

    /// This outline with the given dash pattern.
    #[wasm_bindgen(js_name = "withDash")]
    pub fn with_dash(&self, dash: &LineDash) -> Self {
        let mut line = self.0.clone();
        line.dash = Some(dash.0);
        Self(line)
    }

    /// This outline with the given corner treatment.
    #[wasm_bindgen(js_name = "withJoin")]
    pub fn with_join(&self, join: &LineJoin) -> Self {
        let mut line = self.0.clone();
        line.join = Some(join.0);
        Self(line)
    }

    /// This outline with the given decoration at its start.
    #[wasm_bindgen(js_name = "withHeadEnd")]
    pub fn with_head_end(&self, end: &LineEnd) -> Self {
        let mut line = self.0.clone();
        line.head_end = Some(end.0);
        Self(line)
    }

    /// This outline with the given decoration at its end.
    #[wasm_bindgen(js_name = "withTailEnd")]
    pub fn with_tail_end(&self, end: &LineEnd) -> Self {
        let mut line = self.0.clone();
        line.tail_end = Some(end.0);
        Self(line)
    }

    /// The width, when stated.
    #[wasm_bindgen(getter, js_name = "width")]
    pub fn width(&self) -> Option<LineWidth> {
        self.0.width.map(LineWidth)
    }

    /// The end cap, when stated.
    #[wasm_bindgen(getter, js_name = "cap")]
    pub fn cap(&self) -> Result<Option<LineCap>, JsValue> {
        self.0.cap.map(LineCap::from_model).transpose()
    }

    /// The compound style, when stated.
    #[wasm_bindgen(getter, js_name = "compound")]
    pub fn compound(&self) -> Result<Option<CompoundLine>, JsValue> {
        self.0.compound.map(CompoundLine::from_model).transpose()
    }

    /// The pen alignment, when stated.
    #[wasm_bindgen(getter, js_name = "penAlignment")]
    pub fn pen_alignment(&self) -> Result<Option<PenAlignment>, JsValue> {
        self.0
            .pen_alignment
            .map(PenAlignment::from_model)
            .transpose()
    }

    /// The fill that paints the line, when stated.
    #[wasm_bindgen(getter, js_name = "fill")]
    pub fn fill(&self) -> Option<FillSpec> {
        self.0.fill.clone().map(FillSpec)
    }

    /// The dash pattern, when stated.
    #[wasm_bindgen(getter, js_name = "dash")]
    pub fn dash(&self) -> Option<LineDash> {
        self.0.dash.map(LineDash)
    }

    /// The corner treatment, when stated.
    #[wasm_bindgen(getter, js_name = "join")]
    pub fn join(&self) -> Option<LineJoin> {
        self.0.join.map(LineJoin)
    }

    /// The start decoration, when stated.
    #[wasm_bindgen(getter, js_name = "headEnd")]
    pub fn head_end(&self) -> Option<LineEnd> {
        self.0.head_end.map(LineEnd)
    }

    /// The end decoration, when stated.
    #[wasm_bindgen(getter, js_name = "tailEnd")]
    pub fn tail_end(&self) -> Option<LineEnd> {
        self.0.tail_end.map(LineEnd)
    }
}

// ---------------------------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl BlurEffect {
    /// A blur of the given radius. `grow` says whether the blurred edge may extend past the
    /// shape's bounds.
    #[wasm_bindgen(constructor)]
    pub fn new(radius: Option<Emu>, grow: Option<bool>) -> Self {
        Self(ooxml::BlurEffect {
            radius: radius.map(|radius| radius.0),
            grow,
        })
    }

    /// This blur with the given radius.
    #[wasm_bindgen(js_name = "withRadius")]
    pub fn with_radius(&self, radius: &Emu) -> Self {
        Self(self.0.with_radius(radius.0))
    }

    /// This blur, growing past the shape's bounds or not.
    #[wasm_bindgen(js_name = "withGrow")]
    pub fn with_grow(&self, grow: bool) -> Self {
        Self(self.0.with_grow(grow))
    }

    /// The radius, when stated.
    #[wasm_bindgen(getter, js_name = "radius")]
    pub fn radius(&self) -> Option<Emu> {
        self.0.radius.map(Emu)
    }

    /// Whether the blur may grow past the shape's bounds, when stated.
    #[wasm_bindgen(getter, js_name = "grow")]
    pub fn grow(&self) -> Option<bool> {
        self.0.grow
    }
}

#[wasm_bindgen]
impl FillOverlayEffect {
    /// A fill painted over the shape in the given blend mode.
    #[wasm_bindgen(constructor)]
    pub fn new(fill: &FillSpec, blend: BlendMode) -> Self {
        Self(ooxml::FillOverlayEffect::new(fill.0.clone(), blend.into()))
    }

    /// The overlaid fill.
    #[wasm_bindgen(getter, js_name = "fill")]
    pub fn fill(&self) -> FillSpec {
        FillSpec(self.0.fill.clone())
    }

    /// How it blends with what is beneath.
    #[wasm_bindgen(getter, js_name = "blend")]
    pub fn blend(&self) -> Result<BlendMode, JsValue> {
        BlendMode::from_model(self.0.blend)
    }
}

#[wasm_bindgen]
impl GlowEffect {
    /// A halo in the given colour, optionally of a given radius.
    #[wasm_bindgen(constructor)]
    pub fn new(color: &ColorSpec, radius: Option<Emu>) -> Self {
        Self(ooxml::GlowEffect {
            color: color.0.clone(),
            radius: radius.map(|radius| radius.0),
        })
    }

    /// This glow with the given radius.
    #[wasm_bindgen(js_name = "withRadius")]
    pub fn with_radius(&self, radius: &Emu) -> Self {
        Self(self.0.clone().with_radius(radius.0))
    }

    /// The glow's colour.
    #[wasm_bindgen(getter, js_name = "color")]
    pub fn color(&self) -> ColorSpec {
        ColorSpec(self.0.color.clone())
    }

    /// The radius, when stated.
    #[wasm_bindgen(getter, js_name = "radius")]
    pub fn radius(&self) -> Option<Emu> {
        self.0.radius.map(Emu)
    }
}

#[wasm_bindgen]
impl InnerShadowEffect {
    /// A shadow inside the shape's edge.
    #[wasm_bindgen(constructor)]
    pub fn new(
        color: &ColorSpec,
        blur_radius: Option<Emu>,
        distance: Option<Emu>,
        direction: Option<Angle>,
    ) -> Self {
        Self(ooxml::InnerShadowEffect {
            color: color.0.clone(),
            blur_radius: blur_radius.map(|value| value.0),
            distance: distance.map(|value| value.0),
            direction: direction.map(|value| value.0),
        })
    }

    /// This shadow with the given blur radius.
    #[wasm_bindgen(js_name = "withBlurRadius")]
    pub fn with_blur_radius(&self, blur_radius: &Emu) -> Self {
        Self(self.0.clone().with_blur_radius(blur_radius.0))
    }

    /// This shadow at the given distance from the shape.
    #[wasm_bindgen(js_name = "withDistance")]
    pub fn with_distance(&self, distance: &Emu) -> Self {
        Self(self.0.clone().with_distance(distance.0))
    }

    /// This shadow cast in the given direction.
    #[wasm_bindgen(js_name = "withDirection")]
    pub fn with_direction(&self, direction: &Angle) -> Self {
        Self(self.0.clone().with_direction(direction.0))
    }

    /// The shadow's colour.
    #[wasm_bindgen(getter, js_name = "color")]
    pub fn color(&self) -> ColorSpec {
        ColorSpec(self.0.color.clone())
    }

    /// The blur radius, when stated.
    #[wasm_bindgen(getter, js_name = "blurRadius")]
    pub fn blur_radius(&self) -> Option<Emu> {
        self.0.blur_radius.map(Emu)
    }

    /// The distance from the shape, when stated.
    #[wasm_bindgen(getter, js_name = "distance")]
    pub fn distance(&self) -> Option<Emu> {
        self.0.distance.map(Emu)
    }

    /// The direction the shadow is cast in, when stated.
    #[wasm_bindgen(getter, js_name = "direction")]
    pub fn direction(&self) -> Option<Angle> {
        self.0.direction.map(Angle)
    }
}

#[wasm_bindgen]
impl OuterShadowEffect {
    /// A shadow outside the shape's edge, with its own scale, skew and alignment.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        color: &ColorSpec,
        blur_radius: Option<Emu>,
        distance: Option<Emu>,
        direction: Option<Angle>,
        scale_x: Option<Fraction>,
        scale_y: Option<Fraction>,
        skew_x: Option<Angle>,
        skew_y: Option<Angle>,
        alignment: Option<RectangleAlignment>,
        rotate_with_shape: Option<bool>,
    ) -> Self {
        Self(ooxml::OuterShadowEffect {
            color: color.0.clone(),
            blur_radius: blur_radius.map(|value| value.0),
            distance: distance.map(|value| value.0),
            direction: direction.map(|value| value.0),
            scale_x: scale_x.map(|value| value.0),
            scale_y: scale_y.map(|value| value.0),
            skew_x: skew_x.map(|value| value.0),
            skew_y: skew_y.map(|value| value.0),
            alignment: alignment.map(Into::into),
            rotate_with_shape,
        })
    }

    /// This shadow with the given blur radius.
    #[wasm_bindgen(js_name = "withBlurRadius")]
    pub fn with_blur_radius(&self, blur_radius: &Emu) -> Self {
        Self(self.0.clone().with_blur_radius(blur_radius.0))
    }

    /// This shadow at the given distance from the shape.
    #[wasm_bindgen(js_name = "withDistance")]
    pub fn with_distance(&self, distance: &Emu) -> Self {
        Self(self.0.clone().with_distance(distance.0))
    }

    /// This shadow cast in the given direction.
    #[wasm_bindgen(js_name = "withDirection")]
    pub fn with_direction(&self, direction: &Angle) -> Self {
        Self(self.0.clone().with_direction(direction.0))
    }

    /// This shadow scaled horizontally.
    #[wasm_bindgen(js_name = "withScaleX")]
    pub fn with_scale_x(&self, scale_x: &Fraction) -> Self {
        Self(self.0.clone().with_scale_x(scale_x.0))
    }

    /// This shadow scaled vertically.
    #[wasm_bindgen(js_name = "withScaleY")]
    pub fn with_scale_y(&self, scale_y: &Fraction) -> Self {
        Self(self.0.clone().with_scale_y(scale_y.0))
    }

    /// This shadow skewed horizontally.
    #[wasm_bindgen(js_name = "withSkewX")]
    pub fn with_skew_x(&self, skew_x: &Angle) -> Self {
        Self(self.0.clone().with_skew_x(skew_x.0))
    }

    /// This shadow skewed vertically.
    #[wasm_bindgen(js_name = "withSkewY")]
    pub fn with_skew_y(&self, skew_y: &Angle) -> Self {
        Self(self.0.clone().with_skew_y(skew_y.0))
    }

    /// This shadow anchored to the given corner or edge of the shape.
    #[wasm_bindgen(js_name = "withAlignment")]
    pub fn with_alignment(&self, alignment: RectangleAlignment) -> Self {
        Self(self.0.clone().with_alignment(alignment.into()))
    }

    /// This shadow rotating with the shape, or staying put.
    #[wasm_bindgen(js_name = "withRotateWithShape")]
    pub fn with_rotate_with_shape(&self, rotate_with_shape: bool) -> Self {
        Self(self.0.clone().with_rotate_with_shape(rotate_with_shape))
    }

    /// The shadow's colour.
    #[wasm_bindgen(getter, js_name = "color")]
    pub fn color(&self) -> ColorSpec {
        ColorSpec(self.0.color.clone())
    }

    /// The blur radius, when stated.
    #[wasm_bindgen(getter, js_name = "blurRadius")]
    pub fn blur_radius(&self) -> Option<Emu> {
        self.0.blur_radius.map(Emu)
    }

    /// The distance from the shape, when stated.
    #[wasm_bindgen(getter, js_name = "distance")]
    pub fn distance(&self) -> Option<Emu> {
        self.0.distance.map(Emu)
    }

    /// The direction the shadow is cast in, when stated.
    #[wasm_bindgen(getter, js_name = "direction")]
    pub fn direction(&self) -> Option<Angle> {
        self.0.direction.map(Angle)
    }

    /// The horizontal scale, when stated.
    #[wasm_bindgen(getter, js_name = "scaleX")]
    pub fn scale_x(&self) -> Option<Fraction> {
        self.0.scale_x.map(Fraction)
    }

    /// The vertical scale, when stated.
    #[wasm_bindgen(getter, js_name = "scaleY")]
    pub fn scale_y(&self) -> Option<Fraction> {
        self.0.scale_y.map(Fraction)
    }

    /// The horizontal skew, when stated.
    #[wasm_bindgen(getter, js_name = "skewX")]
    pub fn skew_x(&self) -> Option<Angle> {
        self.0.skew_x.map(Angle)
    }

    /// The vertical skew, when stated.
    #[wasm_bindgen(getter, js_name = "skewY")]
    pub fn skew_y(&self) -> Option<Angle> {
        self.0.skew_y.map(Angle)
    }

    /// Where the shadow is anchored, when stated.
    #[wasm_bindgen(getter, js_name = "alignment")]
    pub fn alignment(&self) -> Result<Option<RectangleAlignment>, JsValue> {
        self.0
            .alignment
            .map(RectangleAlignment::from_model)
            .transpose()
    }

    /// Whether the shadow rotates with the shape, when stated.
    #[wasm_bindgen(getter, js_name = "rotateWithShape")]
    pub fn rotate_with_shape(&self) -> Option<bool> {
        self.0.rotate_with_shape
    }
}

#[wasm_bindgen]
impl PresetShadowEffect {
    /// One of the twenty named shadows, in the given colour.
    #[wasm_bindgen(constructor)]
    pub fn new(
        preset: PresetShadow,
        color: &ColorSpec,
        distance: Option<Emu>,
        direction: Option<Angle>,
    ) -> Self {
        Self(ooxml::PresetShadowEffect {
            preset: preset.into(),
            color: color.0.clone(),
            distance: distance.map(|value| value.0),
            direction: direction.map(|value| value.0),
        })
    }

    /// This shadow at the given distance from the shape.
    #[wasm_bindgen(js_name = "withDistance")]
    pub fn with_distance(&self, distance: &Emu) -> Self {
        Self(self.0.clone().with_distance(distance.0))
    }

    /// This shadow cast in the given direction.
    #[wasm_bindgen(js_name = "withDirection")]
    pub fn with_direction(&self, direction: &Angle) -> Self {
        Self(self.0.clone().with_direction(direction.0))
    }

    /// Which of the twenty shadows this is.
    #[wasm_bindgen(getter, js_name = "preset")]
    pub fn preset(&self) -> Result<PresetShadow, JsValue> {
        PresetShadow::from_model(self.0.preset)
    }

    /// The shadow's colour.
    #[wasm_bindgen(getter, js_name = "color")]
    pub fn color(&self) -> ColorSpec {
        ColorSpec(self.0.color.clone())
    }

    /// The distance from the shape, when stated.
    #[wasm_bindgen(getter, js_name = "distance")]
    pub fn distance(&self) -> Option<Emu> {
        self.0.distance.map(Emu)
    }

    /// The direction the shadow is cast in, when stated.
    #[wasm_bindgen(getter, js_name = "direction")]
    pub fn direction(&self) -> Option<Angle> {
        self.0.direction.map(Angle)
    }
}

#[wasm_bindgen]
impl ReflectionEffect {
    /// A mirrored, fading copy of the shape. Every part is optional.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        blur_radius: Option<Emu>,
        start_alpha: Option<Fraction>,
        start_position: Option<Fraction>,
        end_alpha: Option<Fraction>,
        end_position: Option<Fraction>,
        distance: Option<Emu>,
        direction: Option<Angle>,
        fade_direction: Option<Angle>,
        scale_x: Option<Fraction>,
        scale_y: Option<Fraction>,
        skew_x: Option<Angle>,
        skew_y: Option<Angle>,
        alignment: Option<RectangleAlignment>,
        rotate_with_shape: Option<bool>,
    ) -> Self {
        Self(ooxml::ReflectionEffect {
            blur_radius: blur_radius.map(|value| value.0),
            start_alpha: start_alpha.map(|value| value.0),
            start_position: start_position.map(|value| value.0),
            end_alpha: end_alpha.map(|value| value.0),
            end_position: end_position.map(|value| value.0),
            distance: distance.map(|value| value.0),
            direction: direction.map(|value| value.0),
            fade_direction: fade_direction.map(|value| value.0),
            scale_x: scale_x.map(|value| value.0),
            scale_y: scale_y.map(|value| value.0),
            skew_x: skew_x.map(|value| value.0),
            skew_y: skew_y.map(|value| value.0),
            alignment: alignment.map(Into::into),
            rotate_with_shape,
        })
    }

    /// The blur radius, when stated.
    #[wasm_bindgen(getter, js_name = "blurRadius")]
    pub fn blur_radius(&self) -> Option<Emu> {
        self.0.blur_radius.map(Emu)
    }

    /// The opacity where the reflection starts, when stated.
    #[wasm_bindgen(getter, js_name = "startAlpha")]
    pub fn start_alpha(&self) -> Option<Fraction> {
        self.0.start_alpha.map(Fraction)
    }

    /// Where the reflection starts, when stated.
    #[wasm_bindgen(getter, js_name = "startPosition")]
    pub fn start_position(&self) -> Option<Fraction> {
        self.0.start_position.map(Fraction)
    }

    /// The opacity where the reflection ends, when stated.
    #[wasm_bindgen(getter, js_name = "endAlpha")]
    pub fn end_alpha(&self) -> Option<Fraction> {
        self.0.end_alpha.map(Fraction)
    }

    /// Where the reflection ends, when stated.
    #[wasm_bindgen(getter, js_name = "endPosition")]
    pub fn end_position(&self) -> Option<Fraction> {
        self.0.end_position.map(Fraction)
    }

    /// The distance from the shape, when stated.
    #[wasm_bindgen(getter, js_name = "distance")]
    pub fn distance(&self) -> Option<Emu> {
        self.0.distance.map(Emu)
    }

    /// The direction the reflection is offset in, when stated.
    #[wasm_bindgen(getter, js_name = "direction")]
    pub fn direction(&self) -> Option<Angle> {
        self.0.direction.map(Angle)
    }

    /// The direction the reflection fades in, when stated.
    #[wasm_bindgen(getter, js_name = "fadeDirection")]
    pub fn fade_direction(&self) -> Option<Angle> {
        self.0.fade_direction.map(Angle)
    }

    /// The horizontal scale, when stated.
    #[wasm_bindgen(getter, js_name = "scaleX")]
    pub fn scale_x(&self) -> Option<Fraction> {
        self.0.scale_x.map(Fraction)
    }

    /// The vertical scale, when stated.
    #[wasm_bindgen(getter, js_name = "scaleY")]
    pub fn scale_y(&self) -> Option<Fraction> {
        self.0.scale_y.map(Fraction)
    }

    /// The horizontal skew, when stated.
    #[wasm_bindgen(getter, js_name = "skewX")]
    pub fn skew_x(&self) -> Option<Angle> {
        self.0.skew_x.map(Angle)
    }

    /// The vertical skew, when stated.
    #[wasm_bindgen(getter, js_name = "skewY")]
    pub fn skew_y(&self) -> Option<Angle> {
        self.0.skew_y.map(Angle)
    }

    /// Where the reflection is anchored, when stated.
    #[wasm_bindgen(getter, js_name = "alignment")]
    pub fn alignment(&self) -> Result<Option<RectangleAlignment>, JsValue> {
        self.0
            .alignment
            .map(RectangleAlignment::from_model)
            .transpose()
    }

    /// Whether the reflection rotates with the shape, when stated.
    #[wasm_bindgen(getter, js_name = "rotateWithShape")]
    pub fn rotate_with_shape(&self) -> Option<bool> {
        self.0.rotate_with_shape
    }
}

#[wasm_bindgen]
impl SoftEdgeEffect {
    /// A feathered edge fading out over the given radius.
    #[wasm_bindgen(constructor)]
    pub fn new(radius: &Emu) -> Self {
        Self(ooxml::SoftEdgeEffect::new(radius.0))
    }

    /// The radius the edge fades over.
    #[wasm_bindgen(getter, js_name = "radius")]
    pub fn radius(&self) -> Emu {
        Emu(self.0.radius)
    }
}

#[wasm_bindgen]
impl EffectListSpec {
    /// An effect list that states nothing. Add to it with the `with_…` methods.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        blur: Option<BlurEffect>,
        fill_overlay: Option<FillOverlayEffect>,
        glow: Option<GlowEffect>,
        inner_shadow: Option<InnerShadowEffect>,
        outer_shadow: Option<OuterShadowEffect>,
        preset_shadow: Option<PresetShadowEffect>,
        reflection: Option<ReflectionEffect>,
        soft_edge: Option<SoftEdgeEffect>,
    ) -> Self {
        Self(ooxml::EffectListSpec {
            blur: blur.map(|effect| effect.0),
            fill_overlay: fill_overlay.map(|effect| effect.0),
            glow: glow.map(|effect| effect.0),
            inner_shadow: inner_shadow.map(|effect| effect.0),
            outer_shadow: outer_shadow.map(|effect| effect.0),
            preset_shadow: preset_shadow.map(|effect| effect.0),
            reflection: reflection.map(|effect| effect.0),
            soft_edge: soft_edge.map(|effect| effect.0),
        })
    }

    /// This list with the given blur.
    #[wasm_bindgen(js_name = "withBlur")]
    pub fn with_blur(&self, blur: &BlurEffect) -> Self {
        let mut list = self.0.clone();
        list.blur = Some(blur.0);
        Self(list)
    }

    /// This list with the given fill overlay.
    #[wasm_bindgen(js_name = "withFillOverlay")]
    pub fn with_fill_overlay(&self, fill_overlay: &FillOverlayEffect) -> Self {
        let mut list = self.0.clone();
        list.fill_overlay = Some(fill_overlay.0.clone());
        Self(list)
    }

    /// This list with the given glow.
    #[wasm_bindgen(js_name = "withGlow")]
    pub fn with_glow(&self, glow: &GlowEffect) -> Self {
        let mut list = self.0.clone();
        list.glow = Some(glow.0.clone());
        Self(list)
    }

    /// This list with the given inner shadow.
    #[wasm_bindgen(js_name = "withInnerShadow")]
    pub fn with_inner_shadow(&self, inner_shadow: &InnerShadowEffect) -> Self {
        let mut list = self.0.clone();
        list.inner_shadow = Some(inner_shadow.0.clone());
        Self(list)
    }

    /// This list with the given outer shadow.
    #[wasm_bindgen(js_name = "withOuterShadow")]
    pub fn with_outer_shadow(&self, outer_shadow: &OuterShadowEffect) -> Self {
        let mut list = self.0.clone();
        list.outer_shadow = Some(outer_shadow.0.clone());
        Self(list)
    }

    /// This list with the given preset shadow.
    #[wasm_bindgen(js_name = "withPresetShadow")]
    pub fn with_preset_shadow(&self, preset_shadow: &PresetShadowEffect) -> Self {
        let mut list = self.0.clone();
        list.preset_shadow = Some(preset_shadow.0.clone());
        Self(list)
    }

    /// This list with the given reflection.
    #[wasm_bindgen(js_name = "withReflection")]
    pub fn with_reflection(&self, reflection: &ReflectionEffect) -> Self {
        let mut list = self.0.clone();
        list.reflection = Some(reflection.0);
        Self(list)
    }

    /// This list with the given soft edge.
    #[wasm_bindgen(js_name = "withSoftEdge")]
    pub fn with_soft_edge(&self, soft_edge: &SoftEdgeEffect) -> Self {
        let mut list = self.0.clone();
        list.soft_edge = Some(soft_edge.0);
        Self(list)
    }

    /// The blur, when stated.
    #[wasm_bindgen(getter, js_name = "blur")]
    pub fn blur(&self) -> Option<BlurEffect> {
        self.0.blur.map(BlurEffect)
    }

    /// The fill overlay, when stated.
    #[wasm_bindgen(getter, js_name = "fillOverlay")]
    pub fn fill_overlay(&self) -> Option<FillOverlayEffect> {
        self.0.fill_overlay.clone().map(FillOverlayEffect)
    }

    /// The glow, when stated.
    #[wasm_bindgen(getter, js_name = "glow")]
    pub fn glow(&self) -> Option<GlowEffect> {
        self.0.glow.clone().map(GlowEffect)
    }

    /// The inner shadow, when stated.
    #[wasm_bindgen(getter, js_name = "innerShadow")]
    pub fn inner_shadow(&self) -> Option<InnerShadowEffect> {
        self.0.inner_shadow.clone().map(InnerShadowEffect)
    }

    /// The outer shadow, when stated.
    #[wasm_bindgen(getter, js_name = "outerShadow")]
    pub fn outer_shadow(&self) -> Option<OuterShadowEffect> {
        self.0.outer_shadow.clone().map(OuterShadowEffect)
    }

    /// The preset shadow, when stated.
    #[wasm_bindgen(getter, js_name = "presetShadow")]
    pub fn preset_shadow(&self) -> Option<PresetShadowEffect> {
        self.0.preset_shadow.clone().map(PresetShadowEffect)
    }

    /// The reflection, when stated.
    #[wasm_bindgen(getter, js_name = "reflection")]
    pub fn reflection(&self) -> Option<ReflectionEffect> {
        self.0.reflection.map(ReflectionEffect)
    }

    /// The soft edge, when stated.
    #[wasm_bindgen(getter, js_name = "softEdge")]
    pub fn soft_edge(&self) -> Option<SoftEdgeEffect> {
        self.0.soft_edge.map(SoftEdgeEffect)
    }
}

impl Default for LineSpec {
    /// The same value the no-argument constructor builds.
    fn default() -> Self {
        Self::new()
    }
}

impl Default for EffectListSpec {
    /// An effect list that states nothing, which is what the constructor builds with no arguments.
    fn default() -> Self {
        Self(mjx_ooxml::EffectListSpec::new())
    }
}
