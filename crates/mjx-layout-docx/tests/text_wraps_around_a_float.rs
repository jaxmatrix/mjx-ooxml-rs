//! Text wrapping, and the identity value that makes it so easy to ship unimplemented.
//!
//! # The trap, stated
//!
//! **A float with `wp:wrapNone`, or one positioned outside the text column, changes no line.** A
//! fixture made only of those is green for an engine whose wrapping code has never run — every
//! assertion about the text passes, because the text is exactly where it would have been. So this
//! suite asserts **line measures**, in EMU, and every wrapping fixture is paired with a control that
//! must produce a *different* number.
//!
//! # And the second trap: a bounding box is almost right
//!
//! `wp:wrapTight` and `wp:wrapThrough` carry a real polygon, and a triangle standing on its point
//! leaves almost the whole measure at its apex and almost none at its base. An engine that
//! subtracted the object's bounding box instead would give **every** line beside the object the same
//! width — which looks plausible and is wrong. So the polygon fixtures assert that the measure
//! *varies down the object*, and [`the_bounding_box_reading_is_provably_different`] substitutes the
//! box and asserts the numbers change: the gate can fail, which is the only thing that makes it a
//! gate.

mod support;

use mjx_layout::BoxModel;
use mjx_layout_docx::{
    free_runs, span_in_band, Exclusion, WrapSide, BAND_COMPOSITIONS, WRAP_POLYGON_UNITS,
};
use mjx_ooxml_core::measure::Emu;
use support::{
    cell, constraints, document, float_exclusions, floating_paragraph, flow, lay_out_one_around,
    model, offset_position, page_of_each_paragraph, paragraph, polygon_wrap, row, square_wrap,
    table, text_cell, walk,
};

/// Enough words that a paragraph beside anything produces a dozen lines.
const BODY: &str = "The quick brown fox jumps over the lazy dog and then it turns around and \
    jumps back over the very same lazy dog again while a second fox watches from the hedge and \
    considers whether the whole business is worth the effort it so plainly requires of everyone.";

/// A triangle standing on its point, in the `0..21600` space Word writes a wrap polygon in: wide at
/// the top, a single point at the bottom.
const APEX_DOWN: [(i64, i64); 4] = [(0, 0), (21_600, 0), (10_800, 21_600), (0, 0)];

/// An object one inch square, two inches down the paragraph.
fn one_inch_square(wrap: &str) -> String {
    floating_paragraph("", BODY, 914_400, 914_400, &offset_position(0, 0), wrap)
}

/// The measure of every line of [`BODY`] beside `markup`'s own float, in a six-inch column.
fn measures(markup: String) -> Vec<i64> {
    let exclusions = float_exclusions(markup, 6.0);
    lay_out_one_around("", BODY, 6.0, &exclusions)
        .lines
        .iter()
        .map(|line| line.measure.emu())
        .collect()
}

#[test]
fn a_square_wrap_narrows_the_lines_it_meets_and_a_none_wrap_does_not() {
    let full = Emu::from_inches(6.0).emu();
    let with_float = measures(one_inch_square(&square_wrap("bothSides")));
    let without = measures(one_inch_square(r#"<wp:wrapNone/>"#));

    // **The identity value, stated as one.** `wp:wrapNone` contributes no exclusion at all, so every
    // line keeps the whole measure; if this assertion is ever green for the square wrap too, the
    // wrapping code has not run.
    assert!(
        without.iter().all(|width| *width == full),
        "wp:wrapNone displaces nothing: {without:?}"
    );
    assert!(
        with_float.iter().any(|width| *width < full),
        "a square wrap must narrow some line: {with_float:?}"
    );
    assert_ne!(
        with_float, without,
        "the two wrap modes must not produce the same lines"
    );
    // And the amount is the object's own width, not some fraction of it.
    assert_eq!(
        with_float.first().copied(),
        Some(full - Emu::from_inches(1.0).emu()),
        "the first line loses exactly the inch the object occupies"
    );
}

#[test]
fn a_polygon_wrap_produces_a_measure_that_varies_down_the_object() {
    // A triangle apex-down: the lines near its top meet the whole base and are narrow; the lines
    // near its point meet almost nothing and are wide. A bounding box gives them all one width.
    let markup = floating_paragraph(
        "",
        BODY,
        914_400,
        1_828_800,
        &offset_position(0, 0),
        &polygon_wrap("bothSides", false, &APEX_DOWN),
    );
    let measures = measures(markup);

    let distinct: std::collections::BTreeSet<i64> = measures
        .iter()
        .copied()
        .filter(|width| *width < Emu::from_inches(6.0).emu())
        .collect();
    assert!(
        distinct.len() >= 2,
        "a polygon wrap gives the lines beside it more than one width: {measures:?}"
    );
    // And the direction is the polygon's: further down the triangle is narrower, so the line's own
    // measure is wider.
    let narrowed: Vec<i64> = measures
        .iter()
        .copied()
        .filter(|width| *width < Emu::from_inches(6.0).emu())
        .collect();
    assert!(
        narrowed.last() > narrowed.first(),
        "the measure grows as the triangle narrows: {narrowed:?}"
    );
}

#[test]
fn the_bounding_box_reading_is_provably_different() {
    // **The proof that the gate above can fail.** The same object, read two ways: as its polygon,
    // and as the bounding box an engine that ignored `wp:wrapPolygon` would use. If the two agreed,
    // the assertion above would be green for an implementation that never looked at the polygon.
    let width = Emu::from_inches(1.0);
    let height = Emu::from_inches(2.0);
    let polygon = Exclusion::from_polygon(
        Emu::ZERO,
        Emu::ZERO,
        width,
        height,
        width,
        height,
        &APEX_DOWN,
        WrapSide::BothSides,
        false,
    );
    let box_reading =
        Exclusion::rectangle(Emu::ZERO, Emu::ZERO, width, height, WrapSide::BothSides);

    let band = |exclusion: &Exclusion, top: f64| {
        free_runs(
            Emu::ZERO,
            Emu::from_inches(6.0),
            Emu::from_inches(top),
            Emu::from_inches(top + 0.2),
            std::slice::from_ref(exclusion),
        )
    };

    let near_the_base = band(&polygon, 0.1);
    let near_the_apex = band(&polygon, 1.7);
    assert_ne!(
        near_the_base, near_the_apex,
        "the polygon leaves different room at the two ends of the object"
    );

    assert_eq!(
        band(&box_reading, 0.1),
        band(&box_reading, 1.7),
        "the bounding box leaves the same room everywhere — which is exactly the wrong answer"
    );
    assert_ne!(
        near_the_apex,
        band(&box_reading, 1.7),
        "and near the apex the two readings disagree, so the fixture above can fail"
    );
}

#[test]
fn a_through_wrap_lets_text_into_a_concavity_and_a_tight_wrap_does_not() {
    // A shape like a `U` rotated on to its side, opening to the right: a concavity that reaches the
    // object's own edge. `wp:wrapThrough` lets text into it; `wp:wrapTight` follows the outline.
    // Two uprights joined at the bottom — a scanline through the middle crosses the outline four
    // times and finds a genuine hole between them, which is the only shape that tells the two
    // elements apart at all.
    let cup: [(i64, i64); 8] = [
        (0, 0),
        (6_000, 0),
        (6_000, 15_000),
        (15_600, 15_000),
        (15_600, 0),
        (21_600, 0),
        (21_600, 21_600),
        (0, 21_600),
    ];
    let width = Emu::from_inches(2.0);
    let height = Emu::from_inches(2.0);
    let make = |through: bool| {
        Exclusion::from_polygon(
            Emu::ZERO,
            Emu::ZERO,
            width,
            height,
            width,
            height,
            &cup,
            WrapSide::BothSides,
            through,
        )
    };
    // A band inside the two uprights and clear of the joining bar.
    let middle_top = Emu::from_inches(0.5);
    let middle_bottom = Emu::from_inches(0.7);

    let tight = span_in_band(&make(false), middle_top, middle_bottom);
    let through = span_in_band(&make(true), middle_top, middle_bottom);
    assert_eq!(
        tight.len(),
        1,
        "a tight wrap collapses the band to one blocked interval: {tight:?}"
    );
    assert!(
        through.len() >= 2,
        "a through wrap leaves the hole between the uprights open: {through:?} against {tight:?}"
    );
    // And a line composed in that band therefore *has* somewhere to go that the tight reading denies
    // it: the gap between the uprights.
    let free_through = free_runs(
        Emu::ZERO,
        Emu::from_inches(6.0),
        middle_top,
        middle_bottom,
        std::slice::from_ref(&make(true)),
    );
    let free_tight = free_runs(
        Emu::ZERO,
        Emu::from_inches(6.0),
        middle_top,
        middle_bottom,
        std::slice::from_ref(&make(false)),
    );
    assert!(
        free_through.len() > free_tight.len(),
        "through: {free_through:?}, tight: {free_tight:?}"
    );
}

#[test]
fn the_largest_side_rule_puts_the_text_on_one_side_only() {
    // An object left of centre. `largest` must leave the run to its **right** and nothing else.
    let object = Exclusion::rectangle(
        Emu::from_inches(0.5),
        Emu::ZERO,
        Emu::from_inches(1.5),
        Emu::from_inches(1.0),
        WrapSide::Largest,
    );
    let runs = free_runs(
        Emu::ZERO,
        Emu::from_inches(6.0),
        Emu::from_inches(0.1),
        Emu::from_inches(0.3),
        std::slice::from_ref(&object),
    );
    assert_eq!(runs.len(), 1, "largest leaves exactly one run: {runs:?}");
    assert_eq!(
        runs[0],
        (Emu::from_inches(1.5), Emu::from_inches(6.0)),
        "and it is the wider one, to the object's right"
    );

    // The control: the same object wrapping both sides leaves two runs, so the rule above is doing
    // something rather than describing a geometry that has only one answer.
    let both = Exclusion::rectangle(
        Emu::from_inches(0.5),
        Emu::ZERO,
        Emu::from_inches(1.5),
        Emu::from_inches(1.0),
        WrapSide::BothSides,
    );
    let runs = free_runs(
        Emu::ZERO,
        Emu::from_inches(6.0),
        Emu::from_inches(0.1),
        Emu::from_inches(0.3),
        std::slice::from_ref(&both),
    );
    assert_eq!(
        runs.len(),
        2,
        "bothSides leaves the narrow run too: {runs:?}"
    );

    // And the mirror image, so the rule is not "always the right".
    let right_of_centre = Exclusion::rectangle(
        Emu::from_inches(4.5),
        Emu::ZERO,
        Emu::from_inches(5.5),
        Emu::from_inches(1.0),
        WrapSide::Largest,
    );
    let runs = free_runs(
        Emu::ZERO,
        Emu::from_inches(6.0),
        Emu::from_inches(0.1),
        Emu::from_inches(0.3),
        std::slice::from_ref(&right_of_centre),
    );
    assert_eq!(
        runs,
        vec![(Emu::ZERO, Emu::from_inches(4.5))],
        "an object right of centre leaves the run to its left"
    );
}

#[test]
fn a_top_and_bottom_wrap_clears_the_whole_measure() {
    let band = Exclusion::band(Emu::from_inches(1.0), Emu::from_inches(2.0));
    let inside = free_runs(
        Emu::ZERO,
        Emu::from_inches(6.0),
        Emu::from_inches(1.4),
        Emu::from_inches(1.6),
        std::slice::from_ref(&band),
    );
    assert!(inside.is_empty(), "no text fits beside it: {inside:?}");

    let below = free_runs(
        Emu::ZERO,
        Emu::from_inches(6.0),
        Emu::from_inches(2.1),
        Emu::from_inches(2.3),
        std::slice::from_ref(&band),
    );
    assert_eq!(
        below,
        vec![(Emu::ZERO, Emu::from_inches(6.0))],
        "and below it the whole measure is back"
    );

    // Through the flow engine: a `topAndBottom` object pushes the text past it rather than composing
    // it into nothing, and the paragraph is therefore taller. The band has to sit where the
    // paragraph's own lines actually reach, which for four lines of eleven-point text is well inside
    // the first inch — a band at two inches would be below every line and would assert nothing.
    let band = Exclusion::band(Emu::from_inches(0.2), Emu::from_inches(0.9));
    let around = lay_out_one_around("", BODY, 6.0, std::slice::from_ref(&band));
    let plain = lay_out_one_around("", BODY, 6.0, &[]);
    assert_eq!(
        around.lines.len(),
        plain.lines.len(),
        "the same text still breaks into the same lines — the band moves them, it does not requote them"
    );
    assert!(
        around.height_of(0..around.lines.len()) > plain.height_of(0..plain.lines.len()),
        "and the paragraph is taller by the band it had to clear"
    );
}

#[test]
fn a_float_inside_a_table_cell_wraps_against_the_cell() {
    // The interaction the ticket names: a float anchored in a cell's paragraph is positioned in the
    // **cell's** measure, so the lines it displaces are the cell's and not the page's.
    // An inch and a half square in a cell a little under three inches wide: enough of the cell's own
    // measure that the text it displaces has to take another line.
    let floated = floating_paragraph(
        "",
        BODY,
        1_371_600,
        1_371_600,
        &offset_position(0, 0),
        &square_wrap("bothSides"),
    );
    let with_float = table(
        "",
        &[4000, 4000],
        &[row("", &[cell("", &floated), text_cell("", "right")])],
    );
    let without = table(
        "",
        &[4000, 4000],
        &[row(
            "",
            &[cell("", &paragraph("", BODY)), text_cell("", "right")],
        )],
    );

    let height = |markup: String| {
        let mut document = document(&[markup]);
        let flow = flow(&mut document);
        let mut engine = model();
        let laid = engine
            .lay_out_block(
                &flow,
                0,
                mjx_layout_docx::LayoutRequest {
                    width: Emu::from_inches(3.5),
                    top: Emu::ZERO,
                    exclusions: &[],
                },
            )
            .expect("it lays out");
        let table = laid.as_table().expect("a table").clone();
        (table.rows[0].height, table.rows[0].cells[0].content_width)
    };

    let (floated_height, floated_width) = height(with_float);
    let (plain_height, plain_width) = height(without);
    assert_eq!(
        floated_width, plain_width,
        "the cell is the same width either way — the float is inside it"
    );
    assert!(
        floated_height > plain_height,
        "and the float makes the cell's own text taller: {floated_height:?} against {plain_height:?}"
    );
}

#[test]
fn a_float_moves_the_page_a_paragraph_lands_on() {
    // The end-to-end assertion: wrapping is not a cosmetic property of one paragraph, it changes
    // where the page breaks and therefore which page every later paragraph is on.
    // Two inches of a six-inch column, three inches tall: wide enough that the text beside it cannot
    // break into the same number of lines, which is the whole point — a narrower object gives
    // different *measures* and can still give the same line count, and a line count is what a page
    // is made of.
    let mut body: Vec<String> = vec![floating_paragraph(
        "",
        BODY,
        1_828_800,
        2_743_200,
        &offset_position(0, 0),
        &square_wrap("bothSides"),
    )];
    body.extend((0..12).map(|index| paragraph("", &format!("later paragraph {index} {BODY}"))));

    let mut plain: Vec<String> = vec![floating_paragraph(
        "",
        BODY,
        1_828_800,
        2_743_200,
        &offset_position(0, 0),
        r#"<wp:wrapNone/>"#,
    )];
    plain.extend((0..12).map(|index| paragraph("", &format!("later paragraph {index} {BODY}"))));

    let constraints = constraints(6.0, 4.0);
    let mut wrapped_document = document(&body);
    let wrapped_flow = flow(&mut wrapped_document);
    let mut engine = model();
    let wrapped_pages = walk(&mut engine, &wrapped_flow, &constraints, 24);
    let wrapped_lines: Vec<usize> = wrapped_pages
        .iter()
        .map(|page| support::line_count(page.fragments()))
        .collect();
    let wrapped = page_of_each_paragraph(&mut engine, &wrapped_flow, &constraints, 24);

    let mut plain_document = document(&plain);
    let plain_flow = flow(&mut plain_document);
    let mut engine = model();
    let plain_pages = walk(&mut engine, &plain_flow, &constraints, 24);
    let plain_lines: Vec<usize> = plain_pages
        .iter()
        .map(|page| support::line_count(page.fragments()))
        .collect();
    let unwrapped = page_of_each_paragraph(&mut engine, &plain_flow, &constraints, 24);

    // **How many lines each page holds**, rather than which paragraph is on which. The paragraph
    // assignment is the coarser instrument and it can coincide by luck — the float pushes lines
    // about *inside* the paragraphs it meets — while a page's line count cannot.
    assert_ne!(
        wrapped_lines, plain_lines,
        "wrapping must change how much text fits on a page"
    );
    assert!(
        wrapped.len() == unwrapped.len(),
        "the two documents hold the same blocks"
    );
}

#[test]
fn the_line_band_search_terminates_on_a_pile_of_floats() {
    // Twenty stacked `topAndBottom` bands covering the whole column, so every line has to walk past
    // every one of them. The bound is the exclusion count, and the assertion is that this returns at
    // all — a search that moved a line down without strictly advancing would not.
    let bands: Vec<Exclusion> = (0..20)
        .map(|index| {
            Exclusion::band(
                Emu::from_inches(f64::from(index) * 0.5),
                Emu::from_inches(f64::from(index) * 0.5 + 0.4),
            )
        })
        .collect();
    let laid = lay_out_one_around("", BODY, 6.0, &bands);
    assert!(
        !laid.lines.is_empty(),
        "it produces lines rather than hanging"
    );
    assert!(
        laid.height_of(0..laid.lines.len()) > Emu::from_inches(9.0),
        "and every line was pushed past every band it met"
    );
    assert_eq!(BAND_COMPOSITIONS, 2, "the documented bound is two");
    assert_eq!(WRAP_POLYGON_UNITS, 21_600);
}

#[test]
fn a_float_in_a_document_that_paginates_still_walks() {
    // The float must survive the whole page loop, including the note fixed point and the resumption
    // path — a wrapping engine that only worked on page one would pass every assertion above.
    let mut body: Vec<String> = Vec::new();
    for index in 0..20 {
        body.push(floating_paragraph(
            "",
            &format!("paragraph {index} {BODY}"),
            457_200,
            457_200,
            &offset_position(0, 0),
            &square_wrap("largest"),
        ));
    }
    let mut document = document(&body);
    let flow = flow(&mut document);
    let mut engine = model();
    let constraints = constraints(6.0, 4.0);
    let pages = walk(&mut engine, &flow, &constraints, 40);
    assert!(pages.len() > 1, "it paginates");
    let seen: usize = pages
        .iter()
        .map(|page| support::paragraphs_on(page.fragments()).len())
        .sum();
    assert!(seen >= 20, "every paragraph is placed somewhere: {seen}");
}

#[test]
fn a_float_at_the_very_top_of_a_column_still_displaces_its_own_paragraph() {
    // **The gate on the cache key's third field, written behaviourally so that it survives someone
    // renaming or restructuring the sentinel.**
    //
    // The block that anchors a float is laid out twice: once to settle its own top, and once against
    // the exclusion that top produced. The two are told apart only by that third field — and the
    // block at the very start of a column has `top == 0`, so a sentinel of `0` makes the two keys
    // **identical**. The second layout then hits the first one's float-free entry, and the float
    // changes nothing at all.
    //
    // Every other assertion in this suite is blind to that. The polygon and largest-side tests build
    // their exclusions directly and never touch the cache; the paginated ones put the float on
    // paragraph zero and read *later* paragraphs, which sit at tops greater than zero and so key
    // distinctly whatever the sentinel is. This is the one fixture that reads the anchoring
    // paragraph's own first line, which is the only place the collision is visible.
    //
    // It is a real regression rather than a hypothetical: `NO_FLOATS` was `0` for the length of one
    // test run during MJXOFF-176, and the whole suite was green.
    let float_width = Emu::from_inches(1.0);
    let page = constraints(6.0, 11.0);

    let first_glyph = |wrap: &str| -> i64 {
        let body = vec![floating_paragraph(
            "",
            BODY,
            float_width.emu(),
            914_400,
            &offset_position(0, 0),
            wrap,
        )];
        let mut document = document(&body);
        let flow = flow(&mut document);
        let mut engine = model();
        let laid = engine
            .layout_page(&flow, mjx_layout::PageIndex::FIRST, &page, None)
            .expect("page one lays out");
        let positions = support::glyph_positions(laid.fragments(), 0, 0);
        *positions.first().expect("the first line draws some glyphs")
    };

    let wrapped = first_glyph(&square_wrap("bothSides"));
    let unwrapped = first_glyph(r#"<wp:wrapNone/>"#);

    assert!(
        unwrapped < float_width.emu(),
        "the control starts at the margin, because wp:wrapNone displaces nothing: {unwrapped}"
    );
    assert!(
        wrapped >= float_width.emu(),
        "the anchoring paragraph's own first line starts past the object it anchors — a cache key \
         that cannot tell the float-bearing layout from the float-free one would put it at \
         {unwrapped} instead: {wrapped}"
    );
}

#[test]
fn the_float_free_cache_key_is_not_a_key_a_column_top_can_take() {
    // The direct form of the assertion above. Cheaper, and it fails on the *value* rather than on a
    // rendered consequence — worth having as well as the behavioural one, because it names what is
    // wrong instead of only showing it.
    let width = Emu::from_inches(6.0);

    // A column top of zero is not a hypothetical: it is where the first block of every column sits.
    assert_ne!(
        mjx_layout_docx::cache_key(0, width, Emu::ZERO, true),
        mjx_layout_docx::cache_key(0, width, Emu::ZERO, false),
        "a block at the top of a column must key differently with and without a float beside it"
    );
    // And no *reachable* top collides either. Stated over real column positions rather than as
    // `NO_FLOATS < 0`, which clippy rightly calls a constant assertion — and which is also the weaker
    // claim: what matters is not the sentinel's sign but that nothing a column can actually produce
    // ever keys the same way.
    for inches in [0.0, 0.001, 1.0, 10.0, 100.0] {
        let top = Emu::from_inches(inches);
        assert_ne!(
            mjx_layout_docx::cache_key(3, width, top, true),
            mjx_layout_docx::cache_key(3, width, Emu::ZERO, false),
            "a block at {inches} inches down a column must not key as though nothing floated \
             beside it"
        );
    }
    assert_eq!(
        mjx_layout_docx::cache_key(3, width, Emu::from_inches(2.0), false).2,
        mjx_layout_docx::NO_FLOATS,
        "and the float-free key really is the sentinel, so a rename or a restructure of it still \
         has to come through this assertion"
    );
}
