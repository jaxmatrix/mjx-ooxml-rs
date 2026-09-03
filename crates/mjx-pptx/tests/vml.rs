//! Integration tests for legacy VML recognition (MJX-47, tier V1): finding the `vmlDrawingN.vml`
//! parts a package carries and reading their bytes — preserve-first, without modeling the VML XML,
//! and with fidelity (the deck round-trips byte-identically, and editing a slide leaves the VML part
//! untouched).
//!
//! Gated behind the `vml` feature. The fixture `vml.pptx` is `sample.pptx` plus a single
//! `ppt/drawings/vmlDrawing1.vml` part, related from slide 1 (`rId2`, type `vmlDrawing`) and
//! registered via a `vml` content-type Default. It is hand-crafted (see MJX-140 for producer-authentic
//! validation).
#![cfg(feature = "vml")]

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_opc::{Package, PartName};
use mjx_pptx::{PptxError, Presentation};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

fn part(name: &str) -> PartName {
    PartName::new(name).expect("valid part name")
}

/// Every part tied to the VML drawing that must survive an edit made elsewhere byte-for-byte.
const VML_PARTS: &[&str] = &[
    "ppt/drawings/vmlDrawing1.vml",
    "ppt/slides/_rels/slide1.xml.rels",
];

#[test]
fn vml_part_names_lists_the_drawing() {
    let pres = Presentation::open(&fixture("vml.pptx")).expect("open");
    assert_eq!(
        pres.vml_part_names(),
        vec![part("/ppt/drawings/vmlDrawing1.vml")],
        "the sole VML drawing is recognized by its content type"
    );

    // A deck with no VML answers with an empty list, not an error.
    let plain = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert!(
        plain.vml_part_names().is_empty(),
        "a deck without VML has no VML parts"
    );
}

#[test]
fn vml_part_bytes_resolves_to_the_verbatim_part() {
    let bytes = fixture("vml.pptx");
    let baseline = Package::open(&bytes).expect("baseline");
    let vml_xml = baseline
        .part_bytes(&part("/ppt/drawings/vmlDrawing1.vml"))
        .expect("fixture has a VML part")
        .to_vec();

    let pres = Presentation::open(&bytes).expect("open");
    let names = pres.vml_part_names();
    let name = names.first().expect("one VML part");
    assert_eq!(
        pres.vml_part_bytes(name),
        Some(vml_xml.as_slice()),
        "the resolved bytes are exactly the package's VML part"
    );

    // An absent part answers None.
    assert_eq!(pres.vml_part_bytes(&part("/ppt/drawings/nope.vml")), None);
}

#[test]
fn reading_vml_leaves_every_part_byte_identical() {
    let bytes = fixture("vml.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let pres = Presentation::open(&bytes).expect("open");
    // Exercise every read accessor; none may dirty a part.
    for name in pres.vml_part_names() {
        pres.vml_part_bytes(&name).expect("bytes");
    }

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(reopened, original, "reading VML must dirty nothing");
}

#[test]
fn editing_a_slide_leaves_the_vml_part_byte_identical() {
    let bytes = fixture("vml.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Edit the title text — the slide XML changes, but the separate VML part must not.
    pres.set_shape_text(0, 0, 0, "Edited").expect("set text");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    for &name in VML_PARTS {
        assert_eq!(
            reopened.get(name),
            original.get(name),
            "VML part {name} must be untouched by an edit elsewhere"
        );
    }
    assert_ne!(
        reopened.get("ppt/slides/slide1.xml"),
        original.get("ppt/slides/slide1.xml"),
        "the edited slide should have changed"
    );
}

// ---------------------------------------------------------------------------------------------
// MJX-140 — the typed VML model, and resolving the shape an OLE object or a control points at
// ---------------------------------------------------------------------------------------------

/// A VML drawing shaped like the fallback PowerPoint writes beside an OLE object: a layout header, a
/// shape template, and two `v:shape`s — a decoy that comes **first** in document order, and the one
/// whose `id` is the `spid` the `ole.pptx` fixture's `p:oleObj` names.
///
/// The decoy is what makes the resolution tests mean something: a lookup that ignored the identifier
/// and answered "the first shape" would pass against a one-shape drawing.
const OLE_FALLBACK_DRAWING: &[u8] = br##"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<xml xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:x="urn:schemas-microsoft-com:office:excel" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
 <o:shapelayout v:ext="edit"><o:idmap v:ext="edit" data="1"/></o:shapelayout>
 <v:shapetype id="_x0000_t75" coordsize="21600,21600" o:spt="75" filled="f" stroked="f"><v:stroke joinstyle="miter"/></v:shapetype>
 <v:shape id="_x0000_s1025" type="#_x0000_t75" style="position:absolute;width:10pt;height:10pt" alt="A different shape entirely">
  <v:imagedata r:id="rId9" o:title="Decoy"/>
 </v:shape>
 <v:shape id="_x0000_s1026" type="#_x0000_t75" style="position:absolute;width:240pt;height:180pt" o:ole="" alt="Worksheet" fillcolor="#ffffff">
  <v:imagedata r:id="rId3" o:title="Worksheet"/>
  <x:ClientData ObjectType="Pict"><x:SizeWithCells/><x:CF>Bitmap</x:CF></x:ClientData>
 </v:shape>
</xml>
"##;

#[test]
fn a_vml_part_reads_as_a_typed_drawing() {
    let mut pres = Presentation::open(&fixture("vml.pptx")).expect("open");
    let name = part("/ppt/drawings/vmlDrawing1.vml");

    let summary = pres
        .with_vml_drawing(&name, |drawing, interner| {
            let shape = drawing
                .shape_by_identifier(interner, "_x0000_s1026")
                .expect("the fixture's shape");
            (
                drawing.all_shapes().len(),
                drawing.shape_templates().count(),
                drawing
                    .shape_layout()
                    .and_then(mjx_vml::ShapeLayout::shape_id_map)
                    .and_then(|map| map.data(interner)),
                shape.template_identifier(interner),
                shape.is_filled(interner),
                shape.style(interner),
            )
        })
        .expect("read drawing");

    assert_eq!(summary.0, 1, "one shape");
    assert_eq!(summary.1, 1, "one shape template");
    assert_eq!(summary.2.as_deref(), Some("1"), "the shape-id block");
    assert_eq!(summary.3.as_deref(), Some("_x0000_t202"));
    assert_eq!(summary.4, Some(false));
    assert!(summary
        .5
        .as_deref()
        .is_some_and(|style| style.contains("margin-left:10pt")));
}

#[test]
fn a_part_that_is_not_a_vml_drawing_is_refused() {
    let mut pres = Presentation::open(&fixture("vml.pptx")).expect("open");
    let slide = part("/ppt/slides/slide1.xml");
    assert!(matches!(
        pres.with_vml_drawing(&slide, |_, _| ()),
        Err(PptxError::PartIsNotVmlDrawing { .. })
    ));
}

#[test]
fn the_vml_drawing_a_surface_relates_to_is_found() {
    let pres = Presentation::open(&fixture("vml.pptx")).expect("open");
    assert_eq!(
        pres.vml_drawing_part(0).expect("part"),
        Some(part("/ppt/drawings/vmlDrawing1.vml"))
    );

    // A deck with no VML has none.
    let plain = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert_eq!(plain.vml_drawing_part(0).expect("part"), None);
}

#[test]
fn an_ole_object_resolves_to_the_vml_shape_that_draws_it() {
    // `ole.pptx` carries `p:oleObj@spid="_x0000_s1026"` but no VML part; attach the fallback drawing
    // that spid names and the whole hop resolves.
    let mut pres = Presentation::open(&fixture("ole.pptx")).expect("open");
    assert_eq!(
        pres.with_vml_shape_for_ole_object(0, 1, |_, _| ())
            .expect("resolve"),
        None,
        "without a VML drawing the spid resolves to nothing"
    );

    pres.add_vml_drawing(0, OLE_FALLBACK_DRAWING)
        .expect("attach the fallback drawing");

    let found = pres
        .with_vml_shape_for_ole_object(0, 1, |shape, interner| {
            (
                shape.identifier(interner),
                shape.alternate_text(interner),
                shape.is_embedded_object(interner),
                shape
                    .image_data()
                    .and_then(|image| image.relationship_id(interner)),
                shape
                    .attached_object_data()
                    .and_then(|data| data.kind(interner)),
            )
        })
        .expect("resolve")
        .expect("the spid names a shape in the drawing");

    assert_eq!(found.0.as_deref(), Some("_x0000_s1026"));
    assert_eq!(found.1.as_deref(), Some("Worksheet"));
    assert_eq!(found.2, Some(true), "the fallback shape is marked o:ole");
    assert_eq!(
        found.3.as_deref(),
        Some("rId3"),
        "and it names the image the OLE frame's snapshot uses"
    );
    assert_eq!(found.4, Some(mjx_vml::AttachedObjectKind::Image));

    // A shape that frames no OLE object resolves to nothing rather than the wrong shape.
    assert_eq!(
        pres.with_vml_shape_for_ole_object(0, 0, |_, _| ())
            .expect("resolve"),
        None
    );

    // And a spid naming a shape the drawing does not hold resolves to nothing, rather than to
    // whichever shape happens to come first.
    pres.set_ole_legacy_shape_id(0, 1, "_x0000_s9999")
        .expect("rebind to a shape that is not there");
    assert_eq!(
        pres.with_vml_shape_for_ole_object(0, 1, |_, _| ())
            .expect("resolve"),
        None
    );
}

#[test]
fn an_activex_control_resolves_to_the_vml_shape_that_draws_it() {
    let mut pres = Presentation::open(&fixture("activex.pptx")).expect("open");
    pres.add_vml_drawing(0, OLE_FALLBACK_DRAWING)
        .expect("attach the fallback drawing");

    let identifier = pres
        .with_vml_shape_for_activex_control(0, 0, |shape, interner| shape.identifier(interner))
        .expect("resolve")
        .expect("the control's spid names a shape");
    assert_eq!(
        identifier.as_deref(),
        Some("_x0000_s1026"),
        "p:control@spid resolves the same way p:oleObj@spid does — and past the decoy shape that \
         comes first in the drawing"
    );

    // A control index past the end resolves to nothing.
    assert_eq!(
        pres.with_vml_shape_for_activex_control(0, 4, |_, _| ())
            .expect("resolve"),
        None
    );
}

#[test]
fn an_added_vml_drawing_registers_its_content_type_and_relationship() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let name = pres
        .add_vml_drawing(0, OLE_FALLBACK_DRAWING)
        .expect("add drawing");
    assert_eq!(name, part("/ppt/drawings/vmlDrawing1.vml"));

    let saved = pres.save().expect("save");
    let pkg = Package::open(&saved).expect("reopen");
    assert_eq!(
        pkg.content_type_of(&name),
        Some(mjx_vml::CONTENT_TYPE_VML),
        "the vml extension resolves through a Default"
    );
    let content_types = String::from_utf8_lossy(
        pkg.entries()
            .iter()
            .find(|e| e.name == "[Content_Types].xml")
            .and_then(mjx_opc::ZipEntry::bytes)
            .expect("content types"),
    );
    assert!(
        content_types.contains(r#"Extension="vml""#),
        "registered as a Default, not a per-part Override: {content_types}"
    );
    assert!(
        !content_types.contains("/ppt/drawings/vmlDrawing1.vml"),
        "so no Override is written for it"
    );

    let reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened.vml_drawing_part(0).expect("part"),
        Some(name.clone())
    );
    assert_eq!(reopened.vml_part_bytes(&name), Some(OLE_FALLBACK_DRAWING));

    // A second drawing does not collide with the first.
    let second = pres.add_vml_drawing(0, OLE_FALLBACK_DRAWING).expect("add");
    assert_eq!(second, part("/ppt/drawings/vmlDrawing2.vml"));
}

#[test]
fn adding_a_vml_drawing_that_is_not_xml_is_refused_before_anything_is_written() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let original = byte_map(&Package::open(&fixture("sample.pptx")).expect("baseline"));

    assert!(matches!(
        pres.add_vml_drawing(0, b"<xml><v:shape"),
        Err(PptxError::Xml(_))
    ));

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(
        reopened, original,
        "a refused add must leave the package exactly as it was"
    );
}

#[test]
fn editing_a_vml_shape_dirties_only_the_vml_part() {
    let bytes = fixture("vml.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");
    let name = part("/ppt/drawings/vmlDrawing1.vml");

    let renamed = pres
        .edit_vml_drawing(&name, |drawing, interner| {
            let shape = drawing
                .shape_by_identifier_mut(interner, "_x0000_s1026")
                .expect("the fixture's shape");
            shape.set_fill_color(interner, "#123456");
            shape.set_alternate_text(interner, "Legacy fallback");
            shape.identifier(interner)
        })
        .expect("edit");
    assert_eq!(renamed.as_deref(), Some("_x0000_s1026"));

    let saved = pres.save().expect("save");
    let after = byte_map(&Package::open(&saved).expect("reopen"));

    let vml = String::from_utf8_lossy(after.get("ppt/drawings/vmlDrawing1.vml").expect("vml"));
    assert!(vml.contains(r##"fillcolor="#123456""##), "got:\n{vml}");
    assert!(vml.contains(r#"alt="Legacy fallback""#));
    assert!(
        vml.contains(r#"<v:shapetype id="_x0000_t202""#) && vml.contains("<v:textbox/>"),
        "everything the edit did not name survives, got:\n{vml}"
    );

    for (name, bytes) in &original {
        if name == "ppt/drawings/vmlDrawing1.vml" {
            continue;
        }
        assert_eq!(
            after.get(name),
            Some(bytes),
            "{name} must be byte-identical after a VML edit — the slide included"
        );
    }
    assert_eq!(after.len(), original.len(), "no part was added or removed");

    // The edit reads back through the model.
    let mut reopened = Presentation::open(&saved).expect("reopen");
    let colour = reopened
        .with_vml_drawing(&name, |drawing, interner| {
            drawing
                .shape_by_identifier(interner, "_x0000_s1026")
                .and_then(|shape| shape.fill_color(interner))
        })
        .expect("read");
    assert_eq!(colour.as_deref(), Some("#123456"));
}

#[test]
fn reading_a_vml_drawing_through_the_model_dirties_nothing() {
    let bytes = fixture("vml.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");
    for name in pres.vml_part_names() {
        pres.with_vml_drawing(&name, |drawing, interner| {
            for shape in drawing.all_shapes() {
                let _ = shape.identifier(interner);
                let _ = shape.attached_object_data();
            }
        })
        .expect("read");
    }
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(
        reopened, original,
        "reading a VML drawing through the model must dirty nothing"
    );
}

// -------------------------------------------------------------------------------------------
// Tier 3, inside a part: an edit through the whole-part typed model rewrites only what it changed
// -------------------------------------------------------------------------------------------

/// The bytes of `<open … </close>` in `haystack`, start tag through end tag.
fn subtree_bytes<'a>(haystack: &'a [u8], open: &str, close: &str) -> &'a [u8] {
    let text = std::str::from_utf8(haystack).expect("the fixture part is UTF-8");
    let start = text
        .find(open)
        .unwrap_or_else(|| panic!("no {open} in the part"));
    let end = text
        .find(close)
        .unwrap_or_else(|| panic!("no {close} in the part"))
        + close.len();
    &haystack[start..end]
}

/// `edit_vml_drawing` reads the whole part into a typed model and writes the whole part back from
/// it. Until MJXOFF-143 that rebuilt every element, so the part came back re-flowed even where
/// nothing had changed. Now each element the model reproduced unchanged is copied out of the buffer
/// the part was parsed from.
///
/// The fixture discriminates: `vmlDrawing1.vml` wraps its start tags across lines **with CRLF**, so
/// no reconstruction from the model could produce these bytes — the writer emits exactly one `0x20`
/// between attributes. A part whose start tags were already on one line would pass this test with
/// the mechanism deleted.
#[test]
fn editing_one_vml_shape_leaves_every_other_element_byte_identical() {
    let bytes = fixture("vml.pptx");
    let name = part("/ppt/drawings/vmlDrawing1.vml");
    let before = Package::open(&bytes)
        .expect("baseline")
        .part_bytes(&name)
        .expect("the drawing part")
        .to_vec();
    assert!(
        before.windows(2).any(|w| w == b"\r\n"),
        "the fixture must wrap its start tags, or this test proves nothing"
    );

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.edit_vml_drawing(&name, |drawing, interner| {
        let shape = drawing
            .shape_by_identifier_mut(interner, "_x0000_s1026")
            .expect("the fixture's shape");
        shape.set_fill_color(interner, "#123456");
    })
    .expect("edit");
    let saved = pres.save().expect("save");
    let after = Package::open(&saved)
        .expect("reopen")
        .part_bytes(&name)
        .expect("the drawing part")
        .to_vec();

    assert_ne!(after, before, "the edit should have changed the part");
    let contains = |haystack: &[u8], needle: &[u8]| {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    };
    assert!(
        contains(&after, br##"fillcolor="#123456""##),
        "the edit did not land:\n{}",
        String::from_utf8_lossy(&after)
    );

    // Every element outside the path from the root to the edited shape is *literally* its original
    // bytes — CRLF wrapping, two-space indent and all.
    for (open, close) in [
        ("<o:shapelayout ", "</o:shapelayout>"),
        ("<v:shapetype ", "</v:shapetype>"),
    ] {
        let untouched = subtree_bytes(&before, open, close);
        assert!(
            contains(&after, untouched),
            "{open} was re-flowed rather than copied:\n{}\n\nwanted verbatim:\n{}",
            String::from_utf8_lossy(&after),
            String::from_utf8_lossy(untouched)
        );
        assert!(
            contains(untouched, b"\r\n"),
            "{open} is not wrapped in the fixture, so asserting on it proves nothing"
        );
    }

    // The edited shape itself is the one thing rebuilt from the model, so its own start tag is now
    // on one line — the reflow is confined to what actually changed.
    assert!(
        contains(&after, br##"<v:shape id="_x0000_s1026" type="#_x0000_t202" style="position:absolute;margin-left:10pt;margin-top:10pt;width:100pt;height:50pt" filled="f" stroked="f" fillcolor="#123456">"##),
        "the edited element should reconstruct from the model:\n{}",
        String::from_utf8_lossy(&after)
    );
}
