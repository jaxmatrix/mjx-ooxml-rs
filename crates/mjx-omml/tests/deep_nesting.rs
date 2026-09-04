//! The fixture this crate exists to pass: a fraction whose numerator is itself a radical containing
//! an n-ary with sub/superscripts, a 2×2 matrix whose cells hold their own equations, and a
//! delimiter with three arguments and non-default characters — the shape a model that treats every
//! math object as an opaque container cannot round-trip or nest correctly. A separate compilation
//! unit (an integration test, not a `#[cfg(test)]` module), per MJXOFF-152's own finding that a
//! same-module unit test cannot catch a codec/prefix bug the real reader would.

use mjx_omml::{Math, MathElement};
use mjx_ooxml_core::{FromXml, RawNode};

const FIXTURE: &str = r##"<m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <m:f>
    <m:num>
      <m:rad>
        <m:deg><m:r><m:t>3</m:t></m:r></m:deg>
        <m:e>
          <m:nary>
            <m:naryPr><m:chr m:val="&#8721;"/></m:naryPr>
            <m:sub><m:r><m:t>i=1</m:t></m:r></m:sub>
            <m:sup><m:r><m:t>n</m:t></m:r></m:sup>
            <m:e><m:r><m:t>x</m:t></m:r></m:e>
          </m:nary>
        </m:e>
      </m:rad>
    </m:num>
    <m:den><m:r><m:t>2</m:t></m:r></m:den>
  </m:f>
  <m:m>
    <m:mr>
      <m:e><m:r><m:t>a</m:t></m:r></m:e>
      <m:e><m:r><m:t>b</m:t></m:r></m:e>
    </m:mr>
    <m:mr>
      <m:e><m:r><m:t>c</m:t></m:r></m:e>
      <m:e><m:r><m:t>d</m:t></m:r></m:e>
    </m:mr>
  </m:m>
  <m:d>
    <m:dPr><m:begChr m:val="["/><m:endChr m:val="]"/></m:dPr>
    <m:e><m:r><m:t>x</m:t></m:r></m:e>
    <m:e><m:r><m:t>y</m:t></m:r></m:e>
    <m:e><m:r><m:t>z</m:t></m:r></m:e>
  </m:d>
</m:oMath>"##;

/// Reads the deeply-nested fixture, asserting the exact nesting a flat/opaque-container model would
/// get wrong: the fraction's own numerator is a radical, whose own radicand is an n-ary operator with
/// both a lower and upper limit; the second top-level element is a 2×2 matrix; the third is a
/// three-argument delimiter with non-default `[`/`]` characters.
#[test]
fn the_deeply_nested_fixture_reads_its_exact_structure() {
    let doc = mjx_xml::fidelity::parse(FIXTURE.as_bytes()).expect("parse the fixture");
    let math = Math::from_xml(&doc.root, &doc.interner).expect("read m:oMath");
    let elements = math.elements(&doc.interner);
    assert_eq!(elements.len(), 3, "fraction, matrix, delimiter");

    // --- Fraction -> Radical -> n-ary, three levels of nesting -----------------------------------
    let MathElement::Fraction(fraction) = &elements[0] else {
        panic!("expected a fraction, got {:?}", elements[0]);
    };
    let numerator = fraction.numerator(&doc.interner).expect("m:num is present");
    let numerator_elements = numerator.elements(&doc.interner);
    assert_eq!(numerator_elements.len(), 1, "the numerator is one radical");
    let MathElement::Radical(radical) = &numerator_elements[0] else {
        panic!(
            "expected the numerator to be a radical, got {:?}",
            numerator_elements[0]
        );
    };
    let degree = radical.degree(&doc.interner).expect("m:deg is present");
    assert_eq!(text_of(&degree, &doc), "3");

    let radicand = radical.radicand(&doc.interner).expect("m:e is present");
    let radicand_elements = radicand.elements(&doc.interner);
    assert_eq!(
        radicand_elements.len(),
        1,
        "the radicand is one n-ary operator"
    );
    let MathElement::NaryOperator(nary) = &radicand_elements[0] else {
        panic!(
            "expected the radicand to be an n-ary operator, got {:?}",
            radicand_elements[0]
        );
    };
    let properties = nary.properties(&doc.interner).expect("m:naryPr is present");
    assert_eq!(
        properties
            .character(&doc.interner)
            .expect("m:chr is present")
            .to_wire(),
        "\u{2211}"
    );
    let lower = nary.lower_limit(&doc.interner).expect("m:sub is present");
    assert_eq!(text_of(&lower, &doc), "i=1");
    let upper = nary.upper_limit(&doc.interner).expect("m:sup is present");
    assert_eq!(text_of(&upper, &doc), "n");
    let operand = nary.operand(&doc.interner).expect("m:e is present");
    assert_eq!(text_of(&operand, &doc), "x");

    let denominator = fraction
        .denominator(&doc.interner)
        .expect("m:den is present");
    assert_eq!(text_of(&denominator, &doc), "2");

    // --- 2x2 matrix, each cell its own equation ---------------------------------------------------
    let MathElement::Matrix(matrix) = &elements[1] else {
        panic!("expected a matrix, got {:?}", elements[1]);
    };
    let rows = matrix.rows(&doc.interner);
    assert_eq!(rows.len(), 2, "two rows");
    let expected = [["a", "b"], ["c", "d"]];
    for (row, expected_row) in rows.iter().zip(expected) {
        let cells = row.cells(&doc.interner);
        assert_eq!(cells.len(), 2, "two cells per row");
        for (cell, expected_text) in cells.iter().zip(expected_row) {
            assert_eq!(text_of(cell, &doc), expected_text);
        }
    }

    // --- three-argument delimiter, non-default characters -----------------------------------------
    let MathElement::Delimiter(delimiter) = &elements[2] else {
        panic!("expected a delimiter, got {:?}", elements[2]);
    };
    let delimiter_properties = delimiter
        .properties(&doc.interner)
        .expect("m:dPr is present");
    assert_eq!(
        delimiter_properties
            .begin_character(&doc.interner)
            .expect("m:begChr is present")
            .to_wire(),
        "["
    );
    assert_eq!(
        delimiter_properties
            .end_character(&doc.interner)
            .expect("m:endChr is present")
            .to_wire(),
        "]"
    );
    let arguments = delimiter.arguments(&doc.interner);
    assert_eq!(arguments.len(), 3, "three delimited arguments");
    for (argument, expected_text) in arguments.iter().zip(["x", "y", "z"]) {
        assert_eq!(text_of(argument, &doc), expected_text);
    }
}

/// The concatenated text of every `m:r` an [`mjx_omml::Argument`] holds — the fixture only ever nests
/// a single run per argument, so this is exactly that run's own text.
fn text_of(argument: &mjx_omml::Argument, doc: &mjx_ooxml_core::RawDocument) -> String {
    argument
        .elements(&doc.interner)
        .iter()
        .filter_map(|element| match element {
            MathElement::Run(run) => Some(run.text(&doc.interner)),
            _ => None,
        })
        .collect()
}

/// A round trip that touches nothing must reproduce the fixture's own decompressed bytes exactly —
/// `ToXml::to_xml` rebuilds every element it looked at, so this is the only way to know that
/// rebuilding did not silently drop or reorder anything the fixture actually carried.
#[test]
fn the_deeply_nested_fixture_round_trips_canonicalisation_equal() {
    let doc = mjx_xml::fidelity::parse(FIXTURE.as_bytes()).expect("parse the fixture");
    let math = Math::from_xml(&doc.root, &doc.interner).expect("read m:oMath");
    // `Math`/`RawElement` name every node through interned `Symbol`s, not borrowed strings, so moving
    // `interner` out of `doc` here (leaving `doc.root` behind for the comparison below) borrows
    // nothing that move would invalidate.
    let mut write_interner = doc.interner;
    let rebuilt = mjx_ooxml_core::ToXml::to_xml(&math, &mut write_interner);
    // Structural equality (name/attributes/children/self-closing), the same comparison
    // `RawElement::replace_preserving_verbatim_source` uses to decide whether a span survives —
    // exactly the property "nothing was dropped or reordered" reduces to.
    assert_eq!(
        rebuilt, doc.root,
        "a no-op round trip must reproduce the original tree exactly"
    );
}

/// Editing one run **five levels deep** (`m:oMath` → `m:f` → `m:num` → `m:rad` → `m:e` → `m:nary` →
/// `m:sub` → `m:r` → `m:t`) must leave every untouched sibling subtree — the denominator, the whole
/// matrix, the whole delimiter — byte-identical (not merely equal: still carrying the *same, original*
/// [`mjx_ooxml_core::RawElement::source_span`], which a full reflow that happened to reproduce the
/// same bytes could not).
///
/// Confirmed by hand to fail if the span-preserving path breaks: replacing the targeted
/// `RawNode::Text` mutation below with a full `doc.root = doc.root.clone()` reassignment before the
/// mutation (which clones away every span in the tree, `RawElement::Clone`'s own documented behaviour)
/// turns every `assert_eq!` on `after_den`/`after_matrix`/`after_delimiter` red, each reporting
/// `None` where the span used to be — restored by removing the extra clone, not `git checkout --`.
#[test]
fn editing_a_run_five_levels_deep_leaves_every_sibling_subtree_untouched() {
    let mut doc = mjx_xml::fidelity::parse(FIXTURE.as_bytes()).expect("parse the fixture");

    let before_den = named_child_span(&doc.root, &doc.interner, &["f", "den"]);
    let before_matrix = top_level_span(&doc.root, &doc.interner, "m");
    let before_delimiter = top_level_span(&doc.root, &doc.interner, "d");
    let before_upper_limit = named_child_span(
        &doc.root,
        &doc.interner,
        &["f", "num", "rad", "e", "nary", "sup"],
    );
    assert!(
        before_den.is_some(),
        "a freshly parsed element always has a span"
    );
    assert!(before_matrix.is_some());
    assert!(before_delimiter.is_some());
    assert!(before_upper_limit.is_some());

    // Descend to `m:sub`'s own `m:r/m:t` and replace its text in place.
    let sub = descend_mut(
        &mut doc.root,
        &doc.interner,
        &["f", "num", "rad", "e", "nary", "sub"],
    )
    .expect("m:sub exists");
    let run = find_element_mut(&mut sub.children, &doc.interner, "r").expect("m:r exists");
    let text_element = find_element_mut(&mut run.children, &doc.interner, "t").expect("m:t exists");
    text_element.children = vec![RawNode::Text(b"i=0".to_vec().into_boxed_slice())];
    text_element.empty = false;

    // The edit actually landed: re-reading confirms the new text and nothing else about the n-ary.
    let math = Math::from_xml(&doc.root, &doc.interner).expect("read m:oMath after the edit");
    let MathElement::Fraction(fraction) = &math.elements(&doc.interner)[0] else {
        panic!("still a fraction");
    };
    let numerator = fraction.numerator(&doc.interner).unwrap();
    let MathElement::Radical(radical) = &numerator.elements(&doc.interner)[0] else {
        panic!("still a radical");
    };
    let radicand = radical.radicand(&doc.interner).unwrap();
    let MathElement::NaryOperator(nary) = &radicand.elements(&doc.interner)[0] else {
        panic!("still an n-ary operator");
    };
    assert_eq!(
        text_of(&nary.lower_limit(&doc.interner).unwrap(), &doc),
        "i=0"
    );
    assert_eq!(
        text_of(&nary.upper_limit(&doc.interner).unwrap(), &doc),
        "n",
        "the sibling m:sup is untouched"
    );

    let after_den = named_child_span(&doc.root, &doc.interner, &["f", "den"]);
    let after_matrix = top_level_span(&doc.root, &doc.interner, "m");
    let after_delimiter = top_level_span(&doc.root, &doc.interner, "d");
    let after_upper_limit = named_child_span(
        &doc.root,
        &doc.interner,
        &["f", "num", "rad", "e", "nary", "sup"],
    );

    assert_eq!(
        before_den, after_den,
        "the denominator's own span must survive an edit inside the numerator"
    );
    assert_eq!(
        before_matrix, after_matrix,
        "the whole matrix's own span must survive"
    );
    assert_eq!(
        before_delimiter, after_delimiter,
        "the whole delimiter's own span must survive"
    );
    assert_eq!(
        before_upper_limit, after_upper_limit,
        "the sibling m:sup's own span must survive an edit to m:sub"
    );
}

/// The `local`-named `m:`-namespaced child element of `element` closest to the root among
/// `element`'s own direct children — the read half of [`descend_mut`], used only to find the *first*
/// hop of a path from a top-level element.
fn top_level_span(
    root: &mjx_ooxml_core::RawElement,
    interner: &mjx_ooxml_core::Interner,
    local: &str,
) -> Option<std::ops::Range<u32>> {
    find_child(&root.children, interner, local).and_then(mjx_ooxml_core::RawElement::source_span)
}

/// The retained source span of the element found by following `path` (a sequence of `m:`-namespaced
/// local names) from `root`'s own children, or `None` if any hop is missing or has been reflowed.
fn named_child_span(
    root: &mjx_ooxml_core::RawElement,
    interner: &mjx_ooxml_core::Interner,
    path: &[&str],
) -> Option<std::ops::Range<u32>> {
    let mut current = find_child(&root.children, interner, path[0])?;
    for local in &path[1..] {
        current = find_child(&current.children, interner, local)?;
    }
    current.source_span()
}

fn find_child<'a>(
    children: &'a [RawNode],
    interner: &mjx_ooxml_core::Interner,
    local: &str,
) -> Option<&'a mjx_ooxml_core::RawElement> {
    children.iter().find_map(|node| match node {
        RawNode::Element(element) if interner.resolve(element.name.local) == local => Some(element),
        _ => None,
    })
}

fn find_element_mut<'a>(
    children: &'a mut [RawNode],
    interner: &mjx_ooxml_core::Interner,
    local: &str,
) -> Option<&'a mut mjx_ooxml_core::RawElement> {
    children.iter_mut().find_map(|node| match node {
        RawNode::Element(element) if interner.resolve(element.name.local) == local => Some(element),
        _ => None,
    })
}

/// Descends `path` from `root`, reaching each hop **mutably** — which, per
/// [`mjx_ooxml_core::RawElement`]'s own documented `DerefMut` behaviour, drops the retained source
/// span of every element on the path (and only those), leaving every sibling subtree's span alone.
fn descend_mut<'a>(
    root: &'a mut mjx_ooxml_core::RawElement,
    interner: &mjx_ooxml_core::Interner,
    path: &[&str],
) -> Option<&'a mut mjx_ooxml_core::RawElement> {
    let mut current = find_element_mut(&mut root.children, interner, path[0])?;
    for local in &path[1..] {
        current = find_element_mut(&mut current.children, interner, local)?;
    }
    Some(current)
}
