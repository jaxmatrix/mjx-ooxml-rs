//! ZIP entries that carry no content type, and the line between the two kinds of them.
//!
//! A container may hold entries that are not parts. `zip -r` **without** `-D` writes a directory
//! entry for every folder it walks (`ppt/`, `_rels/`, `ppt/slides/`), which is a legal ZIP that real
//! producers emit, and OPC has no content type for a directory because a directory is not a part.
//! A *file* with neither an `<Override>` nor a `<Default>` covering its extension is the opposite
//! case: it is addressable, nothing types it, and ECMA-376 Part 2 §10.1.2 says every part has
//! exactly one content type — so it is a genuine package defect.
//!
//! Before MJXOFF-284 the gate collapsed both into one `panic!`, which is how
//! `validation-artefacts --ingest` — a command a person points at an arbitrary file they just saved
//! out of Office — aborted with a bare stack trace instead of the report
//! `docs/validation/06-the-office-pass.md` promises. This suite holds both halves at once: the
//! directory entries are skipped **by name**, the orphan file is still failed, and the two verdicts
//! are told apart in the same package.
//!
//! ## Where the discriminator comes from
//!
//! Not from this crate. `mjx_opc::PartName::new` refuses a name ending in `/` — "part name must not
//! end with '/'" — and `Package::part_names`, `Package::validate`'s content-type check and
//! `authored_xml_parts` all skip an entry whose name is not a valid part name for that reason.
//! [`mjx_opc_already_takes_this_position`] pins that, so the gate is following the packaging layer
//! rather than inventing a second rule that could drift from it.

use mjx_opc::{Package, PartName};
use mjx_schema_gate::{assert_rows_are_valid, harness, inspect_deck, PartOutcome};

/// The fixture every case here is rebuilt from: a small `.pptx` the gate already validates clean.
const FIXTURE: &str = "sample.pptx";

/// An extension no `<Default>` in the fixture declares and no `<Override>` names, so an entry
/// carrying it is a part with no content type — the defect half of the discrimination.
const ORPHAN_ENTRY: &str = "ppt/orphan.mjx";

// ---------------------------------------------------------------------------------------------
// Rebuilding a package the way `zip -r` does
// ---------------------------------------------------------------------------------------------

/// CRC-32 (IEEE, reflected), computed bitwise.
///
/// Written out rather than pulled in: `zip` is a dependency of `mjx-opc` and of nothing else in this
/// workspace, and a test that needs eight lines of checksum is not a reason to widen that.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

/// One entry to write: its name, its payload, and whether it is a directory entry.
struct Entry {
    name: String,
    payload: Vec<u8>,
    directory: bool,
}

/// Writes a ZIP holding `entries` verbatim, every one **stored** (method 0).
///
/// Stored rather than deflated because nothing here reads the compression back: what is under test
/// is the entry *set*, and a directory entry is an empty stored entry whose name ends in `/` with
/// the MS-DOS directory attribute set — exactly what Info-ZIP writes without `-D`.
fn write_zip(entries: &[Entry]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();

    for entry in entries {
        let offset = u32::try_from(out.len()).expect("a fixture-sized archive");
        let name = entry.name.as_bytes();
        let name_len = u16::try_from(name.len()).expect("a short entry name");
        let size = u32::try_from(entry.payload.len()).expect("a fixture-sized part");
        let crc = crc32(&entry.payload);
        // MS-DOS attribute bit 4 marks a directory, which is what `zip -r` sets on a folder entry
        // alongside the trailing slash in its name.
        let external_attributes = if entry.directory { 0x10u32 } else { 0 };

        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes()); // local file header
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&0u16.to_le_bytes()); // method: stored
        out.extend_from_slice(&0u16.to_le_bytes()); // mod time
        out.extend_from_slice(&0u16.to_le_bytes()); // mod date
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes()); // compressed size
        out.extend_from_slice(&size.to_le_bytes()); // uncompressed size
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra length
        out.extend_from_slice(name);
        out.extend_from_slice(&entry.payload);

        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes()); // central directory header
        central.extend_from_slice(&20u16.to_le_bytes()); // version made by
        central.extend_from_slice(&20u16.to_le_bytes()); // version needed
        central.extend_from_slice(&0u16.to_le_bytes()); // flags
        central.extend_from_slice(&0u16.to_le_bytes()); // method: stored
        central.extend_from_slice(&0u16.to_le_bytes()); // mod time
        central.extend_from_slice(&0u16.to_le_bytes()); // mod date
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&name_len.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra length
        central.extend_from_slice(&0u16.to_le_bytes()); // comment length
        central.extend_from_slice(&0u16.to_le_bytes()); // disk number start
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attributes
        central.extend_from_slice(&external_attributes.to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name);
    }

    let central_offset = u32::try_from(out.len()).expect("a fixture-sized archive");
    let central_size = u32::try_from(central.len()).expect("a fixture-sized directory");
    let count = u16::try_from(entries.len()).expect("a fixture-sized entry count");
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes()); // end of central directory
    out.extend_from_slice(&0u16.to_le_bytes()); // this disk
    out.extend_from_slice(&0u16.to_le_bytes()); // disk with the central directory
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&central_size.to_le_bytes());
    out.extend_from_slice(&central_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment length
    out
}

/// Every directory prefix implied by `names`, in the order a walk would first reach it.
///
/// Derived from the entry names rather than listed, so this is the set `zip -r` would have written
/// for whatever the fixture actually holds — not a set that has to be kept in step with it.
fn directory_prefixes(names: &[String]) -> Vec<String> {
    let mut prefixes = Vec::new();
    for name in names {
        let mut at = 0usize;
        while let Some(slash) = name[at..].find('/') {
            at += slash + 1;
            let prefix = name[..at].to_owned();
            if !prefixes.contains(&prefix) {
                prefixes.push(prefix);
            }
        }
    }
    prefixes
}

/// The fixture's parts, rebuilt into a fresh container with `directories` and `extra` mixed in.
///
/// The parts themselves are read back through `mjx_opc::Package`, so the payloads are byte-for-byte
/// the fixture's and nothing here depends on how it was compressed. An `extra` entry whose name
/// matches one of them **replaces** its payload in place rather than being appended, which is how a
/// case corrupts one existing part without disturbing the entry order around it.
fn rebuilt(fixture: &str, directories: bool, extra: &[(&str, &[u8])]) -> Vec<u8> {
    let bytes = mjx_fixtures::fixture(fixture);
    let package = Package::open(&bytes).expect("the committed fixture opens");
    let mut parts: Vec<(String, Vec<u8>)> = package
        .entries()
        .iter()
        .map(|entry| {
            (
                entry.name.clone(),
                entry
                    .bytes()
                    .expect("a freshly opened package materializes every entry")
                    .to_vec(),
            )
        })
        .collect();
    let extra: Vec<&(&str, &[u8])> = extra
        .iter()
        .filter(
            |(name, payload)| match parts.iter_mut().find(|(existing, _)| existing == name) {
                Some((_, existing)) => {
                    *existing = (*payload).to_vec();
                    false
                }
                None => true,
            },
        )
        .collect();

    let mut entries = Vec::new();
    if directories {
        let names: Vec<String> = parts
            .iter()
            .map(|(name, _)| name.clone())
            .chain(extra.iter().map(|(name, _)| (*name).to_owned()))
            .collect();
        for prefix in directory_prefixes(&names) {
            entries.push(Entry {
                name: prefix,
                payload: Vec::new(),
                directory: true,
            });
        }
    }
    for (name, payload) in parts {
        entries.push(Entry {
            name,
            payload,
            directory: false,
        });
    }
    for (name, payload) in extra {
        entries.push(Entry {
            name: (*name).to_owned(),
            payload: (*payload).to_vec(),
            directory: false,
        });
    }
    write_zip(&entries)
}

/// The rows for a rebuilt package, or `None` when the gate has no schemas to work with.
fn rows_for(directories: bool, extra: &[(&str, &[u8])]) -> Option<Vec<mjx_schema_gate::PartRow>> {
    rows_of(FIXTURE, directories, extra)
}

/// [`rows_for`], over a named fixture.
fn rows_of(
    fixture: &str,
    directories: bool,
    extra: &[(&str, &[u8])],
) -> Option<Vec<mjx_schema_gate::PartRow>> {
    let harness = harness()?;
    let bytes = rebuilt(fixture, directories, extra);
    Some(inspect_deck(&harness, fixture, &bytes, &[]))
}

// ---------------------------------------------------------------------------------------------
// The position this gate follows
// ---------------------------------------------------------------------------------------------

/// The packaging layer's answer, pinned — needs no schemas, so it runs in every job.
///
/// `mjx-opc` opens a package carrying directory entries without complaint and does not treat one as
/// a part: `PartName` refuses a trailing slash, and `part_names` filters on exactly that. The gate's
/// rule is this rule, which is why the two cannot drift.
#[test]
fn mjx_opc_already_takes_this_position() {
    assert!(
        PartName::from_zip_name("ppt/").is_err(),
        "a trailing slash is not a part name; that is the discriminator the gate reads"
    );
    assert!(
        PartName::from_zip_name("ppt/orphan.mjx").is_ok(),
        "an orphan file *is* a well-formed part name — nothing about its name excuses it"
    );

    let bytes = rebuilt(FIXTURE, true, &[]);
    let package = Package::open(&bytes).expect("a package with directory entries still opens");
    let directories: Vec<&str> = package
        .entries()
        .iter()
        .map(|entry| entry.name.as_str())
        .filter(|name| name.ends_with('/'))
        .collect();
    assert!(
        !directories.is_empty(),
        "the rebuild must actually carry directory entries, or this suite proves nothing"
    );
    assert!(
        package
            .part_names()
            .all(|part| !part.as_str().ends_with('/')),
        "mjx-opc does not report a directory entry as a part"
    );
    assert!(
        package.validate().is_ok(),
        "mjx-opc raises no defect for a package that carries directory entries"
    );
    println!("directory entries mjx-opc skips: {directories:?}");
}

// ---------------------------------------------------------------------------------------------
// The two halves
// ---------------------------------------------------------------------------------------------

/// Half one: a directory entry is reported as one, never panicked on, and never failed.
#[test]
fn a_directory_entry_is_skipped_by_name_and_not_a_failure() {
    let Some(rows) = rows_for(true, &[]) else {
        return;
    };
    let skipped: Vec<&str> = rows
        .iter()
        .filter(|row| matches!(row.outcome, PartOutcome::SkippedDirectoryEntry))
        .map(|row| row.name.as_str())
        .collect();
    assert!(
        skipped.contains(&"/_rels/") && skipped.contains(&"/ppt/"),
        "every directory entry must have its own row; the skipped ones were {skipped:?}"
    );
    assert!(
        !rows.iter().any(|row| row.outcome.is_failure()),
        "a legal container is not a defect: {:?}",
        rows.iter()
            .filter(|row| row.outcome.is_failure())
            .map(|row| format!("{}: {}", row.name, row.outcome.describe()))
            .collect::<Vec<_>>()
    );
    // The whole gate still passes over the rebuilt package, which is the claim that matters: the
    // parts are validated exactly as before and only the directory rows are new.
    assert_rows_are_valid(FIXTURE, &rows);
}

/// Half two: a **file** that no content type covers is still a defect, and still fails the gate.
///
/// This is the half a fix that skipped every untyped entry would have quietly lost.
#[test]
fn a_file_with_no_content_type_is_still_reported_as_a_defect() {
    let Some(rows) = rows_for(false, &[(ORPHAN_ENTRY, b"not typed by anything")]) else {
        return;
    };
    let orphan = rows
        .iter()
        .find(|row| row.name == format!("/{ORPHAN_ENTRY}"))
        .expect("the orphan entry has a row of its own");
    assert!(
        matches!(orphan.outcome, PartOutcome::WithoutContentType { .. }),
        "the orphan reported: {}",
        orphan.outcome.describe()
    );
    assert!(
        orphan.outcome.is_failure(),
        "a part with no content type fails the gate; it reported: {}",
        orphan.outcome.describe()
    );

    let message = std::panic::catch_unwind(|| assert_rows_are_valid(FIXTURE, &rows))
        .err()
        .and_then(|payload| payload.downcast_ref::<String>().cloned())
        .expect("the suite assertion must fail, naming the orphan");
    assert!(
        message.contains(ORPHAN_ENTRY),
        "the failure must name the untyped part; it said:\n{message}"
    );
    println!("the orphan, refused:\n{message}");
}

/// The discrimination itself: **one** package carrying both, and exactly one failure in it.
///
/// Split from the two halves above deliberately. Each of those proves one verdict in isolation, and
/// a fix that got either one right on its own would still pass one of them; only this case says the
/// gate tells them apart when they arrive together, which is what MJXOFF-284 is about.
#[test]
fn the_two_are_told_apart_in_the_same_package() {
    let Some(rows) = rows_for(true, &[(ORPHAN_ENTRY, b"not typed by anything")]) else {
        return;
    };
    let failures: Vec<&str> = rows
        .iter()
        .filter(|row| row.outcome.is_failure())
        .map(|row| row.name.as_str())
        .collect();
    assert_eq!(
        failures,
        vec![format!("/{ORPHAN_ENTRY}").as_str()],
        "exactly the orphan fails; the directory entries are skips"
    );
    assert!(
        rows.iter()
            .filter(|row| matches!(row.outcome, PartOutcome::SkippedDirectoryEntry))
            .all(|row| row.name.ends_with('/')),
        "nothing but a directory entry may be skipped as one"
    );
    assert!(
        rows.iter()
            .filter(|row| matches!(row.outcome, PartOutcome::WithoutContentType { .. }))
            .all(|row| !row.name.ends_with('/')),
        "no directory entry may be reported as an untyped part"
    );
    println!("{}", mjx_schema_gate::outcome_table(FIXTURE, &rows));
}

// ---------------------------------------------------------------------------------------------
// The other two panics on the same path
// ---------------------------------------------------------------------------------------------

/// A part whose content type declares XML and whose bytes are not XML is a row, not a stack trace.
///
/// `sample.pptx` declares `<Default Extension="xml" ContentType="application/xml"/>`, so an entry
/// named `.xml` is typed as XML by the file's own content-types stream without any override — which
/// is exactly the shape a producer's corrupt part arrives in.
#[test]
fn a_part_declared_xml_whose_bytes_are_not_is_reported_rather_than_raised() {
    let Some(rows) = rows_for(false, &[("ppt/broken.xml", b"<not xml at <all")]) else {
        return;
    };
    let broken = rows
        .iter()
        .find(|row| row.name == "/ppt/broken.xml")
        .expect("the malformed part has a row of its own");
    assert!(
        matches!(broken.outcome, PartOutcome::NotWellFormedXml(_)),
        "it reported: {}",
        broken.outcome.describe()
    );
    assert!(
        broken.outcome.is_failure(),
        "markup that is not XML fails the gate; it reported: {}",
        broken.outcome.describe()
    );
    println!("{}", broken.outcome.describe());
}

/// An **embedded** package whose bytes are not a container is a row too.
///
/// The outer package cannot reach this through `--ingest`, which opens the file before it reports on
/// it — but a chart's workbook is opened here for the first time, and a file this library did not
/// write may carry a broken one. `crate::audit_order_report` already reports that rather than
/// raising it; this is the other half of the gate agreeing with it.
#[test]
fn an_embedded_package_that_will_not_open_is_reported_rather_than_raised() {
    const EMBEDDED: &str = "ppt/embeddings/Microsoft_Excel_Sheet1.xlsx";
    let Some(rows) = rows_of(
        "charts.pptx",
        false,
        &[(EMBEDDED, b"PK\x03\x04 not a container")],
    ) else {
        return;
    };
    let refused = rows
        .iter()
        .find(|row| matches!(row.outcome, PartOutcome::PackageWouldNotOpen(_)))
        .expect("the embedded workbook is refused with a row of its own");
    assert!(
        refused.name.contains(EMBEDDED),
        "the row must name the embedded package; it named {}",
        refused.name
    );
    assert!(
        refused.outcome.is_failure(),
        "a package that will not open fails the gate"
    );
    println!("{}: {}", refused.name, refused.outcome.describe());
}
