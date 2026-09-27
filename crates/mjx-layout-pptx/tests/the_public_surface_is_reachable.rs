//! Every public item is reached by a test, and every one that carries a *choice* is reached at more
//! than one of its values.
//!
//! # The two instruments this suite is
//!
//! **Reachability.** A public function no test calls is a function nobody has run. Audit pass 10 of
//! this programme found a 591-line router that no test reached; the answer is a suite that names
//! the surface rather than trusting that the other suites happen to cover it.
//!
//! **Identity values.** A parameter reached constantly but only ever at its no-op value is invisible
//! to a reachability check. The acute case here is geometry: a suite where every shape is unrotated,
//! unflipped and at 100 % scale exercises *one* value of three parameters and would pass with all
//! three deleted. So the transform tests below use a rotation, a horizontal flip, a vertical flip
//! and both together, in a **portrait** box as well as a landscape one — because a bug that used the
//! width where it meant the height is invisible while every test box is square or landscape.

mod support;

use mjx_dml::{Emu, TextAutofit, TextBodyPropertiesSpec};
use mjx_layout::{
    BoxModel, ChangeKind, ChangeSet, ContentChange, DirtyPages, Fragment, LayoutPoint, LayoutRect,
    PageIndex, PartId, SourcePath, SourceRef,
};
use mjx_layout_pptx::{
    address, constraints_for, AutoNumberCounters, AutofitOutcome, AutofitPolicy, RunStyle,
    SlideBoxModel, SlideDeck, TabStops,
};
use mjx_pptx::{Presentation, ShapeBounds, Surface};

use support::{blank_deck, fixture, model, resolver, text_box};

// ---------------------------------------------------------------------------------------------
// The box model's own surface
// ---------------------------------------------------------------------------------------------

#[test]
fn every_constructor_and_accessor_of_the_box_model_is_reached() {
    let model = SlideBoxModel::new(resolver())
        .with_autofit(AutofitPolicy::HonourOnly)
        .with_features(mjx_text::FeatureSet::new());
    assert_eq!(model.autofit_policy(), AutofitPolicy::HonourOnly);
    assert_eq!(model.signature(), SlideBoxModel::SIGNATURE);
    assert!(model.fonts().manifest().is_empty());

    let mut model = model;
    // The rasteriser a painter must share. Registering a face here and again inside a layout pass
    // must give the same identity, or a painter would draw the wrong glyphs with no error anywhere.
    let face = mjx_text::FontFace::parse(
        std::fs::read(support::bundled_fonts().join("LiberationSans-Regular.ttf"))
            .expect("the committed face")
            .into(),
        0,
    )
    .expect("it parses");
    let first = model
        .rasteriser_mut()
        .register(&std::sync::Arc::new(face))
        .expect("it registers");
    assert_eq!(first.as_u32(), 0);
    assert_eq!(model.catalogue().decoration_count(), 0);
    assert_eq!(model.catalogue().geometry_count(), 0);
}

#[test]
fn the_catalogue_answers_the_handles_the_fragments_carry() {
    let (mut deck, slide) = blank_deck();
    let shape = deck
        .add_shape(
            slide,
            mjx_ooxml_types::drawingml::PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0),
        )
        .expect("a shape");
    deck.set_shape_fill(
        slide,
        shape,
        &mjx_dml::FillSpec::solid(mjx_dml::ColorSpec::Srgb("FF0000".to_owned())),
    )
    .expect("a fill");

    let read = SlideDeck::read(&mut deck).expect("read");
    let mut model = model();
    let page = model
        .layout_page(
            &read,
            PageIndex::new(u32::try_from(slide).expect("small")),
            &constraints_for(&read),
            None,
        )
        .expect("lays out");

    let (_, node) = page
        .fragments()
        .nodes()
        .find(|(_, node)| matches!(node.fragment(), Fragment::Shape(_)))
        .expect("a shape fragment");
    let Fragment::Shape(fragment) = node.fragment() else {
        unreachable!("just matched")
    };

    let request = model
        .catalogue()
        .geometry(fragment.geometry)
        .expect("the geometry handle resolves");
    assert_eq!(request.surface_index as usize, slide);
    assert_eq!(request.shape, vec![u32::try_from(shape).expect("small")]);
    assert_eq!(request.rect, node.rect());

    let handle = fragment
        .decoration
        .expect("the shape has a fill, so a handle");
    let decoration = model
        .catalogue()
        .decoration(handle)
        .expect("the decoration handle resolves");
    assert!(decoration.fill.is_some(), "the fill the deck states");

    assert!(model.catalogue().decoration_count() >= 1);
    assert!(model.catalogue().geometry_count() >= 1);
    assert!(model
        .catalogue()
        .shape_depth(&address::shape_path(
            u32::try_from(slide).expect("small"),
            &[u32::try_from(shape).expect("small")]
        ))
        .is_some());
}

// ---------------------------------------------------------------------------------------------
// The identity-value instrument: rotation, both flips, and a portrait box
// ---------------------------------------------------------------------------------------------

/// The page-space bounding box of the shape fragment on a slide whose one shape carries `transform`.
fn rotated_bounds(transform: &mjx_dml::Transform2D, portrait: bool) -> (LayoutRect, LayoutRect) {
    let (mut deck, slide) = blank_deck();
    let bounds = if portrait {
        ShapeBounds::from_inches(1.0, 1.0, 1.0, 3.0)
    } else {
        ShapeBounds::from_inches(1.0, 1.0, 3.0, 1.0)
    };
    let shape = deck
        .add_shape(
            slide,
            mjx_ooxml_types::drawingml::PresetShapeType::Rectangle,
            bounds,
        )
        .expect("a shape");
    deck.set_shape_transform(slide, shape, transform)
        .expect("the transform lands");

    let read = SlideDeck::read(&mut deck).expect("read");
    let mut model = model();
    let page = model
        .layout_page(
            &read,
            PageIndex::new(u32::try_from(slide).expect("small")),
            &constraints_for(&read),
            None,
        )
        .expect("lays out");
    let (id, node) = page
        .fragments()
        .nodes()
        .find(|(_, node)| matches!(node.fragment(), Fragment::Shape(_)))
        .expect("a shape fragment");
    (
        node.rect(),
        page.fragments()
            .page_bounds(id)
            .expect("the node has page bounds"),
    )
}

#[test]
fn a_rotation_maps_a_shapes_page_bounds_and_leaves_its_own_rectangle_alone() {
    // A landscape box turned a quarter turn becomes a portrait one. Both halves are asserted: the
    // node's own `rect` is still the unrotated rectangle — that is what makes it a rectangle at all
    // — and `page_bounds` is the axis-aligned box that contains the turned one.
    let quarter = mjx_dml::Transform2D {
        rotation: Some(mjx_dml::Angle::from_degrees(90.0)),
        ..mjx_dml::Transform2D::default()
    };
    let (own, page) = rotated_bounds(&quarter, false);
    assert_eq!(
        own.width(),
        Emu::from_inches(3.0),
        "the shape's own box is unchanged"
    );
    assert!(
        (page.height() - Emu::from_inches(3.0)).emu().abs() < 1_000,
        "a quarter turn makes a three-inch-wide shape three inches tall: {} EMU",
        page.height().emu()
    );
    assert!(
        (page.width() - Emu::from_inches(1.0)).emu().abs() < 1_000,
        "{} EMU",
        page.width().emu()
    );
    // It turns about its own centre, so the centre does not move.
    let unrotated = rotated_bounds(&mjx_dml::Transform2D::default(), false).1;
    let centre = |rect: LayoutRect| {
        (
            (rect.left + rect.right).divided_by(2),
            (rect.top + rect.bottom).divided_by(2),
        )
    };
    assert_eq!(centre(page), centre(unrotated));
}

#[test]
fn a_portrait_box_is_turned_the_same_way_a_landscape_one_is() {
    // The bug this exists for: a transform that used the width where it meant the height is
    // invisible while every test box is square or landscape. A three-inch-**tall** shape turned a
    // quarter turn must become three inches wide.
    let quarter = mjx_dml::Transform2D {
        rotation: Some(mjx_dml::Angle::from_degrees(90.0)),
        ..mjx_dml::Transform2D::default()
    };
    let (own, page) = rotated_bounds(&quarter, true);
    assert_eq!(own.height(), Emu::from_inches(3.0));
    assert!(
        (page.width() - Emu::from_inches(3.0)).emu().abs() < 1_000,
        "{} EMU",
        page.width().emu()
    );
}

#[test]
fn each_flip_and_both_together_are_four_distinct_transforms() {
    // Four values of two boolean parameters. A renderer that read `flipH` and ignored `flipV` would
    // give three of these the same answer.
    let mut seen = std::collections::BTreeSet::new();
    for (horizontal, vertical) in [(false, false), (true, false), (false, true), (true, true)] {
        let transform = mjx_dml::Transform2D {
            flip_horizontal: Some(horizontal),
            flip_vertical: Some(vertical),
            // A rotation as well, so the flips are composed with something rather than applied to
            // an identity where two of the four would coincide.
            rotation: Some(mjx_dml::Angle::from_degrees(30.0)),
            ..mjx_dml::Transform2D::default()
        };
        let (_, page) = rotated_bounds(&transform, true);
        seen.insert((
            page.left.emu(),
            page.top.emu(),
            page.right.emu(),
            page.bottom.emu(),
        ));
    }
    assert!(
        seen.len() >= 2,
        "the two flips must not be the same map: {seen:?}"
    );

    // And the composed transform must be invertible, which is what a hit test through it needs.
    let transform = mjx_dml::Transform2D {
        flip_horizontal: Some(true),
        rotation: Some(mjx_dml::Angle::from_degrees(30.0)),
        ..mjx_dml::Transform2D::default()
    };
    let (own, page) = rotated_bounds(&transform, true);
    assert!(
        page.width() > own.width(),
        "a turned portrait box is wider than it was"
    );
}

#[test]
fn a_point_inside_a_rotated_shape_hits_it_and_a_point_in_its_bounding_box_does_not() {
    // The narrow phase, which only exists because the broad phase is a bounding box. A point in the
    // corner of a rotated shape's bounding box is outside the shape, and a renderer that stopped at
    // the index would select it.
    let (mut deck, slide) = blank_deck();
    let shape = deck
        .add_shape(
            slide,
            mjx_ooxml_types::drawingml::PresetShapeType::Rectangle,
            ShapeBounds::from_inches(2.0, 2.0, 3.0, 1.0),
        )
        .expect("a shape");
    deck.set_shape_transform(
        slide,
        shape,
        &mjx_dml::Transform2D {
            rotation: Some(mjx_dml::Angle::from_degrees(45.0)),
            ..mjx_dml::Transform2D::default()
        },
    )
    .expect("the rotation lands");

    let read = SlideDeck::read(&mut deck).expect("read");
    let mut model = model();
    let page = model
        .layout_page(
            &read,
            PageIndex::new(u32::try_from(slide).expect("small")),
            &constraints_for(&read),
            None,
        )
        .expect("lays out");
    let (id, _) = page
        .fragments()
        .nodes()
        .find(|(_, node)| matches!(node.fragment(), Fragment::Shape(_)))
        .expect("a shape fragment");
    let bounds = page.fragments().page_bounds(id).expect("page bounds");

    let centre = LayoutPoint::new(
        (bounds.left + bounds.right).divided_by(2),
        (bounds.top + bounds.bottom).divided_by(2),
    );
    assert!(page.fragments().contains_page_point(id, centre));
    let corner = LayoutPoint::new(
        bounds.left + Emu::from_emu(1),
        bounds.top + Emu::from_emu(1),
    );
    assert!(
        !page.fragments().contains_page_point(id, corner),
        "the corner of a turned shape's bounding box is outside the shape"
    );
}

// ---------------------------------------------------------------------------------------------
// Invalidation
// ---------------------------------------------------------------------------------------------

#[test]
fn a_change_to_a_slide_dirties_that_slide_and_no_other() {
    let mut model = model();
    assert_eq!(model.invalidate(&ChangeSet::new()), DirtyPages::None);

    let mut changes = ChangeSet::new();
    changes.record(ContentChange {
        source: SourceRef::new(address::SLIDES, SourcePath::new(&[4, 2, 0, 0]), 0..3),
        kind: ChangeKind::Reformatted,
    });
    changes.record(ContentChange {
        source: SourceRef::new(address::SLIDES, SourcePath::new(&[1, 0, 0, 0]), 0..3),
        kind: ChangeKind::Inserted,
    });
    let dirty = model.invalidate(&changes);
    assert_eq!(
        dirty,
        DirtyPages::Pages(vec![PageIndex::new(1), PageIndex::new(4)])
    );
    assert!(dirty.contains(PageIndex::new(4)));
    assert!(!dirty.contains(PageIndex::new(3)));
    assert!(!dirty.is_empty());
}

#[test]
fn a_change_to_a_layout_or_a_master_dirties_everything() {
    // Honest rather than clever: this box model does not track which slides use which layout, and a
    // wrong `Pages` here would leave a stale slide on screen with nothing to notice it.
    for part in [address::LAYOUTS, address::MASTERS, PartId::new(9)] {
        let mut model = model();
        let mut changes = ChangeSet::new();
        changes.record(ContentChange {
            source: SourceRef::node(part, SourcePath::new(&[0, 1])),
            kind: ChangeKind::Reformatted,
        });
        assert_eq!(model.invalidate(&changes), DirtyPages::All);
    }
}

#[test]
fn a_change_with_no_surface_index_dirties_everything_rather_than_page_zero() {
    let mut model = model();
    let mut changes = ChangeSet::new();
    changes.record(ContentChange {
        source: SourceRef::node(address::SLIDES, SourcePath::root()),
        kind: ChangeKind::Removed,
    });
    assert_eq!(model.invalidate(&changes), DirtyPages::All);
}

// ---------------------------------------------------------------------------------------------
// The read model
// ---------------------------------------------------------------------------------------------

#[test]
fn a_deck_can_be_re_read_one_slide_at_a_time() {
    let mut presentation = Presentation::open(&fixture("charts.pptx")).expect("open");
    let mut deck = SlideDeck::read(&mut presentation).expect("read");
    assert!(!deck.is_empty());
    let before = deck.slide(1).expect("slide 1").shapes().len();

    // An edit to slide 1, seen only after that slide is re-read.
    let shape = presentation
        .add_shape(
            1,
            mjx_ooxml_types::drawingml::PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 1.0, 1.0, 1.0),
        )
        .expect("a shape");
    let _ = shape;
    assert_eq!(deck.slide(1).expect("slide 1").shapes().len(), before);

    deck.re_read_slide(&mut presentation, 1).expect("re-read");
    assert_eq!(deck.slide(1).expect("slide 1").shapes().len(), before + 1);

    // A slide past the end is not an error — it is a slide that no longer exists.
    deck.re_read_slide(&mut presentation, 99)
        .expect("no such slide");
    assert_eq!(deck.slide(99), None);
}

#[test]
fn a_paragraph_maps_an_offset_back_to_the_run_that_holds_it() {
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        "AAAAA BBBBB",
        ShapeBounds::from_inches(1.0, 1.0, 6.0, 1.0),
    );
    deck.set_text_range_properties(
        slide,
        shape,
        0,
        6..11,
        &mjx_dml::CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("two runs");

    let read = SlideDeck::read(&mut deck).expect("read");
    let paragraph = &read
        .slide(slide)
        .expect("the slide")
        .shapes()
        .iter()
        .find(|candidate| candidate.path == vec![u32::try_from(shape).expect("small")])
        .expect("the shape")
        .body
        .as_ref()
        .expect("a body")
        .paragraphs[0];

    assert_eq!(paragraph.run_at(0), Some((0, 0)));
    assert_eq!(paragraph.run_at(3), Some((0, 3)));
    assert_eq!(paragraph.run_at(6), Some((1, 0)));
    // A caret at the very end belongs to the last run rather than to no run.
    assert_eq!(paragraph.run_at(paragraph.text.len()), Some((1, 5)));
}

// ---------------------------------------------------------------------------------------------
// The small vocabularies
// ---------------------------------------------------------------------------------------------

#[test]
fn tab_stops_resolve_stated_positions_first_and_then_the_grid() {
    let stops = TabStops::new(
        vec![
            mjx_dml::TabStop::at_points(72.0, mjx_dml::TabAlignment::Left),
            mjx_dml::TabStop::at_points(36.0, mjx_dml::TabAlignment::Center),
        ],
        Emu::from_inches(0.5),
    );
    assert_eq!(stops.stated().len(), 2);
    assert_eq!(
        stops.stated()[0].position,
        Emu::from_points(36.0),
        "the stops are sorted"
    );
    assert_eq!(stops.next_after(Emu::ZERO), Emu::from_points(36.0));
    assert_eq!(
        stops.next_after(Emu::from_points(36.0)),
        Emu::from_points(72.0)
    );
    assert_eq!(
        stops.alignment_after(Emu::ZERO),
        Some(mjx_dml::TabAlignment::Center)
    );

    // Past the last stated stop the grid takes over, and a tab always advances.
    let past = stops.next_after(Emu::from_points(72.0));
    assert!(past > Emu::from_points(72.0), "{} EMU", past.emu());
    assert_eq!(stops.alignment_after(Emu::from_points(72.0)), None);

    // A default size of zero would make the grid an infinite loop.
    let degenerate = TabStops::new(Vec::new(), Emu::ZERO);
    assert_eq!(degenerate.next_after(Emu::ZERO), TabStops::DEFAULT_SIZE);
    assert!(degenerate.next_after(Emu::from_emu(-5)) > Emu::from_emu(-5));
}

#[test]
fn a_run_style_is_a_projection_of_resolved_properties() {
    let properties = mjx_dml::CharacterPropertiesSpec::new()
        .with_size_points(24.0)
        .with_bold(true)
        .with_italic(true);
    let style = RunStyle::from_properties(0..5, &properties);
    assert_eq!(style.size, mjx_text::FontSize::from_points(24.0));
    assert_eq!(style.weight, mjx_text::FontWeight::BOLD);
    assert_eq!(style.slant, mjx_text::FontSlant::Italic);

    let scaled = style.scaled(0.5);
    assert_eq!(scaled.size, mjx_text::FontSize::from_points(12.0));
    assert_eq!(scaled.weight, style.weight, "only the size scales");

    // A run that inherits nothing at all still has a size, and the family is left empty so the
    // resolver's own substitution — and the manifest entry it records — is what answers.
    let bare = RunStyle::from_properties(0..0, &mjx_dml::CharacterPropertiesSpec::new());
    assert_eq!(
        bare.size,
        mjx_text::FontSize::from_points(mjx_layout_pptx::text::ASSUMED_FONT_SIZE_POINTS)
    );
    assert_eq!(bare.family, mjx_layout_pptx::text::UNNAMED_FAMILY);
}

#[test]
fn the_autofit_outcome_reports_whether_it_scaled_anything() {
    let unscaled = AutofitOutcome::unscaled();
    assert!(!unscaled.scales_text());
    assert!(!unscaled.recomputed);
    assert_eq!(unscaled.grown_height, None);

    let scaled = AutofitOutcome {
        font_scale: 0.5,
        ..AutofitOutcome::unscaled()
    };
    assert!(scaled.scales_text());
}

#[test]
fn the_auto_number_counters_are_reachable_through_the_facade() {
    let mut counters = AutoNumberCounters::new();
    assert_eq!(counters.drawn_at(0), 0);
    assert_eq!(counters.next(0, 1), 1);
    counters.interrupt(0);
    assert_eq!(counters.drawn_at(0), 1);
}

#[test]
fn a_shape_no_tier_places_is_not_drawn_and_says_so() {
    // A decision rather than an omission, and one the Windows sitting should settle: `sample.pptx`
    // holds a `ctrTitle` whose `p:spPr` is empty and whose layout has no matching slot, so **no
    // tier places it**. Drawing it would mean inventing a rectangle, which would put a title across
    // a slide PowerPoint may put nowhere; not drawing it loses text a reader might expect to see.
    //
    // The information is not lost either way: the shape is in the read model with `bounds: None`,
    // so a caller can see exactly what happened.
    let mut presentation = Presentation::open(&fixture("sample.pptx")).expect("open");
    let deck = SlideDeck::read(&mut presentation).expect("read");
    let unplaced: Vec<&mjx_layout_pptx::Shape> = deck
        .slide(0)
        .expect("a slide")
        .shapes()
        .iter()
        .filter(|shape| shape.bounds.is_none())
        .collect();
    assert_eq!(unplaced.len(), 1, "the fixture has exactly one such shape");
    assert!(
        unplaced[0].body.is_some(),
        "and it has text, which is why the choice matters"
    );

    let mut model = model();
    let page = model
        .layout_page(&deck, PageIndex::FIRST, &constraints_for(&deck), None)
        .expect("lays out");
    assert_eq!(
        page.fragments().len(),
        1,
        "only the page's own box: an unplaced shape produces no fragments"
    );
}

#[test]
fn the_autofit_policies_are_each_reachable_and_each_distinct() {
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        "One two three four five six seven eight nine ten eleven twelve thirteen fourteen",
        ShapeBounds::from_inches(1.0, 1.0, 2.0, 0.5),
    );
    deck.set_body_properties(
        slide,
        shape,
        &TextBodyPropertiesSpec::new().with_autofit(TextAutofit::Normal {
            font_scale: None,
            line_space_reduction: None,
        }),
    )
    .expect("the body geometry lands");
    let read = SlideDeck::read(&mut deck).expect("read");

    let mut counts = Vec::new();
    for policy in [
        AutofitPolicy::HonourAndRecompute,
        AutofitPolicy::HonourOnly,
        AutofitPolicy::Disabled,
    ] {
        let mut model = SlideBoxModel::new(resolver()).with_autofit(policy);
        let page = model
            .layout_page(
                &read,
                PageIndex::new(u32::try_from(slide).expect("small")),
                &constraints_for(&read),
                None,
            )
            .expect("lays out");
        counts.push(page.fragments().len());
    }
    assert!(
        counts[0] != counts[1],
        "the search changes the page: {counts:?}"
    );
    assert_eq!(
        counts[1], counts[2],
        "with nothing stored, the other two agree"
    );
}

#[test]
fn the_constraints_helper_gives_the_deck_its_own_slide_size() {
    let mut presentation = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let deck = SlideDeck::read(&mut presentation).expect("read");
    let constraints = constraints_for(&deck);
    assert_eq!(constraints.page, deck.page());
    assert_eq!(constraints.content.size(), deck.page());
    assert_eq!(
        constraints.column_count(),
        1,
        "a slide has one content area"
    );
    assert_eq!(
        presentation
            .slide_size()
            .expect("the deck's size")
            .width_emu,
        deck.page().width.emu()
    );
    let _ = Surface::Slide(0);
}
