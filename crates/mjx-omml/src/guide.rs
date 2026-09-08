// The sixth page of the upper shared markup's guide, whose other five live in `mjx-chart` and whose
// index is `crates/mjx-chart/docs/guide/README.md`. It is hosted here rather than there because
// `mjx-chart` and `mjx-omml` are the same layering rank: neither may depend on the other, so a page
// in that directory could not resolve a single intra-doc link into this crate (MJXOFF-221).
//
// One page, so this module *is* the page rather than an index over submodules — the shape
// `mjx_mce::guide` uses for the same reason.
#![doc = include_str!("../docs/office_math.md")]

// Imported so the page's intra-doc links resolve, exactly as the other guide sets do.
#[allow(unused_imports)]
use crate::{
    Accent, AccentProperties, Argument, ArgumentProperties, Bar, BarProperties, BorderBox,
    BorderBoxProperties, ControlOnlyProperties, ControlProperties, Delimiter, DelimiterProperties,
    EquationArray, EquationArrayProperties, Fraction, FractionProperties, Function, GroupCharacter,
    GroupCharacterProperties, LowerLimit, Math, MathBox, MathElement, MathParagraph,
    MathParagraphProperties, MathProperties, Matrix, MatrixColumn, MatrixColumnProperties,
    MatrixColumns, MatrixProperties, MatrixRow, NaryOperator, NaryOperatorProperties, Phantom,
    PhantomProperties, PreScript, Radical, RadicalProperties, Run, RunProperties, Subscript,
    SubscriptSuperscript, SubscriptSuperscriptProperties, Superscript, Text, UpperLimit,
};
