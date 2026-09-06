//! MJXOFF-107 (E3) — `CT_Worksheet`'s last three owned slots: `drawing` (29), `oleObjects` (34) and
//! `controls` (35).
//!
//! # The non-discriminating test this file exists to avoid
//!
//! Reading a slot back through the same accessor that wrote it passes against a frame that models
//! nothing and hands back the element it was given. So every case here does one of two things
//! instead: it parses markup written out in full, with the three slots **out of order in the source
//! but at their schema ranks after an edit**; or it asserts on a slot's neighbours, which a frame
//! that placed a child at the wrong rank could not leave alone.
//!
//! The `oleObjects` and `controls` cases also carry MJXOFF-127's trap: **six of `CT_ObjectPr`'s and
//! `CT_ControlPr`'s boolean attributes default to `true`**, so an element that writes none is
//! asserted to answer `true` six times, and the element beside it writes `0` for each so the six
//! `true`s cannot be a constant.

use mjx_ooxml_core::Interner;
use mjx_sml::{
    EmbeddedObject, EmbeddedObjects, FormControl, FormControlProperties, FormControls,
    ObjectAnchor, ObjectProperties, WorksheetPart,
};

const SML: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const XDR: &str = "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing";

/// A worksheet carrying all three slots, plus the two neighbours that bracket them.
fn sheet_with_objects() -> String {
    format!(
        r#"<worksheet xmlns="{SML}" xmlns:r="{R}" xmlns:xdr="{XDR}">
  <sheetData/>
  <pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>
  <drawing r:id="rId3"/>
  <legacyDrawing r:id="rId4"/>
  <picture r:id="rId5"/>
  <oleObjects>
    <oleObject progId="Package" shapeId="1025" r:id="rId6">
      <objectPr defaultSize="0" autoFill="0" autoLine="0" autoPict="0" locked="0" print="0" r:id="rId7">
        <anchor moveWithCells="1">
          <xdr:from><xdr:col>2</xdr:col><xdr:colOff>19050</xdr:colOff><xdr:row>4</xdr:row><xdr:rowOff>38100</xdr:rowOff></xdr:from>
          <xdr:to><xdr:col>5</xdr:col><xdr:colOff>57150</xdr:colOff><xdr:row>9</xdr:row><xdr:rowOff>76200</xdr:rowOff></xdr:to>
        </anchor>
      </objectPr>
    </oleObject>
    <oleObject shapeId="1026" r:id="rId8">
      <objectPr>
        <anchor>
          <xdr:from><xdr:col>0</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>0</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>
          <xdr:to><xdr:col>1</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>
        </anchor>
      </objectPr>
    </oleObject>
  </oleObjects>
  <controls>
    <control shapeId="2049" r:id="rId9" name="Check Box 1">
      <controlPr recalcAlways="1" linkedCell="$B$2" listFillRange="$D$1:$D$5" cf="pict">
        <anchor sizeWithCells="1">
          <xdr:from><xdr:col>7</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>
          <xdr:to><xdr:col>9</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>3</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>
        </anchor>
      </controlPr>
    </control>
  </controls>
</worksheet>"#
    )
}

fn read(markup: &str) -> WorksheetPart {
    WorksheetPart::read_part(markup.as_bytes())
        .expect("it parses")
        .expect("its root is an x:worksheet")
}

#[test]
fn the_three_slots_are_modelled_and_the_two_that_stay_raw_are_still_beside_them() {
    let markup = sheet_with_objects();
    let part = read(&markup);
    let interner = part.interner();

    let drawing = part.drawing().expect("rank 29 is modelled");
    assert_eq!(
        drawing.relationship_id(interner, Some("r")).ok().flatten(),
        Some("rId3".to_owned())
    );

    let objects = part.embedded_objects().expect("rank 34 is modelled");
    assert_eq!(objects.len(), 2);

    let controls = part.form_controls().expect("rank 35 is modelled");
    assert_eq!(controls.len(), 1);

    // `legacyDrawing` (30) is MJXOFF-114's and is still held raw — this child did not take it.
    let locals: Vec<&str> = part.child_element_locals().collect();
    assert_eq!(
        locals,
        vec![
            "sheetData",
            "pageMargins",
            "drawing",
            "legacyDrawing",
            "picture",
            "oleObjects",
            "controls"
        ]
    );

    // …and the whole part still re-emits byte for byte, because nothing was edited.
    assert!(part.is_verbatim());
    assert_eq!(String::from_utf8(part.to_markup()).expect("utf-8"), markup);
}

#[test]
fn the_true_defaulting_flag_family_is_reported_the_right_way_round() {
    let part = read(&sheet_with_objects());
    let interner = part.interner();
    let objects = part.embedded_objects().expect("oleObjects");
    let entries: Vec<&EmbeddedObject> = objects.entries().collect();

    // The second entry writes **no** flag at all, and six of `CT_ObjectPr`'s nine default to `true`.
    let bare = entries[1].properties().expect("objectPr");
    for (name, value) in [
        ("locked", bare.is_locked(interner).unwrap_or(false)),
        (
            "defaultSize",
            bare.has_default_size(interner).unwrap_or(false),
        ),
        ("print", bare.is_printed(interner).unwrap_or(false)),
        (
            "autoFill",
            bare.fills_automatically(interner).unwrap_or(false),
        ),
        (
            "autoLine",
            bare.outlines_automatically(interner).unwrap_or(false),
        ),
        (
            "autoPict",
            bare.scales_picture_automatically(interner).unwrap_or(false),
        ),
    ] {
        assert!(
            value,
            "@{name} defaults to true and the element writes none"
        );
    }
    // …and the three that default to `false` still do, so the six above are not a blanket `true`.
    assert!(!bare.is_disabled(interner).unwrap_or(false));
    assert!(!bare.is_user_interface_object(interner).unwrap_or(false));
    assert!(!bare.is_dynamic_data_exchange(interner).unwrap_or(false));

    // The first entry writes `0` for the same six, so the answers above cannot be constants.
    let written = entries[0].properties().expect("objectPr");
    assert!(!written.is_locked(interner).unwrap_or(false));
    assert!(!written.has_default_size(interner).unwrap_or(false));
    assert!(!written.is_printed(interner).unwrap_or(false));
    assert!(!written.fills_automatically(interner).unwrap_or(false));
    assert!(!written.outlines_automatically(interner).unwrap_or(false));
    assert!(!written
        .scales_picture_automatically(interner)
        .unwrap_or(false));
}

#[test]
fn a_control_carries_the_same_six_plus_recalc_always_and_its_three_own_attributes() {
    let part = read(&sheet_with_objects());
    let interner = part.interner();
    let controls = part.form_controls().expect("controls");
    let control = controls.entries().next().expect("one control");

    assert_eq!(control.shape_id(interner).ok(), Some(2049));
    assert_eq!(
        control.control_name(interner).ok().flatten().as_deref(),
        Some("Check Box 1")
    );
    assert_eq!(
        control.relationship_id(interner, Some("r")).ok().flatten(),
        Some("rId9".to_owned())
    );

    let properties = control.properties().expect("controlPr");
    // `@recalcAlways` is the one boolean `CT_ControlPr` adds, and it defaults to `false` — so the
    // written `1` here is not the family default.
    assert!(properties.recalculates_always(interner).unwrap_or(false));
    assert!(
        properties.is_locked(interner).unwrap_or(false),
        "@locked still defaults to true"
    );
    assert_eq!(
        properties
            .linked_cell_formula(interner)
            .ok()
            .flatten()
            .as_deref(),
        Some("$B$2")
    );
    assert_eq!(
        properties
            .list_fill_range_formula(interner)
            .ok()
            .flatten()
            .as_deref(),
        Some("$D$1:$D$5")
    );
    assert_eq!(
        properties
            .clipboard_format(interner)
            .ok()
            .flatten()
            .as_deref(),
        Some("pict")
    );

    // The anchor is MJXOFF-127's shared type, and its two markers are still the `xdr:` elements the
    // file wrote — this crate decodes neither, and `mjx_dml::CellMarker` is what does.
    let anchor = control.anchor().expect("anchor");
    assert!(anchor.sizes_with_cells(interner).unwrap_or(false));
    assert!(!anchor.moves_with_cells(interner).unwrap_or(false));
    let marker = anchor.from_marker(interner).expect("xdr:from");
    assert_eq!(interner.resolve(marker.name.local), "from");
}

#[test]
fn an_ole_objects_entry_reports_its_own_identity_and_the_entry_beside_it_reports_a_different_one() {
    let part = read(&sheet_with_objects());
    let interner = part.interner();
    let objects = part.embedded_objects().expect("oleObjects");
    let entries: Vec<&EmbeddedObject> = objects.entries().collect();

    assert_eq!(entries[0].shape_id(interner).ok(), Some(1025));
    assert_eq!(
        entries[0].program_id(interner).ok().flatten().as_deref(),
        Some("Package")
    );
    assert_eq!(
        entries[0]
            .relationship_id(interner, Some("r"))
            .ok()
            .flatten(),
        Some("rId6".to_owned())
    );

    assert_eq!(entries[1].shape_id(interner).ok(), Some(1026));
    assert_eq!(entries[1].program_id(interner).ok().flatten(), None);
    assert_eq!(
        entries[1]
            .relationship_id(interner, Some("r"))
            .ok()
            .flatten(),
        Some("rId8".to_owned())
    );

    // `@dvAspect` defaults to `DVASPECT_CONTENT`, which neither entry writes.
    use mjx_ooxml_types::spreadsheetml::DataViewAspect;
    assert_eq!(
        entries[0].view_aspect(interner).ok(),
        Some(DataViewAspect::Content)
    );
}

#[test]
fn an_authored_sheet_places_the_three_slots_at_their_schema_ranks() {
    // Authored in the reverse of schema order, so a frame that appended rather than placed would
    // write `controls` before `oleObjects` before `drawing` and this case would fail.
    let mut part = WorksheetPart::authored(None);
    let interner = part.interner_mut();
    let mut controls = FormControls::new(interner, None);
    let mut control = FormControl::new(interner, None, 42);
    let mut properties = FormControlProperties::new(interner, None);
    properties.set_anchor(ObjectAnchor::new(interner, None));
    control.set_properties(properties);
    controls.push(control);
    part.set_form_controls(Some(controls));

    let interner = part.interner_mut();
    let mut objects = EmbeddedObjects::new(interner, None);
    let mut object = EmbeddedObject::new(interner, None, 7);
    let mut object_properties = ObjectProperties::new(interner, None);
    object_properties.set_anchor(ObjectAnchor::new(interner, None));
    object.set_properties(object_properties);
    objects.push(object);
    part.set_embedded_objects(Some(objects));

    let locals: Vec<&str> = part.child_element_locals().collect();
    assert_eq!(
        locals,
        vec!["oleObjects", "controls"],
        "rank 34 must land before rank 35 whatever order the caller wrote them in"
    );

    // …and a `picture` at rank 33 inserted last still lands before both.
    let markup = String::from_utf8(part.to_markup()).expect("utf-8");
    assert!(
        markup.find("<oleObjects").unwrap_or(usize::MAX)
            < markup.find("<controls").unwrap_or(usize::MAX)
    );
}

#[test]
fn setting_a_slot_leaves_every_other_slot_writing_from_its_own_bytes() {
    let markup = sheet_with_objects();
    let mut part = read(&markup);

    // Replace the drawing relationship. Everything else must come back exactly as the file wrote it,
    // including the `oleObjects` block's own attribute order and the `xdr:` prefixes inside it.
    let prefix = part.bind_relationship_prefix();
    let mut interner = Interner::default();
    core::mem::swap(&mut interner, part.interner_mut());
    if let Some(drawing) = part.drawing_mut() {
        drawing.set_relationship_id(&mut interner, &prefix, "rId99");
    }
    core::mem::swap(&mut interner, part.interner_mut());

    let rendered = String::from_utf8(part.to_markup()).expect("utf-8");
    assert!(rendered.contains(r#"<drawing r:id="rId99"/>"#));
    assert!(
        rendered.contains(r#"<oleObject progId="Package" shapeId="1025" r:id="rId6">"#),
        "an untouched slot must re-emit from its own bytes:\n{rendered}"
    );
    assert!(rendered.contains(r#"<control shapeId="2049" r:id="rId9" name="Check Box 1">"#));
    assert!(rendered.contains(r#"<legacyDrawing r:id="rId4"/>"#));
    assert!(rendered.contains(
        r#"<xdr:from><xdr:col>7</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>"#
    ));
}

#[test]
fn removing_the_last_entry_of_a_list_is_the_callers_decision_not_a_silent_repair() {
    let part_markup = sheet_with_objects();
    let mut part = read(&part_markup);
    let mut objects = part.embedded_objects().expect("oleObjects").clone();
    assert!(objects.remove(0).is_some());
    assert_eq!(objects.len(), 1);
    assert!(objects.remove(0).is_some());
    assert!(
        objects.is_empty(),
        "an empty list is what the caller asked for"
    );
    assert!(objects.remove(0).is_none());
    part.set_embedded_objects(None);
    let locals: Vec<&str> = part.child_element_locals().collect();
    assert!(!locals.contains(&"oleObjects"));
    assert!(locals.contains(&"controls"), "the neighbour is untouched");
}
