//! The measures, each named for what it measures.
//!
//! OOXML states lengths in English Metric Units, font sizes in hundredths of a point, line widths in
//! EMU again, and proportions as a fraction of one — four different numbers that all look like `int`
//! from Python. Each gets a class, so `Emu.from_points(12)` cannot be handed to a parameter that
//! wanted a `FontSize`, and nobody has to remember that 914 400 is an inch.
//!
//! Every one is immutable, hashable, comparable and printable, and every one names both directions:
//! `Emu.from_points(12).points == 12.0`.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use pyo3::prelude::*;
use pyo3::types::PyModule;

use mjx_ooxml as ooxml;

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

/// Hashes any hashable model value into the `u64` `__hash__` must return.
fn hash_of<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[pymethods]
impl Emu {
    /// A length stated directly in English Metric Units.
    #[staticmethod]
    fn from_emu(emu: i64) -> Self {
        Self(ooxml::Emu::from_emu(emu))
    }

    /// A length in points — 12 700 EMU each.
    #[staticmethod]
    fn from_points(points: f64) -> Self {
        Self(ooxml::Emu::from_points(points))
    }

    /// A length in inches — 914 400 EMU each.
    #[staticmethod]
    fn from_inches(inches: f64) -> Self {
        Self(ooxml::Emu::from_emu((inches * 914_400.0) as i64))
    }

    /// A length in centimetres — 360 000 EMU each.
    #[staticmethod]
    fn from_centimetres(centimetres: f64) -> Self {
        Self(ooxml::Emu::from_emu((centimetres * 360_000.0) as i64))
    }

    /// The value in English Metric Units.
    #[getter]
    fn emu(&self) -> i64 {
        self.0.emu()
    }

    /// The value in points.
    #[getter]
    fn points(&self) -> f64 {
        self.0.points()
    }

    /// The value in inches.
    #[getter]
    fn inches(&self) -> f64 {
        self.0.emu() as f64 / 914_400.0
    }

    /// The value in centimetres.
    #[getter]
    fn centimetres(&self) -> f64 {
        self.0.emu() as f64 / 360_000.0
    }

    fn __repr__(&self) -> String {
        format!("Emu.from_emu({})", self.0.emu())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_of(&self.0.emu())
    }
}

#[pymethods]
impl Angle {
    /// An angle in degrees, measured clockwise as OOXML measures it.
    #[staticmethod]
    fn from_degrees(degrees: f64) -> Self {
        Self(ooxml::Angle::from_degrees(degrees))
    }

    /// An angle in radians.
    #[staticmethod]
    fn from_radians(radians: f64) -> Self {
        Self(ooxml::Angle::from_radians(radians))
    }

    /// The angle in degrees.
    #[getter]
    fn degrees(&self) -> f64 {
        self.0.degrees()
    }

    /// The angle in radians.
    #[getter]
    fn radians(&self) -> f64 {
        self.0.radians()
    }

    fn __repr__(&self) -> String {
        format!("Angle.from_degrees({})", self.0.degrees())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Fraction {
    /// A proportion of one: `Fraction.of(0.5)` is fifty per cent.
    #[staticmethod]
    fn of(ratio: f64) -> Self {
        Self(ooxml::Fraction::from_ratio(ratio))
    }

    /// A proportion given as a percentage: `Fraction.percent(50)` is the same as `Fraction.of(0.5)`.
    #[staticmethod]
    fn percent(percent: f64) -> Self {
        Self(ooxml::Fraction::from_ratio(percent / 100.0))
    }

    /// The proportion, as a fraction of one.
    #[getter]
    fn ratio(&self) -> f64 {
        self.0.ratio()
    }

    /// The proportion, as a percentage.
    #[getter]
    fn percentage(&self) -> f64 {
        self.0.ratio() * 100.0
    }

    fn __repr__(&self) -> String {
        format!("Fraction.of({})", self.0.ratio())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl FontSize {
    /// A size in points.
    #[staticmethod]
    fn from_points(points: f64) -> Self {
        Self(ooxml::FontSize::from_points(points))
    }

    /// A size in the hundredths of a point the markup stores.
    #[staticmethod]
    fn from_hundredths_of_a_point(hundredths: i32) -> Self {
        Self(ooxml::FontSize::from_wire(hundredths))
    }

    /// The size in points.
    #[getter]
    fn points(&self) -> f64 {
        self.0.points()
    }

    /// The size in hundredths of a point, exactly as it is written.
    #[getter]
    fn hundredths_of_a_point(&self) -> i32 {
        self.0.to_wire()
    }

    fn __repr__(&self) -> String {
        format!("FontSize.from_points({})", self.0.points())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_of(&self.0.to_wire())
    }
}

#[pymethods]
impl TextPoint {
    /// A measure in points.
    #[staticmethod]
    fn from_points(points: f64) -> Self {
        Self(ooxml::TextPoint::from_points(points))
    }

    /// A measure in the hundredths of a point the markup stores.
    #[staticmethod]
    fn from_hundredths_of_a_point(hundredths: i32) -> Self {
        Self(ooxml::TextPoint::from_wire(hundredths))
    }

    /// The measure in points.
    #[getter]
    fn points(&self) -> f64 {
        self.0.points()
    }

    /// The measure in hundredths of a point, exactly as it is written.
    #[getter]
    fn hundredths_of_a_point(&self) -> i32 {
        self.0.to_wire()
    }

    fn __repr__(&self) -> String {
        format!("TextPoint.from_points({})", self.0.points())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_of(&self.0.to_wire())
    }
}

#[pymethods]
impl LineWidth {
    /// A width in points.
    #[staticmethod]
    fn from_points(points: f64) -> Self {
        Self(ooxml::LineWidth::from_points(points))
    }

    /// A width stated directly in English Metric Units.
    #[staticmethod]
    fn from_emu(emu: i64) -> Self {
        Self(ooxml::LineWidth::from_emu(emu))
    }

    /// The width in points.
    #[getter]
    fn points(&self) -> f64 {
        self.0.points()
    }

    /// The width in English Metric Units.
    #[getter]
    fn emu(&self) -> i64 {
        self.0.emu()
    }

    fn __repr__(&self) -> String {
        format!("LineWidth.from_points({})", self.0.points())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_of(&self.0.emu())
    }
}

#[pymethods]
impl IndentLevel {
    /// The list level at this depth, `0` through `8`.
    ///
    /// Raises `ValueError` for anything outside that range: a `p:txBody` list style defines nine
    /// levels and no more, so a tenth would be written and then silently ignored.
    #[new]
    fn new(level: u8) -> PyResult<Self> {
        ooxml::IndentLevel::new(level).map(Self).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "a list level is 0 through 8; {level} is outside what `a:lvl{{n}}pPr` can express"
            ))
        })
    }

    /// The level, `0` through `8`.
    #[getter]
    fn value(&self) -> u8 {
        self.0.value()
    }

    fn __repr__(&self) -> String {
        format!("IndentLevel({})", self.0.value())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_of(&self.0.value())
    }
}

/// Adds every measure class to the extension module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Emu>()?;
    module.add_class::<Angle>()?;
    module.add_class::<Fraction>()?;
    module.add_class::<FontSize>()?;
    module.add_class::<TextPoint>()?;
    module.add_class::<LineWidth>()?;
    module.add_class::<IndentLevel>()
}
