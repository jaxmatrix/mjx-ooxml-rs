//! The mechanical `a:avLst` writer: `set_shape_adjustments`, addressed by wire name (MJXOFF-207).
//!
//! # What this closes, measured rather than quoted
//!
//! `set_shape_geometry` writes a [`Geometry::Preset`], which carries `mjx-dml`'s **typed**
//! `ShapeGeometry`: 118 variants, one of them `Unmodeled`, so 117 presets are named and 70 are not.
//! MJXOFF-206's hand-off reads that arithmetic as *"an extremes deck cannot be authored for 70 of
//! the 187"*, and [`exactly_two_adjustable_presets_have_no_typed_variant`] is the measurement that
//! corrects it: **only 119 presets have an adjustment at all**, 117 of those are typed, and the
//! shapes the typed writer genuinely cannot move are **two** — `sun` and `teardrop`. The other 68
//! untyped presets are `rect`, `ellipse`, `line` and their kin, which have no handle to move and
//! therefore no extreme to author.
//!
//! Two shapes is a much smaller hole than seventy, and it is not the whole reason this writer
//! exists. The larger one is [`every_adjustment_of_every_preset_is_writable_by_wire_name`]: a
//! reference deck states each of 285 adjustments **in the file's own units**, and a typed writer
//! can only get there through a `Fraction` or an `Angle` that it must first have a field for.
//!
//! So there are two writers and they are complementary: `set_shape_geometry` may change *which*
//! shape it is and cannot say `adj5` on a shape it has no variant for; this one may always say
//! `adj5` and can never change the shape.
//!
//! # Why the assertions read the value back through `shape_adjustments`
//!
//! Asserting that the XML contains `val 40000` would prove the writer emitted a string. Reading it
//! back through the *domain* reader proves the string is one the shape's own guide evaluator picks
//! up as that adjustment's value — which is what a renderer and PowerPoint both do — and it is the
//! reader `set_shape_adjustments`'s documentation promises to be the other half of.
//!
//! # Proved by mutation
//!
//! * `set_adjustment(interner, wire_name, *value)` → `set_adjustment(interner, wire_name, 0)`:
//!   [`a_written_adjustment_reads_back_as_itself`] and
//!   [`every_adjustment_of_every_preset_is_writable_by_wire_name`] both fail, naming the value.
//! * Dropping the `*prst_geom = geometry.to_xml(interner)` write-back: every case here fails,
//!   because the edit is then made to a parsed copy and thrown away.
//! * Writing every named adjustment and clearing the rest — i.e. building a fresh `PresetGeometry`
//!   instead of parsing the existing one: [`an_unnamed_adjustment_is_left_alone`] fails.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{GuideContext, ShapeGeometry, Size};
use mjx_ooxml_types::drawingml::{adjustable_shapes, adjustments_of, PresetShapeType};
use mjx_opc::Package;
use mjx_pptx::{Geometry, PptxError, Presentation, ShapeBounds};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn byte_map(package: &Package) -> BTreeMap<String, Vec<u8>> {
    package
        .entries()
        .iter()
        .filter_map(|entry| {
            entry
                .bytes()
                .map(|bytes| (entry.name.clone(), bytes.to_vec()))
        })
        .collect()
}

fn bounds() -> ShapeBounds {
    ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0)
}

/// The size the domains are read against — deliberately not square, so an adjustment wired to the
/// wrong axis would move a different distance in each direction.
fn context() -> GuideContext {
    GuideContext::from_size(Size::from_emu(1_828_800, 914_400))
}

/// What `shape_adjustments` reports, as a map from wire name to value.
fn values(deck: &mut Presentation, shape: usize) -> BTreeMap<String, f64> {
    deck.shape_adjustments(0, shape, context())
        .expect("the adjustments resolve")
        .into_iter()
        .map(|adjustment| (adjustment.spec.wire_name.to_owned(), adjustment.value))
        .collect()
}

#[test]
fn a_written_adjustment_reads_back_as_itself() {
    let mut deck = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = deck
        .add_shape(0, PresetShapeType::RoundedRectangle, bounds())
        .expect("add shape");

    // The default first, so the assertion below is a *change* and not a coincidence.
    let before = values(&mut deck, shape);
    assert_eq!(before.len(), 1, "roundRect has one adjustment: {before:?}");
    assert!(
        (before["adj"] - 16_667.0).abs() < 1e-9,
        "the spec default is 16667, not {before:?}"
    );

    deck.set_shape_adjustments(0, shape, &[("adj", 40_000)])
        .expect("the adjustment is written");
    let after = values(&mut deck, shape);
    assert!(
        (after["adj"] - 40_000.0).abs() < 1e-9,
        "the written value did not reach the reader: {after:?}"
    );

    // And through the typed reader too, which is a second route to the same `a:gd`.
    let Geometry::Preset(ShapeGeometry::RoundedRectangle { corner_radius }) =
        deck.shape_geometry(0, shape).expect("geometry")
    else {
        panic!("a roundRect reads back as a roundRect");
    };
    assert!(
        (corner_radius.ratio() - 0.40).abs() < 1e-9,
        "the typed reader saw {corner_radius:?}"
    );
}

#[test]
fn it_survives_a_save_and_a_reopen() {
    let mut deck = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = deck
        .add_shape(0, PresetShapeType::RoundedRectangle, bounds())
        .expect("add shape");
    deck.set_shape_adjustments(0, shape, &[("adj", 40_000)])
        .expect("the adjustment is written");

    let mut reread = Presentation::open(&deck.save().expect("save")).expect("reopen");
    let after = values(&mut reread, shape);
    assert!(
        (after["adj"] - 40_000.0).abs() < 1e-9,
        "the adjustment did not survive the round trip: {after:?}"
    );
}

/// Several adjustments at once, on a preset the *typed* writer covers, so the two writers are
/// exercised over the same shape.
#[test]
fn several_adjustments_are_written_in_one_call() {
    let mut deck = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = deck
        .add_shape(0, PresetShapeType::LeftCircularArrow, bounds())
        .expect("add shape");
    let before = values(&mut deck, shape);
    assert_eq!(
        before.len(),
        5,
        "leftCircularArrow has five adjustments: {before:?}"
    );
    deck.set_shape_adjustments(0, shape, &[("adj2", 5_000), ("adj5", 900_000)])
        .expect("both adjustments are written");

    let after = values(&mut deck, shape);
    assert!(
        (after["adj2"] - 5_000.0).abs() < 1e-9 && (after["adj5"] - 900_000.0).abs() < 1e-9,
        "the two written values did not reach the reader: {after:?}"
    );
    // Still the same shape. A writer that rebuilt the geometry from a typed value would have
    // written `prst="rect"` here, and nothing else in this file would have noticed.
    assert!(
        matches!(
            deck.shape_geometry(0, shape).expect("geometry"),
            Geometry::Preset(ShapeGeometry::LeftCircularArrow { .. })
        ),
        "the shape stopped being a leftCircularArrow when only its adjustments were named"
    );
}

/// **The measurement that corrects MJXOFF-206's hand-off.** That ticket reports *"117 presets have
/// named adjustments and 70 do not"*, and concludes that an extremes deck is unauthorable for 70
/// shapes. The first half is arithmetic on `ShapeGeometry`'s 118 variants and is right; the
/// conclusion does not follow, and this case is the measurement.
///
/// **Only 119 presets have an adjustment at all.** The other 68 — `rect`, `ellipse`, `triangle`,
/// `line`, the plain arrows — have an empty `a:avLst` and no handle to move, so *"at adjustment
/// extremes"* is not a state they have. Of the 119, **117 carry a typed variant and exactly two do
/// not**: `sun` and `teardrop`, one adjustment each.
///
/// So the gap the typed writer leaves is **two shapes**, not seventy — which is a much smaller
/// reason for this writer to exist than the hand-off gives, and not the only one: the typed writer
/// also cannot state a value in native units without going through a `Fraction` or an `Angle`, and
/// [`every_adjustment_of_every_preset_is_writable_by_wire_name`] is the claim that actually matters.
///
/// Named rather than derived-and-shrugged-at, so a preset acquiring or losing a typed variant fails
/// here instead of quietly changing what this writer is for.
#[test]
fn exactly_two_adjustable_presets_have_no_typed_variant() {
    let mut deck = Presentation::open(&fixture("sample.pptx")).expect("open");
    let mut untyped = Vec::new();
    for preset in adjustable_shapes() {
        let shape = deck.add_shape(0, *preset, bounds()).expect("add shape");
        if let Ok(Geometry::Preset(ShapeGeometry::Unmodeled(_))) = deck.shape_geometry(0, shape) {
            untyped.push(preset.to_wire());
        }
        deck.remove_shape(0, shape).expect("remove shape");
    }
    assert_eq!(
        adjustable_shapes().len(),
        119,
        "the count of presets with an adjustment at all has moved"
    );
    assert_eq!(
        untyped,
        vec!["sun", "teardrop"],
        "the set of adjustable presets the typed enumeration cannot express has changed"
    );

    // And each of the two is authorable here, which is the point.
    for preset in [PresetShapeType::Sun, PresetShapeType::Teardrop] {
        let shape = deck.add_shape(0, preset, bounds()).expect("add shape");
        deck.set_shape_adjustments(0, shape, &[("adj", 33_333)])
            .expect("the adjustment is written");
        let after = values(&mut deck, shape);
        assert!(
            (after["adj"] - 33_333.0).abs() < 1e-9,
            "`{}` did not take its adjustment: {after:?}",
            preset.to_wire()
        );
    }
}

/// **Every adjustment of every adjustable preset, written by wire name and read back.** 285 of
/// them across 119 shapes.
///
/// This is the claim the reference pack's extremes deck rests on, and it is the one the typed
/// writer cannot make: a value here is stated in the file's own units and arrives at the reader as
/// that number, for `adj5` of a five-handled shape exactly as for `adj` of a one-handled one.
///
/// The count is asserted so the sweep cannot go vacuous. A `for` loop over a table that quietly
/// emptied would pass in silence, which is this programme's signature defect.
#[test]
fn every_adjustment_of_every_preset_is_writable_by_wire_name() {
    let mut deck = Presentation::open(&fixture("sample.pptx")).expect("open");
    let context = context();
    let mut swept = 0usize;
    let mut distinct_targets: BTreeMap<i32, usize> = BTreeMap::new();

    for preset in adjustable_shapes() {
        let shape = deck.add_shape(0, *preset, bounds()).expect("add shape");
        for spec in adjustments_of(*preset) {
            // Off the default by a value no default is, so a writer that silently did nothing would
            // read back the default and fail. `saturating_add` because a default may be extreme.
            let target = i32::from(spec.default).saturating_add(1_234);
            *distinct_targets.entry(target).or_default() += 1;
            deck.set_shape_adjustments(0, shape, &[(spec.wire_name, target)])
                .expect("the adjustment is written");
            let read = deck
                .shape_adjustments(0, shape, context)
                .expect("the adjustments resolve")
                .into_iter()
                .find(|adjustment| adjustment.spec.wire_name == spec.wire_name)
                .map(|adjustment| adjustment.value);
            assert!(
                read.is_some_and(|value| (value - f64::from(target)).abs() < 1e-9),
                "`{}`/`{}`: wrote {target}, read {read:?}",
                preset.to_wire(),
                spec.wire_name
            );
            swept += 1;
        }
        deck.remove_shape(0, shape).expect("remove shape");
    }

    assert_eq!(
        swept, 285,
        "the sweep covered {swept} adjustments; the table has moved and the figure in this file's \
         documentation with it"
    );
    // How many distinct values the gate actually saw. A sweep that wrote one number to every
    // adjustment would prove far less than this one, and the difference is invisible in a green.
    assert!(
        distinct_targets.len() > 30,
        "the sweep wrote only {} distinct values across {swept} adjustments, so most of it is one \
         measurement repeated",
        distinct_targets.len()
    );
    println!(
        "wire-name writer: {swept} adjustments across {} presets, {} distinct values written",
        adjustable_shapes().len(),
        distinct_targets.len()
    );
}

#[test]
fn an_unnamed_adjustment_is_left_alone() {
    let mut deck = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = deck
        .add_shape(0, PresetShapeType::LeftCircularArrow, bounds())
        .expect("add shape");
    let before = values(&mut deck, shape);

    deck.set_shape_adjustments(0, shape, &[("adj3", 7_500)])
        .expect("the adjustment is written");
    let after = values(&mut deck, shape);

    assert!(
        (after["adj3"] - 7_500.0).abs() < 1e-9,
        "the named adjustment did not move: {after:?}"
    );
    for name in ["adj1", "adj2", "adj4", "adj5"] {
        assert!(
            (after[name] - before[name]).abs() < 1e-9,
            "`{name}` moved from {} to {} without being named",
            before[name],
            after[name]
        );
    }
}

/// Two calls compose, and the second does not undo the first.
#[test]
fn two_calls_accumulate() {
    let mut deck = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = deck
        .add_shape(0, PresetShapeType::LeftCircularArrow, bounds())
        .expect("add shape");
    deck.set_shape_adjustments(0, shape, &[("adj1", 1_000)])
        .expect("first");
    deck.set_shape_adjustments(0, shape, &[("adj2", 2_000)])
        .expect("second");
    let after = values(&mut deck, shape);
    assert!(
        (after["adj1"] - 1_000.0).abs() < 1e-9 && (after["adj2"] - 2_000.0).abs() < 1e-9,
        "the second call dropped the first: {after:?}"
    );
}

/// Writing the *same* name twice keeps the last value, rather than emitting two `a:gd`s the
/// evaluator would then have to arbitrate.
#[test]
fn writing_one_name_twice_upserts_rather_than_appends() {
    let mut deck = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = deck
        .add_shape(0, PresetShapeType::RoundedRectangle, bounds())
        .expect("add shape");
    deck.set_shape_adjustments(0, shape, &[("adj", 10_000)])
        .expect("first");
    deck.set_shape_adjustments(0, shape, &[("adj", 30_000)])
        .expect("second");

    let saved = deck.save().expect("save");
    let package = Package::open(&saved).expect("reopen the package");
    let slide = package
        .entries()
        .iter()
        .find(|entry| entry.name == "ppt/slides/slide1.xml")
        .and_then(|entry| entry.bytes())
        .expect("the edited slide");
    let text = String::from_utf8_lossy(slide);
    assert_eq!(
        text.matches("name=\"adj\"").count(),
        1,
        "the adjustment was appended rather than upserted; the slide has more than one `adj`"
    );
    assert!(
        text.contains("val 30000") && !text.contains("val 10000"),
        "the second write did not replace the first"
    );
}

/// A shape with no `a:prstGeom` is refused by name rather than silently doing nothing.
#[test]
fn a_shape_with_no_preset_geometry_is_refused() {
    let mut deck = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = deck
        .add_shape(0, PresetShapeType::RoundedRectangle, bounds())
        .expect("add shape");
    // Take its preset geometry away, which is what a `custGeom` shape and an inheriting one both
    // look like from here.
    deck.set_shape_geometry(0, shape, Geometry::Inherited)
        .expect("the geometry is removed");

    let refused = deck.set_shape_adjustments(0, shape, &[("adj", 40_000)]);
    assert!(
        matches!(refused, Err(PptxError::ShapeHasNoPresetGeometry)),
        "expected `ShapeHasNoPresetGeometry`, got {refused:?}"
    );
}

/// Only the slide that was edited changes. The fidelity contract, held for this writer as for every
/// other: an untouched part re-emits verbatim.
#[test]
fn only_the_edited_slide_changes() {
    let original = fixture("sample.pptx");
    let before = byte_map(&Package::open(&original).expect("open the original"));

    let mut deck = Presentation::open(&original).expect("open");
    let shape = deck
        .add_shape(0, PresetShapeType::RoundedRectangle, bounds())
        .expect("add shape");
    deck.set_shape_adjustments(0, shape, &[("adj", 40_000)])
        .expect("the adjustment is written");
    let after = byte_map(&Package::open(&deck.save().expect("save")).expect("reopen"));

    let changed: Vec<&String> = before
        .keys()
        .filter(|name| before.get(*name) != after.get(*name))
        .collect();
    assert_eq!(
        changed,
        vec!["ppt/slides/slide1.xml"],
        "an edit to one slide changed other parts"
    );
}
