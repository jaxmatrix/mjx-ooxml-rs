//! Data validation: `CT_DataValidations` and `CT_DataValidation` — *a constraint over a range,
//! expressed with formulas we never evaluate*.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_DataValidations` | 2571 | `x:dataValidations` (rank **17** of `CT_Worksheet`) |
//! | `CT_DataValidation` | 2581 | `x:dataValidations/dataValidation` |
//!
//! # Why this sits beside [`crate::features::filters`] and not inside it
//!
//! MJXOFF-123 pairs the two because they have the same shape: a range-scoped rule carrying formula
//! text and an operator, and neither is ever applied. They are two files because the reason the
//! ticket gives for the *location* — *"D15's `CT_Table` embeds `autoFilter` and `sortState`
//! directly; D18's pivot tables reference the same filter types"* — is about the filter cluster
//! alone. `filters.rs` is therefore exactly what a sibling consumes, and data validation, which
//! nothing else in Phase D reaches, is its neighbour rather than a lodger in it.
//!
//! # The list source is a formula, and it is never resolved
//!
//! **The single most important rule in this file.** `dataValidation type="list"` states its source in
//! `formula1`, and that source is one of two things:
//!
//! * a **range reference** — `$A$1:$A$9`, or `Sheet2!$A$1:$A$9`, or a defined name; or
//! * a **literal list** — `"Low,Medium,High"`, quotes and all, as one `ST_Formula` string.
//!
//! Nothing here turns the first into the second. Resolving `$A$1:$A$9` into the nine values that
//! range holds would:
//!
//! * change the file's bytes for a caller who edited an unrelated cell;
//! * freeze a list that Excel recomputes every time the drop-down opens, so the workbook would stop
//!   tracking its own source; and
//! * need a cross-sheet read this tier cannot even perform for `Sheet2!`.
//!
//! MJXOFF-115's formula-as-text contract governs it, exactly as it governs a cell's `<f>`:
//! [`FormulaElement`] carries the text a producer wrote and has no way to produce different bytes.
//! `crates/mjx-sml/tests/validation_and_filters.rs` pins it with a test that reaches the one door
//! that drops a validation's verbatim bytes — see that file's own documentation for why a
//! byte-identity gate could not.
//!
//! # The message strings are `ST_Xstring` and are escaped exactly as read
//!
//! `@errorTitle`, `@error`, `@promptTitle` and `@prompt` are free text a user typed, so one may
//! legitimately contain `<`, `&`, or a run of markup-looking characters. They are read through the
//! same [`Text`] codec as every other `ST_Xstring` in this crate, which decodes
//! on read and does not rewrite: an attribute nobody assigned to keeps its position, its prefix, its
//! quote character and its entity spellings.
//!
//! # The `x14` extension namespace
//!
//! A validation whose formula references **another sheet** is not expressible in `CT_DataValidation`
//! at all — the Transitional schema has no room for it — so Excel writes a second, parallel
//! `x14:dataValidations` inside the worksheet's own `extLst`, and leaves a degraded copy here. That
//! extension is **not modelled**: it lands in the worksheet's unmodelled bucket, keeps its prefix,
//! its `uri` and its bytes, and comes back out of an unrelated edit unchanged. Not modelling it is a
//! scope decision; losing it would be a defect, and the suite asserts it survives.

use mjx_ooxml_core::{
    Enumeration, Interner, Number, RawAttribute, RawElement, RawName, RawNode, Text, ToXml,
};
use mjx_ooxml_types::child_order::DATA_VALIDATION;
use mjx_ooxml_types::spreadsheetml::{
    DataValidationErrorStyle, DataValidationImeMode, DataValidationOperator, DataValidationType,
};
use mjx_ooxml_types::support::OnOff;

use crate::address::CellRangeList;
use crate::formula::FormulaElement;
use crate::worksheet::rebuild_element;

/// `x:dataValidation` (`CT_DataValidation`, `sml.xsd:2581`) — one validation rule over one range
/// list.
///
/// **`ST_`/`CT_` symbol:** `CT_DataValidation`. Wire element: `dataValidation`.
///
/// `@sqref` is the only `use="required"` attribute and is an `ST_Sqref` — MJXOFF-93's
/// [`CellRangeList`] — because one rule commonly covers several disjoint blocks.
///
/// `@type` decides what the rest mean. A `list` validation reads `formula1` as its source and
/// ignores `@operator`; a `whole` or `decimal` one reads `@operator` and then one or two formulas
/// (`between` and `notBetween` take two, the other six take one); a `custom` one is a single boolean
/// expression in `formula1`. Nothing here enforces any of that — a `@operator` on a `list`
/// validation is preserved and reported, because it is what the file says.
///
/// `@showDropDown` keeps the wire token's own name. §18.3.1.32 describes it as whether to display
/// the in-cell drop-down of a `list` validation, while producers are widely observed to write `1`
/// on the rules whose drop-down is *hidden*. This library reports the attribute and interprets
/// neither reading, because renaming an accessor on the strength of observed behaviour is the guess
/// the naming convention forbids.
///
/// `@imeMode` is the Input Method Editor state Excel forces while the cell is selected — eleven
/// values, from `noControl` through `fullKatakana` to `halfHangul`. The ticket says twelve; the
/// Transitional schema declares eleven, and
/// [`DataValidationImeMode`] is generated from
/// it.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(
    local = "type",
    codec = Enumeration<DataValidationType>,
    accessor = kind,
    default = DataValidationType::None
))]
#[xml(attribute(
    local = "errorStyle",
    codec = Enumeration<DataValidationErrorStyle>,
    accessor = error_style,
    default = DataValidationErrorStyle::Stop
))]
#[xml(attribute(
    local = "imeMode",
    codec = Enumeration<DataValidationImeMode>,
    accessor = input_method_mode,
    default = DataValidationImeMode::NoControl
))]
#[xml(attribute(
    local = "operator",
    codec = Enumeration<DataValidationOperator>,
    accessor = operator,
    default = DataValidationOperator::Between
))]
#[xml(attribute(local = "allowBlank", codec = OnOff, accessor = allows_blank, default = false))]
#[xml(attribute(
    local = "showDropDown",
    codec = OnOff,
    accessor = shows_drop_down,
    default = false
))]
#[xml(attribute(
    local = "showInputMessage",
    codec = OnOff,
    accessor = shows_input_message,
    default = false
))]
#[xml(attribute(
    local = "showErrorMessage",
    codec = OnOff,
    accessor = shows_error_message,
    default = false
))]
#[xml(attribute(local = "errorTitle", codec = Text, accessor = error_title))]
#[xml(attribute(local = "error", codec = Text, accessor = error_message))]
#[xml(attribute(local = "promptTitle", codec = Text, accessor = prompt_title))]
#[xml(attribute(local = "prompt", codec = Text, accessor = prompt_message))]
#[xml(attribute(local = "sqref", codec = Enumeration<CellRangeList>, accessor = ranges, required))]
pub struct DataValidation {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "formula1", variant = FirstFormula, ty = FormulaElement),
        child(local = "formula2", variant = SecondFormula, ty = FormulaElement)
    )]
    content: Vec<DataValidationContent>,
}

/// One child of [`DataValidation`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataValidationContent {
    /// `x:formula1` (rank 0) — the bound, the expression, or the list source.
    FirstFormula(FormulaElement),
    /// `x:formula2` (rank 1) — the upper bound of a `between`/`notBetween` comparison.
    SecondFormula(FormulaElement),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl DataValidationContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::FirstFormula(_) => "formula1",
            Self::SecondFormula(_) => "formula2",
            Self::Raw(_) => return None,
        })
    }

    /// This child's rank in `CT_DataValidation`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        DATA_VALIDATION.rank_of(None, self.local()?)
    }
}

/// Declares one of the two formula slots: a borrowing getter, a mutable getter, and a setter that
/// replaces the existing element in place or inserts a new one at its rank in the sequence.
macro_rules! formula_slot {
    ($getter:ident, $getter_mut:ident, $setter:ident, $variant:ident, $local:literal, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<&FormulaElement> {
            self.content.iter().find_map(|item| match item {
                DataValidationContent::$variant(formula) => Some(formula),
                _ => None,
            })
        }

        #[doc = concat!("`x:", $local, "`, mutably — `None` if the rule writes none.")]
        pub fn $getter_mut(&mut self) -> Option<&mut FormulaElement> {
            self.content.iter_mut().find_map(|item| match item {
                DataValidationContent::$variant(formula) => Some(formula),
                _ => None,
            })
        }

        #[doc = concat!("Sets `x:", $local, "`: `None` removes it; `Some` replaces the existing \
            element **where it is**, or inserts one at its rank in `CT_DataValidation`'s \
            `xsd:sequence`.\n\nThe text is taken exactly as given. Nothing resolves a range \
            reference, re-escapes a quoted list, or normalises anything; see this module's own \
            documentation.")]
        pub fn $setter(&mut self, formula: Option<FormulaElement>) {
            self.replace_or_insert(
                $local,
                |item| matches!(item, DataValidationContent::$variant(_)),
                formula.map(DataValidationContent::$variant),
            );
        }
    };
}

impl DataValidation {
    /// Builds an `x:dataValidation` with every attribute absent, bound to `prefix` or to the default
    /// namespace.
    ///
    /// `@sqref` is `use="required"` and is **not** set here: a range invented on a caller's behalf
    /// would constrain cells they never named. Build the rule and then state its range.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "dataValidation"),
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

    /// Every child, in document order, including anything unmodelled.
    #[must_use]
    pub fn content(&self) -> &[DataValidationContent] {
        &self.content
    }

    formula_slot!(
        first_formula,
        first_formula_mut,
        set_first_formula,
        FirstFormula,
        "formula1",
        "`x:formula1` — the bound a comparison starts at, a `custom` rule's whole expression, or a \
         `list` rule's **source**, which is a range reference or a quoted literal list and is never \
         resolved into values."
    );
    formula_slot!(
        second_formula,
        second_formula_mut,
        set_second_formula,
        SecondFormula,
        "formula2",
        "`x:formula2` — the upper bound of a `between` or `notBetween` comparison. Absent for every \
         other operator, and for `list` and `custom` rules."
    );

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                DataValidationContent::FirstFormula(formula)
                | DataValidationContent::SecondFormula(formula) => {
                    RawNode::Element(formula.as_raw_element())
                }
                DataValidationContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }

    /// Replaces the first child `is_target` accepts, keeping its position; inserts at the schema
    /// rank when there is none; removes it when `value` is `None`.
    fn replace_or_insert(
        &mut self,
        local: &str,
        is_target: impl Fn(&DataValidationContent) -> bool,
        value: Option<DataValidationContent>,
    ) {
        let existing = self.content.iter().position(&is_target);
        match (existing, value) {
            (Some(at), Some(value)) => self.content[at] = value,
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(value)) => {
                let at = DATA_VALIDATION.insert_index_of_names(
                    self.content.iter().map(DataValidationContent::rank),
                    local,
                );
                self.content.insert(at, value);
                self.empty = false;
            }
            (None, None) => {}
        }
    }
}

impl ToXml for DataValidation {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:dataValidations` (`CT_DataValidations`, `sml.xsd:2571`) — every validation rule on the sheet.
///
/// **`ST_`/`CT_` symbol:** `CT_DataValidations`. Wire element: `dataValidations`, rank **17** of
/// `CT_Worksheet`.
///
/// `@count` is a **cached** count of the `dataValidation` children, exactly as `mergeCells@count`
/// and `dimension@ref` are cached. [`set_count`](Self::set_count) exists and nothing calls it
/// implicitly: a stale count is the producer's, and rewriting one on read would change a file this
/// library was only asked to look at. [`len`](Self::len) counts the children and is what a caller
/// should believe.
///
/// `@xWindow` and `@yWindow` are the screen coordinates of the *Data Validation* dialog the last time
/// a user opened it, and `@disablePrompts` suppresses every input message on the sheet at once.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(
    local = "disablePrompts",
    codec = OnOff,
    accessor = disables_prompts,
    default = false
))]
#[xml(attribute(local = "xWindow", codec = Number<u32>, accessor = dialog_x))]
#[xml(attribute(local = "yWindow", codec = Number<u32>, accessor = dialog_y))]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = count))]
pub struct DataValidations {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "dataValidation", variant = Rule, ty = DataValidation))]
    content: Vec<DataValidationsContent>,
}

/// One child of [`DataValidations`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataValidationsContent {
    /// `x:dataValidation` — one rule.
    Rule(DataValidation),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl DataValidations {
    /// Builds an empty `x:dataValidations`, bound to `prefix` or to the default namespace.
    ///
    /// The schema declares `dataValidation` `minOccurs="1"`, so an empty element is invalid markup;
    /// it is still constructible, because a caller builds one and then fills it.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "dataValidations"),
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

    /// Every child, in document order, including anything unmodelled.
    #[must_use]
    pub fn content(&self) -> &[DataValidationsContent] {
        &self.content
    }

    /// Every `x:dataValidation`, in document order.
    pub fn rules(&self) -> impl Iterator<Item = &DataValidation> + '_ {
        self.content.iter().filter_map(|item| match item {
            DataValidationsContent::Rule(rule) => Some(rule),
            DataValidationsContent::Raw(_) => None,
        })
    }

    /// How many rules the element holds — counted, not read from `@count`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules().count()
    }

    /// Whether the element holds no rule at all, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `index`-th `x:dataValidation`, mutably.
    pub fn rule_mut(&mut self, index: usize) -> Option<&mut DataValidation> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                DataValidationsContent::Rule(rule) => Some(rule),
                DataValidationsContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends a rule after the ones already present.
    ///
    /// **`@count` is not touched.** It is the producer's cached number; a caller who wants it
    /// consistent calls [`set_count`](Self::set_count) with [`len`](Self::len).
    pub fn push_rule(&mut self, rule: DataValidation) {
        self.content.push(DataValidationsContent::Rule(rule));
        self.empty = false;
    }

    /// Removes the `index`-th `x:dataValidation` and returns it, or `None` when there are fewer.
    pub fn remove_rule(&mut self, index: usize) -> Option<DataValidation> {
        let at = self
            .content
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, DataValidationsContent::Rule(_)))
            .map(|(at, _)| at)
            .nth(index)?;
        match self.content.remove(at) {
            DataValidationsContent::Rule(rule) => Some(rule),
            DataValidationsContent::Raw(_) => unreachable!("the position was filtered on `Rule`"),
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                DataValidationsContent::Rule(rule) => RawNode::Element(rule.as_raw_element()),
                DataValidationsContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for DataValidations {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// The authoring vocabulary
// -----------------------------------------------------------------------------------------------

/// A validation rule to author: plain data, no interner, no lifetime.
///
/// The same shape [`ConditionalRuleSpec`](crate::ConditionalRuleSpec) has and for the same reason —
/// see [`crate::features::conditional_specs`]. Every field maps to exactly one attribute or one
/// child of `CT_DataValidation`, and a field left at its schema default writes no attribute at all,
/// so a rule authored here is the smallest markup that says what the caller said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataValidationSpec {
    /// `@sqref` — the ranges the rule constrains. `use="required"`.
    pub ranges: CellRangeList,
    /// `@type`.
    pub kind: DataValidationType,
    /// `@operator`. Meaningful for the comparison kinds; ignored by Excel for `list` and `custom`,
    /// and written anyway when it is not the schema default, because a caller who states it means it.
    pub operator: DataValidationOperator,
    /// `x:formula1` — the bound, the expression, or a `list` rule's source.
    ///
    /// **Written exactly as given.** A range reference stays a range reference; see this module's
    /// own documentation for why nothing resolves one.
    pub first_formula: Option<String>,
    /// `x:formula2` — the upper bound of a `between`/`notBetween` comparison.
    pub second_formula: Option<String>,
    /// `@allowBlank`.
    pub allows_blank: bool,
    /// `@showInputMessage`.
    pub shows_input_message: bool,
    /// `@showErrorMessage`.
    pub shows_error_message: bool,
    /// `@errorStyle`.
    pub error_style: DataValidationErrorStyle,
    /// `@errorTitle` and `@error`.
    pub error: Option<(String, String)>,
    /// `@promptTitle` and `@prompt`.
    pub prompt: Option<(String, String)>,
}

impl DataValidationSpec {
    /// A rule of `kind` over `ranges` with every optional attribute at its schema default.
    #[must_use]
    pub fn new(ranges: CellRangeList, kind: DataValidationType) -> Self {
        Self {
            ranges,
            kind,
            operator: DataValidationOperator::Between,
            first_formula: None,
            second_formula: None,
            allows_blank: false,
            shows_input_message: false,
            shows_error_message: false,
            error_style: DataValidationErrorStyle::Stop,
            error: None,
            prompt: None,
        }
    }

    /// The drop-down list a caller almost always wants: `type="list"` over `ranges`, sourced from
    /// `source`, refusing anything else.
    ///
    /// `source` is `ST_Formula` text and goes through unchanged. Both spellings are legal and
    /// neither is converted into the other:
    ///
    /// * a **range reference** — `$A$1:$A$9`, or `Sheet2!$A$1:$A$9`, or a defined name; or
    /// * a **quoted literal list** — `"\"Low,Medium,High\""`, the outer quotes included, which is
    ///   what Excel itself writes for a list typed into the dialog.
    #[must_use]
    pub fn list(ranges: CellRangeList, source: impl Into<String>) -> Self {
        Self {
            first_formula: Some(source.into()),
            shows_error_message: true,
            ..Self::new(ranges, DataValidationType::List)
        }
    }

    /// A `type="custom"` rule whose `formula1` is a boolean expression. Never evaluated.
    #[must_use]
    pub fn custom(ranges: CellRangeList, expression: impl Into<String>) -> Self {
        Self {
            first_formula: Some(expression.into()),
            shows_error_message: true,
            ..Self::new(ranges, DataValidationType::Custom)
        }
    }

    /// A two-formula comparison — the shape `between` and `notBetween` take.
    #[must_use]
    pub fn between(
        ranges: CellRangeList,
        kind: DataValidationType,
        lower: impl Into<String>,
        upper: impl Into<String>,
    ) -> Self {
        Self {
            first_formula: Some(lower.into()),
            second_formula: Some(upper.into()),
            shows_error_message: true,
            ..Self::new(ranges, kind)
        }
    }

    /// Sets the error title and body, in builder style. Implies `@showErrorMessage`.
    #[must_use]
    pub fn with_error(mut self, title: impl Into<String>, message: impl Into<String>) -> Self {
        self.error = Some((title.into(), message.into()));
        self.shows_error_message = true;
        self
    }

    /// Sets the input-prompt title and body, in builder style. Implies `@showInputMessage`.
    #[must_use]
    pub fn with_prompt(mut self, title: impl Into<String>, message: impl Into<String>) -> Self {
        self.prompt = Some((title.into(), message.into()));
        self.shows_input_message = true;
        self
    }

    /// Builds the `x:dataValidation` this describes, interning its names into `interner`.
    #[must_use]
    pub fn build(&self, interner: &mut Interner, prefix: Option<&str>) -> DataValidation {
        // The attributes are set in `CT_DataValidation`'s own declaration order — `type`,
        // `errorStyle`, `imeMode`, `operator`, the four flags, the four message strings, `sqref` —
        // so an authored rule reads the way the schema lists it and the way Excel writes one.
        let mut rule = DataValidation::new(interner, prefix);
        if self.kind != DataValidationType::None {
            rule.set_kind(interner, Some(self.kind));
        }
        if self.error_style != DataValidationErrorStyle::Stop {
            rule.set_error_style(interner, Some(self.error_style));
        }
        if self.operator != DataValidationOperator::Between {
            rule.set_operator(interner, Some(self.operator));
        }
        if self.allows_blank {
            rule.set_allows_blank(interner, Some(true));
        }
        if self.shows_input_message {
            rule.set_shows_input_message(interner, Some(true));
        }
        if self.shows_error_message {
            rule.set_shows_error_message(interner, Some(true));
        }
        if let Some((title, message)) = &self.error {
            rule.set_error_title(interner, Some(title.as_str()));
            rule.set_error_message(interner, Some(message.as_str()));
        }
        if let Some((title, message)) = &self.prompt {
            rule.set_prompt_title(interner, Some(title.as_str()));
            rule.set_prompt_message(interner, Some(message.as_str()));
        }
        rule.set_ranges(interner, self.ranges.clone());
        if let Some(text) = &self.first_formula {
            let formula = FormulaElement::new(interner, prefix, "formula1", text.as_str());
            rule.set_first_formula(Some(formula));
        }
        if let Some(text) = &self.second_formula {
            let formula = FormulaElement::new(interner, prefix, "formula2", text.as_str());
            rule.set_second_formula(Some(formula));
        }
        rule
    }
}

// -----------------------------------------------------------------------------------------------
// The data-validation surface on the worksheet
// -----------------------------------------------------------------------------------------------

impl crate::WorksheetPart {
    /// Every `x:dataValidation` of the sheet, in document order.
    ///
    /// An empty iterator for a worksheet with no `x:dataValidations` element.
    pub fn data_validation_rules(&self) -> impl Iterator<Item = &DataValidation> + '_ {
        self.data_validations()
            .into_iter()
            .flat_map(DataValidations::rules)
    }

    /// Every rule whose `@sqref` covers `reference`, in document order.
    ///
    /// **Reporting, never enforcing.** This says which rules *claim* the cell; whether the cell's
    /// current value satisfies one is a question only a calculation engine could answer, and
    /// [`crate::formula`] records that there is not going to be one.
    ///
    /// # Errors
    /// [`SmlError::Model`](crate::SmlError::Model) if a rule writes no `@sqref` or writes one that
    /// will not parse — neither of which a coverage question can be answered around.
    pub fn data_validations_for(
        &self,
        reference: crate::CellReference,
    ) -> Result<Vec<&DataValidation>, crate::SmlError> {
        let mut covering = Vec::new();
        for rule in self.data_validation_rules() {
            let ranges = rule
                .ranges(self.interner())
                .map_err(mjx_ooxml_core::FromXmlError::from)?;
            if ranges.contains(reference) {
                covering.push(rule);
            }
        }
        Ok(covering)
    }

    /// Appends `rule` to `x:dataValidations`, creating the element at rank 17 of `CT_Worksheet`'s
    /// sequence when the worksheet has none.
    ///
    /// `@count` is left exactly as it stands: it is the producer's cached number, and this library
    /// does not rewrite a cache it was not asked about. See [`DataValidations`].
    pub fn add_data_validation(&mut self, rule: DataValidation) {
        if self.data_validations().is_none() {
            let prefix = self.element_prefix().map(str::to_owned);
            let block = DataValidations::new(self.interner_mut(), prefix.as_deref());
            self.set_data_validations(Some(block));
        }
        self.data_validations_mut()
            .expect("the dataValidations element was just ensured")
            .push_rule(rule);
    }

    /// Removes the `index`-th `x:dataValidation`, reporting whether there was one.
    ///
    /// When the last rule goes, the whole `x:dataValidations` element goes with it: the schema
    /// declares `dataValidation` `minOccurs="1"`, so an empty one is markup no validator accepts.
    pub fn remove_data_validation(&mut self, index: usize) -> bool {
        let Some(block) = self.data_validations_mut() else {
            return false;
        };
        if block.remove_rule(index).is_none() {
            return false;
        }
        if block.is_empty() {
            self.set_data_validations(None);
        }
        true
    }
}
