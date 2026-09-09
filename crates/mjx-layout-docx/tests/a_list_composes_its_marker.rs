//! **List numbering, asserted on the composed `w:lvlText` and not on "numbers appear"**.
//!
//! # The identity-value trap, and how every assertion here dodges it
//!
//! A single-level decimal list starting at one produces `1`, `2`, `3` for an implementation that
//! composes `w:lvlText` properly **and** for one that prints a running counter and never looks at the
//! template. Every plausible bug is the identity on that fixture, so nothing here uses it alone.
//!
//! What is asserted instead is the set of things a naive implementation gets wrong:
//!
//! * a **three-level** template, `%1.%2.%3`, where the placeholders are one-based and `w:ilvl` is
//!   zero-based and each level has its **own** number format;
//! * a **`w:startOverride`**, which outranks the abstract definition's `w:start`;
//! * a **restart**, including `w:lvlRestart="0"` — which means *never* and not *level zero*;
//! * **`w:isLgl`**, which writes every placeholder in Arabic whatever each level's own format says;
//! * a definition reached **through a paragraph style** rather than through the paragraph's own
//!   `w:numPr`, which is how every document built from Word's Heading styles is numbered.
//!
//! And one thing about layout rather than about text: the marker's `w:suff="tab"` is a **real tab**,
//! so the text after the number begins at a tab stop — which is what a hanging indent is, and which
//! `the_text_after_a_marker_starts_at_a_tab_stop` reads off the fragments.

mod support;

use mjx_layout_docx::{ListNumbering, Marker};
use support::generated::{
    abstract_numbering, document_with_numbering, document_with_numbering_and_styles,
    listed_paragraph, numbering_instance, numbering_level, numbering_part, start_override,
    styled_paragraph, styles_with_numbered_style,
};
use support::{constraints, flow, glyph_positions, model, one_page};

/// The composed marker text of every numbered paragraph, in document order.
fn markers(paragraphs: &[String], numbering: Vec<u8>) -> Vec<String> {
    let mut document = document_with_numbering(paragraphs, numbering);
    let read = flow(&mut document);
    (0..read.paragraphs().len())
        .filter_map(|index| read.lists().marker(index))
        .map(|marker| marker.text.clone())
        .collect()
}

/// A three-level outline: `%1` upper Roman, `%1.%2` upper letter, `%1.%2.%3` decimal.
fn three_level_definition() -> Vec<u8> {
    numbering_part(
        &[abstract_numbering(
            0,
            &[
                numbering_level(0, 1, "upperRoman", "%1.", ""),
                numbering_level(1, 1, "upperLetter", "%1.%2.", ""),
                numbering_level(2, 1, "decimal", "%1.%2.%3", ""),
            ],
        )],
        &[numbering_instance(1, 0, "")],
    )
}

#[test]
fn a_three_level_outline_composes_every_placeholder_in_its_own_format() {
    // The assertion the identity cannot produce: `I.A.1` needs level zero's counter written in upper
    // Roman, level one's in upper letters and level two's in decimal — three systems in one marker.
    // An implementation that formatted every placeholder in the *current* level's format renders
    // `1.1.1` at level two and `I.I.` at level one.
    let paragraphs = vec![
        listed_paragraph(1, 0, "first chapter"),
        listed_paragraph(1, 1, "first section"),
        listed_paragraph(1, 2, "first point"),
        listed_paragraph(1, 2, "second point"),
        listed_paragraph(1, 1, "second section"),
        listed_paragraph(1, 2, "a point of the second section"),
        listed_paragraph(1, 0, "second chapter"),
        listed_paragraph(1, 1, "a section of the second chapter"),
    ];
    assert_eq!(
        markers(&paragraphs, three_level_definition()),
        vec![
            "I.".to_owned(),
            "I.A.".to_owned(),
            "I.A.1".to_owned(),
            "I.A.2".to_owned(),
            "I.B.".to_owned(),
            "I.B.1".to_owned(),
            "II.".to_owned(),
            "II.A.".to_owned(),
        ]
    );
}

#[test]
fn a_deeper_level_restarts_when_a_higher_one_advances() {
    // Read from the outline above: the third `%3` counter goes 1, 2 and then restarts at 1 under the
    // second section, and the `%2` counter restarts under the second chapter. Asserting it here
    // separately makes the *restart* the subject rather than a consequence.
    let composed = markers(
        &[
            listed_paragraph(1, 0, "one"),
            listed_paragraph(1, 1, "one one"),
            listed_paragraph(1, 1, "one two"),
            listed_paragraph(1, 0, "two"),
            listed_paragraph(1, 1, "two one"),
        ],
        three_level_definition(),
    );
    assert_eq!(composed[1], "I.A.");
    assert_eq!(composed[2], "I.B.");
    assert_eq!(
        composed[4], "II.A.",
        "the second level restarted when the first advanced"
    );
}

#[test]
fn lvl_restart_zero_means_never_and_not_level_zero() {
    // **The reading a renderer gets wrong.** Taken as a level index, `0` would mean *restart when
    // level zero advances*, which is almost the default and therefore looks right on a two-level
    // list — and turns a continuous numbering into an endless run of ones.
    let definition = numbering_part(
        &[abstract_numbering(
            0,
            &[
                numbering_level(0, 1, "decimal", "%1.", ""),
                numbering_level(1, 1, "decimal", "%2)", r#"<w:lvlRestart w:val="0"/>"#),
            ],
        )],
        &[numbering_instance(1, 0, "")],
    );
    let composed = markers(
        &[
            listed_paragraph(1, 0, "one"),
            listed_paragraph(1, 1, "first"),
            listed_paragraph(1, 0, "two"),
            listed_paragraph(1, 1, "second"),
        ],
        definition,
    );
    assert_eq!(
        composed,
        vec![
            "1.".to_owned(),
            "1)".to_owned(),
            "2.".to_owned(),
            "2)".to_owned()
        ],
        "the second level never restarts, so it counts 1 then 2 across the two chapters"
    );
}

#[test]
fn lvl_restart_names_the_level_that_resets_this_one_and_the_direction_matters() {
    // **The rule whose direction is easy to invert, and the fixture that catches it.** A three-level
    // outline whose deepest level states `w:lvlRestart="1"` renumbers at each *chapter* and **not**
    // at each *section* — which is what an author asks for when they want continuous numbering
    // within a chapter.
    //
    // The inverted reading makes every stated `w:lvlRestart` behave like the default, because the
    // default *is* the case where the threshold is this level's own number. So a two-level list is
    // identical under both readings and only a three-level one can tell them apart — the identity
    // trap in its purest form.
    let definition = numbering_part(
        &[abstract_numbering(
            0,
            &[
                numbering_level(0, 1, "decimal", "%1.", ""),
                numbering_level(1, 1, "decimal", "%1.%2.", ""),
                numbering_level(2, 1, "decimal", "%3)", r#"<w:lvlRestart w:val="1"/>"#),
            ],
        )],
        &[numbering_instance(1, 0, "")],
    );
    let composed = markers(
        &[
            listed_paragraph(1, 0, "chapter one"),
            listed_paragraph(1, 1, "section one"),
            listed_paragraph(1, 2, "a point"),
            listed_paragraph(1, 1, "section two"),
            listed_paragraph(1, 2, "another point"),
            listed_paragraph(1, 0, "chapter two"),
            listed_paragraph(1, 1, "its first section"),
            listed_paragraph(1, 2, "its first point"),
        ],
        definition,
    );
    assert_eq!(
        composed[2], "1)",
        "the first point of the first section is one"
    );
    assert_eq!(
        composed[4], "2)",
        "and the point in the SECOND section carries on — the section did not reset it"
    );
    assert_eq!(
        composed[7], "1)",
        "while the new chapter did, because chapter is the level `w:lvlRestart` names"
    );
}

#[test]
fn a_start_override_outranks_the_definitions_own_start() {
    let definition = numbering_part(
        &[abstract_numbering(
            0,
            &[numbering_level(0, 1, "decimal", "%1.", "")],
        )],
        &[numbering_instance(1, 0, &start_override(0, 7))],
    );
    let composed = markers(
        &[
            listed_paragraph(1, 0, "seven"),
            listed_paragraph(1, 0, "eight"),
        ],
        definition,
    );
    assert_eq!(composed, vec!["7.".to_owned(), "8.".to_owned()]);
}

#[test]
fn is_legal_numbering_writes_every_placeholder_in_arabic() {
    // §17.9.11: `w:isLgl` on the level being *rendered* forces Arabic for every placeholder in its
    // template, whatever each referenced level's own `w:numFmt` says. `III.B.2` becomes `3.2.2`, and
    // it is the level *above* whose format changes — which is what makes this a composition question
    // rather than a formatting one.
    let definition = numbering_part(
        &[abstract_numbering(
            0,
            &[
                numbering_level(0, 1, "upperRoman", "%1.", ""),
                numbering_level(1, 1, "upperLetter", "%1.%2.", r#"<w:isLgl/>"#),
            ],
        )],
        &[numbering_instance(1, 0, "")],
    );
    let composed = markers(
        &[
            listed_paragraph(1, 0, "one"),
            listed_paragraph(1, 0, "two"),
            listed_paragraph(1, 0, "three"),
            listed_paragraph(1, 1, "a section"),
        ],
        definition,
    );
    assert_eq!(
        composed[2], "III.",
        "the level that does not ask for legal numbering keeps its Roman numerals"
    );
    assert_eq!(
        composed[3], "3.1.",
        "and the one that does writes both placeholders in Arabic"
    );
}

#[test]
fn a_numbering_definition_inherited_through_a_style_still_numbers() {
    // Every document built from Word's own Heading styles is numbered this way: the `w:numPr` is on
    // the *style*, not on the paragraph. A renderer reading only a paragraph's own `w:numPr` numbers
    // nothing at all in such a document — and `mjx-docx`'s ladder has already resolved the two into
    // one reference, which is the whole reason this crate must not re-derive it.
    let definition = numbering_part(
        &[abstract_numbering(
            0,
            &[numbering_level(0, 1, "decimal", "%1.", "")],
        )],
        &[numbering_instance(1, 0, "")],
    );
    let paragraphs = vec![
        styled_paragraph("Heading1", "an inherited item"),
        styled_paragraph("Heading1", "another inherited item"),
    ];
    let mut document = document_with_numbering_and_styles(
        &paragraphs,
        definition,
        Some(styles_with_numbered_style("Heading1", 1, 0)),
    );
    let read = flow(&mut document);
    let composed: Vec<String> = (0..read.paragraphs().len())
        .filter_map(|index| read.lists().marker(index))
        .map(|marker| marker.text.clone())
        .collect();
    assert_eq!(composed, vec!["1.".to_owned(), "2.".to_owned()]);
}

#[test]
fn numbering_id_zero_removes_an_inherited_list_rather_than_starting_one() {
    // §17.9.18 makes `w:numId="0"` the explicit *removal* of an inherited reference, which is how a
    // paragraph opts out of the list its style puts it in. Counting it would number a paragraph Word
    // draws unnumbered — and would advance the counter, so every item after it would be wrong too.
    let definition = numbering_part(
        &[abstract_numbering(
            0,
            &[numbering_level(0, 1, "decimal", "%1.", "")],
        )],
        &[numbering_instance(1, 0, "")],
    );
    let paragraphs = vec![
        listed_paragraph(1, 0, "one"),
        listed_paragraph(0, 0, "not a list item at all"),
        listed_paragraph(1, 0, "two"),
    ];
    let mut document = document_with_numbering(&paragraphs, definition);
    let read = flow(&mut document);
    assert!(
        read.lists().marker(1).is_none(),
        "the opt-out is unnumbered"
    );
    assert_eq!(
        read.lists().marker(2).map(|marker| marker.text.as_str()),
        Some("2.")
    );
}

#[test]
fn the_marker_carries_its_suffix_and_a_tab_is_a_real_tab() {
    // `w:suff` decides what separates the marker from the text, and a tab is a real `U+0009` so that
    // the indent interaction falls out of `crate::tabs` rather than needing a second implementation.
    let with_suffix = |suffix: &str| -> String {
        let definition = numbering_part(
            &[abstract_numbering(
                0,
                &[numbering_level(
                    0,
                    1,
                    "decimal",
                    "%1.",
                    &format!(r#"<w:suff w:val="{suffix}"/>"#),
                )],
            )],
            &[numbering_instance(1, 0, "")],
        );
        let mut document =
            document_with_numbering(&[listed_paragraph(1, 0, "an item")], definition);
        let read = flow(&mut document);
        read.lists()
            .marker(0)
            .map(Marker::with_suffix)
            .unwrap_or_default()
    };
    assert_eq!(with_suffix("tab"), "1.\t");
    assert_eq!(with_suffix("space"), "1. ");
    assert_eq!(with_suffix("nothing"), "1.");
}

#[test]
fn the_text_after_a_marker_starts_at_a_tab_stop() {
    // The layout half: a marker followed by a tab puts the paragraph's own text at the next tab stop
    // rather than immediately after the number, so items whose numbers are different widths still
    // line up. `10.` is wider than `9.`, and both texts must start at the same x.
    let definition = numbering_part(
        &[abstract_numbering(
            0,
            &[numbering_level(0, 9, "decimal", "%1.", "")],
        )],
        &[numbering_instance(1, 0, "")],
    );
    let paragraphs = vec![
        listed_paragraph(1, 0, "nine"),
        listed_paragraph(1, 0, "ten"),
    ];
    let mut document = document_with_numbering(&paragraphs, definition);
    let read = flow(&mut document);
    let mut model = model();
    let constraints = constraints(6.5, 9.0);
    let page = <mjx_layout_docx::DocumentBoxModel as mjx_layout::BoxModel>::layout_page(
        &mut model,
        &read,
        mjx_layout::PageIndex::FIRST,
        &constraints,
        None,
    )
    .expect("the page lays out");
    let tree = page.fragments();
    let ninth = glyph_positions(tree, 0, 0);
    let tenth = glyph_positions(tree, 1, 0);
    assert_eq!(
        ninth.len(),
        2,
        "the marker and the text are two runs: {ninth:?}"
    );
    assert_eq!(tenth.len(), 2);
    assert_eq!(
        ninth[1], tenth[1],
        "`9.` and `10.` are different widths and their texts still start at the same stop"
    );
    assert_ne!(
        ninth[0], ninth[1],
        "and the marker is not where the text is"
    );
    let _ = one_page;
}

#[test]
fn a_list_the_document_does_not_define_costs_the_paragraph_its_marker_and_nothing_else() {
    // A `w:numPr` naming a list `word/numbering.xml` does not hold is a defect in the document, not
    // a reason to refuse to lay it out.
    let definition = numbering_part(&[], &[]);
    let mut document =
        document_with_numbering(&[listed_paragraph(4, 0, "an orphan item")], definition);
    let read = flow(&mut document);
    assert!(read.lists().is_empty());
    assert_eq!(read.paragraphs()[0].text(), "an orphan item");
}

#[test]
fn every_level_of_a_definition_is_resolved_and_not_only_the_one_a_paragraph_names() {
    // Composing a third-level marker needs the *first* and *second* levels' formats, so a resolver
    // that fetched one level would render `I.A.1` as `1`. The residency is what resolves them, and
    // this asserts it hands all of them over.
    let mut document = document_with_numbering(
        &[listed_paragraph(1, 2, "a third-level item")],
        three_level_definition(),
    );
    let read = flow(&mut document);
    let definition = read
        .formatting()
        .numbering_definition(1)
        .expect("the definition is resolved");
    assert_eq!(definition.levels.len(), 3);
    assert!(definition.level(0).is_some() && definition.level(2).is_some());
}

/// A marker for a definition nothing reaches is never composed, which is what keeps a template's
/// dozens of unused lists from costing a `w:numStyleLink` walk each.
#[test]
fn only_the_lists_a_document_reaches_are_resolved() {
    let definition = numbering_part(
        &[
            abstract_numbering(0, &[numbering_level(0, 1, "decimal", "%1.", "")]),
            abstract_numbering(1, &[numbering_level(0, 1, "bullet", "\u{2022}", "")]),
        ],
        &[numbering_instance(1, 0, ""), numbering_instance(2, 1, "")],
    );
    let mut document = document_with_numbering(&[listed_paragraph(1, 0, "an item")], definition);
    let read = flow(&mut document);
    assert_eq!(read.formatting().numbering_definitions().len(), 1);
    assert!(read.formatting().numbering_definition(2).is_none());
}

#[test]
fn a_bullet_level_composes_its_literal_character() {
    let definition = numbering_part(
        &[abstract_numbering(
            0,
            &[numbering_level(0, 1, "bullet", "\u{2022}", "")],
        )],
        &[numbering_instance(1, 0, "")],
    );
    let composed = markers(
        &[listed_paragraph(1, 0, "one"), listed_paragraph(1, 0, "two")],
        definition,
    );
    assert_eq!(
        composed,
        vec!["\u{2022}".to_owned(), "\u{2022}".to_owned()],
        "a bullet's template holds no placeholder, so every item shows the same character"
    );
}

/// The counters are a whole-document walk, so a second list in the same document keeps its own.
#[test]
fn two_lists_in_one_document_count_separately() {
    let definition = numbering_part(
        &[abstract_numbering(
            0,
            &[numbering_level(0, 1, "decimal", "%1.", "")],
        )],
        &[
            numbering_instance(1, 0, ""),
            numbering_instance(2, 0, &start_override(0, 1)),
        ],
    );
    let composed = markers(
        &[
            listed_paragraph(1, 0, "first list, one"),
            listed_paragraph(2, 0, "second list, one"),
            listed_paragraph(1, 0, "first list, two"),
            listed_paragraph(2, 0, "second list, two"),
        ],
        definition,
    );
    assert_eq!(
        composed,
        vec![
            "1.".to_owned(),
            "1.".to_owned(),
            "2.".to_owned(),
            "2.".to_owned()
        ]
    );
}

/// [`ListNumbering`] is a document-order walk and is exposed, so a caller can ask which paragraphs
/// are numbered without laying anything out.
#[test]
fn the_numbering_is_readable_without_a_layout() {
    let mut document = document_with_numbering(
        &[
            listed_paragraph(1, 0, "one"),
            support::paragraph("", "not a list item"),
        ],
        three_level_definition(),
    );
    let read = flow(&mut document);
    let lists: &ListNumbering = read.lists();
    assert_eq!(lists.len(), 1);
    assert!(lists.marker(1).is_none());
}
