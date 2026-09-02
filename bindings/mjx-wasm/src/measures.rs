//! The measures, each named for what it measures.
//!
//! OOXML states lengths in English Metric Units, font sizes in hundredths of a point, line widths in
//! EMU again, and proportions as a fraction of one — four different numbers that all look like `int`
//! from JavaScript, which has one numeric type. Each gets a class, so `Emu.fromPoints(12)` cannot
//! be handed to a parameter that wanted a `FontSize`, and nobody has to remember that 914 400 is an
//! inch.
//!
//! Every one is immutable, comparable with `equals`, and printable with `toString`, and every one
//! names both directions: `Emu.fromPoints(12).points === 12`.
//!
//! Each is a wasm object, so each has `free()`. They are small — leaking one costs a few bytes, not
//! a megabyte — but a loop that builds thousands should free them.

use wasm_bindgen::prelude::*;

use mjx_ooxml as ooxml;

use crate::support::invalid_argument;

value_class! {
    /// A length in English Metric Units — 914 400 to the inch, 12 700 to the point. The unit every
    /// position, size, margin, offset and radius in a document is stated in.
    Emu(ooxml::Emu), derive(Copy, PartialEq, Eq, Hash);

    /// An angle. OOXML stores sixtieths of a degree; this class speaks degrees and radians and
    /// converts.
    Angle(ooxml::Angle), derive(Copy, PartialEq);

    /// A proportion of one: `0.5` is fifty per cent. OOXML stores thousandths of a per cent.
    Fraction(ooxml::Fraction), derive(Copy, PartialEq);

    /// A font size in points. OOXML stores hundredths of a point, which is the resolution a size
    /// actually has — `10.5` is exact, `10.567` is not.
    FontSize(ooxml::FontSize), derive(Copy, PartialEq, Eq, Hash);

    /// A text measure in points — letter spacing, kerning, paragraph spacing. Distinct from
    /// [`FontSize`] because the two are not interchangeable in the markup even though both are
    /// hundredths of a point.
    TextPoint(ooxml::TextPoint), derive(Copy, PartialEq, Eq, Hash);

    /// A line width. EMU on the wire, points in practice.
    LineWidth(ooxml::LineWidth), derive(Copy, PartialEq, Eq, Hash);

    /// A list indent level, `0` through `8` — the nine levels a `p:txBody` list style defines.
    IndentLevel(ooxml::IndentLevel), derive(Copy, PartialEq, Eq, Hash);
}

#[wasm_bindgen]
impl Emu {
    /// A length stated directly in English Metric Units.
    #[wasm_bindgen(js_name = "fromEmu")]
    pub fn from_emu(emu: i64) -> Self {
        Self(ooxml::Emu::from_emu(emu))
    }

    /// A length in points — 12 700 EMU each.
    #[wasm_bindgen(js_name = "fromPoints")]
    pub fn from_points(points: f64) -> Self {
        Self(ooxml::Emu::from_points(points))
    }

    /// A length in inches — 914 400 EMU each.
    #[wasm_bindgen(js_name = "fromInches")]
    pub fn from_inches(inches: f64) -> Self {
        Self(ooxml::Emu::from_emu((inches * 914_400.0) as i64))
    }

    /// A length in centimetres — 360 000 EMU each.
    #[wasm_bindgen(js_name = "fromCentimetres")]
    pub fn from_centimetres(centimetres: f64) -> Self {
        Self(ooxml::Emu::from_emu((centimetres * 360_000.0) as i64))
    }

    /// The value in English Metric Units.
    #[wasm_bindgen(getter, js_name = "emu")]
    pub fn emu(&self) -> i64 {
        self.0.emu()
    }

    /// The value in points.
    #[wasm_bindgen(getter, js_name = "points")]
    pub fn points(&self) -> f64 {
        self.0.points()
    }

    /// The value in inches.
    #[wasm_bindgen(getter, js_name = "inches")]
    pub fn inches(&self) -> f64 {
        self.0.emu() as f64 / 914_400.0
    }

    /// The value in centimetres.
    #[wasm_bindgen(getter, js_name = "centimetres")]
    pub fn centimetres(&self) -> f64 {
        self.0.emu() as f64 / 360_000.0
    }
}

#[wasm_bindgen]
impl Angle {
    /// An angle in degrees, measured clockwise as OOXML measures it.
    #[wasm_bindgen(js_name = "fromDegrees")]
    pub fn from_degrees(degrees: f64) -> Self {
        Self(ooxml::Angle::from_degrees(degrees))
    }

    /// An angle in radians.
    #[wasm_bindgen(js_name = "fromRadians")]
    pub fn from_radians(radians: f64) -> Self {
        Self(ooxml::Angle::from_radians(radians))
    }

    /// The angle in degrees.
    #[wasm_bindgen(getter, js_name = "degrees")]
    pub fn degrees(&self) -> f64 {
        self.0.degrees()
    }

    /// The angle in radians.
    #[wasm_bindgen(getter, js_name = "radians")]
    pub fn radians(&self) -> f64 {
        self.0.radians()
    }
}

#[wasm_bindgen]
impl Fraction {
    /// A proportion of one: `Fraction.of(0.5)` is fifty per cent.
    pub fn of(ratio: f64) -> Self {
        Self(ooxml::Fraction::from_ratio(ratio))
    }

    /// A proportion given as a percentage: `Fraction.percent(50)` is the same as `Fraction.of(0.5)`.
    pub fn percent(percent: f64) -> Self {
        Self(ooxml::Fraction::from_ratio(percent / 100.0))
    }

    /// The proportion, as a fraction of one.
    #[wasm_bindgen(getter, js_name = "ratio")]
    pub fn ratio(&self) -> f64 {
        self.0.ratio()
    }

    /// The proportion, as a percentage.
    #[wasm_bindgen(getter, js_name = "percentage")]
    pub fn percentage(&self) -> f64 {
        self.0.ratio() * 100.0
    }
}

#[wasm_bindgen]
impl FontSize {
    /// A size in points.
    #[wasm_bindgen(js_name = "fromPoints")]
    pub fn from_points(points: f64) -> Self {
        Self(ooxml::FontSize::from_points(points))
    }

    /// A size in the hundredths of a point the markup stores.
    #[wasm_bindgen(js_name = "fromHundredthsOfAPoint")]
    pub fn from_hundredths_of_a_point(hundredths: i32) -> Self {
        Self(ooxml::FontSize::from_wire(hundredths))
    }

    /// The size in points.
    #[wasm_bindgen(getter, js_name = "points")]
    pub fn points(&self) -> f64 {
        self.0.points()
    }

    /// The size in hundredths of a point, exactly as it is written.
    #[wasm_bindgen(getter, js_name = "hundredthsOfAPoint")]
    pub fn hundredths_of_a_point(&self) -> i32 {
        self.0.to_wire()
    }
}

#[wasm_bindgen]
impl TextPoint {
    /// A measure in points.
    #[wasm_bindgen(js_name = "fromPoints")]
    pub fn from_points(points: f64) -> Self {
        Self(ooxml::TextPoint::from_points(points))
    }

    /// A measure in the hundredths of a point the markup stores.
    #[wasm_bindgen(js_name = "fromHundredthsOfAPoint")]
    pub fn from_hundredths_of_a_point(hundredths: i32) -> Self {
        Self(ooxml::TextPoint::from_wire(hundredths))
    }

    /// The measure in points.
    #[wasm_bindgen(getter, js_name = "points")]
    pub fn points(&self) -> f64 {
        self.0.points()
    }

    /// The measure in hundredths of a point, exactly as it is written.
    #[wasm_bindgen(getter, js_name = "hundredthsOfAPoint")]
    pub fn hundredths_of_a_point(&self) -> i32 {
        self.0.to_wire()
    }
}

#[wasm_bindgen]
impl LineWidth {
    /// A width in points.
    #[wasm_bindgen(js_name = "fromPoints")]
    pub fn from_points(points: f64) -> Self {
        Self(ooxml::LineWidth::from_points(points))
    }

    /// A width stated directly in English Metric Units.
    #[wasm_bindgen(js_name = "fromEmu")]
    pub fn from_emu(emu: i64) -> Self {
        Self(ooxml::LineWidth::from_emu(emu))
    }

    /// The width in points.
    #[wasm_bindgen(getter, js_name = "points")]
    pub fn points(&self) -> f64 {
        self.0.points()
    }

    /// The width in English Metric Units.
    #[wasm_bindgen(getter, js_name = "emu")]
    pub fn emu(&self) -> i64 {
        self.0.emu()
    }
}

#[wasm_bindgen]
impl IndentLevel {
    /// The list level at this depth, `0` through `8`.
    ///
    /// Throws for anything outside that range: a `p:txBody` list style defines nine levels and no
    /// more, so a tenth would be written and then silently ignored.
    #[wasm_bindgen(constructor)]
    pub fn new(level: u8) -> Result<Self, JsValue> {
        ooxml::IndentLevel::new(level).map(Self).ok_or_else(|| {
            invalid_argument(format!(
                "a list level is 0 through 8; {level} is outside what `a:lvl{{n}}pPr` can express"
            ))
        })
    }

    /// The level, `0` through `8`.
    #[wasm_bindgen(getter, js_name = "value")]
    pub fn value(&self) -> u8 {
        self.0.value()
    }
}
