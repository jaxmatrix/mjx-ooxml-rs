//! The DrawingML **guide-formula language** (`a:gd@fmla`) and its evaluator.
//!
//! A shape guide is a `name`/`fmla` pair: `<a:gd name="x1" fmla="*/ w adj1 100000"/>`. The formula is
//! a prefix expression — an operator token followed by its arguments, each a numeric literal or the
//! name of another guide — and the result is bound to the guide's name for later guides to use.
//! Anywhere a coordinate or an angle may name a guide ([`AdjustCoordinate`](super::AdjustCoordinate),
//! [`AdjustAngle`](super::AdjustAngle)), that name is looked up in the values these formulas produce.
//!
//! # The seventeen operators
//!
//! [`GuideOperator`] is the closed set, each variant carrying its exact wire token and the semantics
//! **quoted from the ECMA-376 Part 1 prose** (§20.1.9.11, the `fmla` attribute of `a:gd`). Nothing
//! here is inferred from the token's spelling.
//!
//! # Units
//!
//! Every value is a plain number in the shape's own coordinate space — EMU for a length, and
//! **60000ths of a degree** for an angle (§20.1.10.56 states the angular built-ins in those units:
//! `cd4` is `5400000.0`, "equivalent to 90 degrees"). So [`Sine`](GuideOperator::Sine),
//! [`Cosine`](GuideOperator::Cosine) and [`Tangent`](GuideOperator::Tangent) take their angle in
//! 60000ths of a degree, and [`ArcTangent`](GuideOperator::ArcTangent) returns one.
//!
//! # Built-in variables
//!
//! §20.1.10.56 (`ST_ShapeType`) lists the "predefined guides that the generating application shall
//! maintain for calculation purposes at all times" — the shape's width and height, the edges and
//! centres derived from them, and the circle constants. [`GuideContext`] supplies them from a shape
//! size; see [`GuideContext::variable`].
//!
//! # Evaluation order, and why there is no cycle to detect
//!
//! §20.1.9.11 is explicit: *"The order in which guides are specified determines the order in which
//! their values are calculated. For instance it is not possible to specify a guide that uses another
//! guides result when that guide has not yet been calculated."* [`ResolvedGuides`] therefore
//! evaluates a guide list **once, in declaration order**, into a map: each formula sees only the
//! guides before it, so a forward reference or a self-reference is an
//! [`UndefinedGuide`](GuideError::UndefinedGuide) error rather than a loop. There is no recursion and
//! no fixed-point iteration, so a hostile `gdLst` cannot make the evaluator spin or overflow the
//! stack, and a list of *n* guides costs one pass and one hash insert each.

use std::borrow::Cow;
use std::collections::HashMap;

use super::measures::{Angle, Emu};
use super::transform::Size;

/// The native wire scale of an angle — sixtieths of a thousandth of a degree (ECMA-376 Part 1
/// §20.1.10.56: `cd4` is `5400000.0`, "The units here are in 60,000ths of a degree").
pub(crate) const ANGLE_UNITS_PER_DEGREE: f64 = 60_000.0;

/// A full turn in native angle units (`21_600_000` = 360°), the numerator of every circle constant.
const FULL_TURN: f64 = 360.0 * ANGLE_UNITS_PER_DEGREE;

/// The most arguments any guide formula takes (`*/`, `+-`, `+/`, `?:`, `cat2`, `mod`, `pin`, `sat2`).
const MAX_ARGUMENTS: usize = 3;

/// An operator of the guide-formula language (`a:gd@fmla`, `ST_GeomGuideFormula`).
///
/// There are exactly seventeen, listed with the `fmla` attribute in ECMA-376 Part 1 §20.1.9.11. Each
/// variant's documentation quotes that prose; [`to_wire`](Self::to_wire) gives the exact token the
/// file carries, which is never guessed from the Rust name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuideOperator {
    /// `*/` — "Multiply Divide Formula". `"*/ x y z" = ((x * y) / z)`.
    MultiplyDivide,
    /// `+-` — "Add Subtract Formula". `"+- x y z" = ((x + y) - z)`.
    AddSubtract,
    /// `+/` — "Add Divide Formula". `"+/ x y z" = ((x + y) / z)`.
    AddDivide,
    /// `?:` — "If Else Formula". `"?: x y z" = if (x > 0), then y … else z`.
    IfElse,
    /// `abs` — "Absolute Value Formula". `"abs x" = if (x < 0), then (-1) * x … else x`.
    AbsoluteValue,
    /// `at2` — "ArcTan Formula". `"at2 x y" = arctan(y / x)`, in 60000ths of a degree.
    ///
    /// The two arguments are kept apart rather than divided: the token is the *two-argument* arc
    /// tangent, and the sign of `x` selects the half-plane. The spec's own preset definitions require
    /// it — `moon` computes `stAng1 = at2 dx2 dy2` with **both** arguments negative and then treats
    /// the result as a third-quadrant angle (its `enAng1` subtracts a whole turn, `21600000`, to get
    /// back below it), which only holds if a negative `x` turns the angle past ±90°.
    ArcTangent,
    /// `cat2` — "Cosine ArcTan Formula". `"cat2 x y z" = (x*(cos(arctan(z / y)))`.
    ///
    /// As with [`ArcTangent`](Self::ArcTangent) the inner arc tangent is the two-argument one, so the
    /// sign of `y` reaches the result. The spec's `arc` shape is the proof: it places the arc's start
    /// point at `hc + cat2 wd2 ht1 wt1` where `ht1 = cos hd2 stAng` and `wt1 = sin wd2 stAng`, which
    /// must land on the **left** edge for a start angle of `cd2` (180°) — and does so only because
    /// `ht1` is then negative.
    CosineArcTangent,
    /// `cos` — "Cosine Formula". `"cos x y" = (x * cos( y ))`, `y` in 60000ths of a degree.
    Cosine,
    /// `max` — "Maximum Value Formula". `"max x y" = if (x > y), then x … else y`.
    Maximum,
    /// `min` — "Minimum Value Formula". `"min x y" = if (x < y), then x … else y`.
    Minimum,
    /// `mod` — "Modulo Formula". `"mod x y z" = sqrt(x^2 + b^2 + c^2)`.
    ///
    /// The prose's `b` and `c` are typographical slips for the second and third arguments: this is
    /// the modulus (Euclidean length) of the vector `(x, y, z)`, not a remainder — hence the name.
    Modulus,
    /// `pin` — "Pin To Formula". `"pin x y z" = if (y < x), then x … else if (y > z), then z … else y`.
    PinToRange,
    /// `sat2` — "Sine ArcTan Formula". `"sat2 x y z" = (x*sin(arctan(z / y)))`.
    ///
    /// The inner arc tangent is two-argument, for the reason given on
    /// [`CosineArcTangent`](Self::CosineArcTangent).
    SineArcTangent,
    /// `sin` — "Sine Formula". `"sin x y" = (x * sin( y ))`, `y` in 60000ths of a degree.
    Sine,
    /// `sqrt` — "Square Root Formula". `"sqrt x" = sqrt(x)`.
    SquareRoot,
    /// `tan` — "Tangent Formula". `"tan x y" = (x * tan( y ))`, `y` in 60000ths of a degree.
    Tangent,
    /// `val` — "Literal Value Formula". `"val x" = x`.
    LiteralValue,
}

impl GuideOperator {
    /// Every operator, in the order ECMA-376 Part 1 §20.1.9.11 lists them.
    pub const ALL: [Self; 17] = [
        Self::MultiplyDivide,
        Self::AddSubtract,
        Self::AddDivide,
        Self::IfElse,
        Self::AbsoluteValue,
        Self::ArcTangent,
        Self::CosineArcTangent,
        Self::Cosine,
        Self::Maximum,
        Self::Minimum,
        Self::Modulus,
        Self::PinToRange,
        Self::SineArcTangent,
        Self::Sine,
        Self::SquareRoot,
        Self::Tangent,
        Self::LiteralValue,
    ];

    /// Parses an operator from its exact wire token (`*/`, `at2`, `pin`, …), or `None` if the token
    /// is not one of the seventeen.
    #[must_use]
    pub fn from_wire(token: &str) -> Option<Self> {
        Some(match token {
            "*/" => Self::MultiplyDivide,
            "+-" => Self::AddSubtract,
            "+/" => Self::AddDivide,
            "?:" => Self::IfElse,
            "abs" => Self::AbsoluteValue,
            "at2" => Self::ArcTangent,
            "cat2" => Self::CosineArcTangent,
            "cos" => Self::Cosine,
            "max" => Self::Maximum,
            "min" => Self::Minimum,
            "mod" => Self::Modulus,
            "pin" => Self::PinToRange,
            "sat2" => Self::SineArcTangent,
            "sin" => Self::Sine,
            "sqrt" => Self::SquareRoot,
            "tan" => Self::Tangent,
            "val" => Self::LiteralValue,
            _ => return None,
        })
    }

    /// The exact wire token this operator is written as.
    #[must_use]
    pub const fn to_wire(self) -> &'static str {
        match self {
            Self::MultiplyDivide => "*/",
            Self::AddSubtract => "+-",
            Self::AddDivide => "+/",
            Self::IfElse => "?:",
            Self::AbsoluteValue => "abs",
            Self::ArcTangent => "at2",
            Self::CosineArcTangent => "cat2",
            Self::Cosine => "cos",
            Self::Maximum => "max",
            Self::Minimum => "min",
            Self::Modulus => "mod",
            Self::PinToRange => "pin",
            Self::SineArcTangent => "sat2",
            Self::Sine => "sin",
            Self::SquareRoot => "sqrt",
            Self::Tangent => "tan",
            Self::LiteralValue => "val",
        }
    }

    /// How many arguments the operator takes (`1`, `2` or `3`), as stated by its "Arguments:" line.
    #[must_use]
    pub const fn argument_count(self) -> usize {
        match self {
            Self::AbsoluteValue | Self::SquareRoot | Self::LiteralValue => 1,
            Self::ArcTangent
            | Self::Cosine
            | Self::Maximum
            | Self::Minimum
            | Self::Sine
            | Self::Tangent => 2,
            Self::MultiplyDivide
            | Self::AddSubtract
            | Self::AddDivide
            | Self::IfElse
            | Self::CosineArcTangent
            | Self::Modulus
            | Self::PinToRange
            | Self::SineArcTangent => 3,
        }
    }

    /// Applies the operator to already-resolved arguments. `arguments` is exactly
    /// [`argument_count`](Self::argument_count) long; a shorter slice reads the missing arguments as
    /// zero rather than panicking, so this is total even on a formula that failed validation.
    fn apply(self, arguments: &[f64]) -> f64 {
        let x = arguments.first().copied().unwrap_or(0.0);
        let y = arguments.get(1).copied().unwrap_or(0.0);
        let z = arguments.get(2).copied().unwrap_or(0.0);
        match self {
            Self::MultiplyDivide => (x * y) / z,
            Self::AddSubtract => (x + y) - z,
            Self::AddDivide => (x + y) / z,
            Self::IfElse => {
                if x > 0.0 {
                    y
                } else {
                    z
                }
            }
            Self::AbsoluteValue => x.abs(),
            Self::ArcTangent => radians_to_native(y.atan2(x)),
            Self::CosineArcTangent => x * z.atan2(y).cos(),
            Self::Cosine => x * native_to_radians(y).cos(),
            Self::Maximum => {
                if x > y {
                    x
                } else {
                    y
                }
            }
            Self::Minimum => {
                if x < y {
                    x
                } else {
                    y
                }
            }
            Self::Modulus => (x * x + y * y + z * z).sqrt(),
            Self::PinToRange => {
                if y < x {
                    x
                } else if y > z {
                    z
                } else {
                    y
                }
            }
            Self::SineArcTangent => x * z.atan2(y).sin(),
            Self::Sine => x * native_to_radians(y).sin(),
            Self::SquareRoot => x.sqrt(),
            Self::Tangent => x * native_to_radians(y).tan(),
            Self::LiteralValue => x,
        }
    }
}

/// Native angle units (60000ths of a degree) as radians.
fn native_to_radians(native: f64) -> f64 {
    (native / ANGLE_UNITS_PER_DEGREE).to_radians()
}

/// Radians as native angle units (60000ths of a degree).
fn radians_to_native(radians: f64) -> f64 {
    radians.to_degrees() * ANGLE_UNITS_PER_DEGREE
}

/// One argument of a guide formula: a numeric literal, or the name of another guide / a built-in.
///
/// The name borrows from the formula string, so parsing a formula allocates nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuideArgument<'a> {
    /// A numeric literal, e.g. the `100000` of `*/ w adj1 100000` (the spec also writes `1.0`/`2.0`).
    Literal(f64),
    /// A name resolved against the guides evaluated so far, then the built-in variables.
    Name(&'a str),
}

/// A parse failure in a guide formula (`a:gd@fmla`) — the shape of the formula, before any value is
/// looked up.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GuideFormulaError {
    /// The formula is empty or all whitespace, so it names no operator.
    #[error("the guide formula is empty")]
    Empty,
    /// The leading token is not one of the seventeen operators.
    #[error("`{token}` is not one of the seventeen guide-formula operators")]
    UnknownOperator {
        /// The token as written in the file.
        token: String,
    },
    /// The operator was given the wrong number of arguments.
    #[error("the `{operator}` guide formula takes {expected} argument(s), but {found} were given")]
    ArgumentCount {
        /// The operator's wire token.
        operator: &'static str,
        /// How many arguments it takes.
        expected: usize,
        /// How many the formula supplied.
        found: usize,
    },
}

/// A failure while evaluating a guide.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GuideError {
    /// The formula could not be parsed.
    #[error("the guide formula `{formula}` is malformed: {source}")]
    Malformed {
        /// The formula as written in the file.
        formula: String,
        /// Why it did not parse.
        #[source]
        source: GuideFormulaError,
    },
    /// A name resolved to nothing: no guide before this one defines it, and it is not a built-in.
    ///
    /// A guide that refers to itself, or forward to a later guide, arrives here — guides are
    /// evaluated once in declaration order (see the [module docs](self)), so neither can loop.
    #[error("`{name}` names neither an already-evaluated guide nor a built-in variable")]
    UndefinedGuide {
        /// The name the file referenced.
        name: String,
    },
    /// The formula's arithmetic left the reals — a division by zero, the square root of a negative,
    /// or an overflow. Never a panic: the value is checked before it reaches a caller.
    #[error("the guide formula `{formula}` does not evaluate to a finite number")]
    NotFinite {
        /// The formula as written in the file.
        formula: String,
    },
    /// Which guide of a list failed, wrapping the failure itself.
    #[error("guide `{guide}` could not be evaluated: {source}")]
    Guide {
        /// The guide's `name` attribute.
        guide: String,
        /// What went wrong inside it.
        #[source]
        source: Box<GuideError>,
    },
}

/// A parsed guide formula: an operator and its arguments, borrowed from the formula string.
///
/// ```
/// use mjx_dml::{GuideArgument, GuideFormula, GuideOperator};
///
/// let formula = GuideFormula::parse("*/ w adj1 100000").expect("well-formed");
/// assert_eq!(formula.operator(), GuideOperator::MultiplyDivide);
/// assert_eq!(formula.arguments()[0], GuideArgument::Name("w"));
/// assert_eq!(formula.arguments()[2], GuideArgument::Literal(100_000.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuideFormula<'a> {
    text: &'a str,
    operator: GuideOperator,
    arguments: [GuideArgument<'a>; MAX_ARGUMENTS],
    count: u8,
}

impl<'a> GuideFormula<'a> {
    /// Parses a formula: an operator token followed by exactly the arguments it takes, separated by
    /// whitespace. Borrows `formula`, so nothing is allocated on the success path.
    ///
    /// # Errors
    ///
    /// [`GuideFormulaError`] if the formula is empty, names an operator this language does not have,
    /// or supplies the wrong number of arguments.
    pub fn parse(formula: &'a str) -> Result<Self, GuideFormulaError> {
        let mut tokens = formula.split_whitespace();
        let token = tokens.next().ok_or(GuideFormulaError::Empty)?;
        let operator =
            GuideOperator::from_wire(token).ok_or_else(|| GuideFormulaError::UnknownOperator {
                token: token.to_owned(),
            })?;

        let mut arguments = [GuideArgument::Literal(0.0); MAX_ARGUMENTS];
        let mut found = 0usize;
        for argument in tokens {
            if found < MAX_ARGUMENTS {
                arguments[found] = parse_argument(argument);
            }
            found += 1;
        }
        let expected = operator.argument_count();
        if found != expected {
            return Err(GuideFormulaError::ArgumentCount {
                operator: operator.to_wire(),
                expected,
                found,
            });
        }
        Ok(Self {
            text: formula,
            operator,
            arguments,
            count: expected as u8,
        })
    }

    /// The formula exactly as it was written.
    #[must_use]
    pub const fn text(&self) -> &'a str {
        self.text
    }

    /// The operator.
    #[must_use]
    pub const fn operator(&self) -> GuideOperator {
        self.operator
    }

    /// The arguments, in order — always [`GuideOperator::argument_count`] of them.
    #[must_use]
    pub fn arguments(&self) -> &[GuideArgument<'a>] {
        &self.arguments[..self.count as usize]
    }

    /// Evaluates the formula against the guides resolved so far.
    ///
    /// # Errors
    ///
    /// [`GuideError::UndefinedGuide`] if an argument names nothing known, or
    /// [`GuideError::NotFinite`] if the arithmetic leaves the reals (a division by zero, the square
    /// root of a negative, an overflow).
    pub fn evaluate(&self, guides: &ResolvedGuides<'_>) -> Result<f64, GuideError> {
        let mut values = [0.0f64; MAX_ARGUMENTS];
        for (slot, argument) in values.iter_mut().zip(self.arguments()) {
            *slot = match argument {
                GuideArgument::Literal(literal) => *literal,
                GuideArgument::Name(name) => {
                    guides
                        .value(name)
                        .ok_or_else(|| GuideError::UndefinedGuide {
                            name: (*name).to_owned(),
                        })?
                }
            };
        }
        let value = self.operator.apply(&values[..self.count as usize]);
        if value.is_finite() {
            Ok(value)
        } else {
            Err(GuideError::NotFinite {
                formula: self.text.to_owned(),
            })
        }
    }
}

/// Reads one formula argument: a strict decimal literal, otherwise a name.
///
/// The literal grammar is deliberately narrower than [`f64::from_str`], which also accepts `inf`,
/// `infinity` and `nan` — those are legal `ST_GeomGuideName`s, and a file must not be able to smuggle
/// a non-finite value in as a "number".
fn parse_argument(token: &str) -> GuideArgument<'_> {
    if is_decimal_literal(token) {
        if let Ok(literal) = token.parse::<f64>() {
            return GuideArgument::Literal(literal);
        }
    }
    GuideArgument::Name(token)
}

/// Whether `token` matches `[+-]?(digits[.digits?] | .digits)([eE][+-]?digits)?` — a plain decimal
/// number, with no `inf`/`nan` spelling and no hexadecimal form.
fn is_decimal_literal(token: &str) -> bool {
    let bytes = token.as_bytes();
    let mut index = 0usize;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        index += 1;
    }
    let integer_digits = count_digits(bytes, &mut index);
    let mut fraction_digits = 0usize;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        fraction_digits = count_digits(bytes, &mut index);
    }
    if integer_digits + fraction_digits == 0 {
        return false;
    }
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        if count_digits(bytes, &mut index) == 0 {
            return false;
        }
    }
    index == bytes.len()
}

/// Advances `index` over ASCII digits, returning how many it passed.
fn count_digits(bytes: &[u8], index: &mut usize) -> usize {
    let start = *index;
    while matches!(bytes.get(*index), Some(byte) if byte.is_ascii_digit()) {
        *index += 1;
    }
    *index - start
}

/// The shape size every built-in variable is derived from — the `w` and `h` a guide formula reads,
/// taken from the shape's own transform (`a:xfrm/a:ext`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuideContext {
    width: f64,
    height: f64,
}

impl GuideContext {
    /// A context for a shape of the given extents.
    #[must_use]
    pub fn from_extents(width: Emu, height: Emu) -> Self {
        Self {
            width: width.emu() as f64,
            height: height.emu() as f64,
        }
    }

    /// A context for a shape of the given [`Size`] (an `a:ext`).
    #[must_use]
    pub fn from_size(size: Size) -> Self {
        Self::from_extents(size.width, size.height)
    }

    /// The shape width the built-ins are derived from.
    #[must_use]
    pub fn width(self) -> Emu {
        Emu::from_emu(self.width as i64)
    }

    /// The shape height the built-ins are derived from.
    #[must_use]
    pub fn height(self) -> Emu {
        Emu::from_emu(self.height as i64)
    }

    /// The value of a built-in variable, or `None` if the name is not one.
    ///
    /// ECMA-376 Part 1 §20.1.10.56 names these "predefined guides that the generating application
    /// shall maintain for calculation purposes at all times":
    ///
    /// | Name | Value | |
    /// |---|---|---|
    /// | `w`, `h` | the shape's width and height | |
    /// | `l`, `t` | `0` | the left and top edges are the coordinate origin |
    /// | `r`, `b` | `w`, `h` | the right and bottom edges |
    /// | `hc`, `vc` | `w/2`, `h/2` | horizontal / vertical centre |
    /// | `ss`, `ls` | `min w h`, `max w h` | shortest / longest side |
    /// | `wd`*N*, `hd`*N*, `ssd`*N* | `w/N`, `h/N`, `ss/N` | a fraction of the width / height / shorter side |
    /// | *M*`cd`*N*, `cd`*N* | `21600000 · M/N`, `21600000/N` | a fraction of a full turn, in 60000ths of a degree |
    ///
    /// The prose spells out only the divisors its examples need (`wd2`…`wd10`, `cd2`, `cd4`, `cd8`,
    /// `3cd4`, `3cd8`, `5cd8`, `7cd8`, …); the divisor families are read as the general rule they
    /// state, because the spec's own preset shape definitions use members the table omits (`cd3`,
    /// `hd10`, `wd12`, `wd32`) and the rule reproduces every listed constant exactly. `lsd`*N* is
    /// **not** accepted: neither the prose nor the preset definitions have it, and inventing it would
    /// be a guess.
    #[must_use]
    pub fn variable(self, name: &str) -> Option<f64> {
        let (width, height) = (self.width, self.height);
        Some(match name {
            "w" | "r" => width,
            "h" | "b" => height,
            "l" | "t" => 0.0,
            "hc" => width / 2.0,
            "vc" => height / 2.0,
            "ss" => width.min(height),
            "ls" => width.max(height),
            _ => return self.divided_variable(name).or_else(|| angle_constant(name)),
        })
    }

    /// The `wd`*N* / `hd`*N* / `ssd`*N* family — a whole fraction of a shape dimension.
    fn divided_variable(self, name: &str) -> Option<f64> {
        let (width, height) = (self.width, self.height);
        for (prefix, numerator) in [("wd", width), ("hd", height), ("ssd", width.min(height))] {
            if let Some(rest) = name.strip_prefix(prefix) {
                return divisor(rest).map(|divisor| numerator / divisor);
            }
        }
        None
    }
}

/// The circle constants — *M*`cd`*N* and `cd`*N*, a fraction of a full turn in 60000ths of a degree.
///
/// These are the only built-ins that do not depend on the shape's size, which is why they resolve
/// even in a [`ResolvedGuides`] built without one.
fn angle_constant(name: &str) -> Option<f64> {
    let digits = name
        .as_bytes()
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    let (multiplier, rest) = name.split_at(digits);
    let multiplier = if multiplier.is_empty() {
        1.0
    } else {
        multiplier.parse::<u32>().ok()? as f64
    };
    let denominator = divisor(rest.strip_prefix("cd")?)?;
    Some(FULL_TURN * multiplier / denominator)
}

/// A positive whole divisor written as ASCII digits, or `None` (an empty or zero or non-numeric one).
fn divisor(text: &str) -> Option<f64> {
    let value = text.parse::<u32>().ok()?;
    if value == 0 || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(f64::from(value))
}

/// The values a shape's guides evaluate to — the environment a coordinate or angle naming a guide is
/// resolved against.
///
/// Built by [`evaluate`](Self::evaluate) from a guide list in declaration order (see the
/// [module docs](self) for why that order is the whole cycle defence). Lookups fall back to the
/// built-in variables of the [`GuideContext`] it was given.
///
/// The names borrow from the guide list, so evaluating a shape's guides allocates one hash map and
/// nothing per guide — a [`Cow`] rather than a `&str` only because a guide name read out of a file
/// may carry a character reference, which decodes to a fresh string; every other caller binds a
/// borrowed name and allocates nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedGuides<'a> {
    context: Option<GuideContext>,
    values: HashMap<Cow<'a, str>, f64>,
}

impl<'a> ResolvedGuides<'a> {
    /// An empty environment over a shape size: every built-in resolves, no guide does yet.
    #[must_use]
    pub fn new(context: GuideContext) -> Self {
        Self {
            context: Some(context),
            values: HashMap::new(),
        }
    }

    /// An empty environment with **no shape size**: `w`, `h` and everything derived from them are
    /// undefined, and only the size-independent circle constants (`cd4`, `3cd4`, …) resolve.
    ///
    /// This is what an adjust-value list is read in when the caller has not said how big the shape is
    /// — an `a:avLst` holds literal seeds (`val 25000`), so it almost never needs one.
    #[must_use]
    pub fn without_size() -> Self {
        Self {
            context: None,
            values: HashMap::new(),
        }
    }

    /// Evaluates a guide list in declaration order, each formula seeing the guides before it.
    ///
    /// # Errors
    ///
    /// [`GuideError::Guide`] naming the first guide that failed, wrapping why.
    pub fn evaluate<I>(guides: I, context: GuideContext) -> Result<Self, GuideError>
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let mut resolved = Self::new(context);
        resolved.extend(guides)?;
        Ok(resolved)
    }

    /// Evaluates more guides into this environment, in declaration order — the second half of an
    /// `avLst` then `gdLst` pair, say. A guide that repeats an earlier name replaces it from here on.
    ///
    /// # Errors
    ///
    /// [`GuideError::Guide`] naming the first guide that failed, wrapping why.
    pub fn extend<I>(&mut self, guides: I) -> Result<(), GuideError>
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        for (name, formula) in guides {
            let value = self
                .evaluate_formula(formula)
                .map_err(|source| GuideError::Guide {
                    guide: name.to_owned(),
                    source: Box::new(source),
                })?;
            self.values.insert(Cow::Borrowed(name), value);
        }
        Ok(())
    }

    /// Evaluates one formula in this environment, without binding it to a name.
    ///
    /// # Errors
    ///
    /// [`GuideError::Malformed`] if the formula does not parse, or the failure
    /// [`GuideFormula::evaluate`] reports.
    pub fn evaluate_formula(&self, formula: &str) -> Result<f64, GuideError> {
        let parsed = GuideFormula::parse(formula).map_err(|source| GuideError::Malformed {
            formula: formula.to_owned(),
            source,
        })?;
        parsed.evaluate(self)
    }

    /// Binds `name` to `value` directly, as an already-computed guide — how a preset shape's current
    /// adjustment values are seeded before its `gdLst` is evaluated.
    pub fn define(&mut self, name: impl Into<Cow<'a, str>>, value: f64) {
        self.values.insert(name.into(), value);
    }

    /// The value of a guide, or of a built-in variable, or `None` if the name is neither.
    #[must_use]
    pub fn value(&self, name: &str) -> Option<f64> {
        if let Some(value) = self.values.get(name) {
            return Some(*value);
        }
        match self.context {
            Some(context) => context.variable(name),
            None => angle_constant(name),
        }
    }

    /// The value of a guide **this environment evaluated**, ignoring the built-in variables — the
    /// lookup an adjustment name wants, where "the file said so" and "the format always says so" must
    /// not be confused.
    #[must_use]
    pub fn guide(&self, name: &str) -> Option<f64> {
        self.values.get(name).copied()
    }

    /// The value of a guide or built-in.
    ///
    /// # Errors
    ///
    /// [`GuideError::UndefinedGuide`] if the name resolves to nothing.
    pub fn resolve(&self, name: &str) -> Result<f64, GuideError> {
        self.value(name).ok_or_else(|| GuideError::UndefinedGuide {
            name: name.to_owned(),
        })
    }

    /// The shape size the built-ins come from, or `None` for an environment built
    /// [`without_size`](Self::without_size).
    #[must_use]
    pub fn context(&self) -> Option<GuideContext> {
        self.context
    }

    /// How many guides have been evaluated (built-ins are not counted).
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether no guide has been evaluated yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Every evaluated guide, as `(name, value)`, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, f64)> + '_ {
        self.values
            .iter()
            .map(|(name, value)| (name.as_ref(), *value))
    }
}

/// A resolved guide value read as a length in EMU, rounded to the nearest whole unit (and saturating
/// rather than wrapping at the extremes of `i64`).
pub(crate) fn value_as_emu(value: f64) -> Emu {
    Emu::from_emu(value.round() as i64)
}

/// A resolved guide value read as an angle — the wire scale is 60000ths of a degree.
pub(crate) fn value_as_angle(value: f64) -> Angle {
    Angle::from_degrees(value / ANGLE_UNITS_PER_DEGREE)
}
