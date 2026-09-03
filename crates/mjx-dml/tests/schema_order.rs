//! Schema-order placement, through the public API only: a child this library writes lands at its
//! `xsd:sequence` rank, and markup this library does **not** model keeps its place.
//!
//! # Why the fixtures are shaped the way they are
//!
//! An ordering test whose fixture already holds the answer proves nothing — a writer that simply
//! appended, or one that simply prepended, would pass it. Every fixture here therefore leaves a
//! **gap** in the sequence and asks for a child that belongs in the gap:
//!
//! * `a:lnSpc` is rank 0 and `a:defRPr` is rank 8; a new `a:spcBef` is rank 1. An appending writer
//!   puts it after `a:defRPr`, a prepending one before `a:lnSpc`, and only a rank-driven one puts it
//!   between them. Both failure modes are visible in the same assertion.
//! * The unmodelled element sits **between** those two, so a writer that rebuilt the child list from
//!   its typed model — or that sorted it — would drop it or move it, and a writer that treated it as
//!   a boundary would push the new child past `a:defRPr`.

use mjx_dml::diagram::{LayoutVariables, TraversalDirection};
use mjx_dml::{
    ParagraphProperties, ParagraphPropertiesSpec, TextListStyle, TextPoint, TextSpacing,
};
use mjx_ooxml_core::{FromXml, RawDocument, ToXml};
use mjx_ooxml_types::child_order::{
    LAYOUT_VARIABLE_PROPERTY_SET, TEXT_LIST_STYLE, TEXT_PARAGRAPH_PROPERTIES,
};
use mjx_xml::fidelity;

/// A `a:pPr` holding the two ends of the sequence with a caller's own element between them.
const PARAGRAPH_WITH_FOREIGN_CHILD: &[u8] = br#"<a:pPr xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:x="urn:example:caller"><a:lnSpc><a:spcPct val="150000"/></a:lnSpc><x:note kind='keep-me'>caller markup &amp; entities</x:note><a:defRPr sz="1800"/></a:pPr>"#;

fn parse<T: FromXml>(fragment: &[u8]) -> (T, RawDocument) {
    let document = fidelity::parse(fragment).expect("fragment parses");
    let typed = T::from_xml(&document.root, &document.interner).expect("from_xml");
    (typed, document)
}

fn serialize<T: ToXml>(typed: &T, mut document: RawDocument) -> String {
    document.root = typed.to_xml(&mut document.interner);
    String::from_utf8(fidelity::serialize_to_vec(&document)).expect("UTF-8")
}

/// The order of the interesting element names in `xml`, by first occurrence.
fn sequence_of(xml: &str, names: &[&str]) -> Vec<String> {
    let mut found: Vec<(usize, String)> = names
        .iter()
        .filter_map(|name| {
            xml.find(&format!("<{name}"))
                .map(|at| (at, (*name).to_owned()))
        })
        .collect();
    found.sort_by_key(|(at, _)| *at);
    found.into_iter().map(|(_, name)| name).collect()
}

#[test]
fn a_new_child_lands_in_the_gap_the_sequence_leaves_for_it() {
    let (mut properties, mut document) = parse::<ParagraphProperties>(PARAGRAPH_WITH_FOREIGN_CHILD);
    properties.apply(
        &ParagraphPropertiesSpec::new()
            .with_space_before(TextSpacing::Points(TextPoint::from_points(6.0))),
        &mut document.interner,
    );
    let xml = serialize(&properties, document);

    assert_eq!(
        sequence_of(&xml, &["a:lnSpc", "a:spcBef", "a:defRPr"]),
        vec!["a:lnSpc", "a:spcBef", "a:defRPr"],
        "`a:spcBef` is rank 1: appending it after `a:defRPr` or prepending it before `a:lnSpc` are \
         both wrong, and both would show here\n{xml}"
    );
    // Stated as ranks too, so the assertion above cannot drift away from the schema it stands for.
    assert!(
        TEXT_PARAGRAPH_PROPERTIES.rank_of(None, "lnSpc")
            < TEXT_PARAGRAPH_PROPERTIES.rank_of(None, "spcBef")
            && TEXT_PARAGRAPH_PROPERTIES.rank_of(None, "spcBef")
                < TEXT_PARAGRAPH_PROPERTIES.rank_of(None, "defRPr")
    );
}

#[test]
fn unmodelled_markup_between_two_modelled_children_keeps_its_anchors_and_its_bytes() {
    let (mut properties, mut document) = parse::<ParagraphProperties>(PARAGRAPH_WITH_FOREIGN_CHILD);
    properties.apply(
        &ParagraphPropertiesSpec::new()
            .with_space_before(TextSpacing::Points(TextPoint::from_points(6.0)))
            .with_space_after(TextSpacing::Points(TextPoint::from_points(3.0))),
        &mut document.interner,
    );
    let xml = serialize(&properties, document);

    assert!(
        xml.contains(r#"<x:note kind='keep-me'>caller markup &amp; entities</x:note>"#),
        "the caller's element must survive verbatim — prefix, single quotes and entity spelling \
         included\n{xml}"
    );
    assert_eq!(
        sequence_of(&xml, &["a:lnSpc", "x:note", "a:defRPr"]),
        vec!["a:lnSpc", "x:note", "a:defRPr"],
        "the caller's element must still stand between the same two modelled neighbours\n{xml}"
    );
    assert_eq!(
        sequence_of(&xml, &["a:lnSpc", "a:spcBef", "a:spcAft", "a:defRPr"]),
        vec!["a:lnSpc", "a:spcBef", "a:spcAft", "a:defRPr"],
        "and the modelled children are in schema order around it\n{xml}"
    );
}

#[test]
fn an_untouched_element_carrying_unmodelled_markup_round_trips_byte_identically() {
    // Reading is never a reason to rewrite. Parsing and re-serializing without an edit must return
    // the caller's bytes exactly — the ordering table has no say over a document nobody changed.
    let (properties, document) = parse::<ParagraphProperties>(PARAGRAPH_WITH_FOREIGN_CHILD);
    assert_eq!(
        serialize(&properties, document).as_bytes(),
        PARAGRAPH_WITH_FOREIGN_CHILD
    );
}

/// A `a:lstStyle` whose levels are written **in reverse schema order**, with a caller's element in
/// the middle. This is the shape that makes a rank-less writer look correct if it appends and the
/// fixture happens to be reversed, so it is exactly the shape worth pinning.
const REVERSED_LIST_STYLE: &[u8] = br#"<a:lstStyle xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:x="urn:example:caller"><a:lvl9pPr marL="0"/><x:note/><a:lvl1pPr marL="0"/></a:lstStyle>"#;

#[test]
fn a_level_is_placed_by_rank_even_when_the_existing_levels_are_reversed() {
    let (mut style, mut document) = parse::<TextListStyle>(REVERSED_LIST_STYLE);
    style.set_level(
        &mut document.interner,
        mjx_dml::IndentLevel::of(4),
        &ParagraphPropertiesSpec::new().with_left_margin_points(36.0),
    );
    let xml = serialize(&style, document);

    // The file's own children are never reordered — `a:lvl9pPr` still precedes `a:lvl1pPr`, because
    // reordering a caller's document would be corruption, not a fix.
    assert_eq!(
        sequence_of(&xml, &["a:lvl9pPr", "a:lvl1pPr"]),
        vec!["a:lvl9pPr", "a:lvl1pPr"],
        "the caller's existing order must be left exactly as it was\n{xml}"
    );
    // No placement can satisfy a contradictory input: `a:lvl5pPr` must follow `a:lvl1pPr` and
    // precede `a:lvl9pPr`, and this file writes them the other way round. Placement honours the
    // half it can — it goes before every sibling that must follow it — which an appending writer,
    // that would put it last of all, does not.
    assert_eq!(
        sequence_of(&xml, &["a:lvl5pPr", "a:lvl9pPr"]),
        vec!["a:lvl5pPr", "a:lvl9pPr"],
        "the new level must precede the level it has to come before\n{xml}"
    );
    assert!(
        !xml.trim_end()
            .ends_with("<a:lvl5pPr marL=\"457200\"/></a:lstStyle>"),
        "appending would have passed the neighbours check by accident\n{xml}"
    );
    assert!(
        xml.contains("<x:note/>"),
        "the caller's element survives\n{xml}"
    );
    assert_eq!(TEXT_LIST_STYLE.rank_of(None, "lvl5pPr"), Some(5));
}

// ---------------------------------------------------------------------------------------------
// DrawingML Diagram — dgm:varLst (CT_LayoutVariablePropertySet)
// ---------------------------------------------------------------------------------------------
//
// `LayoutVariables` (`dgm:varLst`) used to place its nine named children with a rank array
// hand-copied into `mjx-dml` itself — exactly the kind of table A7c deleted fourteen of. It now goes
// through the generated `LAYOUT_VARIABLE_PROPERTY_SET` table the same way `TextListStyle` goes
// through `TEXT_LIST_STYLE` above, so this is the same test, once more, for the ninth schema.

/// A `dgm:varLst` holding the two ends of the sequence (`orgChart` rank 0, `resizeHandles` rank 8)
/// with a caller's own element between them.
const VAR_LIST_WITH_FOREIGN_CHILD: &[u8] = br#"<dgm:varLst xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" xmlns:x="urn:example:caller"><dgm:orgChart val="1"/><x:note kind='keep-me'>caller markup &amp; entities</x:note><dgm:resizeHandles val="exact"/></dgm:varLst>"#;

#[test]
fn a_new_layout_variable_lands_in_the_gap_the_sequence_leaves_for_it() {
    let (mut variables, mut document) = parse::<LayoutVariables>(VAR_LIST_WITH_FOREIGN_CHILD);
    variables.set_direction(&mut document.interner, Some(TraversalDirection::Reversed));
    let xml = serialize(&variables, document);

    // `dir` is rank 4: an appending writer puts it after `resizeHandles` (rank 8), a prepending one
    // before `orgChart` (rank 0), and only a rank-driven one puts it between them — specifically
    // right after `orgChart`, since nothing else named is between the two.
    assert_eq!(
        sequence_of(&xml, &["dgm:orgChart", "dgm:dir", "dgm:resizeHandles"]),
        vec!["dgm:orgChart", "dgm:dir", "dgm:resizeHandles"],
        "`dgm:dir` is rank 4: appending it after `dgm:resizeHandles` or prepending it before \
         `dgm:orgChart` are both wrong, and both would show here\n{xml}"
    );
    // The caller's own element is never reordered, and keeps its own bytes exactly.
    assert!(
        xml.contains(r#"<x:note kind='keep-me'>caller markup &amp; entities</x:note>"#),
        "the caller's element must survive verbatim\n{xml}"
    );
    // Stated as ranks too, so the assertion above cannot drift away from the schema it stands for —
    // and confirming this is the generated table, not a hand-rolled one.
    assert_eq!(
        LAYOUT_VARIABLE_PROPERTY_SET.symbol,
        "CT_LayoutVariablePropertySet"
    );
    assert_eq!(
        LAYOUT_VARIABLE_PROPERTY_SET.rank_of(None, "orgChart"),
        Some(0)
    );
    assert_eq!(LAYOUT_VARIABLE_PROPERTY_SET.rank_of(None, "dir"), Some(4));
    assert_eq!(
        LAYOUT_VARIABLE_PROPERTY_SET.rank_of(None, "resizeHandles"),
        Some(8)
    );
}
