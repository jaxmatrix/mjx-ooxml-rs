"""Type stubs for `mjx_ooxml` — the committed contract this package is checked against.

Generated from the binding's own `#[pymethods]` blocks and committed, exactly as
`mjx-ooxml-types` is: the source of truth is the Rust, and this file is its statement in Python.
`tests/test_stub_parity.py` proves the two agree — every name here exists at run time, and every
name at run time is declared here — so a method added to the binding and not to this file is a
test failure, not a silent gap.
"""

from collections.abc import Sequence
from typing import final

__all__: list[str]
__version__: str


class OoxmlError(Exception):
    """Every failure this library reports.

    `code` is the stable classification — `"IndexOutOfRange"`, `"MalformedDocument"`, and so on —
    and the five coordinates say where. Each is `None` when the failure carried no such coordinate,
    so none of them ever raises `AttributeError`.
    """

    code: str
    surface: Surface | None
    shape: ShapePath | None
    row: int | None
    column: int | None
    index: int | None

class IoError(OoxmlError):
    """The container bytes could not be read or written."""

class MalformedDocumentError(OoxmlError):
    """The bytes are a package, but its markup is not what the schema requires."""

class InvalidDocumentError(OoxmlError):
    """The document in memory breaks an invariant, so writing it was refused."""

class IndexOutOfRangeError(OoxmlError, IndexError):
    """An index or range argument is outside what the document holds.

    Also an `IndexError`, so code that already guards a lookup with `except IndexError` keeps
    working when the lookup is a slide index.
    """

class WrongKindError(OoxmlError):
    """The thing at that address is of a kind that cannot answer the call."""

class NotFoundError(OoxmlError):
    """A name or identifier resolved to nothing."""

class NothingToReadError(OoxmlError):
    """The target exists and is of the right kind, but states nothing for this call."""

class InvalidArgumentError(OoxmlError):
    """An argument is refused before anything is written."""

class StructureConflictError(OoxmlError):
    """The edit conflicts with the structure the document already has."""

class UnsupportedContentError(OoxmlError):
    """The document uses a construct this build does not model, or asks for one it cannot write."""

class UnsupportedFormatError(OoxmlError):
    """The file is a valid Office document of a format this build cannot open yet."""

DEFAULT_PLACEHOLDER_IMAGE: bytes


@final
class AdjustmentAxis:
    """The projection of [`mjx_ooxml::AdjustmentAxis`], whose documentation is authoritative."""
    Horizontal: AdjustmentAxis
    Vertical: AdjustmentAxis
    Angle: AdjustmentAxis
    Radius: AdjustmentAxis
    def __int__(self) -> int: ...

@final
class AutonumberScheme:
    """The projection of [`mjx_ooxml::AutonumberScheme`], whose documentation is authoritative."""
    LowercaseLetterParenthesesBoth: AutonumberScheme
    UppercaseLetterParenthesesBoth: AutonumberScheme
    LowercaseLetterParenthesisRight: AutonumberScheme
    UppercaseLetterParenthesisRight: AutonumberScheme
    LowercaseLetterPeriod: AutonumberScheme
    UppercaseLetterPeriod: AutonumberScheme
    ArabicParenthesesBoth: AutonumberScheme
    ArabicParenthesisRight: AutonumberScheme
    ArabicPeriod: AutonumberScheme
    ArabicPlain: AutonumberScheme
    LowercaseRomanParenthesesBoth: AutonumberScheme
    UppercaseRomanParenthesesBoth: AutonumberScheme
    LowercaseRomanParenthesisRight: AutonumberScheme
    UppercaseRomanParenthesisRight: AutonumberScheme
    LowercaseRomanPeriod: AutonumberScheme
    UppercaseRomanPeriod: AutonumberScheme
    DoubleByteCircledNumberPlain: AutonumberScheme
    WingdingsBlackCircledNumberPlain: AutonumberScheme
    WingdingsWhiteCircledNumberPlain: AutonumberScheme
    DoubleByteArabicPeriod: AutonumberScheme
    DoubleByteArabicPlain: AutonumberScheme
    SimplifiedChinesePeriod: AutonumberScheme
    SimplifiedChinesePlain: AutonumberScheme
    TraditionalChinesePeriod: AutonumberScheme
    TraditionalChinesePlain: AutonumberScheme
    JapaneseDoubleBytePeriod: AutonumberScheme
    JapaneseKoreanPlain: AutonumberScheme
    JapaneseKoreanPeriod: AutonumberScheme
    BidirectionalArabicAlphabeticMinus: AutonumberScheme
    BidirectionalArabicAbjadMinus: AutonumberScheme
    BidirectionalHebrewMinus: AutonumberScheme
    ThaiLetterPeriod: AutonumberScheme
    ThaiLetterParenthesisRight: AutonumberScheme
    ThaiLetterParenthesesBoth: AutonumberScheme
    ThaiNumberPeriod: AutonumberScheme
    ThaiNumberParenthesisRight: AutonumberScheme
    ThaiNumberParenthesesBoth: AutonumberScheme
    HindiVowelPeriod: AutonumberScheme
    HindiNumberPeriod: AutonumberScheme
    HindiNumberParenthesisRight: AutonumberScheme
    HindiConsonantPeriod: AutonumberScheme
    def __int__(self) -> int: ...

@final
class AxisKind:
    """The projection of [`mjx_ooxml::AxisKind`], whose documentation is authoritative."""
    Category: AxisKind
    Value: AxisKind
    Date: AxisKind
    Series: AxisKind
    def __int__(self) -> int: ...

@final
class AxisOrientation:
    """The projection of [`mjx_ooxml::AxisOrientation`], whose documentation is authoritative."""
    MinimumToMaximum: AxisOrientation
    MaximumToMinimum: AxisOrientation
    def __int__(self) -> int: ...

@final
class AxisPosition:
    """The projection of [`mjx_ooxml::AxisPosition`], whose documentation is authoritative."""
    Bottom: AxisPosition
    Left: AxisPosition
    Right: AxisPosition
    Top: AxisPosition
    def __int__(self) -> int: ...

@final
class BarDirection:
    """The projection of [`mjx_ooxml::BarDirection`], whose documentation is authoritative."""
    Column: BarDirection
    Bar: BarDirection
    def __int__(self) -> int: ...

@final
class BarGrouping:
    """The projection of [`mjx_ooxml::BarGrouping`], whose documentation is authoritative."""
    Clustered: BarGrouping
    Stacked: BarGrouping
    PercentStacked: BarGrouping
    Standard: BarGrouping
    def __int__(self) -> int: ...

@final
class BevelPreset:
    """The projection of [`mjx_ooxml::BevelPreset`], whose documentation is authoritative."""
    RelaxedInset: BevelPreset
    Circle: BevelPreset
    Slope: BevelPreset
    Cross: BevelPreset
    Angle: BevelPreset
    SoftRound: BevelPreset
    Convex: BevelPreset
    CoolSlant: BevelPreset
    Divot: BevelPreset
    Riblet: BevelPreset
    HardEdge: BevelPreset
    ArtDeco: BevelPreset
    def __int__(self) -> int: ...

@final
class BlankDisplay:
    """The projection of [`mjx_ooxml::BlankDisplay`], whose documentation is authoritative."""
    Span: BlankDisplay
    Gap: BlankDisplay
    Zero: BlankDisplay
    def __int__(self) -> int: ...

@final
class BlendMode:
    """The projection of [`mjx_ooxml::BlendMode`], whose documentation is authoritative."""
    Over: BlendMode
    Multiply: BlendMode
    Screen: BlendMode
    Darken: BlendMode
    Lighten: BlendMode
    def __int__(self) -> int: ...

@final
class ColorKind:
    """The projection of [`mjx_ooxml::ColorKind`], whose documentation is authoritative."""
    Srgb: ColorKind
    ScRgb: ColorKind
    Hsl: ColorKind
    System: ColorKind
    Scheme: ColorKind
    Preset: ColorKind
    Unknown: ColorKind
    def __int__(self) -> int: ...

@final
class ColorSchemeSlot:
    """The projection of [`mjx_ooxml::ColorSchemeSlot`], whose documentation is authoritative."""
    Dark1: ColorSchemeSlot
    Light1: ColorSchemeSlot
    Dark2: ColorSchemeSlot
    Light2: ColorSchemeSlot
    Accent1: ColorSchemeSlot
    Accent2: ColorSchemeSlot
    Accent3: ColorSchemeSlot
    Accent4: ColorSchemeSlot
    Accent5: ColorSchemeSlot
    Accent6: ColorSchemeSlot
    Hyperlink: ColorSchemeSlot
    FollowedHyperlink: ColorSchemeSlot
    def __int__(self) -> int: ...

@final
class CompoundLine:
    """The projection of [`mjx_ooxml::CompoundLine`], whose documentation is authoritative."""
    Single: CompoundLine
    Double: CompoundLine
    ThickThin: CompoundLine
    ThinThick: CompoundLine
    Triple: CompoundLine
    def __int__(self) -> int: ...

@final
class DataLabelPosition:
    """The projection of [`mjx_ooxml::DataLabelPosition`], whose documentation is authoritative."""
    BestFit: DataLabelPosition
    Bottom: DataLabelPosition
    Center: DataLabelPosition
    InsideBase: DataLabelPosition
    InsideEnd: DataLabelPosition
    Left: DataLabelPosition
    OutsideEnd: DataLabelPosition
    Right: DataLabelPosition
    Top: DataLabelPosition
    def __int__(self) -> int: ...

@final
class ErrorBarDirection:
    """The projection of [`mjx_ooxml::ErrorBarDirection`], whose documentation is authoritative."""
    X: ErrorBarDirection
    Y: ErrorBarDirection
    def __int__(self) -> int: ...

@final
class ErrorBarType:
    """The projection of [`mjx_ooxml::ErrorBarType`], whose documentation is authoritative."""
    Both: ErrorBarType
    Minus: ErrorBarType
    Plus: ErrorBarType
    def __int__(self) -> int: ...

@final
class ErrorValueType:
    """The projection of [`mjx_ooxml::ErrorValueType`], whose documentation is authoritative."""
    Custom: ErrorValueType
    FixedValue: ErrorValueType
    Percentage: ErrorValueType
    StandardDeviation: ErrorValueType
    StandardError: ErrorValueType
    def __int__(self) -> int: ...

@final
class FontAlignment:
    """The projection of [`mjx_ooxml::FontAlignment`], whose documentation is authoritative."""
    Automatic: FontAlignment
    Top: FontAlignment
    Center: FontAlignment
    Baseline: FontAlignment
    Bottom: FontAlignment
    def __int__(self) -> int: ...

@final
class FontCollectionIndex:
    """The projection of [`mjx_ooxml::FontCollectionIndex`], whose documentation is authoritative."""
    Major: FontCollectionIndex
    Minor: FontCollectionIndex
    NONE: FontCollectionIndex
    def __int__(self) -> int: ...

@final
class FontSchemeSlot:
    """The projection of [`mjx_ooxml::FontSchemeSlot`], whose documentation is authoritative."""
    Major: FontSchemeSlot
    Minor: FontSchemeSlot
    def __int__(self) -> int: ...

@final
class FontSlot:
    """The projection of [`mjx_ooxml::FontSlot`], whose documentation is authoritative."""
    Latin: FontSlot
    EastAsian: FontSlot
    ComplexScript: FontSlot
    Symbol: FontSlot
    def __int__(self) -> int: ...

@final
class LegendPosition:
    """The projection of [`mjx_ooxml::LegendPosition`], whose documentation is authoritative."""
    Bottom: LegendPosition
    Left: LegendPosition
    Right: LegendPosition
    Top: LegendPosition
    TopRight: LegendPosition
    def __int__(self) -> int: ...

@final
class LightRigDirection:
    """The projection of [`mjx_ooxml::LightRigDirection`], whose documentation is authoritative."""
    TopLeft: LightRigDirection
    Top: LightRigDirection
    TopRight: LightRigDirection
    Left: LightRigDirection
    Right: LightRigDirection
    BottomLeft: LightRigDirection
    Bottom: LightRigDirection
    BottomRight: LightRigDirection
    def __int__(self) -> int: ...

@final
class LightRigType:
    """The projection of [`mjx_ooxml::LightRigType`], whose documentation is authoritative."""
    LegacyFlat1: LightRigType
    LegacyFlat2: LightRigType
    LegacyFlat3: LightRigType
    LegacyFlat4: LightRigType
    LegacyNormal1: LightRigType
    LegacyNormal2: LightRigType
    LegacyNormal3: LightRigType
    LegacyNormal4: LightRigType
    LegacyHarsh1: LightRigType
    LegacyHarsh2: LightRigType
    LegacyHarsh3: LightRigType
    LegacyHarsh4: LightRigType
    ThreePoint: LightRigType
    Balanced: LightRigType
    Soft: LightRigType
    Harsh: LightRigType
    Flood: LightRigType
    Contrasting: LightRigType
    Morning: LightRigType
    Sunrise: LightRigType
    Sunset: LightRigType
    Chilly: LightRigType
    Freezing: LightRigType
    Flat: LightRigType
    TwoPoint: LightRigType
    Glow: LightRigType
    BrightRoom: LightRigType
    def __int__(self) -> int: ...

@final
class LineCap:
    """The projection of [`mjx_ooxml::LineCap`], whose documentation is authoritative."""
    Round: LineCap
    Square: LineCap
    Flat: LineCap
    def __int__(self) -> int: ...

@final
class LineEndLength:
    """The projection of [`mjx_ooxml::LineEndLength`], whose documentation is authoritative."""
    Small: LineEndLength
    Medium: LineEndLength
    Large: LineEndLength
    def __int__(self) -> int: ...

@final
class LineEndType:
    """The projection of [`mjx_ooxml::LineEndType`], whose documentation is authoritative."""
    NONE: LineEndType
    Triangle: LineEndType
    Stealth: LineEndType
    Diamond: LineEndType
    Oval: LineEndType
    Arrow: LineEndType
    def __int__(self) -> int: ...

@final
class LineEndWidth:
    """The projection of [`mjx_ooxml::LineEndWidth`], whose documentation is authoritative."""
    Small: LineEndWidth
    Medium: LineEndWidth
    Large: LineEndWidth
    def __int__(self) -> int: ...

@final
class MediaKind:
    """The projection of [`mjx_ooxml::MediaKind`], whose documentation is authoritative."""
    Audio: MediaKind
    Video: MediaKind
    Media: MediaKind
    def __int__(self) -> int: ...

@final
class OfPieType:
    """The projection of [`mjx_ooxml::OfPieType`], whose documentation is authoritative."""
    Pie: OfPieType
    Bar: OfPieType
    def __int__(self) -> int: ...

@final
class OnOffStyle:
    """The projection of [`mjx_ooxml::OnOffStyle`], whose documentation is authoritative."""
    On: OnOffStyle
    Off: OnOffStyle
    Default: OnOffStyle
    def __int__(self) -> int: ...

@final
class Orientation:
    """The projection of [`mjx_ooxml::Orientation`], whose documentation is authoritative."""
    Horizontal: Orientation
    Vertical: Orientation
    def __int__(self) -> int: ...

@final
class PathFillMode:
    """The projection of [`mjx_ooxml::PathFillMode`], whose documentation is authoritative."""
    NONE: PathFillMode
    Normal: PathFillMode
    Lighten: PathFillMode
    LightenLess: PathFillMode
    Darken: PathFillMode
    DarkenLess: PathFillMode
    def __int__(self) -> int: ...

@final
class PatternType:
    """The projection of [`mjx_ooxml::PatternType`], whose documentation is authoritative."""
    Percent5: PatternType
    Percent10: PatternType
    Percent20: PatternType
    Percent25: PatternType
    Percent30: PatternType
    Percent40: PatternType
    Percent50: PatternType
    Percent60: PatternType
    Percent70: PatternType
    Percent75: PatternType
    Percent80: PatternType
    Percent90: PatternType
    Horizontal: PatternType
    Vertical: PatternType
    LightHorizontal: PatternType
    LightVertical: PatternType
    DarkHorizontal: PatternType
    DarkVertical: PatternType
    NarrowHorizontal: PatternType
    NarrowVertical: PatternType
    DashedHorizontal: PatternType
    DashedVertical: PatternType
    Cross: PatternType
    DownwardDiagonal: PatternType
    UpwardDiagonal: PatternType
    LightDownwardDiagonal: PatternType
    LightUpwardDiagonal: PatternType
    DarkDownwardDiagonal: PatternType
    DarkUpwardDiagonal: PatternType
    WideDownwardDiagonal: PatternType
    WideUpwardDiagonal: PatternType
    DashedDownwardDiagonal: PatternType
    DashedUpwardDiagonal: PatternType
    DiagonalCross: PatternType
    SmallCheckerboard: PatternType
    LargeCheckerboard: PatternType
    SmallGrid: PatternType
    LargeGrid: PatternType
    DottedGrid: PatternType
    SmallConfetti: PatternType
    LargeConfetti: PatternType
    HorizontalBrick: PatternType
    DiagonalBrick: PatternType
    SolidDiamond: PatternType
    OpenDiamond: PatternType
    DottedDiamond: PatternType
    Plaid: PatternType
    Sphere: PatternType
    Weave: PatternType
    Divot: PatternType
    Shingle: PatternType
    Wave: PatternType
    Trellis: PatternType
    ZigZag: PatternType
    def __int__(self) -> int: ...

@final
class PenAlignment:
    """The projection of [`mjx_ooxml::PenAlignment`], whose documentation is authoritative."""
    Center: PenAlignment
    Inset: PenAlignment
    def __int__(self) -> int: ...

@final
class PictureFillMode:
    """The projection of [`mjx_ooxml::PictureFillMode`], whose documentation is authoritative."""
    Tile: PictureFillMode
    Stretch: PictureFillMode
    NONE: PictureFillMode
    def __int__(self) -> int: ...

@final
class PlaceholderSize:
    """The projection of [`mjx_ooxml::PlaceholderSize`], whose documentation is authoritative."""
    Full: PlaceholderSize
    Half: PlaceholderSize
    Quarter: PlaceholderSize
    def __int__(self) -> int: ...

@final
class PlaceholderType:
    """The projection of [`mjx_ooxml::PlaceholderType`], whose documentation is authoritative."""
    Title: PlaceholderType
    Body: PlaceholderType
    CenteredTitle: PlaceholderType
    Subtitle: PlaceholderType
    DateAndTime: PlaceholderType
    SlideNumber: PlaceholderType
    Footer: PlaceholderType
    Header: PlaceholderType
    Object: PlaceholderType
    Chart: PlaceholderType
    Table: PlaceholderType
    ClipArt: PlaceholderType
    Diagram: PlaceholderType
    Media: PlaceholderType
    SlideImage: PlaceholderType
    Picture: PlaceholderType
    def __int__(self) -> int: ...

@final
class PresetCamera:
    """The projection of [`mjx_ooxml::PresetCamera`], whose documentation is authoritative."""
    LegacyObliqueTopLeft: PresetCamera
    LegacyObliqueTop: PresetCamera
    LegacyObliqueTopRight: PresetCamera
    LegacyObliqueLeft: PresetCamera
    LegacyObliqueFront: PresetCamera
    LegacyObliqueRight: PresetCamera
    LegacyObliqueBottomLeft: PresetCamera
    LegacyObliqueBottom: PresetCamera
    LegacyObliqueBottomRight: PresetCamera
    LegacyPerspectiveTopLeft: PresetCamera
    LegacyPerspectiveTop: PresetCamera
    LegacyPerspectiveTopRight: PresetCamera
    LegacyPerspectiveLeft: PresetCamera
    LegacyPerspectiveFront: PresetCamera
    LegacyPerspectiveRight: PresetCamera
    LegacyPerspectiveBottomLeft: PresetCamera
    LegacyPerspectiveBottom: PresetCamera
    LegacyPerspectiveBottomRight: PresetCamera
    OrthographicFront: PresetCamera
    IsometricTopUp: PresetCamera
    IsometricTopDown: PresetCamera
    IsometricBottomUp: PresetCamera
    IsometricBottomDown: PresetCamera
    IsometricLeftUp: PresetCamera
    IsometricLeftDown: PresetCamera
    IsometricRightUp: PresetCamera
    IsometricRightDown: PresetCamera
    IsometricOffAxis1Left: PresetCamera
    IsometricOffAxis1Right: PresetCamera
    IsometricOffAxis1Top: PresetCamera
    IsometricOffAxis2Left: PresetCamera
    IsometricOffAxis2Right: PresetCamera
    IsometricOffAxis2Top: PresetCamera
    IsometricOffAxis3Left: PresetCamera
    IsometricOffAxis3Right: PresetCamera
    IsometricOffAxis3Bottom: PresetCamera
    IsometricOffAxis4Left: PresetCamera
    IsometricOffAxis4Right: PresetCamera
    IsometricOffAxis4Bottom: PresetCamera
    ObliqueTopLeft: PresetCamera
    ObliqueTop: PresetCamera
    ObliqueTopRight: PresetCamera
    ObliqueLeft: PresetCamera
    ObliqueRight: PresetCamera
    ObliqueBottomLeft: PresetCamera
    ObliqueBottom: PresetCamera
    ObliqueBottomRight: PresetCamera
    PerspectiveFront: PresetCamera
    PerspectiveLeft: PresetCamera
    PerspectiveRight: PresetCamera
    PerspectiveAbove: PresetCamera
    PerspectiveBelow: PresetCamera
    PerspectiveAboveLeftFacing: PresetCamera
    PerspectiveAboveRightFacing: PresetCamera
    PerspectiveContrastingLeftFacing: PresetCamera
    PerspectiveContrastingRightFacing: PresetCamera
    PerspectiveHeroicLeftFacing: PresetCamera
    PerspectiveHeroicRightFacing: PresetCamera
    PerspectiveHeroicExtremeLeftFacing: PresetCamera
    PerspectiveHeroicExtremeRightFacing: PresetCamera
    PerspectiveRelaxed: PresetCamera
    PerspectiveRelaxedModerately: PresetCamera
    def __int__(self) -> int: ...

@final
class PresetLineDash:
    """The projection of [`mjx_ooxml::PresetLineDash`], whose documentation is authoritative."""
    Solid: PresetLineDash
    Dot: PresetLineDash
    Dash: PresetLineDash
    LargeDash: PresetLineDash
    DashDot: PresetLineDash
    LargeDashDot: PresetLineDash
    LargeDashDotDot: PresetLineDash
    SystemDash: PresetLineDash
    SystemDot: PresetLineDash
    SystemDashDot: PresetLineDash
    SystemDashDotDot: PresetLineDash
    def __int__(self) -> int: ...

@final
class PresetMaterial:
    """The projection of [`mjx_ooxml::PresetMaterial`], whose documentation is authoritative."""
    LegacyMatte: PresetMaterial
    LegacyPlastic: PresetMaterial
    LegacyMetal: PresetMaterial
    LegacyWireframe: PresetMaterial
    Matte: PresetMaterial
    Plastic: PresetMaterial
    Metal: PresetMaterial
    WarmMatte: PresetMaterial
    TranslucentPowder: PresetMaterial
    Powder: PresetMaterial
    DarkEdge: PresetMaterial
    SoftEdge: PresetMaterial
    Clear: PresetMaterial
    Flat: PresetMaterial
    SoftMetal: PresetMaterial
    def __int__(self) -> int: ...

@final
class PresetShadow:
    """The projection of [`mjx_ooxml::PresetShadow`], whose documentation is authoritative."""
    Shadow1: PresetShadow
    Shadow2: PresetShadow
    Shadow3: PresetShadow
    Shadow4: PresetShadow
    Shadow5: PresetShadow
    Shadow6: PresetShadow
    Shadow7: PresetShadow
    Shadow8: PresetShadow
    Shadow9: PresetShadow
    Shadow10: PresetShadow
    Shadow11: PresetShadow
    Shadow12: PresetShadow
    Shadow13: PresetShadow
    Shadow14: PresetShadow
    Shadow15: PresetShadow
    Shadow16: PresetShadow
    Shadow17: PresetShadow
    Shadow18: PresetShadow
    Shadow19: PresetShadow
    Shadow20: PresetShadow
    def __int__(self) -> int: ...

@final
class PresetShapeType:
    """The projection of [`mjx_ooxml::PresetShapeType`], whose documentation is authoritative."""
    StraightLine: PresetShapeType
    StraightLineInverse: PresetShapeType
    Triangle: PresetShapeType
    RightTriangle: PresetShapeType
    Rectangle: PresetShapeType
    Diamond: PresetShapeType
    Parallelogram: PresetShapeType
    Trapezoid: PresetShapeType
    NonIsoscelesTrapezoid: PresetShapeType
    Pentagon: PresetShapeType
    Hexagon: PresetShapeType
    Heptagon: PresetShapeType
    Octagon: PresetShapeType
    Decagon: PresetShapeType
    Dodecagon: PresetShapeType
    FourPointStar: PresetShapeType
    FivePointStar: PresetShapeType
    SixPointStar: PresetShapeType
    SevenPointStar: PresetShapeType
    EightPointStar: PresetShapeType
    TenPointStar: PresetShapeType
    TwelvePointStar: PresetShapeType
    SixteenPointStar: PresetShapeType
    TwentyFourPointStar: PresetShapeType
    ThirtyTwoPointStar: PresetShapeType
    RoundedRectangle: PresetShapeType
    RoundSingleCornerRectangle: PresetShapeType
    RoundSameSideCornersRectangle: PresetShapeType
    RoundDiagonalCornersRectangle: PresetShapeType
    SnipAndRoundSingleCornerRectangle: PresetShapeType
    SnipSingleCornerRectangle: PresetShapeType
    SnipSameSideCornersRectangle: PresetShapeType
    SnipDiagonalCornersRectangle: PresetShapeType
    Plaque: PresetShapeType
    Ellipse: PresetShapeType
    Teardrop: PresetShapeType
    HomePlate: PresetShapeType
    Chevron: PresetShapeType
    PieWedge: PresetShapeType
    Pie: PresetShapeType
    BlockArc: PresetShapeType
    Donut: PresetShapeType
    NoSmoking: PresetShapeType
    RightArrow: PresetShapeType
    LeftArrow: PresetShapeType
    UpArrow: PresetShapeType
    DownArrow: PresetShapeType
    StripedRightArrow: PresetShapeType
    NotchedRightArrow: PresetShapeType
    BentUpArrow: PresetShapeType
    LeftRightArrow: PresetShapeType
    UpDownArrow: PresetShapeType
    LeftUpArrow: PresetShapeType
    LeftRightUpArrow: PresetShapeType
    QuadArrow: PresetShapeType
    LeftArrowCallout: PresetShapeType
    RightArrowCallout: PresetShapeType
    UpArrowCallout: PresetShapeType
    DownArrowCallout: PresetShapeType
    LeftRightArrowCallout: PresetShapeType
    UpDownArrowCallout: PresetShapeType
    QuadArrowCallout: PresetShapeType
    BentArrow: PresetShapeType
    UTurnArrow: PresetShapeType
    CircularArrow: PresetShapeType
    LeftCircularArrow: PresetShapeType
    LeftRightCircularArrow: PresetShapeType
    CurvedRightArrow: PresetShapeType
    CurvedLeftArrow: PresetShapeType
    CurvedUpArrow: PresetShapeType
    CurvedDownArrow: PresetShapeType
    SwooshArrow: PresetShapeType
    Cube: PresetShapeType
    Can: PresetShapeType
    LightningBolt: PresetShapeType
    Heart: PresetShapeType
    Sun: PresetShapeType
    Moon: PresetShapeType
    SmileyFace: PresetShapeType
    IrregularSeal1: PresetShapeType
    IrregularSeal2: PresetShapeType
    FoldedCorner: PresetShapeType
    Bevel: PresetShapeType
    Frame: PresetShapeType
    HalfFrame: PresetShapeType
    Corner: PresetShapeType
    DiagonalStripe: PresetShapeType
    Chord: PresetShapeType
    Arc: PresetShapeType
    LeftBracket: PresetShapeType
    RightBracket: PresetShapeType
    LeftBrace: PresetShapeType
    RightBrace: PresetShapeType
    BracketPair: PresetShapeType
    BracePair: PresetShapeType
    StraightConnector1: PresetShapeType
    BentConnector2: PresetShapeType
    BentConnector3: PresetShapeType
    BentConnector4: PresetShapeType
    BentConnector5: PresetShapeType
    CurvedConnector2: PresetShapeType
    CurvedConnector3: PresetShapeType
    CurvedConnector4: PresetShapeType
    CurvedConnector5: PresetShapeType
    Callout1: PresetShapeType
    Callout2: PresetShapeType
    Callout3: PresetShapeType
    AccentCallout1: PresetShapeType
    AccentCallout2: PresetShapeType
    AccentCallout3: PresetShapeType
    BorderCallout1: PresetShapeType
    BorderCallout2: PresetShapeType
    BorderCallout3: PresetShapeType
    AccentBorderCallout1: PresetShapeType
    AccentBorderCallout2: PresetShapeType
    AccentBorderCallout3: PresetShapeType
    WedgeRectangleCallout: PresetShapeType
    WedgeRoundedRectangleCallout: PresetShapeType
    WedgeEllipseCallout: PresetShapeType
    CloudCallout: PresetShapeType
    Cloud: PresetShapeType
    Ribbon: PresetShapeType
    Ribbon2: PresetShapeType
    EllipseRibbon: PresetShapeType
    EllipseRibbon2: PresetShapeType
    LeftRightRibbon: PresetShapeType
    VerticalScroll: PresetShapeType
    HorizontalScroll: PresetShapeType
    Wave: PresetShapeType
    DoubleWave: PresetShapeType
    Plus: PresetShapeType
    FlowChartProcess: PresetShapeType
    FlowChartDecision: PresetShapeType
    FlowChartInputOutput: PresetShapeType
    FlowChartPredefinedProcess: PresetShapeType
    FlowChartInternalStorage: PresetShapeType
    FlowChartDocument: PresetShapeType
    FlowChartMultidocument: PresetShapeType
    FlowChartTerminator: PresetShapeType
    FlowChartPreparation: PresetShapeType
    FlowChartManualInput: PresetShapeType
    FlowChartManualOperation: PresetShapeType
    FlowChartConnector: PresetShapeType
    FlowChartPunchedCard: PresetShapeType
    FlowChartPunchedTape: PresetShapeType
    FlowChartSummingJunction: PresetShapeType
    FlowChartOr: PresetShapeType
    FlowChartCollate: PresetShapeType
    FlowChartSort: PresetShapeType
    FlowChartExtract: PresetShapeType
    FlowChartMerge: PresetShapeType
    FlowChartOfflineStorage: PresetShapeType
    FlowChartOnlineStorage: PresetShapeType
    FlowChartMagneticTape: PresetShapeType
    FlowChartMagneticDisk: PresetShapeType
    FlowChartMagneticDrum: PresetShapeType
    FlowChartDisplay: PresetShapeType
    FlowChartDelay: PresetShapeType
    FlowChartAlternateProcess: PresetShapeType
    FlowChartOffpageConnector: PresetShapeType
    ActionButtonBlank: PresetShapeType
    ActionButtonHome: PresetShapeType
    ActionButtonHelp: PresetShapeType
    ActionButtonInformation: PresetShapeType
    ActionButtonForwardNext: PresetShapeType
    ActionButtonBackPrevious: PresetShapeType
    ActionButtonEnd: PresetShapeType
    ActionButtonBeginning: PresetShapeType
    ActionButtonReturn: PresetShapeType
    ActionButtonDocument: PresetShapeType
    ActionButtonSound: PresetShapeType
    ActionButtonMovie: PresetShapeType
    Gear6: PresetShapeType
    Gear9: PresetShapeType
    Funnel: PresetShapeType
    MathPlus: PresetShapeType
    MathMinus: PresetShapeType
    MathMultiply: PresetShapeType
    MathDivide: PresetShapeType
    MathEqual: PresetShapeType
    MathNotEqual: PresetShapeType
    CornerTabs: PresetShapeType
    SquareTabs: PresetShapeType
    PlaqueTabs: PresetShapeType
    ChartX: PresetShapeType
    ChartStar: PresetShapeType
    ChartPlus: PresetShapeType
    def __int__(self) -> int: ...

@final
class RadarStyle:
    """The projection of [`mjx_ooxml::RadarStyle`], whose documentation is authoritative."""
    Standard: RadarStyle
    Markers: RadarStyle
    Filled: RadarStyle
    def __int__(self) -> int: ...

@final
class RectangleAlignment:
    """The projection of [`mjx_ooxml::RectangleAlignment`], whose documentation is authoritative."""
    TopLeft: RectangleAlignment
    Top: RectangleAlignment
    TopRight: RectangleAlignment
    Left: RectangleAlignment
    Center: RectangleAlignment
    Right: RectangleAlignment
    BottomLeft: RectangleAlignment
    Bottom: RectangleAlignment
    BottomRight: RectangleAlignment
    def __int__(self) -> int: ...

@final
class ScatterStyle:
    """The projection of [`mjx_ooxml::ScatterStyle`], whose documentation is authoritative."""
    NONE: ScatterStyle
    Line: ScatterStyle
    LineWithMarkers: ScatterStyle
    Markers: ScatterStyle
    SmoothLine: ScatterStyle
    SmoothLineWithMarkers: ScatterStyle
    def __int__(self) -> int: ...

@final
class SchemeColor:
    """The projection of [`mjx_ooxml::SchemeColor`], whose documentation is authoritative."""
    Background1: SchemeColor
    Text1: SchemeColor
    Background2: SchemeColor
    Text2: SchemeColor
    Accent1: SchemeColor
    Accent2: SchemeColor
    Accent3: SchemeColor
    Accent4: SchemeColor
    Accent5: SchemeColor
    Accent6: SchemeColor
    Hyperlink: SchemeColor
    FollowedHyperlink: SchemeColor
    PlaceholderColor: SchemeColor
    Dark1: SchemeColor
    Light1: SchemeColor
    Dark2: SchemeColor
    Light2: SchemeColor
    def __int__(self) -> int: ...

@final
class SeriesGrouping:
    """The projection of [`mjx_ooxml::SeriesGrouping`], whose documentation is authoritative."""
    Standard: SeriesGrouping
    Stacked: SeriesGrouping
    PercentStacked: SeriesGrouping
    def __int__(self) -> int: ...

@final
class SlideLayoutKind:
    """The projection of [`mjx_ooxml::SlideLayoutKind`], whose documentation is authoritative."""
    Title: SlideLayoutKind
    Text: SlideLayoutKind
    TwoColumnText: SlideLayoutKind
    Table: SlideLayoutKind
    TextAndChart: SlideLayoutKind
    ChartAndText: SlideLayoutKind
    Diagram: SlideLayoutKind
    Chart: SlideLayoutKind
    TextAndClipArt: SlideLayoutKind
    ClipArtAndText: SlideLayoutKind
    TitleOnly: SlideLayoutKind
    Blank: SlideLayoutKind
    TextAndObject: SlideLayoutKind
    ObjectAndText: SlideLayoutKind
    ObjectOnly: SlideLayoutKind
    TitleAndObject: SlideLayoutKind
    TextAndMedia: SlideLayoutKind
    MediaAndText: SlideLayoutKind
    ObjectOverText: SlideLayoutKind
    TextOverObject: SlideLayoutKind
    TextAndTwoObjects: SlideLayoutKind
    TwoObjectsAndText: SlideLayoutKind
    TwoObjectsOverText: SlideLayoutKind
    FourObjects: SlideLayoutKind
    VerticalText: SlideLayoutKind
    ClipArtAndVerticalText: SlideLayoutKind
    VerticalTitleAndText: SlideLayoutKind
    VerticalTitleAndTextOverChart: SlideLayoutKind
    TwoObjects: SlideLayoutKind
    ObjectAndTwoObjects: SlideLayoutKind
    TwoObjectsAndObject: SlideLayoutKind
    Custom: SlideLayoutKind
    SectionHeader: SlideLayoutKind
    TwoTextAndTwoObjects: SlideLayoutKind
    TitleObjectAndCaption: SlideLayoutKind
    PictureAndCaption: SlideLayoutKind
    def __int__(self) -> int: ...

@final
class SlideSizeKind:
    """The projection of [`mjx_ooxml::SlideSizeKind`], whose documentation is authoritative."""
    Screen4X3: SlideSizeKind
    Letter: SlideSizeKind
    A4: SlideSizeKind
    Film35Mm: SlideSizeKind
    Overhead: SlideSizeKind
    Banner: SlideSizeKind
    Custom: SlideSizeKind
    Ledger: SlideSizeKind
    A3: SlideSizeKind
    B4Iso: SlideSizeKind
    B5Iso: SlideSizeKind
    B4Jis: SlideSizeKind
    B5Jis: SlideSizeKind
    HagakiCard: SlideSizeKind
    Screen16X9: SlideSizeKind
    Screen16X10: SlideSizeKind
    def __int__(self) -> int: ...

@final
class TabAlignment:
    """The projection of [`mjx_ooxml::TabAlignment`], whose documentation is authoritative."""
    Left: TabAlignment
    Center: TabAlignment
    Right: TabAlignment
    Decimal: TabAlignment
    def __int__(self) -> int: ...

@final
class TargetMode:
    """The projection of [`mjx_ooxml::TargetMode`], whose documentation is authoritative."""
    Internal: TargetMode
    External: TargetMode
    def __int__(self) -> int: ...

@final
class TextAlignment:
    """The projection of [`mjx_ooxml::TextAlignment`], whose documentation is authoritative."""
    Left: TextAlignment
    Center: TextAlignment
    Right: TextAlignment
    Justified: TextAlignment
    JustifiedLow: TextAlignment
    Distributed: TextAlignment
    ThaiDistributed: TextAlignment
    def __int__(self) -> int: ...

@final
class TextAnchoring:
    """The projection of [`mjx_ooxml::TextAnchoring`], whose documentation is authoritative."""
    Top: TextAnchoring
    Center: TextAnchoring
    Bottom: TextAnchoring
    Justified: TextAnchoring
    Distributed: TextAnchoring
    def __int__(self) -> int: ...

@final
class TextCapitalization:
    """The projection of [`mjx_ooxml::TextCapitalization`], whose documentation is authoritative."""
    NONE: TextCapitalization
    Small: TextCapitalization
    All: TextCapitalization
    def __int__(self) -> int: ...

@final
class TextDirection:
    """The projection of [`mjx_ooxml::TextDirection`], whose documentation is authoritative."""
    Horizontal: TextDirection
    Vertical: TextDirection
    Vertical270: TextDirection
    WordArtVertical: TextDirection
    EastAsianVertical: TextDirection
    MongolianVertical: TextDirection
    VerticalWordArtRightToLeft: TextDirection
    def __int__(self) -> int: ...

@final
class TextHorizontalOverflow:
    """The projection of [`mjx_ooxml::TextHorizontalOverflow`], whose documentation is
    authoritative.
    """
    Overflow: TextHorizontalOverflow
    Clip: TextHorizontalOverflow
    def __int__(self) -> int: ...

@final
class TextStrike:
    """The projection of [`mjx_ooxml::TextStrike`], whose documentation is authoritative."""
    NoStrike: TextStrike
    SingleStrike: TextStrike
    DoubleStrike: TextStrike
    def __int__(self) -> int: ...

@final
class TextUnderline:
    """The projection of [`mjx_ooxml::TextUnderline`], whose documentation is authoritative."""
    NONE: TextUnderline
    Words: TextUnderline
    Single: TextUnderline
    Double: TextUnderline
    Heavy: TextUnderline
    Dotted: TextUnderline
    HeavyDotted: TextUnderline
    Dashed: TextUnderline
    HeavyDashed: TextUnderline
    LongDashed: TextUnderline
    HeavyLongDashed: TextUnderline
    DotDash: TextUnderline
    HeavyDotDash: TextUnderline
    DotDotDash: TextUnderline
    HeavyDotDotDash: TextUnderline
    Wavy: TextUnderline
    HeavyWavy: TextUnderline
    DoubleWavy: TextUnderline
    def __int__(self) -> int: ...

@final
class TickLabelPosition:
    """The projection of [`mjx_ooxml::TickLabelPosition`], whose documentation is authoritative."""
    High: TickLabelPosition
    Low: TickLabelPosition
    NextToAxis: TickLabelPosition
    NONE: TickLabelPosition
    def __int__(self) -> int: ...

@final
class TickMark:
    """The projection of [`mjx_ooxml::TickMark`], whose documentation is authoritative."""
    Cross: TickMark
    Inside: TickMark
    NONE: TickMark
    Outside: TickMark
    def __int__(self) -> int: ...

@final
class TrendlineKind:
    """The projection of [`mjx_ooxml::TrendlineKind`], whose documentation is authoritative."""
    Exponential: TrendlineKind
    Linear: TrendlineKind
    Logarithmic: TrendlineKind
    MovingAverage: TrendlineKind
    Polynomial: TrendlineKind
    Power: TrendlineKind
    def __int__(self) -> int: ...

@final
class ActiveXPersistence:
    """The projection of [`mjx_ooxml::ActiveXPersistence`], whose documentation is authoritative."""
    Storage: ActiveXPersistence
    Stream: ActiveXPersistence
    StreamWithLength: ActiveXPersistence
    PropertyBag: ActiveXPersistence
    def __int__(self) -> int: ...

@final
class CellBorder:
    """The projection of [`mjx_ooxml::CellBorder`], whose documentation is authoritative."""
    Left: CellBorder
    Right: CellBorder
    Top: CellBorder
    Bottom: CellBorder
    TopLeftToBottomRight: CellBorder
    BottomLeftToTopRight: CellBorder
    def __int__(self) -> int: ...

@final
class ChartKind:
    """The projection of [`mjx_ooxml::ChartKind`], whose documentation is authoritative."""
    Bar: ChartKind
    Bar3D: ChartKind
    Line: ChartKind
    Line3D: ChartKind
    Pie: ChartKind
    Pie3D: ChartKind
    OfPie: ChartKind
    Area: ChartKind
    Area3D: ChartKind
    Scatter: ChartKind
    Doughnut: ChartKind
    Radar: ChartKind
    Bubble: ChartKind
    Stock: ChartKind
    Surface: ChartKind
    Surface3D: ChartKind
    def __int__(self) -> int: ...

@final
class DiagramPartKind:
    """The projection of [`mjx_ooxml::DiagramPartKind`], whose documentation is authoritative."""
    Data: DiagramPartKind
    Layout: DiagramPartKind
    Style: DiagramPartKind
    Colors: DiagramPartKind
    Drawing: DiagramPartKind
    def __int__(self) -> int: ...

@final
class GraphicFrameKind:
    """The projection of [`mjx_ooxml::GraphicFrameKind`], whose documentation is authoritative."""
    Table: GraphicFrameKind
    Chart: GraphicFrameKind
    Diagram: GraphicFrameKind
    OleObject: GraphicFrameKind
    Other: GraphicFrameKind
    def __int__(self) -> int: ...

@final
class ShapeKind:
    """The projection of [`mjx_ooxml::ShapeKind`], whose documentation is authoritative."""
    Shape: ShapeKind
    Picture: ShapeKind
    GroupShape: ShapeKind
    GraphicFrame: ShapeKind
    ConnectionShape: ShapeKind
    ContentPart: ShapeKind
    def __int__(self) -> int: ...

@final
class TablePart:
    """The projection of [`mjx_ooxml::TablePart`], whose documentation is authoritative."""
    FirstRow: TablePart
    FirstColumn: TablePart
    LastRow: TablePart
    LastColumn: TablePart
    BandedRows: TablePart
    BandedColumns: TablePart
    RightToLeft: TablePart
    def __int__(self) -> int: ...

@final
class TableStyleBorder:
    """The projection of [`mjx_ooxml::TableStyleBorder`], whose documentation is authoritative."""
    Left: TableStyleBorder
    Right: TableStyleBorder
    Top: TableStyleBorder
    Bottom: TableStyleBorder
    InsideHorizontal: TableStyleBorder
    InsideVertical: TableStyleBorder
    TopLeftToBottomRight: TableStyleBorder
    TopRightToBottomLeft: TableStyleBorder
    def __int__(self) -> int: ...

@final
class TableStylePart:
    """The projection of [`mjx_ooxml::TableStylePart`], whose documentation is authoritative."""
    WholeTable: TableStylePart
    Band1Horizontal: TableStylePart
    Band2Horizontal: TableStylePart
    Band1Vertical: TableStylePart
    Band2Vertical: TableStylePart
    FirstRow: TableStylePart
    LastRow: TableStylePart
    FirstColumn: TableStylePart
    LastColumn: TableStylePart
    NorthWestCell: TableStylePart
    NorthEastCell: TableStylePart
    SouthWestCell: TableStylePart
    SouthEastCell: TableStylePart
    def __int__(self) -> int: ...

@final
class Surface:
    """The shape-bearing part a call is about."""
    @staticmethod
    def slide(index: int) -> "Surface":
        """The slide at this index, counting from zero."""
        ...
    @staticmethod
    def layout(index: int) -> "Surface":
        """The slide layout at this index — one flat space across every master."""
        ...
    @staticmethod
    def master(index: int) -> "Surface":
        """The slide master at this index."""
        ...
    @staticmethod
    def notes(slide_index: int) -> "Surface":
        """The notes slide belonging to the slide at this index."""
        ...
    @staticmethod
    def notes_master() -> "Surface":
        """The single notes master every notes slide inherits from."""
        ...
    index: int
    """The index within this surface's own kind. The notes master is unique and reports `0`."""
    kind: str
    """The kind's name: `"slide"`, `"layout"`, `"master"`, `"notes"` or `"notes master"`."""
    is_master_like: bool
    """Whether this stands at the head of its own inheritance chain — a slide master or the notes
    master, neither of which inherits from a further part.
    """

@final
class ShapePath:
    """The address of a shape within a surface's shape tree."""
    @staticmethod
    def top(index: int) -> "ShapePath":
        """The top-level shape at this index."""
        ...
    @staticmethod
    def of(indices: list[int]) -> "ShapePath":
        """The shape at this address: `[2]` top-level, `[2, 1]` for member 1 of the group at index
        2.
        """
        ...
    indices: list[int]
    """The address as a list of indices, outermost first."""
    depth: int
    """How deep the address reaches: `1` for a top-level shape, `2` for a member of a top-level
    group, and so on.
    """
    is_top_level: bool
    """Whether this addresses a top-level shape — a single index, no group descent."""
    def child(self, index: int) -> "ShapePath":
        """The address of member `index` of the group this addresses — one step deeper."""
        ...
    parent: "ShapePath" | None
    """The address of the group this shape belongs to, or `None` for a top-level shape."""

@final
class ChartData:
    """A chart to author: its kind, its categories, its series, and the decoration it starts with."""
    def __init__(self, kind: ChartKind) -> None:
        """A chart of the given kind, with nothing in it yet."""
        ...
    def categories(self, categories: list[str]) -> "ChartData":
        """This chart with the given category labels, replacing any it had."""
        ...
    def series(self, name: str, values: Sequence[float]) -> "ChartData":
        """This chart with one more series."""
        ...
    def title(self, title: str) -> "ChartData":
        """This chart with the given title."""
        ...
    def legend(self, position: LegendPosition) -> "ChartData":
        """This chart with a legend in the given position."""
        ...
    def data_labels(self, spec: DataLabelSpec) -> "ChartData":
        """This chart with the given data labels on every series."""
        ...
    kind: ChartKind
    """Which kind of chart this is."""
    is_empty: bool
    """Whether the chart holds no series at all."""
    series_names: list[str]
    """The series names, in order."""
    series_values: list[list[float]]
    """The series values, in order."""
    category_count: int
    """How many categories the chart states."""
    longest_series: int
    """How many values the longest series holds."""
    def category_label(self, index: int) -> str | None:
        """One category label, when the chart states one at that index."""
        ...
    def validate(self) -> None:
        """Whether this description is one the chart kind will accept — the number of series it
        needs, the decoration its series may carry, and whether every measure is finite.
        """
        ...

@final
class DataLabelSpec:
    """What data labels to show, and where."""
    def __init__(self) -> None:
        """Data labels that state nothing. Add to them with the fluent methods."""
        ...
    def value(self, show: bool) -> "DataLabelSpec":
        """Show, or hide, each point's value."""
        ...
    def category_name(self, show: bool) -> "DataLabelSpec":
        """Show, or hide, each point's category name."""
        ...
    def series_name(self, show: bool) -> "DataLabelSpec":
        """Show, or hide, the series name."""
        ...
    def percentage(self, show: bool) -> "DataLabelSpec":
        """Show, or hide, each point's share of the total."""
        ...
    def bubble_size(self, show: bool) -> "DataLabelSpec":
        """Show, or hide, each bubble's size."""
        ...
    def legend_key(self, show: bool) -> "DataLabelSpec":
        """Show, or hide, the legend swatch beside each label."""
        ...
    def leader_lines(self, show: bool) -> "DataLabelSpec":
        """Show, or hide, the lines that join a label to its point."""
        ...
    def position(self, position: DataLabelPosition) -> "DataLabelSpec":
        """Put the labels in the given position relative to their points."""
        ...
    def separator(self, separator: str) -> "DataLabelSpec":
        """Separate the parts of a label with the given string."""
        ...
    def number_format(self, format_code: str) -> "DataLabelSpec":
        """Format the numbers with the given format code."""
        ...
    is_empty: bool
    """Whether this specification states nothing."""

@final
class DataLabelSettings:
    """What data labels a chart part already states, at one tier of its hierarchy."""
    deleted: bool | None
    """Whether the labels are suppressed, when stated."""
    shows_value: bool | None
    """Whether the value is shown, when stated."""
    shows_category_name: bool | None
    """Whether the category name is shown, when stated."""
    shows_series_name: bool | None
    """Whether the series name is shown, when stated."""
    shows_percentage: bool | None
    """Whether the percentage is shown, when stated."""
    shows_bubble_size: bool | None
    """Whether the bubble size is shown, when stated."""
    shows_legend_key: bool | None
    """Whether the legend key is shown, when stated."""
    shows_leader_lines: bool | None
    """Whether leader lines are shown, when stated."""
    position: DataLabelPosition | None
    """Where the labels sit, when stated."""
    separator: str | None
    """The separator between the parts of a label, when stated."""
    number_format: str | None
    """The number format code, when stated."""
    is_empty: bool
    """Whether these settings state nothing at all."""
    def inherit(self, parent: "DataLabelSettings") -> "DataLabelSettings":
        """These settings laid over `parent`: whatever this tier states wins, and the rest comes
        from the tier above. The same walk `chart_data_labels` makes.
        """
        ...

@final
class ChartLabelScope:
    """Which tier of a chart's data-label hierarchy a call is about."""
    @staticmethod
    def plot(plot_index: int) -> "ChartLabelScope":
        """One plot of the chart — the widest tier."""
        ...
    @staticmethod
    def series(series_index: int) -> "ChartLabelScope":
        """One series."""
        ...
    @staticmethod
    def point(series_index: int, point_index: int) -> "ChartLabelScope":
        """One data point — the narrowest tier."""
        ...
    kind: str
    """Which tier this is: `"plot"`, `"series"` or `"point"`."""
    plot_index: int | None
    """The plot index, when this is a plot scope."""
    series_index: int | None
    """The series index, when this scope names one."""
    point_index: int | None
    """The point index, when this is a point scope."""

@final
class TrendlineSpec:
    """A trendline to add to a series."""
    def __init__(self, kind: TrendlineKind) -> None:
        """A trendline of the given kind."""
        ...
    def name(self, name: str) -> "TrendlineSpec":
        """This trendline with the given name."""
        ...
    def polynomial_order(self, order: int) -> "TrendlineSpec":
        """This trendline as a polynomial of the given order."""
        ...
    def moving_average_period(self, period: int) -> "TrendlineSpec":
        """This trendline as a moving average over the given number of periods."""
        ...
    def projection(self, forward: float, backward: float) -> "TrendlineSpec":
        """This trendline projected forward and backward by the given number of periods."""
        ...
    def intercept(self, intercept: float) -> "TrendlineSpec":
        """This trendline forced through the given intercept."""
        ...
    def display(self, equation: bool, r_squared: bool) -> "TrendlineSpec":
        """This trendline showing its equation and its R² on the chart."""
        ...
    kind: TrendlineKind
    """Which kind of trendline."""
    def validate(self) -> None:
        """Whether this trendline's order and period are in range for its kind."""
        ...

@final
class ErrorBarSpec:
    """Error bars to add to a series."""
    @staticmethod
    def fixed(bar_type: ErrorBarType, value_type: ErrorValueType, value: float) -> "ErrorBarSpec":
        """Error bars of a fixed size — a value, a percentage, a standard deviation or a standard
        error, depending on `value_type`.
        """
        ...
    @staticmethod
    def custom(bar_type: ErrorBarType, plus_values: Sequence[float], minus_values: Sequence[float]) -> "ErrorBarSpec":
        """Error bars whose lengths are given point by point."""
        ...
    def direction(self, direction: ErrorBarDirection) -> "ErrorBarSpec":
        """These error bars along the given axis."""
        ...
    def no_end_cap(self, no_end_cap: bool) -> "ErrorBarSpec":
        """These error bars with, or without, the cap at each end."""
        ...
    def validate(self) -> None:
        """Whether custom error bars carry the values they need."""
        ...

@final
class ChartSeriesData:
    """One series as the chart part states it: its name, its categories and its values."""
    name: str | None
    """The series name, when the chart states one."""
    categories: list[str]
    """The category labels, in order."""
    values: Sequence[float]
    """The values, in order."""

@final
class ChartAxisData:
    """One axis as the chart part states it."""
    kind: AxisKind
    """Whether this is the category, value, date or series axis."""
    axis_id: int | None
    """The axis's own identifier, when stated."""
    cross_axis_id: int | None
    """The identifier of the axis this one crosses, when stated."""
    deleted: bool | None
    """Whether the axis is hidden, when stated."""
    position: AxisPosition | None
    """Which side of the plot the axis sits on, when stated."""
    orientation: AxisOrientation | None
    """Which way the axis runs, when stated."""
    minimum: float | None
    """The lower bound of the scale, when stated."""
    maximum: float | None
    """The upper bound of the scale, when stated."""
    logarithm_base: float | None
    """The logarithm base, when the axis is logarithmic."""
    title: str | None
    """The axis title, when it has one."""
    major_gridlines: bool
    """Whether major gridlines are drawn."""
    minor_gridlines: bool
    """Whether minor gridlines are drawn."""
    major_tick_mark: TickMark | None
    """The major tick mark style, when stated."""
    minor_tick_mark: TickMark | None
    """The minor tick mark style, when stated."""
    tick_label_position: TickLabelPosition | None
    """Where the tick labels sit, when stated."""
    number_format: str | None
    """The tick labels' number format code, when stated."""

@final
class ChartLegendData:
    """The legend as the chart part states it."""
    position: LegendPosition | None
    """Where the legend sits, when stated."""
    overlays_plot: bool | None
    """Whether the legend overlaps the plot rather than reserving space, when stated."""

@final
class ChartPointFormatData:
    """One point's own formatting, overriding its series'."""
    index: int | None
    """Which point this formatting belongs to, when it names one."""
    fill: FillSpec | None
    """The point's own fill, when it states one."""
    line: LineSpec | None
    """The point's own outline, when it states one."""
    explosion: int | None
    """How far the slice is pulled out of a pie, when stated."""
    inverts_if_negative: bool | None
    """Whether a negative value inverts the fill, when stated."""

@final
class ChartTrendlineData:
    """One trendline as the chart part states it."""
    kind: TrendlineKind | None
    """Which kind of trendline, when stated."""
    name: str | None
    """The trendline's name, when stated."""
    polynomial_order: int | None
    """The polynomial order, when stated."""
    moving_average_period: int | None
    """The moving-average period, when stated."""
    forward_periods: float | None
    """How far the line is projected forward, when stated."""
    backward_periods: float | None
    """How far the line is projected backward, when stated."""
    intercept: float | None
    """The intercept the line is forced through, when stated."""
    displays_equation: bool | None
    """Whether the equation is shown, when stated."""
    displays_r_squared: bool | None
    """Whether the R² is shown, when stated."""

@final
class ChartErrorBarData:
    """One set of error bars as the chart part states it."""
    direction: ErrorBarDirection | None
    """Which axis the bars run along, when stated."""
    bar_type: ErrorBarType | None
    """Whether the bars run up, down or both ways, when stated."""
    value_type: ErrorValueType | None
    """How the bar lengths are computed, when stated."""
    no_end_cap: bool | None
    """Whether the end caps are suppressed, when stated."""
    value: float | None
    """The fixed value, when the bars use one."""
    plus_values: Sequence[float]
    """The upward lengths, point by point, when the bars are custom."""
    minus_values: Sequence[float]
    """The downward lengths, point by point, when the bars are custom."""

@final
class ChartWorkbook:
    """A chart's backing workbook: which shape holds the chart, where the workbook is, and whether
    it lies outside the package.
    """
    shape_index: int
    """The top-level index of the graphic frame that holds the chart."""
    target: str
    """Where the workbook is — a part name inside the package, or a URI outside it."""
    external: bool
    """Whether the workbook lies outside the package."""

@final
class DanglingPointReference:
    """A decoration that names a data point the series no longer has."""
    element: str
    """Which element carries the dangling reference — `c:dPt`, `c:dLbl`, and so on."""
    index: int
    """The point index it names, which the series no longer has."""

@final
class LayoutInfo:
    """One slide layout: where it sits, which master it belongs to, its name and its kind."""
    index: int
    """The layout's index in the deck's one flat layout space."""
    master_index: int
    """The index of the master this layout belongs to."""
    name: str | None
    """The layout's name, when it states one."""
    kind: SlideLayoutKind
    """Which of the thirty-six layout kinds `p:sldLayout@type` names."""

@final
class ShapeInfo:
    """One shape on a surface: its index, its kind, and the placeholder it fills."""
    index: int
    """The shape's top-level index on its surface."""
    kind: ShapeKind
    """Which kind of shape this is."""
    placeholder: PlaceholderInfo | None
    """What placeholder the shape fills, when it fills one."""

@final
class PlaceholderInfo:
    """What a placeholder shape declares itself to be."""
    kind: PlaceholderType
    """Which of the sixteen placeholder kinds."""
    index: int
    """The placeholder's index, which is what pairs a slide's placeholder with its layout's."""
    size: PlaceholderSize
    """Full, half or quarter."""
    orientation: Orientation
    """Horizontal or vertical."""
    name: str | None
    """The shape's own name, when it states one."""

@final
class MediaReference:
    """One audio, video or media reference on a surface."""
    rel_id: str
    """The relationship id, which is how `replace_media_with_placeholder` names it."""
    kind: MediaKind
    """Audio, video, or the generic media relationship."""
    target: str
    """Where the media is — a part name inside the package, or a URI outside it."""
    external: bool
    """Whether the media lies outside the package."""

@final
class LinkedImage:
    """A picture whose image lies outside the package."""
    shape_index: int
    """The top-level index of the picture whose image is linked."""
    target: str
    """Where the image is, exactly as the relationship records it."""

@final
class OleObject:
    """One embedded or linked OLE object on a surface."""
    shape_index: int
    """The top-level index of the graphic frame that holds the object."""
    target: str
    """Where the object's data is."""
    external: bool
    """Whether the data lies outside the package."""
    prog_id: str | None
    """The programmatic identifier of the application that owns the object, when stated."""

@final
class ExternalLink:
    """A relationship whose target lies outside the package, and the part that holds it."""
    source: str | None
    """The part whose relationships hold this one, or `None` for the package root."""
    id: str
    """The relationship id, unique within its source."""
    rel_type: str
    """The relationship type URI, which says what kind of external source it binds."""
    target: str
    """The external target, exactly as recorded."""

@final
class InkReference:
    """One ink (InkML) reference on a surface."""
    shape_index: int | None
    """The top-level index of the content part that names the ink, when a shape does."""
    rel_id: str
    """The relationship id the content part carries."""
    part: str | None
    """The ink part the relationship resolves to, when it resolves to one."""

@final
class DiagramParts:
    """The five parts a SmartArt frame names."""
    data: str | None
    """The data model part, when the frame names one that resolves."""
    layout: str | None
    """The layout definition part."""
    style: str | None
    """The style definition part."""
    colors: str | None
    """The colour transform part."""
    drawing: str | None
    """The cached drawing part, which renderers that do not lay out SmartArt fall back on."""

@final
class DiagramRelationshipIds:
    """The four relationship ids a SmartArt frame carries."""
    data: str | None
    """The data model relationship id (`dgm:relIds@dm`)."""
    layout: str | None
    """The layout definition relationship id (`@lo`)."""
    style: str | None
    """The style definition relationship id (`@qs`)."""
    colors: str | None
    """The colour transform relationship id (`@cs`)."""

@final
class DiagramContent:
    """The four parts a SmartArt diagram is built from."""
    @staticmethod
    def from_parts(data: bytes, layout: bytes, style: bytes, colors: bytes) -> "DiagramContent":
        """A diagram built from four part payloads you already have."""
        ...
    @staticmethod
    def vertical_list(labels: list[str]) -> "DiagramContent":
        """A minimal vertical list of labels — enough to write a SmartArt frame that opens."""
        ...
    data: bytes
    """The data model part's bytes."""
    layout: bytes
    """The layout definition part's bytes."""
    style: bytes
    """The style definition part's bytes."""
    colors: bytes
    """The colour transform part's bytes."""

@final
class Hyperlink:
    """Where a hyperlink goes: out to a URL, or in to another slide."""
    @staticmethod
    def url(url: str) -> "Hyperlink":
        """A link out to a URL."""
        ...
    @staticmethod
    def slide(index: int) -> "Hyperlink":
        """A link to another slide in the same deck, by index."""
        ...
    kind: str
    """Which kind this is: `"url"` or `"slide"`."""
    target: str | None
    """The URL, when this links out."""
    slide_index: int | None
    """The slide index, when this links in."""

@final
class OleObjectData:
    """Where an OLE object's data lives."""
    @staticmethod
    def embedded_stream(bytes: bytes) -> "OleObjectData":
        """A raw stream, embedded in the package."""
        ...
    @staticmethod
    def embedded_package(bytes: bytes, extension: str, content_type: str) -> "OleObjectData":
        """A packaged file, embedded with its own extension and content type — a `.docx` inside a
        `.pptx`, say.
        """
        ...
    @staticmethod
    def linked(target: str) -> "OleObjectData":
        """A file outside the package, named by URI."""
        ...
    kind: str
    """Which kind this is: `"embedded_stream"`, `"embedded_package"` or `"linked"`."""

@final
class OleObjectSpec:
    """An OLE object to add to a surface: what application owns it, what its data is, and the
    picture PowerPoint shows in its place.
    """
    def __init__(self, prog_id: str, data: OleObjectData, snapshot_image: bytes, name: str | None = ..., show_as_icon: bool = ...) -> None:
        """An OLE object."""
        ...
    @staticmethod
    def embedded_stream(prog_id: str, data: bytes, snapshot_image: bytes) -> "OleObjectSpec":
        """An embedded-stream object, the common case."""
        ...
    def named(self, name: str) -> "OleObjectSpec":
        """This object with the given display name."""
        ...
    def shown_as_icon(self, show_as_icon: bool) -> "OleObjectSpec":
        """This object shown as an icon rather than as its snapshot."""
        ...
    prog_id: str
    """The programmatic identifier of the owning application."""
    data: OleObjectData
    """Where the object's data lives."""
    snapshot_image: bytes
    """The picture shown in the object's place."""
    name: str | None
    """The display name, when one is set."""
    show_as_icon: bool
    """Whether the object is shown as an icon."""

@final
class ActiveXControlSpec:
    """An ActiveX control to add to a surface."""
    def __init__(self, name: str, class_id: str, state: bytes, snapshot_image: bytes, persistence: ActiveXPersistence = ...) -> None:
        """A control: its name, its class identifier (a GUID in braces), its persisted state, and
        the picture PowerPoint shows in its place.
        """
        ...
    name: str
    """The control's name."""
    class_id: str
    """The control's class identifier."""
    persistence: ActiveXPersistence
    """How the control's state is persisted."""
    state: bytes | None
    """The persisted state, when there is any."""
    snapshot_image: bytes
    """The picture shown in the control's place."""

@final
class Deck:
    """An open PowerPoint deck."""
    @staticmethod
    def blank(size: SlideSize) -> "Deck":
        """A new deck with nothing in it: one slide master, one blank layout, a theme, and no
        slides.
        """
        ...
    @staticmethod
    def open(data: bytes) -> "Deck":
        """Opens a deck from the bytes of a `.pptx`, `.pptm`, `.potx`, `.potm`, `.ppsx` or `.ppsm`."""
        ...
    def format(self) -> Format:
        """What this deck's main part says it is — `Format.Presentation`,
        `Format.PresentationTemplate` and so on. A deck authored by `blank` reports
        `Format.Presentation`.
        """
        ...
    def save(self) -> bytes:
        """The deck as the bytes of a `.pptx`, **validated first**."""
        ...
    def save_unchecked(self) -> bytes:
        """The deck as bytes, **without** the validation pass."""
        ...
    def validate(self) -> None:
        """Runs the packaging and PresentationML checks `save` runs, without writing anything."""
        ...
    def shape_fill(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> FillSpec | None:
        """The explicit fill of shape `shape_idx` on `surface`, as an interner-free `FillSpec`, or
        `None` if the shape declares no fill in its `p:spPr` (its fill is then inherited from
        the placeholder / style / theme — resolving that is a separate, future task). Reading
        does not dirty the part.
        """
        ...
    def set_shape_fill(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, fill: FillSpec) -> None:
        """Sets the fill of shape `shape_idx` on `surface` from an interner-free `FillSpec`,
        rebuilding the `p:spPr` fill element (replacing an existing one in place, or inserting a
        new one after any geometry and before `a:ln`). Marks only that part dirty.
        """
        ...
    def set_shape_no_fill(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> None:
        """Sets shape `shape_idx` on `surface` to an explicit "no fill" (`a:noFill`). A shorthand
        for `set_shape_fill` with `FillSpec::None`.
        """
        ...
    def shape_outline(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> LineSpec | None:
        """The **explicit** outline of shape `shape_idx` on `surface` — its `p:spPr > a:ln` as an
        interner- free `LineSpec` — or `None` when the shape declares no `a:ln` (its outline is
        then inherited; effective outline resolution is a later step). Reading does not dirty
        the part.
        """
        ...
    def set_shape_outline(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, line: LineSpec) -> None:
        """Sets the outline of shape `shape_idx` on `surface` from an interner-free `LineSpec`,
        rebuilding the `p:spPr` `a:ln` element (replacing an existing one in place, or inserting
        a new one after any geometry and fill, before effects). Marks only that part dirty.
        """
        ...
    def set_shape_no_outline(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> None:
        """Sets shape `shape_idx` on `surface` to an explicit "no outline"
        (`<a:ln><a:noFill/></a:ln>`). A shorthand for `set_shape_outline` with a `LineSpec`
        whose fill is `FillSpec::None` — PowerPoint's "no line", distinct from an absent `a:ln`.
        """
        ...
    def shape_effects(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> EffectListSpec | None:
        """The **explicit** effects of shape `shape_idx` on `surface` — its `p:spPr > a:effectLst`
        as an interner-free `EffectListSpec` — or `None` when the shape declares no
        `a:effectLst` (its effects are then inherited; effective effect resolution is a later
        step). A shape whose effects use the rarer `a:effectDag` alternative also reads as
        `None` (that opaque graph is not modeled). Reading does not dirty the part.
        """
        ...
    def set_shape_effects(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, effects: EffectListSpec) -> None:
        """Sets the effects of shape `shape_idx` on `surface` from an interner-free
        `EffectListSpec`, rebuilding the `p:spPr` `a:effectLst` element (replacing an existing
        effect container in place — either an `a:effectLst` or the mutually-exclusive
        `a:effectDag`, which is overwritten — or inserting a new one after any geometry, fill,
        and outline, before the 3-D and extension children). Marks only that part dirty.
        """
        ...
    def set_shape_no_effects(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> None:
        """Sets shape `shape_idx` on `surface` to explicit "no effects" (an empty
        `<a:effectLst/>`). A shorthand for `set_shape_effects` with an empty `EffectListSpec` —
        the explicitly-cleared effect state that overrides inheritance, distinct from an absent
        `a:effectLst`. Reads back as `Some(EffectListSpec::default())`.
        """
        ...
    def shape_scene_3d(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> Scene3DSpec | None:
        """The **explicit** 3-D scene of shape `shape_idx` on `surface` — its `p:spPr > a:scene3d`
        (`CT_Scene3D`) as an interner-free `Scene3DSpec` — or `None` when the shape declares no
        `a:scene3d`. 3-D has no inheritance chain, so an absent scene means the shape is flat,
        not that it inherits one. A scene present but missing a schema-required part (its
        `a:camera` or `a:lightRig`) also reads as `None`. Reading does not dirty the part.
        """
        ...
    def set_shape_scene_3d(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, scene: Scene3DSpec) -> None:
        """Sets the 3-D scene of shape `shape_idx` on `surface` from an interner-free
        `Scene3DSpec`, rebuilding the `p:spPr` `a:scene3d` (replacing an existing one in place,
        or inserting a new one after any geometry, fill, outline, and effects, before `a:sp3d`).
        Rebuilding from a spec drops any opaque scene internals (`a:backdrop`, `extLst`). Marks
        only that part dirty.
        """
        ...
    def clear_shape_scene_3d(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> None:
        """Clears the 3-D scene of shape `shape_idx` on `surface` by **removing** its `a:scene3d`
        entirely — a shape without a scene is flat. Unlike effects, there is no "explicitly
        empty" scene: `CT_Scene3D` requires a camera and light rig, and 3-D does not inherit, so
        clearing removes rather than empties. A no-op (still `Ok`) when the shape has no scene.
        Marks the part dirty only if it removed something.
        """
        ...
    def shape_3d_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> Shape3DSpec | None:
        """The **explicit** 3-D properties of shape `shape_idx` on `surface` — its `p:spPr >
        a:sp3d` (`CT_Shape3D`: extrusion, contour, bevels, material) as an interner-free
        `Shape3DSpec` — or `None` when the shape declares no `a:sp3d`. Reading does not dirty
        the part.
        """
        ...
    def set_shape_3d_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, properties: Shape3DSpec) -> None:
        """Sets the 3-D properties of shape `shape_idx` on `surface` from an interner-free
        `Shape3DSpec`, rebuilding the `p:spPr` `a:sp3d` (replacing an existing one in place, or
        inserting a new one after every other visual property, before any `a:extLst`).
        Rebuilding from a spec drops any opaque `extLst`. Marks only that part dirty.
        """
        ...
    def clear_shape_3d_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> None:
        """Clears the 3-D properties of shape `shape_idx` on `surface` by **removing** its `a:sp3d`
        entirely. A no-op (still `Ok`) when the shape has none. Marks the part dirty only if it
        removed something.
        """
        ...
    def shape_bounds(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> ShapeBounds | None:
        """The position and size of shape `shape_idx` on `surface` **on the slide** — absolute
        within `slide_size`, whether the shape is top-level or nested inside groups.
        """
        ...
    def set_shape_bounds(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, bounds: ShapeBounds) -> None:
        """Moves and resizes shape `shape_idx` on `surface` to `bounds`, given **on the slide** —
        the same absolute space `shape_bounds` answers in. Creates the shape's transform element
        if it had none, and marks only that part dirty.
        """
        ...
    def shape_transform(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> Transform2D | None:
        """The **explicit** transform of shape `shape_idx` on `surface` — its position, size,
        rotation and mirror flags, plus the child coordinate space if it is a group — or `None`
        when the shape declares no transform at all.
        """
        ...
    def set_shape_transform(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, transform: Transform2D) -> None:
        """Applies `transform` to shape `shape_idx` on `surface`, creating its transform element if
        it had none. Marks only that part dirty; everything else re-emits verbatim.
        """
        ...
    def shape_geometry(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> Geometry:
        """The geometry of shape `shape_idx` on `surface`, as a `Geometry` — a preset shape
        (`Geometry::Preset`), a custom path list (`Geometry::Custom`), or `Geometry::Inherited`
        when the shape states no geometry of its own (it takes one from its placeholder /
        layout). Reading does not dirty the part.
        """
        ...
    def shape_adjustments(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, size: GuideContext) -> list[BoundedAdjustment]:
        """Every adjustment of shape `shape_idx`'s **preset** geometry, resolved against a concrete
        shape size: each value *and* the numeric domain it may move in.
        """
        ...
    def set_shape_geometry(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, geometry: Geometry) -> None:
        """Sets the geometry of shape `shape_idx` on `surface` from a `Geometry`: a preset shape
        (`Geometry::Preset`) rewrites the `a:prstGeom`, a custom path list (`Geometry::Custom`)
        writes an `a:custGeom`, and `Geometry::Inherited` removes the shape's own geometry so an
        inherited one takes over. The two kinds are mutually exclusive, so setting one drops the
        other. Marks only that slide part dirty; everything else re-emits verbatim.
        """
        ...
    def cell_text(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int) -> str:
        """The text of the cell at `(row, column)` — its paragraphs joined by newlines."""
        ...
    def visible_cell_text(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int) -> str:
        """The text that actually **renders** at `(row, column)` — the text of the cell if it
        stands alone, or of the merge **anchor** covering it if it is merged away.
        """
        ...
    def set_cell_text(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, run_idx: int, text: str) -> None:
        """Replaces the text of the `run_idx`-th run (flattened over the cell's paragraphs) of the
        cell at `(row, column)`. Marks only that part dirty.
        """
        ...
    def cell_paragraph_count(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int) -> int:
        """The number of paragraphs in the cell at `(row, column)`."""
        ...
    def cell_run_count(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, para_idx: int) -> int:
        """The number of runs in one paragraph of the cell at `(row, column)`."""
        ...
    def cell_paragraph_text(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, para_idx: int) -> str:
        """The text of one paragraph of the cell at `(row, column)`."""
        ...
    def cell_run_text(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, para_idx: int, run_idx: int) -> str:
        """The text of one run of the cell at `(row, column)`."""
        ...
    def cell_paragraph_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, para_idx: int) -> ParagraphPropertiesSpec | None:
        """The layout properties a paragraph of the cell at `(row, column)` declares of its own."""
        ...
    def cell_run_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, para_idx: int, run_idx: int) -> CharacterPropertiesSpec | None:
        """The character properties a run of the cell at `(row, column)` declares of its own."""
        ...
    def cell_end_run_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, para_idx: int) -> CharacterPropertiesSpec | None:
        """The paragraph-mark properties (`a:endParaRPr`) of a paragraph of the cell at `(row,
        column)` — the format an empty cell holds, and what text typed into it would take on.
        """
        ...
    def set_cell_run_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, para_idx: int, run_idx: int, spec: CharacterPropertiesSpec) -> None:
        """Applies `spec` to one run of one paragraph of the cell at `(row, column)`."""
        ...
    def set_cell_paragraph_run_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, para_idx: int, spec: CharacterPropertiesSpec) -> None:
        """Applies `spec` to **every run** of one paragraph of the cell at `(row, column)`, and to
        its paragraph mark.
        """
        ...
    def set_cell_run_properties_all(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, spec: CharacterPropertiesSpec) -> None:
        """Applies `spec` to **every run of every paragraph** of the cell at `(row, column)` — what
        selecting a whole cell and restyling it means, and the usual way to make a header bold.
        """
        ...
    def set_cell_end_run_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, para_idx: int, spec: CharacterPropertiesSpec) -> None:
        """Applies `spec` to a paragraph mark (`a:endParaRPr`) of the cell at `(row, column)`,
        creating the element if the paragraph has none — how an **empty** cell is formatted.
        """
        ...
    def set_cell_paragraph_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, para_idx: int, spec: ParagraphPropertiesSpec) -> None:
        """Applies `spec` to a paragraph's layout properties (`a:pPr`) in the cell at `(row,
        column)`, creating the element if it has none. The properties **merge**, as run
        properties do.
        """
        ...
    def set_cell_text_range_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, para_idx: int, range: range, spec: CharacterPropertiesSpec) -> None:
        """Applies `spec` to part of a paragraph of the cell at `(row, column)` — the characters in
        `range`, counted in Unicode scalars. Splits runs at the range's edges, exactly as the
        shape- addressed form does.
        """
        ...
    def cell_fill(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int) -> FillSpec | None:
        """The fill the cell at `(row, column)` declares, or `None` when it declares none — in
        which case the table style decides. Reading does not dirty the part.
        """
        ...
    def set_cell_fill(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, fill: FillSpec) -> None:
        """Fills the cell at `(row, column)`. Marks only that part dirty."""
        ...
    def clear_cell_fill(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int) -> None:
        """Removes the cell's own fill, so the table style decides how it is filled again."""
        ...
    def cell_border(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, edge: CellBorder) -> LineSpec | None:
        """The border the cell at `(row, column)` declares on `edge`, or `None` if it declares none
        there. Reading does not dirty the part.
        """
        ...
    def set_cell_border(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, edge: CellBorder, line: LineSpec) -> None:
        """Draws a border on one edge of the cell at `(row, column)`. Marks only that part dirty."""
        ...
    def cell_headers(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int) -> list[str]:
        """The ids of the header cells that describe the cell at `(row, column)` (`a:tcPr >
        a:headers`), in order — the accessibility association a screen reader announces. Empty
        when the cell names none. Reading does not dirty the part.
        """
        ...
    def set_cell_headers(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, header_ids: list[str]) -> None:
        """Sets the header-cell ids that describe the cell at `(row, column)`, replacing whatever
        it had; an empty slice removes the association. Marks only that part dirty.
        """
        ...
    def clear_cell_border(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, edge: CellBorder) -> None:
        """Removes the border on one edge of the cell at `(row, column)`."""
        ...
    def cell_margins(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int) -> CellMargins:
        """The four insets between the cell's edges and its text, each `None` when the cell does
        not state it. Reading does not dirty the part.
        """
        ...
    def set_cell_margins(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, margins: CellMargins) -> None:
        """Sets the cell's insets. Each field left `None` is **not written**, so a caller can set
        one margin without stating the other three.
        """
        ...
    def cell_anchor(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int) -> TextAnchoring | None:
        """Where the text sits vertically in the cell at `(row, column)`, or `None` if unstated
        (the wire default is `TextAnchoring::Top`). Reading does not dirty the part.
        """
        ...
    def set_cell_anchor(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, anchor: TextAnchoring) -> None:
        """Sets where the text sits vertically in the cell at `(row, column)`."""
        ...
    def cell_text_direction(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int) -> TextDirection | None:
        """Which way the text flows in the cell at `(row, column)`, or `None` if unstated (the wire
        default is `TextDirection::Horizontal`). Reading does not dirty the part.
        """
        ...
    def set_cell_text_direction(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, direction: TextDirection) -> None:
        """Sets which way the text flows in the cell at `(row, column)` — how a rotated header row
        is made.
        """
        ...
    def format_cells(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, cells: Cells, format: CellFormat) -> None:
        """Applies `format` to every cell in `cells`. Marks only that part dirty."""
        ...
    def format_cell_text(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, cells: Cells, spec: CharacterPropertiesSpec) -> None:
        """Applies `spec` to **every run of every paragraph** in each cell of `cells`, and to each
        paragraph's mark — bolding a header row in one call.
        """
        ...
    def format_cell_paragraphs(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, cells: Cells, spec: ParagraphPropertiesSpec) -> None:
        """Applies `spec` to the layout properties of **every paragraph** in each cell of `cells` —
        right- aligning a column of numbers in one call.
        """
        ...
    def merge_cells(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, cells: Cells) -> None:
        """Merges `cells` into one region. Marks only that part dirty."""
        ...
    def unmerge_cells(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int) -> None:
        """Undoes the merge covering the cell at `(row, column)`, whichever cell of the region is
        named. Marks only that part dirty.
        """
        ...
    def chart_series_fill(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int) -> FillSpec | None:
        """The fill of series `series_idx` of the chart the frame `shape_idx` on `surface`
        references — what colour it is drawn in — or `None` when the series declares none and
        takes its colour from the chart style. Reading does not dirty the part.
        """
        ...
    def set_chart_series_fill(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int, fill: FillSpec) -> None:
        """Sets the fill of series `series_idx` of the chart the frame `shape_idx` on `surface`
        references, creating its `c:spPr` if it had none. Marks only the chart part dirty.
        """
        ...
    def set_chart_series_line(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int, line: LineSpec) -> None:
        """Sets the outline of series `series_idx` of the chart the frame `shape_idx` on `surface`
        references — the line a line or radar plot draws, or the border of a bar or area. Marks
        only the chart part dirty.
        """
        ...
    def chart_data_labels(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int, point_idx: int | None = ...) -> DataLabelSettings:
        """The data-label settings **in force** for one point of series `series_idx` of the chart
        the frame `shape_idx` on `surface` references — the point's `c:dLbl` merged over the
        series' `c:dLbls` merged over the owning plot's.
        """
        ...
    def chart_data_label_tier(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, scope: ChartLabelScope) -> DataLabelSettings | None:
        """The data-label settings one **tier** states in its own right — what that tier
        contributes to the merge, with everything it leaves unset reported as `None`.
        """
        ...
    def chart_point_label_text(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int, point_idx: int) -> str | None:
        """The words one point's label shows in place of its value (`c:dLbl > c:tx`), or `None`
        when it states none and shows what the settings say. Reading does not dirty the part.
        """
        ...
    def set_chart_data_labels(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, scope: ChartLabelScope, spec: DataLabelSpec) -> None:
        """Applies `spec` at one tier of the chart's data labels, creating the element if that tier
        had none and leaving every setting `spec` does not state alone. Marks only the chart
        part dirty.
        """
        ...
    def delete_chart_data_labels(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, scope: ChartLabelScope) -> None:
        """Suppresses the labels at one tier — a `c:delete val="1"` in place of the settings, which
        is how one series of a labelled plot, or one point of a labelled series, is silenced
        without disturbing the rest. Marks only the chart part dirty.
        """
        ...
    def remove_chart_data_labels(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, scope: ChartLabelScope) -> bool:
        """Removes the `c:dLbls`/`c:dLbl` at one tier entirely, so that tier inherits the one above
        it again. Answers whether an element was there. Marks only the chart part dirty.
        """
        ...
    def chart_point_formats(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int) -> list[ChartPointFormatData]:
        """Every point of series `series_idx` that carries its own formatting (`c:dPt`), in
        document order. Reading does not dirty the part.
        """
        ...
    def set_chart_point_fill(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int, point_idx: int, fill: FillSpec) -> None:
        """Colours point `point_idx` of series `series_idx` differently from the rest of its
        series, creating its `c:dPt` at the schema rank if it had none. Marks only the chart
        part dirty.
        """
        ...
    def set_chart_point_line(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int, point_idx: int, line: LineSpec) -> None:
        """Outlines point `point_idx` of series `series_idx` differently from the rest of its
        series. Marks only the chart part dirty.
        """
        ...
    def set_chart_point_explosion(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int, point_idx: int, percent: int | None = ...) -> None:
        """Pulls slice `point_idx` of series `series_idx` out of the centre of its pie or doughnut
        by `percent` of the radius (`c:explosion`), or (for `None`) puts it back. Marks only the
        chart part dirty.
        """
        ...
    def remove_chart_point_format(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int, point_idx: int) -> bool:
        """Removes the formatting of point `point_idx` of series `series_idx`, so it is drawn like
        the rest of its series. Answers whether any was there. Marks only the chart part dirty.
        """
        ...
    def chart_trendlines(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int) -> list[ChartTrendlineData]:
        """Every trendline fitted through series `series_idx` (`c:trendline`), in document order.
        Reading does not dirty the part.
        """
        ...
    def add_chart_trendline(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int, spec: TrendlineSpec) -> None:
        """Fits a trendline through series `series_idx`. `c:trendline` repeats, so this **appends**
        — a series may carry a linear fit and a moving average at once. Marks only the chart
        part dirty.
        """
        ...
    def set_chart_trendline(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int, trendline_idx: int, spec: TrendlineSpec) -> None:
        """Rewrites trendline `trendline_idx` of series `series_idx` from `spec`, **in place** —
        the curve keeps its own `c:spPr` and any `c:trendlineLbl` it carries, and every optional
        setting `spec` leaves unset is cleared. Marks only the chart part dirty.
        """
        ...
    def remove_chart_trendlines(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int) -> int:
        """Removes every trendline from series `series_idx`, answering how many went. Marks only
        the chart part dirty.
        """
        ...
    def chart_error_bars(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int) -> list[ChartErrorBarData]:
        """Every set of error bars series `series_idx` carries (`c:errBars`) — one for a bar or
        line series, up to two (x and y) for scatter, area and bubble. Reading does not dirty
        the part.
        """
        ...
    def set_chart_error_bars(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int, spec: ErrorBarSpec) -> None:
        """Gives series `series_idx` error bars, replacing an existing set that runs along the same
        axis. Marks only the chart part dirty.
        """
        ...
    def remove_chart_error_bars(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int) -> int:
        """Removes every set of error bars from series `series_idx`, answering how many went. Marks
        only the chart part dirty.
        """
        ...
    def chart_dangling_decoration(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int) -> list[DanglingPointReference]:
        """Every `c:dPt` and `c:dLbl` of series `series_idx` whose `c:idx` names a point the series
        no longer has. Reading does not dirty the part.
        """
        ...
    def drop_chart_dangling_decoration(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int) -> int:
        """Removes every `c:dPt` and `c:dLbl` of series `series_idx` that names a point past the
        end of its data, answering how many went. Marks only the chart part dirty.
        """
        ...
    def add_chart(self, surface: int | Surface, chart: ChartData, bounds: ShapeBounds) -> int:
        """Adds `chart` to `surface` as a new chart, laid out inside `bounds`, and returns its
        index in the shape tree.
        """
        ...
    def chart_part_bytes(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> bytes | None:
        """The raw XML bytes of the chart part the chart frame `shape_idx` on `surface` references
        (`/ppt/charts/chartN.xml`), exactly as the package holds them, or `None` when the shape
        frames no chart. Borrowed from the package, so the part is not copied.
        """
        ...
    def chart_workbooks(self, surface: int | Surface) -> list[ChartWorkbook]:
        """Every chart on `surface` that references a backing workbook (`c:externalData`), with
        where each is referenced from and whether that reference is external.
        """
        ...
    def detach_chart_workbook(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> None:
        """Detaches the backing workbook from the chart `shape_idx` on `surface`: removes its
        `c:externalData` reference — the element and its relationship — leaving the chart to
        render from its cached values. This neutralizes a chart that links an unreachable
        external workbook (the caller decides accessibility; use `chart_workbooks` to find the
        candidates), and yields exactly the cache-only shape a freshly authored chart has.
        """
        ...
    def chart_series(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> list[ChartSeriesData]:
        """The series of the chart the frame `shape_idx` on `surface` references — for each, its
        name, category labels and values (for a scatter series, its X labels and Y values),
        flattened across the chart's plots. Reading does not dirty the part.
        """
        ...
    def set_chart_series_values(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int, values: Sequence[float]) -> None:
        """Rewrites the values of series `series_idx` (0-based across the chart's plots) of the
        chart the frame `shape_idx` on `surface` references — whichever source the series names:
        a `c:numRef`'s cache or a `c:numLit`.
        """
        ...
    def set_chart_series_categories(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, series_idx: int, labels: list[str]) -> None:
        """Rewrites the category labels of series `series_idx` (0-based across the chart's plots)
        of the chart the frame `shape_idx` on `surface` references, and refreshes the chart's
        embedded workbook alongside it.
        """
        ...
    def refresh_chart_workbook(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> bool:
        """Rewrites the embedded workbook of the chart the frame `shape_idx` on `surface`
        references so its cells hold exactly what the chart now draws, and answers whether it
        rewrote one.
        """
        ...
    def chart_kinds(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> list[ChartKind]:
        """The kind of every plot the chart the frame `shape_idx` on `surface` references draws, in
        document order — one entry per plot element, so a combo chart yields several. Reading
        does not dirty the part.
        """
        ...
    def chart_axes(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> list[ChartAxisData]:
        """The axes of the chart the frame `shape_idx` on `surface` references, in document order.
        Reading does not dirty the part.
        """
        ...
    def set_chart_axis_scale(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, axis_idx: int, minimum: float | None = ..., maximum: float | None = ...) -> None:
        """Sets or clears the explicit bounds of axis `axis_idx` (0-based, document order) of the
        chart the frame `shape_idx` on `surface` references. `None` returns that end of the axis
        to automatic scaling. Marks only the chart part dirty.
        """
        ...
    def set_chart_axis_orientation(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, axis_idx: int, orientation: AxisOrientation) -> None:
        """Sets the direction of axis `axis_idx` of the chart the frame `shape_idx` on `surface`
        references — smallest value first, or reversed. Marks only the chart part dirty.
        """
        ...
    def set_chart_axis_title(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, axis_idx: int, text: str | None = ...) -> None:
        """Sets or removes the title of axis `axis_idx` of the chart the frame `shape_idx` on
        `surface` references. `None` removes the title. Marks only the chart part dirty.
        """
        ...
    def set_chart_axis_gridlines(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, axis_idx: int, major: bool, minor: bool) -> None:
        """Turns the gridlines of axis `axis_idx` of the chart the frame `shape_idx` on `surface`
        references on or off. Marks only the chart part dirty.
        """
        ...
    def chart_title(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> str | None:
        """The heading of the chart the frame `shape_idx` on `surface` references (`c:title`), or
        `None` when it has none. Reading does not dirty the part.
        """
        ...
    def set_chart_title(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, text: str | None = ...) -> None:
        """Sets or removes the heading of the chart the frame `shape_idx` on `surface` references.
        `None` removes it. Marks only the chart part dirty.
        """
        ...
    def chart_legend(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> ChartLegendData | None:
        """The legend of the chart the frame `shape_idx` on `surface` references, or `None` when it
        has none. Reading does not dirty the part.
        """
        ...
    def set_chart_legend(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, position: LegendPosition | None = ...) -> None:
        """Places the legend of the chart the frame `shape_idx` on `surface` references at
        `position`, adding one if the chart had none. `None` removes the legend. Marks only the
        chart part dirty.
        """
        ...
    def chart_style_id(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> int | None:
        """The built-in style id the chart the frame `shape_idx` on `surface` references names
        (`c:style@val`, 1 to 48) — the palette and effect set Office draws an unstyled series
        with — or `None` when it names none. Reading does not dirty the part.
        """
        ...
    def slide_count(self) -> int:
        """The number of slides, in presentation order."""
        ...
    def master_count(self) -> int:
        """The number of slide masters, in `p:sldMasterIdLst` order."""
        ...
    def master_name(self, idx: int) -> str | None:
        """The name of master `idx` (`p:cSld@name`, e.g. `Office Theme`), or `None` if it is
        unnamed.
        """
        ...
    def layouts(self) -> list[LayoutInfo]:
        """Every slide layout the deck offers, in layout-index order — the inventory a caller reads
        before choosing one to build a slide on.
        """
        ...
    def layout_count(self) -> int:
        """The number of slide layouts across the whole deck, in (master order, `p:sldLayoutIdLst`
        order) — so layout indices run master by master. `layout_master` says which master an
        index belongs to.
        """
        ...
    def layout_master(self, idx: int) -> int | None:
        """The index of the master that lists layout `idx`."""
        ...
    def layout_name(self, idx: int) -> str | None:
        """The name of layout `idx` (`p:cSld@name`, e.g. `Title and Content` — the name PowerPoint
        shows in its layout gallery), or `None` if it is unnamed.
        """
        ...
    def layout_kind(self, idx: int) -> SlideLayoutKind:
        """How layout `idx` arranges its content (`p:sldLayout@type`) — a coarse description of
        which placeholders it offers, which an application can use to map between layouts.
        """
        ...
    def slide_layout(self, slide_idx: int) -> int | None:
        """The index of the layout slide `slide_idx` is built on, or `None` if the slide relates to
        no layout (or to one no master lists).
        """
        ...
    def slide_size(self) -> SlideSize:
        """The size of every slide in the deck (`p:sldSz`) — the extent shape bounds are laid out
        in.
        """
        ...
    def theme(self, surface: int | Surface) -> ThemeInfo | None:
        """The theme that governs `surface`, as an interner-free `ThemeInfo` (its color scheme +
        fill- style matrix) — the theme related to the last part of the surface's inheritance
        chain (slide → slideLayout → slideMaster → theme, and the shorter walks from a layout or
        master). Returns `Ok(None)` if any hop is absent (a deck without a theme). Reading does
        not dirty any part.
        """
        ...
    def color_map(self, surface: int | Surface) -> ColorMap | None:
        """The effective theme `ColorMap` for `surface`: the master's `p:clrMap` (reached along the
        surface's inheritance chain), replaced by the surface's own `p:clrMapOvr >
        a:overrideClrMapping` when it supplies a full mapping (a `masterClrMapping`, an absent
        override, or a schema-loose attribute-less override all inherit the master's map). It
        maps the logical color names a shape may reference (`bg1`/`tx1`/…) to the theme's
        concrete scheme slots. `Ok(None)` when there is no reachable master or no `p:clrMap`.
        Reading does not dirty a part.
        """
        ...
    def effective_shape_fill(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> FillSpec | None:
        """The **effective** fill of shape `shape_idx` on `surface`, as an interner-free `FillSpec`
        whose colors are resolved to concrete `RRGGBB` values — the fill the shape actually
        renders. Three sources are tried, in order: an explicit `p:spPr` fill; a `p:style >
        a:fillRef` (the theme fill- style at that index, `phClr` substituted by the reference's
        color); and, for a placeholder shape (`p:ph`), **inheritance** from the same-slot
        placeholder on the layout then the master. Scheme colors and color transforms are baked
        against the surface's theme + map.
        """
        ...
    def effective_shape_outline(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> LineSpec | None:
        """The **effective** outline of shape `shape_idx` on `surface`, as an interner-free
        `LineSpec` whose stroke color is resolved to a concrete `RRGGBB` value — the outline the
        shape actually renders. Three sources are tried, in order: an explicit `p:spPr > a:ln`;
        a `p:style > a:lnRef` (the theme line-style at that index, `phClr` substituted by the
        reference's color); and, for a placeholder shape (`p:ph`), **inheritance** from the
        same-slot placeholder on the slide layout then the master. Scheme colors and color
        transforms are baked against the slide's theme + map.
        """
        ...
    def effective_shape_effects(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> EffectListSpec | None:
        """The **effective** effects of shape `shape_idx` on `surface`, as an interner-free
        `EffectListSpec` whose colors are resolved to concrete `RRGGBB` values — the effects the
        shape actually renders. Three sources are tried, in order: an explicit `p:spPr >
        a:effectLst`; a `p:style > a:effectRef` (the theme effect-style at that index, `phClr`
        substituted by the reference's color); and, for a placeholder shape (`p:ph`),
        **inheritance** from the same-slot placeholder on the slide layout then the master.
        Scheme colors and color transforms are baked against the slide's theme + map.
        """
        ...
    def effective_shape_transform(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> Transform2D | None:
        """The **effective** transform of shape `shape_idx` on `surface` — where the shape actually
        renders, not what it declares. For a placeholder that places itself nowhere, this is the
        same- slot placeholder's transform on the slide layout, and failing that on the master.
        """
        ...
    def effective_shape_bounds(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> ShapeBounds | None:
        """The **effective** position and size of shape `shape_idx` on `surface` — where the shape
        actually renders, with the layout and master consulted for a placeholder that declares
        no bounds of its own.
        """
        ...
    def effective_run_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int, run_idx: int) -> CharacterPropertiesSpec:
        """The **effective** character properties of run `run_idx` — what the run actually renders
        as, with every tier of inheritance resolved and its colors baked to concrete `RRGGBB`.
        """
        ...
    def effective_paragraph_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int) -> ParagraphPropertiesSpec:
        """The **effective** paragraph properties of paragraph `para_idx` — the layout it actually
        renders with, every tier of inheritance resolved.
        """
        ...
    def effective_cell_fill(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int) -> FillSpec | None:
        """The **effective** fill of the cell at `(row, column)` of the table shape `shape_idx`
        frames — an interner-free `FillSpec` with its colour baked to concrete `RRGGBB`, or
        `None` if nothing fills the cell. The cell's own `a:tcPr` fill wins; else the first
        applicable style part with a fill (explicit or a theme `fillRef`).
        """
        ...
    def effective_cell_border(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, edge: CellBorder) -> LineSpec | None:
        """The **effective** border on one `edge` of the cell at `(row, column)` — an interner-free
        `LineSpec` with its stroke colour baked, or `None`. The cell's own `a:tcPr` edge wins;
        else the applicable style parts' `a:tcBdr`, taking the outer edge (`top`/`left`/…) for a
        cell on the table's rim and the interior edge (`insideH`/`insideV`) for one within it.
        """
        ...
    def effective_cell_run_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int, para_idx: int, run_idx: int) -> CharacterPropertiesSpec:
        """The **effective** run properties of a cell's text run — the `CharacterPropertiesSpec` it
        actually renders with, colours baked. A shorter ladder than a shape's (a cell inherits
        from its table style, not a placeholder chain), highest first: the run's own `a:rPr`,
        the paragraph's `a:defRPr`, the table style's `a:tcTxStyle` for each applicable part
        (bold / italic / colour), then the presentation's `p:defaultTextStyle`.
        """
        ...
    def remove_unused_parts(self) -> list[str]:
        """Removes every part the package no longer reaches from its root, and reports what was
        swept.
        """
        ...
    def external_links(self) -> list[ExternalLink]:
        """Every relationship in the package whose target lies **outside** it — a linked image, a
        chart's external workbook, a linked OLE object or media file — with the part that owns
        each.
        """
        ...
    def retarget_external_link(self, source: str | None, id: str, new_target: str, mode: TargetMode) -> bool:
        """Repoints the relationship `id` of `source` (`None` = the package root) at `new_target`,
        keeping its id and its place in the `.rels`. Returns whether one was found.
        """
        ...
    def run_hyperlink(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int, run_idx: int) -> Hyperlink | None:
        """The click hyperlink on run `run_idx` of paragraph `para_idx` in shape `shape_idx` on
        `surface`, resolved to a `Hyperlink` (a URL or a slide index), or `None` if the run has
        no hyperlink — or one this build does not model (a mouse-over action, a show jump).
        Reading does not dirty the part.
        """
        ...
    def set_run_hyperlink(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int, run_idx: int, link: Hyperlink) -> None:
        """Sets the click hyperlink on run `run_idx` of paragraph `para_idx` in shape `shape_idx`
        to `link`, adding its relationship. If the run already linked somewhere, that
        relationship is removed once nothing else in the part still names it.
        """
        ...
    def clear_run_hyperlink(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int, run_idx: int) -> None:
        """Removes the click hyperlink on run `run_idx` of paragraph `para_idx` in shape
        `shape_idx`, and the relationship it named once nothing else in the part still
        references it. A no-op if the run has no hyperlink.
        """
        ...
    def set_text_range_hyperlink(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int, range: range, link: Hyperlink) -> None:
        """Sets the click hyperlink over a **scalar range** of paragraph `para_idx` in shape
        `shape_idx`, splitting runs at the boundaries so exactly the selected text is linked (as
        `set_text_range_properties` does). One relationship is added and shared by every run in
        the range. An empty range links nothing.
        """
        ...
    def shape_hyperlink(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> Hyperlink | None:
        """The click hyperlink on shape `shape_idx` itself (`p:cNvPr > a:hlinkClick`), resolved to
        a `Hyperlink`, or `None` if the shape has no hyperlink (or one this build does not
        model). Reading does not dirty the part.
        """
        ...
    def set_shape_hyperlink(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, link: Hyperlink) -> None:
        """Sets the click hyperlink on shape `shape_idx` itself to `link`, adding its relationship
        and removing the one any previous link named once unreferenced.
        """
        ...
    def clear_shape_hyperlink(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> None:
        """Removes the click hyperlink on shape `shape_idx` itself, and the relationship it named
        once unreferenced. A no-op if the shape has no hyperlink.
        """
        ...
    def ole_object_part_bytes(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> bytes | None:
        """The raw bytes of the embedded object the OLE frame `shape_idx` on `surface` references
        (`/ppt/embeddings/oleObjectN.bin` or an embedded package), exactly as the package holds
        them, or `None` when the shape frames no OLE object. Borrowed from the package, so the
        part is not copied.
        """
        ...
    def ole_snapshot_image_bytes(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> bytes | None:
        """The stored bytes of the OLE fallback snapshot image the frame `shape_idx` on `surface`
        embeds, exactly as the package holds them (never decoded or re-encoded), or `None` when
        the frame is not an OLE object or carries no snapshot. Borrowed from the package.
        """
        ...
    def ole_prog_id(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> str | None:
        """The `progId` the OLE frame `shape_idx` on `surface` declares (e.g. `"Excel.Sheet.12"`) —
        the application that owns the embedded object — or `None` when the shape frames no OLE
        object or the attribute is absent. Reading does not dirty the part.
        """
        ...
    def ole_objects(self, surface: int | Surface) -> list[OleObject]:
        """Every OLE object frame on `surface`, with where its object data is referenced from and
        whether that reference is external.
        """
        ...
    def replace_ole_object_with_placeholder(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, placeholder: bytes | None = ...) -> None:
        """Replaces the object data of the OLE frame `shape_idx` on `surface` with an in-package
        placeholder, so an object that points at unreachable external data resolves inside the
        package instead. The placeholder is `placeholder` if given, else
        `default_placeholder_ole` (a minimal valid compound file). The `p:oleObj` markup is
        unchanged — its relationship is simply retargeted at the placeholder — and the object
        keeps displaying via its snapshot image.
        """
        ...
    def activex_control_count(self, surface: int | Surface) -> int:
        """The number of legacy **ActiveX** form controls on `surface` (`p:cSld > p:controls >
        p:control`).
        """
        ...
    def activex_control_name(self, surface: int | Surface, control_idx: int) -> str | None:
        """The `name` the ActiveX control `control_idx` on `surface` declares (e.g.
        `"CommandButton1"`), or `None` when there is no such control or it is unnamed. Reading
        does not dirty the part.
        """
        ...
    def activex_part_bytes(self, surface: int | Surface, control_idx: int) -> bytes | None:
        """The raw bytes of the ActiveX control part (`ax:ocx` markup) the control `control_idx` on
        `surface` references, exactly as the package holds them, or `None` when there is no such
        control. Borrowed from the package; reading does not dirty anything.
        """
        ...
    def activex_state_bytes(self, surface: int | Surface, control_idx: int) -> bytes | None:
        """The ActiveX control's **persisted state** — the bytes of `/ppt/activeX/activeXN.bin` —
        for the control `control_idx` on `surface`, or `None` when there is no such control or
        it persists no state. Borrowed from the package; reading does not dirty anything.
        """
        ...
    def activex_snapshot_image_bytes(self, surface: int | Surface, control_idx: int) -> bytes | None:
        """The stored bytes of the ActiveX control's fallback snapshot image for the control
        `control_idx` on `surface`, exactly as the package holds them (never decoded or re-
        encoded), or `None` when there is no such control or snapshot. Borrowed from the
        package.
        """
        ...
    def ink_part_names(self) -> list[str]:
        """The names of every **ink** (InkML) part in the package (`ppt/ink/inkN.xml`), in package
        order.
        """
        ...
    def ink_part_bytes(self, part: str) -> bytes | None:
        """The raw bytes of the ink (InkML) `part`, exactly as the package holds them, or `None`
        when the package has no such part (or it has been edited elsewhere). Borrowed from the
        package, so the part is not copied and nothing is dirtied.
        """
        ...
    def ink_references(self, surface: int | Surface) -> list[InkReference]:
        """Every ink (InkML) part `surface` references, with where it is referenced from."""
        ...
    def ink_part_for_shape(self, surface: int | Surface, shape_idx: int) -> str | None:
        """The ink part the shape `shape_idx` on `surface` references, or `None` when that shape is
        not a content part or does not reference ink.
        """
        ...
    def shape_for_ink_part(self, surface: int | Surface, part: str) -> int | None:
        """The shape index of the content part on `surface` that references the ink `part`, or
        `None` when no shape on that surface does (or the reference lives inside an
        `mc:AlternateContent`, which is out of the shape index space).
        """
        ...
    def add_ink(self, surface: int | Surface, inkml: bytes) -> int:
        """Adds an ink (InkML) part holding `inkml` to the package and a `p:contentPart`
        referencing it to `surface`, and returns the new shape's index in the one shape index
        space.
        """
        ...
    def set_ink_content(self, surface: int | Surface, shape_idx: int, inkml: bytes) -> None:
        """Replaces the strokes of the ink the shape `shape_idx` on `surface` references, in place."""
        ...
    def diagram_relationship_ids(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> DiagramRelationshipIds | None:
        """The four relationship ids the SmartArt frame `shape_idx` on `surface` names in its
        `dgm:relIds`, or `None` when the shape frames no diagram. Reading does not dirty the
        part.
        """
        ...
    def diagram_parts(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> DiagramParts | None:
        """The parts of the SmartArt diagram the frame `shape_idx` on `surface` references,
        resolved to part names — the relationship graph behind the diagram, `None` when the
        shape frames none.
        """
        ...
    def diagram_part_bytes(self, part: str) -> bytes | None:
        """The raw bytes of a diagram `part`, exactly as the package holds them, or `None` when the
        package has no such part (or it has been edited elsewhere). Borrowed; nothing is
        dirtied.
        """
        ...
    def add_diagram(self, surface: int | Surface, content: DiagramContent, bounds: ShapeBounds) -> int:
        """Adds a SmartArt diagram to `surface`, laid out inside `bounds`, and returns its index in
        the shape tree.
        """
        ...
    def set_diagram_part(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, kind: DiagramPartKind, bytes: bytes) -> None:
        """Replaces one part of the SmartArt diagram the frame `shape_idx` on `surface` references,
        in place.
        """
        ...
    def add_ole_object(self, surface: int | Surface, spec: OleObjectSpec, bounds: ShapeBounds) -> int:
        """Adds an OLE object to `surface`, laid out inside `bounds`, and returns its index in the
        shape tree.
        """
        ...
    def set_ole_prog_id(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, prog_id: str) -> None:
        """Sets the `progId` of the OLE frame `shape_idx` on `surface` — which application owns the
        embedded object. Only the surface's part is dirtied.
        """
        ...
    def set_ole_object_data(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, bytes: bytes) -> None:
        """Replaces the data of the OLE object the frame `shape_idx` on `surface` embeds, in place."""
        ...
    def set_ole_snapshot_image(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, bytes: bytes) -> None:
        """Replaces the fallback snapshot image of the OLE frame `shape_idx` on `surface` — the
        picture a consumer draws in place of the object it will never run.
        """
        ...
    def add_activex_control(self, surface: int | Surface, spec: ActiveXControlSpec, bounds: ShapeBounds) -> int:
        """Adds an ActiveX form control to `surface`, laid out inside `bounds`, and returns its
        index in the surface's **control** index space (not the shape index space — a
        `p:control` is a sibling of the shape tree, not a member of it).
        """
        ...
    def set_ole_legacy_shape_id(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, identifier: str) -> None:
        """Points the OLE frame `shape_idx` on `surface` at the VML shape with `identifier`
        (`p:oleObj@spid`) — how an authored object is bound to the legacy fallback that draws
        it.
        """
        ...
    def set_activex_control_shape_id(self, surface: int | Surface, control_idx: int, identifier: str) -> None:
        """Points the ActiveX control `control_idx` on `surface` at the VML shape with `identifier`
        (`p:control@spid`). As `set_ole_legacy_shape_id`.
        """
        ...
    def activex_control_shape_id(self, surface: int | Surface, control_idx: int) -> str | None:
        """The `spid` the ActiveX control `control_idx` on `surface` names — the `id` of the VML
        shape that draws it in a legacy consumer — or `None` when there is no such control or it
        names none.
        """
        ...
    def activex_class_id(self, surface: int | Surface, control_idx: int) -> str | None:
        """The COM class id the ActiveX control `control_idx` on `surface` names
        (`ax:ocx@ax:classid`), or `None` when there is no such control or its part states none.
        """
        ...
    def activex_persistence(self, surface: int | Surface, control_idx: int) -> ActiveXPersistence | None:
        """How the ActiveX control `control_idx` on `surface` persists its state
        (`ax:ocx@ax:persistence`), or `None` when there is no such control, its part states
        none, or it names a value the ActiveX part does not define.
        """
        ...
    def set_activex_control_name(self, surface: int | Surface, control_idx: int, name: str) -> None:
        """Renames the ActiveX control `control_idx` on `surface` (`p:control@name`). Only the
        surface's part is dirtied.
        """
        ...
    def set_activex_state(self, surface: int | Surface, control_idx: int, state: bytes) -> None:
        """Replaces the persisted state of the ActiveX control `control_idx` on `surface`, in
        place.
        """
        ...
    def set_activex_snapshot_image(self, surface: int | Surface, control_idx: int, bytes: bytes) -> None:
        """Replaces the fallback snapshot image of the ActiveX control `control_idx` on `surface` —
        the picture a consumer draws in place of the control it will never run.
        """
        ...
    def remove_activex_control(self, surface: int | Surface, control_idx: int) -> None:
        """Removes the ActiveX control `control_idx` from `surface`, closing the gap in the control
        index space. Only the surface's part is dirtied.
        """
        ...
    def ole_legacy_shape_id(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> str | None:
        """The `spid` the OLE frame `shape_idx` on `surface` names — the `id` of the VML shape that
        draws it in a legacy consumer — or `None` when the shape frames no OLE object or names
        no `spid`.
        """
        ...
    def notes_text(self, slide_idx: int) -> str | None:
        """The speaker notes of slide `slide_idx` — the text of its notes slide's `body`
        placeholder — or `None` if the slide has no notes slide (or its notes slide has no body
        placeholder).
        """
        ...
    def set_notes_text(self, slide_idx: int, text: str) -> None:
        """Sets the speaker notes of slide `slide_idx` to `text`, creating the notes slide (and, if
        the deck has none, the notes master it follows) on demand.
        """
        ...
    def clear_notes(self, slide_idx: int) -> None:
        """Removes the speaker notes of slide `slide_idx`: unwires the slide → notes-slide
        relationship and removes the notes slide part (with its `.rels` and content-type
        override). A no-op if the slide has no notes.
        """
        ...
    def add_picture(self, surface: int | Surface, bytes: bytes, bounds: ShapeBounds) -> int:
        """Appends a picture (`p:pic`) showing `bytes` to `surface`, laid out at `bounds`. Returns
        the index of the new shape in the slide's one shape index space (see `shape_count`);
        `shape_kind` reports it as `ShapeKind::Picture`, and the whole `p:spPr` surface —
        outline, effects, geometry — applies to it like any other shape.
        """
        ...
    def media_references(self, surface: int | Surface) -> list[MediaReference]:
        """Every audio/video/media relationship on `surface`, with where each is referenced from
        and whether it is external.
        """
        ...
    def replace_media_with_placeholder(self, surface: int | Surface, rel_id: str, placeholder: bytes | None = ...) -> None:
        """Replaces the media that relationship `rel_id` on `surface` binds with an in-package
        placeholder, so a reference to unreachable external audio/video resolves inside the
        package instead. The placeholder is `placeholder` if given, else a built-in one matching
        the media kind — a valid silent WAV for audio (`default_placeholder_audio`) or a minimal
        MP4 for video (`default_placeholder_video`). The relationship is retargeted at the
        placeholder, so every carrier that named it — the `p:pic`, its `a14:media` fallback,
        timing/transition sounds — now resolves locally; the poster image is untouched.
        """
        ...
    def picture_image_link_target(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> str | None:
        """The target of the image that picture `shape_idx` on `surface` *links* (`p:blipFill >
        a:blip@r:link`), exactly as the relationship records it — an external path/URL for the
        common case, or an in-package part target for an internal link. `None` when the picture
        embeds its image (or binds none): an embedded image has no separate target, its bytes
        are the image.
        """
        ...
    def picture_image_bytes(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> bytes | None:
        """The stored bytes of the image that picture `shape_idx` on `surface` binds, exactly as
        the package holds them (never decoded or re-encoded), or `None` when the picture binds
        no image. Borrowed from the package, so a large image is not copied.
        """
        ...
    def set_picture_image(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, bytes: bytes) -> None:
        """Points picture `shape_idx` on `surface` at `bytes`, adding the image to the package if
        it is not already there (`add_image`, so identical bytes are stored once) and rewriting
        the blip's `@r:embed`. Any `@r:link` is dropped — the picture now embeds its image — and
        the rest of the `p:blipFill` (source rect, tile/stretch) is preserved.
        """
        ...
    def linked_images(self, surface: int | Surface) -> list[LinkedImage]:
        """Every picture on `surface` that *links* its image (`a:blip@r:link`) rather than
        embedding it, with where each links from — the candidates for
        `replace_linked_image_with_placeholder`. A linked image is the common source that can be
        unreachable on another platform; this saves the caller from walking the shapes
        themselves. Reading does not dirty the part.
        """
        ...
    def replace_linked_image_with_placeholder(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, placeholder: bytes | None = ...) -> None:
        """Replaces the *linked* image of picture `shape_idx` on `surface` with an embedded
        placeholder, so a picture that points at an unreachable external file resolves inside
        the package instead. The placeholder is `placeholder` if given, else
        `DEFAULT_PLACEHOLDER_IMAGE`. The picture becomes an ordinary embedded picture (`@r:link`
        → `@r:embed`), keeping its bounds and the rest of its `p:blipFill`, and the now-unused
        link relationship is dropped.
        """
        ...
    def add_image(self, surface: int | Surface, bytes: bytes) -> str:
        """Stores `bytes` as an image part of the package and relates it to `surface`, returning
        the **slide-scoped relationship id** that names the image — the `rel_id` to hand to
        `FillSpec::Picture` via `set_shape_fill`.
        """
        ...
    def shape_count(self, surface: int | Surface) -> int:
        """The number of **top-level** shapes on `surface` — of **every** `ShapeKind` (autoshapes,
        pictures, groups, graphic frames, connectors), in document order. A group counts as one
        shape here; its own members are addressed by descending into it with a `ShapePath` and
        are not included in this count.
        """
        ...
    def shape_kind(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> ShapeKind:
        """What kind of shape `shape_idx` on `surface` is — which of the index-addressed APIs apply
        to it (a `Picture` takes the `p:spPr` surface but has no text body; a `GroupShape` has
        no `p:spPr` at all).
        """
        ...
    def shape_member_count(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> int:
        """How many member shapes the group at `shape_idx` holds — `0` for anything that is not a
        group, since only a `p:grpSp` has members. This is the range a `ShapePath` may descend
        into.
        """
        ...
    def shapes(self, surface: int | Surface) -> list[ShapeInfo]:
        """Every shape of `surface`, in document order — what it is and the placeholder slot it
        fills.
        """
        ...
    def shape_for_placeholder(self, surface: int | Surface, kind: PlaceholderType) -> int | None:
        """The address of the first shape on `surface` that fills the `kind` placeholder slot, or
        `None` if the surface offers none.
        """
        ...
    def shape_placeholder(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> PlaceholderInfo | None:
        """The placeholder shape `shape_idx` on `surface` occupies (`p:nvPr > p:ph`), or `None` if
        it is not a placeholder.
        """
        ...
    def add_text_box(self, surface: int | Surface, text: str, bounds: ShapeBounds) -> int:
        """Appends a new rectangular text-box shape (`p:sp`) to `surface`, laid out at `bounds` and
        containing `text` (one paragraph per line, split on `\n`). Returns the index of the new
        shape in the slide's one shape index space (see `shape_count`). Only that part is marked
        dirty.
        """
        ...
    def add_shape(self, surface: int | Surface, preset: PresetShapeType, bounds: ShapeBounds) -> int:
        """Appends a new autoshape (`p:sp`) with the given `preset` geometry to `surface`, laid out
        at `bounds`, with an empty text body. Returns the index of the new shape in the slide's
        one shape index space (see `shape_count`). Only that part is marked dirty.
        """
        ...
    def remove_shape(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> None:
        """Removes shape `shape_idx` from `surface`, closing the gap in the shape index space:
        every later shape on that surface moves down one index. Only that part is marked dirty.
        """
        ...
    def group_shapes(self, surface: int | Surface, members: Sequence[int | Sequence[int] | ShapePath]) -> ShapePath:
        """Wraps `members` — which must be siblings — in a new group, returning the group's
        address.
        """
        ...
    def ungroup(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> list[ShapePath]:
        """Dissolves the group at `shape_idx`, returning where its members now are."""
        ...
    def move_shape_into_group(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, group_idx: int | Sequence[int] | ShapePath) -> ShapePath:
        """Moves shape `shape_idx` into the group at `group_idx`, as its last member, and returns
        its new address.
        """
        ...
    def move_shape_out_of_group(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> ShapePath:
        """Moves shape `shape_idx` out of the group holding it, into that group's own container and
        directly after it in z-order. Returns its new address.
        """
        ...
    def graphic_frame_kind(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> GraphicFrameKind | None:
        """What the graphic frame `shape_idx` on `surface` frames — a `Table`, a `Chart`, a
        `Diagram` or something else — or `None` when the shape is not a `p:graphicFrame` at all.
        Reading does not dirty the part.
        """
        ...
    def add_slide(self) -> int:
        """Adds a new empty slide at the end of the deck, wired to the same slide layout as slide 0
        — or, on a deck with no slides yet, to the deck's first layout — and returns its index.
        The new slide is a blank shape tree; add content with `add_text_box` or use
        `add_slide_with_text`.
        """
        ...
    def add_slide_from_layout(self, layout_idx: int) -> int:
        """Adds a new slide at the end of the deck built on layout `layout_idx`, carrying a copy of
        every placeholder that layout declares, and returns the slide's index.
        """
        ...
    def remove_slide(self, slide_idx: int) -> None:
        """Removes slide `slide_idx` from the deck, unwiring it completely: the `p:sldId` naming
        it, the presentation's relationship to it, the slide part, its own `.rels`, and its
        content-type `Override`.
        """
        ...
    def add_slide_with_text(self, text: str, bounds: ShapeBounds) -> int:
        """Adds a new slide (via `add_slide`) carrying a single text box with `text` laid out at
        `bounds`, and returns the new slide's index.
        """
        ...
    def add_table(self, surface: int | Surface, rows: int, columns: int, bounds: ShapeBounds) -> int:
        """Adds a `rows` x `columns` table to `surface`, laid out inside `bounds`, and returns its
        index in the shape tree.
        """
        ...
    def table_dimensions(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> tuple[int, int]:
        """The shape of the table shape `shape_idx` on `surface` frames, as `(rows, columns)`."""
        ...
    def column_width(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, column: int) -> Emu | None:
        """The width of column `column` of the table shape `shape_idx` frames, or `None` if the
        column states none. Reading does not dirty the part.
        """
        ...
    def set_column_width(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, column: int, width: Emu) -> None:
        """Sets the width of column `column`. Marks only that part dirty."""
        ...
    def row_height(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int) -> Emu | None:
        """The height row `row` asks for, or `None` if it states none. PowerPoint grows a row whose
        content does not fit, so a rendered row is never shorter than this but may be taller.
        """
        ...
    def set_row_height(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, height: Emu) -> None:
        """Sets the height row `row` asks for. Marks only that part dirty."""
        ...
    def insert_row(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int) -> None:
        """Inserts a row into the table shape `shape_idx` frames so it becomes row `row`; `row`
        equal to the current row count appends at the end. The new row copies the height of the
        row beside it and its cells are empty and ready for `set_cell_text`. A merge the new row
        falls inside grows to include it. Marks only that part dirty; the frame's own bounds are
        **not** enlarged (as PowerPoint does not either — resize with `set_shape_bounds`).
        """
        ...
    def remove_row(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int) -> None:
        """Removes row `row` from the table shape `shape_idx` frames. A merge the row lies inside
        shrinks; a merge anchored in the row promotes the cell below it, which takes over the
        anchor's text and formatting so the table looks unchanged. Marks only that part dirty.
        """
        ...
    def insert_column(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, column: int) -> None:
        """Inserts a column into the table shape `shape_idx` frames so it becomes column `column`;
        `column` equal to the current column count appends. The grid gains one `a:gridCol`
        (width copied from the column beside it) and every row gains one empty cell, so the grid
        and rows stay in step. A merge the new column falls inside grows to include it. Marks
        only that part dirty; the frame's own bounds are **not** enlarged.
        """
        ...
    def remove_column(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, column: int) -> None:
        """Removes column `column` from the table shape `shape_idx` frames: its `a:gridCol` and one
        cell from every row, together. A merge the column lies inside shrinks; a merge anchored
        in the column promotes the cell to its right, which takes over the anchor's text and
        formatting. Marks only that part dirty.
        """
        ...
    def cell_span(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int) -> tuple[int, int]:
        """How many rows and columns the cell at `(row, column)` spans, as `(rows, columns)` — the
        same order `table_dimensions` answers in, and the order every address on this surface is
        written in.
        """
        ...
    def merged_cell_anchor(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, row: int, column: int) -> tuple[int, int]:
        """Which cell actually renders at `(row, column)` — itself when it is not merged away, or
        the anchor of the merged region covering it.
        """
        ...
    def table_part(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, part: TablePart) -> bool | None:
        """Whether the table shape `shape_idx` frames declares banding/emphasis `part` (a `a:tblPr`
        flag), or `None` if it does not state the flag. Reading does not dirty the part.
        """
        ...
    def set_table_part(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, part: TablePart, on: bool) -> None:
        """Turns a table's banding/emphasis flag `part` on or off, creating its `a:tblPr` if it had
        none. `false` removes the flag rather than writing a `"0"`. Marks only that part dirty.
        """
        ...
    def table_style_id(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> str | None:
        """The GUID of the table style the table shape `shape_idx` frames names (`a:tableStyleId`),
        or `None` if it names none. Reading does not dirty the part.
        """
        ...
    def set_table_style(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, style_id: str) -> None:
        """Points the table shape `shape_idx` frames at the table style `style_id`, creating its
        `a:tblPr` if it had none. Does not check that the style exists — pair it with
        `create_table_style`. Marks only that part dirty.
        """
        ...
    def create_table_style(self, style_id: str, style_name: str) -> None:
        """Creates the presentation's `tableStyles.xml` part if it has none, and adds a style with
        GUID `style_id` and gallery name `style_name` — replacing one already carrying that
        GUID. The style is born empty; give its parts formatting with `format_table_style_part`,
        and point a table at it with `set_table_style`.
        """
        ...
    def format_table_style_part(self, style_id: str, part: TableStylePart, format: TableStyleFormat) -> None:
        """Sets the formatting the style `style_id` gives table `part` (`wholeTbl`, `firstRow`, a
        banded row, a corner cell). Only the facets `format` sets are written; the part keeps
        whatever else it held. Marks only the `tableStyles.xml` part dirty.
        """
        ...
    def set_inline_table_style(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, definition: TableStyleDefinition) -> None:
        """Gives the table shape `shape_idx` frames its own **inline** style (`a:tableStyle`),
        replacing any inline or referenced style it had — the lean alternative to a shared
        `tableStyles.xml` style: the whole look is spelled out in `definition` and travels with
        the table, so no shared part, relationship or referenced GUID is involved. Marks only
        that part dirty.
        """
        ...
    def format_inline_table_style_part(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, part: TableStylePart, format: TableStyleFormat) -> None:
        """Sets the formatting the table's **inline** style gives one `part`, creating the inline
        style if the table had none — the incremental sibling of `set_inline_table_style`,
        mirroring `format_table_style_part` for a self-contained style. Only the facets `format`
        sets are written. Marks only that part dirty.
        """
        ...
    def shape_text(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> str:
        """The full text of shape `shape_idx` on `surface` (paragraphs joined by `\n`)."""
        ...
    def set_shape_text(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, run_idx: int, text: str) -> None:
        """Replaces the text of the `run_idx`-th run (flattened over the shape's paragraphs, in
        document order) of shape `shape_idx` on `surface`. Marks only that part dirty.
        """
        ...
    def set_shape_text_content(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, text: str) -> None:
        """Replaces the **whole text** of shape `shape_idx` on `surface` with `text` — one
        paragraph per line, each holding exactly one run, so `shape_text` reads back exactly
        what was written. Marks only that part dirty.
        """
        ...
    def paragraph_count(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> int:
        """The number of paragraphs in shape `shape_idx`'s text body. Reading does not dirty the
        part.
        """
        ...
    def run_count(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int) -> int:
        """The number of runs in paragraph `para_idx` of shape `shape_idx`. Reading does not dirty
        the part.
        """
        ...
    def paragraph_text(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int) -> str:
        """The text of paragraph `para_idx` — its runs concatenated. Reading does not dirty the
        part.
        """
        ...
    def run_text(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int, run_idx: int) -> str:
        """The text of one run. Reading does not dirty the part."""
        ...
    def paragraph_field_count(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int) -> int:
        """The number of text fields (`a:fld`) in paragraph `para_idx` — generated values such as a
        slide number or a date. Fields are a **separate index space** from the runs, so a field
        never shifts a run index. Reading does not dirty the part.
        """
        ...
    def paragraph_field_text(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int, field_idx: int) -> str:
        """The cached text of field `field_idx` in paragraph `para_idx` — the value the producer
        last computed for it (a slide number, a formatted date), not a live value. Reading does
        not dirty the part.
        """
        ...
    def paragraph_field_type(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int, field_idx: int) -> str | None:
        """What field `field_idx` in paragraph `para_idx` generates (`a:fld@type`, e.g. `slidenum`
        or `datetime`), or `None` if it names no type. Reading does not dirty the part.
        """
        ...
    def paragraph_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int) -> ParagraphPropertiesSpec | None:
        """The layout properties a paragraph declares of its own (`a:pPr`), or `None` if it
        declares none — in which case every property is inherited. Reading does not dirty the
        part.
        """
        ...
    def run_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int, run_idx: int) -> CharacterPropertiesSpec | None:
        """The character properties a run declares of its own (`a:rPr`), or `None` if it declares
        none. Reading does not dirty the part.
        """
        ...
    def end_run_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int) -> CharacterPropertiesSpec | None:
        """The paragraph-mark properties (`a:endParaRPr`), or `None` if the paragraph declares
        none.
        """
        ...
    def set_run_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int, run_idx: int, spec: CharacterPropertiesSpec) -> None:
        """Applies `spec` to one run's character properties, creating its `a:rPr` if it has none."""
        ...
    def set_paragraph_run_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int, spec: CharacterPropertiesSpec) -> None:
        """Applies `spec` to **every run** in paragraph `para_idx`, and to its `a:endParaRPr` if it
        has one — so text typed at the end of the paragraph takes the same formatting, which is
        what selecting a paragraph and restyling it means.
        """
        ...
    def set_shape_run_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, spec: CharacterPropertiesSpec) -> None:
        """Applies `spec` to **every run of every paragraph** in the shape, and to each paragraph's
        `a:endParaRPr` where present — selecting a whole text box and restyling it.
        """
        ...
    def coalesce_paragraph_runs(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int) -> int:
        """Merges adjacent runs in paragraph `para_idx` that would render identically, returning
        the number of runs merged away. This undoes the run splitting that
        `set_text_range_properties` does: formatting a sub-range splits a run, and repeatedly
        formatting overlapping ranges leaves a paragraph with more runs than it needs.
        """
        ...
    def coalesce_shape_runs(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> int:
        """Merges adjacent identical runs across **every** paragraph of a shape's text body,
        returning the total number of runs merged away. The per-paragraph rule is
        `coalesce_paragraph_runs`.
        """
        ...
    def set_end_run_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int, spec: CharacterPropertiesSpec) -> None:
        """Applies `spec` to the paragraph-mark properties (`a:endParaRPr`), creating the element
        if the paragraph has none.
        """
        ...
    def set_paragraph_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int, spec: ParagraphPropertiesSpec) -> None:
        """Applies `spec` to a paragraph's layout properties (`a:pPr`), creating the element if it
        has none. The properties **merge**, as run properties do.
        """
        ...
    def shape_list_style_level(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, level: IndentLevel) -> ParagraphPropertiesSpec | None:
        """The layout properties the shape's own list style offers at `level` (`a:lstStyle >
        a:lvlNpPr`), or `None` if it offers none there — or declares no list style at all.
        Reading does not dirty the part.
        """
        ...
    def shape_list_style_default(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> ParagraphPropertiesSpec | None:
        """The properties the shape's own list style offers where no level applies (`a:lstStyle >
        a:defPPr`), or `None` if it declares none. Reading does not dirty the part.
        """
        ...
    def set_shape_list_style_level(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, level: IndentLevel, spec: ParagraphPropertiesSpec) -> None:
        """Applies `spec` to what the shape's own list style offers at `level`, creating the
        `a:lstStyle` — and the `a:lvlNpPr` within it — if the shape has none. Marks only that
        part dirty.
        """
        ...
    def set_shape_list_style_default(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, spec: ParagraphPropertiesSpec) -> None:
        """Applies `spec` to what the shape's own list style offers where no level applies
        (`a:lstStyle > a:defPPr`), creating the elements if the shape has none. Marks only that
        part dirty. Merges as `set_shape_list_style_level` does.
        """
        ...
    def clear_shape_list_style_level(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, level: IndentLevel) -> bool:
        """Removes what the shape's own list style offers at `level`, so the level falls through to
        the tier below again. Returns whether it offered anything there; a `false` changes
        nothing and does **not** dirty the part.
        """
        ...
    def clear_shape_list_style_default(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> bool:
        """Removes the default properties of the shape's own list style (`a:lstStyle > a:defPPr`).
        Returns whether it had any; a `false` changes nothing and does **not** dirty the part.
        """
        ...
    def clear_shape_list_style(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath) -> bool:
        """Removes the shape's own list style entirely (`a:lstStyle`), so every level falls through
        to the tier below. Returns whether the shape had one; a `false` changes nothing and does
        **not** dirty the part.
        """
        ...
    def set_text_range_properties(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int, range: range, spec: CharacterPropertiesSpec) -> None:
        """Applies `spec` to part of a paragraph — the characters in `range`, counted in **Unicode
        scalars** across the paragraph's whole text.
        """
        ...
    def set_text_range_properties_by_grapheme(self, surface: int | Surface, shape_idx: int | Sequence[int] | ShapePath, para_idx: int, range: range, spec: CharacterPropertiesSpec) -> None:
        """Applies `spec` to part of a paragraph — the characters in `range`, counted in **grapheme
        clusters**: what a reader would call characters, and what a text selection actually
        spans.
        """
        ...

@final
class FormatFamily:
    """The three markup languages ECMA-376 defines, and which this build can edit."""
    Presentation: FormatFamily
    WordProcessing: FormatFamily
    Spreadsheet: FormatFamily
    def __int__(self) -> int: ...

@final
class Format:
    """What a package's main part says the document is."""
    Presentation: Format
    PresentationMacroEnabled: Format
    PresentationTemplate: Format
    PresentationTemplateMacroEnabled: Format
    PresentationSlideshow: Format
    PresentationSlideshowMacroEnabled: Format
    Document: Format
    DocumentMacroEnabled: Format
    DocumentTemplate: Format
    DocumentTemplateMacroEnabled: Format
    Workbook: Format
    WorkbookMacroEnabled: Format
    WorkbookBinary: Format
    WorkbookTemplate: Format
    WorkbookTemplateMacroEnabled: Format
    def __int__(self) -> int: ...
    family: FormatFamily
    """The markup language this format belongs to."""
    is_macro_enabled: bool
    """Whether this format carries macros (`.pptm`, `.docm`, `.xlsm`, and the template forms)."""
    content_type: str
    """The main part's content type, exactly as `[Content_Types].xml` states it."""
    conventional_extension: str
    """The extension this format conventionally carries — `"pptx"`, `"potm"`, `"xlsb"` — with no
    leading dot.
    """
    is_editable: bool
    """Whether `Deck.open` can edit this format. Word and Excel documents are detected before they
    are editable, so a caller can say so precisely instead of reporting a parse failure.
    """

@final
class ShapeBounds:
    """A shape's rectangle on its surface, in English Metric Units."""
    def __init__(self, offset_x_emu: int, offset_y_emu: int, width_emu: int, height_emu: int) -> None:
        """A rectangle stated directly in English Metric Units."""
        ...
    @staticmethod
    def from_inches(x: float, y: float, width: float, height: float) -> "ShapeBounds":
        """A rectangle in inches — the unit slide layouts are usually reasoned about in."""
        ...
    def union(self, other: "ShapeBounds") -> "ShapeBounds":
        """The rectangle that contains both this one and `other`."""
        ...
    @staticmethod
    def from_transform(transform: Transform2D) -> "ShapeBounds" | None:
        """The bounds a transform states, when it states both an offset and an extent."""
        ...
    def to_transform(self) -> Transform2D:
        """This rectangle as a transform."""
        ...
    offset_x_emu: int
    """The left edge, in EMU."""
    offset_y_emu: int
    """The top edge, in EMU."""
    width_emu: int
    """The width, in EMU."""
    height_emu: int
    """The height, in EMU."""

@final
class SlideSize:
    """The slide size: the extent in EMU, plus the paper or screen kind it names."""
    @staticmethod
    def widescreen() -> "SlideSize":
        """13⅓ by 7½ inches — the 16∶9 size PowerPoint has defaulted to since 2013."""
        ...
    @staticmethod
    def standard() -> "SlideSize":
        """10 by 7½ inches — the older 4∶3 size."""
        ...
    @staticmethod
    def from_emu(width_emu: int, height_emu: int) -> "SlideSize":
        """A custom size in English Metric Units."""
        ...
    width_emu: int
    """The width, in EMU."""
    height_emu: int
    """The height, in EMU."""
    kind: SlideSizeKind
    """The paper or screen kind `p:sldSz@type` names."""

@final
class CellMargins:
    """A table cell's four inner margins."""
    def __init__(self, left: Emu | None = ..., right: Emu | None = ..., top: Emu | None = ..., bottom: Emu | None = ...) -> None:
        """Four margins, each optional; an unstated one is inherited."""
        ...
    @staticmethod
    def uniform(margin: Emu) -> "CellMargins":
        """The same margin on all four sides."""
        ...
    left: Emu | None
    """The left margin, when stated."""
    right: Emu | None
    """The right margin, when stated."""
    top: Emu | None
    """The top margin, when stated."""
    bottom: Emu | None
    """The bottom margin, when stated."""

@final
class Position:
    """A point in EMU — the offset half of a transform."""
    def __init__(self, x: Emu, y: Emu) -> None:
        """A point in English Metric Units."""
        ...
    @staticmethod
    def from_emu(x: int, y: int) -> "Position":
        """A point given as two raw EMU values."""
        ...
    x: Emu
    """The horizontal coordinate."""
    y: Emu
    """The vertical coordinate."""

@final
class Size:
    """An extent in EMU — the size half of a transform."""
    def __init__(self, width: Emu, height: Emu) -> None:
        """An extent in English Metric Units."""
        ...
    @staticmethod
    def from_emu(width: int, height: int) -> "Size":
        """An extent given as two raw EMU values."""
        ...
    width: Emu
    """The width."""
    height: Emu
    """The height."""

@final
class Transform2D:
    """A shape's full `a:xfrm`: offset, extent, rotation, flips, and the child space a group maps
    its members through.
    """
    def __init__(self, position: Position | None = ..., size: Size | None = ..., rotation: Angle | None = ..., flip_horizontal: bool | None = ..., flip_vertical: bool | None = ..., child_position: Position | None = ..., child_size: Size | None = ...) -> None:
        """A transform. Every part is optional, and an unstated one is inherited."""
        ...
    position: Position | None
    """The offset, when stated."""
    size: Size | None
    """The extent, when stated."""
    rotation: Angle | None
    """The rotation, when stated."""
    flip_horizontal: bool | None
    """Whether the shape is mirrored horizontally, when stated."""
    flip_vertical: bool | None
    """Whether the shape is mirrored vertically, when stated."""
    child_position: Position | None
    """A group's child-space offset, when stated."""
    child_size: Size | None
    """A group's child-space extent, when stated."""
    is_empty: bool
    """Whether this transform states nothing at all."""
    child_scale: tuple[float, float] | None
    """The scale a group applies to its members, when it states both a child and an outer extent."""
    def child_to_parent(self, point: Position) -> Position | None:
        """A point in a group's child space, mapped to the surface."""
        ...
    def parent_to_child(self, point: Position) -> Position | None:
        """A point on the surface, mapped into a group's child space."""
        ...

@final
class Point:
    """A point in a custom geometry path, whose coordinates may be literal or guide-relative."""
    def __init__(self, x: AdjustCoordinate, y: AdjustCoordinate) -> None:
        """A point whose coordinates may each be literal or guide-relative."""
        ...
    @staticmethod
    def from_emu(x: int, y: int) -> "Point":
        """A point at two literal EMU coordinates."""
        ...
    x: AdjustCoordinate
    """The horizontal coordinate."""
    y: AdjustCoordinate
    """The vertical coordinate."""

@final
class AdjustCoordinate:
    """A coordinate in a custom geometry: an absolute length, or the name of a guide."""
    @staticmethod
    def emu(value: Emu) -> "AdjustCoordinate":
        """A literal length."""
        ...
    @staticmethod
    def guide(name: str) -> "AdjustCoordinate":
        """The value of a named guide."""
        ...
    @staticmethod
    def from_wire(value: str) -> "AdjustCoordinate":
        """A coordinate parsed from the wire form — a number, or a guide name."""
        ...
    value: Emu | None
    """The literal length, when this is one."""
    guide_name: str | None
    """The guide's name, when this names one."""
    def to_wire(self) -> str:
        """The wire form, exactly as it is written."""
        ...

@final
class AdjustAngle:
    """An angle in a custom geometry: an absolute angle, or the name of a guide."""
    @staticmethod
    def angle(value: Angle) -> "AdjustAngle":
        """A literal angle."""
        ...
    @staticmethod
    def guide(name: str) -> "AdjustAngle":
        """The value of a named guide."""
        ...
    @staticmethod
    def from_wire(value: str) -> "AdjustAngle":
        """An angle parsed from the wire form — a number of sixtieths of a degree, or a guide name."""
        ...
    value: Angle | None
    """The literal angle, when this is one."""
    guide_name: str | None
    """The guide's name, when this names one."""
    def to_wire(self) -> str:
        """The wire form, exactly as it is written."""
        ...

@final
class GuideSpec:
    """One guide: a name, and the formula that computes it."""
    def __init__(self, name: str, formula: str) -> None:
        """A guide: the name other formulas refer to it by, and the formula that computes it."""
        ...
    name: str
    """The guide's name."""
    formula: str
    """The formula, in the seventeen-operator prefix language `a:gd@fmla` uses."""

@final
class GuideContext:
    """The width and height a guide formula's `w` and `h` variables stand for."""
    @staticmethod
    def from_extents(width: Emu, height: Emu) -> "GuideContext":
        """The extent a formula's `w` and `h` stand for."""
        ...
    @staticmethod
    def from_size(size: Size) -> "GuideContext":
        """The same, from a `Size`."""
        ...
    width: Emu
    """The width `w` stands for."""
    height: Emu
    """The height `h` stands for."""
    def variable(self, name: str) -> float | None:
        """The value of one built-in variable — `w`, `h`, `l`, `t`, `r`, `b`, `hc`, `vc`, `ss`,
        `ls`, `ssd2`… — or `None` if that is not a variable name.
        """
        ...

@final
class Rectangle:
    """A rectangle in a custom geometry, whose edges may be guide-relative."""
    def __init__(self, left: AdjustCoordinate, top: AdjustCoordinate, right: AdjustCoordinate, bottom: AdjustCoordinate) -> None:
        """A rectangle whose four edges may each be literal or guide-relative."""
        ...
    left: AdjustCoordinate
    """The left edge."""
    top: AdjustCoordinate
    """The top edge."""
    right: AdjustCoordinate
    """The right edge."""
    bottom: AdjustCoordinate
    """The bottom edge."""

@final
class DrawCommand:
    """One command in a custom geometry path."""
    @staticmethod
    def close() -> "DrawCommand":
        """Close the current subpath."""
        ...
    @staticmethod
    def move_to(point: Point) -> "DrawCommand":
        """Start a new subpath at a point."""
        ...
    @staticmethod
    def line_to(point: Point) -> "DrawCommand":
        """Draw a straight segment to a point."""
        ...
    @staticmethod
    def arc_to(width_radius: AdjustCoordinate, height_radius: AdjustCoordinate, start_angle: AdjustAngle, swing_angle: AdjustAngle) -> "DrawCommand":
        """Draw an elliptical arc."""
        ...
    @staticmethod
    def quad_bezier_to(control: Point, end: Point) -> "DrawCommand":
        """Draw a quadratic Bézier through one control point."""
        ...
    @staticmethod
    def cubic_bezier_to(first: Point, second: Point, end: Point) -> "DrawCommand":
        """Draw a cubic Bézier through two control points."""
        ...
    kind: str
    """Which command this is: `"close"`, `"move_to"`, `"line_to"`, `"arc_to"`, `"quad_bezier_to"`
    or `"cubic_bezier_to"`.
    """
    points: list[Point]
    """The points this command names, in order; empty for `close` and for `arc_to`."""
    radii: tuple[AdjustCoordinate, AdjustCoordinate] | None
    """The arc's two radii, when this is an arc."""
    angles: tuple[AdjustAngle, AdjustAngle] | None
    """The arc's start and swing angles, when this is an arc."""

@final
class Path2DSpec:
    """One path of a custom geometry: its own coordinate space, and the commands that draw it."""
    def __init__(self, commands: list[DrawCommand], width: Emu | None = ..., height: Emu | None = ..., fill: PathFillMode | None = ..., stroke: bool | None = ..., extrusion_ok: bool | None = ...) -> None:
        """One path of a custom geometry."""
        ...
    width: Emu | None
    """The path's own coordinate width, when stated."""
    height: Emu | None
    """The path's own coordinate height, when stated."""
    fill: PathFillMode | None
    """How the path is filled, when stated."""
    stroke: bool | None
    """Whether the path is stroked, when stated."""
    extrusion_ok: bool | None
    """Whether the path may be extruded in 3-D, when stated."""
    commands: list[DrawCommand]
    """The commands that draw the path, in order."""

@final
class AdjustHandle:
    """A draggable handle on a custom geometry, and the guide it drives."""
    @staticmethod
    def xy(position: Point, guide_ref_x: str | None = ..., min_x: AdjustCoordinate | None = ..., max_x: AdjustCoordinate | None = ..., guide_ref_y: str | None = ..., min_y: AdjustCoordinate | None = ..., max_y: AdjustCoordinate | None = ...) -> "AdjustHandle":
        """A handle that moves in two dimensions, driving one guide per axis."""
        ...
    @staticmethod
    def polar(position: Point, guide_ref_radius: str | None = ..., min_radius: AdjustCoordinate | None = ..., max_radius: AdjustCoordinate | None = ..., guide_ref_angle: str | None = ..., min_angle: AdjustAngle | None = ..., max_angle: AdjustAngle | None = ...) -> "AdjustHandle":
        """A handle that moves in polar coordinates, driving a radius guide and an angle guide."""
        ...
    kind: str
    """Which kind this is: `"xy"` or `"polar"`."""
    position: Point
    """Where the handle sits."""
    first_guide: str | None
    """The guide the first axis drives — `gdRefX` or `gdRefR` — when the handle names one."""
    second_guide: str | None
    """The guide the second axis drives — `gdRefY` or `gdRefAng` — when the handle names one."""
    first_limits: tuple[AdjustCoordinate | None, AdjustCoordinate | None]
    """The first axis's limits, when stated."""
    second_limits: tuple[AdjustCoordinate | None, AdjustCoordinate | None]
    """The second axis's limits, when stated. An `xy` handle's are coordinates; a `polar` handle's
    are angles, reported through [`second_angle_limits`](Self::second_angle_limits).
    """
    second_angle_limits: tuple[AdjustAngle | None, AdjustAngle | None]
    """A polar handle's angular limits, when stated."""

@final
class ConnectionSite:
    """A point a connector can attach to, and the direction a connector leaves it in."""
    def __init__(self, angle: AdjustAngle, position: Point) -> None:
        """A point a connector can attach to, and the direction it leaves in."""
        ...
    angle: AdjustAngle
    """The direction a connector leaves the site in."""
    position: Point
    """Where the site is."""

@final
class CustomGeometrySpec:
    """A whole `a:custGeom`: guides, handles, connection sites, text rectangle and paths."""
    def __init__(self, paths: list[Path2DSpec] = ..., adjust_values: list[GuideSpec] = ..., guides: list[GuideSpec] = ..., adjust_handles: list[AdjustHandle] = ..., connection_sites: list[ConnectionSite] = ..., text_rectangle: Rectangle | None = ...) -> None:
        """A custom geometry. Only `paths` is usually needed; the rest describe the guides and
        handles PowerPoint's own editor manipulates.
        """
        ...
    adjust_values: list[GuideSpec]
    """The adjust values (`a:avLst`), each a named guide with a default."""
    guides: list[GuideSpec]
    """The computed guides (`a:gdLst`)."""
    adjust_handles: list[AdjustHandle]
    """The adjust handles (`a:ahLst`)."""
    connection_sites: list[ConnectionSite]
    """The connection sites (`a:cxnLst`)."""
    text_rectangle: Rectangle | None
    """The text rectangle (`a:rect`), when stated."""
    paths: list[Path2DSpec]
    """The paths (`a:pathLst`), in order."""
    def guide_values(self, context: GuideContext) -> dict[str, float]:
        """Every guide's value at the given size."""
        ...
    def resolve(self, context: GuideContext) -> ResolvedCustomGeometry:
        """This geometry with every formula evaluated at the given size — what a renderer would
        draw.
        """
        ...

@final
class ShapeGeometry:
    """A preset shape with its named adjustments, or an unmodelled preset carrying only its name."""
    @staticmethod
    def of(preset: PresetShapeType, adjustments: dict[str, Fraction | Angle] | None = ...) -> "ShapeGeometry":
        """The geometry of one preset shape, with values for the adjustments it carries."""
        ...
    preset: PresetShapeType
    """The preset this geometry names."""
    adjustments: dict[str, Fraction | Angle]
    """The adjustments this geometry states, by name."""
    @staticmethod
    def adjustment_names(preset: PresetShapeType) -> list[str]:
        """What a preset's adjustments are called — the keys [`of`](ShapeGeometry::of) expects, in
        the order the specification lists them.
        """
        ...

@final
class Geometry:
    """What draws a shape's outline: a preset, a custom path, or whatever it inherits."""
    @staticmethod
    def preset(geometry: ShapeGeometry) -> "Geometry":
        """One of the presets, with its adjustments."""
        ...
    @staticmethod
    def custom(geometry: CustomGeometrySpec) -> "Geometry":
        """A path the document draws itself."""
        ...
    @staticmethod
    def inherited() -> "Geometry":
        """Whatever the shape's placeholder chain says — the shape states no geometry of its own."""
        ...
    kind: str
    """Which kind this is: `"preset"`, `"custom"` or `"inherited"`."""
    preset_geometry: ShapeGeometry | None
    """The preset geometry, when this is a preset."""
    custom_geometry: CustomGeometrySpec | None
    """The custom geometry, when this is one."""

@final
class BoundedAdjustment:
    """One adjustment of a preset shape, with the range it is allowed to move in."""
    spec: AdjustmentSpec
    """What the specification says about this adjustment."""
    value: float
    """The value the shape states, or the specification's default when it states none."""
    is_overridden: bool
    """Whether the shape states a value of its own."""
    minimum: float
    """The lower end of the range, resolved at the shape's own size."""
    maximum: float
    """The upper end of the range, resolved at the shape's own size."""
    pinned_value: float
    """The value clamped into the range — what a consumer would actually draw."""

@final
class AdjustmentSpec:
    """What the specification says about one adjustment of one preset shape."""
    wire_name: str
    """The adjustment's name on the wire — `"adj"`, `"adj1"`, `"adj2"`…"""
    axis: AdjustmentAxis
    """Which axis the adjustment moves along."""
    default: int
    """The value the shape uses when it states none."""
    minimum: AdjustmentBound
    """The lower end of the adjustment's range."""
    maximum: AdjustmentBound
    """The upper end of the adjustment's range."""

@final
class AdjustmentBound:
    """One end of an adjustment's range: a literal value, or the name of a guide."""
    literal: int | None
    """A literal bound, in the adjustment's own native units."""
    guide: str | None
    """The guide that computes the bound, when it is not a literal."""

@final
class ResolvedPoint:
    """A point with every coordinate resolved to EMU."""
    x: Emu
    """The horizontal coordinate."""
    y: Emu
    """The vertical coordinate."""

@final
class ResolvedRectangle:
    """A rectangle with every edge resolved to EMU."""
    left: Emu
    """The left edge."""
    top: Emu
    """The top edge."""
    right: Emu
    """The right edge."""
    bottom: Emu
    """The bottom edge."""

@final
class ResolvedDrawCommand:
    """A path command with every coordinate resolved."""
    kind: str
    """Which command this is, in the same vocabulary [`DrawCommand.kind`](DrawCommand::kind) uses."""
    points: list[ResolvedPoint]
    """The points this command names, in order."""
    radii: tuple[Emu, Emu] | None
    """The arc's two radii, when this is an arc."""
    angles: tuple[Angle, Angle] | None
    """The arc's start and swing angles, when this is an arc."""

@final
class ResolvedPath:
    """A path with every command resolved."""
    width: Emu | None
    """The path's own coordinate width, when stated."""
    height: Emu | None
    """The path's own coordinate height, when stated."""
    fill: PathFillMode | None
    """How the path is filled, when stated."""
    stroke: bool | None
    """Whether the path is stroked, when stated."""
    extrusion_ok: bool | None
    """Whether the path may be extruded in 3-D, when stated."""
    commands: list[ResolvedDrawCommand]
    """The resolved commands, in order."""

@final
class ResolvedConnectionSite:
    """A connection site with its point and angle resolved."""
    angle: Angle
    """The direction a connector leaves the site in."""
    position: ResolvedPoint
    """Where the site is."""

@final
class ResolvedAdjustHandle:
    """An adjust handle with its point and limits resolved."""
    kind: str
    """Which kind this is: `"xy"` or `"polar"`."""
    position: ResolvedPoint
    """Where the handle sits."""
    first_guide: str | None
    """The guide the first axis drives, when the handle names one."""
    second_guide: str | None
    """The guide the second axis drives, when the handle names one."""
    first_limits: tuple[Emu | None, Emu | None]
    """The first axis's resolved limits, when stated."""
    second_limits: tuple[Emu | None, Emu | None]
    """An `xy` handle's second-axis limits, when stated."""
    second_angle_limits: tuple[Angle | None, Angle | None]
    """A `polar` handle's angular limits, when stated."""

@final
class ResolvedCustomGeometry:
    """A custom geometry with every formula evaluated — what a renderer would draw."""
    paths: list[ResolvedPath]
    """The resolved paths, in order."""
    text_rectangle: ResolvedRectangle | None
    """The resolved text rectangle, when the geometry states one."""
    connection_sites: list[ResolvedConnectionSite]
    """The resolved connection sites."""
    adjust_handles: list[ResolvedAdjustHandle]
    """The resolved adjust handles."""

@final
class Emu:
    """A length in English Metric Units — 914 400 to the inch, 12 700 to the point. The unit every
    position, size, margin, offset and radius in a document is stated in.
    """
    @staticmethod
    def from_emu(emu: int) -> "Emu":
        """A length stated directly in English Metric Units."""
        ...
    @staticmethod
    def from_points(points: float) -> "Emu":
        """A length in points — 12 700 EMU each."""
        ...
    @staticmethod
    def from_inches(inches: float) -> "Emu":
        """A length in inches — 914 400 EMU each."""
        ...
    @staticmethod
    def from_centimetres(centimetres: float) -> "Emu":
        """A length in centimetres — 360 000 EMU each."""
        ...
    emu: int
    """The value in English Metric Units."""
    points: float
    """The value in points."""
    inches: float
    """The value in inches."""
    centimetres: float
    """The value in centimetres."""

@final
class Angle:
    """An angle. OOXML stores sixtieths of a degree; this class speaks degrees and radians and
    converts.
    """
    @staticmethod
    def from_degrees(degrees: float) -> "Angle":
        """An angle in degrees, measured clockwise as OOXML measures it."""
        ...
    @staticmethod
    def from_radians(radians: float) -> "Angle":
        """An angle in radians."""
        ...
    degrees: float
    """The angle in degrees."""
    radians: float
    """The angle in radians."""

@final
class Fraction:
    """A proportion of one: `0.5` is fifty per cent. OOXML stores thousandths of a per cent."""
    @staticmethod
    def of(ratio: float) -> "Fraction":
        """A proportion of one: `Fraction.of(0.5)` is fifty per cent."""
        ...
    @staticmethod
    def percent(percent: float) -> "Fraction":
        """A proportion given as a percentage: `Fraction.percent(50)` is the same as
        `Fraction.of(0.5)`.
        """
        ...
    ratio: float
    """The proportion, as a fraction of one."""
    percentage: float
    """The proportion, as a percentage."""

@final
class FontSize:
    """A font size in points. OOXML stores hundredths of a point, which is the resolution a size
    actually has — `10.5` is exact, `10.567` is not.
    """
    @staticmethod
    def from_points(points: float) -> "FontSize":
        """A size in points."""
        ...
    @staticmethod
    def from_hundredths_of_a_point(hundredths: int) -> "FontSize":
        """A size in the hundredths of a point the markup stores."""
        ...
    points: float
    """The size in points."""
    hundredths_of_a_point: int
    """The size in hundredths of a point, exactly as it is written."""

@final
class TextPoint:
    """A text measure in points — letter spacing, kerning, paragraph spacing. Distinct from
    [`FontSize`] because the two are not interchangeable in the markup even though both are
    hundredths of a point.
    """
    @staticmethod
    def from_points(points: float) -> "TextPoint":
        """A measure in points."""
        ...
    @staticmethod
    def from_hundredths_of_a_point(hundredths: int) -> "TextPoint":
        """A measure in the hundredths of a point the markup stores."""
        ...
    points: float
    """The measure in points."""
    hundredths_of_a_point: int
    """The measure in hundredths of a point, exactly as it is written."""

@final
class LineWidth:
    """A line width. EMU on the wire, points in practice."""
    @staticmethod
    def from_points(points: float) -> "LineWidth":
        """A width in points."""
        ...
    @staticmethod
    def from_emu(emu: int) -> "LineWidth":
        """A width stated directly in English Metric Units."""
        ...
    points: float
    """The width in points."""
    emu: int
    """The width in English Metric Units."""

@final
class IndentLevel:
    """A list indent level, `0` through `8` — the nine levels a `p:txBody` list style defines."""
    def __init__(self, level: int) -> None:
        """The list level at this depth, `0` through `8`."""
        ...
    value: int
    """The level, `0` through `8`."""

@final
class ColorSpec:
    """A colour, as the document states it: six hex digits, a theme slot, or one of the other
    colour elements DrawingML defines.
    """
    @staticmethod
    def srgb(hex: str) -> "ColorSpec":
        """A literal colour, six hexadecimal digits with no leading `#`:
        `ColorSpec.srgb("1F3864")`.
        """
        ...
    @staticmethod
    def scheme(color: SchemeColor) -> "ColorSpec":
        """A theme colour, resolved through the surface's colour map at render time."""
        ...
    @staticmethod
    def other(kind: ColorKind, value: str | None = ...) -> "ColorSpec":
        """One of the other colour elements — `hslClr`, `scrgbClr`, `sysClr`, `prstClr` — kept
        exactly as written so it round-trips, and reported here so a caller knows what it is
        looking at.
        """
        ...
    kind: ColorKind
    """Which kind of colour element this is."""
    srgb_value: str | None
    """The six hex digits, when this is a literal colour."""
    scheme_color: SchemeColor | None
    """The theme slot, when this is a theme colour."""
    value: str | None
    """The raw value of one of the other colour elements, when the document stated one."""

@final
class GradientStopSpec:
    """One stop on a gradient: where it sits, and what colour it is there."""
    def __init__(self, position: Fraction, color: ColorSpec) -> None:
        """A stop at `position` along the gradient, painted `color`."""
        ...
    position: Fraction
    """Where the stop sits, as a proportion of the gradient's length."""
    color: ColorSpec
    """The colour at this stop."""

@final
class FillSpec:
    """How a shape, cell, run or chart element is filled."""
    @staticmethod
    def none() -> "FillSpec":
        """No fill at all — `a:noFill`, which is not the same as stating nothing."""
        ...
    @staticmethod
    def solid(color: ColorSpec) -> "FillSpec":
        """One flat colour."""
        ...
    @staticmethod
    def gradient(stops: list[GradientStopSpec], angle: Angle | None = ...) -> "FillSpec":
        """A linear gradient through the given stops, at the given angle."""
        ...
    @staticmethod
    def picture(rel_id: str, mode: PictureFillMode) -> "FillSpec":
        """An image, named by the relationship id `Deck.add_image` hands back."""
        ...
    @staticmethod
    def pattern(preset: PatternType | None = ..., foreground: ColorSpec | None = ..., background: ColorSpec | None = ...) -> "FillSpec":
        """One of the fifty-four hatch patterns, in a foreground and background colour."""
        ...
    @staticmethod
    def group() -> "FillSpec":
        """Inherit the enclosing group's fill — `a:grpFill`."""
        ...
    kind: str
    """Which kind of fill this is: `"none"`, `"solid"`, `"gradient"`, `"picture"`, `"pattern"` or
    `"group"`.
    """
    color: ColorSpec | None
    """The colour, when this is a solid fill."""
    stops: list[GradientStopSpec]
    """The stops, when this is a gradient; an empty list otherwise."""
    angle: Angle | None
    """The gradient's angle, when it states one."""
    rel_id: str | None
    """The image relationship id, when this is a picture fill."""
    picture_mode: PictureFillMode | None
    """How the image is laid into the shape, when this is a picture fill."""
    pattern_preset: PatternType | None
    """The hatch pattern, when this is a pattern fill and it names one."""
    foreground: ColorSpec | None
    """The pattern's foreground colour, when it states one."""
    background: ColorSpec | None
    """The pattern's background colour, when it states one."""

@final
class LineDash:
    """The dash pattern of a line: one of the eleven presets, or a custom pattern the document
    spells out (which this build preserves but does not model).
    """
    @staticmethod
    def preset(dash: PresetLineDash) -> "LineDash":
        """One of the eleven dash patterns the specification names."""
        ...
    @staticmethod
    def custom() -> "LineDash":
        """A custom dash pattern. The document's own `a:custDash` stops are preserved on write;
        this build does not model the individual dash and space lengths.
        """
        ...
    preset_dash: PresetLineDash | None
    """The named pattern, when this is a preset."""
    is_custom: bool
    """Whether the document spelled the pattern out rather than naming one."""

@final
class LineJoin:
    """How two segments of a line meet."""
    @staticmethod
    def round() -> "LineJoin":
        """A rounded corner."""
        ...
    @staticmethod
    def bevel() -> "LineJoin":
        """A flattened corner."""
        ...
    @staticmethod
    def miter(limit: Fraction | None = ...) -> "LineJoin":
        """A pointed corner, optionally limited so that a very sharp angle does not run away."""
        ...
    kind: str
    """Which join this is: `"round"`, `"bevel"` or `"miter"`."""
    miter_limit: Fraction | None
    """The mitre limit, when this is a mitre join that states one."""

@final
class LineEnd:
    """The head or tail decoration of a line — an arrowhead, and how big it is."""
    def __init__(self, kind: LineEndType | None = ..., width: LineEndWidth | None = ..., length: LineEndLength | None = ...) -> None:
        """An end decoration: which arrowhead, how wide, how long. Every part is optional, and an
        unstated one is inherited.
        """
        ...
    kind: LineEndType | None
    """Which arrowhead, when the line states one."""
    width: LineEndWidth | None
    """How wide the arrowhead is, when the line states it."""
    length: LineEndLength | None
    """How long the arrowhead is, when the line states it."""

@final
class LineSpec:
    """An outline: width, cap, dash, join, ends, and the fill that paints it."""
    def __init__(self) -> None:
        """An outline that states nothing. Add to it with the `with_…` methods."""
        ...
    @staticmethod
    def solid(width: LineWidth, color: ColorSpec) -> "LineSpec":
        """The common case: a solid line of one width and one colour."""
        ...
    def with_width(self, width: LineWidth) -> "LineSpec":
        """This outline with the given width."""
        ...
    def with_cap(self, cap: LineCap) -> "LineSpec":
        """This outline with the given end cap."""
        ...
    def with_compound(self, compound: CompoundLine) -> "LineSpec":
        """This outline drawn as a compound (double, triple, thick-thin) line."""
        ...
    def with_pen_alignment(self, alignment: PenAlignment) -> "LineSpec":
        """This outline centred on, or inset from, the shape's edge."""
        ...
    def with_fill(self, fill: FillSpec) -> "LineSpec":
        """This outline painted with the given fill — which is how a line gets a gradient."""
        ...
    def with_dash(self, dash: LineDash) -> "LineSpec":
        """This outline with the given dash pattern."""
        ...
    def with_join(self, join: LineJoin) -> "LineSpec":
        """This outline with the given corner treatment."""
        ...
    def with_head_end(self, end: LineEnd) -> "LineSpec":
        """This outline with the given decoration at its start."""
        ...
    def with_tail_end(self, end: LineEnd) -> "LineSpec":
        """This outline with the given decoration at its end."""
        ...
    width: LineWidth | None
    """The width, when stated."""
    cap: LineCap | None
    """The end cap, when stated."""
    compound: CompoundLine | None
    """The compound style, when stated."""
    pen_alignment: PenAlignment | None
    """The pen alignment, when stated."""
    fill: FillSpec | None
    """The fill that paints the line, when stated."""
    dash: LineDash | None
    """The dash pattern, when stated."""
    join: LineJoin | None
    """The corner treatment, when stated."""
    head_end: LineEnd | None
    """The start decoration, when stated."""
    tail_end: LineEnd | None
    """The end decoration, when stated."""

@final
class BlurEffect:
    """A Gaussian blur over whatever is behind it."""
    def __init__(self, radius: Emu | None = ..., grow: bool | None = ...) -> None:
        """A blur of the given radius. `grow` says whether the blurred edge may extend past the
        shape's bounds.
        """
        ...
    def with_radius(self, radius: Emu) -> "BlurEffect":
        """This blur with the given radius."""
        ...
    def with_grow(self, grow: bool) -> "BlurEffect":
        """This blur, growing past the shape's bounds or not."""
        ...
    radius: Emu | None
    """The radius, when stated."""
    grow: bool | None
    """Whether the blur may grow past the shape's bounds, when stated."""

@final
class FillOverlayEffect:
    """A fill painted over the shape in a blend mode."""
    def __init__(self, fill: FillSpec, blend: BlendMode) -> None:
        """A fill painted over the shape in the given blend mode."""
        ...
    fill: FillSpec
    """The overlaid fill."""
    blend: BlendMode
    """How it blends with what is beneath."""

@final
class GlowEffect:
    """A coloured halo outside the shape's edge."""
    def __init__(self, color: ColorSpec, radius: Emu | None = ...) -> None:
        """A halo in the given colour, optionally of a given radius."""
        ...
    def with_radius(self, radius: Emu) -> "GlowEffect":
        """This glow with the given radius."""
        ...
    color: ColorSpec
    """The glow's colour."""
    radius: Emu | None
    """The radius, when stated."""

@final
class InnerShadowEffect:
    """A shadow cast inside the shape's edge."""
    def __init__(self, color: ColorSpec, blur_radius: Emu | None = ..., distance: Emu | None = ..., direction: Angle | None = ...) -> None:
        """A shadow inside the shape's edge."""
        ...
    def with_blur_radius(self, blur_radius: Emu) -> "InnerShadowEffect":
        """This shadow with the given blur radius."""
        ...
    def with_distance(self, distance: Emu) -> "InnerShadowEffect":
        """This shadow at the given distance from the shape."""
        ...
    def with_direction(self, direction: Angle) -> "InnerShadowEffect":
        """This shadow cast in the given direction."""
        ...
    color: ColorSpec
    """The shadow's colour."""
    blur_radius: Emu | None
    """The blur radius, when stated."""
    distance: Emu | None
    """The distance from the shape, when stated."""
    direction: Angle | None
    """The direction the shadow is cast in, when stated."""

@final
class OuterShadowEffect:
    """A shadow cast outside the shape's edge, with its own scale, skew and alignment."""
    def __init__(self, color: ColorSpec, blur_radius: Emu | None = ..., distance: Emu | None = ..., direction: Angle | None = ..., scale_x: Fraction | None = ..., scale_y: Fraction | None = ..., skew_x: Angle | None = ..., skew_y: Angle | None = ..., alignment: RectangleAlignment | None = ..., rotate_with_shape: bool | None = ...) -> None:
        """A shadow outside the shape's edge, with its own scale, skew and alignment."""
        ...
    def with_blur_radius(self, blur_radius: Emu) -> "OuterShadowEffect":
        """This shadow with the given blur radius."""
        ...
    def with_distance(self, distance: Emu) -> "OuterShadowEffect":
        """This shadow at the given distance from the shape."""
        ...
    def with_direction(self, direction: Angle) -> "OuterShadowEffect":
        """This shadow cast in the given direction."""
        ...
    def with_scale_x(self, scale_x: Fraction) -> "OuterShadowEffect":
        """This shadow scaled horizontally."""
        ...
    def with_scale_y(self, scale_y: Fraction) -> "OuterShadowEffect":
        """This shadow scaled vertically."""
        ...
    def with_skew_x(self, skew_x: Angle) -> "OuterShadowEffect":
        """This shadow skewed horizontally."""
        ...
    def with_skew_y(self, skew_y: Angle) -> "OuterShadowEffect":
        """This shadow skewed vertically."""
        ...
    def with_alignment(self, alignment: RectangleAlignment) -> "OuterShadowEffect":
        """This shadow anchored to the given corner or edge of the shape."""
        ...
    def with_rotate_with_shape(self, rotate_with_shape: bool) -> "OuterShadowEffect":
        """This shadow rotating with the shape, or staying put."""
        ...
    color: ColorSpec
    """The shadow's colour."""
    blur_radius: Emu | None
    """The blur radius, when stated."""
    distance: Emu | None
    """The distance from the shape, when stated."""
    direction: Angle | None
    """The direction the shadow is cast in, when stated."""
    scale_x: Fraction | None
    """The horizontal scale, when stated."""
    scale_y: Fraction | None
    """The vertical scale, when stated."""
    skew_x: Angle | None
    """The horizontal skew, when stated."""
    skew_y: Angle | None
    """The vertical skew, when stated."""
    alignment: RectangleAlignment | None
    """Where the shadow is anchored, when stated."""
    rotate_with_shape: bool | None
    """Whether the shadow rotates with the shape, when stated."""

@final
class PresetShadowEffect:
    """One of the twenty shadows the specification names, in a colour of your choosing."""
    def __init__(self, preset: PresetShadow, color: ColorSpec, distance: Emu | None = ..., direction: Angle | None = ...) -> None:
        """One of the twenty named shadows, in the given colour."""
        ...
    def with_distance(self, distance: Emu) -> "PresetShadowEffect":
        """This shadow at the given distance from the shape."""
        ...
    def with_direction(self, direction: Angle) -> "PresetShadowEffect":
        """This shadow cast in the given direction."""
        ...
    preset: PresetShadow
    """Which of the twenty shadows this is."""
    color: ColorSpec
    """The shadow's colour."""
    distance: Emu | None
    """The distance from the shape, when stated."""
    direction: Angle | None
    """The direction the shadow is cast in, when stated."""

@final
class ReflectionEffect:
    """A mirrored, fading copy of the shape below it."""
    def __init__(self, blur_radius: Emu | None = ..., start_alpha: Fraction | None = ..., start_position: Fraction | None = ..., end_alpha: Fraction | None = ..., end_position: Fraction | None = ..., distance: Emu | None = ..., direction: Angle | None = ..., fade_direction: Angle | None = ..., scale_x: Fraction | None = ..., scale_y: Fraction | None = ..., skew_x: Angle | None = ..., skew_y: Angle | None = ..., alignment: RectangleAlignment | None = ..., rotate_with_shape: bool | None = ...) -> None:
        """A mirrored, fading copy of the shape. Every part is optional."""
        ...
    blur_radius: Emu | None
    """The blur radius, when stated."""
    start_alpha: Fraction | None
    """The opacity where the reflection starts, when stated."""
    start_position: Fraction | None
    """Where the reflection starts, when stated."""
    end_alpha: Fraction | None
    """The opacity where the reflection ends, when stated."""
    end_position: Fraction | None
    """Where the reflection ends, when stated."""
    distance: Emu | None
    """The distance from the shape, when stated."""
    direction: Angle | None
    """The direction the reflection is offset in, when stated."""
    fade_direction: Angle | None
    """The direction the reflection fades in, when stated."""
    scale_x: Fraction | None
    """The horizontal scale, when stated."""
    scale_y: Fraction | None
    """The vertical scale, when stated."""
    skew_x: Angle | None
    """The horizontal skew, when stated."""
    skew_y: Angle | None
    """The vertical skew, when stated."""
    alignment: RectangleAlignment | None
    """Where the reflection is anchored, when stated."""
    rotate_with_shape: bool | None
    """Whether the reflection rotates with the shape, when stated."""

@final
class SoftEdgeEffect:
    """A feathered edge that fades the shape out over a radius."""
    def __init__(self, radius: Emu) -> None:
        """A feathered edge fading out over the given radius."""
        ...
    radius: Emu
    """The radius the edge fades over."""

@final
class EffectListSpec:
    """The eight effects a shape can carry, in the order the markup writes them."""
    def __init__(self, blur: BlurEffect | None = ..., fill_overlay: FillOverlayEffect | None = ..., glow: GlowEffect | None = ..., inner_shadow: InnerShadowEffect | None = ..., outer_shadow: OuterShadowEffect | None = ..., preset_shadow: PresetShadowEffect | None = ..., reflection: ReflectionEffect | None = ..., soft_edge: SoftEdgeEffect | None = ...) -> None:
        """An effect list that states nothing. Add to it with the `with_…` methods."""
        ...
    def with_blur(self, blur: BlurEffect) -> "EffectListSpec":
        """This list with the given blur."""
        ...
    def with_fill_overlay(self, fill_overlay: FillOverlayEffect) -> "EffectListSpec":
        """This list with the given fill overlay."""
        ...
    def with_glow(self, glow: GlowEffect) -> "EffectListSpec":
        """This list with the given glow."""
        ...
    def with_inner_shadow(self, inner_shadow: InnerShadowEffect) -> "EffectListSpec":
        """This list with the given inner shadow."""
        ...
    def with_outer_shadow(self, outer_shadow: OuterShadowEffect) -> "EffectListSpec":
        """This list with the given outer shadow."""
        ...
    def with_preset_shadow(self, preset_shadow: PresetShadowEffect) -> "EffectListSpec":
        """This list with the given preset shadow."""
        ...
    def with_reflection(self, reflection: ReflectionEffect) -> "EffectListSpec":
        """This list with the given reflection."""
        ...
    def with_soft_edge(self, soft_edge: SoftEdgeEffect) -> "EffectListSpec":
        """This list with the given soft edge."""
        ...
    blur: BlurEffect | None
    """The blur, when stated."""
    fill_overlay: FillOverlayEffect | None
    """The fill overlay, when stated."""
    glow: GlowEffect | None
    """The glow, when stated."""
    inner_shadow: InnerShadowEffect | None
    """The inner shadow, when stated."""
    outer_shadow: OuterShadowEffect | None
    """The outer shadow, when stated."""
    preset_shadow: PresetShadowEffect | None
    """The preset shadow, when stated."""
    reflection: ReflectionEffect | None
    """The reflection, when stated."""
    soft_edge: SoftEdgeEffect | None
    """The soft edge, when stated."""

@final
class ColorMap:
    """A theme's twelve-slot colour mapping: which scheme colour each named slot resolves to."""
    @staticmethod
    def identity() -> "ColorMap":
        """The mapping that sends every slot to itself — what a theme means when it states no
        `clrMap`.
        """
        ...
    def resolve(self, color: SchemeColor) -> ColorSchemeSlot | None:
        """Which scheme colour a named slot resolves to, or `None` for a colour that is not mapped."""
        ...
    background1: ColorSchemeSlot
    """The slot `bg1` maps to."""
    text1: ColorSchemeSlot
    """The slot `tx1` maps to."""
    background2: ColorSchemeSlot
    """The slot `bg2` maps to."""
    text2: ColorSchemeSlot
    """The slot `tx2` maps to."""
    accents: list[ColorSchemeSlot]
    """The six accent slots, in order."""
    hyperlink: ColorSchemeSlot
    """The slot `hlink` maps to."""
    followed_hyperlink: ColorSchemeSlot
    """The slot `folHlink` maps to."""

@final
class ResolvedColor:
    """A colour resolved all the way to channels — what a renderer would actually paint."""
    red: int
    """The red channel, `0`–`255`."""
    green: int
    """The green channel, `0`–`255`."""
    blue: int
    """The blue channel, `0`–`255`."""
    alpha: float
    """The alpha channel as a proportion of one, `1.0` being fully opaque."""
    def to_hex(self) -> str:
        """The colour as six hexadecimal digits."""
        ...

@final
class Cells:
    """Which cells of a table a call is about."""
    @staticmethod
    def one(row: int, column: int) -> "Cells":
        """One cell."""
        ...
    @staticmethod
    def row(row: int) -> "Cells":
        """Every cell of one row."""
        ...
    @staticmethod
    def column(column: int) -> "Cells":
        """Every cell of one column."""
        ...
    @staticmethod
    def rectangle(rows: range, columns: range) -> "Cells":
        """A rectangular block, given as two `range`s: `Cells.rectangle(range(0, 2), range(1, 4))`."""
        ...
    @staticmethod
    def all() -> "Cells":
        """Every cell of the table."""
        ...
    kind: str
    """Which kind of selection this is: `"one"`, `"row"`, `"column"`, `"rectangle"` or `"all"`."""
    rows: tuple[int, int] | None
    """The rows this selection covers, as a `range`, when it names any."""
    columns: tuple[int, int] | None
    """The columns this selection covers, as a `range`, when it names any."""

@final
class CellFormat:
    """A change to apply to a selection of cells: fill, borders, margins, anchoring, and the 3-D
    properties a cell can carry.
    """
    def __init__(self) -> None:
        """A change that changes nothing. Add to it with the `with_…` methods."""
        ...
    def with_fill(self, fill: FillSpec) -> "CellFormat":
        """This change, also setting the cells' fill."""
        ...
    def without_fill(self) -> "CellFormat":
        """This change, also clearing the cells' fill so they inherit the table style's."""
        ...
    def with_border(self, edge: CellBorder, line: LineSpec) -> "CellFormat":
        """This change, also setting one edge's border."""
        ...
    def with_outline(self, line: LineSpec) -> "CellFormat":
        """This change, also setting all four outer edges to the same line."""
        ...
    def without_border(self, edge: CellBorder) -> "CellFormat":
        """This change, also clearing one edge's border."""
        ...
    def without_borders(self) -> "CellFormat":
        """This change, also clearing every border."""
        ...
    def with_margins(self, margins: CellMargins) -> "CellFormat":
        """This change, also setting the cells' inner margins."""
        ...
    def with_anchor(self, anchor: TextAnchoring) -> "CellFormat":
        """This change, also setting how text sits vertically in the cells."""
        ...
    def with_text_direction(self, direction: TextDirection) -> "CellFormat":
        """This change, also setting the cells' text direction."""
        ...
    def with_horizontal_overflow(self, overflow: TextHorizontalOverflow) -> "CellFormat":
        """This change, also setting whether text that does not fit is clipped."""
        ...
    def with_cell_material(self, material: PresetMaterial) -> "CellFormat":
        """This change, also setting the cells' 3-D surface material."""
        ...
    def with_cell_bevel(self, bevel: Bevel) -> "CellFormat":
        """This change, also setting the cells' bevel."""
        ...
    def with_cell_light_rig(self, light_rig: LightRig) -> "CellFormat":
        """This change, also setting the cells' light rig."""
        ...
    is_empty: bool
    """Whether this change would change nothing."""

@final
class TableStyleFormat:
    """The formatting one part of a table style states."""
    def __init__(self) -> None:
        """Formatting that states nothing. Add to it with the `with_…` methods."""
        ...
    def with_fill(self, fill: FillSpec) -> "TableStyleFormat":
        """This formatting with the given cell fill."""
        ...
    def with_bold(self, bold: OnOffStyle) -> "TableStyleFormat":
        """This formatting with the given boldness. A table style's on/off values are three-valued
        — on, off, or "whatever the default is" — which is why this takes an [`OnOffStyle`].
        """
        ...
    def with_italic(self, italic: OnOffStyle) -> "TableStyleFormat":
        """This formatting with the given italicisation."""
        ...
    def with_text_color(self, color: ColorSpec) -> "TableStyleFormat":
        """This formatting with the given text colour."""
        ...
    def with_border(self, edge: TableStyleBorder, line: LineSpec) -> "TableStyleFormat":
        """This formatting with the given border on one edge. A table style has eight edges,
        including the two *inside* ones a single cell does not have.
        """
        ...
    def with_cell_material(self, material: PresetMaterial) -> "TableStyleFormat":
        """This formatting with the given 3-D surface material."""
        ...
    def with_cell_bevel(self, bevel: Bevel) -> "TableStyleFormat":
        """This formatting with the given bevel."""
        ...
    def with_cell_light_rig(self, light_rig: LightRig) -> "TableStyleFormat":
        """This formatting with the given light rig."""
        ...

@final
class TableStyleDefinition:
    """A whole table style: an identifier, a name, and formatting for each of its thirteen parts."""
    def __init__(self) -> None:
        """A style that states nothing. Add to it with the `with_…` methods."""
        ...
    def with_id(self, style_id: str) -> "TableStyleDefinition":
        """This style with the given identifier — a GUID in braces, as `tableStyles.xml` writes
        them.
        """
        ...
    def with_name(self, style_name: str) -> "TableStyleDefinition":
        """This style with the given display name."""
        ...
    def with_part(self, part: TableStylePart, format: TableStyleFormat) -> "TableStyleDefinition":
        """This style with formatting for one of its thirteen parts."""
        ...

@final
class TableStyleFlags:
    """Which of a table style's six banding and heading parts a table has turned on."""
    def __init__(self, first_row: bool = ..., last_row: bool = ..., first_column: bool = ..., last_column: bool = ..., banded_rows: bool = ..., banded_columns: bool = ...) -> None:
        """Which banding and heading parts a table has turned on."""
        ...
    first_row: bool
    """Whether the first row is formatted as a header."""
    last_row: bool
    """Whether the last row is formatted as a total."""
    first_column: bool
    """Whether the first column is formatted as a header."""
    last_column: bool
    """Whether the last column is formatted as a total."""
    banded_rows: bool
    """Whether rows alternate between the two banding parts."""
    banded_columns: bool
    """Whether columns alternate between the two banding parts."""

@final
class CharacterPropertiesSpec:
    """Everything a run of text can state about itself: size, weight, colour, underline, the fonts
    for each script.
    """
    def __init__(self) -> None:
        """A specification that states nothing. Everything it does not state is inherited."""
        ...
    def with_size_points(self, points: float) -> "CharacterPropertiesSpec":
        """This specification at the given size in points."""
        ...
    def with_size(self, size: FontSize) -> "CharacterPropertiesSpec":
        """This specification at the given size."""
        ...
    def with_bold(self, bold: bool) -> "CharacterPropertiesSpec":
        """This specification, bold or not."""
        ...
    def with_italic(self, italic: bool) -> "CharacterPropertiesSpec":
        """This specification, italic or not."""
        ...
    def with_underline(self, underline: TextUnderline) -> "CharacterPropertiesSpec":
        """This specification with the given underline style."""
        ...
    def with_strike(self, strike: TextStrike) -> "CharacterPropertiesSpec":
        """This specification with the given strike-through."""
        ...
    def with_capitalization(self, capitalization: TextCapitalization) -> "CharacterPropertiesSpec":
        """This specification with the given capitalisation."""
        ...
    def with_spacing_points(self, points: float) -> "CharacterPropertiesSpec":
        """This specification with the given letter spacing, in points."""
        ...
    def with_kerning_points(self, points: float) -> "CharacterPropertiesSpec":
        """This specification with the given kerning threshold, in points."""
        ...
    def with_baseline(self, baseline: Fraction) -> "CharacterPropertiesSpec":
        """This specification raised or lowered by the given proportion of the font size."""
        ...
    def with_language(self, language: str) -> "CharacterPropertiesSpec":
        """This specification tagged with the given language, such as `"en-GB"`."""
        ...
    def with_color(self, color: ColorSpec) -> "CharacterPropertiesSpec":
        """This specification in the given colour — a solid fill, which is what a colour is here."""
        ...
    def with_fill(self, fill: FillSpec) -> "CharacterPropertiesSpec":
        """This specification with the given fill, which may be a gradient or a picture."""
        ...
    def with_outline(self, outline: LineSpec) -> "CharacterPropertiesSpec":
        """This specification with the given text outline."""
        ...
    def with_effects(self, effects: EffectListSpec) -> "CharacterPropertiesSpec":
        """This specification with the given text effects."""
        ...
    def with_highlight(self, highlight: ColorSpec) -> "CharacterPropertiesSpec":
        """This specification with the given highlight colour."""
        ...
    def with_underline_line(self, underline_line: UnderlineLine) -> "CharacterPropertiesSpec":
        """This specification with the given underline line style."""
        ...
    def with_underline_fill(self, underline_fill: UnderlineFill) -> "CharacterPropertiesSpec":
        """This specification with the given underline fill."""
        ...
    def with_font(self, typeface: str) -> "CharacterPropertiesSpec":
        """This specification in the given typeface, for the Latin slot."""
        ...
    def with_font_for(self, slot: FontSlot, font: TextFont) -> "CharacterPropertiesSpec":
        """This specification with the given font for one script slot."""
        ...
    size: FontSize | None
    """The size, when stated."""
    size_points: float | None
    """The size in points, when stated."""
    is_bold: bool | None
    """Whether bold is stated, and what it says."""
    is_italic: bool | None
    """Whether italic is stated, and what it says."""
    underline: TextUnderline | None
    """The underline style, when stated."""
    strike: TextStrike | None
    """The strike-through, when stated."""
    capitalization: TextCapitalization | None
    """The capitalisation, when stated."""
    spacing_points: float | None
    """The letter spacing in points, when stated."""
    kerning_points: float | None
    """The kerning threshold in points, when stated."""
    baseline: Fraction | None
    """The baseline offset, when stated."""
    language: str | None
    """The language tag, when stated."""
    fill: FillSpec | None
    """The fill, when stated."""
    outline: LineSpec | None
    """The text outline, when stated."""
    effects: EffectListSpec | None
    """The text effects, when stated."""
    highlight: ColorSpec | None
    """The highlight colour, when stated."""
    underline_line: UnderlineLine | None
    """The underline line style, when stated."""
    underline_fill: UnderlineFill | None
    """The underline fill, when stated."""
    def font(self, slot: FontSlot) -> TextFont | None:
        """The font for one script slot, when stated."""
        ...
    def merge_under(self, lower: "CharacterPropertiesSpec") -> "CharacterPropertiesSpec":
        """This specification laid over `lower`: whatever this one states wins, and whatever it
        leaves unstated comes from `lower`. The same walk the `effective_…` readers make, one
        rung at a time.
        """
        ...

@final
class ParagraphPropertiesSpec:
    """Everything a paragraph can state: alignment, margins, spacing, bullet, tab stops, and the
    run properties its own text inherits.
    """
    def __init__(self) -> None:
        """A specification that states nothing. Everything it does not state is inherited."""
        ...
    def with_level(self, level: IndentLevel) -> "ParagraphPropertiesSpec":
        """This specification at the given list level."""
        ...
    def with_alignment(self, alignment: TextAlignment) -> "ParagraphPropertiesSpec":
        """This specification with the given alignment."""
        ...
    def with_left_margin_points(self, points: float) -> "ParagraphPropertiesSpec":
        """This specification with the given left margin, in points."""
        ...
    def with_right_margin_points(self, points: float) -> "ParagraphPropertiesSpec":
        """This specification with the given right margin, in points."""
        ...
    def with_indent_points(self, points: float) -> "ParagraphPropertiesSpec":
        """This specification with the given first-line indent, in points."""
        ...
    def with_default_tab_size_points(self, points: float) -> "ParagraphPropertiesSpec":
        """This specification with the given default tab size, in points."""
        ...
    def with_right_to_left(self, right_to_left: bool) -> "ParagraphPropertiesSpec":
        """This specification, right-to-left or not."""
        ...
    def with_font_alignment(self, font_alignment: FontAlignment) -> "ParagraphPropertiesSpec":
        """This specification with the given font alignment within the line box."""
        ...
    def with_line_spacing(self, spacing: TextSpacing) -> "ParagraphPropertiesSpec":
        """This specification with the given line spacing."""
        ...
    def with_space_before(self, spacing: TextSpacing) -> "ParagraphPropertiesSpec":
        """This specification with the given space before the paragraph."""
        ...
    def with_space_after(self, spacing: TextSpacing) -> "ParagraphPropertiesSpec":
        """This specification with the given space after the paragraph."""
        ...
    def with_bullet(self, bullet: Bullet) -> "ParagraphPropertiesSpec":
        """This specification with the given bullet."""
        ...
    def with_bullet_character(self, character: str) -> "ParagraphPropertiesSpec":
        """This specification bulleted with the given character."""
        ...
    def without_bullet(self) -> "ParagraphPropertiesSpec":
        """This specification with no bullet — `a:buNone`, which turns off an inherited one."""
        ...
    def with_bullet_color(self, color: BulletColor) -> "ParagraphPropertiesSpec":
        """This specification with the given bullet colour."""
        ...
    def with_bullet_size(self, size: BulletSize) -> "ParagraphPropertiesSpec":
        """This specification with the given bullet size."""
        ...
    def with_bullet_typeface(self, typeface: BulletTypeface) -> "ParagraphPropertiesSpec":
        """This specification with the given bullet typeface."""
        ...
    def with_tab_stops(self, stops: list[TabStop]) -> "ParagraphPropertiesSpec":
        """This specification with the given tab stops, replacing any it already had."""
        ...
    def with_default_run_properties(self, properties: CharacterPropertiesSpec) -> "ParagraphPropertiesSpec":
        """This specification with the given default run properties — what the paragraph's own text
        inherits before any run states anything.
        """
        ...
    level: IndentLevel | None
    """The list level, when stated."""
    alignment: TextAlignment | None
    """The alignment, when stated."""
    left_margin_points: float | None
    """The left margin in points, when stated."""
    right_margin_points: float | None
    """The right margin in points, when stated."""
    indent_points: float | None
    """The first-line indent in points, when stated."""
    default_tab_size_points: float | None
    """The default tab size in points, when stated."""
    is_right_to_left: bool | None
    """Whether right-to-left is stated, and what it says."""
    font_alignment: FontAlignment | None
    """The font alignment, when stated."""
    line_spacing: TextSpacing | None
    """The line spacing, when stated."""
    space_before: TextSpacing | None
    """The space before the paragraph, when stated."""
    space_after: TextSpacing | None
    """The space after the paragraph, when stated."""
    bullet: Bullet | None
    """The bullet, when stated."""
    bullet_color: BulletColor | None
    """The bullet colour, when stated."""
    bullet_size: BulletSize | None
    """The bullet size, when stated."""
    bullet_typeface: BulletTypeface | None
    """The bullet typeface, when stated."""
    tab_stops: list[TabStop]
    """The tab stops, in order."""
    default_run_properties: CharacterPropertiesSpec | None
    """The default run properties, when stated."""
    def merge_under(self, lower: "ParagraphPropertiesSpec") -> "ParagraphPropertiesSpec":
        """This specification laid over `lower`: whatever this one states wins."""
        ...

@final
class TextFont:
    """A typeface reference: the name, and the classification attributes that let a consumer
    substitute when the font is missing.
    """
    def __init__(self, typeface: str, panose: str | None = ..., pitch_family: int | None = ..., charset: int | None = ...) -> None:
        """A typeface by name. `"+mj-lt"` and `"+mn-lt"` name the theme's major and minor Latin
        fonts.
        """
        ...
    typeface: str
    """The typeface name, exactly as written."""
    panose: str | None
    """The PANOSE classification, when the document states one."""
    pitch_family: int | None
    """The pitch and family byte, when stated."""
    charset: int | None
    """The character set byte, when stated."""
    is_theme_reference: bool
    """Whether this names a theme font (`+mj-lt`, `+mn-ea`, …) rather than a typeface."""
    theme_reference: ThemeFontReference | None
    """Which theme font this names, when it names one."""

@final
class ThemeFontReference:
    """A theme font reference — which collection (major or minor) and which script slot."""
    def __init__(self, collection: FontSchemeSlot, slot: FontSlot) -> None:
        """The major or minor collection, and the script slot within it."""
        ...
    collection: FontSchemeSlot
    """Which collection — the major (heading) or minor (body) fonts."""
    slot: FontSlot
    """Which script slot within the collection."""

@final
class TabStop:
    """One tab stop: where it is, and how text aligns at it."""
    @staticmethod
    def at_points(points: float, alignment: TabAlignment) -> "TabStop":
        """A tab stop at the given number of points, with the given alignment."""
        ...
    def __init__(self, position: Emu, alignment: TabAlignment | None = ...) -> None:
        """A tab stop at an absolute position, optionally with an alignment."""
        ...
    position: Emu
    """Where the stop is."""
    position_points: float
    """The position in points."""
    alignment: TabAlignment | None
    """How text aligns at the stop, when stated."""

@final
class Bullet:
    """What marks a paragraph: nothing, a character, an automatic number, or a picture."""
    @staticmethod
    def none() -> "Bullet":
        """No bullet — `a:buNone`, which is how a paragraph turns one off that it would inherit."""
        ...
    @staticmethod
    def character(character: BulletCharacter) -> "Bullet":
        """A literal character."""
        ...
    @staticmethod
    def auto_number(bullet: AutoNumberBullet) -> "Bullet":
        """An automatic number."""
        ...
    @staticmethod
    def picture(picture: BulletPicture) -> "Bullet":
        """A picture."""
        ...
    kind: str
    """Which kind this is: `"none"`, `"character"`, `"auto_number"` or `"picture"`."""
    character_bullet: BulletCharacter | None
    """The character, when this is a character bullet."""
    auto_number_bullet: AutoNumberBullet | None
    """The numbering, when this is an automatic number."""
    picture_bullet: BulletPicture | None
    """The picture, when this is a picture bullet."""

@final
class BulletCharacter:
    """A literal bullet character."""
    def __init__(self, character: str) -> None:
        """A literal bullet character, such as `"•"` or `"–"`."""
        ...
    character: str
    """The character."""

@final
class AutoNumberBullet:
    """An automatically numbered bullet: which numbering scheme, and what it starts at."""
    def __init__(self, scheme: AutonumberScheme, start_at: int = ...) -> None:
        """An automatically numbered bullet in the given scheme, starting at `start_at` (default
        `1`).
        """
        ...
    scheme: AutonumberScheme
    """Which of the forty-one numbering schemes."""
    start_at: int
    """The number the list starts at."""

@final
class BulletPicture:
    """A picture bullet, named by the relationship id of its image."""
    def __init__(self, image_rel_id: str) -> None:
        """A picture bullet, named by the relationship id `Deck.add_image` hands back."""
        ...
    image_rel_id: str
    """The image's relationship id."""

@final
class BulletColor:
    """The bullet's colour: the text's, or one of its own."""
    @staticmethod
    def follow_text() -> "BulletColor":
        """The bullet takes the colour of the text it marks."""
        ...
    @staticmethod
    def explicit(color: ColorSpec) -> "BulletColor":
        """The bullet is painted in a colour of its own."""
        ...
    follows_text: bool
    """Whether the bullet follows the text's colour."""
    color: ColorSpec | None
    """The bullet's own colour, when it has one."""

@final
class BulletSize:
    """The bullet's size: the text's, a proportion of it, or an absolute size."""
    @staticmethod
    def follow_text() -> "BulletSize":
        """The bullet takes the size of the text it marks."""
        ...
    @staticmethod
    def percentage(proportion: float) -> "BulletSize":
        """The bullet is a proportion of the text's size: `0.75` is three quarters."""
        ...
    @staticmethod
    def points(points: float) -> "BulletSize":
        """The bullet is an absolute number of points."""
        ...
    kind: str
    """Which kind this is: `"follow_text"`, `"percentage"` or `"points"`."""
    proportion: Fraction | None
    """The proportion, when this is a proportional size."""
    size: FontSize | None
    """The absolute size, when this is one."""

@final
class BulletTypeface:
    """The bullet's typeface: the text's, or one of its own."""
    @staticmethod
    def follow_text() -> "BulletTypeface":
        """The bullet uses the typeface of the text it marks."""
        ...
    @staticmethod
    def named(typeface: str) -> "BulletTypeface":
        """The bullet uses a typeface of its own — `"Wingdings"`, typically."""
        ...
    follows_text: bool
    """Whether the bullet follows the text's typeface."""
    font: TextFont | None
    """The bullet's own font, when it has one."""

@final
class TextSpacing:
    """A spacing measure: a proportion of the line, or an absolute number of points."""
    @staticmethod
    def proportion(proportion: float) -> "TextSpacing":
        """A proportion of the line's own height: `1.5` is one-and-a-half spacing."""
        ...
    @staticmethod
    def points(points: float) -> "TextSpacing":
        """An absolute number of points."""
        ...
    kind: str
    """Which kind this is: `"percentage"` or `"points"`."""
    ratio: Fraction | None
    """The proportion, when this is proportional spacing."""
    measure: TextPoint | None
    """The absolute measure, when this is absolute spacing."""

@final
class UnderlineLine:
    """The underline's line style: the text's, or one of its own."""
    @staticmethod
    def follow_text() -> "UnderlineLine":
        """The underline takes the run's own outline."""
        ...
    @staticmethod
    def explicit(line: LineSpec) -> "UnderlineLine":
        """The underline is drawn with a line of its own."""
        ...
    follows_text: bool
    """Whether the underline follows the run's outline."""
    line: LineSpec | None
    """The underline's own line, when it has one."""

@final
class UnderlineFill:
    """The underline's fill: the text's, or one of its own."""
    @staticmethod
    def follow_text() -> "UnderlineFill":
        """The underline takes the run's own fill."""
        ...
    @staticmethod
    def explicit(fill: FillSpec) -> "UnderlineFill":
        """The underline is painted with a fill of its own."""
        ...
    follows_text: bool
    """Whether the underline follows the run's fill."""
    fill: FillSpec | None
    """The underline's own fill, when it has one."""

@final
class SupplementalFont:
    """A theme's font for one script beyond the three main slots."""
    script: str
    """The script this font covers, as the theme names it."""
    typeface: str
    """The typeface for that script."""

@final
class FontCollection:
    """One half of a theme's font scheme — the fonts for the Latin, East Asian and complex-script
    slots, plus the supplemental fonts.
    """
    def font(self, slot: FontSlot) -> TextFont | None:
        """The font for one script slot, when the collection states one."""
        ...
    supplemental_fonts: list[SupplementalFont]
    """Every supplemental font this collection lists."""
    def supplemental_font(self, script: str) -> SupplementalFont | None:
        """The supplemental font for one script, when the collection states one."""
        ...

@final
class FontScheme:
    """A theme's font scheme: its name, and its major and minor collections."""
    name: str
    """The scheme's name, as the theme states it."""
    major: FontCollection
    """The major (heading) collection."""
    minor: FontCollection
    """The minor (body) collection."""
    def collection(self, slot: FontSchemeSlot) -> FontCollection:
        """One of the two collections by name."""
        ...
    def font(self, reference: ThemeFontReference) -> TextFont | None:
        """The font a theme reference resolves to, when the scheme states one."""
        ...
    def resolve(self, font: TextFont) -> TextFont | None:
        """The typeface a font resolves to: itself, unless it is a theme reference, in which case
        the font this scheme names for that slot.
        """
        ...

@final
class ThemeInfo:
    """What a theme states, interner-free: its colours, its fonts, and its style matrices."""
    def color(self, slot: ColorSchemeSlot) -> ColorSpec | None:
        """The colour a scheme slot resolves to in this theme, when it states one."""
        ...
    colors: list[tuple[ColorSchemeSlot, ColorSpec]]
    """Every slot the theme states a colour for, paired with that colour."""
    font_scheme: FontScheme | None
    """The theme's font scheme, when it states one."""
    fill_styles: list[FillSpec]
    """The theme's fill style matrix, in order."""
    def fill_style(self, index: int) -> FillSpec | None:
        """One fill style by index — the number a shape's `a:fillRef@idx` names, counting from one."""
        ...
    line_styles: list[LineSpec]
    """The theme's line style matrix, in order."""
    def line_style(self, index: int) -> LineSpec | None:
        """One line style by index."""
        ...
    def effect_style(self, index: int) -> EffectListSpec | None:
        """One effect style by index."""
        ...

@final
class Point3D:
    """A point in three dimensions, in English Metric Units."""
    def __init__(self, x: Emu, y: Emu, z: Emu) -> None:
        """A point in three dimensions."""
        ...
    x: Emu
    """The horizontal coordinate."""
    y: Emu
    """The vertical coordinate."""
    z: Emu
    """The depth coordinate."""

@final
class Vector3D:
    """A direction in three dimensions."""
    def __init__(self, x: Emu, y: Emu, z: Emu) -> None:
        """A direction in three dimensions."""
        ...
    x: Emu
    """The horizontal component."""
    y: Emu
    """The vertical component."""
    z: Emu
    """The depth component."""

@final
class SphereCoordinates:
    """A rotation stated as latitude, longitude and revolution."""
    def __init__(self, latitude: Angle, longitude: Angle, revolution: Angle) -> None:
        """A rotation about the three axes."""
        ...
    latitude: Angle
    """The rotation about the horizontal axis."""
    longitude: Angle
    """The rotation about the vertical axis."""
    revolution: Angle
    """The rotation about the view axis."""

@final
class Camera:
    """Where the viewer stands: one of the sixty-two preset cameras, plus optional field of view,
    zoom and rotation.
    """
    def __init__(self, preset: PresetCamera, field_of_view: Angle | None = ..., zoom: Fraction | None = ..., rotation: SphereCoordinates | None = ...) -> None:
        """A camera: one of the sixty-two presets, optionally overridden."""
        ...
    preset: PresetCamera
    """Which of the sixty-two preset cameras."""
    field_of_view: Angle | None
    """The field of view, when stated."""
    zoom: Fraction | None
    """The zoom, when stated."""
    rotation: SphereCoordinates | None
    """The rotation, when stated."""

@final
class LightRig:
    """Where the light comes from: a rig, a direction, and an optional rotation."""
    def __init__(self, rig: LightRigType, direction: LightRigDirection, rotation: SphereCoordinates | None = ...) -> None:
        """A light rig: which lighting, from which direction, optionally rotated."""
        ...
    rig: LightRigType
    """Which of the twenty-seven lighting rigs."""
    direction: LightRigDirection
    """Which of the eight directions the light comes from."""
    rotation: SphereCoordinates | None
    """The rig's own rotation, when stated."""

@final
class Backdrop:
    """The plane a 3-D scene sits on."""
    def __init__(self, anchor: Point3D, normal: Vector3D, up: Vector3D) -> None:
        """The plane a 3-D scene sits on: a point on it, and two directions that orient it."""
        ...
    anchor: Point3D
    """A point on the plane."""
    normal: Vector3D
    """The direction perpendicular to the plane."""
    up: Vector3D
    """The direction that is "up" on the plane."""

@final
class Bevel:
    """The rounded or chamfered edge of an extruded shape."""
    def __init__(self, width: Emu | None = ..., height: Emu | None = ..., preset: BevelPreset | None = ...) -> None:
        """A bevel: how wide, how deep, and which of the twelve shapes."""
        ...
    width: Emu | None
    """The bevel's width, when stated."""
    height: Emu | None
    """The bevel's height, when stated."""
    preset: BevelPreset | None
    """Which of the twelve bevel shapes, when stated."""

@final
class Scene3DSpec:
    """A 3-D scene: a camera and a light rig."""
    def __init__(self, camera: Camera, light_rig: LightRig) -> None:
        """A 3-D scene: where the viewer stands and where the light comes from."""
        ...
    camera: Camera
    """The camera."""
    light_rig: LightRig
    """The light rig."""

@final
class Shape3DSpec:
    """A shape's own 3-D properties: depth, extrusion, contour, material and bevels."""
    def __init__(self, z: Emu | None = ..., extrusion_height: Emu | None = ..., contour_width: Emu | None = ..., material: PresetMaterial | None = ..., bevel_top: Bevel | None = ..., bevel_bottom: Bevel | None = ..., extrusion_color: ColorSpec | None = ..., contour_color: ColorSpec | None = ...) -> None:
        """A shape's 3-D properties. Everything is optional; a shape that states none is flat."""
        ...
    z: Emu | None
    """How far the shape sits off the scene's plane, when stated."""
    extrusion_height: Emu | None
    """How thick the extrusion is, when stated."""
    contour_width: Emu | None
    """How wide the contour is, when stated."""
    material: PresetMaterial | None
    """Which of the fifteen surface materials, when stated."""
    bevel_top: Bevel | None
    """The top bevel, when stated."""
    bevel_bottom: Bevel | None
    """The bottom bevel, when stated."""
    extrusion_color: ColorSpec | None
    """The extrusion's colour, when stated."""
    contour_color: ColorSpec | None
    """The contour's colour, when stated."""

def default_placeholder_audio() -> bytes:
    """The three placeholder payloads a `replace_…_with_placeholder` call defaults to, as module-
    level functions so a caller can hand one to `set_ole_object_data` or `set_picture_image`
    directly.
    """
    ...

def default_placeholder_video() -> bytes:
    """A one-frame, zero-length MP4 — a video placeholder that every consumer accepts."""
    ...

def default_placeholder_ole() -> bytes:
    """An empty compound file — an OLE object placeholder that every consumer accepts."""
    ...

def detect_format(data: bytes) -> Format:
    """What these bytes are, read from the package's main part rather than from a filename."""
    ...
