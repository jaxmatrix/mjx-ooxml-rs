//! The content a `.pptx` carries that is not a DrawingML shape: OLE objects, ActiveX controls, ink,
//! SmartArt diagrams — and, behind the `vml` feature, the legacy VML that draws the first two in a
//! consumer that will not run them.
//!
//! ```sh
//! cargo run -p mjx-pptx --example legacy_content -- out.pptx
//! cargo run -p mjx-pptx --features vml --example legacy_content -- out.pptx
//! ```
//!
//! The pattern to take away is the same for all five: the *thing* lives in its own part, the slide
//! points at it by relationship id, and the interesting question is always "which shape is this?".
//! Every reader here answers that question in one call.

use anyhow::{Context, Result};
use mjx_pptx::{
    default_placeholder_ole, ActiveXControlSpec, DiagramContent, DiagramPartKind, OleObjectSpec,
    Presentation, ShapeBounds, DEFAULT_PLACEHOLDER_IMAGE,
};

mod support;

/// A minimal but real InkML document: one trace of three points.
const INK_STROKES: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<inkml:ink xmlns:inkml="http://www.w3.org/2003/InkML"><inkml:trace>0 0, 5 9, 11 3</inkml:trace></inkml:ink>"#;

/// The Forms 2.0 command button, the control PowerPoint's toolbox inserts most often.
const COMMAND_BUTTON: &str = "{D7053240-CE69-11CD-A777-00DD01143C57}";

fn main() -> Result<()> {
    let out = support::output_path("legacy_content.pptx");
    let mut deck = Presentation::open(&support::template()?)?;
    let slide = deck.add_slide_from_layout(2)?;

    // ---- A SmartArt diagram ----------------------------------------------------------------
    // Four documents make a diagram: the data model, the layout definition, the quick style and the
    // colour transform. `vertical_list` generates all four; `from_parts` takes four of your own.
    let diagram = deck.add_diagram(
        slide,
        &DiagramContent::vertical_list(&["Plan", "Build", "Ship"]),
        ShapeBounds::from_inches(0.5, 1.0, 3.5, 3.0),
    )?;
    let parts = deck
        .diagram_parts(slide, diagram)?
        .context("the frame we just wrote should frame a diagram")?;
    println!(
        "diagram is shape {diagram}, made of {} parts:",
        parts.all().len()
    );
    for part in parts.all() {
        println!("  {}", part.as_str());
    }

    // Re-labelling a diagram replaces one part. Nothing else in the deck changes.
    deck.set_diagram_part(
        slide,
        diagram,
        DiagramPartKind::Data,
        DiagramContent::vertical_list(&["Plan", "Build", "Ship", "Measure"]).data,
    )?;
    println!("  relabelled by replacing only the data part");

    // ---- An OLE object ---------------------------------------------------------------------
    // An OLE object is never executed by a consumer: it is drawn from its snapshot image, and its
    // data is opened only when a user activates it. So the snapshot is the required half.
    let payload = default_placeholder_ole();
    let ole = deck.add_ole_object(
        slide,
        &OleObjectSpec::embedded_stream("Excel.Sheet.12", &payload, DEFAULT_PLACEHOLDER_IMAGE)
            .named("Quarterly figures"),
        ShapeBounds::from_inches(4.5, 1.0, 4.0, 2.5),
    )?;
    for object in deck.ole_objects(slide)? {
        println!(
            "OLE object at shape {}: progId {:?}, external {}",
            object.shape_index, object.prog_id, object.external
        );
    }
    deck.set_ole_prog_id(slide, ole, "Excel.Sheet.12")?;

    // ---- An ActiveX control ----------------------------------------------------------------
    // A control is *not* a shape: it lives beside the shape tree, in its own per-slide index space.
    let control = deck.add_activex_control(
        slide,
        &ActiveXControlSpec::new(
            "CommandButton1",
            COMMAND_BUTTON,
            b"persisted control state",
            DEFAULT_PLACEHOLDER_IMAGE,
        ),
        ShapeBounds::from_inches(4.5, 4.0, 2.0, 0.5),
    )?;
    println!(
        "control {control} of {}: {:?}, class {:?}, {} bytes of state",
        deck.activex_control_count(slide)?,
        deck.activex_control_name(slide, control)?,
        deck.activex_class_id(slide, control)?,
        deck.activex_binary_bytes(slide, control)?
            .map_or(0, <[u8]>::len),
    );
    deck.set_activex_control_name(slide, control, "OkButton")?;

    // ---- Ink -------------------------------------------------------------------------------
    // Ink is the one that used to be untraceable: the part was findable, but nothing said which
    // shape it belonged to. `ink_references` answers that, and it answers it both ways.
    let ink = deck.add_ink(slide, INK_STROKES)?;
    for reference in deck.ink_references(slide)? {
        println!(
            "ink {:?} belongs to shape {:?}",
            reference.part.as_ref().map(mjx_opc::PartName::as_str),
            reference.shape_index
        );
    }
    let ink_part = deck
        .ink_part_for_shape(slide, ink)?
        .context("the ink we just added should resolve to a part")?;
    println!(
        "  and shape {:?} is where {} came from",
        deck.shape_for_ink_part(slide, &ink_part)?,
        ink_part.as_str()
    );

    // ---- Legacy VML ------------------------------------------------------------------------
    legacy_vml(&mut deck, slide, ole)?;

    let bytes = deck.save()?;
    std::fs::write(&out, &bytes)?;
    println!("wrote {} ({} bytes)", out.display(), bytes.len());

    // Prove the deck reopens and still answers the same questions.
    let mut reopened = Presentation::open(&bytes)?;
    assert_eq!(reopened.ole_objects(slide)?.len(), 1);
    assert_eq!(reopened.activex_control_count(slide)?, 1);
    assert_eq!(reopened.ink_references(slide)?.len(), 1);
    assert!(reopened.diagram_parts(slide, diagram)?.is_some());
    println!("reopened and verified");
    Ok(())
}

/// Attaches the legacy VML fallback and resolves it back from the OLE object that points at it.
///
/// `p:oleObj@spid` names a VML shape's `id`; this is that hop. Compiled only with the `vml` feature,
/// which is what gates the whole surface.
#[cfg(feature = "vml")]
fn legacy_vml(deck: &mut Presentation, slide: usize, ole: usize) -> Result<()> {
    use mjx_vml::{DrawingContent, DrawingPart, Shape, ShapeLayout};

    // The `spid` an OLE frame names is the id its fallback shape must carry. Bind the frame to one,
    // then author the shape it points at.
    let identifier = "_x0000_s1026";
    deck.set_ole_legacy_shape_id(slide, ole, identifier)?;

    let mut drawing = DrawingPart::new();
    {
        let (model, interner) = drawing.drawing_and_interner();
        model.push(DrawingContent::ShapeLayout(ShapeLayout::new(interner, "1")));
        let mut shape = Shape::new(
            interner,
            identifier,
            "position:absolute;margin-left:324pt;margin-top:72pt;width:288pt;height:180pt",
        );
        shape.set_alternate_text(interner, "Quarterly figures");
        model.push(DrawingContent::Shape(shape));
    }
    let part = deck.add_vml_drawing(slide, &drawing.to_bytes())?;
    println!("VML fallback written to {}", part.as_str());

    // Editing one shape leaves the rest of the drawing exactly as it was.
    deck.edit_vml_drawing(&part, |model, interner| {
        if let Some(shape) = model.shape_by_identifier_mut(interner, identifier) {
            shape.set_fill_color(interner, "#dce6f1");
        }
    })?;

    let found = deck
        .with_vml_shape_for_ole_object(slide, ole, |shape, interner| {
            (shape.identifier(interner), shape.fill_color(interner))
        })?
        .context("the spid we just set should resolve to the shape we just wrote")?;
    println!("  the OLE object's spid resolves to {found:?}");
    Ok(())
}

/// Without the `vml` feature the surface is not compiled at all — which is the point of the flag.
#[cfg(not(feature = "vml"))]
fn legacy_vml(_deck: &mut Presentation, _slide: usize, _ole: usize) -> Result<()> {
    println!("VML surface not compiled — rerun with `--features vml`");
    Ok(())
}
