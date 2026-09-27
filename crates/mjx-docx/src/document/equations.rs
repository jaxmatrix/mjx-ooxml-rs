//! An `m:oMath` tree, resolved to plain values for a layout engine — the third thing this crate
//! resolves *out* of a lower model rather than passing the lower model up.
//!
//! # Why this exists rather than a `mjx_omml::Math` in the residency's surface
//!
//! `mjx-omml` models Office MathML the way every fidelity crate in this project models its schema:
//! a wrapper that keeps its children as [`RawNode`](mjx_ooxml_core::RawNode)s verbatim, with typed
//! read accessors layered over them. Every one of those accessors takes an
//! [`Interner`](mjx_ooxml_core::Interner), because a raw child's name is a symbol and not a string.
//!
//! So handing a [`mjx_omml::Math`] to a box model would hand it the interner too, and an
//! [`Interner`](mjx_ooxml_core::Interner) is neither `Clone` nor `PartialEq` — which
//! [`ParagraphFormatting`](crate::ParagraphFormatting) is both of, and has to be, because
//! a stream's paragraphs are cloned out of one flat list. The alternative to resolving here is
//! therefore not "pass the model up"; it is "make the residency uncloneable", which is a much larger
//! change made for a much smaller reason.
//!
//! It is also the line this crate already draws twice. A `w:drawing` becomes plain EMU numbers
//! rather than a `mjx-dml` type, and a `w:themeColor` becomes a concrete `RRGGBB` rather than a
//! reference, both so that **the box model above never parses a lower schema**. An equation is the
//! same shape of thing: `mjx-omml` says what the markup *is*, and everything a typesetter needs from
//! it is a character, a flag, an enumeration or a child list.
//!
//! # What is resolved and what is dropped
//!
//! Every member of `EG_OMathMathElements` — all twenty — is resolved, because a box model that met
//! an unresolved one would have to draw *something* and would draw the wrong thing. What is dropped
//! is what a typesetter does not read: `m:ctrlPr`'s WordprocessingML pass-through (a `w:rPr` on the
//! *object*, which changes the colour of a fraction bar and not its position), the spacing overrides
//! `m:mPr`/`m:eqArrPr` state in twentieths of a point (`m:rSp`, `m:cGp`, …), and `m:argPr`'s size
//! override. Each is named at its site as a declared gap rather than left unmentioned.
//!
//! Nothing here measures anything, and nothing here has an opinion about layout: an
//! [`EquationNode`] is what the file says, in Rust.

use mjx_ooxml_core::Interner;
use mjx_ooxml_types::officemath::{
    Character, DelimiterShape, FractionType, Justification, LimitLocation, MathStyle, ScriptType,
    TopBottom,
};
use mjx_ooxml_types::shared::{RelativeHorizontalAlignment, RelativeVerticalAlignment};

/// One run of mathematical text — the leaf every other node bottoms out in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EquationRun {
    /// `m:t`, concatenated: what the run says.
    pub text: String,
    /// `m:sty` — plain, bold, italic or bold italic.
    ///
    /// **Absent is not plain.** A single variable in an equation is italic in every renderer without
    /// the file saying so, and a reader that treated an absent `m:sty` as `p` would set every
    /// equation upright. `shared-math.xsd` makes the element optional and states no default, so the
    /// reading belongs to whatever lays the equation out — `mjx_layout_docx::math` marks it `GUESS:`
    /// and this carries the value through unchanged.
    pub style: Option<MathStyle>,
    /// `m:scr` — the alphabet the run is drawn in (script, fraktur, double-struck, …).
    pub script: Option<ScriptType>,
    /// `m:nor` — "normal text": the run is prose inside an equation and is **not** italicised.
    pub normal_text: bool,
    /// `m:lit` — literal: the run is exempt from the spacing and italicisation an operator or an
    /// identifier would get.
    pub literal: bool,
}

/// One node of a resolved equation.
///
/// The variant names are this project's own (`NaryOperator`, never `Nary`); the wire element each
/// one comes from is named in its documentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EquationNode {
    /// `m:r` — a run of text.
    Run(EquationRun),
    /// `m:f` — a fraction.
    Fraction {
        /// `m:num`.
        numerator: Vec<EquationNode>,
        /// `m:den`.
        denominator: Vec<EquationNode>,
        /// `m:type` — bar, skewed, linear or no bar.
        kind: Option<FractionType>,
    },
    /// `m:rad` — a radical.
    Radical {
        /// `m:deg` — the index, empty for a square root.
        degree: Vec<EquationNode>,
        /// `m:e` — what is under the sign.
        radicand: Vec<EquationNode>,
        /// `m:degHide` — whether the degree is drawn even when present.
        degree_hidden: bool,
    },
    /// `m:nary` — an n-ary operator: a sum, a product, an integral.
    NaryOperator {
        /// `m:chr` — the operator itself. `None` means the element is absent; `mjx_layout_docx::math` draws `U+222B INTEGRAL`.
        character: Option<String>,
        /// `m:sub` — the lower limit.
        lower: Vec<EquationNode>,
        /// `m:sup` — the upper limit.
        upper: Vec<EquationNode>,
        /// `m:e` — the operand.
        operand: Vec<EquationNode>,
        /// `m:limLoc` — whether the limits sit under and over the operator or beside it.
        limit_location: Option<LimitLocation>,
        /// `m:grow` — whether the operator grows to its operand's height.
        grow: bool,
        /// `m:subHide`.
        lower_hidden: bool,
        /// `m:supHide`.
        upper_hidden: bool,
    },
    /// `m:d` — delimiters around one or more arguments.
    Delimiter {
        /// `m:begChr`. `None` means the element is absent and whatever lays the equation out chooses — `(`, in
        /// `mjx_layout_docx::math`; an explicit empty string means none is
        /// drawn, which is how a one-sided delimiter is written.
        begin: Option<String>,
        /// `m:sepChr`. `None` means the element is absent; `mjx_layout_docx::math` draws `|`.
        separator: Option<String>,
        /// `m:endChr`. `None` means the element is absent; `mjx_layout_docx::math` draws `)`.
        end: Option<String>,
        /// `m:grow` — whether the delimiters grow to their tallest argument.
        grow: bool,
        /// `m:shp` — centred on the argument, or matched to its own height.
        shape: Option<DelimiterShape>,
        /// `m:e`, one per argument.
        arguments: Vec<Vec<EquationNode>>,
    },
    /// `m:m` — a matrix.
    Matrix {
        /// `m:mr`, each a row of `m:e` cells.
        rows: Vec<Vec<Vec<EquationNode>>>,
        /// The per-column justification `m:mcs` states, expanded by each `m:count` so that entry
        /// *i* is column *i*.
        column_alignments: Vec<RelativeHorizontalAlignment>,
        /// `m:baseJc` — where the matrix's own baseline sits relative to its rows.
        base_alignment: Option<RelativeVerticalAlignment>,
    },
    /// `m:acc` — an accent over its base.
    Accent {
        /// `m:chr`. `None` means the element is absent; `mjx_layout_docx::math` draws `U+0302`.
        character: Option<String>,
        /// `m:e`.
        base: Vec<EquationNode>,
    },
    /// `m:bar` — a rule over or under its base.
    Bar {
        /// `m:pos`. `None` means the element is absent; `mjx_layout_docx::math` draws it below.
        position: Option<TopBottom>,
        /// `m:e`.
        base: Vec<EquationNode>,
    },
    /// `m:func` — a named function applied to an argument.
    Function {
        /// `m:fName`.
        name: Vec<EquationNode>,
        /// `m:e`.
        base: Vec<EquationNode>,
    },
    /// `m:limLow` or `m:limUpp` — a limit under or over its base.
    Limit {
        /// Whether the limit is below (`m:limLow`) rather than above (`m:limUpp`).
        below: bool,
        /// `m:e`.
        base: Vec<EquationNode>,
        /// `m:lim`.
        limit: Vec<EquationNode>,
    },
    /// `m:sSub`, `m:sSup` or `m:sSubSup` — scripts after their base.
    Script {
        /// `m:e`.
        base: Vec<EquationNode>,
        /// `m:sub`, empty when there is none.
        subscript: Vec<EquationNode>,
        /// `m:sup`, empty when there is none.
        superscript: Vec<EquationNode>,
        /// `m:alnScr` — whether the two scripts are aligned with each other rather than staggered.
        aligned: bool,
    },
    /// `m:sPre` — scripts **before** their base.
    PreScript {
        /// `m:e`.
        base: Vec<EquationNode>,
        /// `m:sub`.
        subscript: Vec<EquationNode>,
        /// `m:sup`.
        superscript: Vec<EquationNode>,
    },
    /// `m:groupChr` — a brace or arrow grouping its base.
    GroupCharacter {
        /// `m:chr`. `None` means the element is absent; `mjx_layout_docx::math` draws `U+23DF`.
        character: Option<String>,
        /// `m:pos` — which side the character is drawn on.
        position: Option<TopBottom>,
        /// `m:vertJc` — which side the base's own baseline aligns to.
        vertical_alignment: Option<TopBottom>,
        /// `m:e`.
        base: Vec<EquationNode>,
    },
    /// `m:borderBox` — a box with rules and strikes on chosen edges.
    BorderBox {
        /// `m:e`.
        base: Vec<EquationNode>,
        /// The four `m:hide*` flags, as *drawn* edges: `true` means the rule is there.
        top: bool,
        /// See `top`.
        bottom: bool,
        /// See `top`.
        left: bool,
        /// See `top`.
        right: bool,
        /// `m:strikeH`.
        strike_horizontal: bool,
        /// `m:strikeV`.
        strike_vertical: bool,
        /// `m:strikeBLTR`.
        strike_rising: bool,
        /// `m:strikeTLBR`.
        strike_falling: bool,
    },
    /// `m:box` — a grouping with no visible mark, which exists to change spacing and breaking.
    Box {
        /// `m:e`.
        base: Vec<EquationNode>,
        /// `m:noBreak` — whether a line may not break inside it.
        no_break: bool,
        /// `m:diff` — whether it is a differential, which is spaced as an operator.
        differential: bool,
    },
    /// `m:eqArr` — a vertical stack of equations aligned on their alignment points.
    EquationArray {
        /// `m:e`, one per row.
        rows: Vec<Vec<EquationNode>>,
        /// `m:baseJc`.
        base_alignment: Option<RelativeVerticalAlignment>,
    },
    /// `m:phant` — a box that occupies space without drawing.
    Phantom {
        /// `m:e`.
        base: Vec<EquationNode>,
        /// `m:show` — whether it is drawn after all.
        show: bool,
        /// `m:zeroWid`.
        zero_width: bool,
        /// `m:zeroAsc`.
        zero_ascent: bool,
        /// `m:zeroDesc`.
        zero_descent: bool,
    },
}

/// One `m:oMath` in a paragraph's content, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquationFormatting {
    /// The byte of [`ParagraphFormatting::text`](crate::ParagraphFormatting::text) the
    /// equation sits at.
    ///
    /// **It contributes no character**, exactly as a `w:footnoteReference` does and for the same
    /// reason: what is drawn there is generated, not written. What travels here is *where*.
    pub at: usize,
    /// Whether it came from an `m:oMathPara` — a **display** equation on lines of its own — rather
    /// than being inline in the surrounding text.
    pub display: bool,
    /// `m:oMathParaPr/m:jc`, for a display equation. `None` for an inline one, and for a display one
    /// that states nothing.
    ///
    /// **`ST_Jc` here is Office Math's own four-valued one** (`left`, `center`, `right`,
    /// `centerGroup`) and not WordprocessingML's twelve-valued `w:jc`. They share a name in the
    /// specification and mean different things: `centerGroup` centres a *group* of equations on one
    /// another, which `w:jc` has no member for.
    pub justification: Option<Justification>,
    /// Its content, in order.
    pub nodes: Vec<EquationNode>,
}

/// Reads `character`'s wire string, treating an absent attribute and an unreadable one alike.
fn character_of(character: Option<Character>) -> Option<String> {
    character.map(|value| value.to_wire().to_owned())
}

/// Resolves one `m:oMath` into plain nodes.
#[must_use]
pub(super) fn resolve(equation: &mjx_omml::Math, interner: &Interner) -> Vec<EquationNode> {
    nodes(&equation.elements(interner), interner)
}

/// Resolves one argument slot (`m:e`, `m:num`, `m:sub`, …), or nothing when the slot is absent.
fn argument(slot: Option<mjx_omml::Argument>, interner: &Interner) -> Vec<EquationNode> {
    slot.map(|value| nodes(&value.elements(interner), interner))
        .unwrap_or_default()
}

/// Resolves a list of elements.
fn nodes(elements: &[mjx_omml::MathElement], interner: &Interner) -> Vec<EquationNode> {
    elements
        .iter()
        .map(|element| node(element, interner))
        .collect()
}

/// The one place a `m:` element becomes an [`EquationNode`].
#[allow(clippy::too_many_lines)]
fn node(element: &mjx_omml::MathElement, interner: &Interner) -> EquationNode {
    use mjx_omml::MathElement as Element;
    match element {
        Element::Run(run) => {
            let properties = run.properties(interner);
            EquationNode::Run(EquationRun {
                text: run.text(interner),
                style: properties.as_ref().and_then(|value| value.style(interner)),
                script: properties.as_ref().and_then(|value| value.script(interner)),
                normal_text: properties
                    .as_ref()
                    .and_then(|value| value.normal_text(interner))
                    .unwrap_or(false),
                literal: properties
                    .as_ref()
                    .and_then(|value| value.literal(interner))
                    .unwrap_or(false),
            })
        }
        Element::Fraction(fraction) => EquationNode::Fraction {
            numerator: argument(fraction.numerator(interner), interner),
            denominator: argument(fraction.denominator(interner), interner),
            kind: fraction
                .properties(interner)
                .and_then(|properties| properties.fraction_type(interner)),
        },
        Element::Radical(radical) => {
            let properties = radical.properties(interner);
            EquationNode::Radical {
                degree: argument(radical.degree(interner), interner),
                radicand: argument(radical.radicand(interner), interner),
                // GUESS: an absent `m:degHide` is *false*, so a stated `m:deg` is drawn. The schema
                // makes the element optional and states no default; `CT_OnOff`'s own is `true`
                // when the element is present with no `m:val`, and absence is the ordinary "not
                // asserted".
                degree_hidden: properties
                    .and_then(|value| value.degree_hide(interner))
                    .unwrap_or(false),
            }
        }
        Element::NaryOperator(nary) => {
            let properties = nary.properties(interner);
            EquationNode::NaryOperator {
                character: properties
                    .as_ref()
                    .and_then(|value| character_of(value.character(interner))),
                lower: argument(nary.lower_limit(interner), interner),
                upper: argument(nary.upper_limit(interner), interner),
                operand: argument(nary.operand(interner), interner),
                limit_location: properties
                    .as_ref()
                    .and_then(|value| value.limit_location(interner)),
                grow: properties
                    .as_ref()
                    .and_then(|value| value.grow(interner))
                    .unwrap_or(false),
                lower_hidden: properties
                    .as_ref()
                    .and_then(|value| value.subscript_hide(interner))
                    .unwrap_or(false),
                upper_hidden: properties
                    .as_ref()
                    .and_then(|value| value.superscript_hide(interner))
                    .unwrap_or(false),
            }
        }
        Element::Delimiter(delimiter) => {
            let properties = delimiter.properties(interner);
            EquationNode::Delimiter {
                begin: properties
                    .as_ref()
                    .and_then(|value| character_of(value.begin_character(interner))),
                separator: properties
                    .as_ref()
                    .and_then(|value| character_of(value.separator_character(interner))),
                end: properties
                    .as_ref()
                    .and_then(|value| character_of(value.end_character(interner))),
                // GUESS: an absent `m:grow` grows. `shared-math.xsd` makes the element optional
                // and states no default, and reading absence as `false` is the single easiest way
                // to draw an equation with parentheses too small for what they hold — which is what
                // every renderer that gets this wrong looks like.
                grow: properties
                    .as_ref()
                    .and_then(|value| value.grow(interner))
                    .unwrap_or(true),
                shape: properties.as_ref().and_then(|value| value.shape(interner)),
                arguments: delimiter
                    .arguments(interner)
                    .into_iter()
                    .map(|slot| nodes(&slot.elements(interner), interner))
                    .collect(),
            }
        }
        Element::Matrix(matrix) => {
            let properties = matrix.properties(interner);
            EquationNode::Matrix {
                rows: matrix
                    .rows(interner)
                    .into_iter()
                    .map(|row| {
                        row.cells(interner)
                            .into_iter()
                            .map(|cell| nodes(&cell.elements(interner), interner))
                            .collect()
                    })
                    .collect(),
                column_alignments: column_alignments(properties.as_ref(), interner),
                base_alignment: properties
                    .as_ref()
                    .and_then(|value| value.base_alignment(interner)),
            }
        }
        Element::Accent(accent) => EquationNode::Accent {
            character: accent
                .properties(interner)
                .and_then(|value| character_of(value.character(interner))),
            base: argument(accent.base(interner), interner),
        },
        Element::Bar(bar) => EquationNode::Bar {
            position: bar
                .properties(interner)
                .and_then(|value| value.position(interner)),
            base: argument(bar.base(interner), interner),
        },
        Element::Function(function) => EquationNode::Function {
            name: argument(function.function_name(interner), interner),
            base: argument(function.base(interner), interner),
        },
        Element::LowerLimit(limit) => EquationNode::Limit {
            below: true,
            base: argument(limit.base(interner), interner),
            limit: argument(limit.limit(interner), interner),
        },
        Element::UpperLimit(limit) => EquationNode::Limit {
            below: false,
            base: argument(limit.base(interner), interner),
            limit: argument(limit.limit(interner), interner),
        },
        Element::Subscript(script) => EquationNode::Script {
            base: argument(script.base(interner), interner),
            subscript: argument(script.subscript(interner), interner),
            superscript: Vec::new(),
            aligned: false,
        },
        Element::Superscript(script) => EquationNode::Script {
            base: argument(script.base(interner), interner),
            subscript: Vec::new(),
            superscript: argument(script.superscript(interner), interner),
            aligned: false,
        },
        Element::SubscriptSuperscript(script) => EquationNode::Script {
            base: argument(script.base(interner), interner),
            subscript: argument(script.subscript(interner), interner),
            superscript: argument(script.superscript(interner), interner),
            aligned: script
                .properties(interner)
                .and_then(|value| value.aligned_scripts(interner))
                .unwrap_or(false),
        },
        Element::PreScript(script) => EquationNode::PreScript {
            base: argument(script.base(interner), interner),
            subscript: argument(script.subscript(interner), interner),
            superscript: argument(script.superscript(interner), interner),
        },
        Element::GroupCharacter(group) => {
            let properties = group.properties(interner);
            EquationNode::GroupCharacter {
                character: properties
                    .as_ref()
                    .and_then(|value| character_of(value.character(interner))),
                position: properties
                    .as_ref()
                    .and_then(|value| value.position(interner)),
                vertical_alignment: properties
                    .as_ref()
                    .and_then(|value| value.vertical_justification(interner)),
                base: argument(group.base(interner), interner),
            }
        }
        Element::BorderBox(border) => {
            let properties = border.properties(interner);
            let drawn = |hidden: Option<bool>| !hidden.unwrap_or(false);
            EquationNode::BorderBox {
                base: argument(border.base(interner), interner),
                top: drawn(
                    properties
                        .as_ref()
                        .and_then(|value| value.hide_top(interner)),
                ),
                bottom: drawn(
                    properties
                        .as_ref()
                        .and_then(|value| value.hide_bottom(interner)),
                ),
                left: drawn(
                    properties
                        .as_ref()
                        .and_then(|value| value.hide_left(interner)),
                ),
                right: drawn(
                    properties
                        .as_ref()
                        .and_then(|value| value.hide_right(interner)),
                ),
                strike_horizontal: properties
                    .as_ref()
                    .and_then(|value| value.strike_horizontal(interner))
                    .unwrap_or(false),
                strike_vertical: properties
                    .as_ref()
                    .and_then(|value| value.strike_vertical(interner))
                    .unwrap_or(false),
                strike_rising: properties
                    .as_ref()
                    .and_then(|value| value.strike_bottom_left_to_top_right(interner))
                    .unwrap_or(false),
                strike_falling: properties
                    .as_ref()
                    .and_then(|value| value.strike_top_left_to_bottom_right(interner))
                    .unwrap_or(false),
            }
        }
        Element::Box(boxed) => {
            let properties = boxed.properties(interner);
            EquationNode::Box {
                base: argument(boxed.base(interner), interner),
                no_break: properties
                    .as_ref()
                    .and_then(|value| value.no_break(interner))
                    .unwrap_or(false),
                differential: properties
                    .as_ref()
                    .and_then(|value| value.differential(interner))
                    .unwrap_or(false),
            }
        }
        Element::EquationArray(array) => EquationNode::EquationArray {
            rows: array
                .arguments(interner)
                .into_iter()
                .map(|row| nodes(&row.elements(interner), interner))
                .collect(),
            base_alignment: array
                .properties(interner)
                .and_then(|value| value.base_alignment(interner)),
        },
        Element::Phantom(phantom) => {
            let properties = phantom.properties(interner);
            EquationNode::Phantom {
                base: argument(phantom.base(interner), interner),
                show: properties
                    .as_ref()
                    .and_then(|value| value.show(interner))
                    .unwrap_or(false),
                zero_width: properties
                    .as_ref()
                    .and_then(|value| value.zero_width(interner))
                    .unwrap_or(false),
                zero_ascent: properties
                    .as_ref()
                    .and_then(|value| value.zero_ascent(interner))
                    .unwrap_or(false),
                zero_descent: properties
                    .as_ref()
                    .and_then(|value| value.zero_descent(interner))
                    .unwrap_or(false),
            }
        }
    }
}

/// `m:mcs` expanded so that entry *i* is column *i*'s own justification.
///
/// A `m:mc` states a justification and a `m:count` of how many columns share it, which is a
/// run-length encoding; a reader that took the list as one entry per column would left-align every
/// column after the first group of a wide matrix.
fn column_alignments(
    properties: Option<&mjx_omml::MatrixProperties>,
    interner: &Interner,
) -> Vec<RelativeHorizontalAlignment> {
    let Some(columns) = properties.and_then(|value| value.column_properties(interner)) else {
        return Vec::new();
    };
    let mut expanded = Vec::new();
    for column in columns.columns(interner) {
        let Some(settings) = column.properties(interner) else {
            continue;
        };
        // `shared-math.xsd` makes `m:count` optional with no stated default; one is the only reading that
        // keeps the encoding total, and it is also what a `m:mc` with no count means in practice —
        // this column, and no other.
        let count = settings.count(interner).unwrap_or(1).max(0);
        let alignment = settings
            .justification(interner)
            .unwrap_or(RelativeHorizontalAlignment::Center);
        for _ in 0..count {
            expanded.push(alignment);
        }
    }
    expanded
}
