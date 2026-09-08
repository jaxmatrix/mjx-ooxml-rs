//! **The reference pack: what one Windows sitting needs, prepared in advance** (MJXOFF-207).
//!
//! # ⚠ Read this before quoting anything this crate prints
//!
//! **Every gate in this crate can pass with no authoritative reference in existence.** That is not a
//! weakness to be fixed, it is the shape of the problem: the reference is the one thing an agent
//! cannot produce. So the claims are written down here, in the code, rather than left to be inferred
//! from a green run.
//!
//! **What this crate can prove today, with no Office export anywhere:**
//!
//! * The four artefacts generate, byte-for-byte reproducibly, and are packages a real Office
//!   implementation opens.
//! * The plate table, the probe table and the hatch table are complete and self-consistent — 187
//!   presets, 92 probes per family, 54 hatches — and each names the cases it cannot check.
//! * The whole ingest pipeline runs end to end: author → export → rasterise → crop → compare →
//!   report, over a stand-in PDF exported by **our own** painter. `tests/` says so in its own file
//!   name: *the plumbing, not the fidelity.*
//! * LibreOffice renders all four artefacts and every plate gets a row.
//!
//! **What it cannot prove, and will not claim:**
//!
//! * That any shape matches PowerPoint. Not one. [`Baseline::may_be_called_parity`] returns `false`
//!   for every observation this crate can make today, and `parity_count` over a LibreOffice run is
//!   **zero by construction**, which `tests/an_excluded_result_is_not_evidence.rs` asserts rather
//!   than assumes.
//! * That Cambria's metrics, the Arial/Times/Courier advances, the Japanese hanging set or the ten
//!   pictorial hatch masks are right. It can only *ask* those questions mechanically.
//!
//! **A green LibreOffice comparison means "nothing obviously broke". It never means parity.** If a
//! sentence anywhere in this repository comes to say otherwise, the sentence is wrong.
//!
//! # The seven items, one morning
//!
//! [`THE_SITTING`] is the list, in code, and `docs/validation/07-the-reference-pack.md` is the same
//! list in prose addressed to a person. `tests/the_instructions_are_complete.rs` holds the two in
//! step: an item that gains a key here and not a paragraph there fails, and so does an artefact that
//! is renamed in one place only.
//!
//! # The two stages, and they are not equal
//!
//! **The preliminary test is run with LibreOffice. Confirmation is done against Microsoft Office.**
//! That is a sequence, and it decides what a passing preliminary run entitles anyone to say. See
//! [`authority`] for how the three-state result and the provider-attached exclusions enforce it.
//!
//! # What is here
//!
//! | Module | What it is |
//! |---|---|
//! | [`authority`] | Where a reference came from, what it is worth, and the third answer that is neither pass nor fail — `mjx-render-oracle`'s, re-exported |
//! | [`layout`] | The plate grid, stated once, so the deck and our own display list cannot drift |
//! | [`plates`] | What the 187 plates are, at their defaults and at their extremes |
//! | [`deck`] | Authoring the two preset decks |
//! | [`typography`] | The type specimens whose word boxes are the measurement, and the 54 hatches |
//! | [`hanging`] | The one artefact that is a `.docx`, because `w:overflowPunct` is a Word setting |
//! | [`ingest`] | Reading the answers back out of an exported PDF — and refusing a number it could not see |
//! | [`scene`] | Our own side: the same page as a display list, exported through the PDF painter |
//! | [`tools`] | `pdftoppm`, `pdftotext`, `pdfinfo`, `soffice` — and the loud named skip — `mjx-render-oracle`'s, re-exported |
//! | [`compare`] | Two rasters of one page, one plate at a time |
//! | [`pack`] | Generating the whole pack, and the preliminary pass over it |

// `authority` and `tools` live in `mjx-render-oracle` since MJXOFF-165 gave this crate a crate
// below it, and are re-exported here so that `mjx_reference_pack::authority::...` still names them.
// **They were moved rather than copied**, for the reason `authority`'s own documentation gives about
// a second authority enumeration: a workspace with two answers to *"how much is this reference
// worth"* has one too many, and the fidelity oracle needs the same answer this pack does.
pub use mjx_render_oracle::authority;
pub use mjx_render_oracle::tools;

pub mod compare;
pub mod deck;
pub mod hanging;
pub mod ingest;
pub mod layout;
pub mod pack;
pub mod plates;
pub mod scene;
pub mod typography;

pub use mjx_render_oracle::authority::{
    parity_count, provisional_baselines, Baseline, ReferenceProvider, RenderedContent, Verdict,
};
pub use plates::{Plate, PlateKind, PresetDeck};

/// One question the Windows sitting answers.
///
/// It is a record rather than a sentence because two things have to be checkable about it: that the
/// pack carries an artefact which asks it, and that the written instructions tell a person what to
/// do with that artefact. Both are asserted, and an item that gained neither would otherwise sit in
/// a list looking answered.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SittingItem {
    /// A stable key, used in the report and required to appear in the written instructions.
    pub key: &'static str,
    /// What is being asked.
    pub question: &'static str,
    /// The artefact that asks it.
    pub artefact: &'static str,
    /// How the answer is read out of what comes back.
    pub how_it_is_read: &'static str,
}

/// **The seven items, and the whole reason this crate exists.**
///
/// A person with Office in front of them is expensive to arrange, so the sitting happens *once* and
/// covers everything at once. Each row is a question a machine can ask and cannot answer.
pub const THE_SITTING: &[SittingItem] = &[
    SittingItem {
        key: "preset-geometry-defaults",
        question: "Does every one of the 187 preset shapes draw, at its default adjustments, the \
                   outline our generated path table resolves?",
        artefact: "01-presets-at-their-defaults.pptx",
        how_it_is_read: "one raster crop per plate, both sides rasterised by pdftoppm at one DPI",
    },
    SittingItem {
        key: "preset-geometry-extremes",
        question: "And at an end of every one of its handles, which is the case a default is most \
                   likely to be right by accident at?",
        artefact: "02-presets-at-their-extremes.pptx",
        how_it_is_read: "the same crop, against the same table resolved at the same adjustments",
    },
    SittingItem {
        key: "up-arrow",
        question: "What does `upArrow` draw? `presetShapeDefinitions.xml` publishes no geometry for \
                   it, so this is the one plate for which an Office export is the *only* possible \
                   source of truth.",
        artefact: "01-presets-at-their-defaults.pptx",
        how_it_is_read: "its plate is on the sheet, captioned `no published geometry`; the raster \
                         crop is the answer and there is nothing on our side to compare it against",
    },
    SittingItem {
        key: "cambria-metrics",
        question: "What are Cambria's advance widths and line pitch? Microsoft publishes no width \
                   table and no metric collection surveyed carries it, so `mjx-text`'s entry is \
                   `Unverified` and holds no numbers at all.",
        artefact: "03-type-specimens-and-hatches.pptx",
        how_it_is_read: "two word boxes per character — one copy and ten, bracketed by `H` — whose \
                         spans differ by exactly nine advances; and six lines at 60 pt for the pitch",
    },
    SittingItem {
        key: "clone-transcribed-advances",
        question: "Are Arial's, Times New Roman's and Courier New's advances the ones Microsoft's \
                   faces actually carry? The table's numbers come from (URW)++ Nimbus **clones** \
                   and are labelled `Published`, which is a stronger word than the source supports.",
        artefact: "03-type-specimens-and-hatches.pptx",
        how_it_is_read: "the same advance ruler, over the same 92 characters the table has room for",
    },
    SittingItem {
        key: "japanese-hanging-set",
        question: "Does Word hang ASCII `,` and `.` in a Japanese paragraph with `w:overflowPunct` \
                   on, and does it hang them in a Latin paragraph in the same document?",
        artefact: "04-hanging-punctuation.docx",
        how_it_is_read: "the last word box of every line, against the paragraph's own measure: a \
                         character that hangs has a box crossing the right margin",
    },
    SittingItem {
        key: "pictorial-hatches",
        question: "What do the ten pictorial preset hatches actually look like? ECMA-376 gives a \
                   bitmap for none of the fifty-four, and `mjx_paint::PATTERN_MASKS` derives \
                   forty-four from their names and draws these ten by hand.",
        artefact: "03-type-specimens-and-hatches.pptx",
        how_it_is_read: "one swatch per preset at 100x70 points, rasterised at 96 dpi and reduced \
                         to its 8x8 tile",
    },
];

/// Every artefact the pack contains, in the order a person works through them.
pub const ARTEFACTS: &[&str] = &[
    "01-presets-at-their-defaults.pptx",
    "02-presets-at-their-extremes.pptx",
    "03-type-specimens-and-hatches.pptx",
    "04-hanging-punctuation.docx",
];

/// Where the exported PDFs are put when they come back from the Windows machine.
///
/// **Beside `tests/office-authored/` and deliberately not inside it.** The two hold different things
/// and the difference is worth the extra directory: that one holds Office-*authored originals* — a
/// `.pptx` whose markup nobody here chose — while this holds Office's *rendering* of files this
/// project authored. `xtask`'s corpus walker also refuses a subdirectory outright, with a message
/// that says exactly why: *"the corpus is one flat directory of Office-authored packages; a
/// subdirectory has no meaning here and would sit outside every check."* It is right, and this
/// directory is outside its checks.
///
/// **The rule the two share is the one that matters:** the value of an Office-produced file is
/// entirely its provenance, so no agent may fill either.
pub const OFFICE_EXPORT_DIRECTORY: &str = "tests/office-exports";

/// The environment variable that turns an empty [`OFFICE_EXPORT_DIRECTORY`] into a failure.
///
/// MJX-ESCAPE-UNSET: deliberately **not** set anywhere today, for the reason `main`'s
/// `MJX_REQUIRE_OFFICE_CORPUS` is not: the directory is empty, that is the honest state of the
/// project, and setting the variable before a person has run Office would only make the build red
/// about something no build can fix. (The marker is what `xtask/tests/escape_hatches.rs` reads:
/// every escape in this workspace must be either bound by a workflow or explained here, and an
/// escape that is neither is a suite reporting coverage it does not have — MJXOFF-197.)
pub const REQUIRE_OFFICE_EXPORTS: &str = "MJX_REQUIRE_OFFICE_EXPORTS";
