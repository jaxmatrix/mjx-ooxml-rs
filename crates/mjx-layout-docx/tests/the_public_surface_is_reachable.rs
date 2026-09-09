//! Every public item of this crate, named and used from outside it.
//!
//! # What this is for
//!
//! A `pub` item nothing outside the crate ever names is a claim with no consumer, and the whole
//! surface here is a contract two later children (MJXOFF-175 and a scene companion) will be written
//! against. This file is what makes the contract checkable: it compiles against the crate the way a
//! consumer would, so a re-export that quietly stopped existing fails here rather than in whatever
//! is written next.

mod support;

use mjx_layout::{
    BoxModel, ChangeKind, ChangeSet, ContentChange, DirtyPages, PageIndex, SourceRef,
};
use mjx_layout_docx::{
    address, border_width, constraints_for, cut, expansion_points, half_of, is_east_asian,
    leader_character, stroke_rect, widow_control, Alignment, Continuation, CutPolicy,
    DecorationCatalogue, DocumentBoxModel, DocumentFlow, DocumentLayoutError, FlowPosition,
    LineHeight, PageShape, ParagraphDecoration, ParagraphStyle, RunStyle, TabKind, TabRuler,
    ASSUMED_FONT_SIZE_POINTS, DECIMAL_SEPARATOR, HAIRLINE, HYPHEN, MAXIMUM_LEADER_GLYPHS,
    STATE_BYTES, VERSION,
};
use mjx_ooxml_core::measure::Emu;

#[test]
fn the_constants_are_reachable_and_say_what_they_say() {
    assert_eq!(
        VERSION, 2,
        "MJXOFF-175 changed the continuation's shape, so MJXOFF-174's thirteen bytes must be \
         refused rather than decoded as a page-zero document with no notes"
    );
    assert_eq!(STATE_BYTES, 45);
    assert_eq!(
        mjx_layout_docx::DEFAULT_LINE_NUMBER_DISTANCE,
        Emu::from_twips(360)
    );
    assert_eq!(mjx_layout_docx::LARGEST_ROMAN, 3_999);
    assert_eq!(
        HYPHEN, '\u{2010}',
        "the typographic hyphen, not the ASCII one"
    );
    assert_eq!(DECIMAL_SEPARATOR, '.');
    assert!(HAIRLINE > Emu::ZERO);
    const { assert!(MAXIMUM_LEADER_GLYPHS > 100) };
    assert!((ASSUMED_FONT_SIZE_POINTS - 10.0).abs() < f64::EPSILON);
}

#[test]
fn the_free_functions_are_reachable() {
    assert_eq!(half_of(Emu::from_emu(4)), Emu::from_emu(2));
    assert_eq!(border_width(None), HAIRLINE);
    assert!(is_east_asian('本'));
    assert!(!is_east_asian('a'));
    assert_eq!(
        widow_control(1, 3),
        0,
        "an orphan moves the paragraph whole"
    );
    assert_eq!(widow_control(3, 3), 3, "a paragraph that fits is untouched");
    assert!(leader_character(mjx_ooxml_types::wordprocessingml::TabStopLeader::Dot).is_some());

    let text = "a b";
    let pieces = cut(text, 0..text.len(), CutPolicy::Words);
    assert_eq!(pieces.len(), 2);
    assert_eq!(
        expansion_points(text, &pieces, Alignment::Justified).len(),
        1
    );
    assert_eq!(
        expansion_points(text, &pieces, Alignment::Start).len(),
        0,
        "a start-aligned line offers justification nothing"
    );
}

#[test]
fn the_style_projection_is_reachable() {
    let properties = mjx_docx::EffectiveParagraphProperties::default();
    let style = ParagraphStyle::of(&properties);
    assert_eq!(style.alignment, Alignment::Start);
    assert_eq!(style.line_height, LineHeight::SINGLE);
    assert!(style.widow_control, "the schema default is on");
    assert_eq!(
        style.measure_of(0, Emu::from_inches(6.0)),
        Emu::from_inches(6.0)
    );

    let run = RunStyle::of(0..0, &mjx_docx::EffectiveCharacterProperties::default());
    assert!(
        (run.size.in_points() - ASSUMED_FONT_SIZE_POINTS).abs() < f64::EPSILON,
        "a run that inherits nothing takes the assumed size"
    );
    assert!(!run.hidden);
}

#[test]
fn the_decoration_surface_is_reachable() {
    let mut catalogue = DecorationCatalogue::new();
    assert!(catalogue.is_empty());
    assert_eq!(catalogue.intern(ParagraphDecoration::default()), None);
    let rect = mjx_layout::LayoutRect::ZERO;
    assert_eq!(stroke_rect(rect, &ParagraphDecoration::default()), rect);
}

#[test]
fn the_tab_ruler_is_reachable() {
    let ruler = TabRuler::new(&[], Emu::from_twips(720));
    assert_eq!(ruler.interval(), Emu::from_twips(720));
    assert!(ruler.stated().is_empty());
    assert_eq!(ruler.bars().count(), 0);
    assert_eq!(ruler.next_after(Emu::ZERO).kind, TabKind::Leading);
}

#[test]
fn the_continuation_round_trips() {
    let continuation = Continuation {
        position: FlowPosition { block: 7, unit: 3 },
        paragraphs: 400,
        page_number: 12,
        line_number: 41,
        carry: Some(mjx_layout_docx::NoteCarry {
            note: 3,
            number: 9,
            line: 2,
        }),
    };
    let bytes = continuation.encode();
    assert_eq!(bytes.len(), STATE_BYTES);
    let read = Continuation::decode(&bytes, 400).expect("its own bytes");
    assert_eq!(read, continuation);
    assert!(matches!(
        Continuation::decode(&bytes, 401),
        Err(DocumentLayoutError::StaleContinuation { .. })
    ));
    assert!(matches!(
        Continuation::decode(&bytes[..4], 400),
        Err(DocumentLayoutError::MalformedContinuation { .. })
    ));
}

#[test]
fn the_addresses_are_reachable_and_order_as_document_order() {
    let first = address::paragraph(0);
    let second = address::paragraph(1);
    assert!(first < second, "a path orders as document order");
    assert!(address::root().path().contains(first.path()));
    assert_eq!(address::line(2, 1, 0..5).characters(), 0..5);
    assert_eq!(address::segment(2, 1, 0, 0..5).path().depth(), 3);
    assert_eq!(address::BODY, mjx_layout::PartId::PRIMARY);
}

#[test]
fn the_box_model_answers_all_four_questions() {
    let mut document = support::document(&[support::paragraph("", "One paragraph.")]);
    let flow = support::flow(&mut document);
    let mut model = support::model();

    assert_eq!(model.signature(), DocumentBoxModel::SIGNATURE);

    let constraints = support::constraints(6.5, 11.0);
    let page = model
        .layout_page(&flow, PageIndex::FIRST, &constraints, None)
        .expect("a page");
    assert!(page.is_last());
    assert!(!page.fragments().is_empty());
    assert!(!page.index().is_empty(), "the spatial index is built");

    let extent = model.estimate_extent(&flow, &constraints);
    assert!(extent.pages >= 1);

    assert_eq!(model.invalidate(&ChangeSet::new()), DirtyPages::None);
    let mut change = ChangeSet::new();
    change.record(ContentChange {
        source: SourceRef::node(address::BODY, mjx_layout::SourcePath::new(&[0])),
        kind: ChangeKind::Reformatted,
    });
    assert_eq!(
        model.invalidate(&change),
        DirtyPages::From(PageIndex::FIRST),
        "an edit to a flowing document dirties a suffix"
    );

    assert_eq!(
        model.dirty_from_paragraph(),
        Some(0),
        "the paragraph an edit dirtied is reported, because the *page* it dirtied is a question \
         about the caller's checkpoints and not about this box model"
    );
    assert!(model.paragraphs_visited() >= 1);
    assert!(model.paragraphs_visited_in_total() >= 1);
    let _ = model.rasteriser_mut();
    let _ = model.fonts_mut();
}

#[test]
fn the_flow_is_reachable() {
    let mut document = support::document(&[
        support::paragraph("", "One."),
        support::paragraph("", "Two."),
    ]);
    let flow = DocumentFlow::read(&mut document).expect("a flow");
    assert_eq!(flow.paragraph_count(), 2);
    assert_eq!(flow.paragraphs().len(), 2);
    assert_eq!(flow.formatting().paragraphs().len(), 2);

    let same = DocumentFlow::from_formatting(document.formatting().expect("formatting"));
    assert_eq!(same.paragraph_count(), 2);
}

/// A section's own page geometry becomes constraints, so a caller does not have to know that twips
/// are twentieths of a point.
#[test]
fn a_section_becomes_constraints() {
    let mut document = support::document_with(
        &[support::paragraph("", "One.")],
        &support::page_geometry(8.5, 11.0, 1.0),
    );
    let flow = support::flow(&mut document);
    let section = flow.formatting().sections().first().expect("a section");
    let constraints = constraints_for(section);
    assert_eq!(constraints.page.width, Emu::from_twips(12_240));
    assert_eq!(constraints.content.left, Emu::from_twips(1_440));
    assert_eq!(constraints.column_count(), 1);
}

#[test]
fn the_pagination_types_are_reachable() {
    assert_eq!(FlowPosition::START.block, 0);
    let widths = [Emu::from_inches(3.0), Emu::from_inches(3.0)];
    let shape = PageShape {
        height: Emu::from_inches(11.0),
        columns: 2,
        widths: &widths,
        width: Emu::from_inches(3.0),
        section_last: Some(9),
        balance: true,
        frame: mjx_layout_docx::Anchorage::contained(
            Emu::from_inches(3.0),
            Emu::from_inches(11.0),
            Emu::ZERO,
        ),
    };
    assert_eq!(shape.height, Emu::from_inches(11.0));
    assert_eq!(shape.columns, 2);
    assert_eq!(shape.section_last, Some(9));
}

#[test]
fn the_field_types_are_reachable() {
    use mjx_layout_docx::{
        evaluate_field, resolve_fields, CachedReason, Convergence, Evaluation, FieldAddress,
        FieldEnvironment, FieldKind, Instruction, SequenceCounters, MAXIMUM_PASSES,
    };
    const { assert!(MAXIMUM_PASSES >= 2) };
    assert_eq!(FieldAddress::BODY, 0);
    assert_eq!(FieldAddress::body(3, 1).paragraph, 3);
    assert_eq!(FieldAddress::stream(0, 1, 0).stream, 1);
    let instruction = Instruction::parse(" PAGE ");
    assert_eq!(instruction.kind(), FieldKind::Page);
    let mut counters = SequenceCounters::new();
    assert_eq!(counters.next("Figure"), 1);
    assert_eq!(counters.current("Figure"), 1);
    counters.reset("Figure", 5);
    assert_eq!(counters.current("Figure"), 5);
    let mut environment = FieldEnvironment::cached_results();
    assert!(environment.is_empty());
    environment.observe_total_pages(4);
    environment.observe_block(0, 1);
    environment.observe_section_pages(0, 4);
    environment.observe_bookmark("target", 2, "a heading");
    assert_eq!(environment.total_pages(), Some(4));
    assert_eq!(environment.page_of_block(0), Some(1));
    assert_eq!(environment.for_page(9).total_pages(), Some(4));
    assert_eq!(
        environment.clone().with_now("1 January 2026").total_pages(),
        Some(4)
    );
    let field = mjx_docx::FieldSpan {
        at: 0,
        result: 0..1,
        instruction: " PAGE ".to_owned(),
        form: mjx_docx::FieldForm::Complex,
        dirty: false,
        locked: false,
        parent: None,
    };
    assert_eq!(
        evaluate_field(&field, &environment, &mut counters, Some(0), Some(0)),
        Evaluation::Computed("1".to_owned())
    );
    assert_eq!(
        evaluate_field(
            &field,
            &FieldEnvironment::cached_results(),
            &mut counters,
            None,
            None
        ),
        Evaluation::Cached(CachedReason::TargetAbsent)
    );
    let (_, convergence) = resolve_fields(|_| -> Result<FieldEnvironment, ()> {
        Ok(FieldEnvironment::cached_results())
    })
    .expect("the loop runs");
    assert!(convergence.converged());
    assert!(!Convergence::Exhausted.converged());
}

#[test]
fn the_generated_content_types_are_reachable() {
    use mjx_layout_docx::{
        compose, Composition, Generated, InlineObjectKind, PieceKind, PieceRevision, RevisionView,
        OBJECT_REPLACEMENT, SUPERSCRIPT_SCALE,
    };
    assert_eq!(OBJECT_REPLACEMENT, '\u{FFFC}');
    const { assert!(SUPERSCRIPT_SCALE > 0.0 && SUPERSCRIPT_SCALE < 1.0) };
    let mut document = support::document(&[support::paragraph("", "a plain paragraph")]);
    let flow = support::flow(&mut document);
    let paragraph = &flow.paragraphs()[0];
    let composed: Composition = compose(paragraph, &Generated::default());
    assert_eq!(composed.text(), "a plain paragraph");
    assert!(!composed.change_bar());
    assert!(composed.objects().is_empty());
    assert!(composed.equations().is_empty());
    assert_eq!(composed.pieces()[0].kind, PieceKind::Document);
    assert_eq!(composed.document_range(&(0..3)), 0..3);
    assert!(composed.object_at(0).is_none());
    assert_eq!(
        composed.style().alignment,
        mjx_layout_docx::Alignment::Start
    );
    assert_eq!(composed.runs().len(), 1);
    let plain = Composition::plain(paragraph);
    assert_eq!(plain.text(), composed.text());
    // The two enumerations, named so that a change to either fails here.
    let _ = InlineObjectKind::Equation(0);
    let _ = PieceRevision {
        kind: mjx_docx::RevisionKind::Inserted,
        author: None,
    };
    assert!(RevisionView::default().shows_change_bars());
}

#[test]
fn the_list_types_are_reachable() {
    use mjx_layout_docx::{ListNumbering, Marker, LEVELS};
    const { assert!(LEVELS == 9) };
    let marker = Marker {
        text: "1.".to_owned(),
        suffix: mjx_ooxml_types::wordprocessingml::NumberingLevelSuffix::Tab,
        level: 0,
        exact: true,
        picture_bullet: None,
    };
    assert_eq!(marker.with_suffix(), "1.\t");
    let empty = ListNumbering::default();
    assert!(empty.is_empty() && empty.marker(0).is_none());
    assert_eq!(empty.len(), 0);
}

#[test]
fn the_revision_types_are_reachable() {
    use mjx_docx::{RevisionKind, RevisionSpan};
    use mjx_layout_docx::{changes_anything, has_content_change, RevisionView};
    let spans = vec![RevisionSpan {
        range: 0..4,
        kind: RevisionKind::Deleted,
        author: Some("Priya".to_owned()),
        date: None,
    }];
    assert!(has_content_change(&spans));
    assert!(changes_anything(&spans, RevisionView::NoMarkup));
    assert!(!changes_anything(&spans, RevisionView::AllMarkup));
    assert!(RevisionView::AllMarkup.shows(RevisionKind::Deleted));
    assert!(!RevisionView::Original.shows(RevisionKind::Inserted));
}

#[test]
fn the_math_types_are_reachable() {
    use mjx_layout_docx::{
        MathBox, MathContent, MathContext, PlacedMathBox, AXIS_HEIGHT_IN_EMS,
        MAXIMUM_DELIMITER_GROWTH, MAXIMUM_DEPTH, RULE_THICKNESS_IN_EMS, SCRIPT_SCALE,
        SCRIPT_SCRIPT_SCALE,
    };
    const { assert!(AXIS_HEIGHT_IN_EMS > 0.0 && RULE_THICKNESS_IN_EMS > 0.0) };
    const { assert!(SCRIPT_SCRIPT_SCALE < SCRIPT_SCALE && SCRIPT_SCALE < 1.0) };
    const { assert!(MAXIMUM_DELIMITER_GROWTH > 1.0) };
    const { assert!(MAXIMUM_DEPTH >= 8) };
    let style = RunStyle {
        range: 0..0,
        family: "Liberation Sans".to_owned(),
        size: mjx_text::FontSize::from_points(12.0),
        weight: mjx_text::FontWeight::REGULAR,
        slant: mjx_text::FontSlant::Upright,
        language: None,
        hidden: false,
    };
    let context = MathContext::new(&style);
    assert_eq!(context.script_level, 0);
    assert_eq!(context.scripted().script_level, 1);
    assert!(context.axis() > Emu::ZERO);
    assert!(context.rule_thickness() > Emu::ZERO);
    assert!(context.deeper().depth == 1);
    assert!(context.em() > Emu::ZERO);
    let empty = MathBox::empty();
    assert_eq!(empty.height(), Emu::ZERO);
    assert_eq!(empty.content, MathContent::Group);
    let placed = PlacedMathBox {
        x: Emu::ZERO,
        baseline: Emu::ZERO,
        content: MathBox::empty(),
    };
    assert_eq!(placed.content.width, Emu::ZERO);
}
