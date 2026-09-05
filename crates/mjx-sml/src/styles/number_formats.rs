//! `x:numFmts` (`CT_NumFmts`, `sml.xsd:3576`) and the **implied** format codes of ECMA-376 Part 1
//! §18.8.30 — the two halves of answering "what format code does `numFmtId="14"` mean?".
//!
//! # A number format is addressed by an id, not by a position
//!
//! This is the one table in `styles.xml` that is **not** an array. A font, a fill, a border and a
//! `dxf` are each addressed by their position in their table ([`super::fonts`] says why that makes
//! reordering a corruption); a number format is addressed by its own `@numFmtId`, which the element
//! carries. So `<numFmt numFmtId="164" …/>` is entry 164 wherever in the part it sits, two entries
//! may be written in any order, and there is no relationship at all between an entry's position and
//! its id.
//!
//! That difference is why [`NumberFormatTable::get`] takes an **id** and
//! [`FontTable::get`](super::fonts::FontTable::get) takes an index, and why this module is not a
//! copy of that one.
//!
//! # Most ids are never written down
//!
//! §18.8.30: *"Following is a listing of number formats whose `formatCode` value is implied rather
//! than explicitly saved in the file. In this case, a `numFmtId` value is written on the `xf`
//! record, but no corresponding `numFmt` element is written."* So a workbook whose every cell is
//! `General` has **no `numFmts` element at all**, and resolving `numFmtId="0"` means reaching for a
//! table that lives in the specification rather than in the file.
//!
//! [`builtin_format_code`] is that table for the ids §18.8.30 lists under *All Languages*.
//! [`builtin_format_code_in`] is it for the ids whose code depends on the consumer's UI language —
//! **and those are a genuinely different answer per language, not a fallback**, which is why they
//! are a separate function taking a [`NumberFormatLanguage`] rather than rows of the same table.
//!
//! # Three things the specification says that a tidy implementation gets wrong
//!
//! * **The all-languages set is not `0..=49`.** It is twenty-eight specific ids, and §18.8.30 says
//!   so in as many words: *"Ids not specified in the listing, such as 5, 6, 7, and 8, shall follow
//!   the number format specified by the `formatCode` attribute."* 5–8, 23–26 and 41–44 are **not**
//!   built in; an implementation that filled the gaps would invent format codes.
//! * **The locale-dependent ids run past 49** — 27–36, 50–58 for `zh-tw`, `zh-cn`, `ja-jp` and
//!   `ko-kr`, and 59–62, 67–81 for `th-th`.
//! * **Ids 37 and 38 carry a space before their semicolon** (`#,##0 ;(#,##0)`) while 39 and 40 do
//!   not (`#,##0.00;(#,##0.00)`). That asymmetry is in the published table, it is not a
//!   transcription slip here, and "tidying" it is exactly the normalisation
//!   [`NumberFormat::format_code`](super::cell_format::NumberFormat::format_code) exists to refuse.
//!
//! # This module does not *apply* a format code
//!
//! Rendering `0.00` against `3.14159` is a programme non-goal, restated here because this is the
//! module a reader looking for it would open. The resolver reports the code that is in force and
//! stops; see [`super::effective`].

use mjx_ooxml_core::{Interner, Number, RawAttribute, RawName, RawNode};

use super::cell_format::NumberFormat;

/// `x:numFmts` (`CT_NumFmts`, `sml.xsd:3576`) — the custom number formats a workbook writes down.
///
/// Keyed by `numFmt@numFmtId`, **not** by position: see the [module documentation](self).
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count))]
pub struct NumberFormatTable {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "numFmt", variant = Format, ty = NumberFormat))]
    content: Vec<NumberFormatTableContent>,
}

/// One child of [`NumberFormatTable`]: a number format, or markup this type does not model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberFormatTableContent {
    /// `x:numFmt`.
    Format(NumberFormat),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl NumberFormatTable {
    /// Builds an empty `x:numFmts`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "numFmts"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[NumberFormatTableContent] {
        &self.content
    }

    /// Every `x:numFmt`, in document order — which is **not** id order and need not be.
    pub fn formats(&self) -> impl Iterator<Item = &NumberFormat> + '_ {
        self.content.iter().filter_map(|item| match item {
            NumberFormatTableContent::Format(format) => Some(format),
            NumberFormatTableContent::Raw(_) => None,
        })
    }

    /// The entry whose `@numFmtId` is `id`, or `None` when the part declares none.
    ///
    /// `None` does **not** mean the id is meaningless: most ids in use are the implied ones of
    /// [`builtin_format_code`], which no file writes down.
    ///
    /// A part that declares the same id twice is malformed; the **first** entry wins, so that the
    /// answer does not depend on which branch of a search happens to run.
    #[must_use]
    pub fn get(&self, interner: &Interner, id: u32) -> Option<&NumberFormat> {
        self.formats()
            .find(|format| format.number_format_id(interner).ok().flatten() == Some(id))
    }

    /// How many `x:numFmt` entries the table holds — counted, not read from `@count`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.formats().count()
    }

    /// Whether the table holds no entry at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends `format` after the last entry, and updates `@count` when the file declared one.
    ///
    /// Appending is safe here for a different reason than it is in [`super::fonts`]: an entry's id
    /// is written on the entry, so nothing renumbers. What a caller must still not do is append an
    /// entry whose id an existing entry already carries.
    pub fn push(&mut self, interner: &mut Interner, format: NumberFormat) {
        self.content.push(NumberFormatTableContent::Format(format));
        self.empty = false;
        if self.declared_count(interner).ok().flatten().is_some() {
            let count = u32::try_from(self.len()).unwrap_or(u32::MAX);
            self.set_declared_count(interner, Some(count));
        }
    }
}

/// The UI languages ECMA-376 Part 1 §18.8.30 gives a *different* set of implied format codes for.
///
/// These are not translations of one table. Id 30 is `m/d/yy` in `zh-tw` and `mm-dd-yy` in `ko-kr`;
/// id 34 is a date in `ja-jp` and a time in `zh-cn`. So a consumer that does not know the UI
/// language cannot answer for these ids at all, which is why [`builtin_format_code`] returns `None`
/// for every one of them rather than picking a language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumberFormatLanguage {
    /// Traditional Chinese, Taiwan — the `zh-tw` column of §18.8.30.
    ChineseTaiwan,
    /// Simplified Chinese, PRC — the `zh-cn` column of §18.8.30.
    ChineseChina,
    /// Japanese — the `ja-jp` column of §18.8.30.
    Japanese,
    /// Korean — the `ko-kr` column of §18.8.30.
    Korean,
    /// Thai — the `th-th` table of §18.8.30.
    Thai,
}

/// The format code ECMA-376 Part 1 §18.8.30 implies for `id` in **every** UI language.
///
/// `None` for an id that is either locale-dependent — ask [`builtin_format_code_in`] instead, and
/// see [`is_locale_dependent`] — or not built in at all, in which case the workbook has to declare
/// it in its `numFmts` and a dangling id is the file's own error.
///
/// The twenty-eight ids listed under *All Languages*, transcribed character for character:
///
/// | id | code | id | code | id | code |
/// |---|---|---|---|---|---|
/// | 0 | `General` | 14 | `mm-dd-yy` | 22 | `m/d/yy h:mm` |
/// | 1 | `0` | 15 | `d-mmm-yy` | 37 | `#,##0 ;(#,##0)` |
/// | 2 | `0.00` | 16 | `d-mmm` | 38 | `#,##0 ;[Red](#,##0)` |
/// | 3 | `#,##0` | 17 | `mmm-yy` | 39 | `#,##0.00;(#,##0.00)` |
/// | 4 | `#,##0.00` | 18 | `h:mm AM/PM` | 40 | `#,##0.00;[Red](#,##0.00)` |
/// | 9 | `0%` | 19 | `h:mm:ss AM/PM` | 45 | `mm:ss` |
/// | 10 | `0.00%` | 20 | `h:mm` | 46 | `[h]:mm:ss` |
/// | 11 | `0.00E+00` | 21 | `h:mm:ss` | 47 | `mmss.0` |
/// | 12 | `# ?/?` | | | 48 | `##0.0E+0` |
/// | 13 | `# ??/??` | | | 49 | `@` |
///
/// The space before the semicolon in 37 and 38, and its absence in 39 and 40, is the published
/// table's; see the [module documentation](self).
#[must_use]
pub const fn builtin_format_code(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "General",
        1 => "0",
        2 => "0.00",
        3 => "#,##0",
        4 => "#,##0.00",
        9 => "0%",
        10 => "0.00%",
        11 => "0.00E+00",
        12 => "# ?/?",
        13 => "# ??/??",
        14 => "mm-dd-yy",
        15 => "d-mmm-yy",
        16 => "d-mmm",
        17 => "mmm-yy",
        18 => "h:mm AM/PM",
        19 => "h:mm:ss AM/PM",
        20 => "h:mm",
        21 => "h:mm:ss",
        22 => "m/d/yy h:mm",
        37 => "#,##0 ;(#,##0)",
        38 => "#,##0 ;[Red](#,##0)",
        39 => "#,##0.00;(#,##0.00)",
        40 => "#,##0.00;[Red](#,##0.00)",
        45 => "mm:ss",
        46 => "[h]:mm:ss",
        47 => "mmss.0",
        48 => "##0.0E+0",
        49 => "@",
        _ => return None,
    })
}

/// The format code ECMA-376 Part 1 §18.8.30 implies for `id` when the consumer's UI language is
/// `language`.
///
/// Answers for the locale-dependent ids **and** for the all-languages ones, because a consumer that
/// knows its language wants one lookup rather than two. `None` means `id` is built in under neither
/// table and the workbook has to declare it.
#[must_use]
pub const fn builtin_format_code_in(
    id: u32,
    language: NumberFormatLanguage,
) -> Option<&'static str> {
    if let Some(code) = builtin_format_code(id) {
        return Some(code);
    }
    match language {
        NumberFormatLanguage::ChineseTaiwan => chinese_taiwan_format_code(id),
        NumberFormatLanguage::ChineseChina => chinese_china_format_code(id),
        NumberFormatLanguage::Japanese => japanese_format_code(id),
        NumberFormatLanguage::Korean => korean_format_code(id),
        NumberFormatLanguage::Thai => thai_format_code(id),
    }
}

/// Whether §18.8.30 gives `id` a format code that depends on the consumer's UI language.
///
/// True for 27–36 and 50–58 (the CJK block) and for 59–62 and 67–81 (the Thai block). An id this
/// answers `true` for cannot be resolved without a language, which is the honest answer and not a
/// gap.
#[must_use]
pub const fn is_locale_dependent(id: u32) -> bool {
    matches!(id, 27..=36 | 50..=58 | 59..=62 | 67..=81)
}

/// The `zh-tw` column of §18.8.30.
const fn chinese_taiwan_format_code(id: u32) -> Option<&'static str> {
    Some(match id {
        27 | 36 | 50 | 57 => "[$-404]e/m/d",
        28 | 29 | 51 | 54 | 58 => "[$-404]e\u{5e74}\"\u{6708}\"d\"\u{65e5}\"",
        30 => "m/d/yy",
        31 => "yyyy\"\u{5e74}\"m\"\u{6708}\"d\"\u{65e5}\"",
        32 => "hh\"\u{6642}\"mm\"\u{5206}\"",
        33 => "hh\"\u{6642}\"mm\"\u{5206}\"ss\"\u{79d2}\"",
        34 | 52 | 55 => "\u{4e0a}\u{5348}/\u{4e0b}\u{5348} hh\"\u{6642}\"mm\"\u{5206}\"",
        35 | 53 | 56 => {
            "\u{4e0a}\u{5348}/\u{4e0b}\u{5348} hh\"\u{6642}\"mm\"\u{5206}\"ss\"\u{79d2}\""
        }
        _ => return None,
    })
}

/// The `zh-cn` column of §18.8.30.
const fn chinese_china_format_code(id: u32) -> Option<&'static str> {
    Some(match id {
        27 | 36 | 50 | 52 | 57 => "yyyy\"\u{5e74}\"m\"\u{6708}\"",
        28 | 29 | 51 | 53 | 54 | 58 => "m\"\u{6708}\"d\"\u{65e5}\"",
        30 => "m-d-yy",
        31 => "yyyy\"\u{5e74}\"m\"\u{6708}\"d\"\u{65e5}\"",
        32 => "h\"\u{65f6}\"mm\"\u{5206}\"",
        33 => "h\"\u{65f6}\"mm\"\u{5206}\"ss\"\u{79d2}\"",
        34 | 55 => "\u{4e0a}\u{5348}/\u{4e0b}\u{5348} h\"\u{65f6}\"mm\"\u{5206}\"",
        35 | 56 => "\u{4e0a}\u{5348}/\u{4e0b}\u{5348} h\"\u{65f6}\"mm\"\u{5206}\"ss\"\u{79d2}\"",
        _ => return None,
    })
}

/// The `ja-jp` column of §18.8.30.
const fn japanese_format_code(id: u32) -> Option<&'static str> {
    Some(match id {
        27 | 36 | 50 | 57 => "[$-411]ge.m.d",
        28 | 29 | 51 | 54 | 58 => "[$-411]ggge\"\u{5e74}\"m\"\u{6708}\"d\"\u{65e5}\"",
        30 => "m/d/yy",
        31 => "yyyy\"\u{5e74}\"m\"\u{6708}\"d\"\u{65e5}\"",
        32 => "h\"\u{6642}\"mm\"\u{5206}\"",
        33 => "h\"\u{6642}\"mm\"\u{5206}\"ss\"\u{79d2}\"",
        34 | 52 | 55 => "yyyy\"\u{5e74}\"m\"\u{6708}\"",
        35 | 53 | 56 => "m\"\u{6708}\"d\"\u{65e5}\"",
        _ => return None,
    })
}

/// The `ko-kr` column of §18.8.30.
const fn korean_format_code(id: u32) -> Option<&'static str> {
    Some(match id {
        27 | 36 | 50 | 57 => "yyyy\"\u{5e74}\" mm\"\u{6708}\" dd\"\u{65e5}\"",
        28 | 29 | 51 | 54 | 58 => "mm-dd",
        30 => "mm-dd-yy",
        31 => "yyyy\"\u{b144}\" mm\"\u{c6d4}\" dd\"\u{c77c}\"",
        32 => "h\"\u{c2dc}\" mm\"\u{bd84}\"",
        33 => "h\"\u{c2dc}\" mm\"\u{bd84}\" ss\"\u{cd08}\"",
        34 | 35 | 52 | 53 | 55 | 56 => "yyyy-mm-dd",
        _ => return None,
    })
}

/// The `th-th` table of §18.8.30.
const fn thai_format_code(id: u32) -> Option<&'static str> {
    Some(match id {
        59 => "t0",
        60 => "t0.00",
        61 => "t#,##0",
        62 => "t#,##0.00",
        67 => "t0%",
        68 => "t0.00%",
        69 => "t# ?/?",
        70 => "t# ??/??",
        71 => "\u{0e27}/\u{0e14}/\u{0e1b}\u{0e1b}\u{0e1b}\u{0e1b}",
        72 => "\u{0e27}-\u{0e14}\u{0e14}\u{0e14}-\u{0e1b}\u{0e1b}",
        73 => "\u{0e27}-\u{0e14}\u{0e14}\u{0e14}",
        74 => "\u{0e14}\u{0e14}\u{0e14}-\u{0e1b}\u{0e1b}",
        75 => "\u{0e0a}:\u{0e19}\u{0e19}",
        76 => "\u{0e0a}:\u{0e19}\u{0e19}:\u{0e17}\u{0e17}",
        77 => "\u{0e27}/\u{0e14}/\u{0e1b}\u{0e1b}\u{0e1b}\u{0e1b} \u{0e0a}:\u{0e19}\u{0e19}",
        78 => "\u{0e19}\u{0e19}:\u{0e17}\u{0e17}",
        79 => "[\u{0e0a}]:\u{0e19}\u{0e19}:\u{0e17}\u{0e17}",
        80 => "\u{0e19}\u{0e19}:\u{0e17}\u{0e17}.0",
        81 => "d/m/bb",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gaps §18.8.30 names explicitly are gaps here too.
    ///
    /// Written against the tidy mistake: `0..=49` looks like a range, and filling 5–8, 23–26 and
    /// 41–44 with something plausible would invent format codes the specification refuses to give.
    #[test]
    fn the_all_languages_table_has_the_holes_the_specification_has() {
        for id in [5, 6, 7, 8, 23, 24, 25, 26, 41, 42, 43, 44] {
            assert_eq!(
                builtin_format_code(id),
                None,
                "§18.8.30: \"Ids not specified in the listing, such as 5, 6, 7, and 8, shall follow \
                 the number format specified by the formatCode attribute\" — {id} is not built in"
            );
            assert!(!is_locale_dependent(id));
        }
        let listed = [
            0, 1, 2, 3, 4, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 37, 38, 39, 40,
            45, 46, 47, 48, 49,
        ];
        assert_eq!(listed.len(), 28);
        for id in listed {
            assert!(
                builtin_format_code(id).is_some(),
                "id {id} is listed under All Languages"
            );
        }
        for id in 0..=200u32 {
            assert_eq!(
                builtin_format_code(id).is_some(),
                listed.contains(&id),
                "id {id} is built in for every language exactly when §18.8.30 lists it"
            );
        }
    }

    /// 37 and 38 carry a space before the semicolon; 39 and 40 do not.
    ///
    /// This is the assertion that fails if somebody "fixes" the table, and it is here because the
    /// asymmetry looks like a typo and is not.
    #[test]
    fn the_published_spacing_of_the_accounting_formats_is_reproduced_exactly() {
        assert_eq!(builtin_format_code(37), Some("#,##0 ;(#,##0)"));
        assert_eq!(builtin_format_code(38), Some("#,##0 ;[Red](#,##0)"));
        assert_eq!(builtin_format_code(39), Some("#,##0.00;(#,##0.00)"));
        assert_eq!(builtin_format_code(40), Some("#,##0.00;[Red](#,##0.00)"));
    }

    /// A locale-dependent id has no all-languages answer, has one per language, and the languages
    /// genuinely disagree.
    #[test]
    fn a_locale_dependent_id_answers_differently_per_language() {
        for id in [27, 30, 31, 34, 58] {
            assert!(is_locale_dependent(id));
            assert_eq!(
                builtin_format_code(id),
                None,
                "id {id} depends on the UI language, so there is no all-languages answer"
            );
        }
        // Id 30 is the cleanest disagreement: three different codes across four languages.
        assert_eq!(
            builtin_format_code_in(30, NumberFormatLanguage::ChineseTaiwan),
            Some("m/d/yy")
        );
        assert_eq!(
            builtin_format_code_in(30, NumberFormatLanguage::ChineseChina),
            Some("m-d-yy")
        );
        assert_eq!(
            builtin_format_code_in(30, NumberFormatLanguage::Japanese),
            Some("m/d/yy")
        );
        assert_eq!(
            builtin_format_code_in(30, NumberFormatLanguage::Korean),
            Some("mm-dd-yy")
        );
        // Thai's block is disjoint from the CJK one, and neither answers for the other's ids.
        assert_eq!(
            builtin_format_code_in(71, NumberFormatLanguage::Thai),
            Some("\u{0e27}/\u{0e14}/\u{0e1b}\u{0e1b}\u{0e1b}\u{0e1b}")
        );
        assert_eq!(
            builtin_format_code_in(71, NumberFormatLanguage::Japanese),
            None
        );
        assert_eq!(
            builtin_format_code_in(27, NumberFormatLanguage::Thai),
            None,
            "the Thai table starts at 59; 27 is a CJK id"
        );
    }

    /// Every id the locale tables answer for is one [`is_locale_dependent`] admits, and the reverse.
    ///
    /// The trap: a predicate written as a range and a table written as a match drift apart one id
    /// at a time, and nothing else notices.
    #[test]
    fn the_locale_predicate_and_the_locale_tables_name_the_same_ids() {
        let languages = [
            NumberFormatLanguage::ChineseTaiwan,
            NumberFormatLanguage::ChineseChina,
            NumberFormatLanguage::Japanese,
            NumberFormatLanguage::Korean,
            NumberFormatLanguage::Thai,
        ];
        for id in 0..=200u32 {
            let answered = languages
                .iter()
                .any(|language| builtin_format_code_in(id, *language).is_some());
            let expected = is_locale_dependent(id) || builtin_format_code(id).is_some();
            assert_eq!(
                answered,
                expected,
                "id {id}: is_locale_dependent says {}, the tables say {answered}",
                is_locale_dependent(id)
            );
        }
    }

    /// The table is keyed by `@numFmtId`, and a file may write its entries in any order.
    #[test]
    fn entries_are_found_by_id_rather_than_by_position() {
        let markup = concat!(
            r#"<numFmts xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="2">"#,
            r#"<numFmt numFmtId="180" formatCode="0.0"/>"#,
            r#"<numFmt numFmtId="164" formatCode="0.000"/>"#,
            "</numFmts>"
        );
        let document = mjx_xml::fidelity::parse(markup.as_bytes()).expect("the table parses");
        let table = <NumberFormatTable as mjx_ooxml_core::FromXml>::from_xml(
            &document.root,
            &document.interner,
        )
        .expect("the table reads");

        assert_eq!(table.len(), 2);
        let first = table
            .get(&document.interner, 164)
            .expect("id 164 is declared");
        assert_eq!(
            first
                .format_code(&document.interner)
                .expect("the code decodes")
                .as_deref(),
            Some("0.000"),
            "164 is the *second* element; a position-indexed lookup would answer 0.0"
        );
        assert!(table.get(&document.interner, 165).is_none());
    }
}
