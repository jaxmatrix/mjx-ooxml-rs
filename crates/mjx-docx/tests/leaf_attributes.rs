//! MJXOFF-152's "Done when": every attribute accessor `crates/mjx-docx/src/document/body.rs`
//! declares (MJXOFF-92) returns the value real markup states, asserted **per accessor**, against
//! `wml.xsd`-prefixed markup — never against markup this crate's own writer produced, which would
//! let a wrong wire spelling agree with itself.
//!
//! `wml.xsd` is `attributeFormDefault="qualified"`, so every attribute Word writes on these elements
//! is prefixed (`w:font`, `w:alignment`, …), never bare. MJXOFF-92 declared its leaf types'
//! attributes with no prefix, so every accessor below matched nothing and answered `None` (or the
//! schema default) against files that plainly carry the value — confirmed by neutralising the fix
//! and watching every test below that names an affected attribute go red (see this file's own
//! bottom section).
//!
//! # Why this file walks the raw tree instead of going through `Document`
//!
//! `Run`/`Paragraph`/`Body` expose only the handful of members MJXOFF-92 built a reading/editing
//! surface for (`run_properties`, `text`, `run`/`paragraph` and friends) — none of them hand back a
//! `Break`, `Symbol`, `PositionalTab`, `ProofingError`, `PermissionRangeStart`,
//! `PermissionRangeEnd`, `PhoneticGuideTextAlignment` or bare `Text` value, even though every one of
//! those types and their accessors is `pub`. Reaching one from outside this crate today means
//! parsing `word/document.xml`'s raw tree directly (exactly the technique
//! `crates/mjx-docx/tests/wml_child_order.rs` already uses to build markup, and
//! `document::tests::sibling_paragraph_span` uses, same-crate, to inspect it) and calling the leaf
//! type's own [`FromXml::from_xml`] on the [`RawElement`] found — every piece of that is public API
//! (`Package::part_tree`, `RawDocument`, `RawElement`, `RawNode`, `Interner::resolve`, `FromXml`),
//! so no new modelling is added to reach it. That gap — nothing in `Document`'s own surface can
//! reach these seven types — is a real one, but adding accessors for it is out of this ticket's
//! scope (see MJXOFF-152's "not in scope": no new modelling); it is reported on the ticket instead.
//!
//! # Why this is what actually catches the private-codec-type defect
//!
//! [`Symbol::character`] and [`Text::preserve_whitespace`] name a local `AttributeCodec` tag type
//! (`ShortHex`, `WhitespacePreservation`) in their return type. A unit test living inside
//! `document::body`'s own module can call them even if those tag types are private — same-module
//! visibility hides the defect completely. This file is a **separate compilation unit** (a Cargo
//! integration test, its own crate), so the exact same call only compiles if the tag types are
//! `pub` and re-exported — which is what turns "private type in public interface" from a latent
//! defect into `cargo build`'s own `E0446`.

use mjx_docx::{
    Break, PermissionRangeEnd, PermissionRangeStart, PhoneticGuideTextAlignment, PositionalTab,
    ProofingError, Symbol, Text,
};
use mjx_fixtures::fixture;
use mjx_ooxml_core::{FromXml, Interner, RawElement, RawNode};
use mjx_ooxml_types::wordprocessingml::{
    BreakTextWrappingRestart, BreakType, DisplacedByCustomXml, EditingGroup,
    FourDigitHexadecimalNumber, PhoneticGuideAlignment, PositionalTabAlignment, PositionalTabBase,
    PositionalTabLeader, ProofingErrorType,
};
use mjx_opc::{Package, PartName};

const DOCUMENT_XML: &str = "/word/document.xml";

/// The first element among `element` itself and its descendants (depth-first, document order)
/// whose local name is `local`.
fn find_first<'a>(
    element: &'a RawElement,
    interner: &Interner,
    local: &str,
) -> Option<&'a RawElement> {
    if interner.resolve(element.name.local) == local {
        return Some(element);
    }
    element.children.iter().find_map(|child| match child {
        RawNode::Element(child) => find_first(child, interner, local),
        _ => None,
    })
}

/// [`find_first`], panicking with `local` named if nothing matches — every call site below asserts
/// the fixture actually carries the element it claims to.
fn find_or_panic<'a>(root: &'a RawElement, interner: &'a Interner, local: &str) -> &'a RawElement {
    find_first(root, interner, local).unwrap_or_else(|| panic!("no <w:{local}> in this document"))
}

/// `run_content.docx`'s `<w:sym w:font="Wingdings" w:char="F0E0"/>` (MJXOFF-152's own motivating
/// example): both attributes read back, from a file this crate did not write.
///
/// **Would this pass if the work were not done?** No — before `prefix = "w"` was added, both of
/// these read `Ok(None)`: the grammar matched only a bare, unprefixed `font`/`char`, which
/// `<w:sym w:font="…" w:char="…"/>` never carries.
#[test]
fn run_content_docx_symbol_reads_its_font_and_character() {
    let mut package = Package::open(&fixture("run_content.docx")).expect("open the fixture");
    let part = PartName::new(DOCUMENT_XML).expect("a valid part name");
    let doc = package.part_tree(&part).expect("read word/document.xml");
    let interner = &doc.interner;

    let element = find_or_panic(&doc.root, interner, "sym");
    let symbol = Symbol::from_xml(element, interner).expect("parse w:sym");

    assert_eq!(
        symbol.font(interner),
        Ok(Some(std::borrow::Cow::Borrowed("Wingdings"))),
        "w:font"
    );
    assert_eq!(
        symbol.character(interner),
        Ok(Some(FourDigitHexadecimalNumber::from_wire("F0E0"))),
        "w:char"
    );
}

/// `run_content.docx`'s `<w:ptab w:alignment="right" w:relativeTo="margin" w:leader="dot"/>` — all
/// three required attributes, every one enumerated (not the identity codec), so a wrong prefix
/// would have surfaced as a `Missing` error rather than a silently wrong string.
///
/// **Would this pass if the work were not done?** No — all three are `required`, so before the fix
/// every one of these three calls returned `Err(AttributeError::Missing { .. })` instead of `Ok`.
#[test]
fn run_content_docx_positional_tab_reads_alignment_relative_to_and_leader() {
    let mut package = Package::open(&fixture("run_content.docx")).expect("open the fixture");
    let part = PartName::new(DOCUMENT_XML).expect("a valid part name");
    let doc = package.part_tree(&part).expect("read word/document.xml");
    let interner = &doc.interner;

    let element = find_or_panic(&doc.root, interner, "ptab");
    let ptab = PositionalTab::from_xml(element, interner).expect("parse w:ptab");

    assert_eq!(
        ptab.alignment(interner),
        Ok(PositionalTabAlignment::Right),
        "w:alignment"
    );
    assert_eq!(
        ptab.relative_to(interner),
        Ok(PositionalTabBase::Margin),
        "w:relativeTo"
    );
    assert_eq!(
        ptab.leader(interner),
        Ok(PositionalTabLeader::Dot),
        "w:leader"
    );
}

/// `run_content.docx`'s `<w:rubyAlign w:val="center"/>`, inside its one `w:ruby`.
///
/// **Would this pass if the work were not done?** No — `val` is `required`; before the fix this
/// returned `Err(AttributeError::Missing { .. })`.
#[test]
fn run_content_docx_ruby_alignment_reads_its_value() {
    let mut package = Package::open(&fixture("run_content.docx")).expect("open the fixture");
    let part = PartName::new(DOCUMENT_XML).expect("a valid part name");
    let doc = package.part_tree(&part).expect("read word/document.xml");
    let interner = &doc.interner;

    let element = find_or_panic(&doc.root, interner, "rubyAlign");
    let alignment =
        PhoneticGuideTextAlignment::from_xml(element, interner).expect("parse w:rubyAlign");

    assert_eq!(
        alignment.value(interner),
        Ok(PhoneticGuideAlignment::Center),
        "w:val"
    );
}

/// `run_content.docx`'s `<w:t xml:space="preserve">  leading space, </w:t>` — the one attribute
/// MJXOFF-92's own ticket named a prefix for (`xml`, not `w`), so this accessor was never part of
/// the missing-`prefix` defect. It is still exercised here because its codec
/// ([`mjx_docx::WhitespacePreservation`]) is the second local tag type this ticket found private —
/// [`Text::preserve_whitespace`] would not compile in this separate compilation unit until it was
/// made `pub` and re-exported.
///
/// **Would this pass if the work were not done?** For the *value* — yes; `xml:space`'s prefix was
/// always correct, so this call always returned `Ok(Some(true))`. For the *file compiling at
/// all* — no: `Text::preserve_whitespace`'s return type names the private `WhitespacePreservation`
/// tag type, so before that type was made `pub`, this whole test file failed to build with
/// `error: type `mjx_docx::document::body::WhitespacePreservation` is private` (confirmed while
/// developing this fix, reproduced in this ticket's own report).
#[test]
fn run_content_docx_leading_run_text_reads_xml_space_preserve() {
    let mut package = Package::open(&fixture("run_content.docx")).expect("open the fixture");
    let part = PartName::new(DOCUMENT_XML).expect("a valid part name");
    let doc = package.part_tree(&part).expect("read word/document.xml");
    let interner = &doc.interner;

    let element = find_or_panic(&doc.root, interner, "t");
    let text = Text::from_xml(element, interner).expect("parse w:t");

    assert_eq!(
        text.preserve_whitespace(interner),
        Ok(Some(true)),
        "xml:space"
    );
}

/// `tests/fixtures/leaf_attributes.docx`'s `<w:br w:type="page" w:clear="all"/>` — a dedicated
/// fixture, because neither `run_content.docx` nor `sample.docx` carries a `w:br` with either
/// attribute set (`run_content.docx`'s own is bare, `<w:br/>`, which cannot discriminate this
/// defect: an absent attribute reads `Ok(None)` whether or not its prefix is right). See this
/// file's module doc and MJXOFF-152's own report for why a new fixture was needed here and not for
/// `Symbol`/`PositionalTab`/`PhoneticGuideTextAlignment` above.
///
/// **Would this pass if the work were not done?** No — before the fix both attributes read
/// `Ok(None)` against this fixture's `w:type="page" w:clear="all"`.
#[test]
fn leaf_attributes_docx_break_reads_type_and_clear() {
    let mut package = Package::open(&fixture("leaf_attributes.docx")).expect("open the fixture");
    let part = PartName::new(DOCUMENT_XML).expect("a valid part name");
    let doc = package.part_tree(&part).expect("read word/document.xml");
    let interner = &doc.interner;

    let element = find_or_panic(&doc.root, interner, "br");
    let br = Break::from_xml(element, interner).expect("parse w:br");

    assert_eq!(br.kind(interner), Ok(Some(BreakType::Page)), "w:type");
    assert_eq!(
        br.clear(interner),
        Ok(Some(BreakTextWrappingRestart::All)),
        "w:clear"
    );
}

/// `leaf_attributes.docx`'s `<w:proofErr w:type="spellStart"/>`.
///
/// **Would this pass if the work were not done?** No — `type` is `required`; before the fix this
/// returned `Err(AttributeError::Missing { .. })`.
#[test]
fn leaf_attributes_docx_proofing_error_reads_its_type() {
    let mut package = Package::open(&fixture("leaf_attributes.docx")).expect("open the fixture");
    let part = PartName::new(DOCUMENT_XML).expect("a valid part name");
    let doc = package.part_tree(&part).expect("read word/document.xml");
    let interner = &doc.interner;

    let element = find_or_panic(&doc.root, interner, "proofErr");
    let proof_err = ProofingError::from_xml(element, interner).expect("parse w:proofErr");

    assert_eq!(
        proof_err.error_type(interner),
        Ok(ProofingErrorType::SpellingStart),
        "w:type"
    );
}

/// `leaf_attributes.docx`'s
/// `<w:permStart w:id="100" w:displacedByCustomXml="next" w:edGrp="everyone" w:ed="alice"
/// w:colFirst="1" w:colLast="3"/>` — all six attributes.
///
/// **Would this pass if the work were not done?** No — `id` is `required` (`Err(Missing)` before the
/// fix); the other five are optional and each read `Ok(None)` before the fix despite being present.
#[test]
fn leaf_attributes_docx_permission_range_start_reads_every_attribute() {
    let mut package = Package::open(&fixture("leaf_attributes.docx")).expect("open the fixture");
    let part = PartName::new(DOCUMENT_XML).expect("a valid part name");
    let doc = package.part_tree(&part).expect("read word/document.xml");
    let interner = &doc.interner;

    let element = find_or_panic(&doc.root, interner, "permStart");
    let start = PermissionRangeStart::from_xml(element, interner).expect("parse w:permStart");

    assert_eq!(
        start.id(interner),
        Ok(std::borrow::Cow::Borrowed("100")),
        "w:id"
    );
    assert_eq!(
        start.displaced_by_custom_xml(interner),
        Ok(Some(DisplacedByCustomXml::Next)),
        "w:displacedByCustomXml"
    );
    assert_eq!(
        start.editing_group(interner),
        Ok(Some(EditingGroup::Everyone)),
        "w:edGrp"
    );
    assert_eq!(
        start.editor(interner),
        Ok(Some(std::borrow::Cow::Borrowed("alice"))),
        "w:ed"
    );
    assert_eq!(start.first_column(interner), Ok(Some(1)), "w:colFirst");
    assert_eq!(start.last_column(interner), Ok(Some(3)), "w:colLast");
}

/// `leaf_attributes.docx`'s `<w:permEnd w:id="100" w:displacedByCustomXml="prev"/>` — deliberately a
/// *different* `w:displacedByCustomXml` value (`prev`, not `next`) from `w:permStart`'s, so a codec
/// or accessor mix-up between the two `CT_Perm`-shaped types would show up as a wrong enum variant,
/// not a coincidentally-matching one.
///
/// **Would this pass if the work were not done?** No — `id` is `required` (`Err(Missing)` before the
/// fix); `displacedByCustomXml` read `Ok(None)` before the fix despite being present.
#[test]
fn leaf_attributes_docx_permission_range_end_reads_id_and_displaced_by_custom_xml() {
    let mut package = Package::open(&fixture("leaf_attributes.docx")).expect("open the fixture");
    let part = PartName::new(DOCUMENT_XML).expect("a valid part name");
    let doc = package.part_tree(&part).expect("read word/document.xml");
    let interner = &doc.interner;

    let element = find_or_panic(&doc.root, interner, "permEnd");
    let end = PermissionRangeEnd::from_xml(element, interner).expect("parse w:permEnd");

    assert_eq!(
        end.id(interner),
        Ok(std::borrow::Cow::Borrowed("100")),
        "w:id"
    );
    assert_eq!(
        end.displaced_by_custom_xml(interner),
        Ok(Some(DisplacedByCustomXml::Previous)),
        "w:displacedByCustomXml"
    );
}
