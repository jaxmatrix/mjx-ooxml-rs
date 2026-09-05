//! The layering rule, checked against the real dependency graph (MJXOFF-132).
//!
//! `CLAUDE.md` opens its architecture rules with *"dependencies point **downward only**"* and then
//! writes the tiers out. Until this file, that was the one architectural rule in the repository with
//! **no mechanical check** — and MJXOFF-99 and MJXOFF-112 are both specified to rely on it existing,
//! because both hang on `mjx-chart -> mjx-sml` being legal and `mjx-chart -> mjx-xlsx` not being.
//!
//! # Why this reads `cargo metadata` rather than the manifests
//!
//! A hand-rolled scan of each `Cargo.toml` would have to guess at every spelling a dependency can
//! take — `dep.workspace = true`, an inline table, a `[dependencies.dep]` sub-table, a
//! `[target.'cfg(…)'.dependencies]` section, a `package = "…"` rename — and a spelling it did not
//! recognise would *drop an edge silently*, which is the one failure mode a gate must not have.
//! `cargo metadata --no-deps` is Cargo's own answer to "what does this member declare", so an edge
//! cannot hide from it. `--no-deps` means no resolution, no lockfile write and no network.
//!
//! # Why the tier table lives here and not only in prose
//!
//! Shared markup is **not flat**. `mjx-chart` may depend on `mjx-dml` and (from MJXOFF-112) on
//! `mjx-sml`, while `mjx-dml -> mjx-sml` and `mjx-sml -> mjx-chart` must stay illegal, so a single
//! "shared markup" tier could not express the rule the workspace actually holds. The sub-ranks are
//! in [`TIERS`] below and mirrored into `CLAUDE.md`; an edge is legal **iff** it points to a
//! *strictly* lower rank, which also makes the graph acyclic by construction.
//!
//! # The trap this file is written against
//!
//! A tier table that no crate's edges exercise is satisfied by a graph that never had a violation —
//! it would pass on an empty workspace. Three things stop that here:
//!
//! 1. [`every_workspace_member_has_a_declared_tier`] fails on a member with no entry **and** on an
//!    entry naming no member, so the table cannot drift away from the workspace.
//! 2. [`every_dependency_points_strictly_downward`] counts the edges it checked and refuses to pass
//!    on none — a vacuous run is a failure, not a green.
//! 3. It was proved by mutation, each red naming both crates and both ranks:
//!    `mjx-omml -> mjx-pptx` (upward, 2.2 -> 3.0), `mjx-sml -> mjx-chart` (an inversion inside the
//!    shared-markup tier, 2.1 -> 2.2) and `mjx-chart -> mjx-vml` (equal rank, 2.2 -> 2.2, which is
//!    what "strictly" buys), plus removing a row from [`TIERS`], which reports the member it no
//!    longer covers.
//!
//! # What Cargo already catches, and what it does not
//!
//! Cargo refuses a **cyclic** package dependency outright, before a test binary is built. Most of
//! the tempting upward edges in this workspace are cyclic — `mjx-dml -> mjx-pptx` closes
//! `mjx-pptx -> mjx-chart -> mjx-dml`, and Cargo names that cycle rather than letting this file
//! speak — so it is worth being clear about the division: **Cargo enforces acyclicity, and this file
//! enforces direction.** An upward or sideways edge whose reverse does not already exist is
//! perfectly legal to Cargo and silent without this check, which is why the mutations above are
//! chosen from that set. `mjx-chart -> mjx-xlsx`, the edge MJXOFF-99 would have needed without
//! `mjx-sml`, is exactly such an edge: acyclic, buildable, and wrong.

use std::collections::BTreeMap;
use std::fmt;
use std::process::Command;

/// Where a crate sits. The ranked tiers are the shipped graph; the last three are outside it and say
/// so, because `CLAUDE.md` places them outside it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tier {
    /// `mjx-ooxml-core` and `mjx-derive` — rank 0.0, the floor of the workspace. Neither declares
    /// a workspace dependency of any kind.
    FoundationsCore,
    /// `mjx-xml` — rank 0.1. The foundations are *not* flat: `mjx-xml` is built on
    /// `mjx-ooxml-core`'s `RawElement`/`Interner`, so it sits one step above it.
    FoundationsXml,
    /// `mjx-ooxml-types`, `mjx-opc`, `mjx-mce` — rank 1.0.
    Packaging,
    /// `mjx-dml` — rank 2.0, the base of shared markup: every other markup crate may reach it.
    SharedMarkupBase,
    /// `mjx-sml` — rank 2.1. SpreadsheetML is shared markup because an embedded workbook is
    /// SpreadsheetML inside a `.pptx` or a `.docx`; it sits above `mjx-dml` and below `mjx-chart`
    /// precisely so that MJXOFF-112's `mjx-chart -> mjx-sml` edge points down.
    SharedMarkupSpreadsheet,
    /// `mjx-chart`, `mjx-omml`, `mjx-vml` — rank 2.2.
    SharedMarkupUpper,
    /// `mjx-pptx`, `mjx-docx`, `mjx-xlsx` — rank 3.0.
    Formats,
    /// `mjx-ooxml` — rank 4.0.
    Facade,
    /// `bindings/*` — rank 5.0. Nothing may depend on a binding.
    Bindings,
    /// `mjx-fixtures`: the committed corpus, **no dependencies at all**, so `mjx-opc`'s own suites
    /// can reach it without an upward edge. Outside the shipped graph.
    TestCorpus,
    /// `mjx-schema-gate`: the shared ECMA-376 gate, a `dev-dependency` of the three format crates
    /// and of nothing else. Outside the shipped graph.
    TestGate,
    /// `xtask`: a host-only developer binary nothing depends on, so it may reach anything.
    Tooling,
}

/// A tier's position in the ladder, as `major.minor`. Ordered, and compared strictly.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Rank(u8, u8);

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

impl Tier {
    /// The tier's rank, or `None` for a tier that is outside the shipped graph and therefore has no
    /// position in it.
    fn rank(self) -> Option<Rank> {
        Some(match self {
            Self::FoundationsCore => Rank(0, 0),
            Self::FoundationsXml => Rank(0, 1),
            Self::Packaging => Rank(1, 0),
            Self::SharedMarkupBase => Rank(2, 0),
            Self::SharedMarkupSpreadsheet => Rank(2, 1),
            Self::SharedMarkupUpper => Rank(2, 2),
            Self::Formats => Rank(3, 0),
            Self::Facade => Rank(4, 0),
            Self::Bindings => Rank(5, 0),
            Self::TestCorpus | Self::TestGate | Self::Tooling => return None,
        })
    }

    /// How the tier is named in a failure message.
    fn label(self) -> &'static str {
        match self {
            Self::FoundationsCore => "foundations, core",
            Self::FoundationsXml => "foundations, XML",
            Self::Packaging => "packaging/compatibility",
            Self::SharedMarkupBase => "shared markup, base",
            Self::SharedMarkupSpreadsheet => "shared markup, spreadsheet",
            Self::SharedMarkupUpper => "shared markup, upper",
            Self::Formats => "formats",
            Self::Facade => "facade",
            Self::Bindings => "bindings",
            Self::TestCorpus => "test-only corpus (outside the shipped graph)",
            Self::TestGate => "test-only gate (outside the shipped graph)",
            Self::Tooling => "host-only tooling (outside the shipped graph)",
        }
    }

    /// How a tier is written in a message: `2.1 (shared markup, spreadsheet)`.
    fn describe(self) -> String {
        match self.rank() {
            Some(rank) => format!("{rank} ({})", self.label()),
            None => self.label().to_owned(),
        }
    }
}

/// Every workspace member and the tier it belongs to. Mirrored in `CLAUDE.md`; the two are kept in
/// step by [`every_workspace_member_has_a_declared_tier`], which fails on a member missing here and
/// on an entry naming no member.
const TIERS: &[(&str, Tier)] = &[
    ("mjx-ooxml-core", Tier::FoundationsCore),
    ("mjx-derive", Tier::FoundationsCore),
    ("mjx-xml", Tier::FoundationsXml),
    ("mjx-ooxml-types", Tier::Packaging),
    ("mjx-opc", Tier::Packaging),
    ("mjx-mce", Tier::Packaging),
    ("mjx-dml", Tier::SharedMarkupBase),
    ("mjx-sml", Tier::SharedMarkupSpreadsheet),
    ("mjx-chart", Tier::SharedMarkupUpper),
    ("mjx-omml", Tier::SharedMarkupUpper),
    ("mjx-vml", Tier::SharedMarkupUpper),
    ("mjx-pptx", Tier::Formats),
    ("mjx-docx", Tier::Formats),
    ("mjx-xlsx", Tier::Formats),
    ("mjx-ooxml", Tier::Facade),
    ("mjx-python", Tier::Bindings),
    ("mjx-wasm", Tier::Bindings),
    ("mjx-fixtures", Tier::TestCorpus),
    ("mjx-schema-gate", Tier::TestGate),
    ("xtask", Tier::Tooling),
];

/// Which dependency section an edge was declared in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    /// `[dependencies]` — a link-time edge in the shipped artifact.
    Normal,
    /// `[dev-dependencies]` — tests, examples and benches only.
    Development,
    /// `[build-dependencies]` — a build script's own edge; as binding as a normal one.
    Build,
}

impl Kind {
    fn describe(self) -> &'static str {
        match self {
            Self::Normal => "a dependency",
            Self::Development => "a dev-dependency",
            Self::Build => "a build-dependency",
        }
    }
}

/// One workspace member, as Cargo reports it.
struct Member {
    name: String,
    /// Its dependencies on *other workspace members*, with the section each was declared in.
    /// External crates are dropped: the layering rule is about this workspace's own graph.
    edges: Vec<(String, Kind)>,
    /// How many dependencies it declares in total, external ones included. Only
    /// [`Tier::TestCorpus`]'s "no dependencies at all" rule needs this.
    declared_dependencies: usize,
}

/// The tier declared for `crate_name`, or `None` when the table does not name it.
fn tier_of(crate_name: &str) -> Option<Tier> {
    TIERS
        .iter()
        .find(|(name, _)| *name == crate_name)
        .map(|(_, tier)| *tier)
}

/// The workspace's members and their declared dependencies, straight out of Cargo.
fn workspace() -> Vec<Member> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("running `cargo metadata`");
    assert!(
        output.status.success(),
        "`cargo metadata` failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).expect("`cargo metadata` emits UTF-8");
    let root = json::parse(&text).expect("`cargo metadata` emits JSON");

    let names: Vec<String> = TIERS.iter().map(|(name, _)| (*name).to_owned()).collect();
    let packages = root
        .get("packages")
        .and_then(json::Value::array)
        .expect("`cargo metadata` reports a `packages` array");
    packages
        .iter()
        .map(|package| {
            let name = package
                .get("name")
                .and_then(json::Value::string)
                .expect("every package has a name")
                .to_owned();
            let dependencies = package
                .get("dependencies")
                .and_then(json::Value::array)
                .expect("every package has a dependency array");
            let edges = dependencies
                .iter()
                .filter_map(|dependency| {
                    let target = dependency.get("name").and_then(json::Value::string)?;
                    if !names.iter().any(|known| known == target) {
                        return None;
                    }
                    // `kind` is absent or null for a normal dependency, and the string "dev" or
                    // "build" otherwise. An unknown spelling is a hard failure rather than a
                    // silently dropped edge.
                    let kind = match dependency.get("kind").and_then(json::Value::string) {
                        None => Kind::Normal,
                        Some("dev") => Kind::Development,
                        Some("build") => Kind::Build,
                        Some(other) => panic!("`cargo metadata` reported an unknown dependency kind `{other}` on {name} -> {target}"),
                    };
                    Some((target.to_owned(), kind))
                })
                .collect();
            Member {
                name,
                edges,
                declared_dependencies: dependencies.len(),
            }
        })
        .collect()
}

#[test]
fn every_workspace_member_has_a_declared_tier() {
    let members = workspace();
    assert!(
        members.len() > 10,
        "`cargo metadata` reported {} members, which cannot be this workspace",
        members.len()
    );

    for member in &members {
        assert!(
            tier_of(&member.name).is_some(),
            "`{}` is a workspace member with no row in this file's tier table. A new crate has to \
             be given a rank before anything can check its edges — add it to `TIERS` here and to \
             `CLAUDE.md`'s layer list, which is the same table in prose.",
            member.name
        );
    }

    for (name, tier) in TIERS {
        assert!(
            members.iter().any(|member| member.name == *name),
            "the tier table declares `{name}` at {}, but the workspace has no such member — the \
             table has drifted from `Cargo.toml`",
            tier.describe()
        );
    }
}

#[test]
fn every_dependency_points_strictly_downward() {
    let members = workspace();
    let mut checked = 0usize;
    let mut per_tier: BTreeMap<&str, usize> = BTreeMap::new();

    for member in &members {
        let Some(tier) = tier_of(&member.name) else {
            continue;
        };
        // A crate outside the shipped graph has no rank, so its own edges are governed by
        // `the_test_only_crates_and_the_tooling_stay_outside_the_shipped_graph` instead.
        let Some(rank) = tier.rank() else { continue };

        for (target, kind) in &member.edges {
            // Dev-dependencies are deliberately *not* rank-checked. They are not a link-time edge
            // and they legitimately point the other way: `mjx-derive` (0.0) dev-depends on
            // `mjx-ooxml-types` (1.0) to test the code its macros expand to, and every format crate
            // dev-depends on the gate. What a dev-dependency may *not* do is reach a binding or the
            // tooling, which the next case covers.
            if *kind == Kind::Development {
                continue;
            }
            let target_tier =
                tier_of(target).expect("the target is a workspace member, so it has a row");
            let target_rank = target_tier.rank().unwrap_or_else(|| {
                panic!(
                    "`{}` ({}) declares `{target}` as {} — but `{target}` is {}, and a shipped \
                     crate may only reach it from a `[dev-dependencies]` section",
                    member.name,
                    tier.describe(),
                    kind.describe(),
                    target_tier.describe(),
                )
            });
            assert!(
                target_rank < rank,
                "layering violation: `{}` (rank {}) declares `{target}` (rank {}) as {}. An edge is \
                 legal only when it points to a *strictly* lower rank; this one points {}.",
                member.name,
                tier.describe(),
                target_tier.describe(),
                kind.describe(),
                if target_rank == rank {
                    "sideways"
                } else {
                    "upward"
                },
            );
            checked += 1;
            *per_tier.entry(tier.label()).or_default() += 1;
        }
    }

    // A tier table no edge exercises is satisfied by a graph that never had a violation. These
    // floors are not a guess about workspace size: they are what the shipped graph carries today,
    // and a change that empties one of them is a change worth failing on.
    assert!(
        checked >= 50,
        "only {checked} edges were checked, which is fewer than the shipped graph has — the walk \
         is not reaching the manifests"
    );
    for tier in [
        "foundations, XML",
        "packaging/compatibility",
        "shared markup, base",
        "shared markup, spreadsheet",
        "shared markup, upper",
        "formats",
        "facade",
        "bindings",
    ] {
        assert!(
            per_tier.get(tier).copied().unwrap_or_default() > 0,
            "not one edge out of the `{tier}` tier was checked; the rule is unexercised there"
        );
    }
    println!("layering: {checked} workspace edges checked, all downward: {per_tier:?}");
}

#[test]
fn nothing_depends_on_a_binding_or_on_the_tooling() {
    for member in &workspace() {
        for (target, kind) in &member.edges {
            let target_tier =
                tier_of(target).expect("the target is a workspace member, so it has a row");
            assert!(
                !matches!(target_tier, Tier::Bindings | Tier::Tooling),
                "`{}` declares `{target}` as {}, but `{target}` is {} — nothing may depend on it. \
                 A binding projects the facade and a developer binary is host-only; either one \
                 acquiring a consumer inverts the graph.",
                member.name,
                kind.describe(),
                target_tier.describe(),
            );
        }
    }
}

#[test]
fn the_test_only_crates_and_the_tooling_stay_outside_the_shipped_graph() {
    let members = workspace();
    let formats = Tier::Formats.rank().expect("formats are ranked");

    for member in &members {
        match tier_of(&member.name) {
            // "`mjx-fixtures` … with **no dependencies at all** so `mjx-opc`'s suites can reach it
            // without an upward edge" — `CLAUDE.md`. Stated over the total, external crates
            // included: a `serde` here would be as much of a problem as an `mjx-opc`.
            Some(Tier::TestCorpus) => assert_eq!(
                member.declared_dependencies,
                0,
                "`{}` is the committed corpus and must declare no dependencies at all, so that any \
                 crate's tests can reach it from anywhere in the graph; it declares {}",
                member.name,
                member.declared_dependencies
            ),
            // The gate is a `dev-dependency` of the format crates, so it must stay below them.
            Some(Tier::TestGate) => {
                for (target, kind) in &member.edges {
                    let target_tier =
                        tier_of(target).expect("the target is a workspace member, so it has a row");
                    let below_the_formats = match target_tier.rank() {
                        Some(rank) => rank < formats,
                        None => target_tier == Tier::TestCorpus,
                    };
                    assert!(
                        below_the_formats,
                        "`{}` is the shared gate, which every format crate dev-depends on, so it \
                         must stay below the format tier; it declares `{target}` ({}) as {}",
                        member.name,
                        target_tier.describe(),
                        kind.describe(),
                    );
                }
            }
            _ => {}
        }
    }

    // The other half of "outside the graph": no shipped crate may *ship* one of them.
    for member in &members {
        let Some(tier) = tier_of(&member.name) else {
            continue;
        };
        if tier.rank().is_none() {
            continue;
        }
        for (target, kind) in &member.edges {
            if *kind == Kind::Development {
                continue;
            }
            let target_tier =
                tier_of(target).expect("the target is a workspace member, so it has a row");
            assert!(
                !matches!(target_tier, Tier::TestCorpus | Tier::TestGate),
                "`{}` declares the test-only crate `{target}` as {} — it may only appear in a \
                 `[dev-dependencies]` section, or it stops being test-only",
                member.name,
                kind.describe(),
            );
        }
    }
}

/// Just enough JSON to read `cargo metadata`.
///
/// `xtask` carries no JSON dependency and this is the only place in the workspace that wants one, so
/// the reader is here rather than in the dependency graph. It is a complete value parser — not a
/// scan for the fields of interest — because a scanner that misreads a nested string is a scanner
/// that drops an edge, and dropping an edge is exactly how this gate would pass without doing
/// anything.
mod json {
    /// A parsed JSON value.
    ///
    /// Booleans and numbers are recognised and **discarded**: `cargo metadata` has plenty of both
    /// and this file reads none of them, so keeping their payloads would be a field nothing ever
    /// looks at. They still have to be *parsed*, because a value skipped rather than parsed is a
    /// value whose end is a guess.
    pub(crate) enum Value {
        Null,
        Bool,
        Number,
        String(String),
        Array(Vec<Value>),
        Object(Vec<(String, Value)>),
    }

    impl Value {
        /// The value at `key`, if this is an object that has one and it is not `null`.
        pub(crate) fn get(&self, key: &str) -> Option<&Value> {
            match self {
                Value::Object(members) => members
                    .iter()
                    .find(|(name, _)| name == key)
                    .map(|(_, value)| value)
                    .filter(|value| !matches!(value, Value::Null)),
                _ => None,
            }
        }

        /// This value's elements, if it is an array.
        pub(crate) fn array(&self) -> Option<&[Value]> {
            match self {
                Value::Array(items) => Some(items),
                _ => None,
            }
        }

        /// This value's text, if it is a string.
        pub(crate) fn string(&self) -> Option<&str> {
            match self {
                Value::String(text) => Some(text),
                _ => None,
            }
        }
    }

    /// Parses a whole JSON document, or reports the byte offset it gave up at.
    pub(crate) fn parse(text: &str) -> Result<Value, String> {
        let bytes = text.as_bytes();
        let mut at = 0;
        let value = value(bytes, &mut at)?;
        skip_whitespace(bytes, &mut at);
        if at != bytes.len() {
            return Err(format!("trailing input at byte {at}"));
        }
        Ok(value)
    }

    fn skip_whitespace(bytes: &[u8], at: &mut usize) {
        while *at < bytes.len() && matches!(bytes[*at], b' ' | b'\t' | b'\n' | b'\r') {
            *at += 1;
        }
    }

    fn expect(bytes: &[u8], at: &mut usize, byte: u8) -> Result<(), String> {
        if bytes.get(*at) == Some(&byte) {
            *at += 1;
            Ok(())
        } else {
            Err(format!(
                "expected `{}` at byte {at}",
                char::from(byte),
                at = *at
            ))
        }
    }

    fn value(bytes: &[u8], at: &mut usize) -> Result<Value, String> {
        skip_whitespace(bytes, at);
        match bytes.get(*at) {
            Some(b'{') => object(bytes, at),
            Some(b'[') => array(bytes, at),
            Some(b'"') => string(bytes, at).map(Value::String),
            Some(b't') => literal(bytes, at, "true").map(|()| Value::Bool),
            Some(b'f') => literal(bytes, at, "false").map(|()| Value::Bool),
            Some(b'n') => literal(bytes, at, "null").map(|()| Value::Null),
            Some(_) => number(bytes, at),
            None => Err("unexpected end of input".to_owned()),
        }
    }

    fn literal(bytes: &[u8], at: &mut usize, word: &str) -> Result<(), String> {
        if bytes[*at..].starts_with(word.as_bytes()) {
            *at += word.len();
            Ok(())
        } else {
            Err(format!("expected `{word}` at byte {at}", at = *at))
        }
    }

    fn number(bytes: &[u8], at: &mut usize) -> Result<Value, String> {
        let start = *at;
        while *at < bytes.len()
            && matches!(bytes[*at], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
        {
            *at += 1;
        }
        if start == *at {
            return Err(format!("expected a value at byte {start}"));
        }
        Ok(Value::Number)
    }

    fn string(bytes: &[u8], at: &mut usize) -> Result<String, String> {
        expect(bytes, at, b'"')?;
        let mut out = String::new();
        loop {
            let byte = *bytes
                .get(*at)
                .ok_or_else(|| "unterminated string".to_owned())?;
            *at += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let escape = *bytes
                        .get(*at)
                        .ok_or_else(|| "unterminated escape".to_owned())?;
                    *at += 1;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => out.push(unicode_escape(bytes, at)?),
                        other => {
                            return Err(format!("unknown escape `\\{}`", char::from(other)));
                        }
                    }
                }
                // A raw byte of a multi-byte UTF-8 sequence lands here too; pushing the bytes and
                // decoding at the end would be equivalent, but this keeps `out` a `String`
                // throughout. The input came from `String::from_utf8`, so the sequence is valid.
                _ => {
                    let start = *at - 1;
                    let width = utf8_width(byte);
                    *at = start + width;
                    let text = std::str::from_utf8(&bytes[start..*at])
                        .map_err(|error| error.to_string())?;
                    out.push_str(text);
                }
            }
        }
    }

    /// How many bytes the UTF-8 sequence starting with `lead` occupies.
    fn utf8_width(lead: u8) -> usize {
        match lead {
            0x00..=0x7f => 1,
            0xc0..=0xdf => 2,
            0xe0..=0xef => 3,
            _ => 4,
        }
    }

    /// A `\uXXXX` escape, with the surrogate pair a character outside the BMP is written as.
    fn unicode_escape(bytes: &[u8], at: &mut usize) -> Result<char, String> {
        let first = hex4(bytes, at)?;
        if (0xd800..0xdc00).contains(&first) {
            expect(bytes, at, b'\\')?;
            expect(bytes, at, b'u')?;
            let second = hex4(bytes, at)?;
            let combined =
                0x1_0000 + ((u32::from(first) - 0xd800) << 10) + (u32::from(second) - 0xdc00);
            return char::from_u32(combined).ok_or_else(|| "invalid surrogate pair".to_owned());
        }
        char::from_u32(u32::from(first)).ok_or_else(|| "invalid escape".to_owned())
    }

    fn hex4(bytes: &[u8], at: &mut usize) -> Result<u16, String> {
        let digits = bytes
            .get(*at..*at + 4)
            .ok_or_else(|| "truncated \\u escape".to_owned())?;
        *at += 4;
        let text = std::str::from_utf8(digits).map_err(|error| error.to_string())?;
        u16::from_str_radix(text, 16).map_err(|error| error.to_string())
    }

    fn array(bytes: &[u8], at: &mut usize) -> Result<Value, String> {
        expect(bytes, at, b'[')?;
        let mut items = Vec::new();
        skip_whitespace(bytes, at);
        if bytes.get(*at) == Some(&b']') {
            *at += 1;
            return Ok(Value::Array(items));
        }
        loop {
            items.push(value(bytes, at)?);
            skip_whitespace(bytes, at);
            match bytes.get(*at) {
                Some(b',') => *at += 1,
                Some(b']') => {
                    *at += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(format!("expected `,` or `]` at byte {at}", at = *at)),
            }
        }
    }

    fn object(bytes: &[u8], at: &mut usize) -> Result<Value, String> {
        expect(bytes, at, b'{')?;
        let mut members = Vec::new();
        skip_whitespace(bytes, at);
        if bytes.get(*at) == Some(&b'}') {
            *at += 1;
            return Ok(Value::Object(members));
        }
        loop {
            skip_whitespace(bytes, at);
            let key = string(bytes, at)?;
            skip_whitespace(bytes, at);
            expect(bytes, at, b':')?;
            members.push((key, value(bytes, at)?));
            skip_whitespace(bytes, at);
            match bytes.get(*at) {
                Some(b',') => *at += 1,
                Some(b'}') => {
                    *at += 1;
                    return Ok(Value::Object(members));
                }
                _ => return Err(format!("expected `,` or `}}` at byte {at}", at = *at)),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// The reader has to survive the shapes `cargo metadata` actually emits: nested objects and
        /// arrays, `null` (which is how a normal dependency's `kind` is spelled), escapes inside
        /// strings, and non-ASCII text. A reader that mis-tracked a string's end would find the
        /// wrong keys, so this is checked rather than assumed.
        #[test]
        fn the_reader_handles_the_shapes_cargo_emits() {
            let text = r#"{
                "packages": [
                    {"name": "a", "kind": null, "path": "C:\\x\\y", "note": "a \"quoted\" ünïcode ☃ \u2603 \ud83d\ude00"},
                    {"name": "b", "deps": [], "meta": {}, "n": -1.5e3, "ok": true}
                ]
            }"#;
            let root = parse(text).expect("parses");
            let packages = root.get("packages").and_then(Value::array).expect("array");
            assert_eq!(packages.len(), 2);
            assert_eq!(packages[0].get("name").and_then(Value::string), Some("a"));
            // `null` reads as absent, which is exactly how a normal dependency's `kind` is meant to
            // be understood.
            assert!(packages[0].get("kind").is_none());
            assert_eq!(
                packages[0].get("path").and_then(Value::string),
                Some(r"C:\x\y")
            );
            assert_eq!(
                packages[0].get("note").and_then(Value::string),
                Some("a \"quoted\" ünïcode ☃ ☃ 😀")
            );
            assert_eq!(packages[1].get("name").and_then(Value::string), Some("b"));
            assert_eq!(
                packages[1]
                    .get("deps")
                    .and_then(Value::array)
                    .map(<[_]>::len),
                Some(0)
            );
        }

        #[test]
        fn malformed_input_is_an_error_rather_than_a_wrong_answer() {
            for bad in [
                "{",
                "{\"a\"}",
                "[1,]",
                "\"unterminated",
                "{} trailing",
                "{\"a\": \\}",
            ] {
                assert!(parse(bad).is_err(), "`{bad}` should not parse");
            }
        }
    }
}
