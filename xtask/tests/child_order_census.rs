//! **Four serialization ledgers ask what a hand-written pair *loses*. This one asks what it
//! *moves*.** (MJXOFF-265, closing the class MJXOFF-251 found one instance of.)
//!
//! # The gap, stated exactly
//!
//! `crates/mjx-dml/tests/serialization_ledger.rs` (MJXOFF-217),
//! `crates/mjx-sml/tests/serialization_ledger.rs` (MJXOFF-220),
//! `crates/mjx-docx/tests/serialization_ledger.rs` (MJXOFF-218) and
//! `xtask/tests/upper_markup_ledger.rs` (MJXOFF-221) hold every hand-written `FromXml`/`ToXml` pair
//! in the workspace to an idiom, and every one of those idioms is a **conservation** claim: the
//! element's name, its attributes, its self-closing flag and every child it does not model come
//! back. **Child order is outside all four claims by construction** — `Idiom::PropertiesAndGroup`'s
//! own documentation said *"the children are never filtered"*, which was true, and the defect was
//! that they were re-ordered.
//!
//! MJXOFF-251 is what that cost. Six `mjx-docx` types read one named child from anywhere among
//! their children and wrote it back **first**, so `<w:customXml><w:p/><w:customXmlPr/></w:customXml>`
//! came back with its two children swapped, and — because indentation is made of text nodes and a
//! text node is a child — a merely *pretty-printed* wrapper had its own newline stepped over too.
//! Nothing was ever dropped, so all four ledgers stayed green, and the committed fixture corpus is
//! canonical, so the preservation gate reproduced the order it was given and saw nothing either.
//!
//! # Why this file scans **functions** and not impl bodies
//!
//! This is the whole reason the ledgers could not be extended to answer it. MJXOFF-251's defect was
//! not in an `impl` at all: the six impls each called two free functions,
//! `split_leading_properties` and `join_properties_and_group`, and the second one read
//!
//! ```text
//! let mut children = Vec::with_capacity(group.children.len() + 1);
//! if let Some(properties) = properties {
//!     children.push(RawNode::Element(properties.to_xml(interner)));
//! }
//! children.extend(group.children);
//! ```
//!
//! A scanner over impl bodies sees `join_properties_and_group(interner, &self.properties, group_xml)`
//! and learns nothing. So the population here is **every function in every workspace member's
//! `src/` that handles a child sequence at all** — a helper is a function, so following into helpers
//! is free, and no call graph is needed.
//!
//! **This is the whole difference between a census and a spot check, and it is worth stating in
//! full.** A gate built the obvious way — extend the four ledgers, which already scan impl bodies —
//! would have run over MJXOFF-251's six types, matched nothing in any of their bodies, and reported
//! the class clean. It would have been *right about every impl it looked at* and wrong about the
//! workspace, because the two functions that actually moved the child were not impls. That is the
//! difference between *"I looked and found nothing"* and *"my search could not have found it"*, and
//! the second sentence is the one H13 was honest enough to write about its own search. Scanning
//! functions is what makes the first sentence sayable here.
//!
//! # The two shapes, and why they are the shapes
//!
//! A child sequence read out of a file and written back can only lose its order two ways, and
//! MJXOFF-251's defect was made of one of each — which is why both are checked rather than one.
//!
//! * **[`hoisted_child_vector`] — on write.** A `Vec` that is both `push`ed into and `extend`ed
//!   from, or `insert`ed into at a computed index, is a vector whose order is decided by the code
//!   rather than by the file. `join_properties_and_group` above is exactly this.
//! * **[`child_dropped_from_its_sequence`] — on read.** An `Option` set from inside a loop over a
//!   child sequence is a child taken *out* of that sequence; unless the index travels with it,
//!   whatever puts it back is guessing. `split_leading_properties` was exactly this.
//!
//! # The detectors are deliberately over-inclusive
//!
//! Every site either shape finds is on [`ORDER_MOVING_SITES`] with a written reason, and two of the
//! rows there say *the value this loop takes out is not a child at all — it is an index the loop
//! tracks while the child itself is pushed on every arm.* That is not noise, it is the trade this
//! file makes on purpose: **a false positive costs one sentence and a false negative costs a
//! fidelity defect**, so the detectors are drawn wide and the ledger absorbs the difference. A
//! narrower detector tuned to today's population is the thing that develops a false negative
//! silently.
//!
//! The ledger is held to the scan in both directions — [`every_function_that_moves_a_child_is_on_the_ledger`]
//! and [`every_ledger_row_still_names_a_function_that_moves_a_child`] — so it cannot pass forever:
//! a new site fails until somebody writes down why, and a row whose function was rewritten or
//! deleted fails until somebody removes it.
//!
//! # What the census found
//!
//! Six sites, and the answer to MJXOFF-265's question is that **exactly one family of hand-written
//! pairs moves a child**: `mjx-docx`'s [`split_positioned_child`]/[`join_positioned_child`], the six
//! types MJXOFF-251 fixed, where the index now travels with the value. Two further sites move a
//! child for reasons of their own (`mjx-sml`'s packed cell store, which remembers the payload's
//! position as two byte spans; `mjx-mce`'s alternate-content resolution, which is a read-only
//! projection the writer never rebuilds from), one builds fresh markup from owned arguments and has
//! no source order to preserve, and two are the over-inclusion above. **No hand-written pair besides
//! MJXOFF-251's six moves a child it did not lose.** H13 said its own search for a second instance
//! was *"a spot check, not a census"*; this file is the census.
//!
//! # Each row names the markup that proves it
//!
//! A site that genuinely moves a child cannot be argued safe from its source text — that is how
//! MJXOFF-251 survived review, with a helper named for the behaviour it did not have. So a row
//! whose [`Site::proof`] is set names a test file and a needle in it, and
//! [`every_moving_site_is_proved_against_markup`] requires both to still be there. Deleting the
//! eight round-trip cases in `crates/mjx-docx/tests/structured_content.rs` fails here as well as
//! there.
//!
//! # Every arm was made to fail
//!
//! | Mutation | What failed |
//! |---|---|
//! | `join_positioned_child` restored to MJXOFF-251's `push`-then-`extend` body | [`every_ledger_row_still_names_a_function_that_moves_a_child`] (the drop row's function no longer matches) **and** a new unexplained hoist in [`every_function_that_moves_a_child_is_on_the_ledger`] |
//! | a new helper in `mjx-dml` with the pre-fix hoist body | [`every_function_that_moves_a_child_is_on_the_ledger`], naming it |
//! | the `fn` scanner's `\bfn ` needle mistyped | [`the_scanner_still_sees_the_workspaces_child_sequence_handling`] |
//! | the MJXOFF-251 round-trip cases deleted | [`every_moving_site_is_proved_against_markup`] |

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ===============================================================================================
// The ledger
// ===============================================================================================

/// Which of the two shapes a site matches — the vocabulary the detectors speak.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Shape {
    /// A child vector both `push`ed into and `extend`ed from, or `insert`ed into: the order of what
    /// comes out is decided by the code.
    HoistOnWrite,
    /// An `Option` set from inside a loop over a child sequence: a child taken out of the sequence.
    DropOnRead,
}

/// One function in the workspace that moves a child out of, or into, a sequence.
struct Site {
    /// The source file, relative to the repository root. **The first string of a row on purpose:**
    /// `xtask/tests/derived_rosters.rs` sweeps for literal lists whose first string is a member of a
    /// derived population, and a path is a member of none, so this ledger of *sites* is not mistaken
    /// for a roster of crates.
    file: &'static str,
    /// The function's name, as `fn <name>` is written.
    function: &'static str,
    /// The shape the detector matched.
    shape: Shape,
    /// Why this site does not leave a child somewhere the file did not put it. A reader who
    /// disagrees with this sentence has found a defect.
    reason: &'static str,
    /// Where the claim is proved against markup, as `(test file, a needle that must occur in it)` —
    /// `None` where the site moves nothing and there is nothing to prove.
    proof: Option<(&'static str, &'static str)>,
}

/// Every function in the workspace that matches either shape.
const ORDER_MOVING_SITES: &[Site] = &[
    Site {
        file: "crates/mjx-docx/src/document/structured_content.rs",
        function: "split_positioned_child",
        shape: Shape::DropOnRead,
        reason: "MJXOFF-251's fix, and the one family of hand-written pairs in the workspace that \
                 genuinely moves a child: `Placeholder`, the four `CustomXml*` wrappers and \
                 `SmartTagRun` each name one child and pass the rest through. The index the named \
                 child sat at is read with it into a `PositionedChild` and restored by \
                 `join_positioned_child`, so the writer's output is a function of the file rather \
                 than of `wml.xsd`. The counting is over *nodes*, so a whitespace text node before \
                 the properties element keeps its place too.",
        proof: Some((
            "crates/mjx-docx/tests/structured_content.rs",
            "a_block_custom_xml_wrapper_keeps_a_properties_child_the_file_put_second",
        )),
    },
    Site {
        file: "crates/mjx-sml/src/cells/read.rs",
        function: "read_cell_content",
        shape: Shape::DropOnRead,
        reason: "The packed cell store takes `x:v`/`x:is` and `x:f` out of the cell's children so \
                 the value and the formula can be held decoded rather than as a tree. The position \
                 travels, by a different mechanism than a `PositionedChild`: \
                 `read_cell_content_from_model` serializes everything before the payload into \
                 `before_payload` and everything after it into `after_payload`, and \
                 `cells/write.rs` writes the two spans around the payload it rebuilds.",
        proof: Some((
            "crates/mjx-sml/tests/cell_store_fidelity.rs",
            // The discriminating worksheet's `A2`, whose formula is written **before** its value —
            // and its `B2`, whose `x:extLst` is written after one. A cell store that rebuilt in
            // schema order rather than in the file's would change both.
            r#"<x:f t="shared" si="0">SUM(A1:C1)</x:f><x:v>3</x:v>"#,
        )),
    },
    Site {
        file: "crates/mjx-mce/src/resolve.rs",
        function: "resolve_alternate_content",
        shape: Shape::DropOnRead,
        reason: "MCE resolution picks the first `mc:Choice` whose `@Requires` the reader \
                 understands, or the `mc:Fallback`. It is a **read-only projection** and the reason \
                 nothing is moved is that nothing is written back from it: `CLAUDE.md`'s MCE rule \
                 is *preserved on write and resolved (non-mutating) on read/render*, so the \
                 `mc:AlternateContent` element the file holds is re-emitted from its own children, \
                 never from the branch this function chose.",
        proof: None,
    },
    Site {
        file: "crates/mjx-omml/src/objects.rs",
        function: "new",
        shape: Shape::HoistOnWrite,
        reason: "`Delimiter::new` — MJXOFF-251's exact `push`-then-`extend` body, in a **constructor**. \
                 It takes owned `properties` and `arguments` and builds `<m:d>` from them; no \
                 element was read, so there is no source order for it to contradict. This is the \
                 row that says the hoist shape is only a defect when the sequence being extended \
                 came out of a file.",
        proof: None,
    },
    Site {
        file: "crates/mjx-ooxml-types/src/child_order.rs",
        function: "first_out_of_order",
        shape: Shape::DropOnRead,
        reason: "Over-inclusion, and left in rather than tuned away: the `Option` this loop sets is \
                 the **rank** of the previous child, not the child. Nothing is taken out of the \
                 sequence — this function reads a sequence of names and reports the first one whose \
                 rank went backwards, and writes nothing at all.",
        proof: None,
    },
    Site {
        file: "crates/mjx-schema-gate/src/wildcard_slots.rs",
        function: "holds_nothing_but_a_required_wildcard",
        shape: Shape::DropOnRead,
        reason: "Over-inclusion, and the row that shows the scan really is over every workspace \
                 member rather than the shipped graph: the children being looped over are an \
                 `xsd:complexType`'s, in a **schema** the gate reads, and the `Option` holds the \
                 model group it found. Nothing here reads or writes an OOXML part.",
        proof: None,
    },
    Site {
        file: "crates/mjx-sml/src/cells/read.rs",
        function: "read_row",
        shape: Shape::DropOnRead,
        reason: "Over-inclusion, the same way: the `Option` is the previous cell's **column index**, \
                 kept so a row with out-of-order or repeated references is detected. Every child of \
                 the row is read in turn and none is held back.",
        proof: None,
    },
];

// ===============================================================================================
// The scanner
// ===============================================================================================

/// The repository root — `xtask/`'s parent.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// `Cargo.toml`'s `members`, as Cargo declares them — the same derivation `doc_gate.rs` and
/// `derived_rosters.rs` use, and for the same reason: a list of crates typed out here would be a
/// list that stops being the workspace the day a crate joins it.
fn declared_members() -> BTreeSet<String> {
    let manifest = std::fs::read_to_string(repository_root().join("Cargo.toml"))
        .expect("reading the workspace manifest");
    let mut members = BTreeSet::new();
    let mut inside = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("members") && trimmed.contains('[') {
            inside = true;
            continue;
        }
        if inside {
            if trimmed.starts_with(']') {
                break;
            }
            if let Some(path) = trimmed
                .trim_end_matches(',')
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
            {
                members.insert(path.to_owned());
            }
        }
    }
    assert!(
        members.len() >= 15,
        "only {} workspace member(s) were read out of Cargo.toml — the manifest parser has stopped \
         matching, and this census would be over almost nothing",
        members.len()
    );
    members
}

/// Every `.rs` file under every member's `src/`, keyed by its path relative to the repository root.
///
/// `src/` and not `tests/`: a test that reorders children on purpose is what this file wants more
/// of, not less.
fn shipped_sources() -> BTreeMap<String, String> {
    fn walk(directory: &Path, root: &Path, into: &mut BTreeMap<String, String>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(&path, root, into);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let relative = path
                    .strip_prefix(root)
                    .expect("under the repository root")
                    .to_string_lossy()
                    .replace('\\', "/");
                into.insert(
                    relative,
                    std::fs::read_to_string(&path).expect("reading a source file"),
                );
            }
        }
    }
    let root = repository_root();
    let mut sources = BTreeMap::new();
    for member in declared_members() {
        walk(&root.join(&member).join("src"), &root, &mut sources);
    }
    sources
}

/// The `{ … }` block beginning at or after `from`, brace-balanced.
///
/// Adequate for these sources, which hold no unbalanced `{` inside a string or character literal in
/// any function that touches a child sequence; a body that grew one would over-run and land on the
/// ledger loudly rather than pass quietly, which is the right way round for a gate.
fn balanced_block(text: &str, from: usize) -> String {
    let bytes = text.as_bytes();
    let mut at = from;
    while at < bytes.len() && bytes[at] != b'{' {
        at += 1;
    }
    let start = at;
    let mut depth = 0usize;
    while at < bytes.len() {
        match bytes[at] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return text[start..=at].to_owned();
                }
            }
            _ => {}
        }
        at += 1;
    }
    text[start..].to_owned()
}

/// One function written out in a source file.
struct Function {
    /// The source file, relative to the repository root.
    file: String,
    /// The name, as `fn <name>` is written — a `new` in two files is two rows, and the ledger keys
    /// on the pair.
    name: String,
    /// The body, braces included, with every run of whitespace collapsed to one space.
    body: String,
}

/// Every function in every member's `src/`.
///
/// Including the ones inside `macro_rules!` bodies, which is deliberate: `mjx-dml`, `mjx-chart`,
/// `mjx-omml` and `mjx-vml` reach XML through a `fidelity_*!` macro, and a hoist written into one of
/// those would be a hoist in every type on it.
fn functions() -> Vec<Function> {
    let mut found = Vec::new();
    for (file, text) in shipped_sources() {
        for (at, _) in text.match_indices("fn ") {
            // `fn` must be a word: preceded by nothing, whitespace, or the end of `pub`/`unsafe`.
            if at > 0 {
                let previous = text[..at].chars().next_back().unwrap_or(' ');
                if previous.is_alphanumeric() || previous == '_' {
                    continue;
                }
            }
            let after = at + "fn ".len();
            let name: String = text[after..]
                .chars()
                .take_while(|character| character.is_alphanumeric() || *character == '_')
                .collect();
            if name.is_empty() {
                continue;
            }
            let body = balanced_block(&text, after + name.len());
            if !body.starts_with('{') {
                continue;
            }
            found.push(Function {
                file: file.clone(),
                name,
                body: body.split_whitespace().collect::<Vec<_>>().join(" "),
            });
        }
    }
    found
}

/// Whether this body handles a child sequence at all — the population the two detectors run over.
fn handles_a_child_sequence(body: &str) -> bool {
    body.contains("RawNode") || body.contains("children")
}

// ===============================================================================================
// The two detectors
// ===============================================================================================

/// Every `let mut <name> = …` in `body` whose initialiser makes it a `Vec`.
///
/// Restricted to `Vec` on purpose: the three `HashSet::insert` calls in `mjx-pptx`'s and
/// `mjx-xlsx`'s validators are memberships, not positions, and a child sequence is a
/// `Vec<RawNode>`.
fn vector_bindings(body: &str) -> BTreeSet<String> {
    const INITIALISERS: [&str; 4] = [
        "Vec::new()",
        "Vec::with_capacity(",
        "vec![",
        "::std::vec::Vec::new()",
    ];
    let mut bindings = BTreeSet::new();
    for (at, _) in body.match_indices("let mut ") {
        let rest = &body[at + "let mut ".len()..];
        let name: String = rest
            .chars()
            .take_while(|character| character.is_alphanumeric() || *character == '_')
            .collect();
        if name.is_empty() {
            continue;
        }
        // Skip any type annotation, then take the initialiser up to the statement's end.
        let Some(equals) = rest.find('=') else {
            continue;
        };
        let tail = &rest[equals + 1..];
        let initialiser = &tail[..tail.find(';').unwrap_or(tail.len()).min(tail.len())];
        if INITIALISERS
            .iter()
            .any(|needle| initialiser.trim_start().starts_with(needle))
        {
            bindings.insert(name);
        }
    }
    bindings
}

/// The name of a child vector in `body` whose order is decided by the code: one that is both
/// `push`ed into and `extend`ed from, or `insert`ed into.
///
/// **This is MJXOFF-251's writer.** `join_properties_and_group` pushed the properties element and
/// then extended with the group's children, so the properties element came out first whatever the
/// file had said.
fn hoisted_child_vector(body: &str) -> Option<String> {
    vector_bindings(body).into_iter().find(|name| {
        (body.contains(&format!("{name}.push(")) && body.contains(&format!("{name}.extend(")))
            || body.contains(&format!("{name}.insert("))
    })
}

/// The name of an `Option` in `body` that is set from inside a loop over a child sequence — a child
/// taken *out* of the sequence.
///
/// **This is MJXOFF-251's reader.** `split_leading_properties` set `properties` from a match arm
/// that did not push, so the child left the sequence and nothing remembered where it had been.
fn child_dropped_from_its_sequence(body: &str) -> Option<String> {
    if !loops_over_a_child_sequence(body) {
        return None;
    }
    let mut options = BTreeSet::new();
    for (at, _) in body.match_indices("let mut ") {
        let rest = &body[at + "let mut ".len()..];
        let name: String = rest
            .chars()
            .take_while(|character| character.is_alphanumeric() || *character == '_')
            .collect();
        if name.is_empty() {
            continue;
        }
        let Some(equals) = rest.find('=') else {
            continue;
        };
        let tail = &rest[equals + 1..];
        if tail.trim_start().starts_with("None") {
            options.insert(name);
        }
    }
    options
        .into_iter()
        .find(|name| assigned_from_some(body, name))
}

/// Whether `body` contains `<name> = Some(` as a statement rather than as a field or method.
fn assigned_from_some(body: &str, name: &str) -> bool {
    let needle = format!("{name} = Some(");
    body.match_indices(&needle).any(|(at, _)| {
        at == 0
            || !matches!(
                body[..at].chars().next_back(),
                Some(previous) if previous.is_alphanumeric() || previous == '_' || previous == '.'
            )
    })
}

/// Whether `body` iterates something whose name ends in `children`.
fn loops_over_a_child_sequence(body: &str) -> bool {
    body.match_indices("for ").any(|(at, _)| {
        let rest = &body[at..];
        let head = &rest[..rest.find('{').unwrap_or(rest.len())];
        head.contains("children")
    })
}

// ===============================================================================================
// The anti-vacuity floors
// ===============================================================================================

/// The function scanner must still be finding functions. Stated as *the scanner has stopped
/// matching*, never as the exact population — a floor pinned to today's size fires before the
/// assertion it guards and hides the mutation meant to prove it.
const MINIMUM_FUNCTIONS: usize = 5_000;

/// …and enough of them must still be seen to handle a child sequence. This is the floor that
/// matters: a scanner that finds every function but recognises none of them as touching XML would
/// report *no site moves a child* over an empty population and pass, which is precisely the clean
/// bill this file exists not to hand out.
const MINIMUM_CHILD_SEQUENCE_FUNCTIONS: usize = 600;

// ===============================================================================================
// The tests
// ===============================================================================================

/// The scanner is alive, and the population it runs over is the workspace's.
#[test]
fn the_scanner_still_sees_the_workspaces_child_sequence_handling() {
    let functions = functions();
    let handling: Vec<&Function> = functions
        .iter()
        .filter(|function| handles_a_child_sequence(&function.body))
        .collect();
    let files: BTreeSet<&str> = handling
        .iter()
        .map(|function| function.file.as_str())
        .collect();
    println!(
        "child-order census: {} functions in {} workspace members' src/, {} of them handling a \
         child sequence, across {} files",
        functions.len(),
        declared_members().len(),
        handling.len(),
        files.len(),
    );
    assert!(
        functions.len() >= MINIMUM_FUNCTIONS,
        "only {} function(s) found — the function scanner has stopped matching",
        functions.len()
    );
    assert!(
        handling.len() >= MINIMUM_CHILD_SEQUENCE_FUNCTIONS,
        "only {} function(s) seen to handle a child sequence — the child-sequence scanner has \
         stopped matching, and every claim below would be made over almost nothing",
        handling.len()
    );
}

/// Nothing moves a child without a written reason.
#[test]
fn every_function_that_moves_a_child_is_on_the_ledger() {
    let ledger: BTreeSet<(&str, &str)> = ORDER_MOVING_SITES
        .iter()
        .map(|site| (site.file, site.function))
        .collect();
    let mut unexplained = Vec::new();
    let mut found = 0usize;
    for function in functions() {
        if !handles_a_child_sequence(&function.body) {
            continue;
        }
        for (shape, binding) in [
            (Shape::HoistOnWrite, hoisted_child_vector(&function.body)),
            (
                Shape::DropOnRead,
                child_dropped_from_its_sequence(&function.body),
            ),
        ] {
            let Some(binding) = binding else { continue };
            found += 1;
            if !ledger.contains(&(function.file.as_str(), function.name.as_str())) {
                unexplained.push(format!(
                    "{}: fn {} — {shape:?} on `{binding}`",
                    function.file, function.name
                ));
            }
        }
    }
    println!(
        "child-order census: {found} site(s) matched, {} on the ledger",
        ledger.len()
    );
    assert!(
        unexplained.is_empty(),
        "these functions move a child out of, or into, a sequence and nothing says why the file's \
         own order survives it. Add a row to `ORDER_MOVING_SITES` saying what puts the child back \
         where the file had it — or, if nothing does, that is MJXOFF-251 again:\n  {}",
        unexplained.join("\n  ")
    );
    assert!(
        found >= ORDER_MOVING_SITES.len(),
        "only {found} site(s) matched against {} ledger row(s) — the detectors have stopped \
         matching",
        ORDER_MOVING_SITES.len()
    );
}

/// …and no row survives the function it was written about.
#[test]
fn every_ledger_row_still_names_a_function_that_moves_a_child() {
    let functions = functions();
    let mut stale = Vec::new();
    for site in ORDER_MOVING_SITES {
        let matched = functions.iter().any(|function| {
            function.file == site.file
                && function.name == site.function
                && match site.shape {
                    Shape::HoistOnWrite => hoisted_child_vector(&function.body).is_some(),
                    Shape::DropOnRead => child_dropped_from_its_sequence(&function.body).is_some(),
                }
        });
        if !matched {
            stale.push(format!(
                "{}: fn {} no longer matches {:?}",
                site.file, site.function, site.shape
            ));
        }
        assert!(
            site.reason.len() > 80,
            "{}: fn {} has no reason worth reading",
            site.file,
            site.function
        );
    }
    assert!(
        stale.is_empty(),
        "these ledger rows no longer name a function that moves a child — either the function was \
         rewritten (remove the row) or the scanner has stopped matching it (fix the scanner):\n  {}",
        stale.join("\n  ")
    );
}

/// A site that really moves a child is proved against markup, not against its own source text.
#[test]
fn every_moving_site_is_proved_against_markup() {
    let root = repository_root();
    let mut proved = 0usize;
    for site in ORDER_MOVING_SITES {
        let Some((file, needle)) = site.proof else {
            continue;
        };
        let text = std::fs::read_to_string(root.join(file)).unwrap_or_else(|_| {
            panic!(
                "{}: fn {}'s proof file {file} is gone",
                site.file, site.function
            )
        });
        assert!(
            text.contains(needle),
            "{}: fn {} is proved by `{needle}` in {file}, and that is no longer there — a site that \
             moves a child cannot be argued safe from its source text, which is how MJXOFF-251 \
             survived review",
            site.file,
            site.function
        );
        proved += 1;
    }
    println!("child-order census: {proved} moving site(s) proved against markup");
    assert!(
        proved >= 2,
        "only {proved} site(s) carry a proof — the ledger has stopped naming the markup that \
         establishes them"
    );
}
