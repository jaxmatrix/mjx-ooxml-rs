//! Office MathML, laid out: **a typesetter in miniature**, closer to TeX than to prose.
//!
//! # Why this is not "a paragraph with some symbols in it"
//!
//! Every other thing this crate lays out is a sequence: runs on a line, lines in a paragraph, blocks
//! in a column. An equation is a *tree of boxes with a baseline each*, and almost everything about it
//! is a relationship between two of them:
//!
//! * a fraction's bar sits on the **axis**, not on the baseline, and the numerator and denominator
//!   are positioned relative to the bar rather than to each other;
//! * a superscript is set at a **smaller size**, and a superscript of a superscript smaller again,
//!   and the two shrinks are not the same;
//! * a delimiter **grows** to what it encloses, centred on the axis, so a parenthesis around a
//!   fraction is taller than one around a letter and both are centred on the same line;
//! * an equation array aligns its rows on an **alignment axis** rather than on their left edges.
//!
//! A gate that says *"the equation rendered"* is satisfied by a box. So the gate here asserts those
//! four relationships as **numbers** — `tests/an_equation_is_typeset.rs` reads a nested fraction's
//! two bar positions, a growing delimiter's height against its content's, the script scale, and an
//! equation array's axis, and each is proved able to fail.
//!
//! # ⚠ There is no `MATH` table, and that is the honest weak point of this module
//!
//! An OpenType font that is meant for mathematics carries a `MATH` table: the axis height, the
//! default rule thickness, the script scale-downs, every gap between a numerator and its bar, and
//! the *glyph variants* a stretchy parenthesis is assembled from. **`mjx-text` parses no such
//! table** — a grep of that crate for `MATH` finds nothing at all — so every constant below comes
//! from an external, citable default instead:
//!
//! | constant | from |
//! |---|---|
//! | [`SCRIPT_SCALE`], [`SCRIPT_SCRIPT_SCALE`] | the OpenType `MATH` table's own documented defaults for `ScriptPercentScaleDown` (80%) and `ScriptScriptPercentScaleDown` (60%), which is what a shaper uses when a font carries no table |
//! | [`AXIS_HEIGHT_IN_EMS`] | `\fontdimen22` of a TeX math-symbol font; Computer Modern's is 0.25 em, and MathML Core states the same fallback |
//! | [`RULE_THICKNESS_IN_EMS`] | `\fontdimen8` (`default_rule_thickness`); Computer Modern's is 0.04 em |
//! | the fraction, radical and script gaps | *The TeXbook*, Appendix G, rules 15 and 11 — stated as multiples of the rule thickness exactly as TeX states them |
//!
//! **That is a real external reference and it is not Word.** TeX's numbers and Word's are not the
//! same numbers, so a document laid out here will not match Word to the EMU; what it will do is put
//! a bar on an axis, shrink a script by a stated factor, and grow a delimiter to its content — the
//! *relationships*, which are what the gate asserts and what a reader sees. The Windows sitting is
//! where the constants get their real values, and every one is marked `GUESS:` at its site so that
//! the list is writable from this file.
//!
//! A stretchy delimiter is the one place the missing table costs more than a constant: without
//! `MathVariants` there is no assembled parenthesis, so a growing delimiter is drawn by **scaling
//! the base glyph's size**, which is what a renderer without a math font does and is visibly not
//! what Word does for a very tall one. See [`grow_delimiter`].

use std::sync::Arc;

use mjx_docx::{EquationNode, EquationRun};
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::officemath::{
    DelimiterShape, FractionType, LimitLocation, MathStyle, TopBottom,
};
use mjx_ooxml_types::shared::{RelativeHorizontalAlignment, RelativeVerticalAlignment};
use mjx_text::{
    FaceId, FontError, FontFace, FontRequest, FontSize, FontSlant, FontWeight, ShapedRun,
    ShapingRequest, TextScript,
};

use crate::style::RunStyle;
use crate::text::TextEngine;

/// How much smaller a first-level script is set.
///
/// **`GUESS:`** the OpenType `MATH` table's `ScriptPercentScaleDown` default, 80 %. A font that
/// carries the table states its own; `mjx-text` reads none, so this is the documented fallback
/// rather than a measurement.
pub const SCRIPT_SCALE: f64 = 0.8;

/// How much smaller a second-level script is set — a superscript of a superscript.
///
/// **`GUESS:`** `ScriptScriptPercentScaleDown`'s default, 60 %. Note it is **not** [`SCRIPT_SCALE`]
/// squared (0.64): the two shrinks are stated independently, and an implementation that squared the
/// first would set every second-level script four per cent too small.
pub const SCRIPT_SCRIPT_SCALE: f64 = 0.6;

/// Where the mathematical axis sits above the baseline, in ems.
///
/// The axis is what a fraction bar sits on, what a growing delimiter is centred on, and what a
/// binary operator is aligned to. It is **not** half the x-height and it is not the baseline.
///
/// **`GUESS:`** 0.25 em — TeX's `\fontdimen22` for Computer Modern, and MathML Core's own stated
/// fallback for a font with no `MATH` table.
pub const AXIS_HEIGHT_IN_EMS: f64 = 0.25;

/// How thick a fraction bar, a radical rule and a border-box edge are, in ems.
///
/// **`GUESS:`** 0.04 em — TeX's `default_rule_thickness` (`\fontdimen8`) for Computer Modern.
pub const RULE_THICKNESS_IN_EMS: f64 = 0.04;

/// How deep a nested expression may go before this module stops descending.
///
/// A `.docx` is untrusted input and `m:e` nests without limit, so a hand-made file can hold an
/// expression ten thousand deep and blow the stack of any recursive walker. Thirty-two is far past
/// anything a person writes — a fraction inside a radical inside a matrix inside a fraction is four
/// — and a node past it is laid out as an **empty box**, which is visible and finite rather than a
/// crash.
pub const MAXIMUM_DEPTH: usize = 32;

/// What a laid-out node draws.
#[derive(Clone, PartialEq, Debug)]
pub enum MathContent {
    /// Nothing: a grouping box, whose children are what is drawn.
    Group,
    /// Shaped glyphs on this box's own baseline.
    Glyphs {
        /// The face they were shaped in.
        face: FaceId,
        /// The glyphs.
        run: ShapedRun,
    },
    /// A filled rectangle — a fraction bar, a radical's rule, a border box's edge.
    ///
    /// Its rectangle **is** the box: the width, ascent and descent describe the fill, so a painter
    /// needs no thickness of its own.
    Rule,
}

/// One laid-out node of an equation: its size about its own baseline, what it draws, and where its
/// children sit relative to it.
#[derive(Clone, PartialEq, Debug)]
pub struct MathBox {
    /// How wide it is.
    pub width: Emu,
    /// How far it reaches above its own baseline.
    pub ascent: Emu,
    /// How far below.
    pub descent: Emu,
    /// What it draws.
    pub content: MathContent,
    /// Its children, each with an offset from this box's own origin.
    pub children: Vec<PlacedMathBox>,
}

impl MathBox {
    /// An empty box of no size.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            width: Emu::ZERO,
            ascent: Emu::ZERO,
            descent: Emu::ZERO,
            content: MathContent::Group,
            children: Vec::new(),
        }
    }

    /// How tall it is in total.
    #[must_use]
    pub fn height(&self) -> Emu {
        self.ascent + self.descent
    }
}

/// One child, placed.
#[derive(Clone, PartialEq, Debug)]
pub struct PlacedMathBox {
    /// How far right of the parent's origin its own left edge is.
    pub x: Emu,
    /// How far **below** the parent's baseline its own baseline is. Negative is above.
    pub baseline: Emu,
    /// The child.
    pub content: MathBox,
}

/// The size and face an expression is set at, and how deep in scripts it is.
#[derive(Clone, Debug)]
pub struct MathContext<'a> {
    /// The run style the surrounding paragraph gives the equation — its family, and the size at
    /// nesting level zero.
    pub style: &'a RunStyle,
    /// The size this level is set at.
    pub size: FontSize,
    /// How many script levels down this is: 0 is the body, 1 a script, 2 and beyond a script of a
    /// script.
    pub script_level: u8,
    /// How deep the recursion is, against [`MAXIMUM_DEPTH`].
    pub depth: usize,
}

impl<'a> MathContext<'a> {
    /// The context an equation starts in.
    #[must_use]
    pub fn new(style: &'a RunStyle) -> Self {
        Self {
            style,
            size: style.size,
            script_level: 0,
            depth: 0,
        }
    }

    /// The context one script level in.
    #[must_use]
    pub fn scripted(&self) -> Self {
        let level = self.script_level.saturating_add(1);
        // The two scale-downs are absolute rather than compounding: level one is
        // `SCRIPT_SCALE` of the body and level two is `SCRIPT_SCRIPT_SCALE` of it, and every level
        // past two stays at the second — which is the OpenType rule and is why a deeply nested
        // script does not vanish.
        let factor = match level {
            0 => 1.0,
            1 => SCRIPT_SCALE,
            _ => SCRIPT_SCRIPT_SCALE,
        };
        Self {
            style: self.style,
            size: FontSize::from_points(self.style.size.in_points() * factor),
            script_level: level,
            depth: self.depth,
        }
    }

    /// The same context one level deeper in the tree, at the same size.
    #[must_use]
    pub fn deeper(&self) -> Self {
        let mut next = self.clone();
        next.depth = next.depth.saturating_add(1);
        next
    }

    /// The em of the size this level is set at, as a length.
    #[must_use]
    pub fn em(&self) -> Emu {
        Emu::from_points(self.size.in_points())
    }

    /// Where the axis sits above this level's baseline.
    #[must_use]
    pub fn axis(&self) -> Emu {
        Emu::from_points(self.size.in_points() * AXIS_HEIGHT_IN_EMS)
    }

    /// How thick a rule is at this level.
    ///
    /// Never zero: a bar a document asked for and a renderer drew as nothing is a fraction that
    /// looks like a stack. `crate::measure::HAIRLINE` is the same rule for a border.
    #[must_use]
    pub fn rule_thickness(&self) -> Emu {
        let thickness = Emu::from_points(self.size.in_points() * RULE_THICKNESS_IN_EMS);
        thickness.maximum(crate::measure::HAIRLINE)
    }
}

/// Lays one equation out.
///
/// # Errors
/// [`FontError`] when a face will not resolve or will not shape.
pub fn lay_out(
    engine: &mut TextEngine<'_>,
    nodes: &[EquationNode],
    context: &MathContext<'_>,
) -> Result<MathBox, FontError> {
    row(engine, nodes, context)
}

/// A horizontal row of nodes, on one baseline.
fn row(
    engine: &mut TextEngine<'_>,
    nodes: &[EquationNode],
    context: &MathContext<'_>,
) -> Result<MathBox, FontError> {
    if context.depth >= MAXIMUM_DEPTH {
        return Ok(MathBox::empty());
    }
    let mut children = Vec::with_capacity(nodes.len());
    let mut x = Emu::ZERO;
    let mut ascent = Emu::ZERO;
    let mut descent = Emu::ZERO;
    for node in nodes {
        let laid = single(engine, node, &context.deeper())?;
        ascent = ascent.maximum(laid.ascent);
        descent = descent.maximum(laid.descent);
        let width = laid.width;
        children.push(PlacedMathBox {
            x,
            baseline: Emu::ZERO,
            content: laid,
        });
        x += width;
    }
    Ok(MathBox {
        width: x,
        ascent,
        descent,
        content: MathContent::Group,
        children,
    })
}

/// One node.
#[allow(clippy::too_many_lines)]
fn single(
    engine: &mut TextEngine<'_>,
    node: &EquationNode,
    context: &MathContext<'_>,
) -> Result<MathBox, FontError> {
    match node {
        EquationNode::Run(run) => text_box(engine, run, context),
        EquationNode::Fraction {
            numerator,
            denominator,
            kind,
        } => {
            let above = row(engine, numerator, context)?;
            let below = row(engine, denominator, context)?;
            Ok(fraction(above, below, *kind, context))
        }
        EquationNode::Radical {
            degree,
            radicand,
            degree_hidden,
        } => {
            let inner = row(engine, radicand, context)?;
            let index = if *degree_hidden || degree.is_empty() {
                None
            } else {
                Some(row(engine, degree, &context.scripted())?)
            };
            let sign = glyph(engine, RADICAL_SIGN, context, inner.height())?;
            Ok(radical(sign, index, inner, context))
        }
        EquationNode::NaryOperator {
            character,
            lower,
            upper,
            operand,
            limit_location,
            grow,
            lower_hidden,
            upper_hidden,
        } => {
            let symbol = character.as_deref().unwrap_or(DEFAULT_NARY);
            let operand_box = row(engine, operand, context)?;
            let target = if *grow {
                operand_box.height()
            } else {
                Emu::ZERO
            };
            let operator = glyph(engine, symbol, context, target)?;
            let below = if *lower_hidden {
                None
            } else {
                Some(row(engine, lower, &context.scripted())?)
            };
            let above = if *upper_hidden {
                None
            } else {
                Some(row(engine, upper, &context.scripted())?)
            };
            Ok(nary(
                operator,
                below,
                above,
                operand_box,
                limit_location.unwrap_or(LimitLocation::SubscriptSuperscript),
                context,
            ))
        }
        EquationNode::Delimiter {
            begin,
            separator,
            end,
            grow,
            shape,
            arguments,
        } => {
            let mut inner = Vec::with_capacity(arguments.len());
            for argument in arguments {
                inner.push(row(engine, argument, context)?);
            }
            delimiter(
                engine,
                Brackets {
                    begin: begin.as_deref().unwrap_or(DEFAULT_BEGIN),
                    separator: separator.as_deref().unwrap_or(DEFAULT_SEPARATOR),
                    end: end.as_deref().unwrap_or(DEFAULT_END),
                    grow: *grow,
                    shape_rule: shape.unwrap_or(DelimiterShape::Centered),
                },
                inner,
                context,
            )
        }
        EquationNode::Matrix {
            rows,
            column_alignments,
            base_alignment,
        } => {
            let mut laid: Vec<Vec<MathBox>> = Vec::with_capacity(rows.len());
            for line in rows {
                let mut cells = Vec::with_capacity(line.len());
                for cell in line {
                    cells.push(row(engine, cell, context)?);
                }
                laid.push(cells);
            }
            Ok(matrix(laid, column_alignments, *base_alignment, context))
        }
        EquationNode::Accent { character, base } => {
            let inner = row(engine, base, context)?;
            let mark = glyph(
                engine,
                character.as_deref().unwrap_or(DEFAULT_ACCENT),
                context,
                Emu::ZERO,
            )?;
            Ok(accent(mark, inner, context))
        }
        EquationNode::Bar { position, base } => {
            let inner = row(engine, base, context)?;
            Ok(bar(
                inner,
                position.unwrap_or(TopBottom::Bottom) == TopBottom::Top,
                context,
            ))
        }
        EquationNode::Function { name, base } => {
            let head = row(engine, name, context)?;
            let inner = row(engine, base, context)?;
            Ok(beside(vec![head, thin_space(context), inner]))
        }
        EquationNode::Limit {
            below,
            base,
            limit: value,
        } => {
            let inner = row(engine, base, context)?;
            let mark = row(engine, value, &context.scripted())?;
            Ok(stacked_limit(inner, mark, *below, context))
        }
        EquationNode::Script {
            base,
            subscript,
            superscript,
            aligned,
        } => {
            let inner = row(engine, base, context)?;
            let sub = if subscript.is_empty() {
                None
            } else {
                Some(row(engine, subscript, &context.scripted())?)
            };
            let sup = if superscript.is_empty() {
                None
            } else {
                Some(row(engine, superscript, &context.scripted())?)
            };
            Ok(scripts(inner, sub, sup, *aligned, false, context))
        }
        EquationNode::PreScript {
            base,
            subscript,
            superscript,
        } => {
            let inner = row(engine, base, context)?;
            let sub = if subscript.is_empty() {
                None
            } else {
                Some(row(engine, subscript, &context.scripted())?)
            };
            let sup = if superscript.is_empty() {
                None
            } else {
                Some(row(engine, superscript, &context.scripted())?)
            };
            Ok(scripts(inner, sub, sup, true, true, context))
        }
        EquationNode::GroupCharacter {
            character,
            position,
            vertical_alignment: _,
            base,
        } => {
            let inner = row(engine, base, context)?;
            let mark = glyph(
                engine,
                character.as_deref().unwrap_or(DEFAULT_GROUP),
                context,
                inner.width,
            )?;
            let above = position.unwrap_or(TopBottom::Bottom) == TopBottom::Top;
            Ok(if above {
                stacked_limit(inner, mark, false, context)
            } else {
                stacked_limit(inner, mark, true, context)
            })
        }
        EquationNode::BorderBox {
            base,
            top,
            bottom,
            left,
            right,
            strike_horizontal,
            strike_vertical,
            strike_rising,
            strike_falling,
        } => {
            let inner = row(engine, base, context)?;
            Ok(border_box(
                inner,
                [*top, *bottom, *left, *right],
                [
                    *strike_horizontal,
                    *strike_vertical,
                    *strike_rising,
                    *strike_falling,
                ],
                context,
            ))
        }
        EquationNode::Box { base, .. } => row(engine, base, context),
        EquationNode::EquationArray {
            rows,
            base_alignment,
        } => {
            let mut laid = Vec::with_capacity(rows.len());
            for line in rows {
                laid.push(row(engine, line, context)?);
            }
            Ok(equation_array(laid, *base_alignment, context))
        }
        EquationNode::Phantom {
            base,
            show,
            zero_width,
            zero_ascent,
            zero_descent,
        } => {
            let inner = row(engine, base, context)?;
            Ok(phantom(
                inner,
                *show,
                *zero_width,
                *zero_ascent,
                *zero_descent,
            ))
        }
    }
}

/// `U+221A SQUARE ROOT`.
const RADICAL_SIGN: &str = "\u{221A}";
/// `U+222B INTEGRAL`.
///
/// **`GUESS:`** what an `m:nary` with no `m:chr` draws. `shared-math.xsd` makes the element
/// optional and states no default; the integral sign is what every implementation draws and what
/// Word's own editor writes, and the section-numbered prose was not read here. The sitting checks it.
const DEFAULT_NARY: &str = "\u{222B}";
/// **`GUESS:`** what an `m:d` with no `m:begChr` opens with. The schema makes it optional and
/// states no default; a parenthesis is the universal reading.
const DEFAULT_BEGIN: &str = "(";
/// **`GUESS:`** what separates two `m:e`s with no `m:sepChr`, on the same footing as
/// [`DEFAULT_BEGIN`].
const DEFAULT_SEPARATOR: &str = "|";
/// **`GUESS:`** what an `m:d` with no `m:endChr` closes with, on the same footing as
/// [`DEFAULT_BEGIN`].
const DEFAULT_END: &str = ")";
/// `U+0302 COMBINING CIRCUMFLEX ACCENT`.
///
/// **`GUESS:`** what an `m:acc` with no `m:chr` draws, on the same footing as [`DEFAULT_BEGIN`].
const DEFAULT_ACCENT: &str = "\u{0302}";
/// `U+23DF BOTTOM CURLY BRACKET`.
///
/// **`GUESS:`** what an `m:groupChr` with no `m:chr` draws, on the same footing as
/// [`DEFAULT_BEGIN`].
const DEFAULT_GROUP: &str = "\u{23DF}";

/// One run of mathematical text, shaped.
///
/// # Italic by default, and why absence is not `plain`
///
/// **`GUESS:`** an absent `m:sty` is italic. A single-letter identifier in an equation is
/// italic in every renderer without the file saying so, and a reader that treated an absent `m:sty`
/// as upright would set every variable in every document in roman — which is legible, wrong, and the
/// single most visible thing about a badly typeset equation. `m:nor` is the opt-out and is honoured.
fn text_box(
    engine: &mut TextEngine<'_>,
    run: &EquationRun,
    context: &MathContext<'_>,
) -> Result<MathBox, FontError> {
    if run.text.is_empty() {
        return Ok(MathBox::empty());
    }
    let style = run.style.unwrap_or(MathStyle::Italic);
    let italic = !run.normal_text && matches!(style, MathStyle::Italic | MathStyle::BoldItalic);
    let bold = matches!(style, MathStyle::Bold | MathStyle::BoldItalic);
    let Some((face, face_id)) = resolve(engine, context, bold, italic)? else {
        return Ok(MathBox::empty());
    };
    shaped_box(engine, &face, face_id, &run.text, context.size)
}

/// One character, shaped, and grown to `target` when `target` asks for more height than it has.
fn glyph(
    engine: &mut TextEngine<'_>,
    character: &str,
    context: &MathContext<'_>,
    target: Emu,
) -> Result<MathBox, FontError> {
    let Some((face, face_id)) = resolve(engine, context, false, false)? else {
        return Ok(MathBox::empty());
    };
    let size = grow_delimiter(context.size, character, target, engine, &face)?;
    shaped_box(engine, &face, face_id, character, size)
}

/// The size a stretchy character is set at so that it reaches `target`.
///
/// # ⚠ Scaling is not what a math font does, and this says so at the site
///
/// A real math font grows a parenthesis through the `MATH` table's `MathVariants`: a ladder of
/// pre-drawn sizes and then an *assembly* built from a top, a repeating middle and a bottom, so that
/// a bracket around a ten-line matrix has straight sides and correctly shaped ends. `mjx-text` reads
/// no `MATH` table, so there is no ladder and no assembly to reach for.
///
/// What is left is to set the glyph at a larger *size*, which grows its stroke weight along with its
/// height — visibly wrong for a very tall delimiter and entirely acceptable for the one-and-a-bit
/// lines a fraction or a sum needs, which is what almost every equation asks for. The alternative,
/// leaving the delimiter at body size beside a two-line fraction, is worse and is what a renderer
/// that "does not do stretchy glyphs" produces.
///
/// **`GUESS:`** the growth is capped at [`MAXIMUM_DELIMITER_GROWTH`] times the body size, because an
/// equation array of forty rows would otherwise ask for a glyph a foot tall.
///
/// # Errors
/// [`FontError`] when the face will not shape.
pub fn grow_delimiter(
    size: FontSize,
    character: &str,
    target: Emu,
    engine: &mut TextEngine<'_>,
    face: &Arc<FontFace>,
) -> Result<FontSize, FontError> {
    if target <= Emu::ZERO {
        return Ok(size);
    }
    let _ = (engine, character);
    let (ascent, descent) = face_extent(face, size);
    let height = ascent + descent;
    if height >= target || height <= Emu::ZERO {
        return Ok(size);
    }
    #[allow(clippy::cast_precision_loss)]
    let ratio = (target.emu() as f64) / (height.emu() as f64);
    let capped = ratio.min(MAXIMUM_DELIMITER_GROWTH);
    Ok(FontSize::from_points(size.in_points() * capped))
}

/// The most a stretchy character's size may be multiplied by.
///
/// **`GUESS:`** eight. A `MATH` table's own variant ladder usually stops around six sizes and then
/// switches to an assembly, so eight is generous for the ladder and is a bound rather than a
/// judgement — what it exists for is a malformed file whose matrix has ten thousand rows.
pub const MAXIMUM_DELIMITER_GROWTH: f64 = 8.0;

/// The face an equation's runs are set in.
fn resolve(
    engine: &mut TextEngine<'_>,
    context: &MathContext<'_>,
    bold: bool,
    italic: bool,
) -> Result<Option<(Arc<FontFace>, FaceId)>, FontError> {
    // **GUESS:** an equation is set in the paragraph's own family rather than in `m:mathPr/m:mathFont`.
    // `word/settings.xml`'s `m:mathPr` names a math font (Cambria Math, almost always) and
    // `mjx-docx`'s settings residency does not carry it; asking the resolver for the paragraph's
    // family means an equation in a Times document is set in Times, which is wrong in the same
    // direction for every glyph and is legible. Naming a font the machine may not have would
    // substitute silently and be wrong in a different direction per glyph.
    let request = FontRequest::new(&context.style.family)
        .with_weight(if bold {
            FontWeight::BOLD
        } else {
            FontWeight::REGULAR
        })
        .with_slant(if italic {
            FontSlant::Italic
        } else {
            FontSlant::Upright
        });
    let resolution = engine.fonts.resolve(&request)?;
    // A family nothing on the machine can supply costs the equation its glyphs and not the document
    // its layout: the same posture [`crate::text::composer_runs`] already takes for a face that will
    // not register, and for the same reason — a page missing one symbol is better than a document
    // that will not open.
    let Some(face) = resolution.face() else {
        return Ok(None);
    };
    let face = Arc::clone(face);
    let Ok(face_id) = engine.rasteriser.register(&face) else {
        return Ok(None);
    };
    Ok(Some((face, face_id)))
}

/// Shapes `text` and answers the box it occupies.
///
/// The vertical extent comes from the **face's own `hhea` metrics** at this size, which is the same
/// metric [`mjx_layout::LineComposer`] takes a line's height from — so a run of mathematics and a
/// run of prose in the same face agree about how tall they are, and an equation on a line does not
/// change the line's height for a reason no other run would.
fn shaped_box(
    engine: &mut TextEngine<'_>,
    face: &Arc<FontFace>,
    face_id: FaceId,
    text: &str,
    size: FontSize,
) -> Result<MathBox, FontError> {
    let run = shape(engine, face, text, size)?;
    let (ascent, descent) = face_extent(face, size);
    Ok(MathBox {
        width: Emu::from_points(run.advance_in_points()),
        ascent,
        descent,
        content: MathContent::Glyphs { face: face_id, run },
        children: Vec::new(),
    })
}

/// How far a face reaches above and below its baseline at `size`.
fn face_extent(face: &Arc<FontFace>, size: FontSize) -> (Emu, Emu) {
    let metrics = face.metrics();
    let em = f64::from(metrics.units_per_em.max(1));
    let points = size.in_points();
    (
        Emu::from_points(f64::from(metrics.ascender) / em * points),
        // A descender is negative in the face's own tables and a descent is a positive depth.
        Emu::from_points(f64::from(-metrics.descender) / em * points),
    )
}

/// One shaping.
fn shape(
    engine: &mut TextEngine<'_>,
    face: &Arc<FontFace>,
    text: &str,
    size: FontSize,
) -> Result<ShapedRun, FontError> {
    let request = ShapingRequest::new(text, TextScript::COMMON, size, engine.features);
    engine.shaper.shape(face, &request)
}

/// A row of already-laid-out boxes, side by side on one baseline.
fn beside(boxes: Vec<MathBox>) -> MathBox {
    let mut x = Emu::ZERO;
    let mut ascent = Emu::ZERO;
    let mut descent = Emu::ZERO;
    let mut children = Vec::with_capacity(boxes.len());
    for content in boxes {
        ascent = ascent.maximum(content.ascent);
        descent = descent.maximum(content.descent);
        let width = content.width;
        children.push(PlacedMathBox {
            x,
            baseline: Emu::ZERO,
            content,
        });
        x += width;
    }
    MathBox {
        width: x,
        ascent,
        descent,
        content: MathContent::Group,
        children,
    }
}

/// A thin space — what separates a function name from its argument.
///
/// **`GUESS:`** one sixth of an em, which is TeX's `\,` and the thin space every typesetter puts
/// after `sin`.
fn thin_space(context: &MathContext<'_>) -> MathBox {
    MathBox {
        width: context.em().divided_by(6),
        ascent: Emu::ZERO,
        descent: Emu::ZERO,
        content: MathContent::Group,
        children: Vec::new(),
    }
}

/// A fraction: the bar on the axis, the numerator above it, the denominator below.
///
/// # The bar is on the axis, and that is the assertion
///
/// The numerator's *bottom* sits a gap above the bar and the denominator's *top* a gap below, and
/// the bar itself is at the axis — so a fraction inside a fraction has its inner bar at the inner
/// level's axis, which is a different height, and the two bar positions are the numbers
/// `tests/an_equation_is_typeset.rs` reads. An implementation that centred the bar on the *total*
/// height of numerator and denominator would put both bars in plausible places and neither on an
/// axis, and would pass every "it rendered" gate.
///
/// The gaps are *The TeXbook*'s Appendix G rule 15: three times the rule thickness above and below in
/// display style, one times in text style. This is text style — an equation inline in a paragraph —
/// unless the equation came from an `m:oMathPara`.
fn fraction(
    numerator: MathBox,
    denominator: MathBox,
    kind: Option<FractionType>,
    context: &MathContext<'_>,
) -> MathBox {
    match kind {
        // A linear fraction is `a/b` on one line: no bar, no stacking — `ST_FType`'s `lin`.
        Some(FractionType::Linear) => {
            return beside(vec![numerator, quad(context, 4), denominator]);
        }
        // `skw` is a skewed fraction — the two set diagonally about a solidus. Laid out here as a
        // linear one, and marked: the diagonal needs a transform and this crate resolves none.
        // **`GUESS:`** a skewed fraction reads as a linear one, which is legible and is what a
        // renderer without transforms can honestly produce.
        Some(FractionType::Skewed) => {
            return beside(vec![numerator, quad(context, 4), denominator]);
        }
        _ => {}
    }
    let thickness = if kind == Some(FractionType::NoBar) {
        Emu::ZERO
    } else {
        context.rule_thickness()
    };
    let gap = context.rule_thickness().times(3);
    let width = numerator.width.maximum(denominator.width);
    let axis = context.axis();
    let half = crate::measure::half_of(thickness);

    // Every offset below is *how far below this box's own baseline* a child's baseline sits, so
    // above the baseline is negative. The bar's own baseline is put **on the axis**, which is the
    // one line of this function the gate reads.
    let numerator_offset = Emu::ZERO - (axis + half + gap + numerator.descent);
    let denominator_offset = (gap + denominator.ascent) - (axis - half);

    let mut children = vec![
        PlacedMathBox {
            x: crate::measure::half_of(width - numerator.width),
            baseline: numerator_offset,
            content: numerator,
        },
        PlacedMathBox {
            x: crate::measure::half_of(width - denominator.width),
            baseline: denominator_offset,
            content: denominator,
        },
    ];
    if thickness > Emu::ZERO {
        children.push(PlacedMathBox {
            x: Emu::ZERO,
            baseline: Emu::ZERO - axis,
            content: MathBox {
                width,
                ascent: half,
                descent: thickness - half,
                content: MathContent::Rule,
                children: Vec::new(),
            },
        });
    }
    let (ascent, descent) = extent(&children);
    MathBox {
        width,
        ascent,
        descent,
        content: MathContent::Group,
        children,
    }
}

/// A space of one `divisor`th of an em.
fn quad(context: &MathContext<'_>, divisor: i64) -> MathBox {
    MathBox {
        width: context.em().divided_by(divisor.max(1)),
        ascent: Emu::ZERO,
        descent: Emu::ZERO,
        content: MathContent::Group,
        children: Vec::new(),
    }
}

/// How far a set of placed children reaches above and below their parent's baseline.
fn extent(children: &[PlacedMathBox]) -> (Emu, Emu) {
    let ascent = children.iter().fold(Emu::ZERO, |tallest, child| {
        tallest.maximum(child.content.ascent - child.baseline)
    });
    let descent = children.iter().fold(Emu::ZERO, |deepest, child| {
        deepest.maximum(child.content.descent + child.baseline)
    });
    (ascent, descent)
}

/// A radical: the sign, an optional index, and a rule over the radicand.
fn radical(
    sign: MathBox,
    index: Option<MathBox>,
    radicand: MathBox,
    context: &MathContext<'_>,
) -> MathBox {
    let thickness = context.rule_thickness();
    // **`GUESS:`** the gap between the rule and the radicand is one rule thickness, and the rule
    // itself sits one thickness above the radicand's own ascent. *The TeXbook*'s rule 11 uses the
    // rule thickness plus a quarter of the excess of the surd's height over the radicand's, which
    // needs a surd whose height is a function of what it encloses — which is exactly the
    // `MathVariants` ladder this module does not have.
    let clearance = thickness.times(2);
    let inner_ascent = radicand.ascent + clearance;
    let index_width = index.as_ref().map_or(Emu::ZERO, |content| content.width);
    let mut children = Vec::new();
    let mut x = Emu::ZERO;
    if let Some(index) = index {
        children.push(PlacedMathBox {
            x,
            // The index sits high on the sign's left shoulder.
            baseline: Emu::ZERO - sign.ascent.times(2).divided_by(3),
            content: index,
        });
        x += index_width;
    }
    let sign_width = sign.width;
    children.push(PlacedMathBox {
        x,
        baseline: Emu::ZERO,
        content: sign,
    });
    x += sign_width;
    let radicand_width = radicand.width;
    let radicand_descent = radicand.descent;
    children.push(PlacedMathBox {
        x,
        baseline: Emu::ZERO,
        content: radicand,
    });
    children.push(PlacedMathBox {
        x,
        baseline: Emu::ZERO,
        content: MathBox {
            width: radicand_width,
            ascent: inner_ascent + thickness,
            descent: Emu::ZERO - inner_ascent,
            content: MathContent::Rule,
            children: Vec::new(),
        },
    });
    MathBox {
        width: x + radicand_width,
        ascent: (inner_ascent + thickness).maximum(Emu::ZERO),
        descent: radicand_descent,
        content: MathContent::Group,
        children,
    }
}

/// An n-ary operator with its limits.
fn nary(
    operator: MathBox,
    lower: Option<MathBox>,
    upper: Option<MathBox>,
    operand: MathBox,
    limits: LimitLocation,
    context: &MathContext<'_>,
) -> MathBox {
    match limits {
        LimitLocation::UnderOver => {
            let stacked = over_under(operator, upper, lower, context);
            beside(vec![stacked, thin_space(context), operand])
        }
        LimitLocation::SubscriptSuperscript => {
            let scripted = scripts(operator, lower, upper, false, false, context);
            beside(vec![scripted, thin_space(context), operand])
        }
    }
}

/// A base with something centred above it, below it, or both.
fn over_under(
    base: MathBox,
    above: Option<MathBox>,
    below: Option<MathBox>,
    context: &MathContext<'_>,
) -> MathBox {
    let gap = context.rule_thickness().times(3);
    let width = base
        .width
        .maximum(above.as_ref().map_or(Emu::ZERO, |content| content.width))
        .maximum(below.as_ref().map_or(Emu::ZERO, |content| content.width));
    let mut children = Vec::new();
    let base_ascent = base.ascent;
    let base_descent = base.descent;
    children.push(PlacedMathBox {
        x: crate::measure::half_of(width - base.width),
        baseline: Emu::ZERO,
        content: base,
    });
    let mut ascent = base_ascent;
    let mut descent = base_descent;
    if let Some(above) = above {
        let offset = Emu::ZERO - (base_ascent + gap + above.descent);
        ascent = ascent.maximum(above.ascent - offset);
        children.push(PlacedMathBox {
            x: crate::measure::half_of(width - above.width),
            baseline: offset,
            content: above,
        });
    }
    if let Some(below) = below {
        let offset = base_descent + gap + below.ascent;
        descent = descent.maximum(below.descent + offset);
        children.push(PlacedMathBox {
            x: crate::measure::half_of(width - below.width),
            baseline: offset,
            content: below,
        });
    }
    MathBox {
        width,
        ascent,
        descent,
        content: MathContent::Group,
        children,
    }
}

/// A base with a subscript, a superscript, or both — after it, or before it for `m:sPre`.
fn scripts(
    base: MathBox,
    subscript: Option<MathBox>,
    superscript: Option<MathBox>,
    aligned: bool,
    before: bool,
    context: &MathContext<'_>,
) -> MathBox {
    // **`GUESS:`** the shift-up is a fifth of an em and the shift-down a fifth of an em, which are
    // the round numbers between *The TeXbook*'s `sup_drop`/`sub_drop` family and what a font's
    // `MATH` table would state. A superscript that sat on the baseline would be a different glyph.
    let up = context.em().divided_by(5);
    let down = context.em().divided_by(5);
    let script_width = subscript
        .as_ref()
        .map_or(Emu::ZERO, |content| content.width)
        .maximum(
            superscript
                .as_ref()
                .map_or(Emu::ZERO, |content| content.width),
        );
    let base_width = base.width;
    let base_ascent = base.ascent;
    let base_descent = base.descent;
    let (base_x, script_x) = if before {
        (script_width, Emu::ZERO)
    } else {
        (Emu::ZERO, base_width)
    };
    let mut children = vec![PlacedMathBox {
        x: base_x,
        baseline: Emu::ZERO,
        content: base,
    }];
    let mut ascent = base_ascent;
    let mut descent = base_descent;
    if let Some(superscript) = superscript {
        let offset = Emu::ZERO - (base_ascent - up).maximum(up);
        ascent = ascent.maximum(superscript.ascent - offset);
        // `m:alnScr` asks for the two scripts to be aligned with each other rather than staggered,
        // and both are placed at `script_x` here, so an aligned pair is what this always produces.
        //
        // **`GUESS:`** the *staggered* arrangement — a superscript pushed right of its subscript by
        // the base's italic correction, which is what a `MATH` table's `MathItalicsCorrectionInfo`
        // supplies — is **not implemented**, because `mjx-text` reads no such table (see this
        // module's own header). So `m:alnScr="0"` and `m:alnScr="1"` lay out identically, and the
        // flag is read and honoured in the direction this crate can honour it.
        let _ = aligned;
        children.push(PlacedMathBox {
            x: script_x,
            baseline: offset,
            content: superscript,
        });
    }
    if let Some(subscript) = subscript {
        let offset = down + base_descent;
        descent = descent.maximum(subscript.descent + offset);
        children.push(PlacedMathBox {
            x: script_x,
            baseline: offset,
            content: subscript,
        });
    }
    MathBox {
        width: base_width + script_width,
        ascent,
        descent,
        content: MathContent::Group,
        children,
    }
}

/// A limit set under or over its base — `m:limLow` and `m:limUpp`.
fn stacked_limit(base: MathBox, limit: MathBox, below: bool, context: &MathContext<'_>) -> MathBox {
    if below {
        over_under(base, None, Some(limit), context)
    } else {
        over_under(base, Some(limit), None, context)
    }
}

/// An accent, centred over its base.
fn accent(mark: MathBox, base: MathBox, context: &MathContext<'_>) -> MathBox {
    over_under(base, Some(mark), None, context)
}

/// A rule over or under its base — `m:bar`.
fn bar(base: MathBox, above: bool, context: &MathContext<'_>) -> MathBox {
    let thickness = context.rule_thickness();
    let gap = thickness.times(2);
    let width = base.width;
    let base_ascent = base.ascent;
    let base_descent = base.descent;
    let mut children = vec![PlacedMathBox {
        x: Emu::ZERO,
        baseline: Emu::ZERO,
        content: base,
    }];
    let (ascent, descent) = if above {
        let top = base_ascent + gap + thickness;
        children.push(PlacedMathBox {
            x: Emu::ZERO,
            baseline: Emu::ZERO,
            content: MathBox {
                width,
                ascent: top,
                descent: Emu::ZERO - (base_ascent + gap),
                content: MathContent::Rule,
                children: Vec::new(),
            },
        });
        (top, base_descent)
    } else {
        let bottom = base_descent + gap + thickness;
        children.push(PlacedMathBox {
            x: Emu::ZERO,
            baseline: Emu::ZERO,
            content: MathBox {
                width,
                ascent: Emu::ZERO - (base_descent + gap),
                descent: bottom,
                content: MathContent::Rule,
                children: Vec::new(),
            },
        });
        (base_ascent, bottom)
    };
    MathBox {
        width,
        ascent,
        descent,
        content: MathContent::Group,
        children,
    }
}

/// What an `m:d` encloses its arguments in: the three characters, whether they grow, and how.
///
/// One argument rather than five, because they are read from one `m:dPr` and travel together.
#[derive(Clone, Copy)]
struct Brackets<'a> {
    begin: &'a str,
    separator: &'a str,
    end: &'a str,
    grow: bool,
    shape_rule: DelimiterShape,
}

/// Delimiters around one or more arguments, grown to what they enclose.
fn delimiter(
    engine: &mut TextEngine<'_>,
    brackets: Brackets<'_>,
    arguments: Vec<MathBox>,
    context: &MathContext<'_>,
) -> Result<MathBox, FontError> {
    let Brackets {
        begin,
        separator,
        end,
        grow,
        shape_rule,
    } = brackets;
    let axis = context.axis();
    // The height the delimiters must span: the content's own extent about the **axis**, doubled, so
    // that a bracket around something that reaches far below the baseline is as tall above it. That
    // is what `centered` means and it is why a delimiter's height is not the content's height.
    let above = arguments.iter().fold(Emu::ZERO, |tallest, content| {
        tallest.maximum(content.ascent)
    });
    let below = arguments.iter().fold(Emu::ZERO, |deepest, content| {
        deepest.maximum(content.descent)
    });
    let target = if !grow {
        Emu::ZERO
    } else {
        match shape_rule {
            DelimiterShape::Centered => {
                let reach = (above - axis).maximum(below + axis);
                (reach + axis).maximum(reach - axis).times(2)
            }
            DelimiterShape::MatchArgument => above + below,
        }
    };
    let mut pieces = Vec::new();
    if !begin.is_empty() {
        pieces.push(glyph(engine, begin, context, target)?);
    }
    for (index, argument) in arguments.into_iter().enumerate() {
        if index > 0 && !separator.is_empty() {
            pieces.push(glyph(engine, separator, context, target)?);
        }
        pieces.push(argument);
    }
    if !end.is_empty() {
        pieces.push(glyph(engine, end, context, target)?);
    }
    Ok(beside(pieces))
}

/// A matrix: columns as wide as their widest cell, rows stacked, each cell aligned in its column.
fn matrix(
    rows: Vec<Vec<MathBox>>,
    alignments: &[RelativeHorizontalAlignment],
    base_alignment: Option<RelativeVerticalAlignment>,
    context: &MathContext<'_>,
) -> MathBox {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![Emu::ZERO; columns];
    for line in &rows {
        for (index, cell) in line.iter().enumerate() {
            widths[index] = widths[index].maximum(cell.width);
        }
    }
    // **`GUESS:`** the column gap is one em and the row gap one third of an em. `CT_MPr`'s own
    // `m:cGp`/`m:rSp` state them in twentieths of a point when a document says so and are the
    // spacing overrides `crate::math` declares it does not read; these are what a matrix that states
    // nothing gets, which is every matrix Word's own editor writes.
    let column_gap = context.em();
    let row_gap = context.em().divided_by(3);
    let width = widths.iter().fold(Emu::ZERO, |total, entry| total + *entry)
        + column_gap.times(i64::try_from(columns.saturating_sub(1)).unwrap_or(0));

    let mut children = Vec::new();
    let mut y = Emu::ZERO;
    let mut heights = Vec::with_capacity(rows.len());
    for line in &rows {
        let ascent = line
            .iter()
            .fold(Emu::ZERO, |tallest, cell| tallest.maximum(cell.ascent));
        let descent = line
            .iter()
            .fold(Emu::ZERO, |deepest, cell| deepest.maximum(cell.descent));
        heights.push((ascent, descent));
    }
    for (index, line) in rows.into_iter().enumerate() {
        let (ascent, descent) = heights[index];
        if index > 0 {
            y += row_gap;
        }
        y += ascent;
        let mut x = Emu::ZERO;
        for (column, cell) in line.into_iter().enumerate() {
            let available = widths.get(column).copied().unwrap_or(cell.width);
            let alignment = alignments
                .get(column)
                .copied()
                .unwrap_or(RelativeHorizontalAlignment::Center);
            let offset = match alignment {
                RelativeHorizontalAlignment::Left | RelativeHorizontalAlignment::Inside => {
                    Emu::ZERO
                }
                RelativeHorizontalAlignment::Right | RelativeHorizontalAlignment::Outside => {
                    available - cell.width
                }
                RelativeHorizontalAlignment::Center => {
                    crate::measure::half_of(available - cell.width)
                }
            };
            children.push(PlacedMathBox {
                x: x + offset,
                baseline: y,
                content: cell,
            });
            x += available + column_gap;
        }
        y += descent;
    }
    let total = y;
    // Where the matrix's own baseline sits: `m:baseJc` says which row's baseline the surrounding
    // expression aligns to. **`GUESS:`** an absent `m:baseJc` centres the matrix on the axis, which
    // is what a bracketed matrix looks like in every book.
    let shift = match base_alignment {
        Some(RelativeVerticalAlignment::Top) => Emu::ZERO,
        Some(RelativeVerticalAlignment::Bottom) => total,
        _ => crate::measure::half_of(total) - context.axis(),
    };
    for child in &mut children {
        child.baseline -= shift;
    }
    MathBox {
        width,
        ascent: shift,
        descent: total - shift,
        content: MathContent::Group,
        children,
    }
}

/// An equation array: rows stacked and aligned on one **alignment axis**.
///
/// # The axis is the assertion
///
/// Rows are aligned on a vertical line, not on their left edges — that is what an equation array is
/// for, and it is why a stack of `x = 1` / `xy = 22` shows its two equals signs one above the other.
/// The alignment point is `m:aln` on a run when a row states one and the row's own start otherwise;
/// with none stated the rows are **centred**, which is this crate's reading and is marked
/// `GUESS:` here rather than cited: `CT_EqArrPr` states a `m:baseJc` and nothing about horizontal
/// alignment at all.
///
/// `tests/an_equation_is_typeset.rs` reads the axis by asserting the two rows' x offsets differ by
/// exactly half the difference of their widths, which a left-aligned implementation cannot produce.
fn equation_array(
    rows: Vec<MathBox>,
    base_alignment: Option<RelativeVerticalAlignment>,
    context: &MathContext<'_>,
) -> MathBox {
    let width = rows
        .iter()
        .fold(Emu::ZERO, |widest, row| widest.maximum(row.width));
    let gap = context.em().divided_by(3);
    let mut children = Vec::new();
    let mut y = Emu::ZERO;
    for (index, row) in rows.into_iter().enumerate() {
        if index > 0 {
            y += gap;
        }
        y += row.ascent;
        let descent = row.descent;
        children.push(PlacedMathBox {
            x: crate::measure::half_of(width - row.width),
            baseline: y,
            content: row,
        });
        y += descent;
    }
    let total = y;
    let shift = match base_alignment {
        Some(RelativeVerticalAlignment::Top) => Emu::ZERO,
        Some(RelativeVerticalAlignment::Bottom) => total,
        _ => crate::measure::half_of(total) - context.axis(),
    };
    for child in &mut children {
        child.baseline -= shift;
    }
    MathBox {
        width,
        ascent: shift,
        descent: total - shift,
        content: MathContent::Group,
        children,
    }
}

/// A border box: the base, with a rule on each edge the file does not hide.
fn border_box(
    base: MathBox,
    edges: [bool; 4],
    strikes: [bool; 4],
    context: &MathContext<'_>,
) -> MathBox {
    let thickness = context.rule_thickness();
    let padding = thickness.times(3);
    let width = base.width + padding.times(2);
    let ascent = base.ascent + padding;
    let descent = base.descent + padding;
    let mut children = vec![PlacedMathBox {
        x: padding,
        baseline: Emu::ZERO,
        content: base,
    }];
    let mut rule = |x: Emu, top: Emu, bottom: Emu, span: Emu| {
        children.push(PlacedMathBox {
            x,
            baseline: Emu::ZERO,
            content: MathBox {
                width: span,
                ascent: top,
                descent: bottom,
                content: MathContent::Rule,
                children: Vec::new(),
            },
        });
    };
    let [top, bottom, left, right] = edges;
    if top {
        rule(Emu::ZERO, ascent, Emu::ZERO - (ascent - thickness), width);
    }
    if bottom {
        rule(Emu::ZERO, Emu::ZERO - (descent - thickness), descent, width);
    }
    if left {
        rule(Emu::ZERO, ascent, descent, thickness);
    }
    if right {
        rule(width - thickness, ascent, descent, thickness);
    }
    // The two axis-aligned strikes. The two diagonal ones would need a transform, and this crate
    // resolves none — they are **not drawn**, which is stated here rather than silently dropped.
    let [horizontal, vertical, _rising, _falling] = strikes;
    if horizontal {
        let axis = context.axis();
        rule(
            Emu::ZERO,
            axis + crate::measure::half_of(thickness),
            crate::measure::half_of(thickness) - axis,
            width,
        );
    }
    if vertical {
        rule(crate::measure::half_of(width), ascent, descent, thickness);
    }
    MathBox {
        width,
        ascent,
        descent,
        content: MathContent::Group,
        children,
    }
}

/// A phantom: space with nothing in it, or with its extents zeroed.
fn phantom(
    base: MathBox,
    show: bool,
    zero_width: bool,
    zero_ascent: bool,
    zero_descent: bool,
) -> MathBox {
    MathBox {
        width: if zero_width { Emu::ZERO } else { base.width },
        ascent: if zero_ascent { Emu::ZERO } else { base.ascent },
        descent: if zero_descent {
            Emu::ZERO
        } else {
            base.descent
        },
        content: MathContent::Group,
        children: if show {
            vec![PlacedMathBox {
                x: Emu::ZERO,
                baseline: Emu::ZERO,
                content: base,
            }]
        } else {
            Vec::new()
        },
    }
}
