//! **The single most important assertion in MJXOFF-177**: a field whose stored value is wrong
//! renders the *computed* one.
//!
//! # Why an ordinary fixture proves nothing
//!
//! Every field in every `.docx` carries a cached result — `w:fldSimple`'s children, or the runs
//! between a complex field's `w:separate` and its `w:end`. It is ordinary `w:t`, so it is already in
//! `mjx_docx::ParagraphFormatting::text`, so **a renderer that evaluates nothing and draws what is
//! there looks perfect on every file Word last saved.** No fixture taken from Word can tell the two
//! implementations apart, because on such a fixture there is nothing to tell apart.
//!
//! So every fixture here is deliberately **wrong**: a `PAGE` field on page three whose stored value
//! is `1`, a `NUMPAGES` that says `99`, a `PAGEREF` that names a bookmark the document does not
//! define. The assertion is that the first two come out right and the third comes out **as the
//! cache**, which is the other half of the contract: a field this crate cannot compute must show
//! what the file says and never a guess.
//!
//! # And the suite proves it can fail
//!
//! [`the_gate_fails_without_the_evaluator`] lays the same document out with the environment
//! `FieldEnvironment::cached_results` — which is exactly what a renderer that evaluates nothing
//! produces — and asserts the stale value comes back. A gate that cannot fail is not a gate, and
//! this one's failure mode is the implementation it exists to refuse.

mod support;

use mjx_layout::{BoxModel, PageIndex};
use mjx_layout_docx::{
    CachedReason, Composition, DocumentFlow, Evaluation, FieldEnvironment, Instruction, PieceKind,
    SequenceCounters,
};
use support::generated::{complex_field, plain_run, raw_paragraph, simple_field};
use support::{constraints, document, flow, model};

/// The rendered text of every piece of a paragraph's composition that came from a field.
fn field_values(composition: &Composition) -> Vec<String> {
    composition
        .pieces()
        .iter()
        .filter(|piece| piece.kind == PieceKind::FieldResult)
        .filter_map(|piece| composition.text().get(piece.layout.clone()))
        .map(str::to_owned)
        .collect()
}

/// Everything the paragraph at `index` is laid out from, in `environment`.
fn composition_of(
    paragraphs: &[String],
    index: usize,
    environment: FieldEnvironment,
) -> Composition {
    let mut document = document(paragraphs);
    let read = flow(&mut document).with_fields(environment);
    let mut model = model();
    let constraints = constraints(6.5, 9.0);
    // One page is enough: the composition is a function of the environment and the paragraph, and
    // this asks for the one the first page laid out.
    model
        .layout_page(&read, PageIndex::FIRST, &constraints, None)
        .expect("the page lays out");
    let request = mjx_layout_docx::LayoutRequest {
        width: constraints.content.width(),
        top: mjx_ooxml_core::measure::Emu::ZERO,
        exclusions: &[],
    };
    let block = model
        .lay_out_block(&read, index, request)
        .expect("the block lays out");
    block
        .as_paragraph()
        .expect("a paragraph")
        .composition
        .clone()
}

/// An environment that has seen a three-page document whose second block starts on page three.
fn observed() -> FieldEnvironment {
    let mut environment = FieldEnvironment::cached_results();
    environment.observe_total_pages(3);
    environment.observe_block(0, 1);
    environment.observe_block(1, 3);
    environment.observe_bookmark("target", 2, "Chapter Two");
    environment
}

#[test]
fn a_page_field_renders_the_page_it_is_on_and_not_the_page_it_says() {
    let paragraphs = vec![
        raw_paragraph(&plain_run("first")),
        raw_paragraph(&format!(
            "{}{}",
            plain_run("page "),
            // **The stored value is 1 and the block starts on page 3.**
            complex_field(" PAGE ", "1")
        )),
    ];
    let composed = composition_of(&paragraphs, 1, observed());
    assert_eq!(
        field_values(&composed),
        vec!["3".to_owned()],
        "the rendered value is the computed one, not the stale `1` the file stores"
    );
    assert!(
        composed.text().contains("page 3"),
        "and it is on the line the reader sees: {:?}",
        composed.text()
    );
}

#[test]
fn a_simple_field_is_recomputed_too() {
    // `w:fldSimple` and the `w:fldChar` triple are two spellings of one thing, and a renderer that
    // handled only the complex form would be right on documents Word wrote and wrong on documents
    // every other producer wrote.
    let paragraphs = vec![
        raw_paragraph(&plain_run("first")),
        raw_paragraph(&simple_field(" NUMPAGES ", "99")),
    ];
    let composed = composition_of(&paragraphs, 1, observed());
    assert_eq!(field_values(&composed), vec!["3".to_owned()]);
}

#[test]
fn the_gate_fails_without_the_evaluator() {
    // The implementation this suite exists to refuse: evaluate nothing, render the cache. It is what
    // `FieldEnvironment::cached_results` produces by construction, so this is not a mock — it is the
    // engine's own no-information path.
    let paragraphs = vec![
        raw_paragraph(&plain_run("first")),
        raw_paragraph(&format!(
            "{}{}",
            plain_run("page "),
            complex_field(" PAGE ", "1")
        )),
    ];
    let composed = composition_of(&paragraphs, 1, FieldEnvironment::cached_results());
    assert!(
        field_values(&composed).is_empty(),
        "nothing was replaced, so the cached `1` is what is on the line"
    );
    assert!(
        composed.text().contains("page 1"),
        "which is the stale value: {:?}",
        composed.text()
    );
}

#[test]
fn a_field_whose_target_is_absent_renders_its_cache_and_says_why() {
    // `tests/fixtures/fields_and_hyperlinks.docx`'s own case, in miniature: Word wrote a `TOC` whose
    // entries are `PAGEREF _Toc1` and `PAGEREF _Toc2`, and the bookmarks are not in the file.
    let field = mjx_docx::FieldSpan {
        at: 0,
        result: 0..1,
        instruction: " PAGEREF _Toc1 ".to_owned(),
        form: mjx_docx::FieldForm::Complex,
        dirty: false,
        locked: false,
        parent: None,
    };
    let mut counters = SequenceCounters::new();
    let evaluation =
        mjx_layout_docx::evaluate_field(&field, &observed(), &mut counters, None, Some(0));
    assert_eq!(
        evaluation,
        Evaluation::Cached(CachedReason::TargetAbsent),
        "a bookmark the document does not define is reported, not invented"
    );
}

#[test]
fn a_locked_field_is_never_recomputed() {
    // `w:fldLock` is the author saying *do not update this*, and honouring it is the one case where
    // rendering the cache is the **correct** answer rather than a fallback.
    let field = mjx_docx::FieldSpan {
        at: 0,
        result: 0..1,
        instruction: " PAGE ".to_owned(),
        form: mjx_docx::FieldForm::Complex,
        dirty: true,
        locked: true,
        parent: None,
    };
    let mut counters = SequenceCounters::new();
    let evaluation =
        mjx_layout_docx::evaluate_field(&field, &observed(), &mut counters, None, Some(1));
    assert_eq!(evaluation, Evaluation::Cached(CachedReason::Locked));
}

#[test]
fn a_field_that_needs_external_data_renders_its_cache_with_the_reason() {
    for keyword in [
        " MERGEFIELD Name ",
        " DOCPROPERTY Title ",
        " CITATION Knu86 ",
    ] {
        let field = mjx_docx::FieldSpan {
            at: 0,
            result: 0..1,
            instruction: keyword.to_owned(),
            form: mjx_docx::FieldForm::Complex,
            dirty: false,
            locked: false,
            parent: None,
        };
        let mut counters = SequenceCounters::new();
        let evaluation =
            mjx_layout_docx::evaluate_field(&field, &observed(), &mut counters, None, Some(1));
        assert_eq!(
            evaluation,
            Evaluation::Cached(CachedReason::ExternalDataAbsent),
            "`{keyword}` has no data source and must never invent one"
        );
    }
}

#[test]
fn an_instruction_is_split_into_a_keyword_arguments_and_switches() {
    // §17.16.5's grammar, and the three parts of it a renderer needs. The quoting matters: a
    // bookmark whose name has a space in it is written quoted, and a parser that split on whitespace
    // alone would look up the first word.
    let instruction = Instruction::parse(r#" PAGEREF "My Bookmark" \p \* MERGEFORMAT "#);
    assert_eq!(instruction.keyword, "PAGEREF");
    assert_eq!(instruction.arguments, vec!["My Bookmark".to_owned()]);
    assert!(instruction.has_switch('p'));
    assert_eq!(instruction.switch('*'), Some("MERGEFORMAT"));
}

#[test]
fn a_field_name_is_read_case_insensitively() {
    // Word writes `PAGE` and accepts `Page`; a renderer that compared exactly would show the cache
    // for a document a different producer wrote.
    assert_eq!(Instruction::parse(" Page ").keyword, "PAGE");
}

#[test]
fn a_sequence_field_counts_and_a_repeat_does_not_advance_it() {
    let mut counters = SequenceCounters::new();
    let field = |instruction: &str| mjx_docx::FieldSpan {
        at: 0,
        result: 0..1,
        instruction: instruction.to_owned(),
        form: mjx_docx::FieldForm::Complex,
        dirty: false,
        locked: false,
        parent: None,
    };
    let environment = FieldEnvironment::cached_results();
    let mut render = |instruction: &str| match mjx_layout_docx::evaluate_field(
        &field(instruction),
        &environment,
        &mut counters,
        None,
        None,
    ) {
        Evaluation::Computed(value) => value,
        Evaluation::Cached(reason) => panic!("expected a computed value, got {reason:?}"),
    };
    assert_eq!(render(" SEQ Figure "), "1");
    assert_eq!(render(" SEQ Figure "), "2");
    // `\c` repeats the current value — a caption that refers back to the figure above it.
    assert_eq!(render(r" SEQ Figure \c "), "2");
    assert_eq!(render(" SEQ Table "), "1", "a second counter is its own");
    // `\r` resets, and renders the value it reset to.
    assert_eq!(render(r" SEQ Figure \r 10 "), "10");
    assert_eq!(render(" SEQ Figure "), "11");
}

#[test]
fn a_numeral_switch_changes_the_system_a_page_number_is_written_in() {
    // `\* roman` is Word's own vocabulary and is not `w:numFmt`'s, so the mapping is a decision this
    // crate makes and a gate is where it is recorded.
    let field = mjx_docx::FieldSpan {
        at: 0,
        result: 0..1,
        instruction: r" PAGE \* roman ".to_owned(),
        form: mjx_docx::FieldForm::Complex,
        dirty: false,
        locked: false,
        parent: None,
    };
    let mut counters = SequenceCounters::new();
    let evaluation =
        mjx_layout_docx::evaluate_field(&field, &observed(), &mut counters, None, Some(1));
    assert_eq!(evaluation, Evaluation::Computed("iii".to_owned()));
}

#[test]
fn a_nested_field_names_its_parent() {
    // A `TOC` whose entries are `PAGEREF`s is the shape every table of contents has, and the nesting
    // is what stops a renderer computing the `TOC` by concatenating its children's instructions.
    let inner = complex_field(" PAGEREF _Toc1 ", "1");
    let outer = format!(
        concat!(
            r#"<w:r><w:fldChar w:fldCharType="begin"/></w:r>"#,
            r#"<w:r><w:instrText xml:space="preserve"> TOC \o "1-3" </w:instrText></w:r>"#,
            r#"<w:r><w:fldChar w:fldCharType="separate"/></w:r>"#,
            "{inner}",
            r#"<w:r><w:fldChar w:fldCharType="end"/></w:r>"#,
        ),
        inner = inner
    );
    let mut document = document(&[raw_paragraph(&outer)]);
    let read = DocumentFlow::read(&mut document).expect("the document reads");
    let fields = read.paragraphs()[0].fields();
    assert_eq!(fields.len(), 2, "the outer field and the nested one");
    assert_eq!(fields[0].field_name(), Some("TOC"));
    assert_eq!(fields[0].parent, None);
    assert_eq!(fields[1].field_name(), Some("PAGEREF"));
    assert_eq!(
        fields[1].parent,
        Some(0),
        "the `PAGEREF` belongs to the `TOC`, not to the paragraph"
    );
}

#[test]
fn a_sequence_field_counts_across_the_document_and_not_within_a_paragraph() {
    // **The defect this test was written for.** A `SEQ Figure` field's value is *how many like it
    // precede it in the document*, and the first implementation created a fresh counter per
    // paragraph — so every figure in a document was `1`. It is not catchable by a test of the
    // counters alone, because those count correctly; what was wrong was where they were kept.
    //
    // It also cannot be a counter advanced *at layout time*: a paragraph is laid out out of order
    // and more than once — twice for a page's notes, twice beside a float, and once per page on the
    // walk to page forty — so the number would depend on the page a reader opened.
    let paragraphs = vec![
        raw_paragraph(&format!(
            "{}{}",
            plain_run("Figure "),
            complex_field(" SEQ Figure ", "9")
        )),
        raw_paragraph(&plain_run("some prose between them")),
        raw_paragraph(&format!(
            "{}{}",
            plain_run("Figure "),
            complex_field(" SEQ Figure ", "9")
        )),
        raw_paragraph(&format!(
            "{}{}",
            plain_run("Table "),
            complex_field(" SEQ Table ", "9")
        )),
    ];
    assert_eq!(
        field_values(&composition_of(&paragraphs, 0, observed())),
        vec!["1".to_owned()]
    );
    assert_eq!(
        field_values(&composition_of(&paragraphs, 2, observed())),
        vec!["2".to_owned()],
        "the second figure is two, not one"
    );
    assert_eq!(
        field_values(&composition_of(&paragraphs, 3, observed())),
        vec!["1".to_owned()],
        "and a second counter is its own"
    );
}

#[test]
fn a_sequence_field_renders_the_same_value_however_often_the_paragraph_is_laid_out() {
    // The falsification of the fix: composing the same paragraph three times must give the same
    // answer. A running counter advanced at layout time gives 1, then 2, then 3.
    let paragraphs = vec![raw_paragraph(&format!(
        "{}{}",
        plain_run("Figure "),
        complex_field(" SEQ Figure ", "9")
    ))];
    let first = field_values(&composition_of(&paragraphs, 0, observed()));
    let second = field_values(&composition_of(&paragraphs, 0, observed()));
    let third = field_values(&composition_of(&paragraphs, 0, observed()));
    assert_eq!(first, vec!["1".to_owned()]);
    assert_eq!(first, second);
    assert_eq!(second, third);
}

/// [`mjx_docx::FieldSpan`] has no `field_name`; this is the same lexical split
/// [`Instruction::parse`] makes, for the assertion above to read plainly.
trait FieldName {
    fn field_name(&self) -> Option<&str>;
}

impl FieldName for mjx_docx::FieldSpan {
    fn field_name(&self) -> Option<&str> {
        self.instruction.split_whitespace().next()
    }
}
