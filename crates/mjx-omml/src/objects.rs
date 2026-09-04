//! The Office Math "object" family: every construct with visible mathematical structure — fraction,
//! radical, n-ary operator, delimiter, matrix, the four script forms, and their siblings — plus each
//! one's own paired `*Pr` properties type, [`Run`] (`m:r`, `CT_R`) and [`MathElement`], the 20-way
//! choice (`EG_OMathMathElements`) every math object's own arguments hold.
//!
//! Every type here follows [`crate::support::fidelity_element_impls`]'s shape: `name`/`attributes`/
//! `children`/`empty`, preserved verbatim, with typed **read** accessors layered on top through the
//! small macros just below. **Six `*Pr` types collapse into one Rust type**, [`ControlOnlyProperties`]
//! — `CT_FuncPr`, `CT_LimLowPr`, `CT_LimUppPr`, `CT_SPrePr`, `CT_SSubPr`, `CT_SSupPr` are all,
//! byte for byte, "one optional `ctrlPr` child and nothing else"; which one a value models is its
//! wire name, not its Rust type — the same reuse `mjx-docx`'s `StyleString` already establishes for
//! `CT_String`.

use mjx_ooxml_core::{
    Enumeration, FromXml, Interner, Number, RawAttribute, RawName, RawNode, ToXml,
};
use mjx_ooxml_types::officemath::{DelimiterShape, FractionType, LimitLocation, TopBottom};
use mjx_ooxml_types::shared::{RelativeHorizontalAlignment, RelativeVerticalAlignment};

use crate::arg::Argument;
use crate::leaf::{
    read_manual_break_align_at, read_onoff_child, CharacterCodec, ControlProperties,
};
use crate::support::{fidelity_element_impls, m_child, m_children, m_name, read_val_child};

macro_rules! fidelity_struct {
    ($(#[$meta:meta])* $vis:vis struct $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        $vis struct $name {
            name: RawName,
            attributes: Vec<RawAttribute>,
            children: Vec<RawNode>,
            empty: bool,
        }
        fidelity_element_impls!($name);
    };
}

macro_rules! onoff_accessor {
    ($(#[$meta:meta])* $fn_name:ident, $local:literal) => {
        $(#[$meta])*
        #[must_use]
        pub fn $fn_name(&self, interner: &Interner) -> Option<bool> {
            read_onoff_child(&self.children, interner, $local)
        }
    };
}

macro_rules! val_accessor {
    ($(#[$meta:meta])* $fn_name:ident, $local:literal, $codec:ty, $ret:ty) => {
        $(#[$meta])*
        #[must_use]
        pub fn $fn_name(&self, interner: &Interner) -> Option<$ret> {
            read_val_child::<$codec>(&self.children, interner, $local)
        }
    };
}

macro_rules! child_accessor {
    ($(#[$meta:meta])* $fn_name:ident, $local:literal, $ty:ty) => {
        $(#[$meta])*
        #[must_use]
        pub fn $fn_name(&self, interner: &Interner) -> Option<$ty> {
            m_child(&self.children, interner, $local)
                .and_then(|el| <$ty as FromXml>::from_xml(el, interner).ok())
        }
    };
}

macro_rules! child_list_accessor {
    ($(#[$meta:meta])* $fn_name:ident, $local:literal, $ty:ty) => {
        $(#[$meta])*
        #[must_use]
        pub fn $fn_name(&self, interner: &Interner) -> Vec<$ty> {
            m_children(&self.children, interner, $local)
                .filter_map(|el| <$ty as FromXml>::from_xml(el, interner).ok())
                .collect()
        }
    };
}

macro_rules! control_properties_accessor {
    () => {
        /// `m:ctrlPr` (`CT_CtrlPr`) — this object's own pass-through control properties. See
        /// [`crate::leaf::ControlProperties`]'s own doc comment for why its children are raw.
        #[must_use]
        pub fn control_properties(&self, interner: &Interner) -> Option<ControlProperties> {
            m_child(&self.children, interner, "ctrlPr")
                .and_then(|el| ControlProperties::from_xml(el, interner).ok())
        }
    };
}

// =================================================================================================
// ControlOnlyProperties — CT_FuncPr / CT_LimLowPr / CT_LimUppPr / CT_SPrePr / CT_SSubPr / CT_SSupPr.
// =================================================================================================

fidelity_struct! {
    /// One optional `m:ctrlPr` and nothing else — the shape `CT_FuncPr`, `CT_LimLowPr`, `CT_LimUppPr`,
    /// `CT_SPrePr`, `CT_SSubPr` and `CT_SSupPr` all share exactly. See this module's own doc comment
    /// for why one Rust type serves all six.
    pub struct ControlOnlyProperties
}

impl ControlOnlyProperties {
    /// Builds an empty `<m:{local}/>` (`local` is `"funcPr"`, `"limLowPr"`, `"limUppPr"`, `"sPrePr"`,
    /// `"sSubPr"` or `"sSupPr"`).
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str) -> Self {
        Self {
            name: m_name(interner, local),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }

    control_properties_accessor!();
}

// =================================================================================================
// CT_AccPr / CT_Acc — m:accPr / m:acc (Accent)
// =================================================================================================

fidelity_struct! {
    /// `m:accPr` (`CT_AccPr`, §22.1.2.2) — an accent's own combining character and control properties.
    pub struct AccentProperties
}

impl AccentProperties {
    val_accessor!(
        /// `m:chr` — the combining accent character.
        character, "chr", CharacterCodec, mjx_ooxml_types::officemath::Character
    );
    control_properties_accessor!();
}

fidelity_struct! {
    /// `m:acc` (`CT_Acc`, §22.1.2.1 "Accent") — a diacritical mark placed over its base.
    pub struct Accent
}

impl Accent {
    child_accessor!(
        /// `m:accPr` — this accent's own properties.
        properties, "accPr", AccentProperties
    );
    child_accessor!(
        /// `m:e` — the accented base.
        base, "e", Argument
    );
}

// =================================================================================================
// CT_BarPr / CT_Bar — m:barPr / m:bar (Bar)
// =================================================================================================

fidelity_struct! {
    /// `m:barPr` (`CT_BarPr`, §22.1.2.5) — a bar's own position and control properties.
    pub struct BarProperties
}

impl BarProperties {
    val_accessor!(
        /// `m:pos` — whether the bar is drawn above or below its base.
        position, "pos", Enumeration<TopBottom>, TopBottom
    );
    control_properties_accessor!();
}

fidelity_struct! {
    /// `m:bar` (`CT_Bar`, §22.1.2.4 "Bar") — a horizontal line over or under its base.
    pub struct Bar
}

impl Bar {
    child_accessor!(
        /// `m:barPr` — this bar's own properties.
        properties, "barPr", BarProperties
    );
    child_accessor!(
        /// `m:e` — the based argument the bar is drawn against.
        base, "e", Argument
    );
}

// =================================================================================================
// CT_BoxPr / CT_Box — m:boxPr / m:box (Box)
// =================================================================================================

fidelity_struct! {
    /// `m:boxPr` (`CT_BoxPr`, §22.1.2.7) — a box's own spacing/break/alignment and control properties.
    pub struct BoxProperties
}

impl BoxProperties {
    onoff_accessor!(
        /// `m:opEmu` — whether the box is treated as one operator emulator for spacing purposes.
        operator_emulator, "opEmu"
    );
    onoff_accessor!(
        /// `m:noBreak` — whether the box is exempt from linebreaking.
        no_break, "noBreak"
    );
    onoff_accessor!(
        /// `m:diff` — whether the box is a "differential", spaced accordingly.
        differential, "diff"
    );
    /// `m:brk` (`CT_ManualBreak`) — this box's own manual break alignment point (`@alnAt`).
    #[must_use]
    pub fn manual_break_align_at(&self, interner: &Interner) -> Option<i64> {
        read_manual_break_align_at(&self.children, interner)
    }
    onoff_accessor!(
        /// `m:aln` — whether the box participates in alignment-point alignment.
        alignment, "aln"
    );
    control_properties_accessor!();
}

fidelity_struct! {
    /// `m:box` (`CT_Box`, §22.1.2.6 "Box") — a generic wrapper controlling spacing, alignment, and
    /// break/emulation behaviour, drawn with no visible border.
    pub struct MathBox
}

impl MathBox {
    child_accessor!(
        /// `m:boxPr` — this box's own properties.
        properties, "boxPr", BoxProperties
    );
    child_accessor!(
        /// `m:e` — the boxed argument.
        base, "e", Argument
    );
}

// =================================================================================================
// CT_BorderBoxPr / CT_BorderBox — m:borderBoxPr / m:borderBox (Border Box)
// =================================================================================================

fidelity_struct! {
    /// `m:borderBoxPr` (`CT_BorderBoxPr`, §22.1.2.9) — a border box's own edge visibility, strike
    /// lines, and control properties.
    pub struct BorderBoxProperties
}

impl BorderBoxProperties {
    onoff_accessor!(
        /// `m:hideTop` — whether the top border is hidden.
        hide_top, "hideTop"
    );
    onoff_accessor!(
        /// `m:hideBot` — whether the bottom border is hidden.
        hide_bottom, "hideBot"
    );
    onoff_accessor!(
        /// `m:hideLeft` — whether the left border is hidden.
        hide_left, "hideLeft"
    );
    onoff_accessor!(
        /// `m:hideRight` — whether the right border is hidden.
        hide_right, "hideRight"
    );
    onoff_accessor!(
        /// `m:strikeH` — whether a horizontal strikethrough is drawn.
        strike_horizontal, "strikeH"
    );
    onoff_accessor!(
        /// `m:strikeV` — whether a vertical strikethrough is drawn.
        strike_vertical, "strikeV"
    );
    onoff_accessor!(
        /// `m:strikeBLTR` — whether a bottom-left-to-top-right diagonal strikethrough is drawn.
        strike_bottom_left_to_top_right, "strikeBLTR"
    );
    onoff_accessor!(
        /// `m:strikeTLBR` — whether a top-left-to-bottom-right diagonal strikethrough is drawn.
        strike_top_left_to_bottom_right, "strikeTLBR"
    );
    control_properties_accessor!();
}

fidelity_struct! {
    /// `m:borderBox` (`CT_BorderBox`, §22.1.2.8 "Border Box") — a box with a visible border, optional
    /// strikethroughs.
    pub struct BorderBox
}

impl BorderBox {
    child_accessor!(
        /// `m:borderBoxPr` — this border box's own properties.
        properties, "borderBoxPr", BorderBoxProperties
    );
    child_accessor!(
        /// `m:e` — the boxed argument.
        base, "e", Argument
    );
}

// =================================================================================================
// CT_DPr / CT_D — m:dPr / m:d (Delimiter)
// =================================================================================================

fidelity_struct! {
    /// `m:dPr` (`CT_DPr`, §22.1.2.27) — a delimiter's own bracket characters, growth, shape and
    /// control properties.
    pub struct DelimiterProperties
}

impl DelimiterProperties {
    /// Builds `<m:dPr><m:begChr m:val="{begin}"/><m:endChr m:val="{end}"/></m:dPr>`.
    #[must_use]
    pub fn new(interner: &mut Interner, begin: &str, end: &str) -> Self {
        let children = vec![
            RawNode::Element(crate::support::val_element::<CharacterCodec>(
                interner,
                "begChr",
                mjx_ooxml_types::officemath::Character::from_wire(begin),
            )),
            RawNode::Element(crate::support::val_element::<CharacterCodec>(
                interner,
                "endChr",
                mjx_ooxml_types::officemath::Character::from_wire(end),
            )),
        ];
        Self {
            name: m_name(interner, "dPr"),
            attributes: Vec::new(),
            children,
            empty: false,
        }
    }

    val_accessor!(
        /// `m:begChr` — the opening delimiter character (empty string means none is drawn).
        begin_character, "begChr", CharacterCodec, mjx_ooxml_types::officemath::Character
    );
    val_accessor!(
        /// `m:sepChr` — the separator character between arguments.
        separator_character, "sepChr", CharacterCodec, mjx_ooxml_types::officemath::Character
    );
    val_accessor!(
        /// `m:endChr` — the closing delimiter character (empty string means none is drawn).
        end_character, "endChr", CharacterCodec, mjx_ooxml_types::officemath::Character
    );
    onoff_accessor!(
        /// `m:grow` — whether the delimiters grow to enclose their tallest argument.
        grow, "grow"
    );
    val_accessor!(
        /// `m:shp` (`ST_Shp`, §22.1.3.10) — whether the delimiter is centred on its argument or
        /// matched to the argument's own height ("`centered`"/"`match`" — sourced from the prose,
        /// not the wire token, per this crate's naming convention).
        shape, "shp", Enumeration<DelimiterShape>, DelimiterShape
    );
    control_properties_accessor!();
}

fidelity_struct! {
    /// `m:d` (`CT_D`, §22.1.2.26 "Delimiter") — one or more arguments enclosed in matching (or
    /// mismatched) brackets.
    pub struct Delimiter
}

impl Delimiter {
    /// Builds `<m:d>{properties}{arguments}</m:d>`.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        properties: Option<DelimiterProperties>,
        arguments: Vec<Argument>,
    ) -> Self {
        let mut children = Vec::new();
        if let Some(properties) = properties {
            children.push(RawNode::Element(properties.to_xml(interner)));
        }
        children.extend(
            arguments
                .into_iter()
                .map(|argument| RawNode::Element(argument.to_xml(interner))),
        );
        let empty = children.is_empty();
        Self {
            name: m_name(interner, "d"),
            attributes: Vec::new(),
            children,
            empty,
        }
    }

    child_accessor!(
        /// `m:dPr` — this delimiter's own properties.
        properties, "dPr", DelimiterProperties
    );
    child_list_accessor!(
        /// `m:e` — the enclosed arguments, in order (one or more per the schema).
        arguments, "e", Argument
    );
}

// =================================================================================================
// CT_EqArrPr / CT_EqArr — m:eqArrPr / m:eqArr (Equation Array)
// =================================================================================================

fidelity_struct! {
    /// `m:eqArrPr` (`CT_EqArrPr`, §22.1.2.35) — an equation array's own base alignment, spacing and
    /// control properties.
    pub struct EquationArrayProperties
}

impl EquationArrayProperties {
    val_accessor!(
        /// `m:baseJc` — the vertical alignment of the array's own baseline.
        base_alignment, "baseJc", Enumeration<RelativeVerticalAlignment>, RelativeVerticalAlignment
    );
    onoff_accessor!(
        /// `m:maxDist` — whether row spacing is measured from the tallest row.
        maximum_distribution, "maxDist"
    );
    onoff_accessor!(
        /// `m:objDist` — whether spacing accounts for each row's own descent/ascent.
        object_distribution, "objDist"
    );
    val_accessor!(
        /// `m:rSpRule` — which row-spacing rule applies.
        row_spacing_rule, "rSpRule", Number<i64>, i64
    );
    val_accessor!(
        /// `m:rSp` — the explicit row spacing, in twentieths of a point, when `rSpRule` calls for one.
        row_spacing, "rSp", Number<u32>, u32
    );
    control_properties_accessor!();
}

fidelity_struct! {
    /// `m:eqArr` (`CT_EqArr`, §22.1.2.34 "Equation Array") — a vertical stack of aligned equations.
    pub struct EquationArray
}

impl EquationArray {
    child_accessor!(
        /// `m:eqArrPr` — this array's own properties.
        properties, "eqArrPr", EquationArrayProperties
    );
    child_list_accessor!(
        /// `m:e` — the array's own rows, in order.
        arguments, "e", Argument
    );
}

// =================================================================================================
// CT_FPr / CT_F — m:fPr / m:f (Fraction)
// =================================================================================================

fidelity_struct! {
    /// `m:fPr` (`CT_FPr`, §22.1.2.41) — a fraction's own bar style and control properties.
    pub struct FractionProperties
}

impl FractionProperties {
    /// Builds `<m:fPr><m:type m:val="{fraction_type}"/></m:fPr>`, or an empty `<m:fPr/>` for `None`.
    #[must_use]
    pub fn new(interner: &mut Interner, fraction_type: Option<FractionType>) -> Self {
        let mut children = Vec::new();
        if let Some(fraction_type) = fraction_type {
            children.push(RawNode::Element(crate::support::val_element::<
                Enumeration<FractionType>,
            >(
                interner, "type", fraction_type
            )));
        }
        let empty = children.is_empty();
        Self {
            name: m_name(interner, "fPr"),
            attributes: Vec::new(),
            children,
            empty,
        }
    }

    val_accessor!(
        /// `m:type` (`ST_FType`) — the fraction's own bar style (stacked, skewed, linear, or none).
        fraction_type, "type", Enumeration<FractionType>, FractionType
    );
    control_properties_accessor!();
}

fidelity_struct! {
    /// `m:f` (`CT_F`, §22.1.2.40 "Fraction") — a numerator over a denominator.
    pub struct Fraction
}

impl Fraction {
    /// Builds `<m:f><m:num>{numerator}</m:num><m:den>{denominator}</m:den></m:f>`, with no `m:fPr`.
    #[must_use]
    pub fn new(interner: &mut Interner, numerator: Argument, denominator: Argument) -> Self {
        Self {
            name: m_name(interner, "f"),
            attributes: Vec::new(),
            children: vec![
                RawNode::Element(numerator.to_xml(interner)),
                RawNode::Element(denominator.to_xml(interner)),
            ],
            empty: false,
        }
    }

    child_accessor!(
        /// `m:fPr` — this fraction's own properties.
        properties, "fPr", FractionProperties
    );
    child_accessor!(
        /// `m:num` — the numerator. `None` only when the element is malformed (the schema requires
        /// it).
        numerator, "num", Argument
    );
    child_accessor!(
        /// `m:den` — the denominator. `None` only when the element is malformed (the schema requires
        /// it).
        denominator, "den", Argument
    );
}

// =================================================================================================
// CT_FuncPr (ControlOnlyProperties) / CT_Func — m:funcPr / m:func (Function-Apply)
// =================================================================================================

fidelity_struct! {
    /// `m:func` (`CT_Func`, §22.1.2.44 "Function-Apply") — a named function applied to an argument
    /// (`sin x`, `log₂ n`, …).
    pub struct Function
}

impl Function {
    child_accessor!(
        /// `m:funcPr` — this function's own properties.
        properties, "funcPr", ControlOnlyProperties
    );
    child_accessor!(
        /// `m:fName` — the function's own name, itself a math argument (so it can carry formatting).
        function_name, "fName", Argument
    );
    child_accessor!(
        /// `m:e` — the argument the function is applied to.
        base, "e", Argument
    );
}

// =================================================================================================
// CT_GroupChrPr / CT_GroupChr — m:groupChrPr / m:groupChr (Group Character)
// =================================================================================================

fidelity_struct! {
    /// `m:groupChrPr` (`CT_GroupChrPr`, §22.1.2.49) — a group character's own glyph, position and
    /// control properties.
    pub struct GroupCharacterProperties
}

impl GroupCharacterProperties {
    val_accessor!(
        /// `m:chr` — the grouping character drawn above/below the base.
        character, "chr", CharacterCodec, mjx_ooxml_types::officemath::Character
    );
    val_accessor!(
        /// `m:pos` — whether the character is drawn above or below its base.
        position, "pos", Enumeration<TopBottom>, TopBottom
    );
    val_accessor!(
        /// `m:vertJc` — the vertical justification of any accompanying text relative to the base.
        vertical_justification, "vertJc", Enumeration<TopBottom>, TopBottom
    );
    control_properties_accessor!();
}

fidelity_struct! {
    /// `m:groupChr` (`CT_GroupChr`, §22.1.2.48 "Group Character") — a grouping symbol (brace, arrow,
    /// …) drawn above or below its base, e.g. an over/underbrace.
    pub struct GroupCharacter
}

impl GroupCharacter {
    child_accessor!(
        /// `m:groupChrPr` — this group character's own properties.
        properties, "groupChrPr", GroupCharacterProperties
    );
    child_accessor!(
        /// `m:e` — the grouped base.
        base, "e", Argument
    );
}

// =================================================================================================
// CT_LimLowPr (ControlOnlyProperties) / CT_LimLow — m:limLowPr / m:limLow (Lower-Limit)
// =================================================================================================

fidelity_struct! {
    /// `m:limLow` (`CT_LimLow`, §22.1.2.55 "Lower-Limit") — a base with a limit drawn below it.
    pub struct LowerLimit
}

impl LowerLimit {
    child_accessor!(
        /// `m:limLowPr` — this lower limit's own properties.
        properties, "limLowPr", ControlOnlyProperties
    );
    child_accessor!(
        /// `m:e` — the base.
        base, "e", Argument
    );
    child_accessor!(
        /// `m:lim` — the limit drawn below the base.
        limit, "lim", Argument
    );
}

// =================================================================================================
// CT_LimUppPr (ControlOnlyProperties) / CT_LimUpp — m:limUppPr / m:limUpp (Upper-Limit)
// =================================================================================================

fidelity_struct! {
    /// `m:limUpp` (`CT_LimUpp`, §22.1.2.57 "Upper-Limit") — a base with a limit drawn above it.
    pub struct UpperLimit
}

impl UpperLimit {
    child_accessor!(
        /// `m:limUppPr` — this upper limit's own properties.
        properties, "limUppPr", ControlOnlyProperties
    );
    child_accessor!(
        /// `m:e` — the base.
        base, "e", Argument
    );
    child_accessor!(
        /// `m:lim` — the limit drawn above the base.
        limit, "lim", Argument
    );
}

// =================================================================================================
// CT_MCPr / CT_MC / CT_MCS — m:mcPr / m:mc / m:mcs (Matrix Column properties)
// =================================================================================================

fidelity_struct! {
    /// `m:mcPr` (`CT_MCPr`, §22.1.2.61) — one matrix-column-properties entry: how many columns it
    /// applies to, and their justification.
    pub struct MatrixColumnProperties
}

impl MatrixColumnProperties {
    val_accessor!(
        /// `m:count` (`ST_Integer255`) — how many consecutive columns this entry governs.
        count, "count", Number<i64>, i64
    );
    val_accessor!(
        /// `m:mcJc` — the horizontal justification these columns share.
        justification, "mcJc", Enumeration<RelativeHorizontalAlignment>, RelativeHorizontalAlignment
    );
}

fidelity_struct! {
    /// `m:mc` (`CT_MC`, §22.1.2.60 "Matrix Column") — one matrix-column-properties wrapper.
    pub struct MatrixColumn
}

impl MatrixColumn {
    child_accessor!(
        /// `m:mcPr` — this column's own properties.
        properties, "mcPr", MatrixColumnProperties
    );
}

fidelity_struct! {
    /// `m:mcs` (`CT_MCS`, §22.1.2.62 "Matrix Column properties") — the matrix's own list of
    /// column-properties entries, one or more per the schema.
    pub struct MatrixColumns
}

impl MatrixColumns {
    child_list_accessor!(
        /// `m:mc` — the column-properties entries, in order.
        columns, "mc", MatrixColumn
    );
}

// =================================================================================================
// CT_MPr / CT_MR / CT_M — m:mPr / m:mr / m:m (Matrix)
// =================================================================================================

fidelity_struct! {
    /// `m:mPr` (`CT_MPr`, §22.1.2.65) — a matrix's own baseline, placeholder, spacing, column
    /// properties and control properties.
    pub struct MatrixProperties
}

impl MatrixProperties {
    val_accessor!(
        /// `m:baseJc` — the vertical alignment of the matrix's own baseline.
        base_alignment, "baseJc", Enumeration<RelativeVerticalAlignment>, RelativeVerticalAlignment
    );
    onoff_accessor!(
        /// `m:plcHide` — whether placeholder dots in empty cells are hidden.
        placeholder_hide, "plcHide"
    );
    val_accessor!(
        /// `m:rSpRule` — which row-spacing rule applies.
        row_spacing_rule, "rSpRule", Number<i64>, i64
    );
    val_accessor!(
        /// `m:cGpRule` — which column-gap rule applies.
        column_gap_rule, "cGpRule", Number<i64>, i64
    );
    val_accessor!(
        /// `m:rSp` — the explicit row spacing, in twentieths of a point.
        row_spacing, "rSp", Number<u32>, u32
    );
    val_accessor!(
        /// `m:cSp` — the explicit column spacing, in twentieths of a point.
        column_spacing, "cSp", Number<u32>, u32
    );
    val_accessor!(
        /// `m:cGp` — the explicit column gap, in twentieths of a point.
        column_gap, "cGp", Number<u32>, u32
    );
    child_accessor!(
        /// `m:mcs` — this matrix's own per-column properties, if it declares any.
        column_properties, "mcs", MatrixColumns
    );
    control_properties_accessor!();
}

fidelity_struct! {
    /// `m:mr` (`CT_MR`, §22.1.2.66 "Matrix Row") — one row of matrix cells.
    pub struct MatrixRow
}

impl MatrixRow {
    /// Builds `<m:mr>{cells}</m:mr>`.
    #[must_use]
    pub fn new(interner: &mut Interner, cells: Vec<Argument>) -> Self {
        let children = cells
            .into_iter()
            .map(|cell| RawNode::Element(cell.to_xml(interner)))
            .collect();
        Self {
            name: m_name(interner, "mr"),
            attributes: Vec::new(),
            children,
            empty: false,
        }
    }

    child_list_accessor!(
        /// `m:e` — this row's own cells, in order (one or more per the schema).
        cells, "e", Argument
    );
}

fidelity_struct! {
    /// `m:m` (`CT_M`, §22.1.2.59 "Matrix") — a rectangular grid of math arguments.
    pub struct Matrix
}

impl Matrix {
    /// Builds `<m:m>{rows}</m:m>`, with no `m:mPr`.
    #[must_use]
    pub fn new(interner: &mut Interner, rows: Vec<MatrixRow>) -> Self {
        let children = rows
            .into_iter()
            .map(|row| RawNode::Element(row.to_xml(interner)))
            .collect();
        Self {
            name: m_name(interner, "m"),
            attributes: Vec::new(),
            children,
            empty: false,
        }
    }

    child_accessor!(
        /// `m:mPr` — this matrix's own properties.
        properties, "mPr", MatrixProperties
    );
    child_list_accessor!(
        /// `m:mr` — this matrix's own rows, in order (one or more per the schema).
        rows, "mr", MatrixRow
    );
}

// =================================================================================================
// CT_NaryPr / CT_Nary — m:naryPr / m:nary (n-ary Operator)
// =================================================================================================

fidelity_struct! {
    /// `m:naryPr` (`CT_NaryPr`, §22.1.2.75) — an n-ary operator's own glyph, limit placement, growth
    /// and control properties.
    pub struct NaryOperatorProperties
}

impl NaryOperatorProperties {
    /// Builds `<m:naryPr><m:chr m:val="{character}"/></m:naryPr>` — the one field every n-ary
    /// operator in this crate's own tests sets; every other field stays at its schema default.
    #[must_use]
    pub fn new(interner: &mut Interner, character: &str) -> Self {
        let children = vec![RawNode::Element(crate::support::val_element::<
            CharacterCodec,
        >(
            interner,
            "chr",
            mjx_ooxml_types::officemath::Character::from_wire(character),
        ))];
        Self {
            name: m_name(interner, "naryPr"),
            attributes: Vec::new(),
            children,
            empty: false,
        }
    }

    val_accessor!(
        /// `m:chr` — the n-ary operator glyph (∑, ∏, ∫, …).
        character, "chr", CharacterCodec, mjx_ooxml_types::officemath::Character
    );
    val_accessor!(
        /// `m:limLoc` (`ST_LimLoc`, §22.1.3.8) — whether limits are drawn directly above/below the
        /// operator or as trailing sub/superscripts ("`undOvr`"/"`subSup`" — sourced from the prose).
        limit_location, "limLoc", Enumeration<LimitLocation>, LimitLocation
    );
    onoff_accessor!(
        /// `m:grow` — whether the operator glyph grows to match its operand's height.
        grow, "grow"
    );
    onoff_accessor!(
        /// `m:subHide` — whether the lower limit is hidden.
        subscript_hide, "subHide"
    );
    onoff_accessor!(
        /// `m:supHide` — whether the upper limit is hidden.
        superscript_hide, "supHide"
    );
    control_properties_accessor!();
}

fidelity_struct! {
    /// `m:nary` (`CT_Nary`, §22.1.2.74 "n-ary Operator") — a large operator (∑, ∏, ∫, ⋃, …) with an
    /// optional lower and upper limit and one operand.
    pub struct NaryOperator
}

impl NaryOperator {
    /// Builds `<m:nary><m:naryPr>{properties}</m:naryPr><m:sub>{lower_limit}</m:sub><m:sup>{upper_limit}</m:sup><m:e>{operand}</m:e></m:nary>`.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        properties: Option<NaryOperatorProperties>,
        lower_limit: Argument,
        upper_limit: Argument,
        operand: Argument,
    ) -> Self {
        let mut children = Vec::new();
        if let Some(properties) = properties {
            children.push(RawNode::Element(properties.to_xml(interner)));
        }
        children.push(RawNode::Element(lower_limit.to_xml(interner)));
        children.push(RawNode::Element(upper_limit.to_xml(interner)));
        children.push(RawNode::Element(operand.to_xml(interner)));
        Self {
            name: m_name(interner, "nary"),
            attributes: Vec::new(),
            children,
            empty: false,
        }
    }

    child_accessor!(
        /// `m:naryPr` — this operator's own properties.
        properties, "naryPr", NaryOperatorProperties
    );
    child_accessor!(
        /// `m:sub` — the lower limit. `None` only when the element is malformed (the schema requires
        /// it, though [`NaryOperatorProperties::subscript_hide`] may hide it visually).
        lower_limit, "sub", Argument
    );
    child_accessor!(
        /// `m:sup` — the upper limit. `None` only when the element is malformed.
        upper_limit, "sup", Argument
    );
    child_accessor!(
        /// `m:e` — the operand. `None` only when the element is malformed.
        operand, "e", Argument
    );
}

// =================================================================================================
// CT_PhantPr / CT_Phant — m:phantPr / m:phant (Phantom)
// =================================================================================================

fidelity_struct! {
    /// `m:phantPr` (`CT_PhantPr`, §22.1.2.80) — a phantom's own visibility and metric overrides.
    pub struct PhantomProperties
}

impl PhantomProperties {
    onoff_accessor!(
        /// `m:show` — whether the phantom's contents are drawn (invisible but still occupying space
        /// when `false`).
        show, "show"
    );
    onoff_accessor!(
        /// `m:zeroWid` — whether the phantom is given zero width.
        zero_width, "zeroWid"
    );
    onoff_accessor!(
        /// `m:zeroAsc` — whether the phantom is given zero ascent.
        zero_ascent, "zeroAsc"
    );
    onoff_accessor!(
        /// `m:zeroDesc` — whether the phantom is given zero descent.
        zero_descent, "zeroDesc"
    );
    onoff_accessor!(
        /// `m:transp` — whether the phantom's contents are drawn transparently.
        transparent, "transp"
    );
    control_properties_accessor!();
}

fidelity_struct! {
    /// `m:phant` (`CT_Phant`, §22.1.2.79 "Phantom") — a base whose visibility/metrics can be
    /// overridden without changing its layout contribution.
    pub struct Phantom
}

impl Phantom {
    child_accessor!(
        /// `m:phantPr` — this phantom's own properties.
        properties, "phantPr", PhantomProperties
    );
    child_accessor!(
        /// `m:e` — the phantom base.
        base, "e", Argument
    );
}

// =================================================================================================
// CT_RadPr / CT_Rad — m:radPr / m:rad (Radical)
// =================================================================================================

fidelity_struct! {
    /// `m:radPr` (`CT_RadPr`, §22.1.2.86) — a radical's own degree visibility and control properties.
    pub struct RadicalProperties
}

impl RadicalProperties {
    /// Builds `<m:radPr><m:degHide/></m:radPr>` when `degree_hidden` is `true` (or with an explicit
    /// `m:val="false"` when `Some(false)`), or an empty `<m:radPr/>` for `None`.
    #[must_use]
    pub fn new(interner: &mut Interner, degree_hidden: Option<bool>) -> Self {
        let mut children = Vec::new();
        if let Some(hidden) = degree_hidden {
            children.push(RawNode::Element(crate::leaf::onoff_element(
                interner, "degHide", hidden,
            )));
        }
        let empty = children.is_empty();
        Self {
            name: m_name(interner, "radPr"),
            attributes: Vec::new(),
            children,
            empty,
        }
    }

    onoff_accessor!(
        /// `m:degHide` — whether the degree is hidden (a plain square root).
        degree_hide, "degHide"
    );
    control_properties_accessor!();
}

fidelity_struct! {
    /// `m:rad` (`CT_Rad`, §22.1.2.84 "Radical") — a root symbol (`√`) with an optional degree and a
    /// radicand.
    pub struct Radical
}

impl Radical {
    /// Builds `<m:rad><m:deg>{degree}</m:deg><m:e>{radicand}</m:e></m:rad>`, with no `m:radPr`.
    #[must_use]
    pub fn new(interner: &mut Interner, degree: Argument, radicand: Argument) -> Self {
        Self {
            name: m_name(interner, "rad"),
            attributes: Vec::new(),
            children: vec![
                RawNode::Element(degree.to_xml(interner)),
                RawNode::Element(radicand.to_xml(interner)),
            ],
            empty: false,
        }
    }

    child_accessor!(
        /// `m:radPr` — this radical's own properties.
        properties, "radPr", RadicalProperties
    );
    child_accessor!(
        /// `m:deg` — the radical's own degree (empty/hidden for a plain square root). `None` only
        /// when the element is malformed (the schema requires it, though it may be empty).
        degree, "deg", Argument
    );
    child_accessor!(
        /// `m:e` — the radicand. `None` only when the element is malformed.
        radicand, "e", Argument
    );
}

// =================================================================================================
// CT_SPrePr (ControlOnlyProperties) / CT_SPre — m:sPrePr / m:sPre (Pre-Sub-Superscript)
// =================================================================================================

fidelity_struct! {
    /// `m:sPre` (`CT_SPre`, §22.1.2.90 "Pre-Sub-Superscript") — a base with a subscript and
    /// superscript drawn *before* it, e.g. an isotope's mass and atomic number.
    pub struct PreScript
}

impl PreScript {
    child_accessor!(
        /// `m:sPrePr` — this pre-script's own properties.
        properties, "sPrePr", ControlOnlyProperties
    );
    child_accessor!(
        /// `m:sub` — the leading subscript. `None` only when the element is malformed.
        subscript, "sub", Argument
    );
    child_accessor!(
        /// `m:sup` — the leading superscript. `None` only when the element is malformed.
        superscript, "sup", Argument
    );
    child_accessor!(
        /// `m:e` — the base. `None` only when the element is malformed.
        base, "e", Argument
    );
}

// =================================================================================================
// CT_SSubPr (ControlOnlyProperties) / CT_SSub — m:sSubPr / m:sSub (Subscript)
// =================================================================================================

fidelity_struct! {
    /// `m:sSub` (`CT_SSub`, §22.1.2.88 "Subscript") — a base with a trailing subscript.
    pub struct Subscript
}

impl Subscript {
    /// Builds `<m:sSub><m:e>{base}</m:e><m:sub>{subscript}</m:sub></m:sSub>`.
    #[must_use]
    pub fn new(interner: &mut Interner, base: Argument, subscript: Argument) -> Self {
        Self {
            name: m_name(interner, "sSub"),
            attributes: Vec::new(),
            children: vec![
                RawNode::Element(base.to_xml(interner)),
                RawNode::Element(subscript.to_xml(interner)),
            ],
            empty: false,
        }
    }

    child_accessor!(
        /// `m:sSubPr` — this subscript's own properties.
        properties, "sSubPr", ControlOnlyProperties
    );
    child_accessor!(
        /// `m:e` — the base. `None` only when the element is malformed.
        base, "e", Argument
    );
    child_accessor!(
        /// `m:sub` — the subscript. `None` only when the element is malformed.
        subscript, "sub", Argument
    );
}

// =================================================================================================
// CT_SSubSupPr / CT_SSubSup — m:sSubSupPr / m:sSubSup (Subscript-Superscript)
// =================================================================================================

fidelity_struct! {
    /// `m:sSubSupPr` (`CT_SSubSupPr`, §22.1.2.93) — a combined sub/superscript's own alignment and
    /// control properties.
    pub struct SubscriptSuperscriptProperties
}

impl SubscriptSuperscriptProperties {
    onoff_accessor!(
        /// `m:alnScr` — whether the subscript and superscript are vertically aligned with each other.
        aligned_scripts, "alnScr"
    );
    control_properties_accessor!();
}

fidelity_struct! {
    /// `m:sSubSup` (`CT_SSubSup`, §22.1.2.92 "Subscript-Superscript") — a base with both a trailing
    /// subscript and superscript.
    pub struct SubscriptSuperscript
}

impl SubscriptSuperscript {
    child_accessor!(
        /// `m:sSubSupPr` — this combined script's own properties.
        properties, "sSubSupPr", SubscriptSuperscriptProperties
    );
    child_accessor!(
        /// `m:e` — the base. `None` only when the element is malformed.
        base, "e", Argument
    );
    child_accessor!(
        /// `m:sub` — the subscript. `None` only when the element is malformed.
        subscript, "sub", Argument
    );
    child_accessor!(
        /// `m:sup` — the superscript. `None` only when the element is malformed.
        superscript, "sup", Argument
    );
}

// =================================================================================================
// CT_SSupPr (ControlOnlyProperties) / CT_SSup — m:sSupPr / m:sSup (Superscript)
// =================================================================================================

fidelity_struct! {
    /// `m:sSup` (`CT_SSup`, §22.1.2.95 "Superscript") — a base with a trailing superscript.
    pub struct Superscript
}

impl Superscript {
    /// Builds `<m:sSup><m:e>{base}</m:e><m:sup>{superscript}</m:sup></m:sSup>`.
    #[must_use]
    pub fn new(interner: &mut Interner, base: Argument, superscript: Argument) -> Self {
        Self {
            name: m_name(interner, "sSup"),
            attributes: Vec::new(),
            children: vec![
                RawNode::Element(base.to_xml(interner)),
                RawNode::Element(superscript.to_xml(interner)),
            ],
            empty: false,
        }
    }

    child_accessor!(
        /// `m:sSupPr` — this superscript's own properties.
        properties, "sSupPr", ControlOnlyProperties
    );
    child_accessor!(
        /// `m:e` — the base. `None` only when the element is malformed.
        base, "e", Argument
    );
    child_accessor!(
        /// `m:sup` — the superscript. `None` only when the element is malformed.
        superscript, "sup", Argument
    );
}

// =================================================================================================
// CT_R — m:r (Run)
// =================================================================================================

fidelity_struct! {
    /// `m:r` (`CT_R`, §22.1.2.87 "Run") — one run of math content: Office-Math run properties
    /// (`m:rPr`), then a sequence of `m:t` text runs interleaved with WordprocessingML run inner
    /// content (`w:br`, `w:drawing`, …) this crate cannot type — see the crate's own module doc
    /// comment. The latter round-trips through this struct's raw `children` untouched; only `m:rPr`
    /// and `m:t` get typed accessors.
    pub struct Run
}

impl Run {
    /// Builds `<m:r><m:t>{text}</m:t></m:r>`.
    #[must_use]
    pub fn new(interner: &mut Interner, text: &str) -> Self {
        let run_text = crate::leaf::Text::new(interner, text);
        Self {
            name: m_name(interner, "r"),
            attributes: Vec::new(),
            children: vec![RawNode::Element(run_text.to_xml(interner))],
            empty: false,
        }
    }

    child_accessor!(
        /// `m:rPr` — this run's own Office-Math properties.
        properties, "rPr", crate::leaf::RunProperties
    );
    child_list_accessor!(
        /// `m:t` — this run's own text nodes, in order. Almost always exactly one; more than one is
        /// legal (`EG_RunInnerContent`'s `t` choice has no `maxOccurs` cap) and round-trips as such.
        text_runs, "t", crate::leaf::Text
    );

    /// This run's own visible text: every [`Text::text`](crate::leaf::Text::text) concatenated, in
    /// document order.
    #[must_use]
    pub fn text(&self, interner: &Interner) -> String {
        self.text_runs(interner)
            .iter()
            .map(crate::leaf::Text::text)
            .collect()
    }
}

// =================================================================================================
// MathElement — EG_OMathMathElements, the 20-way choice every math object's own arguments hold.
// =================================================================================================

/// One child of `EG_OMathMathElements`/`EG_OMathElements` — the 20-way choice `m:oMath`'s and
/// `m:oMathArg`'s own content is made of. Every variant name is comprehensive per this project's
/// naming convention (`Fraction`, never `F`; `NaryOperator`, never `Nary`).
///
/// [`MathElement::from_node`] and [`MathElement::from_children`] are **read-only projections**: a
/// [`crate::math::Math`]/[`Argument`] keeps its own children as raw nodes regardless, so nothing here
/// needs a `ToXml` — the projection changes nothing about how the owning element serializes.
/// `EG_PContentMath`'s own WordprocessingML-typed fallback content (a `w:sdt`, `w:customXml`, …) is
/// simply not produced by this projection and stays exactly where it was in the raw tree, exactly as
/// a run's own WordprocessingML inner content does in [`Run`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MathElement {
    /// `m:acc`.
    Accent(Accent),
    /// `m:bar`.
    Bar(Bar),
    /// `m:box`.
    Box(MathBox),
    /// `m:borderBox`.
    BorderBox(BorderBox),
    /// `m:d`.
    Delimiter(Delimiter),
    /// `m:eqArr`.
    EquationArray(EquationArray),
    /// `m:f`.
    Fraction(Fraction),
    /// `m:func`.
    Function(Function),
    /// `m:groupChr`.
    GroupCharacter(GroupCharacter),
    /// `m:limLow`.
    LowerLimit(LowerLimit),
    /// `m:limUpp`.
    UpperLimit(UpperLimit),
    /// `m:m`.
    Matrix(Matrix),
    /// `m:nary`.
    NaryOperator(NaryOperator),
    /// `m:phant`.
    Phantom(Phantom),
    /// `m:rad`.
    Radical(Radical),
    /// `m:sPre`.
    PreScript(PreScript),
    /// `m:sSub`.
    Subscript(Subscript),
    /// `m:sSubSup`.
    SubscriptSuperscript(SubscriptSuperscript),
    /// `m:sSup`.
    Superscript(Superscript),
    /// `m:r`.
    Run(Run),
}

impl MathElement {
    /// Reads `node` as a [`MathElement`], or `None` if it is not an `m:`-namespaced element this
    /// choice names (whitespace text, a comment, or `EG_PContentMath`'s WordprocessingML fallback).
    #[must_use]
    pub fn from_node(node: &RawNode, interner: &Interner) -> Option<Self> {
        let RawNode::Element(element) = node else {
            return None;
        };
        if !crate::support::is_m(&element.name, interner) {
            return None;
        }
        macro_rules! try_variant {
            ($local:literal, $variant:ident, $ty:ty) => {
                if interner.resolve(element.name.local) == $local {
                    return <$ty as FromXml>::from_xml(element, interner)
                        .ok()
                        .map(MathElement::$variant);
                }
            };
        }
        try_variant!("acc", Accent, Accent);
        try_variant!("bar", Bar, Bar);
        try_variant!("box", Box, MathBox);
        try_variant!("borderBox", BorderBox, BorderBox);
        try_variant!("d", Delimiter, Delimiter);
        try_variant!("eqArr", EquationArray, EquationArray);
        try_variant!("f", Fraction, Fraction);
        try_variant!("func", Function, Function);
        try_variant!("groupChr", GroupCharacter, GroupCharacter);
        try_variant!("limLow", LowerLimit, LowerLimit);
        try_variant!("limUpp", UpperLimit, UpperLimit);
        try_variant!("m", Matrix, Matrix);
        try_variant!("nary", NaryOperator, NaryOperator);
        try_variant!("phant", Phantom, Phantom);
        try_variant!("rad", Radical, Radical);
        try_variant!("sPre", PreScript, PreScript);
        try_variant!("sSub", Subscript, Subscript);
        try_variant!("sSubSup", SubscriptSuperscript, SubscriptSuperscript);
        try_variant!("sSup", Superscript, Superscript);
        try_variant!("r", Run, Run);
        None
    }

    /// Reads every [`MathElement`] `children` holds, in document order, skipping anything
    /// [`MathElement::from_node`] does not recognise.
    #[must_use]
    pub fn from_children(children: &[RawNode], interner: &Interner) -> Vec<Self> {
        children
            .iter()
            .filter_map(|node| Self::from_node(node, interner))
            .collect()
    }

    /// Rebuilds this element's own wire form — the write side of [`MathElement::from_node`], used by
    /// [`Argument::new`](crate::arg::Argument::new) to assemble freshly authored content.
    #[must_use]
    pub fn to_xml(&self, interner: &mut Interner) -> mjx_ooxml_core::RawElement {
        match self {
            MathElement::Accent(v) => v.to_xml(interner),
            MathElement::Bar(v) => v.to_xml(interner),
            MathElement::Box(v) => v.to_xml(interner),
            MathElement::BorderBox(v) => v.to_xml(interner),
            MathElement::Delimiter(v) => v.to_xml(interner),
            MathElement::EquationArray(v) => v.to_xml(interner),
            MathElement::Fraction(v) => v.to_xml(interner),
            MathElement::Function(v) => v.to_xml(interner),
            MathElement::GroupCharacter(v) => v.to_xml(interner),
            MathElement::LowerLimit(v) => v.to_xml(interner),
            MathElement::UpperLimit(v) => v.to_xml(interner),
            MathElement::Matrix(v) => v.to_xml(interner),
            MathElement::NaryOperator(v) => v.to_xml(interner),
            MathElement::Phantom(v) => v.to_xml(interner),
            MathElement::Radical(v) => v.to_xml(interner),
            MathElement::PreScript(v) => v.to_xml(interner),
            MathElement::Subscript(v) => v.to_xml(interner),
            MathElement::SubscriptSuperscript(v) => v.to_xml(interner),
            MathElement::Superscript(v) => v.to_xml(interner),
            MathElement::Run(v) => v.to_xml(interner),
        }
    }
}
