//! **Where every expected value in this crate came from**, declared row by row and printed on every
//! run.
//!
//! # Nobody ran Excel, or PowerPoint, or Word
//!
//! Not once, anywhere in this child. So a green suite must not be readable as *"this matches
//! Office"*, and the only way to stop it being read that way is to say, for each expectation, what
//! kind of thing it is:
//!
//! * **`SpecCode`** — the value is stated in ECMA-376 Part 1, or is a schema default. The strongest
//!   kind here, and it still says nothing about what Excel *renders*: the schema defines
//!   `c:gapWidth`'s default and not how wide a bar ends up.
//! * **`DocumentedBehaviour`** — the value has an external, checkable definition somewhere other
//!   than this repository: a published algorithm, an arithmetic identity, a statistic's definition.
//!   **These are the rows that are actually evidence.**
//! * **`EngineDerived`** — the expectation was read off this engine. It is a **change detector and
//!   not evidence about Office**, and every one of them is a candidate for the Windows sitting.
//!
//! MJXOFF-172 (R17) split 31 / 110 / 52, MJXOFF-174 (R19) opened the Word crate at 13 / 15 / 19, and
//! MJXOFF-177 (R22) closed it at 14 / 14 / 27.
//!
//! # This child's `DocumentedBehaviour` tier is unusually strong, and the reason is nameable
//!
//! **Charts are arithmetic.** Tick selection has a published algorithm — Heckbert's *Nice Numbers
//! for Graph Labels*, Graphics Gems I, 1990 — which almost every plotting library implements and
//! which anybody can check. A least-squares fit, a standard deviation, a standard error, a
//! logarithm and the area of a circle are all definitions rather than choices. A pie's slices sum
//! to a turn because a circle has 360 degrees. None of that is a fact about Office, and all of it is
//! checkable against something that is not this repository, which is exactly what the middle tier
//! means.
//!
//! **What Office does is a different question, and it is where the `EngineDerived` rows are.** Where
//! a tick label sits relative to its axis, how much of a frame a legend takes, how big a marker is,
//! how far a bubble's radius grows — none of these is published anywhere, and each is this engine
//! agreeing with itself until somebody opens the file in Excel.
//!
//! # The citations are to the XSDs, as MJXOFF-177 established
//!
//! `dml-chart.xsd` is in `References/` as text and was read. ECMA-376 Part 1's prose is there only
//! as a five thousand page PDF and was not, so a value that lives only in the prose is an
//! `EngineDerived` row here with the reading written out, where an earlier child would have cited a
//! section and called it `SpecCode`. That is the standard R22 set and this child keeps it.

use std::collections::BTreeMap;

/// Where one expected value came from.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Provenance {
    /// ECMA-376 Part 1, or a schema default.
    SpecCode,
    /// An external, checkable definition that is not this repository's.
    DocumentedBehaviour,
    /// Read off this engine. A change detector, not evidence.
    EngineDerived,
}

/// One expectation a suite in this crate asserts.
struct Row {
    /// Which suite asserts it.
    suite: &'static str,
    /// What it is about.
    subject: &'static str,
    /// Where the number or the behaviour came from.
    provenance: Provenance,
    /// The citation, or the reason the value is only this engine's.
    because: &'static str,
}

/// Every expectation this crate's suites rest on.
///
/// Hand-maintained, deliberately: a table generated from the tests would say what the tests say and
/// could not say where the tests got it. Adding an assertion without adding a row here is a silent
/// claim about Office.
const LEDGER: &[Row] = &[
    // ---------------------------------------------------------------------------------------
    // SpecCode — the schema says the value.
    // ---------------------------------------------------------------------------------------
    Row {
        suite: "every_family_produces_marks",
        subject: "the sixteen plot elements `CT_PlotArea` admits",
        provenance: Provenance::SpecCode,
        because: "`dml-chart.xsd`'s `CT_PlotArea` choice group, through `mjx_chart::ChartKind`",
    },
    Row {
        suite: "every_family_produces_marks",
        subject: "a scatter and a bubble series carry `c:xVal`/`c:yVal` rather than `c:cat`/`c:val`",
        provenance: Provenance::SpecCode,
        because: "`dml-chart.xsd`'s `CT_ScatterSer` and `CT_BubbleSer`",
    },
    Row {
        suite: "every_family_produces_marks",
        subject: "`c:holeSize` is a percentage of the outer radius",
        provenance: Provenance::SpecCode,
        because: "`dml-chart.xsd`'s `CT_HoleSize`, whose value is a `ST_HoleSize` percentage",
    },
    Row {
        suite: "series_geometry_is_asserted",
        subject: "`c:grouping` admits `clustered`, `stacked`, `percentStacked` and `standard`",
        provenance: Provenance::SpecCode,
        because: "`dml-chart.xsd`'s `ST_BarGrouping` and `ST_Grouping`, which differ by exactly \
                  `clustered`",
    },
    Row {
        suite: "series_geometry_is_asserted",
        subject: "a bar plot with no `c:grouping` is clustered",
        provenance: Provenance::SpecCode,
        because: "`CT_BarGrouping@val` defaults to `clustered` in `dml-chart.xsd`",
    },
    Row {
        suite: "series_geometry_is_asserted",
        subject: "`c:barDir` decides whether bars run across or up",
        provenance: Provenance::SpecCode,
        because: "`ST_BarDir`'s two values, `bar` and `col`",
    },
    Row {
        suite: "a_gap_in_the_data_is_a_hole_and_not_a_shift (series_geometry_is_asserted)",
        subject: "a `c:pt` is addressed by its `c:idx` and not by its position",
        provenance: Provenance::SpecCode,
        because: "`CT_NumVal@idx` is required; a cache is sparse and a blank writes no `c:pt`",
    },
    Row {
        suite: "the_furniture_is_drawn",
        subject: "`c:tickLblSkip` draws every nth label",
        provenance: Provenance::SpecCode,
        because: "`CT_Skip@val`, an `xsd:unsignedInt` on a category or date axis",
    },
    Row {
        suite: "the_furniture_is_drawn",
        subject: "`c:majorGridlines` and `c:minorGridlines` are separate elements",
        provenance: Provenance::SpecCode,
        because: "`EG_AxShared` declares both, so a file can rule one and not the other",
    },
    Row {
        suite: "the_furniture_is_drawn",
        subject: "the five error-bar value types and the three bar types",
        provenance: Provenance::SpecCode,
        because: "`ST_ErrValType` and `ST_ErrBarType` in `dml-chart.xsd`",
    },
    Row {
        suite: "the_furniture_is_drawn",
        subject: "`c:noEndCap` decides whether an error bar is capped",
        provenance: Provenance::SpecCode,
        because: "`CT_Boolean` on `CT_ErrBars`",
    },
    Row {
        suite: "ticks_are_arithmetic",
        subject: "a stated `c:min`, `c:max` and `c:majorUnit` are used exactly",
        provenance: Provenance::SpecCode,
        because: "`CT_Scaling`'s `c:min`/`c:max` and `EG_AxShared`'s `c:majorUnit` are values the \
                  file states; overriding one would override the document",
    },
    Row {
        suite: "ticks_are_arithmetic",
        subject: "`c:logBase` makes an axis logarithmic",
        provenance: Provenance::SpecCode,
        because: "`CT_LogBase@val`, a double between 2 and 1000",
    },
    Row {
        suite: "degenerate_data_is_defined",
        subject: "`c:dispBlanksAs` admits `gap`, `zero` and `span`, and defaults to `gap`",
        provenance: Provenance::SpecCode,
        because: "`ST_DispBlanksAs`; the default is what makes a missing cell a hole",
    },
    Row {
        suite: "the_document_palette_wins",
        subject: "a series with no `c:spPr` states no fill at all",
        provenance: Provenance::SpecCode,
        because: "`CT_ShapeProperties` is `minOccurs=\"0\"` on every `CT_*Ser`, and an absent one \
                  is an absent fill rather than a black one",
    },
    Row {
        suite: "a_hierarchy_is_laid_out",
        subject: "the ten members of `ST_AlgorithmType`",
        provenance: Provenance::SpecCode,
        because: "`dml-diagram.xsd`, through `mjx_ooxml_types::diagram::AlgorithmType`",
    },
    Row {
        suite: "a_hierarchy_is_laid_out",
        subject: "a `parOf` connection is the parent-child edge and `presOf` is not",
        provenance: Provenance::SpecCode,
        because: "`ST_CxnType`'s four values; `presOf` binds a presentation node to a data point",
    },
    Row {
        suite: "a_hierarchy_is_laid_out",
        subject: "only `node` and `asst` points are drawn",
        provenance: Provenance::SpecCode,
        because: "`ST_PtType`; `pres` is the presentation graph, `doc` the root, and the two \
                  transition kinds are connectors",
    },
    Row {
        suite: "a_hierarchy_is_laid_out",
        subject: "`dgm:cxn@srcOrd` ranks a child among its siblings",
        provenance: Provenance::SpecCode,
        because: "`CT_Cxn@srcOrd` is required and is what makes sibling order a fact of the file",
    },
    // ---------------------------------------------------------------------------------------
    // DocumentedBehaviour — an external, checkable definition.
    // ---------------------------------------------------------------------------------------
    Row {
        suite: "ticks_are_arithmetic",
        subject: "the nice-number mantissa set is {1, 2, 5, 10} scaled by a power of ten",
        provenance: Provenance::DocumentedBehaviour,
        because: "Paul Heckbert, *Nice Numbers for Graph Labels*, Graphics Gems I (Academic Press, \
                  1990), pp. 61–63",
    },
    Row {
        suite: "ticks_are_arithmetic",
        subject: "`nicenum`'s two roundings — up for a range, nearest for a step",
        provenance: Provenance::DocumentedBehaviour,
        because: "the same paper's `round` parameter, with its two published thresholds tables",
    },
    Row {
        suite: "ticks_are_arithmetic",
        subject: "the axis extends outward to whole multiples of its step",
        provenance: Provenance::DocumentedBehaviour,
        because: "the same paper's `loose_label`: `graphmin = floor(min/d)*d`, \
                  `graphmax = ceil(max/d)*d`",
    },
    Row {
        suite: "ticks_are_arithmetic",
        subject: "a logarithmic axis ticks at whole powers of its base",
        provenance: Provenance::DocumentedBehaviour,
        because: "the definition of a logarithm: equal distances are equal ratios, so the round \
                  values are the powers",
    },
    Row {
        suite: "ticks_are_arithmetic",
        subject: "100 is halfway from 1 to 10,000 on a base-ten axis",
        provenance: Provenance::DocumentedBehaviour,
        because: "log₁₀100 = 2 is the mean of 0 and 4 — arithmetic, and the check that the log \
                  mapping is a log mapping",
    },
    Row {
        suite: "ticks_are_arithmetic",
        subject: "a whole multiple of a step is the double a reader writes, not an accumulation",
        provenance: Provenance::DocumentedBehaviour,
        because: "IEEE 754: 0.002 is not representable, so nine additions of it are not 0.018 and \
                  9/1000 is; the suite asserts both halves",
    },
    Row {
        suite: "series_geometry_is_asserted",
        subject: "a value of 40 draws four times the height of a value of 10",
        provenance: Provenance::DocumentedBehaviour,
        because: "the definition of a linear scale over a zero-based axis; a bar chart that failed \
                  this would misrepresent every bar on it",
    },
    Row {
        suite: "series_geometry_is_asserted",
        subject: "a hundred-percent stack of 3 and 1 fills three quarters and one quarter",
        provenance: Provenance::DocumentedBehaviour,
        because: "arithmetic: 3/(3+1); the axis is a proportion by construction",
    },
    Row {
        suite: "series_geometry_is_asserted",
        subject: "a pie's slices sweep 360 degrees in proportion to their values",
        provenance: Provenance::DocumentedBehaviour,
        because: "the definition of a pie chart, and of a turn",
    },
    Row {
        suite: "series_geometry_is_asserted",
        subject: "three collinear data points draw three collinear markers",
        provenance: Provenance::DocumentedBehaviour,
        because: "an affine map takes lines to lines; the assertion is that the projection is affine",
    },
    Row {
        suite: "every_family_produces_marks",
        subject: "a bubble of four times the size has twice the radius",
        provenance: Provenance::DocumentedBehaviour,
        because: "the area of a circle is πr², so equal-area scaling is the square root; \
                  `c:sizeRepresents` names `area` as the alternative to `w`",
    },
    Row {
        suite: "the_furniture_is_drawn",
        subject: "a least-squares line through 10, 20, 30, 40 is monotone increasing",
        provenance: Provenance::DocumentedBehaviour,
        because: "the normal equations have one solution for a perfectly linear sample, and it is \
                  the sample",
    },
    Row {
        suite: "the_furniture_is_drawn",
        subject: "a `percentage` error bar of ten is a tenth of the point's own value",
        provenance: Provenance::DocumentedBehaviour,
        because: "the definition of a percentage; the alternative readings (a tenth of the axis, a \
                  tenth of the range) would each be a different statistic",
    },
    Row {
        suite: "a_hierarchy_is_laid_out",
        subject: "a branch with three leaves is three times as wide as one with a single leaf",
        provenance: Provenance::DocumentedBehaviour,
        because: "the definition of proportional allocation; an equal split would give both the \
                  same width, which is what the uneven fixture is there to tell apart",
    },
    // ---------------------------------------------------------------------------------------
    // EngineDerived — this engine agreeing with itself.
    // ---------------------------------------------------------------------------------------
    Row {
        suite: "ticks_are_arithmetic",
        subject: "the step is derived from the raw span rather than from a nicened span",
        provenance: Provenance::EngineDerived,
        because: "Heckbert rounds twice and overshoots — for −45…1203 it gives a lower bound of \
                  −500 where the data starts at −45 — so this engine deviates and the deviation is \
                  a decision nobody has checked against Excel",
    },
    Row {
        suite: "ticks_are_arithmetic",
        subject: "a value axis targets one major tick per 0.45 inch of plot",
        provenance: Provenance::EngineDerived,
        because: "Office scales its tick count with the plot's size — a chart resized in PowerPoint \
                  re-labels itself — and no figure for the spacing is published anywhere",
    },
    Row {
        suite: "ticks_are_arithmetic",
        subject: "a floating axis snaps to zero when its minimum is under five sixths of its maximum",
        provenance: Provenance::EngineDerived,
        because: "the ratio is the one most widely reported for Excel's own rule and is not stated \
                  in ECMA-376; the site carries a `GUESS:` marker",
    },
    Row {
        suite: "ticks_are_arithmetic",
        subject: "the minor unit is a fifth of the major one",
        provenance: Provenance::EngineDerived,
        because: "no schema states a default minor unit; a fifth is what a 1/2/5 major sequence \
                  divides into evenly and what Office appears to draw",
    },
    Row {
        suite: "ticks_are_arithmetic",
        subject: "a tick count more than half again the target steps the step up one nice number",
        provenance: Provenance::EngineDerived,
        because: "the allowance is this engine's own and bounds the loop; nothing published says \
                  how many ticks are too many",
    },
    Row {
        suite: "series_geometry_is_asserted",
        subject: "a clustered bar's width is the band over `(occupied + gap)`",
        provenance: Provenance::EngineDerived,
        because: "`c:gapWidth` and `c:overlap` are percentages of *a bar's width*, and the formula \
                  that turns the pair into a share of the band is not stated in the schema",
    },
    Row {
        suite: "series_geometry_is_asserted",
        subject: "a stacked bar with no `c:overlap` is treated as fully overlapped",
        provenance: Provenance::EngineDerived,
        because: "the schema default is zero, which would draw a stack as a cluster; Office writes \
                  100 for a stacked plot and this engine assumes it when the file does not say",
    },
    Row {
        suite: "series_geometry_is_asserted",
        subject: "a category axis's points sit at band centres unless `c:crossBetween` says otherwise",
        provenance: Provenance::EngineDerived,
        because: "the schema states the element and not the default a line chart takes; this is \
                  what makes a line chart's first point sit inside the plot rather than on its edge",
    },
    Row {
        suite: "every_family_produces_marks",
        subject: "a doughnut's series are concentric rings of equal thickness, innermost first",
        provenance: Provenance::EngineDerived,
        because: "the schema states `c:holeSize` and says nothing about how several series share \
                  the annulus",
    },
    Row {
        suite: "every_family_produces_marks",
        subject: "a radar plot's first spoke points straight up and they run clockwise",
        provenance: Provenance::EngineDerived,
        because: "the schema states no starting angle for a radar plot at all, unlike a pie's \
                  `c:firstSliceAng`",
    },
    Row {
        suite: "every_family_produces_marks",
        subject: "a stock plot's first series carries the high-low line and its fourth the open-close bar",
        provenance: Provenance::EngineDerived,
        because: "`c:stockChart` states no roles for its series; the by-position convention is \
                  Excel's own and is what every producer writes, but nothing in the file says so",
    },
    Row {
        suite: "every_family_produces_marks",
        subject: "the largest bubble spans an eighth of the plot's shorter side at 100% scale",
        provenance: Provenance::EngineDerived,
        because: "`c:bubbleScale` is a percentage *of a default* that ECMA-376 does not state",
    },
    Row {
        suite: "every_family_produces_marks",
        subject: "a three-dimensional plot is laid out exactly as its flat one",
        provenance: Provenance::EngineDerived,
        because: "`c:view3D`'s rotation, perspective and depth axis are not read at all; this is a \
                  stated deferral rather than a rendering decision",
    },
    Row {
        suite: "every_family_produces_marks",
        subject: "the two surface plots plot no data",
        provenance: Provenance::EngineDerived,
        because: "a surface is a mesh over a grid of categories and series, and no flat projection \
                  of one is honest; drawing its rows as lines would be a different chart",
    },
    Row {
        suite: "the_furniture_is_drawn",
        subject: "the chart title is fourteen points and everything else is ten",
        provenance: Provenance::EngineDerived,
        because: "chart text sizes live in a `c:txPr` the file may or may not carry, and ECMA-376 \
                  states no default for any of them",
    },
    Row {
        suite: "the_furniture_is_drawn",
        subject: "a tick label sits one tick-length outside its axis line",
        provenance: Provenance::EngineDerived,
        because: "the gap between an axis and its labels is not stated anywhere; this is the \
                  spacing that reads",
    },
    Row {
        suite: "the_furniture_is_drawn",
        subject: "the frame margin is five points and the gap between furniture is three",
        provenance: Provenance::EngineDerived,
        because: "`c:plotArea > c:layout` states a rectangle when a reader has dragged one and \
                  nothing states the inset when they have not",
    },
    Row {
        suite: "the_furniture_is_drawn",
        subject: "a vertical axis title is turned a quarter turn anticlockwise",
        provenance: Provenance::EngineDerived,
        because: "the direction of the turn is not stated in the schema; the *rotation* would come \
                  from `c:title > c:txPr > a:bodyPr@rot`, which this engine does not read",
    },
    Row {
        suite: "the_furniture_is_drawn",
        subject: "a data label sits above its column and outside its slice",
        provenance: Provenance::EngineDerived,
        because: "`c:dLblPos` states a position when the file carries one and this engine does not \
                  yet read it; where a `bestFit` label goes is unpublished in any case",
    },
    Row {
        suite: "the_furniture_is_drawn",
        subject: "a legend's swatch is nine points wide and precedes its name",
        provenance: Provenance::EngineDerived,
        because: "nothing states a swatch's size; the order is what every chart draws and the size \
                  is this engine's",
    },
    Row {
        suite: "the_furniture_is_drawn",
        subject: "a trendline is sampled at thirty-three points",
        provenance: Provenance::EngineDerived,
        because: "the sample count decides how smooth a fitted curve looks and is a rendering \
                  choice nothing states",
    },
    Row {
        suite: "the_furniture_is_drawn",
        subject: "an error bar past the axis' own maximum is clamped rather than extending the axis",
        provenance: Provenance::EngineDerived,
        because: "whether Excel widens an axis to fit an error bar is not stated and was not \
                  checked; the clamp is this engine's reading",
    },
    Row {
        suite: "fragments_reach_the_tree",
        subject: "the plot-area negotiation is a two-pass fixed point",
        provenance: Provenance::EngineDerived,
        because: "the loop between tick count and gutter width has to be bounded somewhere and two \
                  is where this engine bounds it; nothing states how Office resolves it",
    },
    Row {
        suite: "fragments_reach_the_tree",
        subject: "a chart's handles are numbered from 1 << 32",
        provenance: Provenance::EngineDerived,
        because: "the base keeps a chart's handles out of its host's numbering space and is this \
                  workspace's own arrangement, not anything a file states",
    },
    Row {
        suite: "fragments_reach_the_tree",
        subject: "chart text is measured by a per-character advance table rather than by shaping",
        provenance: Provenance::EngineDerived,
        because: "the ratios are read off Calibri and grouped by this engine; a host that hands in \
                  a real font engine changes every reservation, which is what the seam is for",
    },
    Row {
        suite: "degenerate_data_is_defined",
        subject: "a pie of all-zero values divides the circle equally",
        provenance: Provenance::EngineDerived,
        because: "nothing states what a pie with no total draws; equal slices is what Office \
                  appears to draw and what stops a pie of zeroes being a blank rectangle",
    },
    Row {
        suite: "degenerate_data_is_defined",
        subject: "a single distinct value scales its axis from zero to that value",
        provenance: Provenance::EngineDerived,
        because: "an axis whose ends coincide would divide by zero; reaching zero is what makes a \
                  one-point bar chart draw a bar rather than a line",
    },
    Row {
        suite: "degenerate_data_is_defined",
        subject: "a `c:idx` above 32,000 is dropped rather than allocated for",
        provenance: Provenance::EngineDerived,
        because: "Excel's own limit is 32,000 points per series and a `c:idx` is an unsigned int; \
                  the ceiling is this engine's protection against an out-of-memory kill and not a \
                  rendering rule",
    },
    Row {
        suite: "degenerate_data_is_defined",
        subject: "an inverted stated scale is widened rather than refused",
        provenance: Provenance::EngineDerived,
        because: "a `c:max` below its `c:min` describes no axis, and nothing states what a \
                  consumer should do with one",
    },
    Row {
        suite: "a_hierarchy_is_laid_out",
        subject: "a hierarchy gets one row per depth with a fifth of each span as gutter",
        provenance: Provenance::EngineDerived,
        because: "`dgm:constrLst` states the real gutters and this engine evaluates none of the \
                  constraint language; the fifth stands for all of them",
    },
    Row {
        suite: "a_hierarchy_is_laid_out",
        subject: "a cycle's ring has a radius of a third of the frame",
        provenance: Provenance::EngineDerived,
        because: "the `cycle` algorithm's radius is a constraint in the layout definition this \
                  engine does not read",
    },
    Row {
        suite: "a_hierarchy_is_laid_out",
        subject: "a cyclic `parOf` graph is broken at the first point the walk restarts from",
        provenance: Provenance::EngineDerived,
        because: "a `parOf` cycle is malformed and nothing states what to draw for one; this is a \
                  choice that keeps the walk terminating",
    },
    Row {
        suite: "no_panic_on_a_layout_path",
        subject: "a malformed chart part is one frame drawn empty rather than a failed page",
        provenance: Provenance::EngineDerived,
        because: "refusing a whole slide over one bad chart would lose every other shape on it; \
                  which of the two Office does was not checked",
    },
];

/// How many rows of each kind.
fn split() -> BTreeMap<Provenance, usize> {
    let mut counts = BTreeMap::new();
    for row in LEDGER {
        *counts.entry(row.provenance).or_insert(0) += 1;
    }
    counts
}

#[test]
fn the_split_is_printed_and_asserted_in_both_directions() {
    let counts = split();
    println!("\nProvenance of every expected value in `mjx-layout-chart`:\n");
    for (provenance, count) in &counts {
        println!("  {provenance:>20?}  {count:>3}");
    }
    println!("  {:>20}  {:>3}\n", "total", LEDGER.len());
    println!(
        "  Nobody ran Excel, PowerPoint or Word. `EngineDerived` rows are change detectors and"
    );
    println!(
        "  are not evidence about Office; every one is a candidate for the Windows sitting.\n"
    );

    let spec = counts.get(&Provenance::SpecCode).copied().unwrap_or(0);
    let documented = counts
        .get(&Provenance::DocumentedBehaviour)
        .copied()
        .unwrap_or(0);
    let engine = counts.get(&Provenance::EngineDerived).copied().unwrap_or(0);

    // **Both directions.** A ledger with only a floor would pass for a build that had quietly
    // relabelled everything as `SpecCode`; one with only a ceiling would pass for a build that had
    // stopped asserting anything at all.
    assert_eq!(spec + documented + engine, LEDGER.len());
    assert!(
        spec >= 19,
        "the schema really does state this many of them: {spec}"
    );
    assert!(
        documented >= 14,
        "these are the rows that are evidence, and there must be some: {documented}"
    );
    assert!(
        engine >= 27,
        "and this many are only this engine agreeing with itself — a count that *fell* would mean \
         somebody had relabelled a guess: {engine}"
    );
    assert!(
        engine > documented,
        "if the guesses ever stopped outnumbering the evidence it would be because somebody had \
         run Office, and this assertion is the place to record that: {engine} against {documented}"
    );
}

/// Every suite named in the ledger exists, so a row cannot outlive the assertion it describes.
#[test]
fn every_suite_the_ledger_names_exists() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    for row in LEDGER {
        // A row may name a single test inside a suite as `test_name (suite)`, which is how one row
        // points at the assertion it is about rather than at the whole file.
        let suite = row
            .suite
            .rsplit_once(" (")
            .map_or(row.suite, |(_, file)| file.trim_end_matches(')'));
        let path = directory.join(format!("{suite}.rs"));
        assert!(
            path.exists(),
            "the ledger names `{}`, which is not a suite in this crate",
            row.suite
        );
    }
}

/// Every `EngineDerived` row says *why* it is only this engine's, which is what makes the sitting's
/// list writable from this file alone.
#[test]
fn every_engine_derived_row_says_why() {
    for row in LEDGER {
        if row.provenance != Provenance::EngineDerived {
            continue;
        }
        assert!(
            row.because.len() > 30,
            "`{}` is a change detector and the reason must be usable by whoever runs Office",
            row.subject
        );
    }
}

/// Every citation in the two upper tiers names something outside this repository.
///
/// A `SpecCode` row whose reason was *"this is how the engine does it"* would be an `EngineDerived`
/// row wearing a better label, and relabelling is the one way a ledger can lie.
#[test]
fn every_citation_names_something_outside_this_repository() {
    for row in LEDGER {
        if row.provenance == Provenance::EngineDerived {
            continue;
        }
        let cites_outside = row.because.contains("xsd")
            || row.because.contains("ST_")
            || row.because.contains("CT_")
            || row.because.contains("EG_")
            || row.because.contains("ECMA")
            || row.because.contains("Heckbert")
            || row.because.contains("Graphics Gems")
            || row.because.contains("IEEE")
            || row.because.contains("paper")
            || row.because.contains("definition")
            || row.because.contains("arithmetic")
            || row.because.contains("area of a circle")
            || row.because.contains("affine")
            || row.because.contains("normal equations")
            || row.because.contains("logarithm");
        assert!(
            cites_outside,
            "`{}` is in the {:?} tier and its reason cites nothing outside this repository: {}",
            row.subject, row.provenance, row.because
        );
    }
}
