//! Unit tests for the custom-geometry foundation (`a:custGeom` value types), through the public API.
//!
//! The two properties these carry the whole model on: an [`AdjustCoordinate`] / [`AdjustAngle`]
//! distinguishes a literal from a **guide reference**, and an [`AdjustPoint`] round-trips
//! byte-for-byte with any attribute it does not model preserved verbatim.

use mjx_dml::{
    AdjustAngle, AdjustCoordinate, AdjustPoint, DrawCommand, Emu, Path2D, Path2DList, Path2DSpec,
    PathFillMode, Point,
};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, ToXml};
use mjx_xml::fidelity;

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

fn parse_typed<T: FromXml>(fragment: &[u8]) -> (T, RawDocument) {
    let doc = fidelity::parse(fragment).expect("fragment parses");
    let typed = T::from_xml(&doc.root, &doc.interner).expect("from_xml");
    (typed, doc)
}

#[track_caller]
fn assert_round_trips<T: ToXml>(typed: &T, mut doc: RawDocument, expected: &[u8]) {
    doc.root = typed.to_xml(&mut doc.interner);
    let out = fidelity::serialize_to_vec(&doc);
    assert_eq!(
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(expected),
        "round-trip byte mismatch"
    );
}

// ---------------------------------------------------------------------------------------------
// AdjustCoordinate — a literal EMU, or a guide reference
// ---------------------------------------------------------------------------------------------

#[test]
fn an_adjust_coordinate_reads_an_integer_as_emu_and_anything_else_as_a_guide() {
    assert_eq!(
        AdjustCoordinate::from_wire("100"),
        AdjustCoordinate::Emu(Emu::from_emu(100))
    );
    assert_eq!(
        AdjustCoordinate::from_wire("-2540"),
        AdjustCoordinate::Emu(Emu::from_emu(-2540))
    );
    assert_eq!(
        AdjustCoordinate::from_wire("hc"),
        AdjustCoordinate::Guide("hc".to_owned())
    );
}

#[test]
fn an_adjust_coordinate_round_trips_through_its_wire_form() {
    assert_eq!(AdjustCoordinate::Emu(Emu::from_emu(100)).to_wire(), "100");
    assert_eq!(AdjustCoordinate::Guide("adj1".to_owned()).to_wire(), "adj1");
    for wire in ["0", "914400", "-50", "cd2", "wd8"] {
        assert_eq!(AdjustCoordinate::from_wire(wire).to_wire(), wire);
    }
}

// ---------------------------------------------------------------------------------------------
// AdjustAngle — a literal angle, or a guide reference
// ---------------------------------------------------------------------------------------------

#[test]
fn an_adjust_angle_reads_an_integer_as_an_angle_and_anything_else_as_a_guide() {
    // 5_400_000 sixtieths-of-a-thousandth-of-a-degree is a quarter turn.
    match AdjustAngle::from_wire("5400000") {
        AdjustAngle::Angle(angle) => assert!((angle.degrees() - 90.0).abs() < 1e-9),
        AdjustAngle::Guide(_) => panic!("integer angle read as a guide"),
    }
    assert_eq!(
        AdjustAngle::from_wire("adj"),
        AdjustAngle::Guide("adj".to_owned())
    );
}

#[test]
fn an_adjust_angle_round_trips_through_its_wire_form() {
    for wire in ["0", "5400000", "-1200000", "21600000", "adj2"] {
        assert_eq!(AdjustAngle::from_wire(wire).to_wire(), wire);
    }
}

// ---------------------------------------------------------------------------------------------
// AdjustPoint — a fidelity leaf that reads x/y typed and preserves the rest
// ---------------------------------------------------------------------------------------------

#[test]
fn an_adjust_point_reads_its_coordinates_typed() {
    let xml = format!(r#"<a:pt xmlns:a="{A}" x="914400" y="hc"/>"#);
    let (pt, doc) = parse_typed::<AdjustPoint>(xml.as_bytes());
    assert_eq!(
        pt.x(&doc.interner),
        Some(AdjustCoordinate::Emu(Emu::from_emu(914_400)))
    );
    assert_eq!(
        pt.y(&doc.interner),
        Some(AdjustCoordinate::Guide("hc".to_owned()))
    );
}

#[test]
fn an_adjust_point_round_trips_byte_for_byte_with_an_unknown_attribute() {
    // The `foo` attribute is not modeled; it must survive verbatim, in its original position.
    let xml = format!(r#"<a:pt xmlns:a="{A}" x="100" foo="bar" y="200"/>"#);
    let (pt, doc) = parse_typed::<AdjustPoint>(xml.as_bytes());
    assert_eq!(
        pt.x(&doc.interner),
        Some(AdjustCoordinate::Emu(Emu::from_emu(100)))
    );
    assert_round_trips(&pt, doc, xml.as_bytes());
}

#[test]
fn a_built_adjust_point_reads_back_the_coordinates_it_was_given() {
    let mut interner = Interner::new();
    let pt = AdjustPoint::new(
        &mut interner,
        "pt",
        &AdjustCoordinate::Emu(Emu::from_emu(100)),
        &AdjustCoordinate::Guide("hc".to_owned()),
    );
    assert_eq!(
        pt.x(&interner),
        Some(AdjustCoordinate::Emu(Emu::from_emu(100)))
    );
    assert_eq!(
        pt.y(&interner),
        Some(AdjustCoordinate::Guide("hc".to_owned()))
    );

    // And it serializes to the expected `pt` element with both coordinates as attributes.
    let root = pt.to_xml(&mut interner);
    let doc = RawDocument {
        interner,
        bom: false,
        prologue: Vec::new(),
        root,
        epilogue: Vec::new(),
    };
    let out = String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8");
    assert!(out.contains(r#"x="100""#), "missing x: {out}");
    assert!(out.contains(r#"y="hc""#), "missing y: {out}");
}

// ---------------------------------------------------------------------------------------------
// Path2D / Path2DList — the drawing commands
// ---------------------------------------------------------------------------------------------

fn full_path_list() -> String {
    format!(
        concat!(
            r#"<a:pathLst xmlns:a="{a}">"#,
            r#"<a:path w="200" h="100" fill="lighten" stroke="0" extrusionOk="1">"#,
            r#"<a:moveTo><a:pt x="0" y="0"/></a:moveTo>"#,
            r#"<a:lnTo><a:pt x="100" y="hc"/></a:lnTo>"#,
            r#"<a:arcTo wR="50" hR="25" stAng="0" swAng="5400000"/>"#,
            r#"<a:quadBezTo><a:pt x="10" y="20"/><a:pt x="30" y="40"/></a:quadBezTo>"#,
            r#"<a:cubicBezTo><a:pt x="1" y="2"/><a:pt x="3" y="4"/><a:pt x="5" y="6"/></a:cubicBezTo>"#,
            r#"<a:close/>"#,
            r#"</a:path>"#,
            r#"</a:pathLst>"#,
        ),
        a = A
    )
}

#[test]
fn a_path_reads_its_flags_and_every_command_typed() {
    let list = full_path_list();
    let (path_list, doc) = parse_typed::<Path2DList>(list.as_bytes());
    let paths = path_list.paths(&doc.interner);
    assert_eq!(paths.len(), 1);
    let path = &paths[0];

    assert_eq!(path.width(&doc.interner), Some(Emu::from_emu(200)));
    assert_eq!(path.height(&doc.interner), Some(Emu::from_emu(100)));
    assert_eq!(path.fill(&doc.interner), Some(PathFillMode::Lighten));
    assert_eq!(path.stroke(&doc.interner), Some(false));
    assert_eq!(path.extrusion_ok(&doc.interner), Some(true));

    let commands = path.commands(&doc.interner);
    assert_eq!(commands.len(), 6);
    assert_eq!(commands[0], DrawCommand::MoveTo(Point::from_emu(0, 0)));
    assert_eq!(
        commands[1],
        DrawCommand::LineTo(Point {
            x: AdjustCoordinate::Emu(Emu::from_emu(100)),
            y: AdjustCoordinate::Guide("hc".to_owned()),
        })
    );
    match &commands[2] {
        DrawCommand::ArcTo {
            width_radius,
            height_radius,
            start_angle,
            swing_angle,
        } => {
            assert_eq!(*width_radius, AdjustCoordinate::Emu(Emu::from_emu(50)));
            assert_eq!(*height_radius, AdjustCoordinate::Emu(Emu::from_emu(25)));
            assert_eq!(*start_angle, AdjustAngle::from_wire("0"));
            assert_eq!(*swing_angle, AdjustAngle::from_wire("5400000"));
        }
        other => panic!("expected ArcTo, got {other:?}"),
    }
    assert_eq!(
        commands[3],
        DrawCommand::QuadBezierTo(Point::from_emu(10, 20), Point::from_emu(30, 40))
    );
    assert_eq!(
        commands[4],
        DrawCommand::CubicBezierTo(
            Point::from_emu(1, 2),
            Point::from_emu(3, 4),
            Point::from_emu(5, 6)
        )
    );
    assert_eq!(commands[5], DrawCommand::Close);
}

#[test]
fn a_path_list_round_trips_byte_for_byte() {
    let list = full_path_list();
    let (path_list, doc) = parse_typed::<Path2DList>(list.as_bytes());
    assert_round_trips(&path_list, doc, list.as_bytes());
}

#[test]
fn a_path_round_trips_with_an_unknown_command_preserved_opaquely() {
    // `a:futureCmd` is not a command this model knows; it must survive verbatim and be skipped in the
    // typed command view.
    let xml = format!(
        concat!(
            r#"<a:path xmlns:a="{a}">"#,
            r#"<a:moveTo><a:pt x="0" y="0"/></a:moveTo>"#,
            r#"<a:futureCmd foo="bar"/>"#,
            r#"<a:close/>"#,
            r#"</a:path>"#,
        ),
        a = A
    );
    let (path, doc) = parse_typed::<Path2D>(xml.as_bytes());
    let commands = path.commands(&doc.interner);
    assert_eq!(
        commands,
        vec![
            DrawCommand::MoveTo(Point::from_emu(0, 0)),
            DrawCommand::Close
        ]
    );
    assert_round_trips(&path, doc, xml.as_bytes());
}

#[test]
fn a_bare_path_states_no_flags() {
    let xml = format!(r#"<a:path xmlns:a="{A}"><a:close/></a:path>"#);
    let (path, doc) = parse_typed::<Path2D>(xml.as_bytes());
    assert_eq!(path.width(&doc.interner), None);
    assert_eq!(path.height(&doc.interner), None);
    assert_eq!(path.fill(&doc.interner), None);
    assert_eq!(path.stroke(&doc.interner), None);
    assert_eq!(path.extrusion_ok(&doc.interner), None);
    assert_eq!(path.commands(&doc.interner), vec![DrawCommand::Close]);
}

#[test]
fn a_built_path_reads_back_the_spec_it_was_given() {
    let spec = Path2DSpec {
        width: Some(Emu::from_emu(200)),
        height: None,
        fill: Some(PathFillMode::Darken),
        stroke: Some(false),
        extrusion_ok: None,
        commands: vec![
            DrawCommand::MoveTo(Point::from_emu(0, 0)),
            DrawCommand::ArcTo {
                width_radius: AdjustCoordinate::Emu(Emu::from_emu(50)),
                height_radius: AdjustCoordinate::Guide("hd2".to_owned()),
                start_angle: AdjustAngle::from_wire("0"),
                swing_angle: AdjustAngle::from_wire("5400000"),
            },
            DrawCommand::CubicBezierTo(
                Point::from_emu(1, 2),
                Point::from_emu(3, 4),
                Point::from_emu(5, 6),
            ),
            DrawCommand::Close,
        ],
    };

    let mut interner = Interner::new();
    let path = spec.to_path_2d(&mut interner);
    assert_eq!(path.spec(&interner), spec);
}

#[test]
fn a_built_path_list_holds_its_paths_in_order() {
    let specs = vec![
        Path2DSpec {
            fill: Some(PathFillMode::None),
            commands: vec![
                DrawCommand::MoveTo(Point::from_emu(1, 1)),
                DrawCommand::Close,
            ],
            ..Path2DSpec::default()
        },
        Path2DSpec {
            commands: vec![DrawCommand::LineTo(Point::from_emu(2, 2))],
            ..Path2DSpec::default()
        },
    ];
    let mut interner = Interner::new();
    let list = Path2DList::new(&mut interner, &specs);
    assert_eq!(list.specs(&interner), specs);
}
