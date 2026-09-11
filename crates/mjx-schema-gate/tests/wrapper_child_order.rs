//! **What a VML drawing's child order is audited by, and what is left over** (MJXOFF-264).
//!
//! # The claim this file was written to check, and how it came out
//!
//! MJXOFF-245 made a `.vml` part validate child by child: `<xml>` is a wrapper in no namespace that
//! no VML schema declares a global element for, so each of its children — `v:shape`, `v:shapetype`,
//! `o:shapelayout`, `x:ClientData` — is written out as a standalone document and handed to
//! `xmllint` against a driver over `vml-main.xsd`. MJXOFF-264 was filed against that, and its
//! premise is:
//!
//! > per-child validation catches a wrong element and a wrong attribute but is **blind to sequence
//! > by construction**, because each child is handed to the validator on its own.
//!
//! **That premise is false, and this file is the measurement.** Handing a child to `xmllint` on its
//! own does not remove its own content model; it applies it. `CT_Shapetype` is
//! `EG_ShapeElements*, o:complex?` — a sequence — and a `v:shapetype` that writes `o:complex` before
//! its shape elements **fails validation today**, with the message
//! [`the_gate_that_validates_a_wrapper_child_applies_that_child_s_own_sequence`] asserts. So the
//! order of everything *inside* a wrapper's children has been audited since MJXOFF-245, by the half
//! of the gate the ticket believed could not see it.
//!
//! What per-child validation genuinely cannot reach is the order of the **wrapper's own children**,
//! and there the answer is not "an audit is missing" but "there is nothing to audit":
//! [`no_schema_in_either_pinned_tree_declares_the_vml_wrapper_element`] derives it. `<xml>` is a
//! Microsoft convention, declared by no schema in either pinned tree, so it has no content model and
//! no sequence for a child-order table to enforce. Adding the five `vml-*` schemas to `xtask`'s
//! `CHILD_ORDER_SCHEMAS` — MJXOFF-264's first candidate — would generate complex types for the VML
//! family and change nothing here, because `order.rs`'s walk is keyed on a **root element the tables
//! name** and `<xml>` is in none of them; the walk would have to start at each child, which is
//! exactly where the validator already starts.
//!
//! # Why this is two derivations rather than a sentence
//!
//! MJXOFF-264's second candidate was to say permanently that VML child order is not audited and hold
//! that with `OrderingCoverage::NotGenerated`. Taking it would have written down something false:
//! most of that order **is** audited. Both halves are checked here instead — the sequence-sensitivity
//! against real markup and a real `xmllint`, the wrapper's lack of a declaration against the two
//! schema trees themselves — so neither can quietly stop being true.
//!
//! Skips without `References/`; `MJX_REQUIRE_SCHEMA=1` turns the absence into a failure, which is
//! what CI sets.

use std::collections::BTreeSet;

use mjx_schema_gate::{harness, SchemaRef, SchemaSet, WorkDir, WRAPPER_ROOTS};

/// The wrapper root this file is about, taken from the gate's own table rather than spelled again.
fn vml_wrapper() -> &'static mjx_schema_gate::WrapperRoot {
    WRAPPER_ROOTS
        .iter()
        .find(|wrapper| wrapper.root_local_name == "xml")
        .expect("the VML wrapper is the category-1b entry MJXOFF-245 added")
}

/// A `v:shapetype` whose children are written in the order `CT_Shapetype` declares.
///
/// Not invented markup: this is the shape of the `v:shapetype` every producer writes for a legacy
/// text box, reduced to the three children that matter for the sequence — the shape elements, then
/// `o:complex`, which the type puts last.
const IN_ORDER: &str = concat!(
    r#"<v:shapetype xmlns:v="urn:schemas-microsoft-com:vml" "#,
    r#"xmlns:o="urn:schemas-microsoft-com:office:office" id="t202" coordsize="21600,21600" "#,
    r#"o:spt="202" path="m,l,21600r21600,l21600,xe">"#,
    r#"<v:stroke joinstyle="miter"/>"#,
    r#"<v:path gradientshapeok="t" o:connecttype="rect"/>"#,
    r#"<o:complex v:ext="edit"/>"#,
    r#"</v:shapetype>"#,
);

/// The same three children, with the one the sequence puts last written first.
const OUT_OF_ORDER: &str = concat!(
    r#"<v:shapetype xmlns:v="urn:schemas-microsoft-com:vml" "#,
    r#"xmlns:o="urn:schemas-microsoft-com:office:office" id="t202" coordsize="21600,21600" "#,
    r#"o:spt="202" path="m,l,21600r21600,l21600,xe">"#,
    r#"<o:complex v:ext="edit"/>"#,
    r#"<v:stroke joinstyle="miter"/>"#,
    r#"<v:path gradientshapeok="t" o:connecttype="rect"/>"#,
    r#"</v:shapetype>"#,
);

/// **Validating a wrapper's child on its own applies that child's own sequence.**
///
/// The permanent form of the experiment that answered MJXOFF-264. Two documents differing only in
/// the order of three children: the first validates, the second does not, against the same driver
/// over the same schema that `inspect::validate_each_child` uses for every `.vml` part in the
/// corpus. A change that made the gate order-blind — a laxer schema, a driver that stopped
/// importing `vml-main.xsd`, a validator invoked without `--schema` — turns this red.
#[test]
fn the_gate_that_validates_a_wrapper_child_applies_that_child_s_own_sequence() {
    let Some(harness) = harness() else {
        return;
    };
    let wrapper = vml_wrapper();
    let work = WorkDir::new("vml_wrapper_child_order");

    let good = work.path().join("in_order.xml");
    std::fs::write(&good, IN_ORDER).expect("write the in-order child");
    let report = harness.validate(wrapper.schema, wrapper.namespace, &good);
    assert!(
        report.is_none(),
        "the in-order `v:shapetype` must validate, or the case below proves nothing about \
         *order*:\n{}",
        report.unwrap_or_default()
    );

    let bad = work.path().join("out_of_order.xml");
    std::fs::write(&bad, OUT_OF_ORDER).expect("write the out-of-order child");
    let report = harness
        .validate(wrapper.schema, wrapper.namespace, &bad)
        .unwrap_or_else(|| {
            panic!(
                "a `v:shapetype` with `o:complex` before its shape elements validated. \
                 `CT_Shapetype` is `EG_ShapeElements*, o:complex?`, so this markup is out of \
                 sequence and the gate that reads every `.vml` part child by child has stopped \
                 seeing sequence — which is what MJXOFF-264 believed it never saw."
            )
        });
    assert!(
        report.contains("not expected") || report.contains("Missing child element"),
        "the failure must be the sequence rather than something else about this markup; \
         `xmllint` said:\n{report}"
    );

    println!(
        "the VML wrapper's children are validated against {}, sequence included",
        wrapper.schema.file
    );
}

/// **The wrapper element itself is declared by no schema, so it has no order to audit.**
///
/// This is the half per-child validation really cannot reach, and the reason it cannot is not a gap
/// in the gate. `<xml>` is a Microsoft convention rather than ECMA markup: no schema in either
/// pinned tree declares a global element of that name in no namespace, so it has no complex type, no
/// particle and no sequence. A child-order table generated for the VML family would have nothing to
/// key on — `order.rs` walks from a root element the tables name — which is why MJXOFF-264's first
/// candidate would have changed nothing.
///
/// Derived from the schema trees rather than asserted, so the day a schema declares it, this says
/// so.
#[test]
fn no_schema_in_either_pinned_tree_declares_the_vml_wrapper_element() {
    let Some(harness) = harness() else {
        return;
    };
    let wrapper = vml_wrapper();
    let needle = format!("<xsd:element name=\"{}\"", wrapper.root_local_name);
    let alternative = format!("<xs:element name=\"{}\"", wrapper.root_local_name);

    let mut scanned = 0usize;
    let mut declaring: Vec<String> = Vec::new();
    for directory in harness.schema_directories() {
        let entries = std::fs::read_dir(directory)
            .unwrap_or_else(|e| panic!("reading {}: {e}", directory.display()));
        for entry in entries {
            let path = entry.expect("a schema directory entry").path();
            if path.extension().is_none_or(|value| value != "xsd") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            scanned += 1;
            // A *global* declaration only: an `xsd:element` at the top level of the schema. A
            // nested one names a child of some other type and says nothing about a document root.
            for line in text.lines() {
                let trimmed = line.trim_start();
                let indent = line.len() - trimmed.len();
                if indent <= 2
                    && (trimmed.starts_with(&needle) || trimmed.starts_with(&alternative))
                {
                    declaring.push(format!("{}: {trimmed}", path.display()));
                }
            }
        }
    }

    // A floor phrased as *the walk is still matching*, never as a total: the two pinned trees hold
    // many schemas and a walk that found none would make the assertion below vacuous.
    assert!(
        scanned > 20,
        "only {scanned} schema(s) were read out of {:?} — the walk has stopped matching",
        harness
            .schema_directories()
            .map(|directory| directory.display().to_string())
    );
    // And it must be finding declarations at all, or the indent rule above is what stopped matching
    // rather than the element being absent.
    let sample = SchemaRef {
        set: SchemaSet::Markup,
        file: "vml-main.xsd",
    };
    let vml = std::fs::read_to_string(harness.schema_path(sample)).expect("vml-main.xsd is there");
    let globals: BTreeSet<&str> = vml
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let indent = line.len() - trimmed.len();
            (indent <= 2 && trimmed.starts_with("<xsd:element name=\"")).then_some(trimmed)
        })
        .collect();
    assert!(
        globals.len() > 5,
        "only {} global element declaration(s) were recognised in vml-main.xsd — the same scan \
         that reports `<xml>` absent has stopped matching, so its answer means nothing",
        globals.len()
    );

    assert!(
        declaring.is_empty(),
        "a schema now declares a global `<{}>` element:\n  {}\n\nThe VML wrapper having no \
         declaration is what makes \"its child order is audited by nothing\" a statement about the \
         format rather than a gap in this gate (MJXOFF-264). If that has changed, the wrapper has a \
         content model and the child-order half has something to enforce.",
        wrapper.root_local_name,
        declaring.join("\n  ")
    );

    println!(
        "{scanned} schema(s) read, {} global element(s) recognised in vml-main.xsd, and no global \
         `<{}>` anywhere: the wrapper has no content model",
        globals.len(),
        wrapper.root_local_name
    );
}
