//! The 856 connection sites of the 173 presets that have any, measured against the outline they
//! are supposed to sit on.
//!
//! # Why "on the outline" is the question
//!
//! A connection site is where an elbow or a curved connector attaches. There is no reference render
//! to check it against and no exception if it is wrong — the connector simply starts a few pixels
//! into, or short of, the shape, which reads as a drawing style rather than as a defect. The
//! statement that *is* falsifiable is geometric and needs no second author: **a site lies on the
//! shape's own outline.**
//!
//! That is what this suite measures, point-to-segment over the flattened contours, and it is the
//! reason `crate::seed`'s hand-written reference deliberately carries **no** connection sites: a
//! site's count and placement are a convention rather than a consequence, so transcribing them
//! would have produced a differential whose two sides share a source. This measurement does not.
//!
//! # It found a defect in ECMA-376's own file
//!
//! `squareTabs`'s sixth site reads `<pos x="dx" y="x1"/>`, and `x1 = r - dx` is a **horizontal**
//! guide. Its three siblings are the other three inner corners — `(dx, dx)`, `(x1, dx)`,
//! `(x1, y1)` — so the missing one is `(dx, y1)`, which is a vertex of the shape's own second path.
//! As written the site lands ten points *below* a 120-point shape's bottom edge. `xtask`'s
//! `CONNECTION_ERRATA` corrects it, with the file's own text guarded; with the correction the site
//! lands on the outline exactly, and [`the_sites_that_leave_the_outline_are_named`] is what noticed.
//!
//! # The census, so a reader need not rerun it
//!
//! | | |
//! |---|---|
//! | presets with an `a:cxnLst` | **173** of 186 |
//! | presets without one | **13** — the nine connectors, the three `chart*` marks, `funnel` |
//! | sites | **856** |
//! | position coordinates, and how many are literals | **1 712**, and **none** |
//! | `@ang` values: literal / guide | **208** / **648** |
//! | sites more than 0.01 px off the outline | **8** presets, the same eight in both orientations |
//! | how far off they are | at most **30.0** px landscape, **30.9** px portrait |
//! | sites outside the shape's own box | **4** presets — the callout tails |
//! | how far outside | exactly **h/8**: 15 px landscape, 20 px portrait |
//!
//! # Every census is taken in two orientations
//!
//! `ss` is `min(w, h)`, so in a landscape box the shorter side is always the height — and every box
//! this crate had was landscape or square. Everything below is therefore measured in
//! [`common::box_on_the_page`] (160 × 120) **and** in [`common::portrait_box_on_the_page`]
//! (120 × 160), which have the same `ss` and differ only in which side it is. Every *membership*
//! here came out identical in the two; every *distance* did not, which is why the bounds are stated
//! as pairs and the lists are not.

mod common;

use std::collections::BTreeSet;

use common::{box_on_the_page, distance_to_outline, extents_of_the_box, orientations};
use mjx_geometry::{
    adjustment_domains, connection_sites_of_definition, preset_connection_sites, preset_outline,
    seeded_shapes, AdjustmentOverride, Derivation, GeometryError, PathFillMode, PresetAngle,
    PresetConnectionSite, PresetCoordinate, PresetPath, PresetPathStep, PresetPoint,
    PresetShapeDefinition, PresetShapeType,
};
use mjx_ooxml_types::drawingml::PresetGuide;

/// How far a connection site may sit from the outline before it counts as being off it.
///
/// The same number, for the same reasons, as this crate's other suites use: half an EMU of
/// coordinate rounding plus one `f32` narrowing, a hundred times over. It has thirty times the
/// headroom it needs — the worst of the 165 presets whose sites are *on* the outline is `pentagon`
/// at 0.0003 device pixels — which [`the_sites_that_leave_the_outline_are_named`] asserts so that
/// the tolerance cannot quietly start doing the geometry's work.
const ON_THE_OUTLINE_TOLERANCE_PIXELS: f32 = 0.01;

/// The thirteen presets that declare no connection sites at all.
///
/// **Nine of them are connectors**, and that is the point of them: a `bentConnector3` is the line
/// *between* two shapes and has nothing of its own to connect to. The other four are the three
/// `chart*` tick marks and `funnel`.
const NO_CONNECTION_SITES: &[PresetShapeType] = &[
    PresetShapeType::BentConnector2,
    PresetShapeType::BentConnector3,
    PresetShapeType::BentConnector4,
    PresetShapeType::BentConnector5,
    PresetShapeType::ChartPlus,
    PresetShapeType::ChartStar,
    PresetShapeType::ChartX,
    PresetShapeType::CurvedConnector2,
    PresetShapeType::CurvedConnector3,
    PresetShapeType::CurvedConnector4,
    PresetShapeType::CurvedConnector5,
    PresetShapeType::Funnel,
    PresetShapeType::StraightConnector1,
];

/// The presets with a connection site that is **not** on the outline they draw.
///
/// Eight, and every one of them is a shape whose *frame* and whose *ink* are different things:
///
/// * **`blockArc`** puts a site at `(hc, vc)` — the centre of the arc it sweeps, where there is no
///   ink at all. 29.99 device pixels, the worst case in the landscape box.
/// * **`gear6`** and **`gear9`** put their sites on the pitch circle rather than on a tooth.
/// * **`cloud`** and **`cloudCallout`** put theirs on the notional ellipse the puffs are drawn
///   around, and `cloudCallout`'s fifth is its tail tip.
/// * **`flowChartDocument`** and **`flowChartMultidocument`** put a site at the mid-point of a
///   bottom edge that is a wave.
/// * **`wedgeEllipseCallout`**'s tail tip misses the ellipse by a quarter of a tenth of a pixel.
///
/// None of that is wrong — a connector attaching to the middle of a gear should attach to the
/// gear's circle — but "mostly on the outline" is not a statement a regression could fail, so the
/// exceptions are named and bounded by [`FURTHEST_A_SITE_SITS_OFF_THE_OUTLINE`].
const SITES_OFF_THE_OUTLINE: &[PresetShapeType] = &[
    PresetShapeType::BlockArc,
    PresetShapeType::Cloud,
    PresetShapeType::CloudCallout,
    PresetShapeType::FlowChartDocument,
    PresetShapeType::FlowChartMultidocument,
    PresetShapeType::Gear6,
    PresetShapeType::Gear9,
    PresetShapeType::WedgeEllipseCallout,
];

/// How far off its own outline any connection site sits, at default adjustments, in the landscape
/// box and in the portrait one.
///
/// Set by `blockArc`'s centre site at 29.994 device pixels in the landscape box and by `gear9`'s
/// pitch circle at 30.930 in the portrait one — **the two orientations do not even agree on which
/// shape is worst**, which is the plainest statement available of why one number for both would be
/// measuring nothing. Each is stated just above its own measured worst case, and each is asserted
/// to be within a pixel of it.
const FURTHEST_A_SITE_SITS_OFF_THE_OUTLINE: [(&str, f32); 2] =
    [("landscape", 30.0), ("portrait", 31.0)];

/// The presets with a connection site **outside** the box the shape was drawn in.
///
/// All four are the callouts whose last site is the tip of the tail, and a callout's tail is
/// supposed to leave the box — the same eighteen-shape story
/// `every_preset_stands_where_its_box_is.rs` tells about the outlines themselves, narrowed to the
/// four whose tail tip is also a connection site. At default adjustments every one of them is
/// exactly [`FURTHEST_A_SITE_LEAVES_THE_BOX`] outside, because they share `adj1`/`adj2` defaults.
const SITES_OUTSIDE_THE_BOX: &[PresetShapeType] = &[
    PresetShapeType::CloudCallout,
    PresetShapeType::WedgeEllipseCallout,
    PresetShapeType::WedgeRectangleCallout,
    PresetShapeType::WedgeRoundedRectangleCallout,
];

/// How far outside the shape's own box any connection site sits, at default adjustments, in the
/// landscape box and in the portrait one.
///
/// Both numbers are exact and both are **`h/8`**: the four wedge callouts share their tail's
/// adjustment defaults, `adj2 = 62500`, which puts the tip at `vc + h·0.625` — an eighth of the
/// height past the bottom edge. 15 device pixels of a 120-tall box and 20 of a 160-tall one.
/// Asserted as an equality rather than as a bound, because four shapes sharing a default should land
/// on exactly one number and a range would hide it if they stopped.
const FURTHEST_A_SITE_LEAVES_THE_BOX: [(&str, f32); 2] = [("landscape", 15.0), ("portrait", 20.0)];

/// The presets whose **connection sites** have no value somewhere in their own adjustment domain.
///
/// **This is not the same list as the paths', and that is the finding rather than an accident.**
/// `an_adjustment_moves_the_shape.rs`'s `SINGULAR_SOMEWHERE` names the six presets whose *paths*
/// read a guide with no finite value; `text_goes_inside_the_shape.rs` names the three whose `a:rect`
/// does. This is the six whose `a:cxnLst` does, and the three lists are genuinely different:
///
/// * **`noSmoking`** is singular in a path and not in a site — its `sqrt` of a negative is in the
///   band it draws, which no site reads.
/// * **`parallelogram`** is the mirror image: it is singular in `q3`, which its **connection
///   sites** read and its paths and its text rectangle do not. MJXOFF-204's brief listed it among
///   the shapes singular in the text rectangle's insets; measured, it is singular *here*.
///
/// That is what `mjx-geometry`'s one-guide-at-a-time environment buys. A resolver that refused a
/// shape whose `gdLst` had any singular guide would refuse `parallelogram` at `adj = 0` outright,
/// and it draws perfectly well there.
const SITES_SINGULAR_SOMEWHERE: &[PresetShapeType] = &[
    PresetShapeType::CircularArrow,
    PresetShapeType::CurvedDownArrow,
    PresetShapeType::CurvedUpArrow,
    PresetShapeType::LeftCircularArrow,
    PresetShapeType::LeftRightCircularArrow,
    PresetShapeType::Parallelogram,
];

/// The wire tokens of a set of presets, sorted — what a failure message prints.
fn tokens(presets: impl IntoIterator<Item = PresetShapeType>) -> BTreeSet<&'static str> {
    presets.into_iter().map(PresetShapeType::to_wire).collect()
}

// -------------------------------------------------------------------------------------------
// The census
// -------------------------------------------------------------------------------------------

#[test]
fn the_table_carries_856_connection_sites_across_173_presets() {
    let mut without: BTreeSet<&'static str> = BTreeSet::new();
    let (mut with, mut sites) = (0usize, 0usize);
    let (mut literal_angles, mut guide_angles) = (0usize, 0usize);
    let (mut literal_coordinates, mut coordinates) = (0usize, 0usize);

    for definition in seeded_shapes() {
        if definition.connection_sites.is_empty() {
            without.insert(definition.preset.to_wire());
            continue;
        }
        with += 1;
        for site in definition.connection_sites {
            sites += 1;
            match site.angle {
                PresetAngle::Native(_) => literal_angles += 1,
                PresetAngle::Guide(_) => guide_angles += 1,
            }
            for coordinate in [site.position.x, site.position.y] {
                coordinates += 1;
                if matches!(coordinate, PresetCoordinate::Emu(_)) {
                    literal_coordinates += 1;
                }
            }
        }
    }

    assert_eq!(with, 173, "the table no longer holds 173 shapes with sites");
    assert_eq!(
        without,
        tokens(NO_CONNECTION_SITES.iter().copied()),
        "the presets with no `a:cxnLst` have changed"
    );
    assert_eq!(with + without.len(), 186, "the census no longer adds up");
    assert_eq!(sites, 856, "the table no longer holds 856 connection sites");
    assert_eq!(
        coordinates, 1_712,
        "856 sites no longer have 1 712 coordinates"
    );
    // **Both arms of `PresetAngle` are exercised by real data**, which is why the type has two and
    // not one — and the *positions* are not so evenly split, which is asserted rather than assumed.
    assert_eq!(literal_angles, 208, "the literal `@ang` count has changed");
    assert_eq!(
        guide_angles, 648,
        "the guide-named `@ang` count has changed"
    );
    assert_eq!(
        literal_coordinates, 0,
        "{literal_coordinates} of the 1 712 site coordinates are literals; every one used to be a \
         guide name"
    );
    println!(
        "connection sites: {with} presets, {sites} sites, {literal_angles} literal angles, \
         {guide_angles} guide angles, all {coordinates} coordinates guide names"
    );
}

// -------------------------------------------------------------------------------------------
// Are they on the outline?
// -------------------------------------------------------------------------------------------

#[test]
fn the_sites_that_leave_the_outline_are_named() {
    // **The membership is asserted in both orientations and comes out identical; the bound is
    // per-orientation and does not.** Which shapes attach a connector somewhere they draw no ink is
    // a property of the shape. *How far* off is a property of the box, and the two orientations do
    // not even agree on which shape is worst — `blockArc`'s centre in landscape, `gear9`'s pitch
    // circle in portrait — so one bound for both would be a number neither measurement is near.
    for (index, (orientation, within, extents)) in orientations().into_iter().enumerate() {
        let (bounded, bound) = FURTHEST_A_SITE_SITS_OFF_THE_OUTLINE[index];
        assert_eq!(bounded, orientation, "the bounds are out of step");
        let mut measured: BTreeSet<&'static str> = BTreeSet::new();
        let mut worst = 0.0f32;
        let mut worst_on_the_outline = 0.0f32;
        let mut checked = 0usize;

        for definition in seeded_shapes() {
            let token = definition.preset.to_wire();
            let sites = preset_connection_sites(definition.preset, extents, &[], within)
                .unwrap_or_else(|error| panic!("`{token}` has no connection sites: {error}"));
            if sites.is_empty() {
                continue;
            }
            let outline = preset_outline(definition.preset, extents, &[], within)
                .unwrap_or_else(|error| panic!("`{token}` did not resolve: {error}"));
            let mut shape_worst = 0.0f32;
            for site in &sites {
                checked += 1;
                shape_worst =
                    shape_worst.max(distance_to_outline(site.position, &outline.commands));
            }
            if shape_worst > ON_THE_OUTLINE_TOLERANCE_PIXELS {
                measured.insert(token);
                worst = worst.max(shape_worst);
            } else {
                worst_on_the_outline = worst_on_the_outline.max(shape_worst);
            }
            assert!(
                shape_worst <= bound,
                "in the {orientation} box a connection site of `{token}` is {shape_worst} device \
                 pixels off its outline, past the {bound} that box's worst site accounts for"
            );
        }

        assert_eq!(
            checked, 856,
            "only {checked} of the 856 sites were measured"
        );
        assert_eq!(
            measured,
            tokens(SITES_OFF_THE_OUTLINE.iter().copied()),
            "in the {orientation} box, the presets with a connection site off their own outline \
             have changed"
        );
        println!(
            "{orientation}: {} of 173 presets keep every site on the outline, worst \
             {worst_on_the_outline} px; the other {} are off by at most {worst} px (bound {bound})",
            173 - measured.len(),
            measured.len()
        );
        // The bound bites, and the tolerance is not what makes this pass: the worst exception is
        // within a pixel of the bound, and the 165 that land on the outline do so thirty times
        // inside the tolerance rather than scraping it.
        assert!(
            worst > bound - 1.0,
            "in the {orientation} box the furthest site is {worst} off, well inside the {bound} \
             bound — the bound measures nothing"
        );
        assert!(
            worst_on_the_outline <= ON_THE_OUTLINE_TOLERANCE_PIXELS / 10.0,
            "in the {orientation} box the worst site that counts as on the outline is \
             {worst_on_the_outline} px off, within a tenth of the tolerance — the tolerance has \
             started doing the geometry's work"
        );
    }
}

#[test]
fn the_sites_outside_the_shapes_own_box_are_the_four_callout_tails() {
    for (index, (orientation, within, extents)) in orientations().into_iter().enumerate() {
        let (bounded, bound) = FURTHEST_A_SITE_LEAVES_THE_BOX[index];
        assert_eq!(bounded, orientation, "the bounds are out of step");
        let mut measured: BTreeSet<&'static str> = BTreeSet::new();
        let mut worst = 0.0f32;

        for definition in seeded_shapes() {
            let token = definition.preset.to_wire();
            let sites = preset_connection_sites(definition.preset, extents, &[], within)
                .unwrap_or_else(|error| panic!("`{token}`: {error}"));
            let mut shape_worst = 0.0f32;
            for site in &sites {
                assert!(
                    site.position.x.is_finite() && site.position.y.is_finite(),
                    "`{token}` produced a site at {:?}",
                    site.position
                );
                shape_worst = shape_worst
                    .max(within.left - site.position.x)
                    .max(site.position.x - within.right)
                    .max(within.top - site.position.y)
                    .max(site.position.y - within.bottom);
            }
            if shape_worst > ON_THE_OUTLINE_TOLERANCE_PIXELS {
                measured.insert(token);
                worst = worst.max(shape_worst);
            }
            assert!(
                shape_worst <= bound,
                "in the {orientation} box a connection site of `{token}` is {shape_worst} device \
                 pixels outside its own box"
            );
        }

        assert_eq!(
            measured,
            tokens(SITES_OUTSIDE_THE_BOX.iter().copied()),
            "in the {orientation} box, the presets with a connection site outside their own box \
             have changed"
        );
        assert!(
            (worst - bound).abs() <= 0.001,
            "in the {orientation} box the four callout tails reach {worst} px outside their box, \
             not the {bound} their shared adjustment defaults give"
        );
        println!(
            "{orientation}: the four callout tail sites sit exactly {worst} px outside their box"
        );
    }
}

// -------------------------------------------------------------------------------------------
// The angle is what makes a site more than a point
// -------------------------------------------------------------------------------------------

#[test]
fn a_literal_angle_and_a_guide_named_one_are_two_different_answers() {
    // Both arms of `PresetAngle`, from the table, each with the number it must produce.
    //
    // `rect`'s four sites name the circle constants: `3cd4` is 270° (up, because `y` grows
    // downward), `cd2` 180°, `cd4` 90°, and the fourth is the literal `0`. `gear6`'s are literals
    // in the wire scale: `19800000` is 330°.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());

    let rectangle = preset_connection_sites(PresetShapeType::Rectangle, extents, &[], within)
        .expect("`rect` has connection sites");
    let degrees: Vec<f64> = rectangle.iter().map(|site| site.angle.degrees()).collect();
    assert_eq!(
        degrees,
        vec![270.0, 180.0, 90.0, 0.0],
        "`rect`'s four sites no longer leave up, left, down and right"
    );

    let gear = preset_connection_sites(PresetShapeType::Gear6, extents, &[], within)
        .expect("`gear6` has connection sites");
    assert!(
        (gear[0].angle.degrees() - 330.0).abs() < 1e-6,
        "`gear6`'s first site leaves at {}°, not the 330° its literal `19800000` states",
        gear[0].angle.degrees()
    );

    // The downstream consequence, which is the half that matters: the angle is not a decoration on
    // a point. Two sites of the same shape at *different* positions have different angles, and a
    // resolver that dropped `@ang` would give them all the same one.
    let angles: BTreeSet<i64> = rectangle
        .iter()
        .map(|site| site.angle.degrees().round() as i64)
        .collect();
    assert_eq!(angles.len(), 4, "`rect`'s four sites share an angle");
}

#[test]
fn moving_an_adjustment_moves_a_connection_site() {
    // A site is *resolved*, not read off a constant. `wedgeRectCallout`'s fifth site is its tail
    // tip, whose position is `adj1`/`adj2` — so dragging the tail must drag the site with it, and a
    // resolver that ignored the overrides would answer the same point twice.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let at = |adjustment: f64| {
        preset_connection_sites(
            PresetShapeType::WedgeRectangleCallout,
            extents,
            &[AdjustmentOverride::new("adj1", adjustment)],
            within,
        )
        .expect("`wedgeRectCallout` resolves")[4]
            .position
    };
    let (left, right) = (at(-60_000.0), at(60_000.0));
    assert!(
        right.x - left.x > 100.0,
        "dragging the tail from -60 % to +60 % of the width moved the site from {left:?} to \
         {right:?}"
    );
    // The other four sites are the edge midpoints and must **not** have moved, which is what says
    // the difference above is the tail and not a global shift.
    let stationary = |adjustment: f64| {
        preset_connection_sites(
            PresetShapeType::WedgeRectangleCallout,
            extents,
            &[AdjustmentOverride::new("adj1", adjustment)],
            within,
        )
        .expect("resolves")[0]
            .position
    };
    assert_eq!(
        stationary(-60_000.0),
        stationary(60_000.0),
        "the top-edge site moved when only the tail's adjustment changed"
    );
}

// -------------------------------------------------------------------------------------------
// A defect is a defect, and a singularity is not
// -------------------------------------------------------------------------------------------

/// A shape whose connection site names a guide it never defines.
const A_SITE_NAMING_A_GUIDE_NOBODY_DEFINES: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "a triangle whose second connection site names a guide that is not in its guide list",
    adjustment_values: &[],
    guides: &[],
    text_rectangle: None,
    connection_sites: &[
        PresetConnectionSite {
            angle: PresetAngle::Native(0),
            position: PresetPoint::at("l", "t"),
        },
        PresetConnectionSite {
            angle: PresetAngle::Native(0),
            position: PresetPoint::at("r", "nothing_defines_this"),
        },
    ],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: &[
            PresetPathStep::MoveTo(PresetPoint::at("l", "t")),
            PresetPathStep::LineTo(PresetPoint::at("r", "b")),
            PresetPathStep::Close,
        ],
    }],
};

/// The same shape whose site names a guide the shape **does** define and whose formula has no
/// finite value — a singularity rather than a defect.
const A_SITE_READING_A_SINGULAR_GUIDE: PresetShapeDefinition = PresetShapeDefinition {
    guides: &[PresetGuide {
        wire_name: "over_nothing",
        formula: "*/ w h 0",
    }],
    connection_sites: &[PresetConnectionSite {
        angle: PresetAngle::Native(0),
        position: PresetPoint::at("l", "over_nothing"),
    }],
    ..A_SITE_NAMING_A_GUIDE_NOBODY_DEFINES
};

#[test]
fn a_site_naming_a_guide_the_shape_lacks_names_the_site_and_is_not_fillable() {
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let error =
        connection_sites_of_definition(&A_SITE_NAMING_A_GUIDE_NOBODY_DEFINES, extents, &[], within)
            .expect_err("a site naming an undefined guide is a table defect");
    // The **index** is the load-bearing part: a connector names a site by `a:cxn@idx`, so a failure
    // that did not say which site would send a reader to the wrong one.
    assert!(
        matches!(error, GeometryError::ConnectionSite { index: 1, .. }),
        "a site with an undefined guide answered {error}, not connection site 1"
    );
    assert_eq!(error.shape(), Some("rect"));
    assert!(
        !error.has_no_geometry_to_draw(),
        "a table defect was classified as `there is nothing to draw`"
    );
    println!("{error}");
}

#[test]
fn a_site_reading_a_singular_guide_is_the_shape_having_no_geometry_here() {
    // The other half of the distinction, and the one that decides whether a page renders: a guide
    // the table *defines* whose own formula has no finite value here is not a table defect. The
    // whole list fails rather than the site being dropped, because dropping site 4 renumbers site 5
    // and every connector after it attaches to the wrong side.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let error =
        connection_sites_of_definition(&A_SITE_READING_A_SINGULAR_GUIDE, extents, &[], within)
            .expect_err("a site reading a singular guide has no answer");
    assert!(
        matches!(&error, GeometryError::SingularGeometry { guide, .. } if guide == "over_nothing"),
        "a singular guide in a site answered {error}"
    );
    assert!(
        error.has_no_geometry_to_draw(),
        "a shape with no geometry at these adjustments answered with a failure no stand-in may fill"
    );
    println!("{error}");
}

#[test]
fn every_site_of_every_preset_survives_its_whole_adjustment_domain() {
    // The sweep: every preset, every adjustment at both ends of its domain, its middle, and well
    // outside it. Nothing may panic and nothing may put a `NaN` or an infinity into a position —
    // `SceneBuilder` would sanitise it to zero and the connector would attach to the page's corner.
    //
    // The failures that *are* allowed are named: a shape whose own formulas are singular here, and
    // nothing else.
    let (mut cases, mut resolved) = (0usize, 0usize);
    for (orientation, within, extents) in orientations() {
        let mut singular: BTreeSet<&'static str> = BTreeSet::new();

        for definition in seeded_shapes() {
            let preset = definition.preset;
            let token = preset.to_wire();
            let mut sets: Vec<Vec<AdjustmentOverride>> = vec![Vec::new()];
            if let Ok(domains) = adjustment_domains(preset, extents, &[]) {
                for domain in &domains {
                    for value in [
                        domain.minimum,
                        domain.maximum,
                        (domain.minimum + domain.maximum) / 2.0,
                        -1_000_000.0,
                        1_000_000.0,
                    ] {
                        sets.push(vec![AdjustmentOverride::new(domain.spec.wire_name, value)]);
                    }
                }
            }
            for adjustments in &sets {
                cases += 1;
                match preset_connection_sites(preset, extents, adjustments, within) {
                    Ok(sites) => {
                        resolved += sites.len();
                        for site in sites {
                            assert!(
                                site.position.x.is_finite()
                                    && site.position.y.is_finite()
                                    && site.angle.degrees().is_finite(),
                                "`{token}` at {adjustments:?} produced {site:?}"
                            );
                        }
                    }
                    Err(error) => {
                        assert!(
                        matches!(error, GeometryError::SingularGeometry { .. }),
                        "`{token}` at {adjustments:?} ({orientation}) failed with {error}, which \
                         is a defect in the table rather than a point its own formulas have no \
                         value at"
                    );
                        assert!(error.has_no_geometry_to_draw());
                        singular.insert(token);
                    }
                }
            }
        }

        assert_eq!(
        singular,
        tokens(SITES_SINGULAR_SOMEWHERE.iter().copied()),
        "in the {orientation} box, the presets whose connection sites have no value somewhere have \
         changed"
    );
        // The two statements the list's documentation makes, asserted rather than left as prose: a
        // singularity belongs to the *consumer* that reads the guide, not to the shape.
        assert!(
        singular.contains("parallelogram"),
        "`parallelogram`'s connection sites are no longer singular anywhere, so the difference \
         between this list and the paths' has gone"
    );
        assert!(
        !singular.contains("noSmoking"),
        "`noSmoking`'s connection sites are singular somewhere, which they were not — its `sqrt` \
         of a negative used to be confined to the band it draws"
    );
        println!(
            "{orientation}: {} presets singular somewhere: {singular:?}",
            singular.len()
        );
    }
    assert!(
        cases > 3_000,
        "the sweep visited only {cases} cases across the two orientations"
    );
    assert!(
        resolved > 14_000,
        "the sweep resolved only {resolved} connection sites"
    );
    println!("connection-site sweep: {cases} cases, {resolved} sites resolved");
}
