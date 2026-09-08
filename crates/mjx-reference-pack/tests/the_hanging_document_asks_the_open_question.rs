//! **The artefact probes exactly the set the engine acts on** — asked of `mjx-text`, one character
//! at a time, rather than assumed.
//!
//! # Why this link has to be a test and not a re-export
//!
//! `KinsokuRules` keeps its hangable set private behind [`KinsokuRules::hangs`], which answers about
//! one character. So `crate::hanging::HANGABLE` is a restatement, and a restatement is a second copy
//! of a table — the thing this repository refuses everywhere else.
//!
//! What makes it safe is that the copy is checked *by asking the original*, in both directions: every
//! character the artefact probes must hang, and every character in the whole probe alphabet that the
//! engine hangs must be probed. A set that drifted in either direction would fail here, and a
//! re-export could not have checked the second direction at all.
//!
//! # And the question the artefact exists to ask
//!
//! `mjx-text`'s own module documentation leaves it open by name: *"Should ASCII `,` and `.` be in
//! the Japanese hangable set at all?"* [`the_two_ascii_characters_in_question_are_on_the_sheet`] is
//! what makes sure the artefact does not quietly stop asking it.

use mjx_reference_pack::hanging::{hanging_document, paragraphs, HangingRole, HANGABLE, REPEATS};
use mjx_text::{KinsokuRules, LineBreakOptions};

#[test]
fn every_character_the_artefact_probes_is_one_the_engine_hangs() {
    let rules = KinsokuRules::japanese_standard();
    for candidate in HANGABLE {
        assert!(
            rules.hangs(candidate),
            "the artefact probes {candidate:?} and `KinsokuRules::japanese_standard` does not hang \
             it, so the sitting would answer a question nothing asked"
        );
    }
}

#[test]
fn every_character_the_engine_hangs_is_one_the_artefact_probes() {
    let rules = KinsokuRules::japanese_standard();
    // Over a set wide enough to catch an addition: the whole probe alphabet the type specimens use,
    // plus the ideographic punctuation the Japanese rules are about.
    let candidates: Vec<char> = (0x20u32..0x7fu32)
        .filter_map(char::from_u32)
        .chain("、。，．｡､〜～：；！？「」『』（）".chars())
        .collect();
    assert!(
        candidates.len() > 100,
        "the sweep is too narrow to catch anything"
    );

    let hangs: Vec<char> = candidates
        .into_iter()
        .filter(|candidate| rules.hangs(*candidate))
        .collect();
    for candidate in &hangs {
        assert!(
            HANGABLE.contains(candidate),
            "`KinsokuRules::japanese_standard` hangs {candidate:?} and the artefact does not probe \
             it, so the sitting would not answer for it"
        );
    }
    assert_eq!(
        hangs.len(),
        HANGABLE.len(),
        "the engine hangs {hangs:?} and the artefact probes {HANGABLE:?}"
    );
}

/// The two the whole item is about.
#[test]
fn the_two_ascii_characters_in_question_are_on_the_sheet() {
    for ascii in [',', '.'] {
        assert!(
            HANGABLE.contains(&ascii),
            "ASCII {ascii:?} is the character `mjx-text`'s own documentation leaves open, and the \
             artefact does not probe it"
        );
        assert!(
            KinsokuRules::japanese_standard().hangs(ascii),
            "the engine no longer hangs ASCII {ascii:?}, so the question has been answered \
             somewhere other than a Windows machine"
        );
        // And the *default* still does not turn the Japanese rules on, which is the half MJXOFF-160
        // settled and this artefact must not disturb.
        assert!(
            !LineBreakOptions::default().hanging_punctuation,
            "the default line-break options hang punctuation again; a caller who said nothing is \
             getting the Japanese answer for an English paragraph"
        );
    }
}

/// Every candidate appears in every paragraph, enough times that a line ends on it somewhere.
#[test]
fn every_candidate_appears_often_enough_to_land_at_a_line_end() {
    for paragraph in paragraphs() {
        let text = paragraph.role.text();
        for candidate in HANGABLE {
            let occurrences = text.matches(candidate).count();
            assert_eq!(
                occurrences,
                REPEATS,
                "`{}` carries {candidate:?} {occurrences} times; the construction needs {REPEATS}, \
                 because it deliberately predicts nothing about where Word will break",
                paragraph.role.label()
            );
        }
    }
    // The three paragraphs are two texts and three settings — the differential.
    let japanese = HangingRole::JapaneseHanging.text();
    assert_eq!(
        japanese,
        HangingRole::JapaneseNotHanging.text(),
        "the two Japanese paragraphs must be the same text, or the differential compares two things"
    );
    assert_ne!(
        japanese,
        HangingRole::LatinHanging.text(),
        "the Latin control must be Latin"
    );
    assert!(
        !HangingRole::JapaneseNotHanging.overflow_punctuation(),
        "there is no control if every paragraph has the setting on"
    );
}

/// And the whole of it survives into the file, rather than only into the model.
#[test]
fn the_authored_document_carries_every_candidate() {
    let bytes = hanging_document().expect("the document authors");
    let mut document = mjx_docx::Document::open(&bytes).expect("the document opens");
    for paragraph in paragraphs() {
        let text = document
            .paragraph_text(paragraph.index)
            .expect("the paragraph reads");
        for candidate in HANGABLE {
            assert_eq!(
                text.matches(candidate).count(),
                REPEATS,
                "`{}` lost {candidate:?} between the model and the file",
                paragraph.role.label()
            );
        }
    }
}
