//! The seventy-one value enumerations, projected one variant at a time.
//!
//! Every enumeration in the [`mjx_ooxml`] vocabulary that carries no payload becomes a Python class
//! of the same name with the same members — `mjx_ooxml.TextAlignment.Center`, not the string
//! `"ctr"`. A class gives completion, `repr`, identity comparison and a `mypy --strict` error when
//! a caller invents a member; a string gives none of that and fails at run time in the middle of an
//! edit.
//!
//! # The one renamed member
//!
//! Rust's `None` variant is spelled **`NONE`** in Python, because `None` is a Python keyword and
//! `TextUnderline.None` is a syntax error, not a lookup. That is the only name this module changes;
//! the TypeScript binding, where `None` is a legal member name, keeps it. Nine enumerations are
//! affected: `FontCollectionIndex`, `LineEndType`, `PathFillMode`, `PictureFillMode`,
//! `ScatterStyle`, `TextCapitalization`, `TextUnderline`, `TickLabelPosition` and `TickMark`.
//!
//! # Sealed and open
//!
//! Most of these enumerations are closed: a variant added below stops this file compiling until it
//! is projected, which is exactly the discipline `mjx_ooxml::Error`'s own mapping uses. Nine are
//! `#[non_exhaustive]` upstream and so cannot be matched exhaustively from here; for those,
//! [`from_model`](TablePart::from_model) raises [`UnsupportedContentError`] rather than inventing a
//! member. It has never fired — the projection is complete for every variant that exists today, and
//! `tests/test_enums.py` checks the member counts — but a binding compiled against an older
//! `mjx-ooxml` than it runs beside would otherwise have to guess.
//!
//! [`UnsupportedContentError`]: crate::errors

use pyo3::prelude::*;
use pyo3::types::PyModule;

use mjx_ooxml as ooxml;

use crate::errors::unsupported_content;

/// Projects one payload-free enumeration whose upstream is **closed**: the inbound match names
/// every variant, so adding one upstream is a compile error here.
macro_rules! sealed_enums {
    ($(
        $(#[$attribute:meta])*
        $name:ident { $($(#[$variant_attribute:meta])* $variant:ident),+ $(,)? }
    )*) => {
        $(
            $(#[$attribute])*
            #[pyclass(eq, eq_int, frozen, from_py_object, module = "mjx_ooxml")]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            pub enum $name {
                $(
                    #[doc = concat!(
                        "[`mjx_ooxml::", stringify!($name), "::", stringify!($variant), "`]."
                    )]
                    $(#[$variant_attribute])*
                    $variant,
                )+
            }

            impl From<$name> for ooxml::$name {
                fn from(value: $name) -> Self {
                    match value { $($name::$variant => Self::$variant),+ }
                }
            }

            impl $name {
                /// The model's value as this class's member. Never fails: the upstream enumeration
                /// is closed and every variant is named above.
                pub fn from_model(value: ooxml::$name) -> PyResult<Self> {
                    Ok(match value { $(ooxml::$name::$variant => Self::$variant),+ })
                }
            }
        )*

        /// Registers every closed enumeration on the module.
        fn register_sealed(module: &Bound<'_, PyModule>) -> PyResult<()> {
            $( module.add_class::<$name>()?; )*
            Ok(())
        }
    };
}

/// Projects one payload-free enumeration whose upstream is `#[non_exhaustive]`. The outbound
/// direction is still total; the inbound one raises rather than guessing.
macro_rules! open_enums {
    ($(
        $(#[$attribute:meta])*
        $name:ident { $($(#[$variant_attribute:meta])* $variant:ident),+ $(,)? }
    )*) => {
        $(
            $(#[$attribute])*
            #[pyclass(eq, eq_int, frozen, from_py_object, module = "mjx_ooxml")]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            pub enum $name {
                $(
                    #[doc = concat!(
                        "[`mjx_ooxml::", stringify!($name), "::", stringify!($variant), "`]."
                    )]
                    $(#[$variant_attribute])*
                    $variant,
                )+
            }

            impl From<$name> for ooxml::$name {
                fn from(value: $name) -> Self {
                    match value { $($name::$variant => Self::$variant),+ }
                }
            }

            impl $name {
                /// The model's value as this class's member, or an `UnsupportedContentError` if the
                /// model has grown a variant this build does not project.
                pub fn from_model(value: ooxml::$name) -> PyResult<Self> {
                    Ok(match value {
                        $(ooxml::$name::$variant => Self::$variant,)+
                        _ => return Err(unsupported_content(concat!(
                            "this build of the bindings does not project every `",
                            stringify!($name),
                            "` the document holds"
                        ))),
                    })
                }
            }
        )*

        /// Registers every open enumeration on the module.
        fn register_open(module: &Bound<'_, PyModule>) -> PyResult<()> {
            $( module.add_class::<$name>()?; )*
            Ok(())
        }
    };
}

/// Adds every enumeration in this module to the extension module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_sealed(module)?;
    register_open(module)
}

sealed_enums! {
    /// The projection of [`mjx_ooxml::AdjustmentAxis`], whose documentation is authoritative.
    AdjustmentAxis {
        Horizontal,
        Vertical,
        Angle,
        Radius,
    }
    /// The projection of [`mjx_ooxml::AutonumberScheme`], whose documentation is authoritative.
    AutonumberScheme {
        LowercaseLetterParenthesesBoth,
        UppercaseLetterParenthesesBoth,
        LowercaseLetterParenthesisRight,
        UppercaseLetterParenthesisRight,
        LowercaseLetterPeriod,
        UppercaseLetterPeriod,
        ArabicParenthesesBoth,
        ArabicParenthesisRight,
        ArabicPeriod,
        ArabicPlain,
        LowercaseRomanParenthesesBoth,
        UppercaseRomanParenthesesBoth,
        LowercaseRomanParenthesisRight,
        UppercaseRomanParenthesisRight,
        LowercaseRomanPeriod,
        UppercaseRomanPeriod,
        DoubleByteCircledNumberPlain,
        WingdingsBlackCircledNumberPlain,
        WingdingsWhiteCircledNumberPlain,
        DoubleByteArabicPeriod,
        DoubleByteArabicPlain,
        SimplifiedChinesePeriod,
        SimplifiedChinesePlain,
        TraditionalChinesePeriod,
        TraditionalChinesePlain,
        JapaneseDoubleBytePeriod,
        JapaneseKoreanPlain,
        JapaneseKoreanPeriod,
        BidirectionalArabicAlphabeticMinus,
        BidirectionalArabicAbjadMinus,
        BidirectionalHebrewMinus,
        ThaiLetterPeriod,
        ThaiLetterParenthesisRight,
        ThaiLetterParenthesesBoth,
        ThaiNumberPeriod,
        ThaiNumberParenthesisRight,
        ThaiNumberParenthesesBoth,
        HindiVowelPeriod,
        HindiNumberPeriod,
        HindiNumberParenthesisRight,
        HindiConsonantPeriod,
    }
    /// The projection of [`mjx_ooxml::AxisKind`], whose documentation is authoritative.
    AxisKind {
        Category,
        Value,
        Date,
        Series,
    }
    /// The projection of [`mjx_ooxml::AxisOrientation`], whose documentation is authoritative.
    AxisOrientation {
        MinimumToMaximum,
        MaximumToMinimum,
    }
    /// The projection of [`mjx_ooxml::AxisPosition`], whose documentation is authoritative.
    AxisPosition {
        Bottom,
        Left,
        Right,
        Top,
    }
    /// The projection of [`mjx_ooxml::BarDirection`], whose documentation is authoritative.
    BarDirection {
        Column,
        Bar,
    }
    /// The projection of [`mjx_ooxml::BarGrouping`], whose documentation is authoritative.
    BarGrouping {
        Clustered,
        Stacked,
        PercentStacked,
        Standard,
    }
    /// The projection of [`mjx_ooxml::BevelPreset`], whose documentation is authoritative.
    BevelPreset {
        RelaxedInset,
        Circle,
        Slope,
        Cross,
        Angle,
        SoftRound,
        Convex,
        CoolSlant,
        Divot,
        Riblet,
        HardEdge,
        ArtDeco,
    }
    /// The projection of [`mjx_ooxml::BlankDisplay`], whose documentation is authoritative.
    BlankDisplay {
        Span,
        Gap,
        Zero,
    }
    /// The projection of [`mjx_ooxml::BlendMode`], whose documentation is authoritative.
    BlendMode {
        Over,
        Multiply,
        Screen,
        Darken,
        Lighten,
    }
    /// The projection of [`mjx_ooxml::ColorKind`], whose documentation is authoritative.
    ColorKind {
        Srgb,
        ScRgb,
        Hsl,
        System,
        Scheme,
        Preset,
        Unknown,
    }
    /// The projection of [`mjx_ooxml::ColorSchemeSlot`], whose documentation is authoritative.
    ColorSchemeSlot {
        Dark1,
        Light1,
        Dark2,
        Light2,
        Accent1,
        Accent2,
        Accent3,
        Accent4,
        Accent5,
        Accent6,
        Hyperlink,
        FollowedHyperlink,
    }
    /// The projection of [`mjx_ooxml::CompoundLine`], whose documentation is authoritative.
    CompoundLine {
        Single,
        Double,
        ThickThin,
        ThinThick,
        Triple,
    }
    /// The projection of [`mjx_ooxml::DataLabelPosition`], whose documentation is authoritative.
    DataLabelPosition {
        BestFit,
        Bottom,
        Center,
        InsideBase,
        InsideEnd,
        Left,
        OutsideEnd,
        Right,
        Top,
    }
    /// The projection of [`mjx_ooxml::ErrorBarDirection`], whose documentation is authoritative.
    ErrorBarDirection {
        X,
        Y,
    }
    /// The projection of [`mjx_ooxml::ErrorBarType`], whose documentation is authoritative.
    ErrorBarType {
        Both,
        Minus,
        Plus,
    }
    /// The projection of [`mjx_ooxml::ErrorValueType`], whose documentation is authoritative.
    ErrorValueType {
        Custom,
        FixedValue,
        Percentage,
        StandardDeviation,
        StandardError,
    }
    /// The projection of [`mjx_ooxml::FontAlignment`], whose documentation is authoritative.
    FontAlignment {
        Automatic,
        Top,
        Center,
        Baseline,
        Bottom,
    }
    /// The projection of [`mjx_ooxml::FontCollectionIndex`], whose documentation is authoritative.
    FontCollectionIndex {
        Major,
        Minor,
        #[pyo3(name = "NONE")]
        None,
    }
    /// The projection of [`mjx_ooxml::FontSchemeSlot`], whose documentation is authoritative.
    FontSchemeSlot {
        Major,
        Minor,
    }
    /// The projection of [`mjx_ooxml::FontSlot`], whose documentation is authoritative.
    FontSlot {
        Latin,
        EastAsian,
        ComplexScript,
        Symbol,
    }
    /// The projection of [`mjx_ooxml::LegendPosition`], whose documentation is authoritative.
    LegendPosition {
        Bottom,
        Left,
        Right,
        Top,
        TopRight,
    }
    /// The projection of [`mjx_ooxml::LightRigDirection`], whose documentation is authoritative.
    LightRigDirection {
        TopLeft,
        Top,
        TopRight,
        Left,
        Right,
        BottomLeft,
        Bottom,
        BottomRight,
    }
    /// The projection of [`mjx_ooxml::LightRigType`], whose documentation is authoritative.
    LightRigType {
        LegacyFlat1,
        LegacyFlat2,
        LegacyFlat3,
        LegacyFlat4,
        LegacyNormal1,
        LegacyNormal2,
        LegacyNormal3,
        LegacyNormal4,
        LegacyHarsh1,
        LegacyHarsh2,
        LegacyHarsh3,
        LegacyHarsh4,
        ThreePoint,
        Balanced,
        Soft,
        Harsh,
        Flood,
        Contrasting,
        Morning,
        Sunrise,
        Sunset,
        Chilly,
        Freezing,
        Flat,
        TwoPoint,
        Glow,
        BrightRoom,
    }
    /// The projection of [`mjx_ooxml::LineCap`], whose documentation is authoritative.
    LineCap {
        Round,
        Square,
        Flat,
    }
    /// The projection of [`mjx_ooxml::LineEndLength`], whose documentation is authoritative.
    LineEndLength {
        Small,
        Medium,
        Large,
    }
    /// The projection of [`mjx_ooxml::LineEndType`], whose documentation is authoritative.
    LineEndType {
        #[pyo3(name = "NONE")]
        None,
        Triangle,
        Stealth,
        Diamond,
        Oval,
        Arrow,
    }
    /// The projection of [`mjx_ooxml::LineEndWidth`], whose documentation is authoritative.
    LineEndWidth {
        Small,
        Medium,
        Large,
    }
    /// The projection of [`mjx_ooxml::MediaKind`], whose documentation is authoritative.
    MediaKind {
        Audio,
        Video,
        Media,
    }
    /// The projection of [`mjx_ooxml::OfPieType`], whose documentation is authoritative.
    OfPieType {
        Pie,
        Bar,
    }
    /// The projection of [`mjx_ooxml::OnOffStyle`], whose documentation is authoritative.
    OnOffStyle {
        On,
        Off,
        Default,
    }
    /// The projection of [`mjx_ooxml::Orientation`], whose documentation is authoritative.
    Orientation {
        Horizontal,
        Vertical,
    }
    /// The projection of [`mjx_ooxml::PathFillMode`], whose documentation is authoritative.
    PathFillMode {
        #[pyo3(name = "NONE")]
        None,
        Normal,
        Lighten,
        LightenLess,
        Darken,
        DarkenLess,
    }
    /// The projection of [`mjx_ooxml::PatternType`], whose documentation is authoritative.
    PatternType {
        Percent5,
        Percent10,
        Percent20,
        Percent25,
        Percent30,
        Percent40,
        Percent50,
        Percent60,
        Percent70,
        Percent75,
        Percent80,
        Percent90,
        Horizontal,
        Vertical,
        LightHorizontal,
        LightVertical,
        DarkHorizontal,
        DarkVertical,
        NarrowHorizontal,
        NarrowVertical,
        DashedHorizontal,
        DashedVertical,
        Cross,
        DownwardDiagonal,
        UpwardDiagonal,
        LightDownwardDiagonal,
        LightUpwardDiagonal,
        DarkDownwardDiagonal,
        DarkUpwardDiagonal,
        WideDownwardDiagonal,
        WideUpwardDiagonal,
        DashedDownwardDiagonal,
        DashedUpwardDiagonal,
        DiagonalCross,
        SmallCheckerboard,
        LargeCheckerboard,
        SmallGrid,
        LargeGrid,
        DottedGrid,
        SmallConfetti,
        LargeConfetti,
        HorizontalBrick,
        DiagonalBrick,
        SolidDiamond,
        OpenDiamond,
        DottedDiamond,
        Plaid,
        Sphere,
        Weave,
        Divot,
        Shingle,
        Wave,
        Trellis,
        ZigZag,
    }
    /// The projection of [`mjx_ooxml::PenAlignment`], whose documentation is authoritative.
    PenAlignment {
        Center,
        Inset,
    }
    /// The projection of [`mjx_ooxml::PictureFillMode`], whose documentation is authoritative.
    PictureFillMode {
        Tile,
        Stretch,
        #[pyo3(name = "NONE")]
        None,
    }
    /// The projection of [`mjx_ooxml::PlaceholderSize`], whose documentation is authoritative.
    PlaceholderSize {
        Full,
        Half,
        Quarter,
    }
    /// The projection of [`mjx_ooxml::PlaceholderType`], whose documentation is authoritative.
    PlaceholderType {
        Title,
        Body,
        CenteredTitle,
        Subtitle,
        DateAndTime,
        SlideNumber,
        Footer,
        Header,
        Object,
        Chart,
        Table,
        ClipArt,
        Diagram,
        Media,
        SlideImage,
        Picture,
    }
    /// The projection of [`mjx_ooxml::PresetCamera`], whose documentation is authoritative.
    PresetCamera {
        LegacyObliqueTopLeft,
        LegacyObliqueTop,
        LegacyObliqueTopRight,
        LegacyObliqueLeft,
        LegacyObliqueFront,
        LegacyObliqueRight,
        LegacyObliqueBottomLeft,
        LegacyObliqueBottom,
        LegacyObliqueBottomRight,
        LegacyPerspectiveTopLeft,
        LegacyPerspectiveTop,
        LegacyPerspectiveTopRight,
        LegacyPerspectiveLeft,
        LegacyPerspectiveFront,
        LegacyPerspectiveRight,
        LegacyPerspectiveBottomLeft,
        LegacyPerspectiveBottom,
        LegacyPerspectiveBottomRight,
        OrthographicFront,
        IsometricTopUp,
        IsometricTopDown,
        IsometricBottomUp,
        IsometricBottomDown,
        IsometricLeftUp,
        IsometricLeftDown,
        IsometricRightUp,
        IsometricRightDown,
        IsometricOffAxis1Left,
        IsometricOffAxis1Right,
        IsometricOffAxis1Top,
        IsometricOffAxis2Left,
        IsometricOffAxis2Right,
        IsometricOffAxis2Top,
        IsometricOffAxis3Left,
        IsometricOffAxis3Right,
        IsometricOffAxis3Bottom,
        IsometricOffAxis4Left,
        IsometricOffAxis4Right,
        IsometricOffAxis4Bottom,
        ObliqueTopLeft,
        ObliqueTop,
        ObliqueTopRight,
        ObliqueLeft,
        ObliqueRight,
        ObliqueBottomLeft,
        ObliqueBottom,
        ObliqueBottomRight,
        PerspectiveFront,
        PerspectiveLeft,
        PerspectiveRight,
        PerspectiveAbove,
        PerspectiveBelow,
        PerspectiveAboveLeftFacing,
        PerspectiveAboveRightFacing,
        PerspectiveContrastingLeftFacing,
        PerspectiveContrastingRightFacing,
        PerspectiveHeroicLeftFacing,
        PerspectiveHeroicRightFacing,
        PerspectiveHeroicExtremeLeftFacing,
        PerspectiveHeroicExtremeRightFacing,
        PerspectiveRelaxed,
        PerspectiveRelaxedModerately,
    }
    /// The projection of [`mjx_ooxml::PresetLineDash`], whose documentation is authoritative.
    PresetLineDash {
        Solid,
        Dot,
        Dash,
        LargeDash,
        DashDot,
        LargeDashDot,
        LargeDashDotDot,
        SystemDash,
        SystemDot,
        SystemDashDot,
        SystemDashDotDot,
    }
    /// The projection of [`mjx_ooxml::PresetMaterial`], whose documentation is authoritative.
    PresetMaterial {
        LegacyMatte,
        LegacyPlastic,
        LegacyMetal,
        LegacyWireframe,
        Matte,
        Plastic,
        Metal,
        WarmMatte,
        TranslucentPowder,
        Powder,
        DarkEdge,
        SoftEdge,
        Clear,
        Flat,
        SoftMetal,
    }
    /// The projection of [`mjx_ooxml::PresetShadow`], whose documentation is authoritative.
    PresetShadow {
        Shadow1,
        Shadow2,
        Shadow3,
        Shadow4,
        Shadow5,
        Shadow6,
        Shadow7,
        Shadow8,
        Shadow9,
        Shadow10,
        Shadow11,
        Shadow12,
        Shadow13,
        Shadow14,
        Shadow15,
        Shadow16,
        Shadow17,
        Shadow18,
        Shadow19,
        Shadow20,
    }
    /// The projection of [`mjx_ooxml::PresetShapeType`], whose documentation is authoritative.
    PresetShapeType {
        StraightLine,
        StraightLineInverse,
        Triangle,
        RightTriangle,
        Rectangle,
        Diamond,
        Parallelogram,
        Trapezoid,
        NonIsoscelesTrapezoid,
        Pentagon,
        Hexagon,
        Heptagon,
        Octagon,
        Decagon,
        Dodecagon,
        FourPointStar,
        FivePointStar,
        SixPointStar,
        SevenPointStar,
        EightPointStar,
        TenPointStar,
        TwelvePointStar,
        SixteenPointStar,
        TwentyFourPointStar,
        ThirtyTwoPointStar,
        RoundedRectangle,
        RoundSingleCornerRectangle,
        RoundSameSideCornersRectangle,
        RoundDiagonalCornersRectangle,
        SnipAndRoundSingleCornerRectangle,
        SnipSingleCornerRectangle,
        SnipSameSideCornersRectangle,
        SnipDiagonalCornersRectangle,
        Plaque,
        Ellipse,
        Teardrop,
        HomePlate,
        Chevron,
        PieWedge,
        Pie,
        BlockArc,
        Donut,
        NoSmoking,
        RightArrow,
        LeftArrow,
        UpArrow,
        DownArrow,
        StripedRightArrow,
        NotchedRightArrow,
        BentUpArrow,
        LeftRightArrow,
        UpDownArrow,
        LeftUpArrow,
        LeftRightUpArrow,
        QuadArrow,
        LeftArrowCallout,
        RightArrowCallout,
        UpArrowCallout,
        DownArrowCallout,
        LeftRightArrowCallout,
        UpDownArrowCallout,
        QuadArrowCallout,
        BentArrow,
        UTurnArrow,
        CircularArrow,
        LeftCircularArrow,
        LeftRightCircularArrow,
        CurvedRightArrow,
        CurvedLeftArrow,
        CurvedUpArrow,
        CurvedDownArrow,
        SwooshArrow,
        Cube,
        Can,
        LightningBolt,
        Heart,
        Sun,
        Moon,
        SmileyFace,
        IrregularSeal1,
        IrregularSeal2,
        FoldedCorner,
        Bevel,
        Frame,
        HalfFrame,
        Corner,
        DiagonalStripe,
        Chord,
        Arc,
        LeftBracket,
        RightBracket,
        LeftBrace,
        RightBrace,
        BracketPair,
        BracePair,
        StraightConnector1,
        BentConnector2,
        BentConnector3,
        BentConnector4,
        BentConnector5,
        CurvedConnector2,
        CurvedConnector3,
        CurvedConnector4,
        CurvedConnector5,
        Callout1,
        Callout2,
        Callout3,
        AccentCallout1,
        AccentCallout2,
        AccentCallout3,
        BorderCallout1,
        BorderCallout2,
        BorderCallout3,
        AccentBorderCallout1,
        AccentBorderCallout2,
        AccentBorderCallout3,
        WedgeRectangleCallout,
        WedgeRoundedRectangleCallout,
        WedgeEllipseCallout,
        CloudCallout,
        Cloud,
        Ribbon,
        Ribbon2,
        EllipseRibbon,
        EllipseRibbon2,
        LeftRightRibbon,
        VerticalScroll,
        HorizontalScroll,
        Wave,
        DoubleWave,
        Plus,
        FlowChartProcess,
        FlowChartDecision,
        FlowChartInputOutput,
        FlowChartPredefinedProcess,
        FlowChartInternalStorage,
        FlowChartDocument,
        FlowChartMultidocument,
        FlowChartTerminator,
        FlowChartPreparation,
        FlowChartManualInput,
        FlowChartManualOperation,
        FlowChartConnector,
        FlowChartPunchedCard,
        FlowChartPunchedTape,
        FlowChartSummingJunction,
        FlowChartOr,
        FlowChartCollate,
        FlowChartSort,
        FlowChartExtract,
        FlowChartMerge,
        FlowChartOfflineStorage,
        FlowChartOnlineStorage,
        FlowChartMagneticTape,
        FlowChartMagneticDisk,
        FlowChartMagneticDrum,
        FlowChartDisplay,
        FlowChartDelay,
        FlowChartAlternateProcess,
        FlowChartOffpageConnector,
        ActionButtonBlank,
        ActionButtonHome,
        ActionButtonHelp,
        ActionButtonInformation,
        ActionButtonForwardNext,
        ActionButtonBackPrevious,
        ActionButtonEnd,
        ActionButtonBeginning,
        ActionButtonReturn,
        ActionButtonDocument,
        ActionButtonSound,
        ActionButtonMovie,
        Gear6,
        Gear9,
        Funnel,
        MathPlus,
        MathMinus,
        MathMultiply,
        MathDivide,
        MathEqual,
        MathNotEqual,
        CornerTabs,
        SquareTabs,
        PlaqueTabs,
        ChartX,
        ChartStar,
        ChartPlus,
    }
    /// The projection of [`mjx_ooxml::RadarStyle`], whose documentation is authoritative.
    RadarStyle {
        Standard,
        Markers,
        Filled,
    }
    /// The projection of [`mjx_ooxml::RectangleAlignment`], whose documentation is authoritative.
    RectangleAlignment {
        TopLeft,
        Top,
        TopRight,
        Left,
        Center,
        Right,
        BottomLeft,
        Bottom,
        BottomRight,
    }
    /// The projection of [`mjx_ooxml::ScatterStyle`], whose documentation is authoritative.
    ScatterStyle {
        #[pyo3(name = "NONE")]
        None,
        Line,
        LineWithMarkers,
        Markers,
        SmoothLine,
        SmoothLineWithMarkers,
    }
    /// The projection of [`mjx_ooxml::SchemeColor`], whose documentation is authoritative.
    SchemeColor {
        Background1,
        Text1,
        Background2,
        Text2,
        Accent1,
        Accent2,
        Accent3,
        Accent4,
        Accent5,
        Accent6,
        Hyperlink,
        FollowedHyperlink,
        PlaceholderColor,
        Dark1,
        Light1,
        Dark2,
        Light2,
    }
    /// The projection of [`mjx_ooxml::SeriesGrouping`], whose documentation is authoritative.
    SeriesGrouping {
        Standard,
        Stacked,
        PercentStacked,
    }
    /// The projection of [`mjx_ooxml::SlideLayoutKind`], whose documentation is authoritative.
    SlideLayoutKind {
        Title,
        Text,
        TwoColumnText,
        Table,
        TextAndChart,
        ChartAndText,
        Diagram,
        Chart,
        TextAndClipArt,
        ClipArtAndText,
        TitleOnly,
        Blank,
        TextAndObject,
        ObjectAndText,
        ObjectOnly,
        TitleAndObject,
        TextAndMedia,
        MediaAndText,
        ObjectOverText,
        TextOverObject,
        TextAndTwoObjects,
        TwoObjectsAndText,
        TwoObjectsOverText,
        FourObjects,
        VerticalText,
        ClipArtAndVerticalText,
        VerticalTitleAndText,
        VerticalTitleAndTextOverChart,
        TwoObjects,
        ObjectAndTwoObjects,
        TwoObjectsAndObject,
        Custom,
        SectionHeader,
        TwoTextAndTwoObjects,
        TitleObjectAndCaption,
        PictureAndCaption,
    }
    /// The projection of [`mjx_ooxml::SlideSizeKind`], whose documentation is authoritative.
    SlideSizeKind {
        Screen4X3,
        Letter,
        A4,
        Film35Mm,
        Overhead,
        Banner,
        Custom,
        Ledger,
        A3,
        B4Iso,
        B5Iso,
        B4Jis,
        B5Jis,
        HagakiCard,
        Screen16X9,
        Screen16X10,
    }
    /// The projection of [`mjx_ooxml::TabAlignment`], whose documentation is authoritative.
    TabAlignment {
        Left,
        Center,
        Right,
        Decimal,
    }
    /// The projection of [`mjx_ooxml::TargetMode`], whose documentation is authoritative.
    TargetMode {
        Internal,
        External,
    }
    /// The projection of [`mjx_ooxml::TextAlignment`], whose documentation is authoritative.
    TextAlignment {
        Left,
        Center,
        Right,
        Justified,
        JustifiedLow,
        Distributed,
        ThaiDistributed,
    }
    /// The projection of [`mjx_ooxml::TextAnchoring`], whose documentation is authoritative.
    TextAnchoring {
        Top,
        Center,
        Bottom,
        Justified,
        Distributed,
    }
    /// The projection of [`mjx_ooxml::TextCapitalization`], whose documentation is authoritative.
    TextCapitalization {
        #[pyo3(name = "NONE")]
        None,
        Small,
        All,
    }
    /// The projection of [`mjx_ooxml::TextDirection`], whose documentation is authoritative.
    TextDirection {
        Horizontal,
        Vertical,
        Vertical270,
        WordArtVertical,
        EastAsianVertical,
        MongolianVertical,
        VerticalWordArtRightToLeft,
    }
    /// The projection of [`mjx_ooxml::TextHorizontalOverflow`], whose documentation is authoritative.
    TextHorizontalOverflow {
        Overflow,
        Clip,
    }
    /// The projection of [`mjx_ooxml::TextStrike`], whose documentation is authoritative.
    TextStrike {
        NoStrike,
        SingleStrike,
        DoubleStrike,
    }
    /// The projection of [`mjx_ooxml::TextUnderline`], whose documentation is authoritative.
    TextUnderline {
        #[pyo3(name = "NONE")]
        None,
        Words,
        Single,
        Double,
        Heavy,
        Dotted,
        HeavyDotted,
        Dashed,
        HeavyDashed,
        LongDashed,
        HeavyLongDashed,
        DotDash,
        HeavyDotDash,
        DotDotDash,
        HeavyDotDotDash,
        Wavy,
        HeavyWavy,
        DoubleWavy,
    }
    /// The projection of [`mjx_ooxml::TickLabelPosition`], whose documentation is authoritative.
    TickLabelPosition {
        High,
        Low,
        NextToAxis,
        #[pyo3(name = "NONE")]
        None,
    }
    /// The projection of [`mjx_ooxml::TickMark`], whose documentation is authoritative.
    TickMark {
        Cross,
        Inside,
        #[pyo3(name = "NONE")]
        None,
        Outside,
    }
    /// The projection of [`mjx_ooxml::TrendlineKind`], whose documentation is authoritative.
    TrendlineKind {
        Exponential,
        Linear,
        Logarithmic,
        MovingAverage,
        Polynomial,
        Power,
    }
    // --- Word (MJXOFF-139) --------------------------------------------------------------------
    /// The projection of [`mjx_ooxml::Justification`], whose documentation is authoritative.
    Justification {
        Start,
        Center,
        End,
        Justified,
        MediumKashida,
        Distribute,
        AlignToListTab,
        WidestKashida,
        LowKashida,
        ThaiDistribute,
        Left,
        Right,
    }
    /// The projection of [`mjx_ooxml::MergedCellType`], whose documentation is authoritative.
    MergedCellType {
        Continue,
        Restart,
    }
    /// The projection of [`mjx_ooxml::PageOrientation`], whose documentation is authoritative.
    PageOrientation {
        Portrait,
        Landscape,
    }
    /// The projection of [`mjx_ooxml::HeaderFooterType`], whose documentation is authoritative.
    HeaderFooterType {
        Even,
        Default,
        First,
    }
    /// The projection of [`mjx_ooxml::FieldForm`], whose documentation is authoritative.
    FieldForm {
        Simple,
        Complex,
    }
}

open_enums! {
    /// The projection of [`mjx_ooxml::ActiveXPersistence`], whose documentation is authoritative.
    ActiveXPersistence {
        Storage,
        Stream,
        StreamWithLength,
        PropertyBag,
    }
    /// The projection of [`mjx_ooxml::CellBorder`], whose documentation is authoritative.
    CellBorder {
        Left,
        Right,
        Top,
        Bottom,
        TopLeftToBottomRight,
        BottomLeftToTopRight,
    }
    /// The projection of [`mjx_ooxml::ChartKind`], whose documentation is authoritative.
    ChartKind {
        Bar,
        Bar3D,
        Line,
        Line3D,
        Pie,
        Pie3D,
        OfPie,
        Area,
        Area3D,
        Scatter,
        Doughnut,
        Radar,
        Bubble,
        Stock,
        Surface,
        Surface3D,
    }
    /// The projection of [`mjx_ooxml::DiagramPartKind`], whose documentation is authoritative.
    DiagramPartKind {
        Data,
        Layout,
        Style,
        Colors,
        Drawing,
    }
    /// The projection of [`mjx_ooxml::GraphicFrameKind`], whose documentation is authoritative.
    GraphicFrameKind {
        Table,
        Chart,
        Diagram,
        OleObject,
        Other,
    }
    /// The projection of [`mjx_ooxml::ShapeKind`], whose documentation is authoritative.
    ShapeKind {
        Shape,
        Picture,
        GroupShape,
        GraphicFrame,
        ConnectionShape,
        ContentPart,
    }
    /// The projection of [`mjx_ooxml::TablePart`], whose documentation is authoritative.
    TablePart {
        FirstRow,
        FirstColumn,
        LastRow,
        LastColumn,
        BandedRows,
        BandedColumns,
        RightToLeft,
    }
    /// The projection of [`mjx_ooxml::TableStyleBorder`], whose documentation is authoritative.
    TableStyleBorder {
        Left,
        Right,
        Top,
        Bottom,
        InsideHorizontal,
        InsideVertical,
        TopLeftToBottomRight,
        TopRightToBottomLeft,
    }
    /// The projection of [`mjx_ooxml::TableStylePart`], whose documentation is authoritative.
    TableStylePart {
        WholeTable,
        Band1Horizontal,
        Band2Horizontal,
        Band1Vertical,
        Band2Vertical,
        FirstRow,
        LastRow,
        FirstColumn,
        LastColumn,
        NorthWestCell,
        NorthEastCell,
        SouthWestCell,
        SouthEastCell,
    }
    // --- Word (MJXOFF-139) --------------------------------------------------------------------
    /// The projection of [`mjx_ooxml::CellBorderEdge`], whose documentation is authoritative.
    CellBorderEdge {
        Top,
        Start,
        Left,
        Bottom,
        End,
        Right,
        InsideHorizontal,
        InsideVertical,
    }
    /// The projection of [`mjx_ooxml::RevisionKind`], whose documentation is authoritative.
    RevisionKind {
        Inserted,
        Deleted,
        MovedFromContent,
        MovedToContent,
        RunPropertiesChanged,
        ParagraphPropertiesChanged,
        ParagraphMarkPropertiesChanged,
        SectionPropertiesChanged,
        TablePropertiesChanged,
        TableExceptionPropertiesChanged,
        TableGridChanged,
        CellPropertiesChanged,
        RowPropertiesChanged,
        CellMerged,
        NumberingChanged,
        MarkerInserted,
        MarkerDeleted,
        MarkerMovedFrom,
        MarkerMovedTo,
    }
}
