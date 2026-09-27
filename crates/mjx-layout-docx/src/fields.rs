//! Fields: what a field's instruction says, what it evaluates to, and **the fixed point between a
//! field's value and the pagination that produced it — with its termination argument written out**.
//!
//! # The trap this module exists to avoid, stated before anything else
//!
//! **Every field in every file carries a cached result.** `w:fldSimple`'s children and the runs
//! between a complex field's `w:separate` and its `w:end` are the value Word computed the last time
//! it saved. They are ordinary `w:t`, so they are already in a paragraph's text, and a renderer that
//! evaluates nothing at all and simply draws them **looks perfect on every document Word last
//! touched**.
//!
//! It is not merely a weak implementation; it is one that cannot be told apart from a correct one by
//! any test written against a normal fixture. So the gate is written against an *abnormal* one:
//! `tests/a_stale_field_is_recomputed.rs` builds a document whose `PAGE` field on page three has the
//! stored value `1`, and asserts the rendered value is `3`. It then asserts the same suite fails
//! when the evaluator is switched off, because a gate that cannot fail is not a gate.
//!
//! # Which fields are computed, and which are passed through
//!
//! Computed here — **the ones whose value is a fact about the layout**, which is the only reason a
//! box model is the right place for any of them: `PAGE`, `NUMPAGES`, `SECTIONPAGES`, `PAGEREF`,
//! `REF` (both its text and its `\p` page form), `SEQ` with `\c`/`\r`/`\*`, `QUOTE`, and the
//! `DATE`/`TIME`/`PRINTDATE`/`SAVEDATE`/`CREATEDATE` family **from a clock the caller supplies**.
//!
//! Passed through — the cached result is rendered and the field is recorded as
//! [`Evaluation::Cached`] with a reason: `MERGEFIELD` and the mail-merge family (there is no data
//! source), `DOCPROPERTY`/`DOCVARIABLE` naming something the package does not hold, `HYPERLINK`
//! (whose *display* is its result and whose instruction is a target), `FORMTEXT`/`FORMCHECKBOX`/
//! `FORMDROPDOWN` (whose result is the user's own answer), `INCLUDETEXT`, `LINK`, `USERNAME`,
//! `FILENAME`, `AUTHOR`, `CITATION`, and everything this module does not name.
//!
//! A `TOC` is in the second list and that is a decision rather than an omission: the entries between
//! its `w:separate` and its `w:end` are ordinary text plus a **nested** `PAGEREF` each, and those
//! nested fields *are* evaluated. So a table of contents' page numbers are recomputed and its entry
//! list is not — which is exactly the split Word's own markup makes, and is why a stale `TOC` in a
//! document whose headings have not changed comes out right.
//!
//! **Never invent a value.** A field whose data is absent renders what the file says and is
//! reported; a field that renders a guess is a document that says something its author did not.
//!
//! # ⚠ The fixed point, and why the two-assembly bound survives it
//!
//! A `PAGE` field's *value* depends on where the page break fell. Its *width* — one digit or two —
//! changes where the line breaks, therefore where the page breaks, therefore what the value is. That
//! is a genuine cycle, and it is the second one this crate holds (the first is `crate::notes`').
//!
//! It is cut in a way that leaves `crate::notes`' proof **completely untouched**, and the reason is
//! worth stating precisely, because getting it wrong is how a hang gets in:
//!
//! * The note fixed point iterates *within one page*, varying the reserved height *R* and
//!   re-assembling the body. `PageReport::assemblies` asserts it never needs a third assembly.
//! * A [`FieldEnvironment`] is **constant for a whole layout run**. Every value in it is decided
//!   before the first page is laid out and does not change while it is. So a field's rendered text
//!   is *the same string* in assembly one and assembly two, every block's layout is the same
//!   function of *R* it was before this module existed, and the monotonicity fact the note proof
//!   rests on (*the body content placed is non-increasing in R*) is unchanged.
//! * **In particular a body `PAGE` field does not read "the page being assembled".** It reads
//!   [`FieldEnvironment::page_of_block`] — the page that block started on *in the previous pass* —
//!   which is a constant. Reading the current page would have been the obvious implementation and is
//!   wrong twice over: it makes a field's text depend on the assembly, and it makes a paragraph that
//!   *splits* across a page boundary lay out differently on the two pages, which can drop or repeat
//!   a line.
//! * The one value that is page-local is a `PAGE` field in a **header, footer or note**, and it is
//!   safe because those streams are laid out afresh per page and never split: their environment is
//!   [`FieldEnvironment::for_page`], a copy with one number replaced, and it too is constant for the
//!   page.
//!
//! So the circularity lives entirely in an **outer** loop, [`resolve`], which paginates the whole
//! document, reads the page every block landed on, and paginates again.
//!
//! ## Why the outer loop terminates
//!
//! It does not terminate by convergence, because it **cannot be proved to converge**: a document can
//! be built whose `NUMPAGES` field is 9 when the document is 10 pages long and 10 when it is 9, and
//! that oscillates for ever. Word has the same problem and the same answer — press F9 twice and
//! watch the number flip — so the honest design is a *bounded* iteration with a documented fallback
//! rather than a proof that cannot be had:
//!
//! 1. pass 0 evaluates every field against an empty environment, so every page-dependent one renders
//!    its **cached result**;
//! 2. each pass paginates the whole document, records where every block and every bookmark landed,
//!    and evaluates again;
//! 3. the loop stops the moment a pass produces an environment equal to the one it was given — the
//!    fixed point, and the ordinary outcome for every real document;
//! 4. otherwise it stops after [`MAXIMUM_PASSES`], reports [`Convergence::Exhausted`], and **the
//!    caller decides**: [`resolve`] hands back the last environment, and a caller that would rather
//!    show what Word last computed passes [`FieldEnvironment::cached_results`].
//!
//! `tests/a_field_fixed_point_terminates.rs` runs both: a document whose `TOC` grows from one page
//! to two and settles, and a deliberately non-converging one that exhausts the budget and falls
//! back. The second is the important one — a fallback that is never exercised is a fallback that
//! does not work.

use std::collections::BTreeMap;

use mjx_docx::FieldSpan;
use mjx_ooxml_types::wordprocessingml::NumberFormat;

use crate::numbering::format_number;

/// How many whole-document passes [`resolve`] may take before it gives up and reports
/// [`Convergence::Exhausted`].
///
/// **Four**, and the number is reasoned rather than round. One pass settles a document with no
/// length-changing field in it; two settle `PAGE` and `PAGEREF`, whose values depend on a pagination
/// that the values themselves barely perturb; three settle a `TOC` that grows by a page, because the
/// growth moves every later page and the entries have to be read again; four is one more than any
/// document anybody has produced here needs, and is where a genuinely oscillating document is
/// declared oscillating instead of being iterated for ever.
pub const MAXIMUM_PASSES: usize = 4;

/// What a field's instruction asks for, once its keyword has been read.
///
/// Only the renderer-relevant ones are named. Everything else is [`FieldKind::Other`] and renders
/// its cached result — which is not a gap in coverage but the design: a field this crate cannot
/// compute must show what the file says, and a variant per unimplemented keyword would be ninety
/// variants that all do the same thing.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FieldKind {
    /// `PAGE` — the number of the page it is on, in this section's own numbering.
    Page,
    /// `NUMPAGES` — how many pages the document has.
    NumberOfPages,
    /// `SECTIONPAGES` — how many pages this section has.
    SectionPages,
    /// `PAGEREF <bookmark>` — the page a bookmark is on.
    PageReference {
        /// The bookmark named.
        bookmark: String,
        /// `\p` — render *above*/*below* relative to this field's own page rather than the number.
        relative: bool,
    },
    /// `REF <bookmark>` — a bookmark's own text, or with `\p` its page.
    Reference {
        /// The bookmark named.
        bookmark: String,
        /// `\p`.
        page: bool,
    },
    /// `SEQ <name>` — the next value of a named counter.
    Sequence {
        /// The counter's name.
        name: String,
        /// `\c` — repeat the current value rather than advancing.
        repeat: bool,
        /// `\n` — advance (the default).
        next: bool,
        /// `\r <n>` — reset the counter to *n*.
        reset: Option<i64>,
        /// `\* <format>` — the numeral system.
        format: NumberFormat,
    },
    /// `DATE`, `TIME`, `PRINTDATE`, `SAVEDATE`, `CREATEDATE`.
    DateTime,
    /// `QUOTE <text>` — literal text.
    Quote(String),
    /// `TOC` — a table of contents. Its own entries are nested `PAGEREF`s; this field renders
    /// nothing of its own.
    TableOfContents,
    /// Anything else.
    Other,
}

/// One field's instruction, parsed.
///
/// # The instruction language, and how much of it is read here
///
/// §17.16.5 defines a small grammar: a keyword, then arguments, then switches. An argument may be
/// quoted, and inside quotes a backslash escapes the next character. A switch is a backslash and one
/// letter, optionally followed by an argument of its own. That is all this parses — it does **not**
/// evaluate nested field instructions inside an argument, because a nested field is a
/// [`FieldSpan`] of its own with its own instruction and its own result.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Instruction {
    /// The keyword, upper-cased. Field names are case-insensitive in Word and are compared here in
    /// one case rather than at every site.
    pub keyword: String,
    /// The positional arguments, in order, with quoting removed.
    pub arguments: Vec<String>,
    /// The switches, as `(letter, argument)`. The letter keeps its case: `\*` and `\#` are distinct
    /// from letters, and `\C` is not `\c`.
    pub switches: Vec<(char, Option<String>)>,
}

impl Instruction {
    /// Parses one instruction.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let tokens = tokenise(text);
        let mut iterator = tokens.into_iter();
        let keyword = iterator
            .next()
            .map(|token| token.text.to_uppercase())
            .unwrap_or_default();
        let mut arguments = Vec::new();
        let mut switches: Vec<(char, Option<String>)> = Vec::new();
        let mut pending_switch: Option<usize> = None;
        for token in iterator {
            if token.is_switch {
                let letter = token.text.chars().next().unwrap_or('?');
                switches.push((letter, None));
                pending_switch = Some(switches.len() - 1);
                // `\*` and `\#` take their argument as the rest of the token when it is written
                // without a space — `\*MERGEFORMAT` is one token in files Word itself writes.
                let rest: String = token.text.chars().skip(1).collect();
                if !rest.is_empty() {
                    if let Some(index) = pending_switch.take() {
                        switches[index].1 = Some(rest);
                    }
                }
                continue;
            }
            match pending_switch.take() {
                Some(index) => switches[index].1 = Some(token.text),
                None => arguments.push(token.text),
            }
        }
        Self {
            keyword,
            arguments,
            switches,
        }
    }

    /// The argument of switch `letter`, if it carries one.
    #[must_use]
    pub fn switch(&self, letter: char) -> Option<&str> {
        self.switches
            .iter()
            .find(|(name, _)| *name == letter)
            .and_then(|(_, argument)| argument.as_deref())
    }

    /// Whether switch `letter` is present at all.
    #[must_use]
    pub fn has_switch(&self, letter: char) -> bool {
        self.switches.iter().any(|(name, _)| *name == letter)
    }

    /// What this instruction asks for.
    #[must_use]
    pub fn kind(&self) -> FieldKind {
        let first = || self.arguments.first().cloned().unwrap_or_default();
        match self.keyword.as_str() {
            "PAGE" => FieldKind::Page,
            "NUMPAGES" => FieldKind::NumberOfPages,
            "SECTIONPAGES" => FieldKind::SectionPages,
            "PAGEREF" => FieldKind::PageReference {
                bookmark: first(),
                relative: self.has_switch('p'),
            },
            "REF" => FieldKind::Reference {
                bookmark: first(),
                page: self.has_switch('p'),
            },
            "SEQ" => FieldKind::Sequence {
                name: first(),
                repeat: self.has_switch('c'),
                next: !self.has_switch('c'),
                reset: self
                    .switch('r')
                    .and_then(|value| value.trim().parse::<i64>().ok()),
                format: numeral_format(self.switch('*')),
            },
            "DATE" | "TIME" | "PRINTDATE" | "SAVEDATE" | "CREATEDATE" => FieldKind::DateTime,
            "QUOTE" => FieldKind::Quote(self.arguments.join(" ")),
            "TOC" => FieldKind::TableOfContents,
            _ => FieldKind::Other,
        }
    }
}

/// The numeral system a `\*` switch names.
///
/// Word's own vocabulary here is *not* `w:numFmt`'s — `\* roman` and `\* ALPHABETIC` — so the
/// mapping is stated rather than assumed, and an unrecognised one falls back to Arabic rather than
/// refusing the field.
fn numeral_format(switch: Option<&str>) -> NumberFormat {
    match switch.map(str::trim).unwrap_or("") {
        "roman" => NumberFormat::LowercaseRomanNumerals,
        "ROMAN" => NumberFormat::UppercaseRomanNumerals,
        "alphabetic" => NumberFormat::LowercaseLatinAlphabet,
        "ALPHABETIC" => NumberFormat::UppercaseLatinAlphabet,
        "Ordinal" | "ordinal" => NumberFormat::Ordinal,
        "CardText" | "cardtext" => NumberFormat::CardinalText,
        "OrdText" | "ordtext" => NumberFormat::OrdinalText,
        _ => NumberFormat::Decimal,
    }
}

/// One token of an instruction.
struct Token {
    text: String,
    is_switch: bool,
}

/// Splits an instruction into tokens: quoted strings are one token, a backslash escapes the next
/// character inside quotes, and a backslash outside quotes starts a switch.
fn tokenise(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut is_switch = false;
    let mut escaped = false;
    let mut started = false;
    for character in text.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        match character {
            '\\' if in_quotes => escaped = true,
            '\\' if !in_quotes => {
                if started {
                    tokens.push(Token {
                        text: std::mem::take(&mut current),
                        is_switch,
                    });
                }
                started = true;
                is_switch = true;
            }
            '"' => {
                in_quotes = !in_quotes;
                started = true;
            }
            character if character.is_whitespace() && !in_quotes => {
                if started {
                    tokens.push(Token {
                        text: std::mem::take(&mut current),
                        is_switch,
                    });
                    started = false;
                    is_switch = false;
                }
            }
            character => {
                started = true;
                current.push(character);
            }
        }
    }
    if started {
        tokens.push(Token {
            text: current,
            is_switch,
        });
    }
    tokens
}

/// Where one field is, across the whole document.
///
/// A field's identity has to survive being re-evaluated on every pass of the fixed point, and it has
/// to survive the document being laid out at a different page size — so it is *where the field is in
/// the document* and never *where it landed*.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct FieldAddress {
    /// Which stream: [`FieldAddress::BODY`], or a header/footer/note stream's own index plus one.
    pub stream: usize,
    /// Which paragraph of that stream.
    pub paragraph: usize,
    /// Which field of that paragraph, as [`mjx_docx::ParagraphFormatting::fields`] numbers them.
    pub field: usize,
}

impl FieldAddress {
    /// The body's stream number.
    pub const BODY: usize = 0;

    /// One field of the body.
    #[must_use]
    pub fn body(paragraph: usize, field: usize) -> Self {
        Self {
            stream: Self::BODY,
            paragraph,
            field,
        }
    }

    /// One field of a secondary stream.
    #[must_use]
    pub fn stream(stream: usize, paragraph: usize, field: usize) -> Self {
        Self {
            stream: stream.saturating_add(1),
            paragraph,
            field,
        }
    }
}

/// What a field rendered, and where the value came from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Evaluation {
    /// Computed by this crate. The string is what is drawn.
    Computed(String),
    /// **Not** computed: the cached result in the file is what is drawn, and this says why.
    ///
    /// The reason is not decoration — it is the ledger entry the ticket asks for, and it is what a
    /// caller shows a reader who asks why a field will not update.
    Cached(CachedReason),
}

/// Why a field rendered its cached result.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CachedReason {
    /// The field needs data the package does not carry — a merge source, a document property.
    ExternalDataAbsent,
    /// `w:fldLock`: the author locked the result and it must not be recomputed.
    Locked,
    /// This crate does not compute this field's kind.
    NotComputed,
    /// The field's kind is computable and its input is not — a `PAGEREF` naming a bookmark the
    /// document does not define.
    TargetAbsent,
}

/// Everything a field needs that is not in the field: where every block landed, where every bookmark
/// is, how long the document is.
///
/// **Immutable while a document is laid out.** See this module's own documentation for why that is
/// the whole of the compatibility argument with `crate::notes`' two-assembly bound.
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct FieldEnvironment {
    /// The page number each block starts on, as the previous pass observed it.
    page_of_block: BTreeMap<u32, i64>,
    /// The page number each bookmark is on.
    page_of_bookmark: BTreeMap<String, i64>,
    /// The text each bookmark covers.
    text_of_bookmark: BTreeMap<String, String>,
    /// How many pages the document has.
    total_pages: Option<i64>,
    /// How many pages each section has, by section index.
    pages_of_section: BTreeMap<usize, i64>,
    /// The page a *stream* — a header, a footer, a note — is being drawn on. Set by
    /// [`FieldEnvironment::for_page`] and by nothing else.
    stream_page: Option<i64>,
    /// The date and time a `DATE` field renders, as the caller's own string. Absent means a `DATE`
    /// field renders its cache — this crate never reads the host's clock, because a layout that
    /// changed with the wall clock could not have a committed baseline.
    now: Option<String>,
}

impl FieldEnvironment {
    /// The environment pass zero uses: nothing is known, so every page-dependent field renders its
    /// cached result.
    #[must_use]
    pub fn cached_results() -> Self {
        Self::default()
    }

    /// The same environment with the page a secondary stream is being drawn on.
    #[must_use]
    pub fn for_page(&self, page_number: i64) -> Self {
        let mut copy = self.clone();
        copy.stream_page = Some(page_number);
        copy
    }

    /// The date and time `DATE`, `TIME` and their kin render.
    #[must_use]
    pub fn with_now(mut self, now: impl Into<String>) -> Self {
        self.now = Some(now.into());
        self
    }

    /// Records that block `block` starts on page `page`.
    pub fn observe_block(&mut self, block: u32, page: i64) {
        self.page_of_block.entry(block).or_insert(page);
    }

    /// Records a bookmark's page and text.
    pub fn observe_bookmark(
        &mut self,
        name: impl Into<String>,
        page: i64,
        text: impl Into<String>,
    ) {
        let name = name.into();
        self.page_of_bookmark.insert(name.clone(), page);
        self.text_of_bookmark.insert(name, text.into());
    }

    /// Records how long the document is.
    pub fn observe_total_pages(&mut self, pages: i64) {
        self.total_pages = Some(pages);
    }

    /// Records how long one section is.
    pub fn observe_section_pages(&mut self, section: usize, pages: i64) {
        self.pages_of_section.insert(section, pages);
    }

    /// The page block `block` starts on, if a pass has observed it.
    #[must_use]
    pub fn page_of_block(&self, block: u32) -> Option<i64> {
        self.page_of_block.get(&block).copied()
    }

    /// How many pages the document has, if a pass has observed it.
    #[must_use]
    pub fn total_pages(&self) -> Option<i64> {
        self.total_pages
    }

    /// Whether anything at all has been observed — that is, whether this is pass zero's environment.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.page_of_block.is_empty()
            && self.page_of_bookmark.is_empty()
            && self.total_pages.is_none()
            && self.pages_of_section.is_empty()
    }
}

/// The counters a document's `SEQ` and `LISTNUM` fields advance as a pass walks it.
///
/// Separate from [`FieldEnvironment`] because it is **not** constant: a `SEQ` field's value is a
/// function of how many `SEQ` fields of the same name precede it in document order, so it is
/// accumulated as the evaluator walks and reset at the start of each pass. It is here rather than in
/// the environment precisely so that nothing about it can leak into the pagination cycle.
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct SequenceCounters {
    counters: BTreeMap<String, i64>,
}

impl SequenceCounters {
    /// A fresh set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances `name` and answers its new value.
    pub fn next(&mut self, name: &str) -> i64 {
        let counter = self.counters.entry(name.to_owned()).or_insert(0);
        *counter = counter.saturating_add(1);
        *counter
    }

    /// The current value of `name` without advancing it.
    #[must_use]
    pub fn current(&self, name: &str) -> i64 {
        self.counters.get(name).copied().unwrap_or(0)
    }

    /// Sets `name` to `value`.
    pub fn reset(&mut self, name: &str, value: i64) {
        self.counters.insert(name.to_owned(), value);
    }
}

/// Every `SEQ` field in a document, evaluated once in document order.
///
/// # Why this is a whole-document walk and not a counter passed around
///
/// A `SEQ Figure` field's value is *how many `SEQ Figure` fields precede it*, which is a fact about
/// the document and not about the paragraph — exactly the shape
/// [`crate::lists::ListNumbering`] already has, and for the same reason.
///
/// It cannot be a running counter threaded through the layout, and that is the trap: a paragraph is
/// laid out **out of order and more than once** — a page is assembled twice for its notes, a block
/// beside a float is laid out twice for its own top, and a walk to page forty lays out every page
/// before it. A counter advanced at layout time would answer a different number each time and would
/// depend on the page a reader happened to open. Composing them once, in document order, is the only
/// answer that is a function of the file.
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct SequenceValues {
    values: BTreeMap<(usize, usize), String>,
}

impl SequenceValues {
    /// Walks `paragraphs` in document order and renders every `SEQ` field in them.
    #[must_use]
    pub fn read(paragraphs: &[mjx_docx::ParagraphFormatting]) -> Self {
        let mut counters = SequenceCounters::new();
        let mut values = BTreeMap::new();
        for (at, paragraph) in paragraphs.iter().enumerate() {
            for (index, field) in paragraph.fields().iter().enumerate() {
                let instruction = Instruction::parse(&field.instruction);
                let FieldKind::Sequence {
                    name,
                    repeat,
                    next: _,
                    reset,
                    format,
                } = instruction.kind()
                else {
                    continue;
                };
                if field.locked {
                    continue;
                }
                let value = match reset {
                    Some(value) => {
                        counters.reset(&name, value);
                        value
                    }
                    None if repeat => counters.current(&name),
                    None => counters.next(&name),
                };
                values.insert((at, index), format_number(value, format));
            }
        }
        Self { values }
    }

    /// What the `field`th field of paragraph `paragraph` renders, when it is a `SEQ`.
    #[must_use]
    pub fn value(&self, paragraph: usize, field: usize) -> Option<&str> {
        self.values
            .get(&(paragraph, field))
            .map(std::string::String::as_str)
    }

    /// How many `SEQ` fields the document holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether it holds none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// What one field renders.
///
/// `page` is the page the field's own **block** starts on for a body field, and the page being drawn
/// for a stream field; see this module's documentation for why those are different questions.
#[must_use]
pub fn evaluate(
    span: &FieldSpan,
    environment: &FieldEnvironment,
    counters: &mut SequenceCounters,
    section: Option<usize>,
    block: Option<u32>,
) -> Evaluation {
    if span.locked {
        return Evaluation::Cached(CachedReason::Locked);
    }
    let instruction = Instruction::parse(&span.instruction);
    let page_here = || match environment.stream_page {
        Some(page) => Some(page),
        None => block.and_then(|block| environment.page_of_block(block)),
    };
    match instruction.kind() {
        FieldKind::Page => match page_here() {
            Some(page) => {
                Evaluation::Computed(format_number(page, numeral_format(instruction.switch('*'))))
            }
            None => Evaluation::Cached(CachedReason::TargetAbsent),
        },
        FieldKind::NumberOfPages => match environment.total_pages {
            Some(pages) => Evaluation::Computed(format_number(
                pages,
                numeral_format(instruction.switch('*')),
            )),
            None => Evaluation::Cached(CachedReason::TargetAbsent),
        },
        FieldKind::SectionPages => {
            match section.and_then(|index| environment.pages_of_section.get(&index).copied()) {
                Some(pages) => Evaluation::Computed(format_number(
                    pages,
                    numeral_format(instruction.switch('*')),
                )),
                None => Evaluation::Cached(CachedReason::TargetAbsent),
            }
        }
        FieldKind::PageReference {
            bookmark,
            relative: _,
        } => match environment.page_of_bookmark.get(&bookmark).copied() {
            Some(page) => {
                Evaluation::Computed(format_number(page, numeral_format(instruction.switch('*'))))
            }
            // A `PAGEREF` naming a bookmark the document does not define is exactly the
            // `fields_and_hyperlinks.docx` case: Word wrote `_Toc1` into a table of contents and the
            // bookmark is not in the file. Rendering the cache is right, and saying so is the point.
            None => Evaluation::Cached(CachedReason::TargetAbsent),
        },
        FieldKind::Reference { bookmark, page } => {
            if page {
                match environment.page_of_bookmark.get(&bookmark).copied() {
                    Some(number) => Evaluation::Computed(format_number(
                        number,
                        numeral_format(instruction.switch('*')),
                    )),
                    None => Evaluation::Cached(CachedReason::TargetAbsent),
                }
            } else {
                match environment.text_of_bookmark.get(&bookmark) {
                    Some(text) => Evaluation::Computed(text.clone()),
                    None => Evaluation::Cached(CachedReason::TargetAbsent),
                }
            }
        }
        FieldKind::Sequence {
            name,
            repeat,
            next: _,
            reset,
            format,
        } => {
            if let Some(value) = reset {
                counters.reset(&name, value);
                return Evaluation::Computed(format_number(value, format));
            }
            let value = if repeat {
                counters.current(&name)
            } else {
                counters.next(&name)
            };
            Evaluation::Computed(format_number(value, format))
        }
        FieldKind::Quote(text) => Evaluation::Computed(text),
        FieldKind::DateTime => match &environment.now {
            Some(now) => Evaluation::Computed(now.clone()),
            // **The host's clock is deliberately not read.** A `DATE` field that rendered
            // `std::time::SystemTime::now` would make this crate's own committed baselines expire
            // overnight, and would make two runs of the same document differ — which is the one
            // property every gate in this workspace rests on.
            None => Evaluation::Cached(CachedReason::ExternalDataAbsent),
        },
        // A `TOC` renders nothing of its own: what a reader sees is the entries between its
        // `w:separate` and its `w:end`, each of which is ordinary text plus a nested `PAGEREF` that
        // *is* evaluated. Recomputing the entry list would mean generating content this crate has no
        // outline to generate it from — the headings are in the document and their page numbers are
        // in the nested fields, which is exactly the split Word itself writes.
        FieldKind::TableOfContents => Evaluation::Cached(CachedReason::NotComputed),
        FieldKind::Other => Evaluation::Cached(reason_for(&instruction.keyword)),
    }
}

/// Why a field this crate does not compute renders its cache.
fn reason_for(keyword: &str) -> CachedReason {
    match keyword {
        "MERGEFIELD" | "MERGEREC" | "MERGESEQ" | "NEXT" | "NEXTIF" | "SKIPIF" | "DATABASE"
        | "DOCPROPERTY" | "DOCVARIABLE" | "INCLUDETEXT" | "INCLUDEPICTURE" | "LINK" | "DDE"
        | "DDEAUTO" | "USERNAME" | "USERADDRESS" | "USERINITIALS" | "FILENAME" | "FILESIZE"
        | "AUTHOR" | "TITLE" | "SUBJECT" | "KEYWORDS" | "COMMENTS" | "LASTSAVEDBY" | "NUMCHARS"
        | "NUMWORDS" | "TEMPLATE" | "CITATION" | "BIBLIOGRAPHY" => CachedReason::ExternalDataAbsent,
        _ => CachedReason::NotComputed,
    }
}

/// How a run of the fixed point ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Convergence {
    /// A pass produced the environment it was given. The ordinary outcome.
    Converged {
        /// How many passes it took, counting from one.
        passes: usize,
    },
    /// [`MAXIMUM_PASSES`] passes did not settle. The last environment is returned and the caller
    /// decides whether to use it or to fall back to the cached results.
    Exhausted,
}

impl Convergence {
    /// Whether the fixed point was reached.
    #[must_use]
    pub fn converged(self) -> bool {
        matches!(self, Self::Converged { .. })
    }
}

/// Runs the fixed point: `observe` paginates the whole document under an environment and reports what
/// it saw, and this iterates that until it stops changing or the budget runs out.
///
/// The closure rather than a concrete box model because the loop is the interesting part and the
/// pagination is not — and because a test can then drive the loop with a hand-written oscillator,
/// which is the only way to exercise [`Convergence::Exhausted`] without shipping a document designed
/// to hang.
///
/// # Errors
/// Whatever `observe` returns.
pub fn resolve<E>(
    mut observe: impl FnMut(&FieldEnvironment) -> Result<FieldEnvironment, E>,
) -> Result<(FieldEnvironment, Convergence), E> {
    let mut environment = FieldEnvironment::cached_results();
    for pass in 0..MAXIMUM_PASSES {
        let observed = observe(&environment)?;
        if observed == environment {
            return Ok((
                environment,
                Convergence::Converged {
                    passes: pass.saturating_add(1),
                },
            ));
        }
        environment = observed;
    }
    Ok((environment, Convergence::Exhausted))
}
