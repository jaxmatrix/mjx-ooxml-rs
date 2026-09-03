//! Tier-1 byte-identity proof for `sample.docx` (MJXOFF-90's "Done when"): `Document::open` then
//! `save()` reproduces every one of the package's parts with byte-identical decompressed payloads —
//! checked **part by part**, not by a whole-container hash — with the typed model **actually having
//! run**.
//!
//! # Why "actually having run" needs its own test
//!
//! `mjx-opc` already round-trips `sample.docx` byte-identically today, before a line of this crate
//! runs: part-level copy-on-write re-emits every stored part verbatim when nothing dirties it. A
//! bare `Document::open(bytes).save()` assertion would pass on the strength of that machinery alone
//! and prove nothing about `mjx-docx`'s own code — the same trap `crates/mjx-opc/tests/roundtrip.rs`
//! exists to close for the container layer, restated here for the model layer.
//!
//! So this suite calls [`Document::set_conformance`] with the value already there
//! (`sample.docx`'s `w:document` carries no `@conformance` at all, so this is `None` on both sides):
//! that forces `word/document.xml` from `Stored` to `Edited` and sends it through
//! [`MainDocument::from_xml`]/[`MainDocument::write_back`] — the real decode/typed-model/encode path
//! — before `save()` ever runs. The eleven namespace declarations and `mc:Ignorable="w14 wp14 w15"`
//! `sample.docx`'s root carries are exactly what a broken preservation path would corrupt first.

use mjx_docx::Document;
use mjx_fixtures::fixture;
use mjx_opc::Package;

#[test]
fn open_then_save_reproduces_every_part_with_the_model_materialized() {
    let original = fixture("sample.docx");

    let mut document = Document::open(&original).expect("open sample.docx");
    assert_eq!(
        document.conformance().expect("read @conformance"),
        None,
        "sample.docx declares no @conformance — the fixture assumption this test rests on"
    );
    // The forcing step: dirties word/document.xml, decoding it into `MainDocument` and writing it
    // straight back. See the module docs for why this, and not a bare open/save, is the proof.
    document
        .set_conformance(None)
        .expect("materialize word/document.xml through the typed model");

    let saved = document.save().expect("save");

    let original_pkg = Package::open(&original).expect("open the original for comparison");
    let saved_pkg = Package::open(&saved).expect("open the saved bytes for comparison");

    let original_names: Vec<&str> = original_pkg
        .entries()
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    let saved_names: Vec<&str> = saved_pkg
        .entries()
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(
        original_names, saved_names,
        "the container's part set/order changed"
    );
    // The ten parts the ticket counts: [Content_Types].xml, _rels/.rels, docProps/{core,app}.xml,
    // word/_rels/document.xml.rels, word/{document,styles,fontTable,settings}.xml,
    // word/theme/theme1.xml.
    assert_eq!(original_names.len(), 10, "sample.docx's own part count");

    // Per-part decompressed-payload byte identity — checked one part at a time, so a defect in any
    // single part is named rather than folded into a whole-container hash.
    for (before, after) in original_pkg.entries().iter().zip(saved_pkg.entries()) {
        assert_eq!(
            before.bytes(),
            after.bytes(),
            "decompressed bytes changed for {}",
            before.name
        );
    }
}
