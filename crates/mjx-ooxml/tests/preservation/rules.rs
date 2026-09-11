//! What a call is **allowed** to do, declared once per method and checked against every fixture.
//!
//! # Why the declaration is by class and not by part name
//!
//! The same method runs against every fixture of its format, and the part it touches is called
//! `slide1.xml` in one and `slide7.xml` in another. A declaration written in part names could only
//! be written per fixture — which is a golden file with fifty-four columns, unreadable and rewritten
//! whenever the corpus grows. A declaration written in **classes** is written once and is *stronger*
//! where it matters: `set_shape_text` may change exactly one slide, and it is the count that says
//! "one", not the name.
//!
//! # The two directions
//!
//! [`Touches::check`] is failed by both:
//!
//! * **Something happened that was not declared** — the direction that catches all three of the
//!   defects this suite locks down. An unconditional theme write changes a part of class
//!   `…theme+xml` that no method declares; a regenerated workbook removes parts of the producer's
//!   embedded package; a lost percent-decode removes an image.
//! * **Something declared did not happen.** An [`Count::Exactly`] rule that matched nothing means
//!   the call reported success and did not do what it says. Without this direction a method wired to
//!   nothing at all would pass every fixture.
//!
//! The second direction is suspended when the call changed *nothing*, which is tallied as its own
//! outcome (*no-op*) and counted separately, because "set this to what it already was" is a
//! legitimate success with an empty diff and asserting a change there would be false.

use crate::diff::{Change, PackageDiff};

/// How many parts of a class a rule covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Count {
    /// Exactly this many, no more and no fewer — the assertion that gives a declaration its teeth.
    Exactly(usize),
    /// Up to this many. For a part whose presence depends on the fixture: a chart that draws from a
    /// live range carries no embedded workbook, so the workbook rule is satisfied by zero.
    UpTo(usize),
    /// Any number, including none. Only for a method whose whole contract is "as many as there
    /// are", and every use carries a comment saying which.
    Any,
}

impl Count {
    /// The most parts this rule may cover, or `None` for unbounded.
    const fn capacity(self) -> Option<usize> {
        match self {
            Self::Exactly(n) | Self::UpTo(n) => Some(n),
            Self::Any => None,
        }
    }
}

/// One clause of a declaration: this many parts of this class may be added, changed or removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rule {
    /// Which side of the comparison.
    pub(crate) change: Change,
    /// The class, matched against [`crate::diff::DiffEntry::class`]. A trailing `*` matches by
    /// prefix, which [`ANY_CLASS`] and [`ANY_EMBEDDED_CLASS`] are the only two uses of.
    pub(crate) class: &'static str,
    /// How many.
    pub(crate) count: Count,
}

/// Everything one method may do to a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Touches {
    /// The clauses. A difference matching none of them fails.
    pub(crate) rules: &'static [Rule],
}

/// The wildcard class. Only a method whose contract really is "whatever is there" may name it, and
/// every use carries the sentence saying which.
pub(crate) const ANY_CLASS: &str = "*";

/// Any part **inside** an embedded package. Named only by the methods that create such a package:
/// a chart's workbook arrives whole, so its own sheets, styles and theme arrive with it, and
/// enumerating them clause by clause would state the writer's current layout rather than the rule.
/// It is an `added` clause everywhere it appears — a *data edit* never gets it, which is what keeps
/// MJXOFF-208 caught.
pub(crate) const ANY_EMBEDDED_CLASS: &str = "embedded:*";

/// A reader: the package must come back part for part identical.
///
/// This is not a weaker assertion than a mutator's — it is the strongest one in the file. Every part
/// of this library is lazily parsed, so a reader materializes the model of whatever part it reads,
/// and a reader that left that part *dirty* would re-serialize it on save. `mjx-pptx`'s
/// `reading_does_not_dirty_parts` states this for one method on one fixture; this states it for
/// every reader of the facade on every fixture of its format.
pub(crate) const NOTHING: Touches = Touches { rules: &[] };

/// A rule for added parts.
pub(crate) const fn added(class: &'static str, count: Count) -> Rule {
    Rule {
        change: Change::Added,
        class,
        count,
    }
}

/// A rule for changed parts.
pub(crate) const fn changed(class: &'static str, count: Count) -> Rule {
    Rule {
        change: Change::Changed,
        class,
        count,
    }
}

/// A rule for removed parts.
pub(crate) const fn removed(class: &'static str, count: Count) -> Rule {
    Rule {
        change: Change::Removed,
        class,
        count,
    }
}

/// Exactly one part of this class changes. The commonest clause there is.
pub(crate) const fn changed_one(class: &'static str) -> Rule {
    changed(class, Count::Exactly(1))
}

/// Up to one part of this class changes.
pub(crate) const fn changed_up_to_one(class: &'static str) -> Rule {
    changed(class, Count::UpTo(1))
}

/// Whether a rule's class covers a difference's. A trailing `*` matches by prefix.
fn class_matches(rule: &str, observed: &str) -> bool {
    match rule.strip_suffix('*') {
        Some(prefix) => observed.starts_with(prefix),
        None => rule == observed,
    }
}

impl Touches {
    /// Whether this declares any effect at all — the test that tells a mutator from a reader, and
    /// therefore which anti-vacuity floor a case is held to.
    pub(crate) const fn is_reader(&self) -> bool {
        self.rules.is_empty()
    }

    /// Every way `diff` disagrees with this declaration, one message per disagreement.
    ///
    /// `enforce_minimums` is false for a call that changed nothing: see the module doc.
    pub(crate) fn check(&self, diff: &PackageDiff, enforce_minimums: bool) -> Vec<String> {
        let mut violations = Vec::new();
        let mut matched = vec![0usize; self.rules.len()];

        'entries: for entry in &diff.entries {
            for (idx, rule) in self.rules.iter().enumerate() {
                if rule.change != entry.change {
                    continue;
                }
                if !class_matches(rule.class, &entry.class) {
                    continue;
                }
                if rule.count.capacity().is_some_and(|cap| matched[idx] >= cap) {
                    continue;
                }
                matched[idx] += 1;
                continue 'entries;
            }
            violations.push(format!("undeclared: {entry}"));
        }

        if enforce_minimums {
            for (idx, rule) in self.rules.iter().enumerate() {
                if let Count::Exactly(wanted) = rule.count {
                    if matched[idx] != wanted {
                        violations.push(format!(
                            "declared {} {} × {wanted}, saw {}",
                            rule.change.verb(),
                            rule.class,
                            matched[idx]
                        ));
                    }
                }
            }
        }
        violations
    }
}

// =================================================================================================
// The classes a declaration names. Every one is an ECMA-376 content type, spelled here once so a
// declaration reads as a sentence rather than as a URL.
// =================================================================================================

/// `.pptx` — one slide.
pub(crate) const SLIDE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.slide+xml";
/// `.pptx` — the presentation part.
pub(crate) const PRESENTATION: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml";
/// `.pptx` — a slide layout.
pub(crate) const SLIDE_LAYOUT: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml";
/// `.pptx` — a notes slide.
pub(crate) const NOTES_SLIDE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml";
/// `.pptx` — the table-style definitions part.
pub(crate) const TABLE_STYLES: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml";
/// `.docx` — the main document part.
pub(crate) const WORD_DOCUMENT: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
/// `.docx` — the comments part.
pub(crate) const WORD_COMMENTS: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml";
/// `.docx` — the footnotes part.
pub(crate) const WORD_FOOTNOTES: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml";
/// `.docx` — the endnotes part.
pub(crate) const WORD_ENDNOTES: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.endnotes+xml";
/// `.docx` — the settings part.
pub(crate) const WORD_SETTINGS: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml";
/// `.docx` — a header.
pub(crate) const WORD_HEADER: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml";
/// `.docx` — a footer.
pub(crate) const WORD_FOOTER: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml";
/// `.docx` — the numbering definitions.
pub(crate) const WORD_NUMBERING: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml";
/// `.xlsx` — the workbook part.
pub(crate) const WORKBOOK: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";
/// `.xlsx` — one worksheet.
pub(crate) const WORKSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";
/// `.xlsx` — the shared string table.
pub(crate) const SHARED_STRINGS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml";
/// `.xlsx` — the style table.
pub(crate) const SHEET_STYLES: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml";
/// `.xlsx` — a cell-comment part.
pub(crate) const SHEET_COMMENTS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml";
/// `.xlsx` — the calculation chain, which Excel rebuilds and an edit may invalidate.
pub(crate) const CALC_CHAIN: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.calcChain+xml";
/// A DrawingML chart part, in all three formats.
pub(crate) const CHART: &str = "application/vnd.openxmlformats-officedocument.drawingml.chart+xml";
/// A DrawingML drawing part — the `.xlsx` anchor host.
pub(crate) const DRAWING: &str = "application/vnd.openxmlformats-officedocument.drawing+xml";
/// A theme part, in all three formats.
pub(crate) const THEME: &str = "application/vnd.openxmlformats-officedocument.theme+xml";
/// A VML drawing — the legacy fallback surface.
pub(crate) const VML_DRAWING: &str = "application/vnd.openxmlformats-officedocument.vmlDrawing";
/// A PNG image.
pub(crate) const PNG: &str = "image/png";
/// A JPEG image.
pub(crate) const JPEG: &str = "image/jpeg";
/// An InkML part.
pub(crate) const INKML: &str = "application/inkml+xml";
/// An OLE object's stream.
pub(crate) const OLE_OBJECT: &str = "application/vnd.openxmlformats-officedocument.oleObject";
/// An ActiveX control's XML half.
pub(crate) const ACTIVEX: &str = "application/vnd.ms-office.activeX+xml";
/// An ActiveX control's binary half.
pub(crate) const ACTIVEX_BINARY: &str = "application/vnd.ms-office.activeX";
/// A chart's embedded workbook, as one part of the host package.
pub(crate) const EMBEDDED_WORKBOOK: &str = crate::diff::EMBEDDED_PACKAGE;
/// The `[Content_Types].xml` stream.
pub(crate) const CONTENT_TYPES: &str = crate::diff::CONTENT_TYPES;
/// A `.rels` stream.
pub(crate) const RELATIONSHIPS: &str = crate::diff::RELATIONSHIPS;

/// A worksheet **inside** a chart's embedded workbook — the part a data edit patches.
pub(crate) const EMBEDDED_WORKSHEET: &str = concat!(
    "embedded:",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"
);
/// The shared string table inside a chart's embedded workbook.
pub(crate) const EMBEDDED_SHARED_STRINGS: &str = concat!(
    "embedded:",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"
);
/// The workbook part inside a chart's embedded workbook.
pub(crate) const EMBEDDED_WORKBOOK_MAIN: &str = concat!(
    "embedded:",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"
);
/// The `[Content_Types].xml` of a chart's embedded workbook.
pub(crate) const EMBEDDED_CONTENT_TYPES: &str = concat!("embedded:", "«content types»");
/// A `.rels` stream inside a chart's embedded workbook.
pub(crate) const EMBEDDED_RELATIONSHIPS: &str = concat!("embedded:", "«relationships»");
/// The style table inside a chart's embedded workbook.
pub(crate) const EMBEDDED_SHEET_STYLES: &str = concat!(
    "embedded:",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"
);
