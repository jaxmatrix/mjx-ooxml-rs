//! Integration tests for the VML model (MJX-140): reading a producer-shaped drawing, resolving the
//! shape-level references a modern construct points at, authoring a fresh drawing, and editing one
//! shape without disturbing its siblings.
//!
//! Fidelity is the acceptance criterion here, not a test category: a drawing parsed and re-emitted
//! without an edit must be byte-identical, and an edited drawing must keep everything the edit did
//! not name.

use mjx_vml::{
    AttachedObjectKind, DrawingContent, DrawingPart, OleDrawAspect, OleObjectKind, Shape,
    ShapeContent, ShapeTemplate, VmlError,
};

/// A drawing shaped like what PowerPoint and Word emit: a layout header, a shape template, the OLE
/// fallback shape an `p:oleObj@spid` names, a legacy form control with its attached data, an ink
/// shape, and markup this crate does not model at all.
const PRODUCER_SHAPED: &[u8] = br##"<xml xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:x="urn:schemas-microsoft-com:office:excel" xmlns:p="urn:schemas-microsoft-com:office:powerpoint" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
 <o:shapelayout v:ext="edit">
  <o:idmap v:ext="edit" data="1"/>
 </o:shapelayout>
 <v:shapetype id="_x0000_t75" coordsize="21600,21600" o:spt="75" o:preferrelative="t" path="m@4@5l@4@11@9@11@9@5xe" filled="f" stroked="f">
  <v:stroke joinstyle="miter"/>
  <v:formulas><v:f eqn="if lineDrawn pixelLineWidth 0"/></v:formulas>
  <v:path o:extrusionok="f" gradientshapeok="t" o:connecttype="rect"/>
 </v:shapetype>
 <v:shape id="_x0000_s1026" type="#_x0000_t75" style="position:absolute;width:100pt;height:50pt" o:ole="" o:spid="_x0000_s1026" alt="Worksheet" filled="f">
  <v:imagedata r:id="rId4" o:title="Worksheet"/>
 </v:shape>
 <v:shape id="_x0000_s1027" type="#_x0000_t75" style="width:80pt" fillcolor="#c0c0c0">
  <x:ClientData ObjectType="Radio">
   <x:SizeWithCells/>
   <x:AutoFill>False</x:AutoFill>
   <x:FmlaLink>Sheet1!$A$1</x:FmlaLink>
   <x:FirstButton/>
  </x:ClientData>
 </v:shape>
 <v:shape id="_x0000_s1028" style="width:40pt">
  <o:ink i="AQID" annotation="t" contentType="application/inkml+xml"/>
  <p:textdata id="rId9"/>
 </v:shape>
 <o:OLEObject Type="Embed" ProgID="Excel.Sheet.12" ShapeID="_x0000_s1026" DrawAspect="Content" ObjectID="_1219561732" r:id="rId5"/>
 <v:background id="unmodelled" o:bwmode="white"><v:fill type="gradient"/></v:background>
</xml>
"##;

fn parsed() -> DrawingPart {
    DrawingPart::parse(PRODUCER_SHAPED).expect("the producer-shaped drawing parses")
}

// ---------------------------------------------------------------------------------------------
// Fidelity
// ---------------------------------------------------------------------------------------------

#[test]
fn an_unedited_drawing_re_emits_byte_identically() {
    let mut part = parsed();
    assert_eq!(
        part.to_bytes(),
        PRODUCER_SHAPED,
        "parsing and re-emitting a drawing must reproduce it byte-for-byte"
    );
}

#[test]
fn markup_this_crate_does_not_model_survives_an_edit_elsewhere() {
    let mut part = parsed();
    let (drawing, interner) = part.drawing_and_interner();
    drawing
        .shape_by_identifier_mut(interner, "_x0000_s1027")
        .expect("the form-control shape")
        .set_fill_color(interner, "#ff0000");

    let text = String::from_utf8(part.to_bytes()).expect("utf-8");
    assert!(
        text.contains(r#"<v:background id="unmodelled" o:bwmode="white"><v:fill type="gradient"/></v:background>"#),
        "an element this crate does not model must be re-emitted verbatim, got:\n{text}"
    );
    assert!(
        text.contains(r#"<v:f eqn="if lineDrawn pixelLineWidth 0"/>"#),
        "an unmodelled *nested* element must survive too, got:\n{text}"
    );
    assert!(
        text.contains(r##"fillcolor="#ff0000""##),
        "the edit itself must land, got:\n{text}"
    );
}

#[test]
fn editing_one_shape_leaves_its_siblings_byte_identical() {
    let before = String::from_utf8(parsed().to_bytes()).expect("utf-8");

    let mut part = parsed();
    let (drawing, interner) = part.drawing_and_interner();
    drawing
        .shape_by_identifier_mut(interner, "_x0000_s1028")
        .expect("the ink shape")
        .set_alternate_text(interner, "Handwriting");
    let after = String::from_utf8(part.to_bytes()).expect("utf-8");

    // Every line but the one holding the edited shape's start tag is untouched.
    let before_lines: Vec<&str> = before.lines().collect();
    let after_lines: Vec<&str> = after.lines().collect();
    assert_eq!(
        before_lines.len(),
        after_lines.len(),
        "an attribute edit must not change the drawing's shape"
    );
    for (index, (old, new)) in before_lines.iter().zip(&after_lines).enumerate() {
        if old.contains(r#"id="_x0000_s1028""#) {
            assert_ne!(old, new, "line {index} carries the edit and should differ");
            continue;
        }
        assert_eq!(
            old, new,
            "line {index} was not edited and must be identical"
        );
    }
    assert!(
        after.contains(r#"alt="Handwriting""#),
        "the new attribute must be written"
    );
}

#[test]
fn a_malformed_drawing_is_a_typed_error_not_a_panic() {
    let error = DrawingPart::parse(b"<xml><v:shape").expect_err("truncated markup must fail");
    assert!(matches!(error, VmlError::Xml(_)), "got {error:?}");
}

// ---------------------------------------------------------------------------------------------
// Reading — the shape-level references
// ---------------------------------------------------------------------------------------------

#[test]
fn a_shape_resolves_from_the_identifier_a_modern_construct_names() {
    let part = parsed();
    let interner = part.interner();

    // This is the hop `p:oleObj@spid` / `p:control@spid` / `o:OLEObject@ShapeID` needs.
    let shape = part
        .drawing()
        .shape_by_identifier(interner, "_x0000_s1026")
        .expect("the OLE fallback shape");
    assert_eq!(
        shape.template_identifier(interner).as_deref(),
        Some("_x0000_t75"),
        "the `#`-prefixed template reference resolves to a bare identifier"
    );
    assert_eq!(shape.alternate_text(interner).as_deref(), Some("Worksheet"));
    assert_eq!(shape.is_filled(interner), Some(false));
    assert_eq!(
        shape.is_embedded_object(interner),
        Some(true),
        "a value-less o:ole marks the shape as an embedded object"
    );
    assert_eq!(
        shape.application_shape_identifier(interner).as_deref(),
        Some("_x0000_s1026"),
        "o:spid is read separately from the unprefixed id"
    );

    // The template it names is in the same drawing.
    let template = part
        .drawing()
        .shape_template_by_identifier(interner, "_x0000_t75")
        .expect("the shape template");
    assert_eq!(template.is_stroked(interner), Some(false));

    // An identifier no shape carries answers None rather than the wrong shape.
    assert!(part
        .drawing()
        .shape_by_identifier(interner, "_x0000_s9999")
        .is_none());
}

#[test]
fn an_unprefixed_id_is_never_confused_with_a_relationship_id() {
    let part = parsed();
    let interner = part.interner();
    let shape = part
        .drawing()
        .shape_by_identifier(interner, "_x0000_s1026")
        .expect("the OLE fallback shape");
    let image = shape.image_data().expect("v:imagedata");

    // The shape's own `id` and the image's `r:id` share a local name and must not cross.
    assert_eq!(image.relationship_id(interner).as_deref(), Some("rId4"));
    assert_eq!(image.title(interner).as_deref(), Some("Worksheet"));
    assert_eq!(shape.identifier(interner).as_deref(), Some("_x0000_s1026"));
}

#[test]
fn an_ole_binding_resolves_to_the_shape_that_draws_it() {
    let part = parsed();
    let interner = part.interner();
    let object = part
        .drawing()
        .embedded_ole_objects()
        .next()
        .expect("the o:OLEObject binding");

    assert_eq!(object.kind(interner), Some(OleObjectKind::Embedded));
    assert_eq!(object.draw_aspect(interner), Some(OleDrawAspect::Content));
    assert_eq!(
        object.program_id(interner).as_deref(),
        Some("Excel.Sheet.12")
    );
    assert_eq!(object.relationship_id(interner).as_deref(), Some("rId5"));
    assert_eq!(object.object_id(interner).as_deref(), Some("_1219561732"));
    assert!(object.update_mode(interner).is_none());

    let shape = part
        .drawing()
        .shape_for_ole_object(interner, object)
        .expect("the shape the binding names");
    assert_eq!(shape.identifier(interner).as_deref(), Some("_x0000_s1026"));
}

#[test]
fn a_legacy_form_control_resolves_from_the_shape_that_points_at_it() {
    let part = parsed();
    let interner = part.interner();
    let shape = part
        .drawing()
        .shape_by_identifier(interner, "_x0000_s1027")
        .expect("the form-control shape");
    let data = shape
        .attached_object_data()
        .expect("x:ClientData on the control shape");

    assert_eq!(data.kind(interner), Some(AttachedObjectKind::RadioButton));
    assert_eq!(data.kind_wire_value(interner).as_deref(), Some("Radio"));
    assert_eq!(
        data.linked_formula(interner).as_deref(),
        Some("Sheet1!$A$1")
    );
    assert_eq!(
        data.flag(interner, "SizeWithCells"),
        Some(true),
        "a value-less ST_TrueFalseBlank child reads as true"
    );
    assert_eq!(data.flag(interner, "AutoFill"), Some(false));
    assert_eq!(data.flag(interner, "FirstButton"), Some(true));
    assert_eq!(
        data.flag(interner, "Locked"),
        None,
        "a setting the control does not state is absent, not false"
    );
    assert!(!data.hosts_embedded_control(interner));

    // A shape with no attached data answers None rather than an empty record.
    let plain = part
        .drawing()
        .shape_by_identifier(interner, "_x0000_s1026")
        .expect("the OLE fallback shape");
    assert!(plain.attached_object_data().is_none());
}

#[test]
fn ink_and_diagram_text_read_off_the_shape_that_carries_them() {
    let part = parsed();
    let interner = part.interner();
    let shape = part
        .drawing()
        .shape_by_identifier(interner, "_x0000_s1028")
        .expect("the ink shape");

    let ink = shape.ink().expect("o:ink");
    assert_eq!(ink.data(interner).as_deref(), Some("AQID"));
    assert_eq!(ink.is_annotation(interner), Some(true));
    assert_eq!(
        ink.content_type(interner).as_deref(),
        Some("application/inkml+xml")
    );

    let text = shape.diagram_text().expect("p:textdata");
    assert_eq!(text.relationship_id(interner).as_deref(), Some("rId9"));
}

#[test]
fn the_layout_header_states_which_shape_id_blocks_the_drawing_owns() {
    let part = parsed();
    let map = part
        .drawing()
        .shape_layout()
        .expect("o:shapelayout")
        .shape_id_map()
        .expect("o:idmap");
    assert_eq!(map.data(part.interner()).as_deref(), Some("1"));
}

#[test]
fn a_rebound_conventional_prefix_does_not_match_the_wrong_attribute() {
    // `o` is bound to something that is not the Office drawing namespace, so `o:spid` here does not
    // mean the Office `spid` and must not be reported as one.
    let part = DrawingPart::parse(
        br#"<xml xmlns:v="urn:schemas-microsoft-com:vml">
 <v:shape id="s1" xmlns:o="urn:example:not-office" o:spid="misleading"/>
</xml>"#,
    )
    .expect("parses");
    let shape = part
        .drawing()
        .shape_by_identifier(part.interner(), "s1")
        .expect("the shape");
    assert_eq!(
        shape.application_shape_identifier(part.interner()),
        None,
        "a rebound prefix must not be read as the Office one"
    );
    assert_eq!(
        shape.attribute(part.interner(), "id").as_deref(),
        Some("s1"),
        "the unprefixed id is unaffected"
    );
}

#[test]
fn shapes_inside_a_group_are_found_by_identifier() {
    let part = DrawingPart::parse(
        br#"<xml xmlns:v="urn:schemas-microsoft-com:vml">
 <v:group id="g1"><v:group id="g2"><v:shape id="deep" style="width:1pt"/></v:group></v:group>
</xml>"#,
    )
    .expect("parses");
    let shapes = part.drawing().all_shapes();
    assert_eq!(shapes.len(), 1, "the nested shape is found");
    assert!(
        part.drawing().shapes().next().is_none(),
        "and it is not a top-level shape"
    );
    assert!(part
        .drawing()
        .shape_by_identifier(part.interner(), "deep")
        .is_some());
}

// ---------------------------------------------------------------------------------------------
// Authoring
// ---------------------------------------------------------------------------------------------

#[test]
fn a_drawing_authored_from_nothing_reads_back_as_what_was_written() {
    let mut part = DrawingPart::new();
    {
        let (drawing, interner) = part.drawing_and_interner();
        drawing.push(DrawingContent::ShapeLayout(mjx_vml::ShapeLayout::new(
            interner, "1",
        )));

        let template = ShapeTemplate::new(interner, "_x0000_t202", "");
        drawing.push(DrawingContent::ShapeTemplate(template));

        let mut shape = Shape::new(
            interner,
            "_x0000_s1026",
            "position:absolute;width:100pt;height:50pt",
        );
        shape.set_template_identifier(interner, "_x0000_t202");
        shape.set_fill_color(interner, "#ffff00");
        shape.push(ShapeContent::AttachedObjectData(
            mjx_vml::AttachedObjectData::new(interner, AttachedObjectKind::PushButton),
        ));
        drawing.push(DrawingContent::Shape(shape));

        drawing.push(DrawingContent::EmbeddedOleObject(
            mjx_vml::EmbeddedOleObject::new(
                interner,
                "_x0000_s1026",
                "rId2",
                "Excel.Sheet.12",
                OleObjectKind::Embedded,
            ),
        ));
    }
    let bytes = part.to_bytes();

    let reparsed = DrawingPart::parse(&bytes).expect("an authored drawing parses back");
    let interner = reparsed.interner();
    let shape = reparsed
        .drawing()
        .shape_by_identifier(interner, "_x0000_s1026")
        .expect("the authored shape");
    assert_eq!(
        shape.template_identifier(interner).as_deref(),
        Some("_x0000_t202")
    );
    assert_eq!(shape.fill_color(interner).as_deref(), Some("#ffff00"));
    assert_eq!(
        shape
            .attached_object_data()
            .and_then(|data| data.kind(interner)),
        Some(AttachedObjectKind::PushButton)
    );
    let object = reparsed
        .drawing()
        .embedded_ole_objects()
        .next()
        .expect("the authored binding");
    assert_eq!(
        reparsed
            .drawing()
            .shape_for_ole_object(interner, object)
            .and_then(|shape| shape.identifier(interner))
            .as_deref(),
        Some("_x0000_s1026"),
        "the authored binding resolves to the authored shape"
    );
    assert!(
        String::from_utf8_lossy(&bytes).starts_with(r#"<?xml version="1.0""#),
        "an authored part opens with an XML declaration"
    );
}

#[test]
fn an_authored_drawing_round_trips_through_a_second_write() {
    let mut part = DrawingPart::new();
    {
        let (drawing, interner) = part.drawing_and_interner();
        let shape = Shape::new(interner, "s1", "width:10pt");
        drawing.push(DrawingContent::Shape(shape));
    }
    let first = part.to_bytes();
    let mut reparsed = DrawingPart::parse(&first).expect("parses");
    assert_eq!(
        reparsed.to_bytes(),
        first,
        "a part written, read and written again is byte-identical"
    );
}

// ---------------------------------------------------------------------------------------------
// Editing
// ---------------------------------------------------------------------------------------------

#[test]
fn a_shapes_attributes_and_control_settings_can_be_edited_in_place() {
    let mut part = parsed();
    {
        let (drawing, interner) = part.drawing_and_interner();
        let shape = drawing
            .shape_by_identifier_mut(interner, "_x0000_s1027")
            .expect("the form-control shape");
        shape.set_style(interner, "width:120pt");
        shape.set_identifier(interner, "_x0000_s2027");
        shape.remove_attribute(interner, "fillcolor");
    }
    let bytes = part.to_bytes();
    let reparsed = DrawingPart::parse(&bytes).expect("parses");
    let interner = reparsed.interner();

    let shape = reparsed
        .drawing()
        .shape_by_identifier(interner, "_x0000_s2027")
        .expect("the renamed shape");
    assert_eq!(shape.style(interner).as_deref(), Some("width:120pt"));
    assert_eq!(
        shape.fill_color(interner),
        None,
        "the attribute was removed"
    );
    assert!(
        reparsed
            .drawing()
            .shape_by_identifier(interner, "_x0000_s1027")
            .is_none(),
        "the old identifier is gone"
    );
    assert!(
        shape.attached_object_data().is_some(),
        "the control data rode along untouched"
    );
}

#[test]
fn a_control_setting_can_be_added_and_rewritten() {
    let mut part = parsed();
    {
        let (drawing, interner) = part.drawing_and_interner();
        let shape = drawing
            .shape_by_identifier_mut(interner, "_x0000_s1027")
            .expect("the form-control shape");
        let mut data = shape.attached_object_data().expect("x:ClientData").clone();
        data.set_setting(interner, "FmlaLink", "Sheet2!$B$4");
        data.set_setting(interner, "FmlaMacro", "Module1.OnClick");
        for child in shape.content_mut() {
            if let ShapeContent::AttachedObjectData(existing) = child {
                *existing = data;
                break;
            }
        }
    }
    let bytes = part.to_bytes();
    let reparsed = DrawingPart::parse(&bytes).expect("parses");
    let interner = reparsed.interner();
    let data = reparsed
        .drawing()
        .shape_by_identifier(interner, "_x0000_s1027")
        .and_then(Shape::attached_object_data)
        .expect("x:ClientData");

    assert_eq!(
        data.linked_formula(interner).as_deref(),
        Some("Sheet2!$B$4"),
        "an existing setting is rewritten in place"
    );
    assert_eq!(
        data.macro_formula(interner).as_deref(),
        Some("Module1.OnClick"),
        "a new setting is appended"
    );
    assert_eq!(
        data.flag(interner, "FirstButton"),
        Some(true),
        "the settings that were not named are untouched"
    );
}

#[test]
fn a_shape_can_be_removed_from_a_drawing() {
    let mut part = parsed();
    {
        let drawing = part.drawing_mut();
        drawing
            .content_mut()
            .retain(|child| !matches!(child, DrawingContent::Shape(_)));
    }
    let bytes = part.to_bytes();
    let reparsed = DrawingPart::parse(&bytes).expect("parses");
    assert!(
        reparsed.drawing().all_shapes().is_empty(),
        "every shape was removed"
    );
    assert!(
        reparsed.drawing().shape_layout().is_some(),
        "the layout header stayed"
    );
    assert_eq!(
        reparsed.drawing().embedded_ole_objects().count(),
        1,
        "so did the OLE binding"
    );
}

// ---------------------------------------------------------------------------------------------
// A known `mjx-xml` limitation, pinned here so it is visible rather than silent
// ---------------------------------------------------------------------------------------------

#[test]
fn attributes_wrapped_across_lines_reflow_when_the_part_is_re_serialized() {
    // The `mjx-xml` fidelity reader records each attribute's name, value and quote but **not** the
    // whitespace that separated it from the previous one, and the writer therefore emits exactly one
    // space. Office wraps a VML start tag across lines far more often than it wraps a slide's, so
    // this is where the gap shows.
    //
    // It is a `mjx-xml` gap, not a VML one, and it never breaks the round-trip contract: a part
    // nobody edits keeps its original bytes through the `mjx-opc` copy-on-write layer and is never
    // re-serialized at all. Only a part a caller deliberately edits is rewritten, and only its own
    // start tags reflow. This test states exactly that, so a fix to `mjx-xml` fails here loudly
    // rather than passing unnoticed.
    let wrapped = br#"<xml xmlns:v="urn:schemas-microsoft-com:vml">
 <v:shape id="s1"
  style="width:10pt"/>
</xml>"#;
    let mut part = DrawingPart::parse(wrapped).expect("parses");
    let written = part.to_bytes();

    assert_ne!(
        written, wrapped,
        "if this now passes, mjx-xml preserves inter-attribute whitespace — delete this test and          assert byte identity instead"
    );
    assert_eq!(
        String::from_utf8_lossy(&written),
        "<xml xmlns:v=\"urn:schemas-microsoft-com:vml\">\n <v:shape id=\"s1\" style=\"width:10pt\"/>\n</xml>",
        "the reflow collapses the wrap to one space and changes nothing else"
    );

    // What must hold regardless: the model reads back identically.
    let reparsed = DrawingPart::parse(&written).expect("parses");
    let shape = reparsed
        .drawing()
        .shape_by_identifier(reparsed.interner(), "s1")
        .expect("the shape");
    assert_eq!(
        shape.style(reparsed.interner()).as_deref(),
        Some("width:10pt")
    );
}
