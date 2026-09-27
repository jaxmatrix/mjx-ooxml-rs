//! Line spacing, tab stops and indents — the three parts of a paragraph's geometry that are *not*
//! line breaking, each asserted at a value where getting it wrong is visible.
//!
//! # `w:line` is not a length when `w:lineRule` says `auto`
//!
//! It is in **240ths of a line**, and that is the one place in WordprocessingML where a
//! `ST_SignedTwipsMeasure` is not a measure. Reading `w:line="240"` as 240 twips gives a
//! twelve-point line, which looks almost right at eleven points and is not — and every page boundary
//! in the document moves. That is the first test below.

mod support;

use mjx_layout_docx::{ParagraphStyle, TabRuler};
use mjx_ooxml_core::measure::Emu;
use support::{constraints, glyph_positions, lines_of, one_page, paragraph, paragraph_with_run};

/// The height of a line of paragraph 0 on `tree`.
fn line_height(tree: &mjx_layout::FragmentTree) -> i64 {
    tree.nodes()
        .find(|(_, node)| matches!(node.fragment(), mjx_layout::Fragment::Line(_)))
        .map(|(_, node)| node.rect().height().emu())
        .expect("a line")
}

// -------------------------------------------------------------------------------------------
// Line spacing
// -------------------------------------------------------------------------------------------

#[test]
fn the_three_line_rules_give_three_different_heights() {
    let constraints = constraints(6.5, 11.0);
    let single = line_height(&one_page(&[paragraph("", "One line.")], &constraints));

    let double = line_height(&one_page(
        &[paragraph(
            r#"<w:spacing w:line="480" w:lineRule="auto"/>"#,
            "One line.",
        )],
        &constraints,
    ));
    assert!(
        (double - single * 2).abs() <= 2,
        "`auto` with `w:line=480` is double spacing: {single} then {double}"
    );

    // `exact` is a length, and one small enough to clip the text is still honoured — that is what
    // makes it `exact`.
    let exact = line_height(&one_page(
        &[paragraph(
            r#"<w:spacing w:line="200" w:lineRule="exact"/>"#,
            "One line.",
        )],
        &constraints,
    ));
    assert_eq!(
        exact,
        Emu::from_twips(200).emu(),
        "`exact` is the height, whatever is on the line"
    );

    // `atLeast` below the natural height changes nothing; above it, it wins.
    let at_least_small = line_height(&one_page(
        &[paragraph(
            r#"<w:spacing w:line="100" w:lineRule="atLeast"/>"#,
            "One line.",
        )],
        &constraints,
    ));
    assert_eq!(
        at_least_small, single,
        "`atLeast` below the natural height is the identity"
    );
    let at_least_large = line_height(&one_page(
        &[paragraph(
            r#"<w:spacing w:line="600" w:lineRule="atLeast"/>"#,
            "One line.",
        )],
        &constraints,
    ));
    assert_eq!(at_least_large, Emu::from_twips(600).emu());
}

/// **The trap named in this file's own documentation.** A `w:line` of 240 under `auto` is single
/// spacing — a *ratio* — and not 240 twips.
#[test]
fn auto_line_spacing_is_a_ratio_and_not_a_length() {
    let constraints = constraints(6.5, 11.0);
    let plain = line_height(&one_page(&[paragraph("", "One line.")], &constraints));
    let stated = line_height(&one_page(
        &[paragraph(
            r#"<w:spacing w:line="240" w:lineRule="auto"/>"#,
            "One line.",
        )],
        &constraints,
    ));
    assert_eq!(stated, plain, "240/240 is one line");
    assert_ne!(
        stated,
        Emu::from_twips(240).emu(),
        "and it is emphatically not 240 twips"
    );
}

/// The tallest run on a line decides its natural height, which is what makes a line with one large
/// word in it taller than the rest of its paragraph.
#[test]
fn the_tallest_run_on_a_line_sets_its_height() {
    let constraints = constraints(6.5, 11.0);
    let small = line_height(&one_page(
        &[support::paragraph_at("", "Small.", 8.0)],
        &constraints,
    ));
    let large = line_height(&one_page(
        &[support::paragraph_at("", "Large.", 24.0)],
        &constraints,
    ));
    assert!(large > small * 2, "{small} then {large}");
}

/// `w:spacing/@before` and `@after` push the paragraphs apart, which moves the second one's first
/// line down the page.
#[test]
fn space_before_moves_the_paragraph_that_follows_it() {
    let constraints = constraints(6.5, 11.0);
    let plain = one_page(
        &[paragraph("", "First."), paragraph("", "Second.")],
        &constraints,
    );
    let spaced = one_page(
        &[
            paragraph("", "First."),
            paragraph(r#"<w:spacing w:before="240"/>"#, "Second."),
        ],
        &constraints,
    );
    let top_of = |tree: &mjx_layout::FragmentTree| {
        tree.nodes()
            .filter(|(_, node)| matches!(node.fragment(), mjx_layout::Fragment::Line(_)))
            .filter(|(_, node)| node.source().path().segments().first() == Some(&1))
            .map(|(_, node)| node.rect().top.emu())
            .next()
            .expect("the second paragraph's line")
    };
    assert_eq!(
        top_of(&spaced) - top_of(&plain),
        Emu::from_twips(240).emu(),
        "twelve points of space before moves the paragraph twelve points down"
    );
}

// -------------------------------------------------------------------------------------------
// Indents
// -------------------------------------------------------------------------------------------

/// A hanging indent makes the **first** line start further left than the rest, which is the shape
/// every bulleted list has — and a measure that did not vary with the line number could not express
/// it.
#[test]
fn a_hanging_indent_starts_the_first_line_left_of_the_others() {
    let style = ParagraphStyle::of(&Default::default());
    assert_eq!(style.leading_indent_of(0), Emu::ZERO);

    let constraints = constraints(6.5, 11.0);
    let prose = "Alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima mike \
                 november oscar papa quebec romeo sierra tango uniform victor whiskey xray";
    let tree = one_page(
        &[paragraph(
            r#"<w:ind w:left="1440" w:hanging="720"/>"#,
            prose,
        )],
        &constraints,
    );
    assert!(lines_of(&tree, 0) >= 2, "the fixture must wrap");
    let first = glyph_positions(&tree, 0, 0)[0];
    let second = glyph_positions(&tree, 0, 1)[0];
    assert_eq!(first, Emu::from_twips(720).emu(), "1440 out, 720 back");
    assert_eq!(
        second,
        Emu::from_twips(1440).emu(),
        "the rest at the indent"
    );
    assert!(first < second, "hanging means the first line hangs left");
}

/// `w:firstLine` is the other way round.
#[test]
fn a_first_line_indent_starts_the_first_line_right_of_the_others() {
    let constraints = constraints(6.5, 11.0);
    let prose = "Alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima mike \
                 november oscar papa quebec romeo sierra tango uniform victor whiskey xray";
    let tree = one_page(
        &[paragraph(r#"<w:ind w:firstLine="720"/>"#, prose)],
        &constraints,
    );
    assert_eq!(glyph_positions(&tree, 0, 0)[0], Emu::from_twips(720).emu());
    assert_eq!(glyph_positions(&tree, 0, 1)[0], 0);
}

// -------------------------------------------------------------------------------------------
// Tab stops
// -------------------------------------------------------------------------------------------

/// The five kinds that place text, at one fixture each, asserted on where the text after the tab
/// lands relative to the stop.
#[test]
fn the_five_tab_kinds_place_text_five_different_ways() {
    let constraints = constraints(6.5, 11.0);
    let stop = 2880_i64; // two inches
    let at = Emu::from_twips(stop).emu();

    let run_for = |kind: &str| -> Vec<i64> {
        let tabs = format!(r#"<w:tabs><w:tab w:val="{kind}" w:pos="{stop}"/></w:tabs>"#);
        let tree = one_page(
            &[paragraph_with_run(
                &tabs,
                r#"<w:t>a</w:t><w:tab/><w:t>1234.56</w:t>"#,
            )],
            &constraints,
        );
        glyph_positions(&tree, 0, 0)
    };

    let leading = run_for("left");
    assert_eq!(leading[1], at, "a leading tab starts the text at the stop");

    let trailing = run_for("right");
    assert!(
        trailing[1] < at,
        "a trailing tab ends the text at the stop, so it starts before it: {trailing:?}"
    );

    let centred = run_for("center");
    assert!(
        centred[1] < at && centred[1] > trailing[1],
        "a centred tab starts half a chunk before the stop: {centred:?} against {trailing:?}"
    );

    let decimal = run_for("decimal");
    assert!(
        decimal[1] < at && decimal[1] > trailing[1],
        "a decimal tab puts the separator on the stop, so it starts between the two: {decimal:?}"
    );

    // A `bar` stop is a **rule**, not a tab: it does not advance a pen, so the tab in front of it
    // falls through to the implicit grid. **GUESS:** the grid still starts *past* the bar stop,
    // because §17.3.1.37 says the default interval applies after the last custom stop and a bar
    // stop is one — so the answer is the first multiple of 720 twips past 2880, which is 3600, and
    // not 720. The other reading (a bar stop is invisible to the grid too) would give 720, and the
    // two are a full two inches apart on the page.
    let bar = run_for("bar");
    assert_eq!(
        bar[1],
        Emu::from_twips(3600).emu(),
        "a bar stop is a rule and not a tab, so the grid past it answers: {bar:?}"
    );
}

/// `w:val="clear"` **removes** a stop rather than adding one — the one rule the effective-property
/// ladder cannot express, because it is a subtraction.
#[test]
fn a_cleared_stop_is_removed_rather_than_placed() {
    let stated = [
        mjx_docx::EffectiveTabStop {
            alignment: mjx_ooxml_types::wordprocessingml::TabStopType::Left,
            leader: None,
            position: mjx_ooxml_types::wordprocessingml::SignedTwipsMeasure::from_wire("1440"),
        },
        mjx_docx::EffectiveTabStop {
            alignment: mjx_ooxml_types::wordprocessingml::TabStopType::Clear,
            leader: None,
            position: mjx_ooxml_types::wordprocessingml::SignedTwipsMeasure::from_wire("1440"),
        },
    ];
    let ruler = TabRuler::new(&stated, Emu::from_twips(720));
    assert!(
        ruler.stated().is_empty(),
        "the clear removed the stop it names: {:?}",
        ruler.stated()
    );
    assert_eq!(
        ruler.next_after(Emu::ZERO).position,
        Emu::from_twips(720),
        "so the default grid answers instead"
    );
}

/// The implicit grid past the last stated stop, and the guard that keeps `w:defaultTabStop="0"`
/// from being an infinite loop.
#[test]
fn the_default_grid_takes_over_past_the_last_stop_and_never_stands_still() {
    let ruler = TabRuler::new(&[], Emu::from_twips(720));
    assert_eq!(ruler.next_after(Emu::ZERO).position, Emu::from_twips(720));
    assert_eq!(
        ruler.next_after(Emu::from_twips(720)).position,
        Emu::from_twips(1440),
        "a tab exactly on a stop still moves to the next one"
    );

    let degenerate = TabRuler::new(&[], Emu::ZERO);
    assert_eq!(
        degenerate.interval(),
        TabRuler::FALLBACK_INTERVAL,
        "a stated interval of zero falls back rather than standing still"
    );
}

/// A leader fills the gap a tab opened, as **one** glyph run rather than one per character.
#[test]
fn a_leader_fills_the_gap_with_one_run() {
    let constraints = constraints(6.5, 11.0);
    let tabs = r#"<w:tabs><w:tab w:val="left" w:pos="4320" w:leader="dot"/></w:tabs>"#;
    let with_leader = one_page(
        &[paragraph_with_run(
            tabs,
            r#"<w:t>Chapter one</w:t><w:tab/><w:t>7</w:t>"#,
        )],
        &constraints,
    );
    let without = one_page(
        &[paragraph_with_run(
            r#"<w:tabs><w:tab w:val="left" w:pos="4320"/></w:tabs>"#,
            r#"<w:t>Chapter one</w:t><w:tab/><w:t>7</w:t>"#,
        )],
        &constraints,
    );
    assert_eq!(
        glyph_positions(&with_leader, 0, 0).len(),
        glyph_positions(&without, 0, 0).len() + 1,
        "the leader is exactly one more glyph run"
    );
}

/// Every leader style names a character, and `none` names none.
#[test]
fn every_leader_style_has_a_character_and_none_has_none() {
    use mjx_layout_docx::leader_character;
    use mjx_ooxml_types::wordprocessingml::TabStopLeader;
    assert_eq!(leader_character(TabStopLeader::None), None);
    for leader in [
        TabStopLeader::Dot,
        TabStopLeader::Hyphen,
        TabStopLeader::Underscore,
        TabStopLeader::Heavy,
        TabStopLeader::MiddleDot,
    ] {
        assert!(
            leader_character(leader).is_some(),
            "{leader:?} must draw something"
        );
    }
}

/// `w:contextualSpacing` — *don't add space between paragraphs of the same style* — which is what
/// every bulleted list in every document relies on, and which is a question about the two
/// paragraphs' **identity** rather than about their resolved values.
#[test]
fn contextual_spacing_suppresses_the_gap_between_two_paragraphs_of_one_style() {
    let constraints = constraints(6.5, 11.0);
    let spacing = r#"<w:spacing w:before="240" w:after="240"/>"#;
    let listed = |style: &str, contextual: bool| -> String {
        let flag = if contextual {
            "<w:contextualSpacing/>"
        } else {
            ""
        };
        paragraph(
            &format!(r#"<w:pStyle w:val="{style}"/>{spacing}{flag}"#),
            "An item.",
        )
    };
    let top_of_second = |tree: &mjx_layout::FragmentTree| {
        tree.nodes()
            .filter(|(_, node)| matches!(node.fragment(), mjx_layout::Fragment::Line(_)))
            .filter(|(_, node)| node.source().path().segments().first() == Some(&1))
            .map(|(_, node)| node.rect().top.emu())
            .next()
            .expect("the second paragraph's line")
    };

    let apart = one_page(
        &[listed("ListItem", false), listed("ListItem", false)],
        &constraints,
    );
    let together = one_page(
        &[listed("ListItem", true), listed("ListItem", true)],
        &constraints,
    );
    assert!(
        top_of_second(&together) < top_of_second(&apart),
        "contextual spacing must close the gap: {} against {}",
        top_of_second(&together),
        top_of_second(&apart)
    );

    // …and it must **not** close it between two paragraphs of *different* styles, which is the half
    // that says the rule is about identity rather than about the flag alone.
    let mixed = one_page(
        &[listed("ListItem", true), listed("BodyText", true)],
        &constraints,
    );
    assert_eq!(
        top_of_second(&mixed),
        top_of_second(&apart),
        "two different styles keep their space even with the flag on both"
    );
}
