//! Fields (`w:fldSimple`/`w:fldChar`), hyperlink attributes (typed on
//! [`super::body::Hyperlink`] itself — see that type's own doc comment), and form fields
//! (`w:ffData`).
//!
//! # Two wire forms, one read model
//!
//! WordprocessingML writes a field two different ways. `w:fldSimple` (`CT_SimpleField`) is
//! self-contained: its instruction is the `instr` attribute, its cached result is its own child
//! content. The `begin`/`separate`/`end` form spreads the same two things across sibling runs in
//! one paragraph: a run holding `<w:fldChar w:fldCharType="begin"/>`, then run(s) of
//! `w:instrText` (the instruction), then a run holding `<w:fldChar w:fldCharType="separate"/>`
//! (optional — a field with no cached result omits it, which is legal, not malformed), then the
//! cached-result runs, then a run holding `<w:fldChar w:fldCharType="end"/>`. [`Field`] is the one
//! read model over both: [`Field::instruction`] and [`Field::cached_result`] are always distinct,
//! regardless of which form produced the value — [`Field::form`] says which one it was.
//!
//! # Nesting is paired with a stack, not counted
//!
//! Fields nest: a `TOC` field's cached result can itself contain `PAGEREF` fields, each with its
//! own `begin`/`separate`/`end`. [`parse_top`]/[`parse_complex`] below pair markers with an
//! explicit recursive-descent stack (`nested` fields are parsed *inside* their parent's own scan,
//! consuming their own `begin`...`end` span before the parent's scan resumes) rather than by
//! counting `begin`s and `end`s — a counting implementation cannot tell "the third `begin`" from
//! "a `begin` nested two deep", so it mis-scopes which `w:instrText` belongs to which field the
//! moment two fields nest. It also cannot tell a well-formed nested pair from an unbalanced one:
//! counting three `begin`s and two `end`s still "balances" three separate top-level fields against
//! two, silently swallowing the defect nesting-aware pairing reports as
//! [`crate::DocxError::UnbalancedField`]. `tests/fields.rs`'s nested-`TOC` fixture and its
//! counting-mutation prove this directly.
//!
//! # A typed error, not the prose's own "reinterpret as plain text"
//!
//! ECMA-376 Part 1 §17.16.18 describes what a *rendering* application should do with an unclosed
//! complex field, and it is not an error: *"If a complex field is not closed before the end of a
//! document story, then no field shall be generated and each individual run shall be processed as
//! if the field characters did not exist (i.e. the contents of all field code run content shall
//! not be displayed, and the field results shall be displayed as literal text)."* That is a
//! rendering fallback, not a structural one — reinterpreting an unbalanced marker sequence as "not
//! a field" is itself a form of evaluating field semantics, which this ticket puts out of scope
//! ("Evaluating or refreshing a field"). [`parse_top`]/[`parse_complex`] instead report
//! [`crate::DocxError::UnbalancedField`], matching this ticket's own "Done when" ("An unbalanced
//! field sequence returns a typed error") — a deliberate choice for a structural editor, not an
//! oversight of the prose above.
//!
//! # Instruction and cached result are never the same accessor
//!
//! An instruction (`w:instrText`) is a field's *source* — opaque, uninterpreted beyond splitting
//! the field name from its arguments ([`Field::field_name`]/[`Field::arguments`]) — and a cached
//! result is what Word last computed and will overwrite on refresh. Conflating them either breaks
//! the field (the instruction is rewritten) or writes a value Word discards (the cached result is
//! rewritten, but nothing re-evaluates it). [`crate::Document::set_field_instruction`] and
//! [`crate::Document::set_field_cached_result_text`] are two separate methods for exactly this
//! reason, and each refuses (rather than silently destroying) a zone that itself contains a nested
//! field — collapsing a nested field's own markup to plain text would corrupt it.
//!
//! # Instruction text splits across runs arbitrarily, and is never rewritten unless touched
//!
//! `{ HYPER` + `LINK "http://…" }` is a legal encoding of one instruction spread across two
//! `w:instrText` runs. [`parse_complex`] concatenates every `w:instrText` between a field's own
//! `begin` and its own `separate` (or `end`, when there is no `separate`) for reading. Nothing here
//! ever touches a field's underlying runs unless a caller calls
//! [`crate::Document::set_field_instruction`]/`set_field_cached_result_text` on that specific
//! field — every other field, and every other part, keeps its original bytes (the same
//! copy-on-write contract [`mjx_ooxml_core::ToXml::write_back`] gives every other typed edit in
//! this crate).
//!
//! # Form fields
//!
//! `w:ffData` (inside a `begin` `w:fldChar`) carries a form field's name, help/status text, entry
//! and exit macros, enabled/calc-on-exit flags, and one of a checkbox, drop-down list or text input.
//! Four of its members are length-bounded strings, not enumerations (`ST_FFName` maxLength 65,
//! `ST_FFHelpTextVal` 256, `ST_FFStatusTextVal` 140, `ST_MacroName` 33) — reading never rejects an
//! over-long value (an untrusted file's own violation of its own schema is preserved, not
//! corrected), but every *setter* this module exposes for one of the four refuses an over-long
//! value with [`crate::DocxError::ValueTooLong`] rather than writing schema-invalid markup. The raw,
//! unchecked setter the [`mjx_derive::XmlAttributes`] derive also generates for each stays available
//! (needed to preserve an already-over-long value's own bytes untouched when nothing edits it), but
//! is not the path a caller reaching for "set the form field's name" should use.

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, Interner, Number, RawAttribute, RawElement, RawName,
    RawNode, Text as TextCodec, ToXml,
};
use mjx_ooxml_types::child_order::{
    ChildOrder, FORM_FIELD_CHECK_BOX, FORM_FIELD_DROP_DOWN_LIST, FORM_FIELD_TEXT_INPUT,
};
use mjx_ooxml_types::shared::UnsignedDecimalNumber;
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::wordprocessingml::{
    FieldCharacterType, FormFieldTextType, HelpOrStatusTextType,
};

use crate::error::DocxError;

use super::body::{wml_name, ParagraphContent, Run, RunInnerContent, Text, Unmodeled};
use super::paragraph_properties::DecimalNumberValue;
use super::run_properties::{HalfPointMeasureValue, Toggle};

// =================================================================================================
// w:fldChar (CT_FldChar)
// =================================================================================================

/// `w:fldChar` (`CT_FldChar`, "Complex Field Character", §17.16.18) — one marker (`begin`,
/// `separate` or `end`) of the complex field form, plus whatever it carries: a `begin` marker's own
/// `ffData` is where a form field's definition lives.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "fldCharType", prefix = "w", codec = Enumeration<FieldCharacterType>, accessor = kind, required))]
#[xml(attribute(local = "fldLock", prefix = "w", codec = OnOff, accessor = locked, default = false))]
#[xml(attribute(local = "dirty", prefix = "w", codec = OnOff, accessor = dirty, default = false))]
pub struct FieldCharacter {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "fldData", variant = FieldData, ty = Text),
        child(local = "ffData", variant = FormFieldData, ty = FormFieldData),
        child(local = "numberingChange", variant = NumberingChange, ty = Unmodeled)
    )]
    content: Vec<FieldCharacterContent>,
}

/// One ordered child of a [`FieldCharacter`]: `CT_FldChar`'s own choice of at most one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldCharacterContent {
    /// `w:fldData` (`CT_Text`) — legacy pre-computed field data; rarely written by modern Word.
    FieldData(Text),
    /// `w:ffData` (`CT_FFData`) — this child's own type; present only on the `begin` marker of a
    /// form field.
    FormFieldData(FormFieldData),
    /// `w:numberingChange` (`CT_TrackChangeNumbering`) — tracked-change numbering; MJXOFF-126 owns
    /// revision semantics, kept opaque here.
    NumberingChange(Unmodeled),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl FieldCharacter {
    /// Builds a new, bare `w:fldChar` of `kind` — no `ffData`, no `fldLock`/`dirty` stated.
    #[must_use]
    pub(crate) fn new(interner: &mut Interner, kind: FieldCharacterType) -> Self {
        let mut value = Self {
            name: wml_name(interner, "fldChar"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        value.set_kind(interner, kind);
        value
    }

    /// This marker's own `w:ffData` (`CT_FFData`), or `None` if it carries none — the common case
    /// for every marker except a form field's `begin`.
    #[must_use]
    pub fn form_field_data(&self) -> Option<&FormFieldData> {
        self.content.iter().find_map(|item| match item {
            FieldCharacterContent::FormFieldData(data) => Some(data),
            _ => None,
        })
    }

    /// [`FieldCharacter::form_field_data`], mutably.
    pub fn form_field_data_mut(&mut self) -> Option<&mut FormFieldData> {
        self.content.iter_mut().find_map(|item| match item {
            FieldCharacterContent::FormFieldData(data) => Some(data),
            _ => None,
        })
    }

    /// Sets (replaces) or removes this marker's own `w:ffData`.
    pub fn set_form_field_data(&mut self, value: Option<FormFieldData>) {
        let at = self
            .content
            .iter()
            .position(|item| matches!(item, FieldCharacterContent::FormFieldData(_)));
        match (at, value) {
            (Some(at), Some(value)) => {
                self.content[at] = FieldCharacterContent::FormFieldData(value)
            }
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(value)) => self
                .content
                .push(FieldCharacterContent::FormFieldData(value)),
            (None, None) => {}
        }
    }
}

// =================================================================================================
// w:ffData (CT_FFData) and its twelve members
// =================================================================================================

/// `w:ffData` (`CT_FFData`, "Form Field Properties") — a form field's name, help/status text,
/// macros, enabled/calc-on-exit flags, and its checkbox/drop-down/text-input definition.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct FormFieldData {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "name", variant = Name, ty = FormFieldNameElement),
        child(local = "label", variant = Label, ty = DecimalNumberValue),
        child(local = "tabIndex", variant = TabIndex, ty = UnsignedDecimalNumberValue),
        child(local = "enabled", variant = Enabled, ty = Toggle),
        child(local = "calcOnExit", variant = CalcOnExit, ty = Toggle),
        child(local = "entryMacro", variant = EntryMacro, ty = MacroNameElement),
        child(local = "exitMacro", variant = ExitMacro, ty = MacroNameElement),
        child(local = "helpText", variant = HelpText, ty = FormFieldHelpTextElement),
        child(local = "statusText", variant = StatusText, ty = FormFieldStatusTextElement),
        child(local = "checkBox", variant = CheckBox, ty = FormFieldCheckBox),
        child(local = "ddList", variant = DropDownList, ty = FormFieldDropDownList),
        child(local = "textInput", variant = TextInput, ty = FormFieldTextInput)
    )]
    content: Vec<FormFieldDataContent>,
}

/// One ordered child of a [`FormFieldData`]: `CT_FFData`'s own repeatable choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormFieldDataContent {
    /// `w:name` (`CT_FFName`) — the form field's own name.
    Name(FormFieldNameElement),
    /// `w:label` (`CT_DecimalNumber`) — legacy help-index label.
    Label(DecimalNumberValue),
    /// `w:tabIndex` (`CT_UnsignedDecimalNumber`) — tab order.
    TabIndex(UnsignedDecimalNumberValue),
    /// `w:enabled` (`CT_OnOff`) — whether the field can be edited.
    Enabled(Toggle),
    /// `w:calcOnExit` (`CT_OnOff`) — whether leaving the field recalculates other fields.
    CalcOnExit(Toggle),
    /// `w:entryMacro` (`CT_MacroName`) — the macro run on entering the field.
    EntryMacro(MacroNameElement),
    /// `w:exitMacro` (`CT_MacroName`) — the macro run on leaving the field.
    ExitMacro(MacroNameElement),
    /// `w:helpText` (`CT_FFHelpText`) — status-bar or F1 help text.
    HelpText(FormFieldHelpTextElement),
    /// `w:statusText` (`CT_FFStatusText`) — status-bar text.
    StatusText(FormFieldStatusTextElement),
    /// `w:checkBox` (`CT_FFCheckBox`) — present when this is a checkbox form field.
    CheckBox(FormFieldCheckBox),
    /// `w:ddList` (`CT_FFDDList`) — present when this is a drop-down-list form field.
    DropDownList(FormFieldDropDownList),
    /// `w:textInput` (`CT_FFTextInput`) — present when this is a text-input form field.
    TextInput(FormFieldTextInput),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// Finds the first `content` item matching `matcher`, or appends a freshly built one (via `build`)
/// and returns that — the "read or create in place" pattern every accessor pair below (`x()`/
/// `x_or_insert()`, mirroring [`super::body::Run::run_properties_or_insert`]) shares. `CT_FFData`'s
/// own content model is an unordered, repeatable `xsd:choice` (confirmed against `wml.xsd`
/// directly), so — unlike `w:pPr`/`w:rPr`'s fixed rank-0 position — a newly created member has no
/// schema rank to respect and simply appends.
fn find_or_insert<T>(
    content: &mut Vec<FormFieldDataContent>,
    matcher: impl Fn(&FormFieldDataContent) -> Option<&T>,
    matcher_mut: impl Fn(&mut FormFieldDataContent) -> Option<&mut T>,
    build: impl FnOnce() -> FormFieldDataContent,
) -> &mut T {
    let at = match content.iter().position(|item| matcher(item).is_some()) {
        Some(at) => at,
        None => {
            content.push(build());
            content.len() - 1
        }
    };
    matcher_mut(&mut content[at]).unwrap_or_else(|| unreachable!("`at` was just found or inserted"))
}

impl FormFieldData {
    /// Builds an empty `w:ffData` — every member absent until a setter below adds one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "ffData"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The form field's own name (`w:name/@val`), or `None` if it carries none (schema-legal — the
    /// attribute itself is optional — though Word always writes one).
    #[must_use]
    pub fn name(&self, interner: &Interner) -> Option<String> {
        self.content.iter().find_map(|item| match item {
            FormFieldDataContent::Name(element) => element.raw_name_lossy(interner),
            _ => None,
        })
    }

    /// Sets the form field's name, creating `w:name` first if it does not already carry one.
    ///
    /// # Errors
    /// [`DocxError::ValueTooLong`] if `name` is longer than `ST_FFName`'s 65-character bound —
    /// refused here rather than written and only failing the schema gate later.
    pub fn set_name(&mut self, interner: &mut Interner, name: &str) -> Result<(), DocxError> {
        let element = find_or_insert(
            &mut self.content,
            |item| match item {
                FormFieldDataContent::Name(element) => Some(element),
                _ => None,
            },
            |item| match item {
                FormFieldDataContent::Name(element) => Some(element),
                _ => None,
            },
            || FormFieldDataContent::Name(FormFieldNameElement::empty(interner)),
        );
        element.set_name(interner, name)?;
        self.empty = false;
        Ok(())
    }

    /// Whether the field can be edited (`w:enabled`), or `None` if it carries no `w:enabled` at all.
    #[must_use]
    pub fn enabled(&self, interner: &Interner) -> Option<bool> {
        self.content.iter().find_map(|item| match item {
            FormFieldDataContent::Enabled(toggle) => toggle_value(toggle, interner),
            _ => None,
        })
    }

    /// Sets (or clears, given `None`) whether the field can be edited.
    pub fn set_enabled(&mut self, interner: &mut Interner, enabled: Option<bool>) {
        set_toggle_member(
            &mut self.content,
            enabled,
            |item| match item {
                FormFieldDataContent::Enabled(toggle) => Some(toggle),
                _ => None,
            },
            |item| match item {
                FormFieldDataContent::Enabled(toggle) => Some(toggle),
                _ => None,
            },
            FormFieldDataContent::Enabled,
            "enabled",
            interner,
        );
        self.empty = false;
    }

    /// Whether leaving the field recalculates other fields (`w:calcOnExit`), or `None` if absent.
    #[must_use]
    pub fn calc_on_exit(&self, interner: &Interner) -> Option<bool> {
        self.content.iter().find_map(|item| match item {
            FormFieldDataContent::CalcOnExit(toggle) => toggle_value(toggle, interner),
            _ => None,
        })
    }

    /// Sets (or clears) whether leaving the field recalculates other fields.
    pub fn set_calc_on_exit(&mut self, interner: &mut Interner, calc_on_exit: Option<bool>) {
        set_toggle_member(
            &mut self.content,
            calc_on_exit,
            |item| match item {
                FormFieldDataContent::CalcOnExit(toggle) => Some(toggle),
                _ => None,
            },
            |item| match item {
                FormFieldDataContent::CalcOnExit(toggle) => Some(toggle),
                _ => None,
            },
            FormFieldDataContent::CalcOnExit,
            "calcOnExit",
            interner,
        );
        self.empty = false;
    }

    /// The macro run on entering the field (`w:entryMacro/@val`), or `None` if absent.
    #[must_use]
    pub fn entry_macro(&self, interner: &Interner) -> Option<String> {
        self.content.iter().find_map(|item| match item {
            FormFieldDataContent::EntryMacro(element) => element.raw_name_lossy(interner),
            _ => None,
        })
    }

    /// Sets the entry macro, creating `w:entryMacro` first if absent.
    ///
    /// # Errors
    /// [`DocxError::ValueTooLong`] if `macro_name` is longer than `ST_MacroName`'s 33-character
    /// bound.
    pub fn set_entry_macro(
        &mut self,
        interner: &mut Interner,
        macro_name: &str,
    ) -> Result<(), DocxError> {
        set_macro_member(
            &mut self.content,
            macro_name,
            interner,
            |item| match item {
                FormFieldDataContent::EntryMacro(element) => Some(element),
                _ => None,
            },
            |item| match item {
                FormFieldDataContent::EntryMacro(element) => Some(element),
                _ => None,
            },
            FormFieldDataContent::EntryMacro,
            "entryMacro",
        )?;
        self.empty = false;
        Ok(())
    }

    /// The macro run on leaving the field (`w:exitMacro/@val`), or `None` if absent.
    #[must_use]
    pub fn exit_macro(&self, interner: &Interner) -> Option<String> {
        self.content.iter().find_map(|item| match item {
            FormFieldDataContent::ExitMacro(element) => element.raw_name_lossy(interner),
            _ => None,
        })
    }

    /// Sets the exit macro, creating `w:exitMacro` first if absent.
    ///
    /// # Errors
    /// [`DocxError::ValueTooLong`] if `macro_name` is longer than `ST_MacroName`'s 33-character
    /// bound.
    pub fn set_exit_macro(
        &mut self,
        interner: &mut Interner,
        macro_name: &str,
    ) -> Result<(), DocxError> {
        set_macro_member(
            &mut self.content,
            macro_name,
            interner,
            |item| match item {
                FormFieldDataContent::ExitMacro(element) => Some(element),
                _ => None,
            },
            |item| match item {
                FormFieldDataContent::ExitMacro(element) => Some(element),
                _ => None,
            },
            FormFieldDataContent::ExitMacro,
            "exitMacro",
        )?;
        self.empty = false;
        Ok(())
    }

    /// The field's help text (`w:helpText`), or `None` if it carries none.
    #[must_use]
    pub fn help_text(&self) -> Option<&FormFieldHelpTextElement> {
        self.content.iter().find_map(|item| match item {
            FormFieldDataContent::HelpText(element) => Some(element),
            _ => None,
        })
    }

    /// Sets the field's help text, creating `w:helpText` first if it does not already carry one.
    ///
    /// # Errors
    /// [`DocxError::ValueTooLong`] if `text` is longer than `ST_FFHelpTextVal`'s 256-character
    /// bound.
    pub fn set_help_text(
        &mut self,
        interner: &mut Interner,
        kind: Option<HelpOrStatusTextType>,
        text: &str,
    ) -> Result<(), DocxError> {
        let element = find_or_insert(
            &mut self.content,
            |item| match item {
                FormFieldDataContent::HelpText(element) => Some(element),
                _ => None,
            },
            |item| match item {
                FormFieldDataContent::HelpText(element) => Some(element),
                _ => None,
            },
            || FormFieldDataContent::HelpText(FormFieldHelpTextElement::empty(interner)),
        );
        element.set_kind(interner, kind);
        element.set_text(interner, text)?;
        self.empty = false;
        Ok(())
    }

    /// The field's status-bar text (`w:statusText`), or `None` if it carries none.
    #[must_use]
    pub fn status_text(&self) -> Option<&FormFieldStatusTextElement> {
        self.content.iter().find_map(|item| match item {
            FormFieldDataContent::StatusText(element) => Some(element),
            _ => None,
        })
    }

    /// Sets the field's status-bar text, creating `w:statusText` first if absent.
    ///
    /// # Errors
    /// [`DocxError::ValueTooLong`] if `text` is longer than `ST_FFStatusTextVal`'s 140-character
    /// bound.
    pub fn set_status_text(
        &mut self,
        interner: &mut Interner,
        kind: Option<HelpOrStatusTextType>,
        text: &str,
    ) -> Result<(), DocxError> {
        let element = find_or_insert(
            &mut self.content,
            |item| match item {
                FormFieldDataContent::StatusText(element) => Some(element),
                _ => None,
            },
            |item| match item {
                FormFieldDataContent::StatusText(element) => Some(element),
                _ => None,
            },
            || FormFieldDataContent::StatusText(FormFieldStatusTextElement::empty(interner)),
        );
        element.set_kind(interner, kind);
        element.set_text(interner, text)?;
        self.empty = false;
        Ok(())
    }

    /// This form field's checkbox definition (`w:checkBox`), or `None` if it is not a checkbox
    /// field.
    #[must_use]
    pub fn check_box(&self) -> Option<&FormFieldCheckBox> {
        self.content.iter().find_map(|item| match item {
            FormFieldDataContent::CheckBox(check_box) => Some(check_box),
            _ => None,
        })
    }

    /// Sets (replaces) or removes this field's checkbox definition.
    pub fn set_check_box(&mut self, value: Option<FormFieldCheckBox>) {
        replace_choice_member(
            &mut self.content,
            value,
            |item| matches!(item, FormFieldDataContent::CheckBox(_)),
            FormFieldDataContent::CheckBox,
        );
        self.empty = false;
    }

    /// This form field's drop-down-list definition (`w:ddList`), or `None` if it is not a
    /// drop-down-list field.
    #[must_use]
    pub fn drop_down_list(&self) -> Option<&FormFieldDropDownList> {
        self.content.iter().find_map(|item| match item {
            FormFieldDataContent::DropDownList(list) => Some(list),
            _ => None,
        })
    }

    /// Sets (replaces) or removes this field's drop-down-list definition.
    pub fn set_drop_down_list(&mut self, value: Option<FormFieldDropDownList>) {
        replace_choice_member(
            &mut self.content,
            value,
            |item| matches!(item, FormFieldDataContent::DropDownList(_)),
            FormFieldDataContent::DropDownList,
        );
        self.empty = false;
    }

    /// This form field's text-input definition (`w:textInput`), or `None` if it is not a
    /// text-input field.
    #[must_use]
    pub fn text_input(&self) -> Option<&FormFieldTextInput> {
        self.content.iter().find_map(|item| match item {
            FormFieldDataContent::TextInput(input) => Some(input),
            _ => None,
        })
    }

    /// Sets (replaces) or removes this field's text-input definition.
    pub fn set_text_input(&mut self, value: Option<FormFieldTextInput>) {
        replace_choice_member(
            &mut self.content,
            value,
            |item| matches!(item, FormFieldDataContent::TextInput(_)),
            FormFieldDataContent::TextInput,
        );
        self.empty = false;
    }
}

/// The value of a `CT_OnOff`-shaped member — `None` when the member itself is absent, matching
/// [`Toggle::value`]'s own "present with no `val` defaults to `true`" contract, but never
/// panicking if the attribute is present and malformed (reads `None` in that case, the same
/// leniency every other accessor in this crate gives untrusted input).
fn toggle_value(toggle: &Toggle, interner: &Interner) -> Option<bool> {
    toggle.value(interner).ok()
}

/// Sets (or removes) a `CT_OnOff`-shaped member of `content` by variant, creating one at
/// `local`'s own wire name first if `value` is `Some` and none exists yet.
fn set_toggle_member<T>(
    content: &mut Vec<T>,
    value: Option<bool>,
    matcher: impl Fn(&T) -> Option<&Toggle>,
    matcher_mut: impl Fn(&mut T) -> Option<&mut Toggle>,
    wrap: impl Fn(Toggle) -> T,
    local: &str,
    interner: &mut Interner,
) {
    let at = content.iter().position(|item| matcher(item).is_some());
    match (at, value) {
        (Some(at), Some(value)) => {
            if let Some(toggle) = matcher_mut(&mut content[at]) {
                toggle.set_value(interner, Some(value));
            }
        }
        (Some(at), None) => {
            content.remove(at);
        }
        (None, Some(value)) => {
            let mut toggle = Toggle::new(interner, local);
            toggle.set_value(interner, Some(value));
            content.push(wrap(toggle));
        }
        (None, None) => {}
    }
}

/// Sets a `CT_MacroName`-shaped member (`w:entryMacro`/`w:exitMacro`), validating the 33-character
/// bound before writing — the shared body of [`FormFieldData::set_entry_macro`]/`set_exit_macro`.
fn set_macro_member<T>(
    content: &mut Vec<T>,
    macro_name: &str,
    interner: &mut Interner,
    matcher: impl Fn(&T) -> Option<&MacroNameElement>,
    matcher_mut: impl Fn(&mut T) -> Option<&mut MacroNameElement>,
    wrap: impl Fn(MacroNameElement) -> T,
    local: &str,
) -> Result<(), DocxError> {
    check_max_length("macro name", macro_name, MACRO_NAME_MAX_LENGTH)?;
    let at = content.iter().position(|item| matcher(item).is_some());
    match at {
        Some(at) => {
            if let Some(element) = matcher_mut(&mut content[at]) {
                element.set_raw_name(interner, macro_name);
            }
        }
        None => {
            let mut element = MacroNameElement::empty(interner, local);
            element.set_raw_name(interner, macro_name);
            content.push(wrap(element));
        }
    }
    Ok(())
}

/// Replaces (or removes) the one member of `content` `is_target` matches with `value` — the shared
/// body of [`FormFieldData::set_check_box`]/`set_drop_down_list`/`set_text_input`: `w:checkBox`,
/// `w:ddList` and `w:textInput` are siblings in the same `xsd:choice`, but only one is ever the
/// field's actual kind, so setting one never removes another kind a caller has not asked to clear
/// (mirrors "a caller who edits one half never touches the other" the rest of this module follows).
fn replace_choice_member<T, V>(
    content: &mut Vec<T>,
    value: Option<V>,
    is_target: impl Fn(&T) -> bool,
    wrap: impl FnOnce(V) -> T,
) {
    let at = content.iter().position(is_target);
    match (at, value) {
        (Some(at), Some(value)) => content[at] = wrap(value),
        (Some(at), None) => {
            content.remove(at);
        }
        (None, Some(value)) => content.push(wrap(value)),
        (None, None) => {}
    }
}

/// Places `value` into `content` at its schema rank in `order`: replaces the existing member
/// `is_target` matches, keeping its position, or — when there is none — inserts at the position
/// `order`'s own `xsd:sequence` requires (after every sibling that must precede `new_local`,
/// skipping any item `local_of` cannot name, exactly as [`ChildOrder::insert_index_of_names`]
/// documents). The shared body of every setter on [`FormFieldCheckBox`], [`FormFieldDropDownList`]
/// and [`FormFieldTextInput`] — all three `xsd:sequence`-shaped, unlike [`FormFieldData`]'s own
/// top-level `xsd:choice`, which imposes no order at all (see `mjx_ooxml_types::child_order`'s own
/// doc comment), so [`replace_choice_member`]/[`set_toggle_member`]'s plain-append fallback is
/// correct there and *only* there — reusing it for one of these three sequence types is exactly the
/// defect this function exists to fix (confirmed against `wml.xsd` directly: `CT_FFDDList` is
/// `result?, default?, listEntry*`, not an unordered set).
fn place_at_rank<T>(
    content: &mut Vec<T>,
    order: &'static ChildOrder,
    is_target: impl Fn(&T) -> bool,
    local_of: impl Fn(&T) -> &'static str,
    new_local: &'static str,
    value: T,
) {
    if let Some(at) = content.iter().position(is_target) {
        content[at] = value;
        return;
    }
    let ranks: Vec<Option<u16>> = content
        .iter()
        .map(|item| order.rank_of(None, local_of(item)))
        .collect();
    let at = order.insert_index_of_names(ranks.into_iter(), new_local);
    content.insert(at, value);
}

/// [`FormFieldCheckBoxContent`]'s own wire local name, for [`place_at_rank`].
fn checkbox_local(item: &FormFieldCheckBoxContent) -> &'static str {
    match item {
        FormFieldCheckBoxContent::Size(_) => "size",
        FormFieldCheckBoxContent::SizeAuto(_) => "sizeAuto",
        FormFieldCheckBoxContent::Default(_) => "default",
        FormFieldCheckBoxContent::Checked(_) => "checked",
        FormFieldCheckBoxContent::Raw(_) => "",
    }
}

/// [`FormFieldDropDownListContent`]'s own wire local name, for [`place_at_rank`].
fn drop_down_list_local(item: &FormFieldDropDownListContent) -> &'static str {
    match item {
        FormFieldDropDownListContent::Result(_) => "result",
        FormFieldDropDownListContent::Default(_) => "default",
        FormFieldDropDownListContent::ListEntry(_) => "listEntry",
        FormFieldDropDownListContent::Raw(_) => "",
    }
}

/// [`FormFieldTextInputContent`]'s own wire local name, for [`place_at_rank`].
fn text_input_local(item: &FormFieldTextInputContent) -> &'static str {
    match item {
        FormFieldTextInputContent::Kind(_) => "type",
        FormFieldTextInputContent::Default(_) => "default",
        FormFieldTextInputContent::MaxLength(_) => "maxLength",
        FormFieldTextInputContent::Format(_) => "format",
        FormFieldTextInputContent::Raw(_) => "",
    }
}

// =================================================================================================
// Length-bounded leaves: ST_FFName (65), ST_FFHelpTextVal (256), ST_FFStatusTextVal (140),
// ST_MacroName (33) — xsd:string with a maxLength facet, not enumerations. Reading never rejects an
// over-long value (fidelity-first); every validated setter below does, with `DocxError::ValueTooLong`.
// =================================================================================================

const FORM_FIELD_NAME_MAX_LENGTH: usize = 65;
const FORM_FIELD_HELP_TEXT_MAX_LENGTH: usize = 256;
const FORM_FIELD_STATUS_TEXT_MAX_LENGTH: usize = 140;
const MACRO_NAME_MAX_LENGTH: usize = 33;

/// Refuses `value` with [`DocxError::ValueTooLong`] if it is longer (in Unicode scalar values, the
/// same unit `xsd:maxLength` counts a string-based facet in) than `max`.
fn check_max_length(field: &'static str, value: &str, max: usize) -> Result<(), DocxError> {
    let len = value.chars().count();
    if len > max {
        return Err(DocxError::ValueTooLong { field, max, len });
    }
    Ok(())
}

/// `w:name` (`CT_FFName`) — a form field's own name. `val` is optional per the schema (unlike
/// `w:entryMacro`/`w:exitMacro`'s `val`), though Word always writes one.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = raw_name))]
pub struct FormFieldNameElement {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FormFieldNameElement {
    fn empty(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "name"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }

    /// The raw name, or `None` if malformed/absent — never panics on untrusted input.
    fn raw_name_lossy(&self, interner: &Interner) -> Option<String> {
        match self.raw_name(interner) {
            Ok(Some(cow)) => Some(cow.into_owned()),
            _ => None,
        }
    }

    /// Sets the form field's name.
    ///
    /// # Errors
    /// [`DocxError::ValueTooLong`] if `name` is longer than `ST_FFName`'s 65-character bound.
    pub fn set_name(&mut self, interner: &mut Interner, name: &str) -> Result<(), DocxError> {
        check_max_length("form field name", name, FORM_FIELD_NAME_MAX_LENGTH)?;
        self.set_raw_name(interner, Some(name));
        self.empty = false;
        Ok(())
    }
}

impl FromXml for FormFieldNameElement {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for FormFieldNameElement {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:helpText` (`CT_FFHelpText`) — a form field's status-bar or F1 help text.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "type", prefix = "w", codec = Enumeration<HelpOrStatusTextType>, accessor = kind_raw))]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = raw_text))]
pub struct FormFieldHelpTextElement {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FormFieldHelpTextElement {
    fn empty(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "helpText"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }

    /// The help text, or `None` if malformed/absent.
    #[must_use]
    pub fn text(&self, interner: &Interner) -> Option<String> {
        match self.raw_text(interner) {
            Ok(Some(cow)) => Some(cow.into_owned()),
            _ => None,
        }
    }

    /// Whether this help text is literal (`text`) or auto-generated (`autoText`), or `None` if
    /// malformed/absent.
    #[must_use]
    pub fn kind(&self, interner: &Interner) -> Option<HelpOrStatusTextType> {
        self.kind_raw(interner).ok().flatten()
    }

    fn set_kind(&mut self, interner: &mut Interner, kind: Option<HelpOrStatusTextType>) {
        self.set_kind_raw(interner, kind);
    }

    /// Sets the help text.
    ///
    /// # Errors
    /// [`DocxError::ValueTooLong`] if `text` is longer than `ST_FFHelpTextVal`'s 256-character
    /// bound.
    pub fn set_text(&mut self, interner: &mut Interner, text: &str) -> Result<(), DocxError> {
        check_max_length("help text", text, FORM_FIELD_HELP_TEXT_MAX_LENGTH)?;
        self.set_raw_text(interner, Some(text));
        self.empty = false;
        Ok(())
    }
}

impl FromXml for FormFieldHelpTextElement {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for FormFieldHelpTextElement {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:statusText` (`CT_FFStatusText`) — a form field's status-bar text. Same shape as
/// [`FormFieldHelpTextElement`], a distinct type because its own bound (140, not 256) differs.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "type", prefix = "w", codec = Enumeration<HelpOrStatusTextType>, accessor = kind_raw))]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = raw_text))]
pub struct FormFieldStatusTextElement {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FormFieldStatusTextElement {
    fn empty(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "statusText"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }

    /// The status text, or `None` if malformed/absent.
    #[must_use]
    pub fn text(&self, interner: &Interner) -> Option<String> {
        match self.raw_text(interner) {
            Ok(Some(cow)) => Some(cow.into_owned()),
            _ => None,
        }
    }

    /// Whether this status text is literal (`text`) or auto-generated (`autoText`), or `None` if
    /// malformed/absent.
    #[must_use]
    pub fn kind(&self, interner: &Interner) -> Option<HelpOrStatusTextType> {
        self.kind_raw(interner).ok().flatten()
    }

    fn set_kind(&mut self, interner: &mut Interner, kind: Option<HelpOrStatusTextType>) {
        self.set_kind_raw(interner, kind);
    }

    /// Sets the status text.
    ///
    /// # Errors
    /// [`DocxError::ValueTooLong`] if `text` is longer than `ST_FFStatusTextVal`'s 140-character
    /// bound.
    pub fn set_text(&mut self, interner: &mut Interner, text: &str) -> Result<(), DocxError> {
        check_max_length("status text", text, FORM_FIELD_STATUS_TEXT_MAX_LENGTH)?;
        self.set_raw_text(interner, Some(text));
        self.empty = false;
        Ok(())
    }
}

impl FromXml for FormFieldStatusTextElement {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for FormFieldStatusTextElement {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:entryMacro`/`w:exitMacro` (`CT_MacroName`) — a macro name, required by the schema (`val` is
/// `use="required"`, unlike the three siblings above).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = raw_name, required))]
pub struct MacroNameElement {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl MacroNameElement {
    fn empty(interner: &mut Interner, local: &str) -> Self {
        Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }

    /// The macro name, or `None` if malformed/missing (illegal per the schema — `val` is required
    /// — but a malformed file is read, not panicked on).
    fn raw_name_lossy(&self, interner: &Interner) -> Option<String> {
        match self.raw_name(interner) {
            Ok(cow) => Some(cow.into_owned()),
            Err(_) => None,
        }
    }
}

impl FromXml for MacroNameElement {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for MacroNameElement {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// =================================================================================================
// Small unbounded leaves: CT_String (w:default/w:format/w:listEntry), CT_UnsignedDecimalNumber
// (w:tabIndex), CT_FFTextType (w:type inside w:textInput). None of these three carries a maxLength
// facet, so no validated setter is needed — the derive's own setter is the one write path.
// =================================================================================================

/// `CT_String` — one required plain-string `val`. Reused for `w:default` (both `w:ddList`'s and
/// `w:textInput`'s own), `w:format` and `w:listEntry` — four different elements sharing one wire
/// shape, exactly as [`super::body::Text`] is reused across four `EG_RunInnerContent` members.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = raw_value, required))]
pub struct StringElement {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl StringElement {
    /// Builds a new `local` element (`"default"`, `"format"` or `"listEntry"`) of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str, value: &str) -> Self {
        let mut item = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_raw_value(interner, value);
        item
    }

    /// The value, or `None` if malformed/missing.
    #[must_use]
    pub fn value(&self, interner: &Interner) -> Option<String> {
        match self.raw_value(interner) {
            Ok(cow) => Some(cow.into_owned()),
            Err(_) => None,
        }
    }
}

impl FromXml for StringElement {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for StringElement {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_UnsignedDecimalNumber` (`w:tabIndex`) — a required unsigned integer `val`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Number<UnsignedDecimalNumber>, accessor = value, required))]
pub struct UnsignedDecimalNumberValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl UnsignedDecimalNumberValue {
    /// Builds a new `local` element (`"tabIndex"`) of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str, value: UnsignedDecimalNumber) -> Self {
        let mut item = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for UnsignedDecimalNumberValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for UnsignedDecimalNumberValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_FFTextType` (`w:type`, inside `w:textInput`) — the text input's own kind (`regular`,
/// `number`, `date`, …).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<FormFieldTextType>, accessor = value, required))]
pub struct FormFieldTextTypeElement {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FormFieldTextTypeElement {
    /// Builds a new `w:type` of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, value: FormFieldTextType) -> Self {
        let mut item = Self {
            name: wml_name(interner, "type"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for FormFieldTextTypeElement {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for FormFieldTextTypeElement {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// =================================================================================================
// The three form-field kinds: w:checkBox (CT_FFCheckBox), w:ddList (CT_FFDDList),
// w:textInput (CT_FFTextInput)
// =================================================================================================

/// `w:checkBox` (`CT_FFCheckBox`) — a checkbox form field's own size (fixed or automatic), default
/// state and current checked state.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct FormFieldCheckBox {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "size", variant = Size, ty = HalfPointMeasureValue),
        child(local = "sizeAuto", variant = SizeAuto, ty = Toggle),
        child(local = "default", variant = Default, ty = Toggle),
        child(local = "checked", variant = Checked, ty = Toggle)
    )]
    content: Vec<FormFieldCheckBoxContent>,
}

/// One ordered child of a [`FormFieldCheckBox`]: `CT_FFCheckBox`'s `(size | sizeAuto), default?,
/// checked?`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormFieldCheckBoxContent {
    /// `w:size` (`CT_HpsMeasure`) — a fixed size, in half-points.
    Size(HalfPointMeasureValue),
    /// `w:sizeAuto` (`CT_OnOff`) — sized automatically from the surrounding text.
    SizeAuto(Toggle),
    /// `w:default` (`CT_OnOff`) — the checkbox's default (unset) state.
    Default(Toggle),
    /// `w:checked` (`CT_OnOff`) — the checkbox's current state.
    Checked(Toggle),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl FormFieldCheckBox {
    /// Builds a new checkbox with an automatic size and no stated default/checked state.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        let mut auto = Toggle::new(interner, "sizeAuto");
        auto.set_value(interner, Some(true));
        Self {
            name: wml_name(interner, "checkBox"),
            attributes: Vec::new(),
            empty: false,
            content: vec![FormFieldCheckBoxContent::SizeAuto(auto)],
        }
    }

    /// This checkbox's fixed size (`w:size`, half-points), or `None` if it is sized automatically
    /// or carries no size element at all.
    #[must_use]
    pub fn fixed_size(&self) -> Option<&HalfPointMeasureValue> {
        self.content.iter().find_map(|item| match item {
            FormFieldCheckBoxContent::Size(size) => Some(size),
            _ => None,
        })
    }

    /// Whether this checkbox is sized automatically (`w:sizeAuto`).
    #[must_use]
    pub fn is_auto_sized(&self) -> bool {
        self.content
            .iter()
            .any(|item| matches!(item, FormFieldCheckBoxContent::SizeAuto(_)))
    }

    /// The checkbox's default (unset) state (`w:default`), or `None` if absent.
    #[must_use]
    pub fn default_checked(&self, interner: &Interner) -> Option<bool> {
        self.content.iter().find_map(|item| match item {
            FormFieldCheckBoxContent::Default(toggle) => toggle_value(toggle, interner),
            _ => None,
        })
    }

    /// Sets (or clears) the checkbox's default (unset) state.
    pub fn set_default_checked(&mut self, interner: &mut Interner, value: Option<bool>) {
        match value {
            Some(value) => {
                let mut toggle = Toggle::new(interner, "default");
                toggle.set_value(interner, Some(value));
                place_at_rank(
                    &mut self.content,
                    FORM_FIELD_CHECK_BOX,
                    |item| matches!(item, FormFieldCheckBoxContent::Default(_)),
                    checkbox_local,
                    "default",
                    FormFieldCheckBoxContent::Default(toggle),
                );
            }
            None => self
                .content
                .retain(|item| !matches!(item, FormFieldCheckBoxContent::Default(_))),
        }
    }

    /// The checkbox's current state (`w:checked`), or `None` if absent.
    #[must_use]
    pub fn checked(&self, interner: &Interner) -> Option<bool> {
        self.content.iter().find_map(|item| match item {
            FormFieldCheckBoxContent::Checked(toggle) => toggle_value(toggle, interner),
            _ => None,
        })
    }

    /// Sets (or clears) the checkbox's current state.
    pub fn set_checked(&mut self, interner: &mut Interner, value: Option<bool>) {
        match value {
            Some(value) => {
                let mut toggle = Toggle::new(interner, "checked");
                toggle.set_value(interner, Some(value));
                place_at_rank(
                    &mut self.content,
                    FORM_FIELD_CHECK_BOX,
                    |item| matches!(item, FormFieldCheckBoxContent::Checked(_)),
                    checkbox_local,
                    "checked",
                    FormFieldCheckBoxContent::Checked(toggle),
                );
            }
            None => self
                .content
                .retain(|item| !matches!(item, FormFieldCheckBoxContent::Checked(_))),
        }
    }
}

/// `w:ddList` (`CT_FFDDList`) — a drop-down-list form field's own entries, default selection and
/// current (result) selection, both by index into [`FormFieldDropDownList::entries`].
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct FormFieldDropDownList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "result", variant = Result, ty = DecimalNumberValue),
        child(local = "default", variant = Default, ty = DecimalNumberValue),
        child(local = "listEntry", variant = ListEntry, ty = StringElement)
    )]
    content: Vec<FormFieldDropDownListContent>,
}

/// One ordered child of a [`FormFieldDropDownList`]: `CT_FFDDList`'s `result?, default?,
/// listEntry*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormFieldDropDownListContent {
    /// `w:result` (`CT_DecimalNumber`) — the currently selected entry's index.
    Result(DecimalNumberValue),
    /// `w:default` (`CT_DecimalNumber`) — the default selected entry's index.
    Default(DecimalNumberValue),
    /// `w:listEntry` (`CT_String`) — one entry's own text; repeatable.
    ListEntry(StringElement),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl FormFieldDropDownList {
    /// Builds a new drop-down list with `entries` and no stated default/current selection.
    #[must_use]
    pub fn new(interner: &mut Interner, entries: &[&str]) -> Self {
        let content = entries
            .iter()
            .map(|entry| {
                FormFieldDropDownListContent::ListEntry(StringElement::new(
                    interner,
                    "listEntry",
                    entry,
                ))
            })
            .collect();
        Self {
            name: wml_name(interner, "ddList"),
            attributes: Vec::new(),
            empty: entries.is_empty(),
            content,
        }
    }

    /// Every entry's own text, in document order.
    pub fn entries<'a>(&'a self, interner: &'a Interner) -> impl Iterator<Item = String> + 'a {
        self.content.iter().filter_map(|item| match item {
            FormFieldDropDownListContent::ListEntry(entry) => entry.value(interner),
            _ => None,
        })
    }

    /// The currently selected entry's index (`w:result`), or `None` if absent.
    #[must_use]
    pub fn selected_index(&self, interner: &Interner) -> Option<i64> {
        self.content.iter().find_map(|item| match item {
            FormFieldDropDownListContent::Result(value) => decimal_value(value, interner),
            _ => None,
        })
    }

    /// Sets the currently selected entry's index, creating `w:result` first if absent.
    pub fn set_selected_index(&mut self, interner: &mut Interner, index: i64) {
        let value = DecimalNumberValue::new(interner, "result", index);
        place_at_rank(
            &mut self.content,
            FORM_FIELD_DROP_DOWN_LIST,
            |item| matches!(item, FormFieldDropDownListContent::Result(_)),
            drop_down_list_local,
            "result",
            FormFieldDropDownListContent::Result(value),
        );
    }

    /// The default selected entry's index (`w:default`), or `None` if absent.
    #[must_use]
    pub fn default_index(&self, interner: &Interner) -> Option<i64> {
        self.content.iter().find_map(|item| match item {
            FormFieldDropDownListContent::Default(value) => decimal_value(value, interner),
            _ => None,
        })
    }
}

/// `w:textInput` (`CT_FFTextInput`) — a text-input form field's own kind, default text, maximum
/// length and legacy display format.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct FormFieldTextInput {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "type", variant = Kind, ty = FormFieldTextTypeElement),
        child(local = "default", variant = Default, ty = StringElement),
        child(local = "maxLength", variant = MaxLength, ty = DecimalNumberValue),
        child(local = "format", variant = Format, ty = StringElement)
    )]
    content: Vec<FormFieldTextInputContent>,
}

/// One ordered child of a [`FormFieldTextInput`]: `CT_FFTextInput`'s `type?, default?, maxLength?,
/// format?`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormFieldTextInputContent {
    /// `w:type` (`CT_FFTextType`) — `regular`, `number`, `date`, `currentTime`, `currentDate` or
    /// `calculated`.
    Kind(FormFieldTextTypeElement),
    /// `w:default` (`CT_String`) — the field's default text.
    Default(StringElement),
    /// `w:maxLength` (`CT_DecimalNumber`) — the maximum length Word enforces while editing.
    MaxLength(DecimalNumberValue),
    /// `w:format` (`CT_String`) — a legacy display-format picture string.
    Format(StringElement),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl FormFieldTextInput {
    /// Builds a new text input of `kind`, with no stated default text, maximum length or format.
    #[must_use]
    pub fn new(interner: &mut Interner, kind: FormFieldTextType) -> Self {
        Self {
            name: wml_name(interner, "textInput"),
            attributes: Vec::new(),
            empty: false,
            content: vec![FormFieldTextInputContent::Kind(
                FormFieldTextTypeElement::new(interner, kind),
            )],
        }
    }

    /// This text input's own kind (`w:type`), or `None` if malformed/absent.
    #[must_use]
    pub fn kind(&self, interner: &Interner) -> Option<FormFieldTextType> {
        self.content.iter().find_map(|item| match item {
            FormFieldTextInputContent::Kind(element) => element.value(interner).ok(),
            _ => None,
        })
    }

    /// The field's default text (`w:default`), or `None` if absent.
    #[must_use]
    pub fn default_text(&self, interner: &Interner) -> Option<String> {
        self.content.iter().find_map(|item| match item {
            FormFieldTextInputContent::Default(element) => element.value(interner),
            _ => None,
        })
    }

    /// Sets the field's default text, creating `w:default` first if absent.
    pub fn set_default_text(&mut self, interner: &mut Interner, text: &str) {
        let element = StringElement::new(interner, "default", text);
        place_at_rank(
            &mut self.content,
            FORM_FIELD_TEXT_INPUT,
            |item| matches!(item, FormFieldTextInputContent::Default(_)),
            text_input_local,
            "default",
            FormFieldTextInputContent::Default(element),
        );
    }

    /// The maximum length Word enforces while editing (`w:maxLength`), or `None` if unbounded.
    #[must_use]
    pub fn max_length(&self, interner: &Interner) -> Option<i64> {
        self.content.iter().find_map(|item| match item {
            FormFieldTextInputContent::MaxLength(value) => decimal_value(value, interner),
            _ => None,
        })
    }

    /// Sets the maximum length, creating `w:maxLength` first if absent.
    pub fn set_max_length(&mut self, interner: &mut Interner, max_length: i64) {
        let value = DecimalNumberValue::new(interner, "maxLength", max_length);
        place_at_rank(
            &mut self.content,
            FORM_FIELD_TEXT_INPUT,
            |item| matches!(item, FormFieldTextInputContent::MaxLength(_)),
            text_input_local,
            "maxLength",
            FormFieldTextInputContent::MaxLength(value),
        );
    }
}

/// The value of a `CT_DecimalNumber`-shaped member, or `None` if malformed.
fn decimal_value(value: &DecimalNumberValue, interner: &Interner) -> Option<i64> {
    value.value(interner).ok()
}

// =================================================================================================
// w:fldSimple (CT_SimpleField)
// =================================================================================================

/// `w:fldSimple` (`CT_SimpleField`) — the self-contained field form: the instruction is the `instr`
/// attribute itself, and the cached result is this element's own child content
/// (`EG_PContent*`, the *same* recursive content [`super::body::Paragraph`]/
/// [`super::body::Hyperlink`] hold, which is also how a simple field can nest another field inside
/// its own cached result).
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "instr", prefix = "w", codec = TextCodec, accessor = instruction_raw, required))]
#[xml(attribute(local = "fldLock", prefix = "w", codec = OnOff, accessor = locked, default = false))]
#[xml(attribute(local = "dirty", prefix = "w", codec = OnOff, accessor = dirty, default = false))]
pub struct SimpleField {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pPr", variant = Properties, ty = super::paragraph_properties::ParagraphProperties),
        child(local = "customXml", variant = CustomXml, ty = Unmodeled),
        child(local = "smartTag", variant = SmartTag, ty = Unmodeled),
        child(local = "sdt", variant = StructuredDocumentTag, ty = Unmodeled),
        child(local = "dir", variant = BidirectionalEmbedding, ty = Unmodeled),
        child(local = "bdo", variant = BidirectionalOverride, ty = Unmodeled),
        child(local = "r", variant = Run, ty = Run),
        child(local = "proofErr", variant = ProofingError, ty = super::body::ProofingError),
        child(local = "permStart", variant = PermissionRangeStart, ty = super::body::PermissionRangeStart),
        child(local = "permEnd", variant = PermissionRangeEnd, ty = super::body::PermissionRangeEnd),
        child(local = "fldSimple", variant = SimpleField, ty = SimpleField),
        child(local = "hyperlink", variant = Hyperlink, ty = super::body::Hyperlink),
        child(local = "subDoc", variant = SubDocument, ty = super::body::RelationshipReference),
        child(local = "fldData", variant = FieldData, ty = Text)
    )]
    content: Vec<ParagraphContent>,
}

impl SimpleField {
    /// The field's instruction, verbatim (the `instr` attribute) — never parsed beyond what
    /// [`Field::field_name`]/[`Field::arguments`] split lazily on read.
    #[must_use]
    pub fn instruction(&self, interner: &Interner) -> String {
        match self.instruction_raw(interner) {
            Ok(cow) => cow.into_owned(),
            Err(_) => String::new(),
        }
    }

    /// Sets the field's instruction (the `instr` attribute). `ST_String` carries no length bound —
    /// unlike the four form-field string types this module also declares — so nothing here refuses
    /// an over-long value.
    pub fn set_instruction(&mut self, interner: &mut Interner, text: &str) {
        self.set_instruction_raw(interner, text);
    }

    /// This field's own cached-result content, immutably.
    pub(crate) fn content(&self) -> &[ParagraphContent] {
        &self.content
    }

    /// [`SimpleField::content`], mutably.
    pub(crate) fn content_mut(&mut self) -> &mut Vec<ParagraphContent> {
        &mut self.content
    }

    /// The field's cached result, as visible text — every run reachable from this field's own
    /// content, descending into any nested hyperlink/field it holds (mirrors
    /// [`super::body::Paragraph::text`] exactly, since the content model is the same enum).
    #[must_use]
    pub fn cached_result_text(&self) -> String {
        let mut text = String::new();
        super::body::paragraph_content_text(&self.content, &mut text);
        text
    }
}

// =================================================================================================
// Field — the unified read model over both wire forms
// =================================================================================================

/// Which wire form a [`Field`] was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldForm {
    /// `w:fldSimple` (`CT_SimpleField`).
    Simple,
    /// `w:fldChar` `begin`/`separate` (optional)/`end`.
    Complex,
}

/// The address of a [`Field`] within one paragraph's own top-level [`Field`] sequence — a top-level
/// index, then the indices to descend through [`Field::nested_fields`]. Follows [`crate::BlockPath`]/
/// [`crate::RunPath`]'s own vocabulary: construct one from a bare index for a top-level field, or
/// from an array/slice/`Vec` of indices to address a nested field (a `TOC` field's own `PAGEREF`,
/// say).
///
/// ```
/// use mjx_docx::FieldPath;
/// let top: FieldPath = 0.into(); // the paragraph's first field
/// let nested: FieldPath = [0, 1].into(); // that field's second nested field
/// assert_eq!(top.indices(), [0]);
/// assert_eq!(nested.indices(), [0, 1]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldPath(Vec<usize>);

impl FieldPath {
    /// The address as a slice of indices, outermost first.
    #[must_use]
    pub fn indices(&self) -> &[usize] {
        &self.0
    }
}

impl From<usize> for FieldPath {
    fn from(index: usize) -> Self {
        Self(vec![index])
    }
}

impl From<&FieldPath> for FieldPath {
    fn from(path: &FieldPath) -> Self {
        path.clone()
    }
}

impl From<Vec<usize>> for FieldPath {
    fn from(indices: Vec<usize>) -> Self {
        Self(indices)
    }
}

impl From<&[usize]> for FieldPath {
    fn from(indices: &[usize]) -> Self {
        Self(indices.to_vec())
    }
}

impl<const N: usize> From<[usize; N]> for FieldPath {
    fn from(indices: [usize; N]) -> Self {
        Self(indices.to_vec())
    }
}

/// The position of a [`Field`] within its own paragraph's content, sufficient to locate the exact
/// runs a later edit ([`set_field_instruction`]/[`set_field_cached_result_text`]) touches. Private —
/// re-derived by re-parsing rather than exposed, so a caller cannot hold a stale [`Coord`] across an
/// edit that shifts everything after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Coord {
    /// Index into the paragraph's own top-level `content` (a [`ParagraphContent::Run`] or
    /// [`ParagraphContent::SimpleField`]).
    content_index: usize,
    /// Index into that [`Run`]'s own inner content — meaningless for a [`ParagraphContent::SimpleField`]
    /// coordinate.
    run_index: usize,
}

impl Coord {
    fn key(self) -> (usize, usize) {
        (self.content_index, self.run_index)
    }
}

/// Where a [`Field`] lives, for editing — [`FieldSite::Simple`]'s instruction is a plain attribute
/// (trivial to rewrite); [`FieldSite::Complex`]'s instruction/result are zones of sibling runs that
/// must be collapsed to a single run on edit (see [`collapse_zone`]).
#[derive(Debug, Clone, PartialEq, Eq)]
enum FieldSite {
    Simple {
        content_index: usize,
    },
    Complex {
        begin: Coord,
        separate: Option<Coord>,
        end: Coord,
        /// Whether a nested field was found before this field's own `separate` (or, if it has
        /// none, before its own `end`) — if so, the instruction zone contains a nested field and
        /// [`set_field_instruction`] refuses rather than collapsing over it.
        nested_in_instruction: bool,
        /// As `nested_in_instruction`, for the zone after `separate` — governs
        /// [`set_field_cached_result_text`].
        nested_in_result: bool,
    },
}

/// One field — the union of `w:fldSimple` and the `begin`/`separate`/`end` marker form — read as
/// a snapshot: [`Field::instruction`] and [`Field::cached_result`] are always distinct, regardless
/// of which wire form produced this value. See [`Field::form`] for which one this is, and this
/// module's own doc comment for the nesting/nesting-vs-counting distinction this type exists to
/// make representable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    form: FieldForm,
    instruction: String,
    /// `None` only for the complex form with no `separate` marker — a field with no cached result,
    /// which is legal markup, not malformed (see this module's own doc comment). The simple form
    /// always reports `Some` (its content is always structurally a result slot, even when empty).
    cached_result: Option<String>,
    nested: Vec<Field>,
    site: FieldSite,
}

impl Field {
    /// Which wire form this field was read from.
    #[must_use]
    pub fn form(&self) -> FieldForm {
        self.form
    }

    /// The field's instruction, verbatim — concatenated from every `w:instrText` this field's own
    /// `begin`/`separate` span directly holds (complex form) or the `instr` attribute (simple
    /// form), **excluding** anything belonging to a nested field. Never parsed beyond what
    /// [`Field::field_name`]/[`Field::arguments`] split.
    #[must_use]
    pub fn instruction(&self) -> &str {
        &self.instruction
    }

    /// The field's own instruction keyword — the first whitespace-delimited token of
    /// [`Field::instruction`] (`"TOC"`, `"PAGEREF"`, `"HYPERLINK"`, …), or `None` if the
    /// instruction is empty or entirely whitespace. A lexical split only — this crate does not
    /// parse or normalize the field instruction language itself.
    #[must_use]
    pub fn field_name(&self) -> Option<&str> {
        self.instruction.split_whitespace().next()
    }

    /// Everything in [`Field::instruction`] after [`Field::field_name`], trimmed of leading
    /// whitespace — `""` if there is no field name.
    #[must_use]
    pub fn arguments(&self) -> &str {
        let trimmed = self.instruction.trim_start();
        match self.field_name() {
            Some(name) => trimmed[name.len()..].trim_start(),
            None => "",
        }
    }

    /// The field's cached result, as visible text — `None` only for a complex-form field with no
    /// `separate` marker (see [`Field`]'s own doc comment).
    #[must_use]
    pub fn cached_result(&self) -> Option<&str> {
        self.cached_result.as_deref()
    }

    /// The fields nested directly inside this one's own instruction or cached-result zone, in
    /// document order — a `TOC` field's own `PAGEREF` fields, for instance.
    #[must_use]
    pub fn nested_fields(&self) -> &[Field] {
        &self.nested
    }
}

/// Every field-relevant leaf item, flattened from a paragraph's (or a hyperlink's, or a simple
/// field's own) `EG_PContent`-shaped content, in document order — the input [`parse_top`]/
/// [`parse_complex`] pair with an explicit stack.
enum Leaf<'a> {
    Begin(Coord),
    Separate(Coord),
    End(Coord),
    InstrText(Coord, &'a str),
    Text(&'a str),
    SimpleField(usize, &'a SimpleField),
    Other,
}

fn flatten<'a>(content: &'a [ParagraphContent], interner: &Interner) -> Vec<Leaf<'a>> {
    let mut out = Vec::new();
    for (content_index, item) in content.iter().enumerate() {
        match item {
            ParagraphContent::Run(run) => {
                for (run_index, inner) in run.content().iter().enumerate() {
                    let coord = Coord {
                        content_index,
                        run_index,
                    };
                    out.push(match inner {
                        RunInnerContent::ComplexFieldCharacter(field_char) => {
                            match field_char.kind(interner) {
                                Ok(FieldCharacterType::Begin) => Leaf::Begin(coord),
                                Ok(FieldCharacterType::Separate) => Leaf::Separate(coord),
                                Ok(FieldCharacterType::End) => Leaf::End(coord),
                                Err(_) => Leaf::Other,
                            }
                        }
                        RunInnerContent::FieldCode(text) => Leaf::InstrText(coord, text.text()),
                        RunInnerContent::Text(text) => Leaf::Text(text.text()),
                        _ => Leaf::Other,
                    });
                }
            }
            ParagraphContent::SimpleField(field) => {
                out.push(Leaf::SimpleField(content_index, field))
            }
            _ => out.push(Leaf::Other),
        }
    }
    out
}

/// Reads every field at the top level of `content`, in document order.
///
/// # Errors
/// [`DocxError::UnbalancedField`] if a `separate`/`end` marker appears with no currently open
/// field, or a `begin` marker's own field never reaches a matching `end` before `content` is
/// exhausted — both legal, schema-valid markup (`ST_FldCharType` imposes no ordering or balance
/// constraint of its own) that this crate refuses to silently mispair rather than panic on.
pub(crate) fn parse_top(
    content: &[ParagraphContent],
    interner: &Interner,
) -> Result<Vec<Field>, DocxError> {
    let leaves = flatten(content, interner);
    let mut pos = 0;
    let mut fields = Vec::new();
    while pos < leaves.len() {
        match &leaves[pos] {
            Leaf::Begin(_) => fields.push(parse_complex(&leaves, &mut pos, interner)?),
            Leaf::Separate(_) => {
                return Err(DocxError::UnbalancedField(
                    "a w:fldChar separate marker with no matching begin".to_owned(),
                ));
            }
            Leaf::End(_) => {
                return Err(DocxError::UnbalancedField(
                    "a w:fldChar end marker with no matching begin".to_owned(),
                ));
            }
            Leaf::SimpleField(content_index, field) => {
                fields.push(field_from_simple(*content_index, field, interner)?);
                pos += 1;
            }
            Leaf::InstrText(..) | Leaf::Text(_) | Leaf::Other => pos += 1,
        }
    }
    Ok(fields)
}

/// Builds a [`Field`] from a `w:fldSimple` at `content_index`, recursing into its own cached-result
/// content for nested fields (a simple field's `EG_PContent*` can hold another `w:fldSimple`, or a
/// `begin`/`separate`/`end` sequence in one of its own runs).
fn field_from_simple(
    content_index: usize,
    field: &SimpleField,
    interner: &Interner,
) -> Result<Field, DocxError> {
    let nested = parse_top(field.content(), interner)?;
    Ok(Field {
        form: FieldForm::Simple,
        instruction: field.instruction(interner),
        cached_result: Some(field.cached_result_text()),
        nested,
        site: FieldSite::Simple { content_index },
    })
}

/// Parses one complex field starting at `leaves[*pos]` (which must be a [`Leaf::Begin`]), consuming
/// through its own matching [`Leaf::End`] — recursing for any nested field found along the way, so
/// nesting is paired by the call stack, never by counting markers. See this module's own doc
/// comment for why that distinction is exactly what the nested-`TOC` trap exercises.
fn parse_complex(
    leaves: &[Leaf<'_>],
    pos: &mut usize,
    interner: &Interner,
) -> Result<Field, DocxError> {
    let begin = match leaves[*pos] {
        Leaf::Begin(coord) => coord,
        _ => unreachable!("parse_complex is only called at a Leaf::Begin"),
    };
    *pos += 1;

    let mut instruction = String::new();
    let mut result_text = String::new();
    let mut separate: Option<Coord> = None;
    let mut nested = Vec::new();
    let mut nested_in_instruction = false;
    let mut nested_in_result = false;

    loop {
        let Some(leaf) = leaves.get(*pos) else {
            return Err(DocxError::UnbalancedField(format!(
                "a w:fldChar begin at run {} of paragraph content item {} has no matching end",
                begin.run_index, begin.content_index
            )));
        };
        match leaf {
            Leaf::Begin(_) => {
                let child = parse_complex(leaves, pos, interner)?;
                if separate.is_some() {
                    nested_in_result = true;
                    // The nested field's own cached result is what actually renders where it
                    // sits — a `TOC`'s displayed page numbers *are* its nested `PAGEREF`s' own
                    // results — so it folds into this field's own `result_text` exactly as a
                    // plain `w:t` would; only the nested field's own `w:instrText` stays scoped
                    // to `child.instruction`, never touching this field's own accumulator.
                    if let Some(text) = &child.cached_result {
                        result_text.push_str(text);
                    }
                } else {
                    nested_in_instruction = true;
                }
                nested.push(child);
            }
            Leaf::SimpleField(content_index, field) => {
                let child = field_from_simple(*content_index, field, interner)?;
                if separate.is_some() {
                    nested_in_result = true;
                    if let Some(text) = &child.cached_result {
                        result_text.push_str(text);
                    }
                } else {
                    nested_in_instruction = true;
                }
                nested.push(child);
                *pos += 1;
            }
            Leaf::Separate(coord) => {
                if separate.is_some() {
                    return Err(DocxError::UnbalancedField(
                        "a field's w:fldChar separate marker appears twice".to_owned(),
                    ));
                }
                separate = Some(*coord);
                *pos += 1;
            }
            Leaf::End(coord) => {
                let end = *coord;
                *pos += 1;
                return Ok(Field {
                    form: FieldForm::Complex,
                    instruction,
                    cached_result: separate.is_some().then_some(result_text),
                    nested,
                    site: FieldSite::Complex {
                        begin,
                        separate,
                        end,
                        nested_in_instruction,
                        nested_in_result,
                    },
                });
            }
            Leaf::InstrText(_, text) if separate.is_none() => {
                instruction.push_str(text);
                *pos += 1;
            }
            Leaf::Text(text) if separate.is_some() => {
                result_text.push_str(text);
                *pos += 1;
            }
            Leaf::InstrText(..) | Leaf::Text(_) | Leaf::Other => {
                *pos += 1;
            }
        }
    }
}

/// Locates the field at `path` (a sequence of top-level-then-nested indices) within `fields`, or
/// `None` if `path` is out of range at any level.
fn locate<'a>(fields: &'a [Field], path: &[usize]) -> Option<&'a Field> {
    let (&first, rest) = path.split_first()?;
    let field = fields.get(first)?;
    if rest.is_empty() {
        Some(field)
    } else {
        locate(&field.nested, rest)
    }
}

// =================================================================================================
// Editing: set_field_instruction / set_field_cached_result_text
// =================================================================================================

/// Sets the field at `path`'s own instruction. See [`crate::Document::set_field_instruction`] for
/// the full contract.
pub(crate) fn set_instruction(
    content: &mut Vec<ParagraphContent>,
    path: &[usize],
    text: &str,
    interner: &mut Interner,
) -> Result<(), DocxError> {
    let fields = parse_top(content, interner)?;
    let field =
        locate(&fields, path).ok_or_else(|| DocxError::FieldNotFound(format_field_path(path)))?;
    match &field.site {
        FieldSite::Simple { content_index } => {
            let Some(ParagraphContent::SimpleField(simple)) = content.get_mut(*content_index)
            else {
                unreachable!("FieldSite::Simple always names a ParagraphContent::SimpleField");
            };
            simple.set_instruction(interner, text);
            Ok(())
        }
        FieldSite::Complex {
            begin,
            separate,
            end,
            nested_in_instruction,
            ..
        } => {
            if *nested_in_instruction {
                return Err(DocxError::FieldHasNestedContent {
                    zone: "instruction",
                });
            }
            let boundary = separate.unwrap_or(*end);
            collapse_zone(content, *begin, boundary, text, interner, true);
            Ok(())
        }
    }
}

/// Sets the field at `path`'s own cached result. See
/// [`crate::Document::set_field_cached_result_text`] for the full contract.
pub(crate) fn set_cached_result_text(
    content: &mut Vec<ParagraphContent>,
    path: &[usize],
    text: &str,
    interner: &mut Interner,
) -> Result<(), DocxError> {
    let fields = parse_top(content, interner)?;
    let field =
        locate(&fields, path).ok_or_else(|| DocxError::FieldNotFound(format_field_path(path)))?;
    match &field.site {
        FieldSite::Simple { content_index } => {
            let Some(ParagraphContent::SimpleField(simple)) = content.get_mut(*content_index)
            else {
                unreachable!("FieldSite::Simple always names a ParagraphContent::SimpleField");
            };
            let interner_ref: &Interner = interner;
            if simple
                .content()
                .iter()
                .any(|item| complex_begin_present(item, interner_ref))
            {
                return Err(DocxError::FieldHasNestedContent {
                    zone: "cached result",
                });
            }
            let new_content = vec![ParagraphContent::Run(Run::with_text(interner, text))];
            *simple.content_mut() = new_content;
            Ok(())
        }
        FieldSite::Complex {
            separate,
            end,
            nested_in_result,
            ..
        } => {
            let Some(separate) = separate else {
                return Err(DocxError::FieldHasNoCachedResult);
            };
            if *nested_in_result {
                return Err(DocxError::FieldHasNestedContent {
                    zone: "cached result",
                });
            }
            collapse_zone(content, *separate, *end, text, interner, false);
            Ok(())
        }
    }
}

/// Whether `item` is (or, for a run, contains) a `w:fldChar begin` — a cheap, conservative check
/// used only to decide whether a simple field's own cached result is safe to collapse; the
/// authoritative nesting check for the complex form is [`FieldSite::Complex`]'s own
/// `nested_in_result` flag, computed once during [`parse_complex`].
fn complex_begin_present(item: &ParagraphContent, interner: &Interner) -> bool {
    match item {
        ParagraphContent::Run(run) => run.content().iter().any(|inner| {
            matches!(inner, RunInnerContent::ComplexFieldCharacter(fc)
                if matches!(fc.kind(interner), Ok(FieldCharacterType::Begin)))
        }),
        ParagraphContent::SimpleField(_) => true,
        _ => false,
    }
}

fn format_field_path(path: &[usize]) -> String {
    path.iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

/// Collapses every item strictly between `start` and `boundary` (exclusive both ends) to a single
/// new run holding `text` — the shared body of [`set_instruction`] (`is_instruction = true`, so the
/// new run holds a `w:instrText`, inserted right after `start`'s own run) and
/// [`set_cached_result_text`] (`is_instruction = false`, a `w:t`, inserted right after `start`).
/// Every `w:fldChar`/`w:instrText`/`w:t` item outside the (start, boundary) interval — including
/// `start` and `boundary` themselves — is left untouched, byte-for-byte, which is what keeps the
/// other half of the field (and every other field) byte-identical across this edit.
fn collapse_zone(
    content: &mut Vec<ParagraphContent>,
    start: Coord,
    boundary: Coord,
    text: &str,
    interner: &mut Interner,
    is_instruction: bool,
) {
    // 1. Collect the coordinates of every item to remove: for the instruction zone, `w:instrText`;
    //    for the result zone, `w:t`. Both are gathered from the read-only flatten pass, before any
    //    mutation, so their coordinates are still valid against `content` as it stands right now.
    let leaves = flatten(content, interner);
    let mut to_remove: Vec<Coord> = leaves
        .iter()
        .filter_map(|leaf| match leaf {
            Leaf::InstrText(coord, _) if is_instruction => Some(*coord),
            Leaf::Text(_) => None, // Leaf::Text carries no coord; handled via a second pass below.
            _ => None,
        })
        .collect();
    if !is_instruction {
        // `Leaf::Text` does not carry a `Coord` (nothing needed it before this write path), so the
        // result-zone items are found directly instead, by walking the same range `flatten` would.
        for (content_index, item) in content.iter().enumerate() {
            if content_index < start.content_index || content_index > boundary.content_index {
                continue;
            }
            if let ParagraphContent::Run(run) = item {
                for (run_index, inner) in run.content().iter().enumerate() {
                    let coord = Coord {
                        content_index,
                        run_index,
                    };
                    if !is_between(start, coord, boundary) {
                        continue;
                    }
                    if matches!(inner, RunInnerContent::Text(_)) {
                        to_remove.push(coord);
                    }
                }
            }
        }
    }
    to_remove.retain(|coord| is_between(start, *coord, boundary));

    // 2. Insert the new item right after `start`, within `start`'s own run — this never shifts any
    //    `content_index`, only `run_index` values within that one run that sit after `start`.
    let new_item = if is_instruction {
        RunInnerContent::FieldCode(new_text(interner, "instrText", text))
    } else {
        RunInnerContent::Text(new_text(interner, "t", text))
    };
    if let Some(ParagraphContent::Run(run)) = content.get_mut(start.content_index) {
        run.content_mut().insert(start.run_index + 1, new_item);
    }
    for coord in &mut to_remove {
        if coord.content_index == start.content_index && coord.run_index > start.run_index {
            coord.run_index += 1;
        }
    }

    // 3. Remove every collected item, grouped by run and processed content-index-descending so an
    //    emptied run's removal from `content` never invalidates a not-yet-processed, smaller index.
    let mut by_content: std::collections::BTreeMap<usize, std::collections::BTreeSet<usize>> =
        std::collections::BTreeMap::new();
    for coord in to_remove {
        by_content
            .entry(coord.content_index)
            .or_default()
            .insert(coord.run_index);
    }
    for (&content_index, run_indices) in by_content.iter().rev() {
        let Some(ParagraphContent::Run(run)) = content.get_mut(content_index) else {
            continue;
        };
        let items = run.content_mut();
        let mut index = 0usize;
        items.retain(|_| {
            let keep = !run_indices.contains(&index);
            index += 1;
            keep
        });
        // A run holding a `w:fldChar` marker (`start`'s own run, or `boundary`'s) never empties
        // here — its marker item is never a member of `run_indices` — so this only ever removes a
        // run that held nothing but the instruction/result content just collapsed.
        if items.is_empty() {
            content.remove(content_index);
        }
    }
}

fn is_between(start: Coord, coord: Coord, boundary: Coord) -> bool {
    start.key() < coord.key() && coord.key() < boundary.key()
}

/// Builds a fresh [`Text`] with wire name `local` (`"t"` or `"instrText"`) — [`Text::new`] always
/// hardcodes `"t"`, which is wrong for the instruction side of a collapse (see [`Text::with_local`]'s
/// own doc comment for why the wire name must match the destination `RunInnerContent` variant, not
/// just whichever one `Text::new` happens to build).
fn new_text(interner: &mut Interner, local: &str, text: &str) -> Text {
    let mut t = Text::with_local(interner, local);
    t.set_text(interner, text);
    t
}

/// Builds a run holding one `w:fldChar` of `kind` and nothing else — `crate::Document::
/// insert_form_field` (MJXOFF-121) uses this to build a fresh `begin`/`separate`/`end` triple; the
/// test module below reuses it rather than declaring a second copy.
pub(crate) fn marker_run(interner: &mut Interner, kind: FieldCharacterType) -> Run {
    let mut run = Run::with_text(interner, "");
    run.content_mut().clear();
    run.content_mut()
        .push(RunInnerContent::ComplexFieldCharacter(FieldCharacter::new(
            interner, kind,
        )));
    run
}

// =================================================================================================
// Paragraph::fields — the public read entry point
// =================================================================================================

impl super::body::Paragraph {
    /// Every field this paragraph's own top-level content holds, in document order — see [`Field`]'s
    /// own doc comment for the read model, and this module's own doc comment for how nesting is
    /// paired. `interner` must be the same one this paragraph was parsed with (the same rule every
    /// other typed accessor in this crate that needs one already follows).
    ///
    /// # Errors
    /// [`DocxError::UnbalancedField`] if a `w:fldChar` marker sequence anywhere in this paragraph's
    /// own content does not balance.
    pub fn fields(&self, interner: &Interner) -> Result<Vec<Field>, DocxError> {
        parse_top(self.content(), interner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::body::{Paragraph, Run};

    fn interner() -> Interner {
        Interner::new()
    }

    fn instr_run(interner: &mut Interner, text: &str) -> Run {
        Run::with_field_code(interner, text)
    }

    fn text_run(interner: &mut Interner, text: &str) -> Run {
        Run::with_text(interner, text)
    }

    fn paragraph_of(interner: &mut Interner, runs: Vec<Run>) -> Paragraph {
        let mut paragraph = Paragraph::new(interner);
        for run in runs {
            paragraph.append_run(run);
        }
        paragraph
    }

    #[test]
    fn a_simple_non_nested_field_reads_instruction_and_result_separately() {
        let mut interner = interner();
        let runs = vec![
            marker_run(&mut interner, FieldCharacterType::Begin),
            instr_run(&mut interner, " PAGE "),
            marker_run(&mut interner, FieldCharacterType::Separate),
            text_run(&mut interner, "3"),
            marker_run(&mut interner, FieldCharacterType::End),
        ];
        let paragraph = paragraph_of(&mut interner, runs);
        let fields = paragraph.fields(&interner).unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].instruction(), " PAGE ");
        assert_eq!(fields[0].field_name(), Some("PAGE"));
        assert_eq!(fields[0].cached_result(), Some("3"));
    }

    #[test]
    fn a_field_with_no_separate_reads_correctly_and_is_not_an_error() {
        let mut interner = interner();
        let runs = vec![
            marker_run(&mut interner, FieldCharacterType::Begin),
            instr_run(&mut interner, " DATE "),
            marker_run(&mut interner, FieldCharacterType::End),
        ];
        let paragraph = paragraph_of(&mut interner, runs);
        let fields = paragraph.fields(&interner).unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].instruction(), " DATE ");
        assert_eq!(fields[0].cached_result(), None);
    }

    #[test]
    fn an_instruction_split_across_three_runs_reads_as_one_instruction() {
        let mut interner = interner();
        let runs = vec![
            marker_run(&mut interner, FieldCharacterType::Begin),
            instr_run(&mut interner, " HYPER"),
            instr_run(&mut interner, "LINK "),
            instr_run(&mut interner, "\"http://example.com\" "),
            marker_run(&mut interner, FieldCharacterType::Separate),
            text_run(&mut interner, "example.com"),
            marker_run(&mut interner, FieldCharacterType::End),
        ];
        let paragraph = paragraph_of(&mut interner, runs);
        let fields = paragraph.fields(&interner).unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(
            fields[0].instruction(),
            " HYPERLINK \"http://example.com\" "
        );
        assert_eq!(fields[0].field_name(), Some("HYPERLINK"));
    }

    #[test]
    fn nested_pageref_fields_do_not_pollute_the_outer_tocs_own_instruction() {
        let mut interner = interner();
        // { TOC \o "1-3" }{ PAGEREF _Toc1 }, { PAGEREF _Toc2 } — an outer TOC field whose cached
        // result holds two nested PAGEREF fields, each with its own begin/separate/end.
        let runs = vec![
            marker_run(&mut interner, FieldCharacterType::Begin),
            instr_run(&mut interner, " TOC \\o \"1-3\" "),
            marker_run(&mut interner, FieldCharacterType::Separate),
            marker_run(&mut interner, FieldCharacterType::Begin),
            instr_run(&mut interner, " PAGEREF _Toc1 "),
            marker_run(&mut interner, FieldCharacterType::Separate),
            text_run(&mut interner, "1"),
            marker_run(&mut interner, FieldCharacterType::End),
            marker_run(&mut interner, FieldCharacterType::Begin),
            instr_run(&mut interner, " PAGEREF _Toc2 "),
            marker_run(&mut interner, FieldCharacterType::Separate),
            text_run(&mut interner, "2"),
            marker_run(&mut interner, FieldCharacterType::End),
            marker_run(&mut interner, FieldCharacterType::End),
        ];
        let paragraph = paragraph_of(&mut interner, runs);
        let fields = paragraph.fields(&interner).unwrap();
        assert_eq!(fields.len(), 1);
        let toc = &fields[0];
        assert_eq!(toc.instruction(), " TOC \\o \"1-3\" ");
        assert_eq!(toc.field_name(), Some("TOC"));
        assert_eq!(toc.cached_result(), Some("12"));
        assert_eq!(toc.nested_fields().len(), 2);
        assert_eq!(toc.nested_fields()[0].instruction(), " PAGEREF _Toc1 ");
        assert_eq!(toc.nested_fields()[0].cached_result(), Some("1"));
        assert_eq!(toc.nested_fields()[1].instruction(), " PAGEREF _Toc2 ");
        assert_eq!(toc.nested_fields()[1].cached_result(), Some("2"));
    }

    /// A marker-pairer that counts `begin`s/`end`s instead of nesting them cannot tell where the
    /// outer `TOC` field's own instruction ends — pasted here as the mutation this ticket's own trap
    /// asks for, proved red by running it against the fixture above.
    #[test]
    fn a_counting_pairer_would_report_the_wrong_outer_instruction() {
        let mut interner = interner();
        let runs = vec![
            marker_run(&mut interner, FieldCharacterType::Begin),
            instr_run(&mut interner, " TOC \\o \"1-3\" "),
            marker_run(&mut interner, FieldCharacterType::Separate),
            marker_run(&mut interner, FieldCharacterType::Begin),
            instr_run(&mut interner, " PAGEREF _Toc1 "),
            marker_run(&mut interner, FieldCharacterType::Separate),
            text_run(&mut interner, "1"),
            marker_run(&mut interner, FieldCharacterType::End),
            marker_run(&mut interner, FieldCharacterType::End),
        ];
        let paragraph = paragraph_of(&mut interner, runs);
        // The nesting-aware parser under test:
        let fields = paragraph.fields(&interner).unwrap();
        let correct_instruction = fields[0].instruction().to_owned();
        assert_eq!(correct_instruction, " TOC \\o \"1-3\" ");

        // The counting mutation: concatenate *every* w:instrText in the paragraph before the first
        // top-level `end`, which is what "pair by counting begins/ends, concatenate every instrText"
        // does — it cannot distinguish the outer field's own instruction from the nested one's.
        let leaves = flatten(paragraph.content(), &interner);
        let mut counted = String::new();
        for leaf in &leaves {
            if let Leaf::InstrText(_, text) = leaf {
                counted.push_str(text);
            }
        }
        assert_ne!(
            counted, correct_instruction,
            "a counting pairer's concatenation must disagree with the nesting-aware instruction \
             for this fixture to discriminate the two implementations"
        );
    }

    #[test]
    fn an_unbalanced_sequence_returns_a_typed_error() {
        let mut interner = interner();
        // begin, begin, end — one field never closes.
        let runs = vec![
            marker_run(&mut interner, FieldCharacterType::Begin),
            marker_run(&mut interner, FieldCharacterType::Begin),
            marker_run(&mut interner, FieldCharacterType::End),
        ];
        let paragraph = paragraph_of(&mut interner, runs);
        let error = paragraph.fields(&interner).unwrap_err();
        assert!(matches!(error, DocxError::UnbalancedField(_)));
    }

    #[test]
    fn an_unmatched_separate_returns_a_typed_error() {
        let mut interner = interner();
        let runs = vec![marker_run(&mut interner, FieldCharacterType::Separate)];
        let paragraph = paragraph_of(&mut interner, runs);
        let error = paragraph.fields(&interner).unwrap_err();
        assert!(matches!(error, DocxError::UnbalancedField(_)));
    }

    #[test]
    fn editing_the_instruction_leaves_the_cached_result_byte_identical() {
        let mut interner = interner();
        let runs = vec![
            marker_run(&mut interner, FieldCharacterType::Begin),
            instr_run(&mut interner, " PAGE "),
            marker_run(&mut interner, FieldCharacterType::Separate),
            text_run(&mut interner, "3"),
            marker_run(&mut interner, FieldCharacterType::End),
        ];
        let mut paragraph = paragraph_of(&mut interner, runs);
        let before_result_run = find_text_run(paragraph.content()).clone();
        set_instruction(paragraph.content_mut(), &[0], " NUMPAGES ", &mut interner).unwrap();
        let fields = paragraph.fields(&interner).unwrap();
        assert_eq!(fields[0].instruction(), " NUMPAGES ");
        assert_eq!(fields[0].cached_result(), Some("3"));
        // The result run — located by its own content, not a fixed top-level index, since
        // collapsing the (now-empty) instruction run removes that slot and shifts every index
        // after it — is untouched byte-for-byte: the very same run object.
        let after_result_run = find_text_run(paragraph.content());
        assert_eq!(&before_result_run, after_result_run);
    }

    /// The run holding this paragraph's own visible cached-result text (`w:t`), located by content
    /// rather than a fixed top-level index — [`collapse_zone`] removes a now-empty sibling run and
    /// shifts every later index, so tests that check "the same run, untouched" must not assume a
    /// stable position.
    fn find_text_run(content: &[ParagraphContent]) -> &Run {
        content
            .iter()
            .find_map(|item| match item {
                ParagraphContent::Run(run)
                    if run
                        .content()
                        .iter()
                        .any(|inner| matches!(inner, RunInnerContent::Text(_))) =>
                {
                    Some(run)
                }
                _ => None,
            })
            .expect("fixture always carries exactly one visible-text run")
    }

    #[test]
    fn editing_the_cached_result_leaves_the_instruction_byte_identical() {
        let mut interner = interner();
        let runs = vec![
            marker_run(&mut interner, FieldCharacterType::Begin),
            instr_run(&mut interner, " PAGE "),
            marker_run(&mut interner, FieldCharacterType::Separate),
            text_run(&mut interner, "3"),
            marker_run(&mut interner, FieldCharacterType::End),
        ];
        let mut paragraph = paragraph_of(&mut interner, runs);
        let before_instr_run = find_field_code_run(paragraph.content()).clone();
        set_cached_result_text(paragraph.content_mut(), &[0], "42", &mut interner).unwrap();
        let fields = paragraph.fields(&interner).unwrap();
        assert_eq!(fields[0].instruction(), " PAGE ");
        assert_eq!(fields[0].cached_result(), Some("42"));
        let after_instr_run = find_field_code_run(paragraph.content());
        assert_eq!(&before_instr_run, after_instr_run);
    }

    /// [`find_text_run`]'s own counterpart for the instruction side (`w:instrText`).
    fn find_field_code_run(content: &[ParagraphContent]) -> &Run {
        content
            .iter()
            .find_map(|item| match item {
                ParagraphContent::Run(run)
                    if run
                        .content()
                        .iter()
                        .any(|inner| matches!(inner, RunInnerContent::FieldCode(_))) =>
                {
                    Some(run)
                }
                _ => None,
            })
            .expect("fixture always carries exactly one instrText-bearing run")
    }

    #[test]
    fn setting_an_over_long_form_field_name_is_refused() {
        let mut interner = interner();
        let mut data = FormFieldData::new(&mut interner);
        let too_long = "x".repeat(66);
        let error = data.set_name(&mut interner, &too_long).unwrap_err();
        assert!(matches!(
            error,
            DocxError::ValueTooLong {
                field: "form field name",
                max: 65,
                len: 66
            }
        ));
    }

    #[test]
    fn setting_an_over_long_help_text_is_refused() {
        let mut interner = interner();
        let mut data = FormFieldData::new(&mut interner);
        let too_long = "x".repeat(257);
        let error = data
            .set_help_text(&mut interner, None, &too_long)
            .unwrap_err();
        assert!(matches!(
            error,
            DocxError::ValueTooLong {
                field: "help text",
                max: 256,
                len: 257
            }
        ));
    }

    #[test]
    fn setting_an_over_long_status_text_is_refused() {
        let mut interner = interner();
        let mut data = FormFieldData::new(&mut interner);
        let too_long = "x".repeat(141);
        let error = data
            .set_status_text(&mut interner, None, &too_long)
            .unwrap_err();
        assert!(matches!(
            error,
            DocxError::ValueTooLong {
                field: "status text",
                max: 140,
                len: 141
            }
        ));
    }

    #[test]
    fn setting_an_over_long_macro_name_is_refused() {
        let mut interner = interner();
        let mut data = FormFieldData::new(&mut interner);
        let too_long = "x".repeat(34);
        let error = data.set_entry_macro(&mut interner, &too_long).unwrap_err();
        assert!(matches!(
            error,
            DocxError::ValueTooLong {
                field: "macro name",
                max: 33,
                len: 34
            }
        ));
    }

    #[test]
    fn a_checkbox_form_field_round_trips_through_ffdata() {
        let mut interner = interner();
        let mut data = FormFieldData::new(&mut interner);
        data.set_name(&mut interner, "Check1").unwrap();
        data.set_enabled(&mut interner, Some(true));
        let mut checkbox = FormFieldCheckBox::new(&mut interner);
        checkbox.set_checked(&mut interner, Some(true));
        checkbox.set_default_checked(&mut interner, Some(false));
        data.set_check_box(Some(checkbox));

        assert_eq!(data.name(&interner), Some("Check1".to_owned()));
        assert_eq!(data.enabled(&interner), Some(true));
        let checkbox = data.check_box().unwrap();
        assert_eq!(checkbox.checked(&interner), Some(true));
        assert_eq!(checkbox.default_checked(&interner), Some(false));
    }

    #[test]
    fn a_drop_down_form_field_round_trips_its_entries_and_selection() {
        let mut interner = interner();
        let mut data = FormFieldData::new(&mut interner);
        data.set_name(&mut interner, "List1").unwrap();
        let mut list = FormFieldDropDownList::new(&mut interner, &["One", "Two", "Three"]);
        list.set_selected_index(&mut interner, 1);
        data.set_drop_down_list(Some(list));

        let list = data.drop_down_list().unwrap();
        assert_eq!(
            list.entries(&interner).collect::<Vec<_>>(),
            vec!["One".to_owned(), "Two".to_owned(), "Three".to_owned()]
        );
        assert_eq!(list.selected_index(&interner), Some(1));
    }

    #[test]
    fn a_text_input_form_field_round_trips_its_kind_and_default_text() {
        let mut interner = interner();
        let mut data = FormFieldData::new(&mut interner);
        data.set_name(&mut interner, "Text1").unwrap();
        let mut input = FormFieldTextInput::new(&mut interner, FormFieldTextType::Regular);
        input.set_default_text(&mut interner, "placeholder");
        input.set_max_length(&mut interner, 40);
        data.set_text_input(Some(input));

        let input = data.text_input().unwrap();
        assert_eq!(input.kind(&interner), Some(FormFieldTextType::Regular));
        assert_eq!(
            input.default_text(&interner),
            Some("placeholder".to_owned())
        );
        assert_eq!(input.max_length(&interner), Some(40));
    }
}
