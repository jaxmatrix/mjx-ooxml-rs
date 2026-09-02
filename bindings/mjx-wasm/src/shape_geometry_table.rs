// The preset-shape adjustment table: 117 shapes, 283 named adjustments.
//
// One line per preset shape that carries adjustments, in the order `mjx_dml::ShapeGeometry`
// declares them, each naming its adjustments and their units. `shape_geometries!` (in
// `geometry.rs`) turns this into the three functions that build a geometry, read one back, and
// name a preset's adjustments — so this file is the whole of the projection, and a shape added
// upstream fails to compile until it is added here.
//
// `fraction` is a proportion of the shape's own extent; `angle` is an angle. The names are
// `mjx-dml`'s, which are ECMA-376 Part 1's prose names for each `a:gd`.

shape_geometries! {
    RoundedRectangle { corner_radius: fraction }
    RoundSingleCornerRectangle { corner_radius: fraction }
    SnipSingleCornerRectangle { snip_size: fraction }
    Octagon { corner_cut: fraction }
    Plaque { corner_size: fraction }
    FoldedCorner { fold_size: fraction }
    Frame { border_thickness: fraction }
    FourPointStar { inner_radius: fraction }
    FivePointStar { inner_radius: fraction }
    SixPointStar { inner_radius: fraction }
    SevenPointStar { inner_radius: fraction }
    EightPointStar { inner_radius: fraction }
    TenPointStar { inner_radius: fraction }
    TwelvePointStar { inner_radius: fraction }
    SixteenPointStar { inner_radius: fraction }
    TwentyFourPointStar { inner_radius: fraction }
    ThirtyTwoPointStar { inner_radius: fraction }
    BracketPair { corner_radius: fraction }
    BracePair { curl_radius: fraction }
    LeftBracket { corner_radius: fraction }
    RightBracket { corner_radius: fraction }
    MathMinus { bar_thickness: fraction }
    MathPlus { arm_thickness: fraction }
    MathMultiply { stroke_thickness: fraction }
    Hexagon { point_inset: fraction }
    Trapezoid { top_inset: fraction }
    Triangle { apex_x: fraction }
    Parallelogram { skew_offset: fraction }
    Chevron { point_depth: fraction }
    HomePlate { point_depth: fraction }
    Plus { arm_inset: fraction }
    Donut { ring_thickness: fraction }
    NoSmoking { band_thickness: fraction }
    HorizontalScroll { curl_size: fraction }
    VerticalScroll { curl_size: fraction }
    Bevel { bevel_width: fraction }
    Can { top_ellipse_height: fraction }
    Cube { depth: fraction }
    Moon { crescent_width: fraction }
    SmileyFace { mouth_curve: fraction }
    DiagonalStripe { stripe_width: fraction }
    BentConnector3 { bend_position: fraction }
    CurvedConnector3 { bend_position: fraction }
    Arc { start_angle: angle, end_angle: angle }
    Chord { start_angle: angle, end_angle: angle }
    Pie { start_angle: angle, end_angle: angle }
    DownArrow { shaft_thickness: fraction, head_length: fraction }
    LeftArrow { shaft_thickness: fraction, head_length: fraction }
    RightArrow { shaft_thickness: fraction, head_length: fraction }
    LeftRightArrow { shaft_thickness: fraction, head_length: fraction }
    UpDownArrow { shaft_thickness: fraction, head_length: fraction }
    NotchedRightArrow { shaft_thickness: fraction, head_length: fraction }
    StripedRightArrow { shaft_thickness: fraction, head_length: fraction }
    SwooshArrow { head_thickness: fraction, head_length: fraction }
    CloudCallout { tail_x: fraction, tail_y: fraction }
    WedgeEllipseCallout { tail_x: fraction, tail_y: fraction }
    WedgeRectangleCallout { tail_x: fraction, tail_y: fraction }
    WedgeRoundedRectangleCallout { tail_x: fraction, tail_y: fraction }
    RoundSameSideCornersRectangle { top_corner_radius: fraction, bottom_corner_radius: fraction }
    RoundDiagonalCornersRectangle { top_left_bottom_right_radius: fraction, top_right_bottom_left_radius: fraction }
    SnipSameSideCornersRectangle { top_corner_snip: fraction, bottom_corner_snip: fraction }
    SnipDiagonalCornersRectangle { top_left_bottom_right_snip: fraction, top_right_bottom_left_snip: fraction }
    SnipAndRoundSingleCornerRectangle { round_corner_radius: fraction, snip_corner_size: fraction }
    LeftBrace { curl_radius: fraction, point_position: fraction }
    RightBrace { curl_radius: fraction, point_position: fraction }
    Ribbon { band_height: fraction, panel_width: fraction }
    Ribbon2 { band_height: fraction, panel_width: fraction }
    Wave { amplitude: fraction, skew: fraction }
    DoubleWave { amplitude: fraction, skew: fraction }
    Gear6 { tooth_depth: fraction, tooth_width: fraction }
    Gear9 { tooth_depth: fraction, tooth_width: fraction }
    BentConnector4 { bend_x: fraction, bend_y: fraction }
    CurvedConnector4 { bend_x: fraction, bend_y: fraction }
    Corner { horizontal_arm_thickness: fraction, vertical_arm_thickness: fraction }
    HalfFrame { top_arm_thickness: fraction, side_arm_thickness: fraction }
    MathEqual { bar_thickness: fraction, bar_gap: fraction }
    NonIsoscelesTrapezoid { left_top_inset: fraction, right_top_inset: fraction }
    Callout1 { vertex1_x: fraction, vertex1_y: fraction, vertex2_x: fraction, vertex2_y: fraction }
    Callout2 { vertex1_x: fraction, vertex1_y: fraction, vertex2_x: fraction, vertex2_y: fraction, vertex3_x: fraction, vertex3_y: fraction }
    Callout3 { vertex1_x: fraction, vertex1_y: fraction, vertex2_x: fraction, vertex2_y: fraction, vertex3_x: fraction, vertex3_y: fraction, vertex4_x: fraction, vertex4_y: fraction }
    AccentCallout1 { vertex1_x: fraction, vertex1_y: fraction, vertex2_x: fraction, vertex2_y: fraction }
    AccentCallout2 { vertex1_x: fraction, vertex1_y: fraction, vertex2_x: fraction, vertex2_y: fraction, vertex3_x: fraction, vertex3_y: fraction }
    AccentCallout3 { vertex1_x: fraction, vertex1_y: fraction, vertex2_x: fraction, vertex2_y: fraction, vertex3_x: fraction, vertex3_y: fraction, vertex4_x: fraction, vertex4_y: fraction }
    BorderCallout1 { vertex1_x: fraction, vertex1_y: fraction, vertex2_x: fraction, vertex2_y: fraction }
    BorderCallout2 { vertex1_x: fraction, vertex1_y: fraction, vertex2_x: fraction, vertex2_y: fraction, vertex3_x: fraction, vertex3_y: fraction }
    BorderCallout3 { vertex1_x: fraction, vertex1_y: fraction, vertex2_x: fraction, vertex2_y: fraction, vertex3_x: fraction, vertex3_y: fraction, vertex4_x: fraction, vertex4_y: fraction }
    AccentBorderCallout1 { vertex1_x: fraction, vertex1_y: fraction, vertex2_x: fraction, vertex2_y: fraction }
    AccentBorderCallout2 { vertex1_x: fraction, vertex1_y: fraction, vertex2_x: fraction, vertex2_y: fraction, vertex3_x: fraction, vertex3_y: fraction }
    AccentBorderCallout3 { vertex1_x: fraction, vertex1_y: fraction, vertex2_x: fraction, vertex2_y: fraction, vertex3_x: fraction, vertex3_y: fraction, vertex4_x: fraction, vertex4_y: fraction }
    BentConnector5 { bend1_x: fraction, bend2_y: fraction, bend3_x: fraction }
    CurvedConnector5 { bend1_x: fraction, bend2_y: fraction, bend3_x: fraction }
    CurvedDownArrow { body_thickness: fraction, head_width: fraction, head_length: fraction }
    CurvedUpArrow { body_thickness: fraction, head_width: fraction, head_length: fraction }
    CurvedLeftArrow { body_thickness: fraction, head_width: fraction, head_length: fraction }
    CurvedRightArrow { body_thickness: fraction, head_width: fraction, head_length: fraction }
    EllipseRibbon { arch_height: fraction, center_width: fraction, fold_thickness: fraction }
    EllipseRibbon2 { arch_height: fraction, center_width: fraction, fold_thickness: fraction }
    LeftRightRibbon { band_height: fraction, end_width: fraction, center_fold: fraction }
    BentUpArrow { shaft_thickness: fraction, head_width: fraction, head_length: fraction }
    LeftUpArrow { shaft_thickness: fraction, head_width: fraction, head_length: fraction }
    LeftRightUpArrow { shaft_thickness: fraction, head_width: fraction, head_length: fraction }
    QuadArrow { shaft_thickness: fraction, head_width: fraction, head_length: fraction }
    DownArrowCallout { shaft_thickness: fraction, arrowhead_width: fraction, arrowhead_length: fraction, text_box_size: fraction }
    UpArrowCallout { shaft_thickness: fraction, arrowhead_width: fraction, arrowhead_length: fraction, text_box_size: fraction }
    LeftArrowCallout { shaft_thickness: fraction, arrowhead_width: fraction, arrowhead_length: fraction, text_box_size: fraction }
    RightArrowCallout { shaft_thickness: fraction, arrowhead_width: fraction, arrowhead_length: fraction, text_box_size: fraction }
    LeftRightArrowCallout { shaft_thickness: fraction, arrowhead_width: fraction, arrowhead_length: fraction, text_box_size: fraction }
    UpDownArrowCallout { shaft_thickness: fraction, arrowhead_width: fraction, arrowhead_length: fraction, text_box_size: fraction }
    QuadArrowCallout { shaft_thickness: fraction, arrowhead_width: fraction, arrowhead_length: fraction, text_box_size: fraction }
    BentArrow { shaft_thickness: fraction, arrowhead_width: fraction, arrowhead_length: fraction, bend_radius: fraction }
    UTurnArrow { shaft_thickness: fraction, arrowhead_width: fraction, arrowhead_length: fraction, bend_radius: fraction, tip_height: fraction }
    BlockArc { start_angle: angle, end_angle: angle, ring_thickness: fraction }
    MathDivide { bar_thickness: fraction, dot_gap: fraction, dot_radius: fraction }
    MathNotEqual { bar_thickness: fraction, slash_angle: angle, bar_gap: fraction }
    CircularArrow { body_thickness: fraction, head_pointer_angle: angle, end_angle: angle, start_angle: angle, head_width: fraction }
    LeftCircularArrow { body_thickness: fraction, head_pointer_angle: angle, end_angle: angle, start_angle: angle, head_width: fraction }
    LeftRightCircularArrow { body_thickness: fraction, head_pointer_angle: angle, end_angle: angle, start_angle: angle, head_width: fraction }
}
