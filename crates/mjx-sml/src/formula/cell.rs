//! `CT_CellFormula` (`sml.xsd:2751`) — a formula, read out of the bytes the file wrote.
//!
//! # Why this is a view and not a struct with fields
//!
//! The obvious model is a struct: a `String` for the text, an `Option<CellRange>` for `@ref`, an
//! `Option<u32>` for `@si`, seven `bool`s. It would be about sixty bytes and one heap allocation per
//! formula cell, and it would be wrong here for two separate reasons.
//!
//! * **Memory.** `docs/BENCHMARKS.md` measures a 300,000-cell worksheet at ≈ 913 bytes of peak
//!   resident set per cell as a [`RawElement`](mjx_ooxml_core::RawElement) tree, and attributes the
//!   cost to the small per-element heap allocations rather than to the structs. A sheet can be a
//!   million formula cells. A `String` per cell is the same mistake in a new place — and most of
//!   those cells are shared-group members whose `<f>` holds *no text at all*, so the allocation
//!   would be for nothing.
//! * **Fidelity.** The formula has to be re-emitted byte for byte: `&amp;` written as `&amp;` and not
//!   as `&`, `si` before `t` if that is how the producer wrote them, a single-quoted `@ref` still
//!   single-quoted. A decoded struct cannot reproduce any of that, so the bytes would have to be kept
//!   *as well* — at which point the struct is a second, redundant copy that is free to disagree with
//!   the one the writer uses.
//!
//! So there is one copy of a formula in this workspace — the range MJXOFF-95's cell store already
//! keeps — and this type is a reader over it. Every accessor is a bounded scan of a start tag that
//! is a few dozen bytes long, with no allocation unless the caller asks for a decoded `String`.
//!
//! # What "checked" means for a run of bytes
//!
//! `crate::arena::decompose` states the rule: a byte range is a *claim about somebody else's
//! buffer*, and a claim that does not check out must degrade to nothing rather than to wrong bytes.
//! [`CellFormula::parse`] therefore takes the element's qualified name **from the bytes**, requires
//! its local part to be `f`, and requires the element to close the way an element closes. It cannot
//! be handed a `<v>` and answer as though it were a formula.

use std::borrow::Cow;

use mjx_ooxml_core::AttributeError;
use mjx_ooxml_types::support::on_off;

use crate::address::{CellRange, CellReference};
use crate::arena::attributes;
use crate::arena::decompose::{decompose, qualified_name_of};

use super::FormulaKind;

/// `x:f` (`CT_CellFormula`) — one cell's formula, as the bytes the file wrote.
///
/// A borrowed view: it holds no owned data, allocates nothing on construction, and answers every
/// accessor by scanning the element's own bytes. See the [module docs](self) for why the alternative
/// — a decoded struct — is both larger and less faithful.
///
/// # The twelve attributes
///
/// `CT_CellFormula` is a `simpleContent` extension of `ST_Formula` (`xsd:string`) with exactly
/// twelve attributes, every one `use="optional"`. The names below come from ECMA-376 Part 1
/// §18.3.1.40's own attribute table, which is what `CLAUDE.md` requires for tokens whose meaning is
/// not inferable — `bx` is *"Assigns Value to Name"* and nothing about the token says so.
///
/// | Wire | Prose name (§18.3.1.40) | Accessor | Type |
/// |---|---|---|---|
/// | `t` | Formula Type | [`kind`](Self::kind) / [`written_kind`](Self::written_kind) | `ST_CellFormulaType`, default `normal` |
/// | `ref` | Range of Cells | [`range`](Self::range) | `ST_Ref` |
/// | `si` | Shared Group Index | [`shared_group_index`](Self::shared_group_index) | `xsd:unsignedInt` |
/// | `aca` | Always Calculate Array | [`always_calculate_array`](Self::always_calculate_array) | `xsd:boolean`, default `false` |
/// | `ca` | Calculate Cell | [`needs_recalculation`](Self::needs_recalculation) | `xsd:boolean`, default `false` |
/// | `bx` | Assigns Value to Name | [`assigns_value_to_name`](Self::assigns_value_to_name) | `xsd:boolean`, default `false` |
/// | `dt2D` | Data Table 2-D | [`is_two_dimensional_data_table`](Self::is_two_dimensional_data_table) | `xsd:boolean`, default `false` |
/// | `dtr` | Data Table Row | [`is_row_oriented_data_table`](Self::is_row_oriented_data_table) | `xsd:boolean`, default `false` |
/// | `del1` | Input 1 Deleted | [`first_input_cell_deleted`](Self::first_input_cell_deleted) | `xsd:boolean`, default `false` |
/// | `del2` | Input 2 Deleted | [`second_input_cell_deleted`](Self::second_input_cell_deleted) | `xsd:boolean`, default `false` |
/// | `r1` | Data Table Cell 1 | [`first_input_cell`](Self::first_input_cell) | `ST_CellRef` |
/// | `r2` | Input Cell 2 | [`second_input_cell`](Self::second_input_cell) | `ST_CellRef` |
///
/// **The prose documents a thirteenth, `xml:space`, that the Transitional schema does not declare**
/// on this type — `CT_CellFormula` has no attribute wildcard, so an `<f xml:space="preserve">` is
/// prose-legal and schema-invalid. Nothing here rejects one: it is preserved with the rest of the
/// start tag and readable through [`raw_attribute`](Self::raw_attribute), which is what this
/// workspace does with every attribute it does not model.
///
/// # Nothing here writes
///
/// There is no setter on this type, and that is deliberate rather than unfinished. Changing a
/// formula's text is a caller's edit to a cell, which goes through the cell store's one door; nothing
/// this library does *on its own* may ever change a formula or the value cached beside it. See the
/// [module docs](self).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellFormula<'a> {
    markup: &'a [u8],
    attribute_run: &'a [u8],
    text: &'a [u8],
    self_closing: bool,
}

impl<'a> CellFormula<'a> {
    /// The wire local name this type is written under: `f`.
    pub const WIRE_LOCAL: &'static str = "f";

    /// Reads a formula out of one `<f …>…</f>` element's own bytes.
    ///
    /// `None` when `markup` is not exactly one element whose local name is `f` — the same refusal
    /// `crate::arena::decompose` makes, and for the same reason. The vetted way to reach this is
    /// [`Cell::formula`](crate::Cell::formula), which hands over a range the reader recorded for an
    /// element it had already matched by local name.
    #[must_use]
    pub fn parse(markup: &'a [u8]) -> Option<Self> {
        let qname = qualified_name_of(markup)?;
        let local = match qname.iter().position(|byte| *byte == b':') {
            Some(colon) => qname.get(colon + 1..)?,
            None => qname,
        };
        if local != Self::WIRE_LOCAL.as_bytes() {
            return None;
        }
        let parts = decompose(markup, qname)?;
        Some(Self {
            markup,
            attribute_run: markup.get(parts.attribute_run)?,
            text: markup.get(parts.inner)?,
            self_closing: parts.self_closing,
        })
    }

    /// The whole `<f …>…</f>`, exactly as the file wrote it.
    ///
    /// This is what the writer copies, and therefore the only description of the formula that
    /// matters for a round trip. Every other accessor here is derived from it.
    #[must_use]
    pub fn markup(&self) -> &'a [u8] {
        self.markup
    }

    /// The start tag's attribute run — everything between `<f` and the `>` that closes it.
    #[must_use]
    pub fn attribute_run(&self) -> &'a [u8] {
        self.attribute_run
    }

    /// The formula expression, **still escaped**, exactly as the file wrote it.
    ///
    /// Empty for a shared-group member, whose `<f>` carries `@si` and no text at all, and for a
    /// data-table formula, whose expression is the implied `TABLE()`.
    #[must_use]
    pub fn raw_text(&self) -> &'a [u8] {
        self.text
    }

    /// Whether this `<f>` carries any expression text.
    ///
    /// **The question a shared group turns on.** The host carries the text; every other member
    /// carries none, and writing the host's text into a member is a corruption rather than an
    /// optimisation — see [`SharedFormulaGroups`](super::SharedFormulaGroups).
    #[must_use]
    pub fn has_text(&self) -> bool {
        !self.text.is_empty()
    }

    /// Whether the file wrote `<f …/>` rather than `<f …></f>`.
    ///
    /// Two spellings of an empty formula, and two different byte sequences; a store that
    /// re-emitted one as the other would change a part nobody edited.
    #[must_use]
    pub fn is_self_closing(&self) -> bool {
        self.self_closing
    }

    /// The formula expression with its entity references resolved.
    ///
    /// Formula text is untrusted and may hold anything a string may hold — `&`, `<`, quotes, and
    /// whatever escapes its producer chose for them. This decodes; nothing normalises, and the
    /// decoded form is never written back in place of the original.
    ///
    /// # Errors
    /// [`mjx_xml::XmlError`] if the text is not UTF-8 or carries a reference that will not decode.
    pub fn text(&self) -> Result<Cow<'a, str>, mjx_xml::XmlError> {
        let text = core::str::from_utf8(self.text)
            .map_err(|_| mjx_xml::XmlError::Syntax("a formula was not UTF-8".to_owned()))?;
        mjx_xml::text::unescape_text(text)
    }

    /// The still-escaped value of the attribute `name`, or `None` if the start tag does not carry
    /// it — including attributes this workspace does not model, such as the `xml:space` §18.3.1.40
    /// documents and the schema does not declare.
    #[must_use]
    pub fn raw_attribute(&self, name: &str) -> Option<&'a [u8]> {
        attributes::value(self.attribute_run, name)
    }

    /// Whether `f@t` was written at all.
    ///
    /// **This, and not [`kind`](Self::kind), is the question a round trip turns on.** The schema
    /// declares `t` with `default="normal"`, so an absent `t` and `t="normal"` mean the same thing
    /// and are different bytes; a file that said nothing must come back saying nothing.
    #[must_use]
    pub fn has_written_kind(&self) -> bool {
        self.raw_attribute("t").is_some()
    }

    /// `f@t` exactly as written, or `None` when the attribute is absent.
    ///
    /// # Errors
    /// [`AttributeError`] if the value will not decode or is not one of the four tokens
    /// `ST_CellFormulaType` declares. An unrecognised token is reported rather than folded into
    /// `normal`: the file says something this workspace does not understand, and saying so is the
    /// honest answer.
    pub fn written_kind(&self) -> Result<Option<FormulaKind>, AttributeError> {
        let Some(raw) = self.raw_attribute("t") else {
            return Ok(None);
        };
        let text = decoded(raw, "f@t")?;
        FormulaKind::from_wire(&text)
            .map(Some)
            .ok_or_else(|| AttributeError::InvalidValue {
                attribute: "f@t",
                detail: format!("{text:?} is not one of normal, array, dataTable or shared"),
            })
    }

    /// The kind this formula has, with the schema's `default="normal"` applied.
    ///
    /// # Errors
    /// As [`written_kind`](Self::written_kind).
    pub fn kind(&self) -> Result<FormulaKind, AttributeError> {
        Ok(self.written_kind()?.unwrap_or(FormulaKind::Normal))
    }

    /// `f@ref` — *Range of Cells*: the range a shared group, an array formula or a data table
    /// applies to.
    ///
    /// §18.3.1.40: *"Only required for shared formula, array formula or data table. Only written on
    /// the master formula, not subsequent formulas belonging to the same shared group, array, or
    /// data table."* So a present `@ref` is what distinguishes a group's **host** from its members.
    ///
    /// # Errors
    /// [`AttributeError`] if the value will not decode or is not an `ST_Ref`.
    pub fn range(&self) -> Result<Option<CellRange>, AttributeError> {
        parse_range(self.raw_attribute("ref"), "f@ref")
    }

    /// `f@si` — *Shared Group Index*: which group of shared formulas this cell's formula belongs to.
    ///
    /// # Errors
    /// [`AttributeError`] if the value will not decode or is not an `xsd:unsignedInt`.
    pub fn shared_group_index(&self) -> Result<Option<u32>, AttributeError> {
        let Some(raw) = self.raw_attribute("si") else {
            return Ok(None);
        };
        let text = decoded(raw, "f@si")?;
        text.trim()
            .parse::<u32>()
            .map(Some)
            .map_err(|error| AttributeError::InvalidValue {
                attribute: "f@si",
                detail: format!("{text:?} is not an xsd:unsignedInt ({error})"),
            })
    }

    /// Whether this `<f>` is the **host** of a shared group — the cell §18.3.1.40 calls the *master*,
    /// which carries the text and the `@ref` naming the range the group covers.
    ///
    /// # Errors
    /// As [`kind`](Self::kind) and [`range`](Self::range).
    pub fn is_shared_group_host(&self) -> Result<bool, AttributeError> {
        Ok(self.kind()? == FormulaKind::Shared && self.raw_attribute("ref").is_some())
    }

    /// Whether this `<f>` is a **member** of a shared group other than its host: `t="shared"` with an
    /// `@si` and no `@ref`.
    ///
    /// Such a cell normally carries no text either, and this library never gives it any. Reporting
    /// a member's own expression would mean shifting the host's text by the offset between the two
    /// cells — reference translation, which §18.3.1.40 describes and which is explicitly out of this
    /// workspace's scope.
    ///
    /// # Errors
    /// As [`kind`](Self::kind).
    pub fn is_shared_group_member(&self) -> Result<bool, AttributeError> {
        Ok(self.kind()? == FormulaKind::Shared && self.raw_attribute("ref").is_none())
    }

    /// `f@aca` — *Always Calculate Array*: the whole array is calculated in full rather than cell by
    /// cell. Ignored unless `t` is `array`.
    ///
    /// # Errors
    /// [`AttributeError`] if the value will not decode or is not an `xsd:boolean`.
    pub fn always_calculate_array(&self) -> Result<bool, AttributeError> {
        self.flag("aca", "f@aca")
    }

    /// `f@ca` — *Calculate Cell*: §18.3.1.40's *"Indicates that this formula needs to be recalculated
    /// the next time calculation is performed"*, which Excel sets on volatile functions such as
    /// `RAND()` and on circular references.
    ///
    /// **Reported, never acted on.** Nothing in this workspace calculates, so this is a fact about
    /// the file rather than an instruction to it.
    ///
    /// # Errors
    /// As [`always_calculate_array`](Self::always_calculate_array).
    pub fn needs_recalculation(&self) -> Result<bool, AttributeError> {
        self.flag("ca", "f@ca")
    }

    /// `f@bx` — *Assigns Value to Name*: this formula assigns a value to a defined name.
    ///
    /// # Errors
    /// As [`always_calculate_array`](Self::always_calculate_array).
    pub fn assigns_value_to_name(&self) -> Result<bool, AttributeError> {
        self.flag("bx", "f@bx")
    }

    /// `f@dt2D` — *Data Table 2-D*: the data table has two input variables rather than one. Written
    /// on the master cell of a data-table formula only.
    ///
    /// # Errors
    /// As [`always_calculate_array`](Self::always_calculate_array).
    pub fn is_two_dimensional_data_table(&self) -> Result<bool, AttributeError> {
        self.flag("dt2D", "f@dt2D")
    }

    /// `f@dtr` — *Data Table Row*: a one-input data table is row-oriented rather than
    /// column-oriented.
    ///
    /// # Errors
    /// As [`always_calculate_array`](Self::always_calculate_array).
    pub fn is_row_oriented_data_table(&self) -> Result<bool, AttributeError> {
        self.flag("dtr", "f@dtr")
    }

    /// `f@del1` — *Input 1 Deleted*: the data table's first input cell has been deleted.
    ///
    /// # Errors
    /// As [`always_calculate_array`](Self::always_calculate_array).
    pub fn first_input_cell_deleted(&self) -> Result<bool, AttributeError> {
        self.flag("del1", "f@del1")
    }

    /// `f@del2` — *Input 2 Deleted*: the data table's second input cell has been deleted.
    ///
    /// # Errors
    /// As [`always_calculate_array`](Self::always_calculate_array).
    pub fn second_input_cell_deleted(&self) -> Result<bool, AttributeError> {
        self.flag("del2", "f@del2")
    }

    /// `f@r1` — *Data Table Cell 1*: the data table's first input cell.
    ///
    /// # Errors
    /// [`AttributeError`] if the value will not decode or is not an `ST_CellRef`.
    pub fn first_input_cell(&self) -> Result<Option<CellReference>, AttributeError> {
        parse_reference(self.raw_attribute("r1"), "f@r1")
    }

    /// `f@r2` — *Input Cell 2*: the data table's second input cell, used when `@dt2D` is set.
    ///
    /// # Errors
    /// As [`first_input_cell`](Self::first_input_cell).
    pub fn second_input_cell(&self) -> Result<Option<CellReference>, AttributeError> {
        parse_reference(self.raw_attribute("r2"), "f@r2")
    }

    /// One `xsd:boolean` attribute, with the schema's `default="false"` applied.
    fn flag(&self, name: &str, qualified: &'static str) -> Result<bool, AttributeError> {
        let Some(raw) = self.raw_attribute(name) else {
            return Ok(false);
        };
        let text = decoded(raw, qualified)?;
        on_off::from_wire(text.trim()).ok_or(AttributeError::InvalidValue {
            attribute: qualified,
            detail: format!("{text:?} is not an xsd:boolean"),
        })
    }
}

/// One attribute value, decoded from the raw bytes of a start tag.
fn decoded<'a>(raw: &'a [u8], attribute: &'static str) -> Result<Cow<'a, str>, AttributeError> {
    let text = core::str::from_utf8(raw).map_err(|_| AttributeError::InvalidUtf8 { attribute })?;
    mjx_xml::text::unescape_text(text).map_err(|error| AttributeError::InvalidEntity {
        attribute,
        detail: error.to_string(),
    })
}

fn parse_range(
    raw: Option<&[u8]>,
    attribute: &'static str,
) -> Result<Option<CellRange>, AttributeError> {
    let Some(raw) = raw else { return Ok(None) };
    let text = decoded(raw, attribute)?;
    CellRange::parse(&text)
        .map(Some)
        .map_err(|error| AttributeError::InvalidValue {
            attribute,
            detail: format!("{text:?} is not an ST_Ref ({error})"),
        })
}

fn parse_reference(
    raw: Option<&[u8]>,
    attribute: &'static str,
) -> Result<Option<CellReference>, AttributeError> {
    let Some(raw) = raw else { return Ok(None) };
    let text = decoded(raw, attribute)?;
    CellReference::parse(&text)
        .map(Some)
        .map_err(|error| AttributeError::InvalidValue {
            attribute,
            detail: format!("{text:?} is not an ST_CellRef ({error})"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_t_and_a_written_normal_are_the_same_meaning_and_different_answers() {
        let absent = CellFormula::parse(b"<f>SUM(A1:A3)</f>").expect("an element");
        assert!(!absent.has_written_kind());
        assert_eq!(absent.written_kind(), Ok(None));
        assert_eq!(absent.kind(), Ok(FormulaKind::Normal));

        let written = CellFormula::parse(br#"<f t="normal">SUM(A1:A3)</f>"#).expect("an element");
        assert!(written.has_written_kind());
        assert_eq!(written.written_kind(), Ok(Some(FormulaKind::Normal)));
        assert_eq!(written.kind(), Ok(FormulaKind::Normal));

        // Same meaning, different bytes — and the bytes are what is written back.
        assert_eq!(absent.kind(), written.kind());
        assert_ne!(absent.markup(), written.markup());
    }

    #[test]
    fn a_shared_host_and_its_members_are_told_apart_by_ref_and_by_text() {
        let host = CellFormula::parse(br#"<f t="shared" ref="B2:B6" si="0">A2*2</f>"#)
            .expect("an element");
        assert_eq!(host.is_shared_group_host(), Ok(true));
        assert_eq!(host.is_shared_group_member(), Ok(false));
        assert!(host.has_text());
        assert_eq!(host.shared_group_index(), Ok(Some(0)));
        assert_eq!(
            host.range(),
            Ok(Some(CellRange::parse("B2:B6").expect("a range")))
        );

        let member = CellFormula::parse(br#"<f t="shared" si="0"/>"#).expect("an element");
        assert_eq!(member.is_shared_group_host(), Ok(false));
        assert_eq!(member.is_shared_group_member(), Ok(true));
        assert!(!member.has_text(), "a member carries no text at all");
        assert!(member.is_self_closing());
        assert_eq!(member.shared_group_index(), Ok(Some(0)));
        assert_eq!(member.range(), Ok(None));
    }

    #[test]
    fn an_empty_formula_written_long_hand_is_not_a_self_closing_one() {
        let long_hand = CellFormula::parse(br#"<f t="shared" si="3"></f>"#).expect("an element");
        assert!(!long_hand.has_text());
        assert!(!long_hand.is_self_closing());
    }

    #[test]
    fn formula_text_is_decoded_and_the_bytes_are_kept() {
        let formula =
            CellFormula::parse(br#"<f>IF(A1&lt;2,&quot;a&amp;b&quot;,&quot;c&quot;)</f>"#)
                .expect("an element");
        assert_eq!(formula.text().expect("decodes"), r#"IF(A1<2,"a&b","c")"#);
        assert_eq!(
            formula.raw_text(),
            br#"IF(A1&lt;2,&quot;a&amp;b&quot;,&quot;c&quot;)"#,
            "the escaped bytes are what round-trips; the decoded string is a report"
        );
    }

    #[test]
    fn a_data_table_formula_reports_its_six_attributes() {
        let formula = CellFormula::parse(
            br#"<f t="dataTable" ref="B4:C6" dt2D="1" dtr="0" del1="0" del2="1" r1="B1" r2="B2"/>"#,
        )
        .expect("an element");
        assert_eq!(formula.kind(), Ok(FormulaKind::DataTable));
        assert_eq!(formula.is_two_dimensional_data_table(), Ok(true));
        assert_eq!(formula.is_row_oriented_data_table(), Ok(false));
        assert_eq!(formula.first_input_cell_deleted(), Ok(false));
        assert_eq!(formula.second_input_cell_deleted(), Ok(true));
        assert_eq!(
            formula.first_input_cell(),
            Ok(Some(CellReference::parse("B1").expect("a reference")))
        );
        assert_eq!(
            formula.second_input_cell(),
            Ok(Some(CellReference::parse("B2").expect("a reference")))
        );
    }

    #[test]
    fn the_flags_default_to_false_and_accept_both_xsd_boolean_spellings() {
        let bare = CellFormula::parse(b"<f>A1</f>").expect("an element");
        for flag in [
            bare.always_calculate_array(),
            bare.needs_recalculation(),
            bare.assigns_value_to_name(),
            bare.is_two_dimensional_data_table(),
            bare.is_row_oriented_data_table(),
            bare.first_input_cell_deleted(),
            bare.second_input_cell_deleted(),
        ] {
            assert_eq!(flag, Ok(false));
        }
        assert_eq!(
            CellFormula::parse(br#"<f ca="1" bx="true" aca="0">A1</f>"#)
                .expect("an element")
                .needs_recalculation(),
            Ok(true)
        );
        assert_eq!(
            CellFormula::parse(br#"<f ca="1" bx="true" aca="0">A1</f>"#)
                .expect("an element")
                .assigns_value_to_name(),
            Ok(true)
        );
        assert_eq!(
            CellFormula::parse(br#"<f ca="1" bx="true" aca="0">A1</f>"#)
                .expect("an element")
                .always_calculate_array(),
            Ok(false)
        );
    }

    #[test]
    fn a_prefixed_formula_reads_and_an_element_that_is_not_a_formula_does_not() {
        let prefixed = CellFormula::parse(br#"<x:f t="array" ref="D2:D4">SUM(1)</x:f>"#)
            .expect("the prefix is the file's choice, not the reader's");
        assert_eq!(prefixed.kind(), Ok(FormulaKind::Array));
        assert!(prefixed.has_text());

        // The refusals: another element, a truncated one, an unclosed name, and empty bytes.
        assert_eq!(CellFormula::parse(b"<v>3</v>"), None);
        assert_eq!(CellFormula::parse(b"<f>SUM(1)"), None);
        assert_eq!(CellFormula::parse(b"<f"), None);
        assert_eq!(CellFormula::parse(b""), None);
        assert_eq!(
            CellFormula::parse(b"<fx>1</fx>"),
            None,
            "a local name that merely starts with `f` is not `f`"
        );
    }

    #[test]
    fn an_unrecognised_t_is_reported_rather_than_folded_into_normal() {
        let formula = CellFormula::parse(br#"<f t="tableRow">A1</f>"#).expect("an element");
        let error = formula
            .written_kind()
            .expect_err("`tableRow` is not a kind");
        assert_eq!(error.attribute(), "f@t");
        assert!(error.to_string().contains("tableRow"), "{error}");
        // …and the bytes are still there to be written back unchanged.
        assert_eq!(formula.markup(), br#"<f t="tableRow">A1</f>"#);
    }

    #[test]
    fn an_attribute_the_schema_does_not_declare_is_readable_rather_than_refused() {
        // §18.3.1.40's prose documents `xml:space` on this element; `CT_CellFormula` declares no
        // such attribute and no wildcard, so it is prose-legal and schema-invalid. It is preserved
        // with the rest of the start tag either way.
        let formula = CellFormula::parse(br#"<f xml:space="preserve"> A1 </f>"#).expect("element");
        assert_eq!(formula.raw_attribute("xml:space"), Some(&b"preserve"[..]));
        assert_eq!(formula.raw_text(), b" A1 ");
    }
}
