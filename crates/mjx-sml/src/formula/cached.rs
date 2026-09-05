//! The value cached beside a formula: `c/v`, read through the `c@t` that says what it means.
//!
//! # Why a type rather than "the cell's value"
//!
//! MJXOFF-95's [`Cell`](crate::Cell) already answers `value`, `number`, `boolean` and
//! `shared_string_index` for any cell. A `<v>` beside an `<f>` is the same bytes read the same way —
//! but it is a **different fact about the file**, and the difference is the whole point of this
//! child:
//!
//! * A `<v>` with no `<f>` is a value somebody typed. It is the cell's content.
//! * A `<v>` beside an `<f>` is *the result Excel last computed*, stored so that a consumer need not
//!   calculate to display the sheet. It is a **cache**, and this library will not maintain it.
//!
//! [`Cell::cached_value`](crate::Cell::cached_value) answers only in the second case, so a caller
//! cannot ask "what is cached here" and be handed a literal. And the answer is a report: there is no
//! setter, nothing recomputes it, and nothing blanks it when a cell it depends on changes. **A stale
//! cached value is the correct behaviour of this library**, stated at every level it appears —
//! see the [module docs](super).

use std::borrow::Cow;

use mjx_ooxml_types::spreadsheetml::CellType;

/// The result a producer last computed for a formula cell — `c/v`, with the `c@t` that says how to
/// read it.
///
/// # Reading it
///
/// `c@t` decides what the same digits mean: `<v>3</v>` is the number three under `n`, the string at
/// index three of the shared-string table under `s`, and `true` under `b`. The typed accessors below
/// each answer **only** for the type they are about, so a caller cannot read a shared-string index as
/// a number by accident. [`raw_text`](Self::raw_text) is always available and is what the writer
/// copies.
///
/// # It is never written back from here
///
/// This type has no setter and no constructor a caller can reach. A cached value comes out of the
/// bytes a file was read from and goes back as those bytes; the one way it changes is a caller
/// explicitly setting the cell's value through
/// [`SheetData::set_cell_value`](crate::SheetData::set_cell_value), which is that caller's edit
/// rather than this library's correction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedValue<'a> {
    cell_type: CellType,
    written_cell_type: bool,
    raw: &'a [u8],
}

impl<'a> CachedValue<'a> {
    /// Builds the view. Crate-private: a cached value is something a file has, not something a
    /// caller makes.
    pub(crate) fn new(cell_type: CellType, written_cell_type: bool, raw: &'a [u8]) -> Self {
        Self {
            cell_type,
            written_cell_type,
            raw,
        }
    }

    /// The `c@t` this value is read under, with the schema's `default="n"` applied.
    #[must_use]
    pub fn cell_type(self) -> CellType {
        self.cell_type
    }

    /// Whether `c@t` was written on the cell at all.
    ///
    /// The same absent-versus-default distinction `f@t` has one element down, and kept for the same
    /// reason: `n` is the schema default, so a cell that wrote nothing must come back writing
    /// nothing. MJXOFF-95 stores it as a distinct code rather than as [`CellType::Number`].
    #[must_use]
    pub fn has_written_cell_type(self) -> bool {
        self.written_cell_type
    }

    /// The `<v>`'s text, **still escaped**, exactly as the file wrote it.
    #[must_use]
    pub fn raw_text(self) -> &'a [u8] {
        self.raw
    }

    /// The `<v>`'s text with its entity references resolved.
    ///
    /// # Errors
    /// [`mjx_xml::XmlError`] if the text is not UTF-8 or carries a reference that will not decode.
    pub fn text(self) -> Result<Cow<'a, str>, mjx_xml::XmlError> {
        let text = core::str::from_utf8(self.raw)
            .map_err(|_| mjx_xml::XmlError::Syntax("a cached value was not UTF-8".to_owned()))?;
        mjx_xml::text::unescape_text(text)
    }

    /// The cached number, for a value whose `c@t` is `n` — the schema default, so this is also the
    /// answer for a cell that wrote no `t` at all.
    ///
    /// `None` for any other type, and for an `n` value whose text is not a number, which a file can
    /// say and which is reported as absence rather than repaired.
    #[must_use]
    pub fn as_number(self) -> Option<f64> {
        (self.cell_type == CellType::Number)
            .then(|| self.decoded_ascii()?.trim().parse::<f64>().ok())
            .flatten()
    }

    /// The cached boolean, for a value whose `c@t` is `b`. `1` and `0` are what a producer writes.
    #[must_use]
    pub fn as_boolean(self) -> Option<bool> {
        (self.cell_type == CellType::Boolean)
            .then(|| match self.decoded_ascii()?.trim() {
                "1" | "true" => Some(true),
                "0" | "false" => Some(false),
                _ => None,
            })
            .flatten()
    }

    /// The cached shared-string index, for a value whose `c@t` is `s`. MJXOFF-97's
    /// [`SharedStringTable`](crate::SharedStringTable) is what it indexes into.
    #[must_use]
    pub fn as_shared_string_index(self) -> Option<u32> {
        (self.cell_type == CellType::SharedString)
            .then(|| self.decoded_ascii()?.trim().parse::<u32>().ok())
            .flatten()
    }

    /// The cached error token — `#REF!`, `#DIV/0!`, `#N/A` and the rest — for a value whose `c@t`
    /// is `e`.
    ///
    /// Returned as the text the file wrote. There is no enumeration of error values in the schema
    /// (`ST_CellType`'s `e` says only that the content is an error), and inventing one would mean
    /// guessing at a token set the standard does not fix.
    ///
    /// # Errors
    /// As [`text`](Self::text).
    pub fn as_error_text(self) -> Result<Option<Cow<'a, str>>, mjx_xml::XmlError> {
        if self.cell_type != CellType::Error {
            return Ok(None);
        }
        self.text().map(Some)
    }

    /// The cached string, for a value whose `c@t` is `str` — a formula that returned text, stored
    /// inline in the `<v>` rather than through the shared-string table.
    ///
    /// # Errors
    /// As [`text`](Self::text).
    pub fn as_formula_string(self) -> Result<Option<Cow<'a, str>>, mjx_xml::XmlError> {
        if self.cell_type != CellType::FormulaString {
            return Ok(None);
        }
        self.text().map(Some)
    }

    /// The raw bytes as a `&str` when they are UTF-8, for the numeric and boolean readers, which
    /// have nothing to unescape: a number, a boolean and an index carry no character that could be
    /// escaped, so a value that needs unescaping to parse was never one of them.
    fn decoded_ascii(self) -> Option<&'a str> {
        core::str::from_utf8(self.raw).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_reader_answers_only_for_its_own_cell_type() {
        let number = CachedValue::new(CellType::Number, false, b"12.5");
        assert_eq!(number.as_number(), Some(12.5));
        assert_eq!(number.as_boolean(), None);
        assert_eq!(number.as_shared_string_index(), None);
        assert!(!number.has_written_cell_type());

        let index = CachedValue::new(CellType::SharedString, true, b"3");
        assert_eq!(index.as_shared_string_index(), Some(3));
        assert_eq!(
            index.as_number(),
            None,
            "the same digits are an index, not the number three"
        );
        assert!(index.has_written_cell_type());

        let boolean = CachedValue::new(CellType::Boolean, true, b"1");
        assert_eq!(boolean.as_boolean(), Some(true));
        assert_eq!(boolean.as_number(), None);

        let error = CachedValue::new(CellType::Error, true, b"#DIV/0!");
        assert_eq!(
            error.as_error_text().expect("decodes").as_deref(),
            Some("#DIV/0!")
        );
        assert_eq!(error.as_number(), None);

        let text = CachedValue::new(CellType::FormulaString, true, b"a&amp;b");
        assert_eq!(
            text.as_formula_string().expect("decodes").as_deref(),
            Some("a&b")
        );
        assert_eq!(
            text.raw_text(),
            b"a&amp;b",
            "the escaped bytes are what round-trips"
        );
    }

    #[test]
    fn a_value_that_does_not_parse_is_reported_absent_rather_than_repaired() {
        assert_eq!(
            CachedValue::new(CellType::Number, true, b"not a number").as_number(),
            None
        );
        assert_eq!(
            CachedValue::new(CellType::Boolean, true, b"maybe").as_boolean(),
            None
        );
        assert_eq!(
            CachedValue::new(CellType::SharedString, true, b"-1").as_shared_string_index(),
            None
        );
    }
}
