//! Reading and editing an element's attribute run *as bytes*.
//!
//! # Why the store keeps attributes as a byte run
//!
//! [`RawElement`](mjx_ooxml_core::RawElement) models attributes as a `Vec<RawAttribute>`, where each
//! attribute owns a `Box<[u8]>` for its value. That is the right shape for a slide or a paragraph and
//! the wrong one for a worksheet: `docs/BENCHMARKS.md` attributes most of the measured 913 bytes of
//! peak resident set per cell to exactly this — a small heap allocation per attribute, over roughly
//! two attributes per cell, over hundreds of thousands of cells.
//!
//! So a row keeps its attributes the way the file wrote them: one range of bytes, `r="7" spans="1:3"`,
//! addressed in a [`TextArena`](super::text::TextArena). Reading one attribute is a scan of that
//! range; there is no allocation on either the read or the store path, and re-emitting the element is
//! a copy.
//!
//! It also buys fidelity that a decomposed list cannot: the run preserves attribute **order**,
//! **prefixes**, **quote characters** and the exact whitespace between attributes — the last of which
//! `mjx-xml`'s own writer explicitly gives up on for a rewritten element. A row this store rewrites
//! because one of its cells changed comes back with its start tag byte-identical.
//!
//! # Editing without losing what is not modelled
//!
//! The reason this module has a writer as well as a reader is that the alternative loses data.
//! Setting `c@s` on a cell whose start tag also carries an `x14ac:` attribute this workspace does not
//! model must not regenerate the tag from the four attributes the store decodes — that would drop the
//! fifth. [`set_attribute`] therefore rewrites the run *in place*: the named attribute's value is
//! replaced where it stands, or the attribute is appended if it was absent, and every other byte of
//! the run is copied through untouched.

/// One attribute found in a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Attribute<'a> {
    /// The qualified name exactly as written — `r`, `s`, `x14ac:dyDescent`.
    pub(crate) name: &'a [u8],
    /// The raw, still-escaped value bytes between the quotes.
    pub(crate) value: &'a [u8],
    /// Where the whole attribute sits in the run, name through closing quote.
    pub(crate) extent: core::ops::Range<usize>,
    /// Where the value sits in the run, between the quotes.
    pub(crate) value_extent: core::ops::Range<usize>,
}

/// Every attribute in `run`, in document order.
///
/// `run` is the bytes between an element's name and the `>` (or `/>`) that closes its start tag —
/// what [`super::read`] records and what [`super::write`] copies back. Parsing stops at the first
/// byte that cannot begin an attribute, so a malformed run yields the attributes it did understand
/// rather than an error: this is a *reader* over bytes that were already accepted as well-formed XML
/// by `mjx-xml`, and its job on anything else is to be uninformative, never to panic.
pub(crate) fn iter(run: &[u8]) -> AttributeRun<'_> {
    AttributeRun { run, cursor: 0 }
}

/// The iterator [`iter`] returns.
#[derive(Debug)]
pub(crate) struct AttributeRun<'a> {
    run: &'a [u8],
    cursor: usize,
}

impl<'a> Iterator for AttributeRun<'a> {
    type Item = Attribute<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        skip_whitespace(self.run, &mut self.cursor);
        let name_start = self.cursor;
        while let Some(byte) = self.run.get(self.cursor) {
            if byte.is_ascii_whitespace() || *byte == b'=' {
                break;
            }
            self.cursor += 1;
        }
        if self.cursor == name_start {
            return None;
        }
        let name = &self.run[name_start..self.cursor];
        skip_whitespace(self.run, &mut self.cursor);
        if self.run.get(self.cursor) != Some(&b'=') {
            // A bare attribute name is not XML; stop rather than guess at a value.
            return None;
        }
        self.cursor += 1;
        skip_whitespace(self.run, &mut self.cursor);
        let quote = *self.run.get(self.cursor)?;
        if quote != b'"' && quote != b'\'' {
            return None;
        }
        self.cursor += 1;
        let value_start = self.cursor;
        while self.run.get(self.cursor).is_some_and(|byte| *byte != quote) {
            self.cursor += 1;
        }
        if self.cursor >= self.run.len() {
            // An unterminated value; the run is not well-formed, so stop.
            return None;
        }
        let value_end = self.cursor;
        self.cursor += 1;
        Some(Attribute {
            name,
            value: &self.run[value_start..value_end],
            extent: name_start..self.cursor,
            value_extent: value_start..value_end,
        })
    }
}

/// The raw value of the attribute named `name`, or `None` if the run does not carry it.
pub(crate) fn value<'a>(run: &'a [u8], name: &str) -> Option<&'a [u8]> {
    iter(run)
        .find(|attribute| attribute.name == name.as_bytes())
        .map(|attribute| attribute.value)
}

/// Rewrites `run` with `name` set to `value`, or removed when `value` is `None`, appending the
/// result to `out`.
///
/// Everything the run carried that is not the named attribute is copied through byte for byte —
/// order, prefixes, quote characters and the whitespace between attributes included. An attribute
/// that was not there is appended at the end, preceded by one space, with a double quote; that is
/// the one place this function writes a byte of its own choosing, and it is the only place where a
/// start tag it produces can differ from one the file wrote.
///
/// `value` is written **raw**: the caller escapes it. Every value this crate sets is a number, a
/// wire token or an already-escaped payload, and an escape applied twice is a corruption, so the
/// decision is left where the value is known.
pub(crate) fn set_attribute(run: &[u8], name: &str, value: Option<&[u8]>, out: &mut Vec<u8>) {
    let mut written = false;
    let mut copied = 0usize;
    for attribute in iter(run) {
        if attribute.name != name.as_bytes() {
            continue;
        }
        match value {
            Some(new_value) if !written => {
                // Replace just the bytes between the quotes, so the name, the quote character and
                // the spacing around them all survive.
                out.extend_from_slice(&run[copied..attribute.value_extent.start]);
                out.extend_from_slice(new_value);
                copied = attribute.value_extent.end;
                written = true;
            }
            _ => {
                // Removed, or a duplicate of one already rewritten — drop the whole attribute along
                // with the whitespace that introduced it, so `a b c` does not become `a  c`.
                let from = leading_whitespace_start(run, attribute.extent.start, copied);
                out.extend_from_slice(&run[copied..from]);
                copied = attribute.extent.end;
            }
        }
    }
    out.extend_from_slice(&run[copied..]);
    if value.is_some() && !written {
        out.push(b' ');
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(b"=\"");
        out.extend_from_slice(value.unwrap_or_default());
        out.push(b'"');
    }
}

/// Where the whitespace introducing the attribute at `start` begins, but never before `floor`.
fn leading_whitespace_start(run: &[u8], start: usize, floor: usize) -> usize {
    let mut at = start;
    while at > floor && run[at - 1].is_ascii_whitespace() {
        at -= 1;
    }
    at
}

fn skip_whitespace(run: &[u8], cursor: &mut usize) {
    while run.get(*cursor).is_some_and(u8::is_ascii_whitespace) {
        *cursor += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names_and_values(run: &[u8]) -> Vec<(String, String)> {
        iter(run)
            .map(|attribute| {
                (
                    String::from_utf8_lossy(attribute.name).into_owned(),
                    String::from_utf8_lossy(attribute.value).into_owned(),
                )
            })
            .collect()
    }

    fn rewritten(run: &[u8], name: &str, value: Option<&[u8]>) -> String {
        let mut out = Vec::new();
        set_attribute(run, name, value, &mut out);
        String::from_utf8(out).expect("the run stays UTF-8")
    }

    #[test]
    fn reads_order_prefixes_and_both_quote_characters() {
        let run = br#" r="A1" s='3' x14ac:dyDescent="0.25""#;
        assert_eq!(
            names_and_values(run),
            vec![
                ("r".to_owned(), "A1".to_owned()),
                ("s".to_owned(), "3".to_owned()),
                ("x14ac:dyDescent".to_owned(), "0.25".to_owned()),
            ]
        );
        assert_eq!(value(run, "s"), Some(&b"3"[..]));
        assert_eq!(value(run, "t"), None);
    }

    #[test]
    fn a_value_may_carry_the_other_quote_and_a_bare_angle_bracket() {
        // `>` is legal unescaped inside an attribute value, and a single-quoted value may hold `"`.
        let run = br#" note='a > b "c"' r="A1""#;
        assert_eq!(
            names_and_values(run),
            vec![
                ("note".to_owned(), r#"a > b "c""#.to_owned()),
                ("r".to_owned(), "A1".to_owned()),
            ]
        );
    }

    #[test]
    fn setting_an_attribute_leaves_every_other_byte_alone() {
        let run = br#" r="A1"  s='3' x14ac:dyDescent="0.25""#;
        assert_eq!(
            rewritten(run, "s", Some(b"7")),
            r#" r="A1"  s='7' x14ac:dyDescent="0.25""#,
            "the double space, the single quotes and the unmodelled attribute all survive"
        );
    }

    #[test]
    fn setting_an_absent_attribute_appends_it_and_touches_nothing_else() {
        let run = br#" r="A1""#;
        assert_eq!(rewritten(run, "t", Some(b"s")), r#" r="A1" t="s""#);
        assert_eq!(
            rewritten(b"", "r", Some(b"B2")),
            r#" r="B2""#,
            "an element with no attributes gains exactly one"
        );
    }

    #[test]
    fn removing_an_attribute_takes_its_whitespace_with_it() {
        let run = br#" r="A1" s="3" t="s""#;
        assert_eq!(rewritten(run, "s", None), r#" r="A1" t="s""#);
        assert_eq!(rewritten(run, "r", None), r#" s="3" t="s""#);
        assert_eq!(rewritten(run, "t", None), r#" r="A1" s="3""#);
        assert_eq!(
            rewritten(run, "missing", None),
            r#" r="A1" s="3" t="s""#,
            "removing what is not there changes nothing"
        );
    }

    #[test]
    fn a_run_that_is_not_well_formed_stops_rather_than_guessing() {
        assert_eq!(names_and_values(b" r"), Vec::new(), "no `=`");
        assert_eq!(names_and_values(b" r="), Vec::new(), "no value");
        assert_eq!(names_and_values(b" r=A1"), Vec::new(), "unquoted value");
        assert_eq!(
            names_and_values(br#" r="A1"#),
            Vec::new(),
            "unterminated value"
        );
        assert_eq!(
            names_and_values(br#" r="A1" s"#),
            vec![("r".to_owned(), "A1".to_owned())],
            "what was understood is kept; what was not stops the scan"
        );
    }

    #[test]
    fn a_duplicated_attribute_is_rewritten_once_and_the_copy_is_dropped() {
        // Not well-formed XML, so `mjx-xml` will never hand this over — but the rewriter must still
        // produce one attribute rather than two disagreeing ones.
        let run = br#" s="1" s="2""#;
        assert_eq!(rewritten(run, "s", Some(b"9")), r#" s="9""#);
    }
}
