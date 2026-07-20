//! Integration tests for addressing shapes on layouts and masters, read and written over
//! `tests/fixtures/layouts.pptx` (its structure is tabulated in `tests/layouts.rs`).
//!
//! The point of the `Surface` address is inheritance: a slide placeholder that declares no property
//! of its own takes it from the same-slot placeholder on its layout, then its master — so editing a
//! layout is how one change reaches many slides.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{ColorSpec, FillSpec, LineSpec, LineWidth};
use mjx_ooxml_types::presentationml::{Orientation, PlaceholderSize, PlaceholderType};
use mjx_opc::Package;
use mjx_pptx::{PptxError, Presentation, ShapeKind, Surface};

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

fn deck() -> Presentation {
    Presentation::open(&fixture("layouts.pptx")).expect("open")
}

/// A solid fill in a colour no fixture part already uses.
fn marker_fill() -> FillSpec {
    FillSpec::solid(ColorSpec::Srgb("C00000".into()))
}

#[test]
fn a_layouts_shapes_are_addressable() {
    let mut pres = deck();
    // Layout 0 is "Title Slide": a centered title and a subtitle. Layout 2 is "Blank".
    assert_eq!(pres.shape_count(Surface::Layout(0)).expect("count"), 2);
    assert_eq!(pres.shape_count(Surface::Layout(2)).expect("count"), 0);
    assert_eq!(
        pres.shape_kind(Surface::Layout(0), 0).expect("kind"),
        ShapeKind::Shape
    );
    assert_eq!(
        pres.shape_count(Surface::Master(0)).expect("count"),
        2,
        "the master's own title and body placeholders"
    );
}

#[test]
fn a_layout_reports_the_placeholders_it_offers() {
    let mut pres = deck();
    let slots: Vec<(PlaceholderType, u32, Option<String>)> =
        (0..pres.shape_count(Surface::Layout(0)).expect("count"))
            .map(|idx| {
                let ph = pres
                    .shape_placeholder(Surface::Layout(0), idx)
                    .expect("placeholder")
                    .expect("every shape on this layout is a placeholder");
                (ph.kind, ph.index, ph.name)
            })
            .collect();
    assert_eq!(
        slots,
        vec![
            (
                PlaceholderType::CenteredTitle,
                0,
                Some("Title 1".to_owned())
            ),
            (PlaceholderType::Subtitle, 1, Some("Subtitle 2".to_owned())),
        ]
    );

    // Schema defaults are reported, not left blank.
    let ph = pres
        .shape_placeholder(Surface::Layout(0), 0)
        .expect("placeholder")
        .expect("placeholder");
    assert_eq!(ph.size, PlaceholderSize::Full);
    assert_eq!(ph.orientation, Orientation::Horizontal);

    // A non-placeholder shape reports None: add a plain autoshape to a slide and ask.
    let shape = pres
        .add_shape(
            0,
            mjx_ooxml_types::drawingml::PresetShapeType::Rectangle,
            mjx_pptx::ShapeBounds::from_inches(1.0, 1.0, 1.0, 1.0),
        )
        .expect("add shape");
    assert_eq!(pres.shape_placeholder(0, shape).expect("placeholder"), None);
}

#[test]
fn editing_a_layout_reaches_the_slides_built_on_it() {
    // The payoff of surface addressing. Slide 0 is on layout 1 ("Title and Content"); its title
    // placeholder declares no fill of its own, so it must inherit the layout's.
    let mut pres = deck();
    assert_eq!(
        pres.effective_shape_fill(0, 0).expect("effective fill"),
        None,
        "the fixture's title starts with no fill anywhere up the chain"
    );

    pres.set_shape_fill(Surface::Layout(1), 0, &marker_fill())
        .expect("fill the layout's title placeholder");

    assert_eq!(
        pres.effective_shape_fill(0, 0).expect("effective fill"),
        Some(marker_fill()),
        "the slide's title must inherit the layout's fill"
    );
    // The slide on the *other* layout is unaffected.
    assert_eq!(
        pres.effective_shape_fill(1, 0).expect("effective fill"),
        None
    );
    // …and the inheritance survives a save/reopen.
    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened.effective_shape_fill(0, 0).expect("effective fill"),
        Some(marker_fill())
    );
}

#[test]
fn a_slides_own_property_still_wins_over_the_layouts() {
    let mut pres = deck();
    pres.set_shape_fill(Surface::Layout(1), 0, &marker_fill())
        .expect("fill the layout");
    let own = FillSpec::solid(ColorSpec::Srgb("2E74B5".into()));
    pres.set_shape_fill(0, 0, &own).expect("fill the slide");

    assert_eq!(
        pres.effective_shape_fill(0, 0).expect("effective fill"),
        Some(own),
        "an explicit fill on the shape must beat the inherited one"
    );
}

#[test]
fn a_layout_shape_resolves_through_its_master_not_a_slide() {
    let mut pres = deck();
    // Give the master's title placeholder an outline; the layout's title has none of its own.
    let outline = LineSpec {
        fill: Some(FillSpec::solid(ColorSpec::Srgb("548235".into()))),
        width: Some(LineWidth::from_points(2.0)),
        ..LineSpec::new()
    };
    pres.set_shape_outline(Surface::Master(0), 0, &outline)
        .expect("outline the master's title");

    assert_eq!(
        pres.effective_shape_outline(Surface::Layout(1), 0)
            .expect("effective outline"),
        Some(outline.clone()),
        "a layout placeholder inherits from its master"
    );
    assert_eq!(
        pres.effective_shape_outline(0, 0)
            .expect("effective outline"),
        Some(outline),
        "and a slide placeholder inherits through the layout to the same master"
    );
}

#[test]
fn theme_and_color_map_resolve_from_every_surface() {
    let mut pres = deck();
    let accent1 = |theme: &mjx_dml::ThemeInfo| {
        theme
            .color(mjx_dml::ColorSchemeSlot::Accent1)
            .cloned()
            .expect("the theme defines accent1")
    };
    let from_slide = accent1(&pres.theme(0).expect("theme").expect("a theme"));
    for surface in [Surface::Layout(0), Surface::Layout(2), Surface::Master(0)] {
        let theme = pres.theme(surface).expect("theme").expect("a theme");
        assert_eq!(
            accent1(&theme),
            from_slide,
            "{surface} must resolve the same theme as its slides"
        );
    }
    // The master has no p:clrMapOvr of its own; it simply reports its p:clrMap.
    assert!(pres
        .color_map(Surface::Master(0))
        .expect("color map")
        .is_some());
    assert_eq!(
        pres.color_map(Surface::Layout(1)).expect("color map"),
        pres.color_map(0).expect("color map"),
    );
}

#[test]
fn editing_a_layout_dirties_only_that_layout() {
    let bytes = fixture("layouts.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.set_shape_fill(Surface::Layout(1), 0, &marker_fill())
        .expect("fill the layout");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    for (name, orig) in &original {
        if name == "ppt/slideLayouts/slideLayout2.xml" {
            continue; // layout index 1 is slideLayout2.xml
        }
        assert_eq!(reopened.get(name), Some(orig), "part {name} changed");
    }
    assert_ne!(
        reopened.get("ppt/slideLayouts/slideLayout2.xml"),
        original.get("ppt/slideLayouts/slideLayout2.xml"),
        "the edited layout should differ"
    );
}

#[test]
fn reading_a_layout_or_master_dirties_nothing() {
    let bytes = fixture("layouts.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    for surface in [Surface::Layout(0), Surface::Layout(1), Surface::Master(0)] {
        for idx in 0..pres.shape_count(surface).expect("count") {
            let _ = pres.shape_placeholder(surface, idx).expect("placeholder");
            let _ = pres.shape_fill(surface, idx).expect("fill");
            let _ = pres.effective_shape_fill(surface, idx).expect("effective");
        }
    }

    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        original
    );
}

#[test]
fn an_out_of_range_surface_names_itself_in_the_error() {
    let mut pres = deck();
    let err = pres
        .shape_count(Surface::Layout(9))
        .expect_err("no such layout");
    assert!(
        matches!(err, PptxError::LayoutIndexOutOfRange { index: 9, count: 3 }),
        "{err:?}"
    );
    let err = pres
        .shape_fill(Surface::Master(4), 0)
        .expect_err("no such master");
    assert!(
        matches!(err, PptxError::MasterIndexOutOfRange { .. }),
        "{err:?}"
    );

    // A shape index that is out of range names the surface it was addressing.
    let err = pres
        .shape_fill(Surface::Layout(2), 0)
        .expect_err("the Blank layout has no shapes");
    assert_eq!(
        err.to_string(),
        "shape index 0 out of range on layout 2 (0..0)"
    );
}

#[test]
fn a_bare_index_still_means_a_slide() {
    let mut pres = deck();
    assert_eq!(
        pres.shape_count(0).expect("count"),
        pres.shape_count(Surface::Slide(0)).expect("count")
    );
    assert_eq!(pres.shape_text(0, 0).expect("text"), "Title and Content");
}
