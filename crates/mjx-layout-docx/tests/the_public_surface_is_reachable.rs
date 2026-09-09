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
        position: FlowPosition {
            paragraph: 7,
            line: 3,
        },
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
    assert_eq!(FlowPosition::START.paragraph, 0);
    let widths = [Emu::from_inches(3.0), Emu::from_inches(3.0)];
    let shape = PageShape {
        height: Emu::from_inches(11.0),
        columns: 2,
        widths: &widths,
        width: Emu::from_inches(3.0),
        section_last: Some(9),
        balance: true,
    };
    assert_eq!(shape.height, Emu::from_inches(11.0));
    assert_eq!(shape.columns, 2);
    assert_eq!(shape.section_last, Some(9));
}
