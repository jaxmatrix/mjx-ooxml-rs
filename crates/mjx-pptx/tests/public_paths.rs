//! One deck driven through **every seam of the `Presentation` surface**, using only the paths a
//! caller imports (`mjx_pptx::…`) and never a module path inside the crate.
//!
//! `presentation.rs` is split by subject — deck addressing, slides, shapes, text, hyperlinks, table
//! cells, table structure, bounds, appearance, effective readers, charts, chart decoration,
//! pictures, legacy content, notes. The split is only correct if none of it is visible from
//! outside: every method stays an inherent method on the one re-exported `Presentation`, with the
//! same signature and the same answer.
//!
//! A test that reached into `crate::presentation::text` would prove nothing about that, because a
//! caller cannot write such a path. So this file imports what `lib.rs` re-exports, nothing else, and
//! asserts a value only the method under test can produce — a fill it just wrote back out of the
//! effective reader, the dimensions of a table it just built, the series of a chart it just added.

use mjx_dml::{ColorSpec, FillSpec};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_ooxml_types::presentationml::SlideSizeKind;
use mjx_pptx::{
    Cells, ChartData, ChartKind, Geometry, Hyperlink, Package, Presentation, ShapeBounds,
    ShapeKind, SlideSize, Surface, DEFAULT_PLACEHOLDER_IMAGE,
};

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn widescreen() -> SlideSize {
    SlideSize {
        width_emu: 12_192_000,
        height_emu: 6_858_000,
        kind: SlideSizeKind::Screen16X9,
    }
}

fn bounds(x: i64, y: i64, w: i64, h: i64) -> ShapeBounds {
    ShapeBounds::new(x, y, w, h)
}

/// Every subject the surface is split into, exercised on one authored deck through `mjx_pptx::`.
#[test]
fn every_seam_of_the_surface_is_reachable_on_the_re_exported_presentation() {
    // Lifecycle: `blank` / `validate` / `save`.
    let mut deck = Presentation::blank(widescreen()).expect("a blank deck");

    // Deck addressing.
    assert_eq!(deck.slide_count(), 0, "a blank deck has no slides");
    assert_eq!(deck.master_count(), 1);
    assert_eq!(deck.layout_count(), 1);
    assert_eq!(
        deck.slide_size().expect("a slide size").width_emu,
        12_192_000
    );

    // Slide lifecycle.
    let slide = deck
        .add_slide_from_layout(0)
        .expect("a slide on the layout");
    assert_eq!(deck.slide_count(), 1);
    assert!(deck.color_map(slide).expect("a colour map").is_some());

    // Shape tree.
    let shapes = deck.shape_count(slide).expect("a shape count");
    assert!(shapes > 0, "the layout carried placeholders onto the slide");
    assert_eq!(
        deck.shape_kind(slide, 0).expect("a shape kind"),
        ShapeKind::Shape
    );

    // Text.
    deck.set_shape_text_content(slide, 0, "Public paths")
        .expect("set the title");
    assert_eq!(
        deck.shape_text(slide, 0).expect("read back"),
        "Public paths"
    );
    assert_eq!(deck.paragraph_count(slide, 0).expect("paragraphs"), 1);

    // Hyperlinks.
    deck.set_shape_hyperlink(
        slide,
        0,
        &Hyperlink::Url("https://example.invalid/".to_owned()),
    )
    .expect("set a shape hyperlink");
    assert_eq!(
        deck.shape_hyperlink(slide, 0).expect("read the hyperlink"),
        Some(Hyperlink::Url("https://example.invalid/".to_owned()))
    );

    // Shapes: authoring one, and its bounds and geometry.
    let box_idx = deck
        .add_shape(
            slide,
            PresetShapeType::RoundedRectangle,
            bounds(0, 0, 914_400, 457_200),
        )
        .expect("an authored shape");
    assert_eq!(
        deck.shape_bounds(slide, box_idx).expect("bounds"),
        Some(bounds(0, 0, 914_400, 457_200))
    );
    assert!(matches!(
        deck.shape_geometry(slide, box_idx).expect("geometry"),
        Geometry::Preset(_)
    ));

    // Appearance, and the effective reader that answers for it.
    let navy = FillSpec::Solid(ColorSpec::Srgb("1F3864".to_owned()));
    deck.set_shape_fill(slide, box_idx, &navy).expect("fill it");
    assert_eq!(
        deck.shape_fill(slide, box_idx).expect("own fill"),
        Some(navy.clone())
    );
    assert_eq!(
        deck.effective_shape_fill(slide, box_idx)
            .expect("effective fill"),
        Some(navy)
    );

    // Tables, and table cells.
    let table = deck
        .add_table(slide, 2, 3, bounds(0, 914_400, 5_486_400, 1_828_800))
        .expect("a table");
    assert_eq!(
        deck.table_dimensions(slide, table).expect("dimensions"),
        (2, 3)
    );
    deck.set_cell_text(slide, table, 0, 0, 0, "Header")
        .expect("cell text");
    assert_eq!(
        deck.cell_text(slide, table, 0, 0).expect("read the cell"),
        "Header"
    );
    deck.merge_cells(slide, table, Cells::rectangle(0..1, 0..2))
        .expect("merge two cells across");
    assert_eq!(
        deck.cell_span(slide, table, 0, 0).expect("the merged span"),
        (1, 2),
        "cell_span answers (rows, columns), the order table_dimensions answers in"
    );

    // Charts, and chart decoration.
    let chart_data = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2"])
        .series("Revenue", [1.0, 2.0]);
    let chart = deck
        .add_chart(
            slide,
            &chart_data,
            bounds(5_486_400, 914_400, 3_657_600, 2_743_200),
        )
        .expect("a chart");
    assert_eq!(
        deck.chart_kinds(slide, chart).expect("kinds"),
        vec![ChartKind::Bar]
    );
    assert_eq!(deck.chart_series(slide, chart).expect("series").len(), 1);
    assert!(deck
        .chart_data_labels(slide, chart, 0, None)
        .expect("data-label settings")
        .position
        .is_none());

    // Pictures.
    let png = DEFAULT_PLACEHOLDER_IMAGE;
    let picture = deck
        .add_picture(slide, png, bounds(0, 3_657_600, 914_400, 914_400))
        .expect("a picture");
    assert_eq!(
        deck.picture_image_bytes(slide, picture)
            .expect("image bytes"),
        Some(png)
    );

    // Notes.
    deck.set_notes_text(slide, "Say this out loud")
        .expect("notes");
    assert_eq!(
        deck.notes_text(slide).expect("read the notes").as_deref(),
        Some("Say this out loud")
    );

    // Surfaces other than a slide are addressed the same way.
    assert!(deck.shape_count(Surface::Layout(0)).expect("layout shapes") > 0);

    // And the whole thing still validates and saves.
    deck.validate().expect("the authored deck validates");
    let saved = deck.save().expect("save");
    assert!(Presentation::open(&saved).is_ok(), "the saved deck reopens");
}

/// The legacy-content seam, which needs a file that carries an OLE object and a VML drawing.
#[test]
fn the_legacy_content_seam_is_reachable_on_the_re_exported_presentation() {
    let mut deck = Presentation::open(&fixture("ole.pptx")).expect("open the OLE fixture");
    let objects = deck.ole_objects(0).expect("the slide's OLE objects");
    assert!(!objects.is_empty(), "the fixture carries an OLE object");
    assert!(deck
        .ole_prog_id(0, objects[0].shape_index)
        .expect("the object's ProgID")
        .is_some());

    // The VML half of the seam is reached through the fixture that actually carries a drawing.
    // `ole.pptx` holds an embedded object and its snapshot image and no `ppt/drawings/` part at
    // all, so asserting a VML drawing beside its OLE object asserted something untrue of the file.
    #[cfg(feature = "vml")]
    {
        let drawings = Presentation::open(&fixture("vml.pptx")).expect("open the VML fixture");
        assert!(
            !drawings.vml_part_names().is_empty(),
            "vml.pptx carries `ppt/drawings/vmlDrawing1.vml`"
        );
    }
}

/// `from_package` is the constructor for a caller who already holds the package — the facade opens a
/// container once and dispatches on its content type rather than handing the bytes back to each
/// format crate to re-open. It answers exactly what `open` answers for the same bytes, and it takes
/// its parameter type from `mjx_pptx::Package`, so a caller need not name `mjx-opc` to call it.
#[test]
fn from_package_resolves_the_same_deck_open_does() {
    let bytes = fixture("layouts.pptx");

    let mut opened = Presentation::open(&bytes).expect("open the bytes");
    let package = Package::open(&bytes).expect("open the package");
    let mut resolved = Presentation::from_package(package).expect("resolve the package");

    assert_eq!(resolved.slide_count(), opened.slide_count());
    assert_eq!(resolved.master_count(), opened.master_count());
    assert_eq!(
        resolved.layouts().expect("layouts"),
        opened.layouts().expect("layouts"),
        "the same layout inventory, resolved the same way"
    );
    assert_eq!(
        resolved.shapes(0).expect("shapes"),
        opened.shapes(0).expect("shapes")
    );
    assert_eq!(
        resolved.save().expect("save"),
        opened.save().expect("save"),
        "and it saves the same container bytes"
    );
}
