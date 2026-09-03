//! Regression tests for defects the fuzz campaign found in the OPC opener (MJXOFF-146).
//!
//! Run the campaign that produced them with `cargo run -p xtask -- fuzz --target opc-container`.

use mjx_opc::Package;

/// **The invariant: what a container *declares* never sizes an allocation on its own.**
///
/// A ZIP entry's uncompressed size lives in its header, is attacker-controlled, and is checked
/// against the data only after the data has arrived. `Package::open` used to reserve exactly that
/// many bytes, so `tests/fixtures/declared_size_lie.zip` — 757 bytes, declaring 4 GiB for a
/// four-byte part — made the opener ask the allocator for four gigabytes. The campaign's counting
/// allocator caught it on the first hostile container it tried, before any mutation.
///
/// The assertion is the property, not the error: whatever `open` decides about this container, it
/// decides it without believing the header. The reservation is what changed, so the reservation is
/// what is checked — a test asserting only "this returns `Err`" would pass against the defect,
/// because it returned `Err` then too, four gigabytes later.
#[test]
fn a_declared_size_never_sizes_an_allocation_by_itself() {
    let container = mjx_fixtures::fixture("declared_size_lie.zip");
    assert_eq!(
        container.len(),
        757,
        "the fixture is the minimised container the campaign found; if it grew, it is not that one"
    );

    let package = Package::open(&container).expect("757 bytes must open, not exhaust memory");
    let part = package
        .entries()
        .iter()
        .find(|entry| entry.name == "a.xml")
        .expect("the lying entry is in the container");
    assert_eq!(
        part.bytes(),
        Some(&b"<a/>"[..]),
        "the part must hold the bytes that were actually stored, not the size claimed"
    );

    // The discriminating assertion. Every entry's buffer is reserved before its bytes arrive, and
    // the *capacity* is the only place the difference shows: without the clamp this is 4,294,967,294
    // and the test still passes on a machine that overcommits, which is exactly how this defect
    // would survive a "does it return `Err`" test.
    for entry in package.entries() {
        let mjx_opc::PartBody::Raw(buffer) = &entry.body else {
            panic!("a freshly opened part is raw bytes");
        };
        assert!(
            buffer.capacity() <= 2 * 1024 * 1024,
            "{} reserved {} bytes for a {}-byte part — the header was believed",
            entry.name,
            buffer.capacity(),
            buffer.len()
        );
    }
}

/// Every committed package fixture still opens, and the parts it yields are far inside the reader's
/// nesting limit.
///
/// The depth limit added alongside these fixes (`mjx_xml::fidelity::MAXIMUM_DEPTH`) is a bound on
/// what can be opened at all, so it needs a standing check that real markup is nowhere near it —
/// otherwise the constant is one Phase C fixture away from refusing a legitimate document, and
/// nothing would say so until it did.
#[test]
fn every_committed_fixture_part_is_far_inside_the_nesting_limit() {
    let limit = mjx_xml::fidelity::MAXIMUM_DEPTH;
    let mut deepest = (0usize, String::new());
    let mut parts = 0usize;

    for name in mjx_fixtures::package_fixtures() {
        let package = Package::open(&mjx_fixtures::fixture(&name))
            .unwrap_or_else(|e| panic!("{name} must still open: {e}"));
        for entry in package.entries() {
            if !entry.name.ends_with(".xml") && !entry.name.ends_with(".rels") {
                continue;
            }
            let Some(bytes) = entry.bytes() else { continue };
            let document = mjx_xml::fidelity::parse(bytes)
                .unwrap_or_else(|e| panic!("{name} :: {} must still parse: {e}", entry.name));
            parts += 1;
            let depth = depth_of(&document);
            if depth > deepest.0 {
                deepest = (depth, format!("{name} :: {}", entry.name));
            }
        }
    }

    assert!(
        parts > 100,
        "only {parts} parts were read — the corpus is not being swept"
    );
    assert!(
        deepest.0 * 8 < limit,
        "the deepest committed part is now {} ({}), which is no longer comfortably inside the \
         reader's limit of {limit}; redo the measurement in MAXIMUM_DEPTH's documentation before \
         raising it",
        deepest.0,
        deepest.1
    );
}

/// Measured iteratively: a recursive walk would overflow on the inputs this file exists for.
fn depth_of(document: &mjx_ooxml_core::RawDocument) -> usize {
    let mut deepest = 0usize;
    let mut stack = vec![(&document.root, 1usize)];
    while let Some((element, depth)) = stack.pop() {
        deepest = deepest.max(depth);
        for child in element.children.iter() {
            if let mjx_ooxml_core::RawNode::Element(child) = child {
                stack.push((child, depth + 1));
            }
        }
    }
    deepest
}
