//! The fourth artefact, and the only one that is not a deck: **`w:overflowPunct` is a
//! WordprocessingML setting, so the hanging question cannot be asked of a `.pptx` at all.**
//!
//! # The question, in `mjx-text`'s own words
//!
//! `crates/mjx-text/src/line_breaking.rs` leaves it open by name:
//!
//! > *Should ASCII `,` and `.` be in the Japanese hangable set at all? In a Japanese paragraph with
//! > `w:overflowPunct` on, ASCII punctuation is used and Word does hang it — but exactly which
//! > characters, and whether a Latin paragraph in the same document is treated the same way, is a
//! > measurement against Word rather than something the specification states.*
//!
//! `KinsokuRules::japanese_standard`'s hangable set is `、。，．｡､,.` — eight characters, of which the
//! last two are ASCII and are the ones in question. This artefact asks about all eight, plus a Latin
//! paragraph as the control, and it asks about them **in one document**, because "does it treat a
//! Latin paragraph the same way" is a question about one Word instance and not two.
//!
//! # Why the text is long and repetitive rather than engineered
//!
//! The obvious construction is a measure chosen so that a particular character lands at a line end.
//! That requires knowing Word's own metrics for the East Asian font it substitutes — which is one of
//! the things this pack is trying to *find out*, so it cannot be an input.
//!
//! So instead: a narrow measure and a long run of `text、text。text，text．text,text.` repeated. Across
//! forty-odd lines every one of the eight candidates lands at a line end somewhere, and the reader
//! reports **which characters were observed with a box crossing the right margin**, over however many
//! lines there turn out to be. A construction that needs no prediction cannot mispredict.
//!
//! # The two paragraphs are a differential
//!
//! Paragraph one has `w:overflowPunct` explicitly **on**, paragraph two explicitly **off**, and both
//! carry the same text at the same measure. A reader who saw hanging in both would have learned that
//! the setting does nothing here; one who saw it in neither would have learned that the measure never
//! put a candidate at a line end. Neither conclusion is available from a single paragraph, which is
//! why there are two — and a third, in Latin script, for the *"and does it treat a Latin paragraph the
//! same way"* half of the question.

use mjx_docx::{Document, DocxError, MainDocument, Package, PageSize, PartName};
use mjx_ooxml_core::{FromXml, ToXml};

/// The file this document is written to.
pub const FILE_NAME: &str = "04-hanging-punctuation.docx";

/// The part every edit here lands in.
const DOCUMENT_XML: &str = "/word/document.xml";

/// The eight characters `KinsokuRules::japanese_standard` calls hangable, in its own order.
///
/// Restated here rather than read from `mjx-text` because that crate keeps the set private behind
/// `KinsokuRules::hangs`, one character at a time — so the list is checked against it by *asking*,
/// in `tests/the_hanging_document_asks_the_open_question.rs`, which is a stronger link than a
/// re-export would be: it proves the artefact probes exactly the set the engine acts on.
pub const HANGABLE: [char; 8] = ['、', '。', '，', '．', '｡', '､', ',', '.'];

/// How many times the probe sentence is repeated, which is what makes every candidate land at a line
/// end somewhere without predicting where.
pub const REPEATS: usize = 24;

/// The Japanese text between the punctuation marks. Four characters, so the run between candidates
/// is short enough that a narrow measure breaks often.
pub const JAPANESE_FILLER: &str = "文字組版";

/// The Latin control's filler, chosen the same way.
pub const LATIN_FILLER: &str = "typesetting";

/// # The measure, and why this file states no font and no column width
///
/// The document is a blank A4 one and says nothing about either. That is deliberate on both counts.
///
/// * **The font.** Word will fall back to whatever Japanese face the machine has, and *which face
///   it chose* is part of what the sitting records — a document that pinned `MS Mincho` would be
///   asking about a face the machine may not have rather than about what Word actually does.
/// * **The measure.** [`crate::ingest::read_hanging`] infers it from the export, as the **median**
///   line width of the paragraph itself. A measure written down here would be a prediction about
///   Word's page setup, and a prediction is exactly what a reader must not need.

/// What one paragraph of the document is for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HangingParagraph {
    /// Its index in the body.
    pub index: usize,
    /// What it asks.
    pub role: HangingRole,
}

/// Which of the three paragraphs this is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HangingRole {
    /// Japanese text, `w:overflowPunct` explicitly on. The case the question is about.
    JapaneseHanging,
    /// The same text, `w:overflowPunct` explicitly off. The control that says the setting is doing
    /// the work.
    JapaneseNotHanging,
    /// Latin text, `w:overflowPunct` explicitly on. The *"and a Latin paragraph in the same
    /// document?"* half.
    LatinHanging,
}

impl HangingRole {
    /// All three, so a sweep cannot miss one.
    pub const ALL: [Self; 3] = [
        Self::JapaneseHanging,
        Self::JapaneseNotHanging,
        Self::LatinHanging,
    ];

    /// Whether the paragraph states `w:overflowPunct` as on.
    #[must_use]
    pub fn overflow_punctuation(self) -> bool {
        !matches!(self, Self::JapaneseNotHanging)
    }

    /// The text it carries.
    #[must_use]
    pub fn text(self) -> String {
        let filler = match self {
            Self::LatinHanging => LATIN_FILLER,
            Self::JapaneseHanging | Self::JapaneseNotHanging => JAPANESE_FILLER,
        };
        let mut text = String::new();
        for _ in 0..REPEATS {
            for candidate in HANGABLE {
                text.push_str(filler);
                text.push(candidate);
            }
        }
        text
    }

    /// How the paragraph is described in the report and in the written instructions.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::JapaneseHanging => "Japanese, overflowPunct on",
            Self::JapaneseNotHanging => "Japanese, overflowPunct off",
            Self::LatinHanging => "Latin, overflowPunct on",
        }
    }
}

/// The three paragraphs, in body order.
#[must_use]
pub fn paragraphs() -> Vec<HangingParagraph> {
    HangingRole::ALL
        .into_iter()
        .enumerate()
        .map(|(index, role)| HangingParagraph { index, role })
        .collect()
}

/// Author the hanging-punctuation document.
///
/// # The three-pass shape, and the defect that makes the third pass necessary
///
/// `Document` exposes text editing and no `w:pPr` writer — deliberately, by `mjx-docx`'s own stated
/// precedent that *"a child adding typed properties does not also add a `Document`-level surface for
/// them"*. The typed [`mjx_docx::ParagraphProperties`] is public and is edited through the package,
/// which is exactly how that crate's own `tests/paragraph_properties.rs` exercises it. So this
/// authors the text with `Document`, saves, reopens the *package*, and sets the three paragraphs'
/// properties on the typed model.
///
/// **Then it saves the package and reopens it as a `Document`, rather than handing the package
/// straight to [`Document::from_package`], because that call refuses it.**
/// `from_package` probes for the main part with `Package::part_bytes`, and `part_bytes` answers
/// `None` for a part in the `Edited` state — which is exactly the state a part is in after
/// `part_tree_mut`. The failure is
/// [`DocxError::MissingDocumentPart`](mjx_docx::DocxError::MissingDocumentPart), naming a part that
/// is present and correct. `mjx_pptx::Presentation::from_package` probes the same way and so has the
/// same behaviour. Both constructors document themselves as taking *"one authored part by part"*,
/// which is the case that does not work; MJXOFF-207 found it here and reported it rather than
/// changing two format crates from inside a test-only one.
///
/// The extra round trip costs one serialisation and changes nothing about the file.
///
/// # Errors
///
/// Whatever the document layer fails with, and a sentence if the main part is missing or the body
/// has fewer paragraphs than were written into it — both of which would be defects here rather than
/// inputs.
pub fn hanging_document() -> Result<Vec<u8>, DocxError> {
    let mut document = Document::blank(PageSize::a4())?;
    let paragraphs = paragraphs();
    for (position, paragraph) in paragraphs.iter().enumerate() {
        if position > 0 {
            document.append_paragraph()?;
        }
        document.insert_run(position, 0, &paragraph.role.text())?;
    }
    let bytes = document.save()?;

    let mut package = Package::open(&bytes)?;
    let part = PartName::new(DOCUMENT_XML).map_err(mjx_docx::DocxError::from)?;
    {
        let raw = package.part_tree_mut(&part)?;
        let mut main = MainDocument::from_xml(&raw.root, &raw.interner)?;
        {
            let body = main
                .body_mut()
                .ok_or(DocxError::MalformedDocument("a blank document has a body"))?;
            for paragraph in &paragraphs {
                let element =
                    body.paragraph_mut(paragraph.index)
                        .ok_or(DocxError::MalformedDocument(
                            "every paragraph written above is in the body",
                        ))?;
                let properties = element.properties_or_insert(&mut raw.interner);
                properties.set_overflow_punctuation(
                    &mut raw.interner,
                    Some(paragraph.role.overflow_punctuation()),
                );
                // Kinsoku on, so the East Asian line-breaking rules the question is about are the
                // ones in effect. A document that said nothing would be asking Word what its
                // *default* is, which is a different question and one nobody asked.
                properties.set_east_asian_line_breaking_rules(&mut raw.interner, Some(true));
            }
        }
        raw.root = main.to_xml(&mut raw.interner);
    }
    // See this function's own documentation: `Document::from_package` refuses a package whose main
    // part is `Edited`, so the package is serialised and reopened instead.
    Document::open(&package.save()?)?.save()
}
