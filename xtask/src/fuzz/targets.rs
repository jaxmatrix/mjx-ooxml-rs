//! The targets: one entry point each, with the property that entry point must hold.
//!
//! # Adding a target
//!
//! A target is a name, a seed corpus and a function from bytes to an [`Outcome`]. Phase C and D add
//! `wml` and `sml` parsers to the same three entry points these sit on, and putting one under the
//! campaign is one `Target` literal plus one `fn`:
//!
//! ```ignore
//! // Sketched against a `mjx_docx` that does not exist yet; `xml_seeds` below is the real helper a
//! // `wml` target would reuse, since a Word body part is XML like any other.
//! Target {
//!     name: "wml-body",
//!     entry_point: "mjx_docx::parse_body",
//!     seeds: xml_seeds,
//!     run: |input| {
//!         let mut outcome = Outcome::default();
//!         match mjx_docx::parse_body(input) {
//!             Ok(body) => outcome.note("paragraphs", bucket(body.paragraphs.len())),
//!             Err(error) => outcome.note("error", docx_error_label(&error)),
//!         }
//!         outcome
//!     },
//! }
//! ```
//!
//! Everything else — mutation, corpus growth, the memory ceiling, the watchdog, the crash log — is
//! the driver's and is shared.
//!
//! # What a target asserts
//!
//! Not "this input returns `Err`". Returning a typed error for garbage is the *correct* outcome, and
//! a test that pins the current error pins behaviour rather than the property. A target reports a
//! finding only for a violated invariant: a panic (caught by the driver), an unbounded allocation
//! (measured by the driver), or an oracle failure — which is what makes these more than
//! does-not-crash runs.

use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use mjx_mce::UnderstoodNamespaces;
use mjx_ooxml_core::{RawElement, RawNode};
use mjx_opc::Package;
use mjx_xml::fidelity;

use crate::fuzz::container::{self, Entry};

/// One violated invariant, with enough detail to write the regression test from.
#[derive(Debug, Clone)]
pub struct Finding {
    /// A short, stable label — the campaign groups findings by it.
    pub kind: &'static str,
    /// What went wrong, in words.
    pub detail: String,
}

/// What one execution produced.
#[derive(Debug, Default)]
pub struct Outcome {
    hasher: FeatureHasher,
    /// Invariants this execution violated. Empty is the expected case.
    pub findings: Vec<Finding>,
}

impl Outcome {
    /// Folds one observed feature into this execution's signature.
    ///
    /// The signature is what drives corpus growth: an input whose signature the campaign has not
    /// seen is *behaviourally new* and is kept, which is how a black-box campaign grows a corpus
    /// without compiler instrumentation. It is deliberately coarse — buckets, not counts — because a
    /// signature that changes with every byte would keep every input and grow no corpus at all.
    pub fn note(&mut self, label: &str, value: impl Hash) {
        label.hash(&mut self.hasher);
        value.hash(&mut self.hasher);
    }

    /// Records a violated invariant.
    pub fn fault(&mut self, kind: &'static str, detail: impl Into<String>) {
        self.findings.push(Finding {
            kind,
            detail: detail.into(),
        });
    }

    /// This execution's behavioural signature.
    #[must_use]
    pub fn signature(&self) -> u64 {
        self.hasher.finish()
    }
}

/// FNV-1a — a hasher with no dependency and no per-run randomisation, so a signature means the same
/// thing across processes and a campaign's corpus growth is reproducible.
#[derive(Debug)]
struct FeatureHasher(u64);

impl Default for FeatureHasher {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for FeatureHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100_0000_01b3);
        }
    }
}

/// A campaign target.
pub struct Target {
    /// The name the operator selects with `--target`.
    pub name: &'static str,
    /// What the entry point is, in one line, for the report.
    pub entry_point: &'static str,
    /// The seed corpus.
    pub seeds: fn() -> Vec<Vec<u8>>,
    /// One execution.
    pub run: fn(&[u8]) -> Outcome,
}

impl std::fmt::Debug for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Target").field("name", &self.name).finish()
    }
}

/// Every target the campaign knows.
pub const TARGETS: &[Target] = &[
    Target {
        name: "xml-fidelity",
        entry_point: "mjx_xml::fidelity::parse + serialize_to_vec (the round-trip oracle)",
        seeds: xml_seeds,
        run: run_xml_fidelity,
    },
    Target {
        name: "xml-dirtied-root",
        entry_point: "mjx_xml::fidelity, with the root dirtied (the verbatim-span oracle)",
        seeds: xml_seeds,
        run: run_xml_dirtied_root,
    },
    Target {
        name: "opc-container",
        entry_point: "mjx_opc::Package::open, on raw container bytes",
        seeds: container_seeds,
        run: run_opc_container,
    },
    Target {
        name: "opc-structured",
        entry_point:
            "mjx_opc::Package::open + validate + save_unchecked, on synthesized containers",
        seeds: recipe_seeds,
        run: run_opc_structured,
    },
    Target {
        name: "mce-resolve",
        entry_point: "mjx_mce::resolve + NamespaceScope",
        seeds: mce_seeds,
        run: run_mce_resolve,
    },
];

// -------------------------------------------------------------------------------------------
// Seeds
// -------------------------------------------------------------------------------------------

/// XML seeds: the shared adversarial corpus, plus real Office markup pulled out of the fixtures.
///
/// Both halves are needed. The adversarial list is where the reader's edges are; the fixture parts
/// are the only source of markup with real prefixes, real namespace declarations and real
/// `mc:AlternateContent`, which is what lets a mutant stay plausible long enough to reach code past
/// the tokenizer.
fn xml_seeds() -> Vec<Vec<u8>> {
    let mut seeds: Vec<Vec<u8>> = mjx_fixtures::adversarial_xml()
        .iter()
        .map(|case| case.to_vec())
        .collect();
    seeds.extend(fixture_xml_parts());
    seeds
}

/// Every XML part of every package fixture that fits under the mutator's input ceiling.
///
/// A part larger than the ceiling would be truncated on its first mutation, so seeding it would put a
/// half-document in the corpus and claim it as real markup. Skipping it is honest; in the committed
/// corpus it skips nothing, because no fixture part is that large.
fn fixture_xml_parts() -> Vec<Vec<u8>> {
    let mut parts = Vec::new();
    for name in mjx_fixtures::package_fixtures() {
        let Ok(package) = Package::open(&mjx_fixtures::fixture(&name)) else {
            continue;
        };
        for entry in package.entries() {
            if !entry.name.ends_with(".xml") && !entry.name.ends_with(".rels") {
                continue;
            }
            if let Some(bytes) = entry.bytes() {
                if bytes.len() <= crate::fuzz::mutate::MAXIMUM_INPUT {
                    parts.push(bytes.to_vec());
                }
            }
        }
    }
    parts
}

/// Container seeds: the committed fixtures themselves, plus hand-built containers whose headers lie.
fn container_seeds() -> Vec<Vec<u8>> {
    let mut seeds: Vec<Vec<u8>> = mjx_fixtures::package_fixtures()
        .iter()
        .map(|name| mjx_fixtures::fixture(name))
        .filter(|bytes| bytes.len() <= crate::fuzz::mutate::MAXIMUM_INPUT)
        .collect();
    seeds.extend(hostile_containers());
    seeds
}

/// Containers a ZIP writer would never produce, each aimed at one thing an opener might trust.
fn hostile_containers() -> Vec<Vec<u8>> {
    let types = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/></Types>"#.to_vec();
    let rels = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#.to_vec();

    let mut out = Vec::new();
    out.push(container::build(&[]));
    out.push(container::build(&[Entry::stored(
        "[Content_Types].xml",
        types.clone(),
    )]));

    // A header that claims four gigabytes for four bytes of payload.
    let mut lying = Entry::stored("a.xml", b"<a/>".to_vec());
    lying.declared_uncompressed_size = Some(u32::MAX - 1);
    out.push(container::build(&[
        Entry::stored("[Content_Types].xml", types.clone()),
        Entry::stored("_rels/.rels", rels.clone()),
        lying,
    ]));

    for name in [
        "../escape.xml",
        "/absolute.xml",
        "a/../../b.xml",
        "..\\windows.xml",
        "",
        "a\u{0}b.xml",
        "[Content_Types].xml",
    ] {
        out.push(container::build(&[
            Entry::stored("[Content_Types].xml", types.clone()),
            Entry::stored("_rels/.rels", rels.clone()),
            Entry::stored(name, b"<a/>".to_vec()),
        ]));
    }

    // A relationship graph that points at itself, and one that points nowhere.
    for target in ["/_rels/.rels", "../../etc/passwd", "a.xml", "http://x/y"] {
        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="urn:t" Target="{target}"/></Relationships>"#
        );
        out.push(container::build(&[
            Entry::stored("[Content_Types].xml", types.clone()),
            Entry::stored("_rels/.rels", body.into_bytes()),
        ]));
    }

    // A container whose CRC does not match its bytes.
    let mut wrong_crc = Entry::stored("a.xml", b"<a/>".to_vec());
    wrong_crc.declared_crc = Some(0);
    out.push(container::build(&[
        Entry::stored("[Content_Types].xml", types),
        Entry::stored("_rels/.rels", rels),
        wrong_crc,
    ]));

    out
}

/// Recipe seeds for the structured OPC target: short byte strings the decoder turns into containers.
///
/// They are short on purpose. The decoder reads a recipe left to right, so a short seed is one the
/// mutator can extend in every direction, and every extension is still a *valid container* — which
/// is the whole reason this target exists alongside the raw-bytes one.
fn recipe_seeds() -> Vec<Vec<u8>> {
    vec![
        vec![1, 0, 0],
        vec![2, 0, 0, 1, 0],
        vec![3, 0, 0, 1, 0, 2, 0],
        vec![3, 0, 0, 1, 0, 2, 0x0f],
        vec![4, 0, 0, 1, 0, 2, 0, 3, 0],
        b"\x02\x00\x00\x05\x00<a/>".to_vec(),
    ]
}

/// MCE seeds: the XML corpus, plus compatibility markup the resolver is actually gated on.
fn mce_seeds() -> Vec<Vec<u8>> {
    let mut seeds = xml_seeds();
    const MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
    seeds.push(format!(r#"<r xmlns:mc="{MC}" xmlns:n="urn:n" xmlns:o="urn:o"><mc:AlternateContent><mc:Choice Requires="n"><n:s/></mc:Choice><mc:Fallback><o:s/></mc:Fallback></mc:AlternateContent></r>"#).into_bytes());
    seeds.push(format!(r#"<r xmlns:mc="{MC}" mc:Ignorable="q" mc:ProcessContent="q"><q:x xmlns:q="urn:q"><b/></q:x></r>"#).into_bytes());
    seeds.push(
        format!(r#"<r xmlns:mc="{MC}" mc:MustUnderstand="z" xmlns:z="urn:z"><a/></r>"#)
            .into_bytes(),
    );
    seeds.push(format!(r#"<r xmlns:mc="{MC}"><mc:AlternateContent><mc:Choice><a/></mc:Choice></mc:AlternateContent></r>"#).into_bytes());
    seeds.push(format!(r#"<r xmlns:mc="{MC}" mc:Ignorable="undeclared undeclared2"><mc:AlternateContent><mc:AlternateContent><mc:Fallback><a/></mc:Fallback></mc:AlternateContent></mc:AlternateContent></r>"#).into_bytes());
    seeds
}

// -------------------------------------------------------------------------------------------
// The XML targets
// -------------------------------------------------------------------------------------------

/// The round-trip oracle: whatever parses must come back byte-for-byte.
///
/// Far stronger than "did not panic". A reader that silently drops a byte passes a
/// does-not-crash run and fails here, which is how the `<!DOCTYPE a>` space loss was caught by hand
/// before this campaign existed.
fn run_xml_fidelity(input: &[u8]) -> Outcome {
    let mut outcome = Outcome::default();
    match fidelity::parse(input) {
        Err(error) => outcome.note("error", error_label(&error)),
        Ok(document) => {
            let shape = measure(&document.root);
            outcome.note("nodes", bucket(shape.nodes));
            outcome.note("depth", bucket(shape.depth));
            outcome.note("attributes", bucket(shape.attributes));
            outcome.note("kinds", shape.kinds);
            outcome.note("prologue", document.prologue.len().min(4));
            outcome.note("epilogue", document.epilogue.len().min(4));

            let written = fidelity::serialize_to_vec(&document);
            outcome.note("round-trips", written == input);
            if written != input {
                outcome.fault(
                    "round-trip",
                    format!(
                        "parse accepted {} bytes and serialize produced {} that differ",
                        input.len(),
                        written.len()
                    ),
                );
            }
        }
    }
    outcome
}

/// The verbatim-span oracle: dirty the root, and what comes out must still be the document it
/// describes.
///
/// Dirtying the root forces the serializer to mix a reconstructed start tag with verbatim children,
/// which is the one arrangement in which a byte range that does not describe its element produces
/// plausible-looking wrong bytes rather than a reflow.
fn run_xml_dirtied_root(input: &[u8]) -> Outcome {
    let mut outcome = Outcome::default();
    let Ok(mut document) = fidelity::parse(input) else {
        outcome.note("unparsed", true);
        return outcome;
    };
    let before = measure(&document.root);
    outcome.note("nodes", bucket(before.nodes));
    outcome.note("depth", bucket(before.depth));

    document.root.empty = false;
    document
        .root
        .children
        .push(RawNode::Comment(Box::from(&b"x"[..])));
    let written = fidelity::serialize_to_vec(&document);

    match fidelity::parse(&written) {
        Err(error) => {
            outcome.note("reparse-error", error_label(&error));
            outcome.fault(
                "dirty-root-unparseable",
                format!("dirtying the root produced output that will not parse: {error}"),
            );
        }
        Ok(reparsed) => {
            let after = measure(&reparsed.root);
            outcome.note("after", (bucket(after.nodes), bucket(after.depth)));
            if reparsed.root.children.len() != document.root.children.len() {
                outcome.fault(
                    "dirty-root-child-count",
                    format!(
                        "the root had {} children and came back with {}",
                        document.root.children.len(),
                        reparsed.root.children.len()
                    ),
                );
            }
        }
    }
    outcome
}

/// What an element tree looks like, measured **iteratively**.
///
/// A recursive measurement would overflow the stack on exactly the inputs this campaign is meant to
/// survive, and a harness that dies on the input it is measuring reports nothing.
#[derive(Debug, Default)]
struct Shape {
    nodes: usize,
    depth: usize,
    attributes: usize,
    /// A bit per node kind seen, so "this input contains CDATA" is part of the signature.
    kinds: u8,
}

fn measure(root: &RawElement) -> Shape {
    let mut shape = Shape::default();
    let mut stack: Vec<(&RawElement, usize)> = vec![(root, 1)];
    while let Some((element, depth)) = stack.pop() {
        shape.nodes += 1;
        shape.depth = shape.depth.max(depth);
        shape.attributes += element.attributes.len();
        shape.kinds |= 1;
        for child in element.children.iter() {
            match child {
                RawNode::Element(child) => stack.push((child, depth + 1)),
                RawNode::Text(_) => shape.kinds |= 1 << 1,
                RawNode::CData(_) => shape.kinds |= 1 << 2,
                RawNode::Comment(_) => shape.kinds |= 1 << 3,
                RawNode::ProcessingInstruction(_) => shape.kinds |= 1 << 4,
                RawNode::Declaration(_) => shape.kinds |= 1 << 5,
                RawNode::DocType(_) => shape.kinds |= 1 << 6,
            }
        }
    }
    shape
}

/// Coarsens a count to its order of magnitude, so the signature tracks behaviour rather than size.
fn bucket(value: usize) -> u32 {
    usize::BITS - value.leading_zeros()
}

fn error_label(error: &mjx_xml::XmlError) -> &'static str {
    match error {
        mjx_xml::XmlError::Syntax(_) => "syntax",
        mjx_xml::XmlError::Utf8(_) => "utf8",
        mjx_xml::XmlError::DepthLimit { .. } => "depth-limit",
    }
}

// -------------------------------------------------------------------------------------------
// The OPC targets
// -------------------------------------------------------------------------------------------

/// The raw-container target: whatever the bytes are, opening them must not panic.
fn run_opc_container(input: &[u8]) -> Outcome {
    let mut outcome = Outcome::default();
    inspect_package(&mut outcome, Package::open(input));
    outcome
}

/// The structured target: the recipe is decoded into a *well-formed* container with hostile
/// contents, so the campaign reaches part naming, content types and the relationship graph rather
/// than stopping at the central directory.
fn run_opc_structured(input: &[u8]) -> Outcome {
    let mut outcome = Outcome::default();
    let entries = decode_recipe(input);
    outcome.note("entries", entries.len());
    let bytes = container::build(&entries);
    inspect_package(&mut outcome, Package::open(&bytes));
    outcome
}

/// What an opened package must satisfy: nothing panics, and a package written back and reopened
/// holds the same part bytes.
///
/// Byte identity per part is the project's round-trip contract, so this is the OPC oracle — the
/// counterpart of `serialize_to_vec(&doc) == input` on the XML side. A package the opener accepts
/// and the validator rejects is a legitimate outcome and is recorded, not faulted.
fn inspect_package(outcome: &mut Outcome, opened: Result<Package, mjx_opc::OpcError>) {
    let package = match opened {
        Err(error) => {
            outcome.note("error", opc_error_label(&error));
            return;
        }
        Ok(package) => package,
    };

    outcome.note("entries", bucket(package.entries().len()));
    outcome.note("parts", bucket(package.part_names().count()));
    outcome.note("relationships", bucket(package.relationships().len()));
    match package.validate() {
        Ok(()) => outcome.note("valid", true),
        Err(defect) => outcome.note("defect", defect_label(&defect)),
    }

    let before: Vec<(String, Option<Vec<u8>>)> = package
        .entries()
        .iter()
        .map(|entry| (entry.name.clone(), entry.bytes().map(<[u8]>::to_vec)))
        .collect();

    match package.save_unchecked() {
        Err(error) => outcome.note("save-error", opc_error_label(&error)),
        Ok(written) => match Package::open(&written) {
            Err(error) => {
                outcome.note("reopen-error", opc_error_label(&error));
                outcome.fault(
                    "reopen",
                    format!("a package this library wrote could not be reopened: {error}"),
                );
            }
            Ok(reopened) => {
                outcome.note("reopened", true);
                let after: Vec<(String, Option<Vec<u8>>)> = reopened
                    .entries()
                    .iter()
                    .map(|entry| (entry.name.clone(), entry.bytes().map(<[u8]>::to_vec)))
                    .collect();
                if before != after {
                    outcome.fault(
                        "part-byte-identity",
                        format!(
                            "{} entries went in and {} came back, or their bytes changed",
                            before.len(),
                            after.len()
                        ),
                    );
                }
            }
        },
    }
}

fn defect_label(defect: &mjx_opc::PackageDefect) -> &'static str {
    use mjx_opc::PackageDefect as D;
    match defect {
        D::PartWithoutContentType { .. } => "no-content-type",
        D::UnresolvableRelationshipTarget { .. } => "unresolvable-target",
        D::RelationshipTargetMissing { .. } => "missing-target",
        D::DuplicateRelationshipId { .. } => "duplicate-id",
        D::UndeclaredRelationshipReference { .. } => "undeclared-reference",
        D::PartIsNotWellFormedXml { .. } => "not-well-formed",
        // `PackageDefect` is `#[non_exhaustive]`; a defect added later groups here until a campaign
        // wants to tell it apart, which costs one arm.
        _ => "other",
    }
}

fn opc_error_label(error: &mjx_opc::OpcError) -> &'static str {
    use mjx_opc::OpcError as E;
    match error {
        E::Zip(_) => "zip",
        E::Io(_) => "io",
        E::Invalid(_) => "invalid",
        E::Xml(_) => "xml",
        E::Malformed(_) => "malformed",
        E::UnknownPart(_) => "unknown-part",
        E::ExternalTarget(_) => "external-target",
        E::TargetResolution(_) => "target-resolution",
        E::ControlPart(_) => "control-part",
    }
}

/// Part names a recipe can select. Enough real ones that a synthesized package can be *valid*, and
/// enough hostile ones that it can be anything but.
const RECIPE_NAMES: &[&str] = &[
    "[Content_Types].xml",
    "_rels/.rels",
    "ppt/presentation.xml",
    "ppt/_rels/presentation.xml.rels",
    "ppt/slides/slide1.xml",
    "word/document.xml",
    "xl/workbook.xml",
    "docProps/core.xml",
    "media/image1.png",
    "../escape.xml",
    "/absolute.xml",
    "a/../b.xml",
    "",
    "a b.xml",
    "A.XML",
    "a.xml",
];

/// Payload templates a recipe can select, so a synthesized container carries markup the control-part
/// parsers will actually engage with rather than random bytes.
const RECIPE_PAYLOADS: &[&[u8]] = &[
    br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/></Types>"#,
    br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Override PartName="/a.xml" ContentType="application/xml"/></Types>"#,
    br#"<Types/>"#,
    br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#,
    br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="urn:t" Target="a.xml"/><Relationship Id="rId1" Type="urn:t" Target="../b.xml"/></Relationships>"#,
    br#"<a/>"#,
    b"",
    b"not xml",
];

/// Decodes a recipe into container entries.
///
/// The decoder is total: every byte string is a container, and running off the end simply stops the
/// entry list. That totality is the point — the mutator can do anything to a recipe and the result
/// is still a container the opener will read, so mutation pressure lands on packaging structure
/// instead of on the ZIP checksum.
fn decode_recipe(input: &[u8]) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut cursor = input.iter().copied();
    let count = usize::from(cursor.next().unwrap_or(1) % 12);
    for _ in 0..count.max(1) {
        let Some(name_index) = cursor.next() else {
            break;
        };
        let flags = cursor.next().unwrap_or(0);
        let name = RECIPE_NAMES[usize::from(name_index) % RECIPE_NAMES.len()];
        let payload = RECIPE_PAYLOADS[usize::from(flags >> 4) % RECIPE_PAYLOADS.len()];
        let mut entry = Entry::stored(name, payload.to_vec());
        if flags & 1 != 0 {
            entry.declared_uncompressed_size = Some(u32::MAX - 1);
        }
        if flags & 2 != 0 {
            entry.declared_compressed_size = Some(u32::MAX - 1);
        }
        if flags & 4 != 0 {
            entry.declared_crc = Some(0);
        }
        if flags & 8 != 0 {
            entry.method = 8; // claims deflate over bytes that are stored
        }
        entries.push(entry);
    }
    entries
}

// -------------------------------------------------------------------------------------------
// The MCE target
// -------------------------------------------------------------------------------------------

/// Resolution must terminate and must not panic, whatever the compatibility markup says.
///
/// Three understanding sets, because the resolver's branches are chosen by what the consumer claims
/// to understand: nothing (every `Choice` fails, `Fallback` wins), the seeds' namespaces (a `Choice`
/// wins), and a set that satisfies `MustUnderstand`.
fn run_mce_resolve(input: &[u8]) -> Outcome {
    let mut outcome = Outcome::default();
    let Ok(document) = fidelity::parse(input) else {
        outcome.note("unparsed", true);
        return outcome;
    };
    let understood = [
        UnderstoodNamespaces::new(),
        UnderstoodNamespaces::from_uris(["urn:n", "urn:a", "urn:x"]),
        UnderstoodNamespaces::from_uris(["urn:n", "urn:o", "urn:q", "urn:z", "urn:a", "urn:b"]),
    ];
    for (index, set) in understood.iter().enumerate() {
        match mjx_mce::resolve(&document, set) {
            Err(error) => outcome.note(
                "error",
                (
                    index,
                    match error {
                        mjx_mce::ResolveError::MustUnderstand(_) => "must-understand",
                        mjx_mce::ResolveError::MalformedAlternateContent(_) => "malformed",
                    },
                ),
            ),
            Ok(resolved) => {
                let (nodes, attributes) = count_resolved(&resolved);
                outcome.note("resolved", (index, bucket(nodes), bucket(attributes)));
            }
        }
    }
    outcome
}

/// Counts a resolved view iteratively — same reason as [`measure`].
fn count_resolved(root: &mjx_mce::ResolvedElement<'_>) -> (usize, usize) {
    let mut nodes = 0usize;
    let mut attributes = 0usize;
    let mut stack = vec![root];
    while let Some(element) = stack.pop() {
        nodes += 1;
        attributes += element.attributes.len();
        for child in &element.children {
            match child {
                mjx_mce::ResolvedNode::Element(child) => stack.push(child),
                mjx_mce::ResolvedNode::Text(_) | mjx_mce::ResolvedNode::CData(_) => nodes += 1,
            }
        }
    }
    (nodes, attributes)
}

/// Every target name, for the command's help text and for the "all targets ran" check.
#[must_use]
pub fn names() -> HashSet<&'static str> {
    TARGETS.iter().map(|target| target.name).collect()
}

#[cfg(test)]
mod tests {
    use super::{decode_recipe, names, TARGETS};

    #[test]
    fn every_target_has_seeds_and_a_distinct_name() {
        assert_eq!(names().len(), TARGETS.len(), "two targets share a name");
        for target in TARGETS {
            let seeds = (target.seeds)();
            assert!(
                !seeds.is_empty(),
                "{} has an empty seed corpus, so its campaign would explore nothing",
                target.name
            );
        }
    }

    #[test]
    fn the_recipe_decoder_is_total() {
        // Every byte string must decode, or the structured target would have holes the mutator
        // could fall into and the campaign would quietly stop covering packaging structure.
        for input in [
            b"".as_slice(),
            b"\x00",
            b"\xff\xff\xff",
            b"\x0b\x0f\xff\x0f\xff\x0f\xff",
        ] {
            let _ = decode_recipe(input);
        }
    }
}
