//! `x:xf` (`CT_Xf`, `sml.xsd:3598`) and the two tables that hold them — `CT_CellXfs`
//! (`sml.xsd:3592`) and `CT_CellStyleXfs` (`sml.xsd:3586`).
//!
//! # An `xf` is a pointer, not a format
//!
//! `CT_Xf` declares **thirteen** attributes and states almost nothing itself. Four are indices into
//! the resource tables MJXOFF-105 built (`numFmtId`, `fontId`, `fillId`, `borderId`), a fifth
//! (`xfId`) is an index into the *other* `xf` table, two are cell flags (`quotePrefix`,
//! `pivotButton`), and the remaining six are the `applyX` flags that decide which of those layers
//! actually participates. Only `alignment` and `protection` are stated inline, as children.
//!
//! (The ticket for this child said fourteen. It is thirteen; count them in the schema.)
//!
//! # `applyX` has three states, and the schema will not tell you
//!
//! Every one of the six is declared exactly like this:
//!
//! ```xml
//! <xsd:attribute name="applyFont" type="xsd:boolean" use="optional"/>
//! ```
//!
//! `use="optional"` and **no `default=`**. So `applyFont` **absent**, `applyFont="1"` and
//! `applyFont="0"` are three distinct states, and a model that decodes the attribute into a `bool`
//! has silently merged two of them. Nothing catches that: a document with the attribute absent and
//! one with `applyFont="0"` are *both* schema-valid, so the ECMA-376 gate cannot see the difference,
//! and every round-trip assertion still passes because the raw attribute vector is what gets written
//! back.
//!
//! The contrast is worth keeping in view, because the same file has both shapes.
//! `CT_CellFormula@t` **does** carry `default="normal"`, and MJXOFF-95 stores it as one byte with a
//! distinct code for *the attribute was absent* — same technique, opposite schema situation. Here
//! the three states are kept by declaring the attribute [`Presence::Optional`][optional] in
//! [`mjx_derive`]'s grammar, so the getter's type is `Option<bool>` and *absent is `None`*.
//! [`ApplyFlag`] is that `Option<bool>` named, for the callers who would rather match three variants
//! than remember which of them `None` is.
//!
//! [optional]: mjx_derive#read-never-normalizes-a-write-does
//!
//! # What the three states *mean*
//!
//! §18.8.45 defines each flag in one sentence — *"A boolean value indicating whether the alignment
//! formatting specified for this xf should be applied"* — and says **nothing about absence**. The
//! answer is in §18.8.9 instead, in the worked example beneath `cellStyleXfs`:
//!
//! > Note that 0th record does not express any "apply" attributes, while the other records do
//! > express "apply" attribute values. For example, the last record specifies that number format,
//! > alignment, and protection formatting will not be applied to the cell, even when that
//! > information is specified in related formatting records.
//!
//! The record that expresses no `apply` attributes is the `Normal` style, and it *is* applied. So an
//! absent flag does not suppress: [`ApplyFlag::Unstated`] behaves as [`ApplyFlag::Applied`] and not
//! as [`ApplyFlag::Suppressed`]. That is a reading of an example rather than a normative sentence,
//! which is exactly why [`super::effective`] reports the flag it saw alongside every answer, and why
//! the comparison table handed to MJXOFF-122 has a row for it.
//!
//! # The two tables are the same complex type twice
//!
//! `CT_CellXfs` and `CT_CellStyleXfs` are character for character identical — a sequence of `xf`,
//! plus `@count` — and differ only in the local name they stand under and in what they mean. So
//! there is one [`CellFormatTable`] here and a [`CellFormatTableKind`] to say which slot an authored
//! one belongs in. Declaring the type twice would have been two copies of an index-identity
//! discipline to keep in step, which is the duplication [`crate::font`] exists to have avoided once
//! already.
//!
//! Index identity applies to both, and for the same reason it applies to [`super::fonts`]: a cell's
//! `@s` is a *position* in `cellXfs`, so [`CellFormatTable::push`] is the only mutation.

use mjx_ooxml_core::{AttributeError, Interner, Number, RawAttribute, RawName, RawNode};
use mjx_ooxml_types::child_order::STYLESHEET_CELL_FORMAT;
use mjx_ooxml_types::support::OnOff;

use super::cell_format::{CellAlignment, CellProtection};

/// The six aspects of formatting an `xf` decides about, one per `applyX` flag.
///
/// A value of this type names *which* `applyX` is being asked about, so that the resolver in
/// [`super::effective`] is a loop over six aspects rather than six copies of one paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormatAspect {
    /// `@numFmtId`, gated by `@applyNumberFormat`.
    NumberFormat,
    /// `@fontId`, gated by `@applyFont`.
    Font,
    /// `@fillId`, gated by `@applyFill`.
    Fill,
    /// `@borderId`, gated by `@applyBorder`.
    Border,
    /// The `x:alignment` child, gated by `@applyAlignment`.
    Alignment,
    /// The `x:protection` child, gated by `@applyProtection`.
    Protection,
}

impl FormatAspect {
    /// All six, in the order `CT_Xf` declares their `applyX` attributes.
    pub const ALL: [Self; 6] = [
        Self::NumberFormat,
        Self::Font,
        Self::Fill,
        Self::Border,
        Self::Alignment,
        Self::Protection,
    ];

    /// The wire name of the `applyX` attribute that gates this aspect.
    #[must_use]
    pub const fn apply_attribute(self) -> &'static str {
        match self {
            Self::NumberFormat => "applyNumberFormat",
            Self::Font => "applyFont",
            Self::Fill => "applyFill",
            Self::Border => "applyBorder",
            Self::Alignment => "applyAlignment",
            Self::Protection => "applyProtection",
        }
    }

    /// The wire name of the attribute or child the aspect resolves to — `@numFmtId` for a number
    /// format, the `x:alignment` element for an alignment.
    #[must_use]
    pub const fn value_name(self) -> &'static str {
        match self {
            Self::NumberFormat => "numFmtId",
            Self::Font => "fontId",
            Self::Fill => "fillId",
            Self::Border => "borderId",
            Self::Alignment => "alignment",
            Self::Protection => "protection",
        }
    }

    /// Whether the aspect's value is an index into a resource table, rather than an element stated
    /// inline on the `xf`.
    #[must_use]
    pub const fn is_index(self) -> bool {
        matches!(
            self,
            Self::NumberFormat | Self::Font | Self::Fill | Self::Border
        )
    }
}

/// One `applyX` attribute, in **all three** of the states `CT_Xf` gives it.
///
/// The schema declares every `applyX` `use="optional"` with no `default=`, so this is not a
/// convenience over a `bool`: it is the attribute's actual value space. Collapsing
/// [`Unstated`](Self::Unstated) into [`Suppressed`](Self::Suppressed) is a defect no schema
/// validator can see, because both spellings are valid documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApplyFlag {
    /// The attribute is **absent**.
    ///
    /// Not the same as `Suppressed`. §18.8.9's example contrasts a record that "does not express any
    /// apply attributes" — the `Normal` style, which applies — with records that switch aspects off
    /// by writing `applyX="0"`. So this state *participates*; see
    /// [`participates`](Self::participates).
    Unstated,
    /// `applyX` was written and is true (`1`, `true`, or `on`).
    Applied,
    /// `applyX` was written and is false (`0`, `false`, or `off`) — this `xf` does **not**
    /// contribute the aspect, and the layer beneath it does.
    Suppressed,
}

impl ApplyFlag {
    /// The flag for an attribute read through [`mjx_derive`]'s optional grammar: `None` is
    /// [`Unstated`](Self::Unstated), and that is the whole point of the type.
    #[must_use]
    pub const fn from_attribute(value: Option<bool>) -> Self {
        match value {
            None => Self::Unstated,
            Some(true) => Self::Applied,
            Some(false) => Self::Suppressed,
        }
    }

    /// The attribute value this flag came from — `None` for [`Unstated`](Self::Unstated).
    #[must_use]
    pub const fn as_attribute(self) -> Option<bool> {
        match self {
            Self::Unstated => None,
            Self::Applied => Some(true),
            Self::Suppressed => Some(false),
        }
    }

    /// Whether the `xf` carrying this flag contributes the aspect it gates.
    ///
    /// True for [`Applied`](Self::Applied) **and** for [`Unstated`](Self::Unstated); false only for
    /// [`Suppressed`](Self::Suppressed). See this module's documentation for the §18.8.9 example
    /// that says so.
    #[must_use]
    pub const fn participates(self) -> bool {
        !matches!(self, Self::Suppressed)
    }
}

/// `x:xf` (`CT_Xf`, `sml.xsd:3598`) — one master formatting record.
///
/// See the [module documentation](self) for the thirteen attributes and for why `applyX` is
/// `Option<bool>` rather than `bool`.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "numFmtId", codec = Number<u32>, accessor = number_format_id))]
#[xml(attribute(local = "fontId", codec = Number<u32>, accessor = font_index))]
#[xml(attribute(local = "fillId", codec = Number<u32>, accessor = fill_index))]
#[xml(attribute(local = "borderId", codec = Number<u32>, accessor = border_index))]
#[xml(attribute(local = "xfId", codec = Number<u32>, accessor = cell_style_format_index))]
#[xml(attribute(local = "quotePrefix", codec = OnOff, accessor = text_is_quote_prefixed, default = false))]
#[xml(attribute(local = "pivotButton", codec = OnOff, accessor = shows_pivot_button, default = false))]
#[xml(attribute(local = "applyNumberFormat", codec = OnOff, accessor = applies_number_format))]
#[xml(attribute(local = "applyFont", codec = OnOff, accessor = applies_font))]
#[xml(attribute(local = "applyFill", codec = OnOff, accessor = applies_fill))]
#[xml(attribute(local = "applyBorder", codec = OnOff, accessor = applies_border))]
#[xml(attribute(local = "applyAlignment", codec = OnOff, accessor = applies_alignment))]
#[xml(attribute(local = "applyProtection", codec = OnOff, accessor = applies_protection))]
pub struct CellFormat {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "alignment", variant = Alignment, ty = CellAlignment),
        child(local = "protection", variant = Protection, ty = CellProtection)
    )]
    content: Vec<CellFormatContent>,
}

/// One child of [`CellFormat`]: the two modelled members, and `extLst`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellFormatContent {
    /// `x:alignment` (rank 0).
    Alignment(CellAlignment),
    /// `x:protection` (rank 1).
    Protection(CellProtection),
    /// `x:extLst` (rank 2) and anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl CellFormatContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::Alignment(_) => "alignment",
            Self::Protection(_) => "protection",
            Self::Raw(_) => return None,
        })
    }

    /// This child's rank in `CT_Xf`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        STYLESHEET_CELL_FORMAT.rank_of(None, self.local()?)
    }
}

impl CellFormat {
    /// The wire local name this type is written under.
    pub const WIRE_LOCAL: &'static str = "xf";

    /// Builds an `x:xf` with every attribute absent and no children, bound to `prefix` or to the
    /// default namespace.
    ///
    /// Every attribute absent is a meaningful record and not a placeholder: `<xf/>` names font 0,
    /// fill 0, border 0 and number format `General` by omission, and suppresses nothing.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, Self::WIRE_LOCAL),
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

    /// Every child, in document order, including `extLst` and anything else unmodelled.
    #[must_use]
    pub fn content(&self) -> &[CellFormatContent] {
        &self.content
    }

    /// The `applyX` flag that gates `aspect`, in all three of its states.
    ///
    /// # Errors
    /// [`AttributeError`] if the attribute is present but is not one of the spellings `xsd:boolean`
    /// allows.
    pub fn apply_flag(
        &self,
        interner: &Interner,
        aspect: FormatAspect,
    ) -> Result<ApplyFlag, AttributeError> {
        let written = match aspect {
            FormatAspect::NumberFormat => self.applies_number_format(interner),
            FormatAspect::Font => self.applies_font(interner),
            FormatAspect::Fill => self.applies_fill(interner),
            FormatAspect::Border => self.applies_border(interner),
            FormatAspect::Alignment => self.applies_alignment(interner),
            FormatAspect::Protection => self.applies_protection(interner),
        }?;
        Ok(ApplyFlag::from_attribute(written))
    }

    /// The resource-table index this record states for `aspect`, or `None` for an aspect that is
    /// not index-valued ([`FormatAspect::is_index`]) or whose attribute is absent.
    ///
    /// # Errors
    /// [`AttributeError`] if the attribute is present but is not an `xsd:unsignedInt`.
    pub fn resource_index(
        &self,
        interner: &Interner,
        aspect: FormatAspect,
    ) -> Result<Option<u32>, AttributeError> {
        match aspect {
            FormatAspect::NumberFormat => self.number_format_id(interner),
            FormatAspect::Font => self.font_index(interner),
            FormatAspect::Fill => self.fill_index(interner),
            FormatAspect::Border => self.border_index(interner),
            FormatAspect::Alignment | FormatAspect::Protection => Ok(None),
        }
    }

    /// `x:alignment` — how this record says a cell's content sits in it, or `None` when it says
    /// nothing.
    #[must_use]
    pub fn alignment(&self) -> Option<&CellAlignment> {
        self.content.iter().find_map(|item| match item {
            CellFormatContent::Alignment(value) => Some(value),
            _ => None,
        })
    }

    /// `x:protection` — this record's locked and formula-hidden flags, or `None`.
    #[must_use]
    pub fn protection(&self) -> Option<&CellProtection> {
        self.content.iter().find_map(|item| match item {
            CellFormatContent::Protection(value) => Some(value),
            _ => None,
        })
    }

    /// Sets `x:alignment`: `None` removes it; `Some` replaces the existing element **where it is**,
    /// or inserts one at its rank in `CT_Xf`'s `xsd:sequence`.
    pub fn set_alignment(&mut self, value: Option<CellAlignment>) {
        self.replace_or_insert(
            "alignment",
            |item| matches!(item, CellFormatContent::Alignment(_)),
            value.map(CellFormatContent::Alignment),
        );
    }

    /// Sets `x:protection`, with the placement rule of [`set_alignment`](Self::set_alignment).
    pub fn set_protection(&mut self, value: Option<CellProtection>) {
        self.replace_or_insert(
            "protection",
            |item| matches!(item, CellFormatContent::Protection(_)),
            value.map(CellFormatContent::Protection),
        );
    }

    /// Replaces the first child `is_target` accepts, keeping its position; inserts at the schema
    /// rank when there is none; removes it when `value` is `None`.
    fn replace_or_insert(
        &mut self,
        local: &str,
        is_target: impl Fn(&CellFormatContent) -> bool,
        value: Option<CellFormatContent>,
    ) {
        let existing = self.content.iter().position(&is_target);
        match (existing, value) {
            (Some(at), Some(value)) => self.content[at] = value,
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(value)) => {
                let at = STYLESHEET_CELL_FORMAT
                    .insert_index_of_names(self.content.iter().map(CellFormatContent::rank), local);
                self.content.insert(at, value);
                self.empty = false;
            }
            (None, None) => {}
        }
    }
}

/// Which of the two `xf` slots of `CT_Stylesheet` a [`CellFormatTable`] stands in.
///
/// The two complex types are identical; only the local name and the meaning differ. See the
/// [module documentation](self).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellFormatTableKind {
    /// `x:cellXfs` (`CT_CellXfs`, `sml.xsd:3592`) — the records a cell's `@s` indexes.
    CellFormats,
    /// `x:cellStyleXfs` (`CT_CellStyleXfs`, `sml.xsd:3586`) — the records a **named cell style**
    /// names through `cellStyle@xfId`, and which a `cellXfs` record sits on top of through its own
    /// `@xfId`.
    CellStyleFormats,
}

impl CellFormatTableKind {
    /// The wire local name of the slot.
    #[must_use]
    pub const fn wire_local(self) -> &'static str {
        match self {
            Self::CellFormats => "cellXfs",
            Self::CellStyleFormats => "cellStyleXfs",
        }
    }
}

/// `x:cellXfs` or `x:cellStyleXfs` — a table of [`CellFormat`]s, in index order.
///
/// Addressed by position, so [`push`](Self::push) is the only mutation: see [`super::fonts`] for
/// what reordering one of these costs.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count))]
pub struct CellFormatTable {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "xf", variant = Format, ty = CellFormat))]
    content: Vec<CellFormatTableContent>,
}

/// One child of [`CellFormatTable`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellFormatTableContent {
    /// `x:xf`.
    Format(CellFormat),
    /// Anything else — preserved verbatim, in position, and occupying no index.
    Raw(RawNode),
}

impl CellFormatTable {
    /// Builds an empty table for `kind`, bound to `prefix` or to the default namespace.
    ///
    /// The schema declares `xf` `minOccurs="1"` in both tables, so a table with no records is
    /// invalid; it is still constructible, because a caller builds one and then fills it.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>, kind: CellFormatTableKind) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, kind.wire_local()),
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
    pub fn content(&self) -> &[CellFormatTableContent] {
        &self.content
    }

    /// Every `x:xf`, in index order.
    pub fn formats(&self) -> impl Iterator<Item = &CellFormat> + '_ {
        self.content.iter().filter_map(|item| match item {
            CellFormatTableContent::Format(format) => Some(format),
            CellFormatTableContent::Raw(_) => None,
        })
    }

    /// The record at `index` — the number a cell's `@s`, a row's `@s`, a column's `@style` or an
    /// `xf`'s own `@xfId` carries.
    ///
    /// Indexes the **records**, stepping over anything unmodelled between them: a comment between
    /// two `<xf>` elements does not shift the numbering Excel counts in.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&CellFormat> {
        self.formats().nth(index)
    }

    /// How many records the table holds — counted, not read from `@count`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.formats().count()
    }

    /// Whether the table holds no record at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends `format` after the last record, giving it the next index, and updates `@count` when
    /// the file declared one.
    ///
    /// **The only mutation this type offers.** Appending is what keeps index identity: every `@s`,
    /// `@style` and `@xfId` already written in the workbook still names what it named.
    pub fn push(&mut self, interner: &mut Interner, format: CellFormat) {
        self.content.push(CellFormatTableContent::Format(format));
        self.empty = false;
        if self.declared_count(interner).ok().flatten().is_some() {
            let count = u32::try_from(self.len()).unwrap_or(u32::MAX);
            self.set_declared_count(interner, Some(count));
        }
    }
}

#[cfg(test)]
mod tests {
    use mjx_ooxml_core::FromXml;

    use super::*;

    /// Reads one `<xf>` out of markup.
    fn xf(markup: &str) -> (mjx_ooxml_core::RawDocument, CellFormat) {
        let wrapped = format!(
            r#"<xf xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" {markup}"#
        );
        let document = mjx_xml::fidelity::parse(wrapped.as_bytes()).expect("the xf parses");
        let format =
            CellFormat::from_xml(&document.root, &document.interner).expect("the xf reads");
        (document, format)
    }

    /// The three states are three states — the assertion the whole child rests on.
    #[test]
    fn an_absent_apply_flag_is_neither_true_nor_false() {
        let (absent_doc, absent) = xf(r#"fontId="7"/>"#);
        let (true_doc, applied) = xf(r#"fontId="7" applyFont="1"/>"#);
        let (false_doc, suppressed) = xf(r#"fontId="7" applyFont="0"/>"#);

        assert_eq!(
            absent
                .apply_flag(&absent_doc.interner, FormatAspect::Font)
                .expect("the flag reads"),
            ApplyFlag::Unstated
        );
        assert_eq!(
            applied
                .apply_flag(&true_doc.interner, FormatAspect::Font)
                .expect("the flag reads"),
            ApplyFlag::Applied
        );
        assert_eq!(
            suppressed
                .apply_flag(&false_doc.interner, FormatAspect::Font)
                .expect("the flag reads"),
            ApplyFlag::Suppressed
        );

        assert_ne!(ApplyFlag::Unstated, ApplyFlag::Suppressed);
        assert_eq!(ApplyFlag::Unstated.as_attribute(), None);
        assert_eq!(ApplyFlag::Suppressed.as_attribute(), Some(false));
        assert!(ApplyFlag::Unstated.participates());
        assert!(ApplyFlag::Applied.participates());
        assert!(!ApplyFlag::Suppressed.participates());
    }

    /// Every aspect reads its own flag, and no two aspects read the same attribute.
    ///
    /// Written against the transcription mistake a six-armed `match` invites: one arm pointing at
    /// the neighbouring accessor passes every single-aspect test.
    #[test]
    fn each_aspect_reads_the_attribute_that_gates_it() {
        for aspect in FormatAspect::ALL {
            let markup = format!(r#"{}="0"/>"#, aspect.apply_attribute());
            let (document, format) = xf(&markup);
            for other in FormatAspect::ALL {
                let flag = format
                    .apply_flag(&document.interner, other)
                    .expect("the flag reads");
                let expected = if other == aspect {
                    ApplyFlag::Suppressed
                } else {
                    ApplyFlag::Unstated
                };
                assert_eq!(
                    flag,
                    expected,
                    "with only @{} written, asking about {other:?} must answer {expected:?}",
                    aspect.apply_attribute()
                );
            }
        }
    }

    /// `alignment` lands before `protection`, whichever order a caller sets them in.
    #[test]
    fn the_two_children_are_placed_at_their_schema_ranks() {
        assert_eq!(STYLESHEET_CELL_FORMAT.symbol, "CT_Xf");
        assert_eq!(STYLESHEET_CELL_FORMAT.slots.len(), 3);

        let markup = r#"<xf xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><extLst/></xf>"#;
        let mut document = mjx_xml::fidelity::parse(markup.as_bytes()).expect("the xf parses");
        let mut format =
            CellFormat::from_xml(&document.root, &document.interner).expect("the xf reads");

        let protection = CellProtection::new(&mut document.interner, None);
        format.set_protection(Some(protection));
        let alignment = CellAlignment::new(&mut document.interner, None);
        format.set_alignment(Some(alignment));

        let locals: Vec<Option<&'static str>> = format
            .content()
            .iter()
            .map(CellFormatContent::local)
            .collect();
        assert_eq!(
            locals,
            vec![Some("alignment"), Some("protection"), None],
            "`alignment` is rank 0 and `protection` rank 1, and `extLst` stays last"
        );
    }

    /// Both slots are the same complex type, and an authored table knows which name it wears.
    #[test]
    fn the_two_tables_differ_only_in_their_local_name() {
        assert_eq!(CellFormatTableKind::CellFormats.wire_local(), "cellXfs");
        assert_eq!(
            CellFormatTableKind::CellStyleFormats.wire_local(),
            "cellStyleXfs"
        );

        let mut interner = Interner::default();
        for kind in [
            CellFormatTableKind::CellFormats,
            CellFormatTableKind::CellStyleFormats,
        ] {
            let table = CellFormatTable::new(&mut interner, None, kind);
            assert_eq!(
                interner.resolve(table.element_name().local),
                kind.wire_local()
            );
            assert!(table.is_empty());
        }
    }

    /// A record between two comments still has the index the file counted it at.
    #[test]
    fn unmodelled_nodes_occupy_no_index() {
        let markup = concat!(
            r#"<cellXfs xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="2">"#,
            r#"<!-- first --><xf fontId="1"/><!-- between --><xf fontId="2"/><!-- last -->"#,
            "</cellXfs>"
        );
        let document = mjx_xml::fidelity::parse(markup.as_bytes()).expect("the table parses");
        let table =
            CellFormatTable::from_xml(&document.root, &document.interner).expect("the table reads");

        assert_eq!(table.len(), 2);
        assert_eq!(table.content().len(), 5, "three comments are preserved");
        assert_eq!(
            table
                .get(0)
                .expect("index 0")
                .font_index(&document.interner)
                .expect("the index reads"),
            Some(1)
        );
        assert_eq!(
            table
                .get(1)
                .expect("index 1")
                .font_index(&document.interner)
                .expect("the index reads"),
            Some(2)
        );
        assert!(table.get(2).is_none());
    }
}
