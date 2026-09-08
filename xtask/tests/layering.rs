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
//!    on none — a vacuous run is a failure, not a green. It counts them **from both ends**, because
//!    the floor of the graph and the data crates declare no dependency at all and so can only ever
//!    be exercised as an edge's *target*; a list that only counted outgoing edges would leave
//!    `mjx-ooxml-core`, `mjx-derive` and `mjx-tokens` unchecked in a workspace that is exactly
//!    right, and would go on saying nothing if something later reached one of them upwards.
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
    /// `mjx-tokens` — rank 0.2 (MJXOFF-156). The generated design-token table and its runtime
    /// resolver. It is *data*: it declares no workspace dependency at all today, and its ceiling is
    /// `mjx-ooxml-core`. The rank is about who may reach **it** — the client platform's renderer
    /// crates, every one of which sits above the whole document graph, so a token table below
    /// `mjx-ooxml-types` is reachable from all of them without an upward edge.
    FoundationsTokens,
    /// `mjx-ooxml-types`, `mjx-opc`, `mjx-mce` — rank 1.0.
    Packaging,
    /// `mjx-text` — rank 1.5 (MJXOFF-157). Typography: face parsing and metrics, the system font
    /// database, the metric-compatible substitution table and the per-document substitution
    /// manifest. It sits *above* the packaging tier and *below* shared markup because it has never
    /// heard of OOXML — a document's font *reference* is `mjx-dml`'s model of `<a:latin>`, and a
    /// font *engine* is this, and the two meet above both. Its edges are what first exercise
    /// `mjx-tokens`'s tier.
    Typography,
    /// `mjx-layout` — rank 1.6 (MJXOFF-160). The box model contract: the `BoxModel` trait, the
    /// `FragmentTree` every box model produces, the checkpoint that makes flow layout resumable and
    /// the spatial index that makes a hit test a query. It sits one step above `mjx-text`, which it
    /// calls to measure text, and **below shared markup**, which is the whole point: `FragmentTree`
    /// is the seam above which nothing has heard of OOXML, so a crate that may not name a
    /// `.docx` must sit where it cannot reach one. Swapping the box model for a CSS or Markdown one
    /// is what this rank buys.
    BoxModel,
    /// `mjx-scene` — rank 1.7 (MJXOFF-161). The display list: the command vocabulary, the paint and
    /// effect vocabularies, the resource tables and the flat binary encoding a `FragmentTree`
    /// becomes. It sits one step above the box model contract and **below `mjx-dml`**, and that is
    /// the whole reason it has a rank at all.
    ///
    /// A reading of the stack would put it far higher — its consumers are painters, and
    /// `docs/UI_PLATFORM_PLAN.md` §7 first wrote it at 2.6. That number is wrong, and wrong in the
    /// way this file exists to prevent: this check only refuses an edge that points **up or
    /// sideways**, so a `mjx-scene` above shared markup makes `mjx-scene -> mjx-dml` a legal
    /// *downward* edge, and the guarantee the crate exists to hold — below a display list, nothing
    /// has heard of a font, a layout algorithm or a document — would be enforced by nothing at all.
    /// At 1.7 that edge is refused here by name, exactly as `mjx-layout`'s 1.6 refuses an edge to a
    /// format crate. Everything `mjx-scene` actually depends on is below 1.7, so the rank costs it
    /// nothing. **Do not raise it.**
    DisplayList,
    /// `mjx-dml` — rank 2.0, the base of shared markup: every other markup crate may reach it.
    SharedMarkupBase,
    /// `mjx-sml` — rank 2.1. SpreadsheetML is shared markup because an embedded workbook is
    /// SpreadsheetML inside a `.pptx` or a `.docx`; it sits above `mjx-dml` and below `mjx-chart`
    /// precisely so that MJXOFF-112's `mjx-chart -> mjx-sml` edge points down.
    SharedMarkupSpreadsheet,
    /// `mjx-chart`, `mjx-omml`, `mjx-vml` — rank 2.2.
    SharedMarkupUpper,
    /// `mjx-geometry` — rank 2.5 (MJXOFF-202). The preset shape path tables and the
    /// `GeometryProvider` that resolves them, which is what ends `mjx-scene`'s placeholder.
    ///
    /// Every other rank in this table is justified by what its crates may not *reach*. This one is
    /// justified by what may not reach **it**, and it makes three things impossible:
    ///
    /// * **`mjx-scene` (1.7) cannot depend on it.** A preset path table names `ST_ShapeType`, is
    ///   written in the guide-formula language and resolves through `mjx-dml`'s evaluator, so it
    ///   lives at or above 2.0 — and a display list that could read one would be a display list
    ///   that knows what a `.pptx` is. `mjx-scene` was put at 1.7 precisely so this check would
    ///   refuse `mjx-scene -> mjx-dml` by name; at 2.5 it refuses `mjx-scene -> mjx-geometry` for
    ///   the same arithmetic, which is what stops the provider being "just moved into `mjx-scene`"
    ///   the first time the seam is inconvenient.
    /// * **`mjx-layout` (1.6) cannot depend on it.** A box model issues a `GeometryRef` — a bare
    ///   number — because it must not know what the number means. An edge from 1.6 to 2.5 would let
    ///   it resolve its own handles and the seam would be decoration.
    /// * **`mjx-dml` (2.0) cannot depend on it**, which keeps the fidelity model free of a
    ///   rendering decision: how many cubics an `a:arcTo` becomes is a renderer's business, and
    ///   `mjx-dml` resolves an arc to numbers and stops.
    ///
    /// **What it deliberately does not buy.** It is *below* the format tier, so `mjx-pptx` (3.0)
    /// may legally depend on it — intended, because a format crate is allowed to know what its own
    /// shapes look like. And it is below `mjx-paint` (5.5), so a painter could legally declare the
    /// edge; that is the hole no rank can close at the top of the ladder, and it is closed the way
    /// the painter's other seam is, by name in `crates/mjx-paint/tests/the_seam_holds.rs`.
    ///
    /// **Which half of the rule actually catches which edge, measured rather than assumed.** The
    /// first two bullets above are true and this file is not what proves them *today*: because
    /// `mjx-geometry` depends on `mjx-scene`, which depends on `mjx-layout`, both
    /// `mjx-scene -> mjx-geometry` and `mjx-layout -> mjx-geometry` are **cycles**, and Cargo
    /// refuses them before a test binary is built — exactly the division of labour this file's own
    /// header describes. That is a stronger guarantee, not a weaker one, but it means the rank's
    /// own work is the *acyclic* illegal edges, and those were the mutations used to prove it:
    /// `mjx-sml -> mjx-geometry` (2.1 -> 2.5, upward, and not a cycle because nothing here reaches
    /// SpreadsheetML) and `mjx-geometry -> mjx-pptx` (2.5 -> 3.0, upward). Both went red naming
    /// both crates and both ranks. The rank is also what keeps the first two bullets true **if
    /// `mjx-geometry` ever stops depending on `mjx-scene`** — a provider that answered in its own
    /// vocabulary rather than in `ResolvedOutline` would do exactly that, and Cargo's cycle check
    /// would go quiet on the day the architecture needed it most.
    PresetGeometry,
    /// `mjx-pptx`, `mjx-docx`, `mjx-xlsx` — rank 3.0.
    Formats,
    /// `mjx-ooxml` — rank 4.0.
    Facade,
    /// `bindings/*` — rank 5.0. Nothing may depend on a binding.
    Bindings,
    /// `mjx-paint` — rank 5.5 (MJXOFF-163). **The platform boundary**: the `Painter` contract, the
    /// `SurfaceHost` contract and the `wgpu` painter.
    ///
    /// Its rank sits **above the facade**, and that is the whole of what the rank buys — it says
    /// who may reach *it*, and the answer is nothing in the document graph. No format crate, no
    /// `mjx-ooxml`, and above all no binding can declare an edge to a crate that links Vulkan,
    /// Metal or Direct3D; `bindings/mjx-python` must never grow a GPU dependency, and at 5.5 it
    /// structurally cannot. **Do not lower it**: below the format tier the formats and the facade
    /// would sit *above* it and could legally depend on it, which is the outcome this position
    /// exists to prevent.
    ///
    /// **What the rank does not buy is the other direction, and this is worth reading before
    /// relying on it.** This file refuses only an edge that points up or sideways, so at 5.5 every
    /// crate in the workspace is a legal dependency of `mjx-paint` — `mjx-dml`, the format crates,
    /// `mjx-text`, `mjx-layout`, all of them. The architecture's second seam (*below a display
    /// list, nothing has heard of a font, a layout algorithm or a document*) is therefore held for
    /// that crate by an explicit manifest gate, `crates/mjx-paint/tests/the_seam_holds.rs`, and by
    /// nothing else. `mjx-scene` got 1.7 so this file could refuse its illegal edge by name; **no
    /// rank can do the same job for a painter, in either direction.**
    PlatformBoundary,
    /// `mjx-fixtures`: the committed corpus, **no dependencies at all**, so `mjx-opc`'s own suites
    /// can reach it without an upward edge. Outside the shipped graph.
    TestCorpus,
    /// `mjx-schema-gate`: the shared ECMA-376 gate, a `dev-dependency` of the three format crates
    /// and of nothing else. Outside the shipped graph.
    TestGate,
    /// `mjx-allocation-counter`: the counting global allocator, **no dependencies at all** for the
    /// same reason `mjx-fixtures` has none — its two consumers sit in different tiers (`xtask`'s
    /// fuzz campaign and `mjx-sml`'s allocation gate) and nothing may depend on `xtask`. Outside
    /// the shipped graph.
    TestInstrument,
    /// `mjx-reference-pack` (MJXOFF-207): the artefacts one Windows sitting needs, and the harness
    /// that ingests what comes back. Outside the shipped graph, and outside it in the **opposite**
    /// direction from the three above.
    ///
    /// `mjx-fixtures` and `mjx-allocation-counter` have no rank because they must be reachable from
    /// *everywhere*, so they declare no dependencies at all. This one has no rank because it sits at
    /// the **top**: it names the format tier (to author a `.pptx` and a `.docx`), `mjx-geometry` (to
    /// know what a preset shape is) and `mjx-paint` (to export and rasterise our own side of a
    /// comparison), which is a set of edges no shipped crate could legally declare together —
    /// `mjx-paint` is rank 5.5 and `mjx-pptx` is 3.0, so a crate depending on both would have to be
    /// above 5.5, and above 5.5 is where nothing in the document graph may go.
    ///
    /// **Giving it a rank of 6.0 would have been wrong**, and worth saying why: a rank is a promise
    /// about who may reach *it*, and the answer for this crate is *nobody, ever*. That is stronger
    /// than any rank can express and it is enforced directly, by
    /// [`the_test_only_crates_and_the_tooling_stay_outside_the_shipped_graph`], which refuses the
    /// edge from any ranked crate in either dependency section — a stricter rule than the one the
    /// other three test-only crates live under, since those are legitimately `dev-dependencies` of
    /// shipped crates and this is a dependency of nothing at all.
    ReferencePack,
    /// `mjx-render-oracle` (MJXOFF-165): the fidelity oracle — the three assertion tiers, the
    /// perceptual metric, the committed baselines and their approval events, and the plate gallery.
    /// Outside the shipped graph, at the top, one step **below** [`Tier::ReferencePack`].
    ///
    /// It is a rung of its own rather than a second `ReferencePack`, because the two live under
    /// different rules and the difference is the reason the crate was split out at all. The pack
    /// names the format tier — it authors a `.pptx` and a `.docx` — and **nothing may depend on
    /// it**. The oracle names no format crate: it is `FragmentTree`, `DisplayList`, the geometry
    /// provider and the painters, which is the rendering path with no document anywhere in it. That
    /// is what lets exactly one crate depend on it.
    ///
    /// **No crate with a rank may reach it, in either section**, and
    /// [`the_test_only_crates_and_the_tooling_stay_outside_the_shipped_graph`] is what refuses the
    /// edge — including, deliberately, one from a ranked crate's `[dev-dependencies]`, which is the
    /// section the other three test-only crates legitimately live in. A shipped crate that
    /// dev-depended on this would pull `mjx-paint`, and therefore a graphics stack, into its test
    /// build; a test build that links Vulkan is still a build that links Vulkan.
    ///
    /// **What it may have is a consumer above the graph, and that is the point.** MJXOFF-165 needed
    /// the authority vocabulary MJXOFF-207 had already written — `ReferenceProvider`, the
    /// three-state `Verdict`, the provider-attached exclusions — and the rule against a second
    /// answer to *"how much is this reference worth"* is the same rule that put `ReferenceAuthority`
    /// in `mjx-text` rather than in two crates. Since nothing may depend on the pack, the vocabulary
    /// moved **down** into the oracle and the pack re-exports it. MJXOFF-166's canvas harness is
    /// specified to reach the plate generator here rather than write a second PNG emitter, and it
    /// will be the second such consumer; a rule that named `mjx-reference-pack` and nothing else
    /// would have made that child amend this file before it could start.
    ///
    /// **Giving it a rank would have been wrong**, for the reason the pack's own comment gives: a
    /// rank is a promise about who may reach it, and the answer here is *one named crate*, which is
    /// not something a number can say.
    RenderOracle,
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
            Self::FoundationsTokens => Rank(0, 2),
            Self::Packaging => Rank(1, 0),
            Self::Typography => Rank(1, 5),
            Self::BoxModel => Rank(1, 6),
            Self::DisplayList => Rank(1, 7),
            Self::SharedMarkupBase => Rank(2, 0),
            Self::SharedMarkupSpreadsheet => Rank(2, 1),
            Self::SharedMarkupUpper => Rank(2, 2),
            Self::PresetGeometry => Rank(2, 5),
            Self::Formats => Rank(3, 0),
            Self::Facade => Rank(4, 0),
            Self::Bindings => Rank(5, 0),
            Self::PlatformBoundary => Rank(5, 5),
            Self::TestCorpus
            | Self::TestGate
            | Self::TestInstrument
            | Self::ReferencePack
            | Self::RenderOracle
            | Self::Tooling => return None,
        })
    }

    /// How the tier is named in a failure message.
    fn label(self) -> &'static str {
        match self {
            Self::FoundationsCore => "foundations, core",
            Self::FoundationsXml => "foundations, XML",
            Self::FoundationsTokens => "foundations, design tokens",
            Self::Packaging => "packaging/compatibility",
            Self::Typography => "typography",
            Self::BoxModel => "box model",
            Self::DisplayList => "display list",
            Self::SharedMarkupBase => "shared markup, base",
            Self::SharedMarkupSpreadsheet => "shared markup, spreadsheet",
            Self::SharedMarkupUpper => "shared markup, upper",
            Self::PresetGeometry => "preset geometry",
            Self::Formats => "formats",
            Self::Facade => "facade",
            Self::Bindings => "bindings",
            Self::PlatformBoundary => "platform boundary",
            Self::TestCorpus => "test-only corpus (outside the shipped graph)",
            Self::TestGate => "test-only gate (outside the shipped graph)",
            Self::TestInstrument => "test-only instrument (outside the shipped graph)",
            Self::ReferencePack => "test-only reference pack (above the shipped graph)",
            Self::RenderOracle => "test-only fidelity oracle (above the shipped graph)",
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
    ("mjx-tokens", Tier::FoundationsTokens),
    ("mjx-ooxml-types", Tier::Packaging),
    ("mjx-opc", Tier::Packaging),
    ("mjx-mce", Tier::Packaging),
    ("mjx-text", Tier::Typography),
    ("mjx-layout", Tier::BoxModel),
    ("mjx-scene", Tier::DisplayList),
    ("mjx-dml", Tier::SharedMarkupBase),
    ("mjx-sml", Tier::SharedMarkupSpreadsheet),
    ("mjx-chart", Tier::SharedMarkupUpper),
    ("mjx-omml", Tier::SharedMarkupUpper),
    ("mjx-vml", Tier::SharedMarkupUpper),
    ("mjx-geometry", Tier::PresetGeometry),
    ("mjx-pptx", Tier::Formats),
    ("mjx-docx", Tier::Formats),
    ("mjx-xlsx", Tier::Formats),
    ("mjx-ooxml", Tier::Facade),
    ("mjx-python", Tier::Bindings),
    ("mjx-wasm", Tier::Bindings),
    ("mjx-paint", Tier::PlatformBoundary),
    ("mjx-fixtures", Tier::TestCorpus),
    ("mjx-schema-gate", Tier::TestGate),
    ("mjx-allocation-counter", Tier::TestInstrument),
    ("mjx-render-oracle", Tier::RenderOracle),
    ("mjx-reference-pack", Tier::ReferencePack),
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
    let mut per_source_tier: BTreeMap<&str, usize> = BTreeMap::new();
    let mut per_target_tier: BTreeMap<&str, usize> = BTreeMap::new();

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
            *per_source_tier.entry(tier.label()).or_default() += 1;
            *per_target_tier.entry(target_tier.label()).or_default() += 1;
        }
    }

    // A tier table no edge exercises is satisfied by a graph that never had a violation. These
    // floors are not a guess about workspace size: they are what the shipped graph carries today,
    // and a change that empties one of them is a change worth failing on.
    //
    // The check has **two** halves, because a tier can be exercised from either end and not every
    // tier can be exercised from both.
    //
    // * `foundations, core` and `foundations, design tokens` declare no workspace dependency at all
    //   — they are the floor and the data crate — so no edge ever *leaves* them, and listing them in
    //   the first list would fail on a workspace that is exactly right. What can be asserted about
    //   them is that something reaches them, which is the second list's job. MJXOFF-156 left a note
    //   asking MJXOFF-157 to add `foundations, design tokens` to the exercised-tier list;
    //   `mjx-text -> mjx-tokens` is that edge, and the second list is the one it belongs in.
    // * `bindings` and `platform boundary` sit at the top and nothing may reach them — the former
    //   by `nothing_depends_on_a_binding_or_on_the_tooling`, the latter because every crate that
    //   would is in the document graph and must never link a GPU — so they appear only in the first
    //   list.
    // * `preset geometry` is in the first list only, and for a reason that will expire: MJXOFF-202
    //   creates `mjx-geometry` and **nothing depends on it yet**, because its consumer is the
    //   application in the second loop. Adding it to the incoming list today would fail on a
    //   workspace that is exactly right, which is the same reason MJXOFF-161 left `display list`
    //   out of that list and MJXOFF-163 put it in. The child that gives it a consumer adds it.
    // * Everything between is in both. `display list` was added to the *incoming* list by
    //   MJXOFF-163, which is the first child to depend on `mjx-scene`: MJXOFF-161 deliberately left
    //   it out because nothing depended on the crate yet and the assertion would have failed, and
    //   MJXOFF-162 was the same crate. `box model` was added by MJXOFF-161 for the same reason one
    //   child earlier.
    assert!(
        checked >= 50,
        "only {checked} edges were checked, which is fewer than the shipped graph has — the walk \
         is not reaching the manifests"
    );
    for tier in [
        "foundations, XML",
        "packaging/compatibility",
        "typography",
        "box model",
        "display list",
        "shared markup, base",
        "shared markup, spreadsheet",
        "shared markup, upper",
        "preset geometry",
        "formats",
        "facade",
        "bindings",
        "platform boundary",
    ] {
        assert!(
            per_source_tier.get(tier).copied().unwrap_or_default() > 0,
            "not one edge out of the `{tier}` tier was checked; the rule is unexercised there"
        );
    }
    for tier in [
        "foundations, core",
        "foundations, XML",
        "foundations, design tokens",
        "packaging/compatibility",
        "typography",
        "box model",
        "display list",
        "shared markup, base",
        "shared markup, spreadsheet",
        "shared markup, upper",
        "formats",
    ] {
        assert!(
            per_target_tier.get(tier).copied().unwrap_or_default() > 0,
            "not one edge *into* the `{tier}` tier was checked; nothing in the workspace reaches \
             it, so its rank constrains nothing"
        );
    }
    println!(
        "layering: {checked} workspace edges checked, all downward. Out of: {per_source_tier:?}. \
         Into: {per_target_tier:?}."
    );
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
            // The allocator instrument carries the same rule, and for the same reason: `xtask`
            // reaches it from outside the graph and `mjx-sml` from rank 2.1, so it must be
            // reachable from both without an edge of its own. Anything it depended on would also
            // be allocating inside the process whose allocations it counts.
            Some(Tier::TestCorpus | Tier::TestInstrument) => assert_eq!(
                member.declared_dependencies,
                0,
                "`{}` is outside the shipped graph and must declare no dependencies at all, so that \
                 any crate's tests can reach it from anywhere in the graph; it declares {}",
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

    // **Nothing at all may depend on the reference pack, in either section.** It sits above every
    // ranked crate — it names `mjx-pptx` (3.0) and `mjx-paint` (5.5) together, which no shipped
    // crate could legally do — so an edge into it would drag the platform boundary into whatever
    // declared it. This is stricter than the rule the other three test-only crates live under,
    // because those are legitimately `dev-dependencies` of shipped crates and this is a dependency
    // of nothing.
    for member in &members {
        for (target, kind) in &member.edges {
            let target_tier =
                tier_of(target).expect("the target is a workspace member, so it has a row");
            assert!(
                target_tier != Tier::ReferencePack,
                "`{}` declares `{target}` as {}, but `{target}` is {} and nothing may depend on it \
                 in any section: it is the top of the workspace, and an edge into it would pull the \
                 format tier and the platform boundary into whatever declared it",
                member.name,
                kind.describe(),
                target_tier.describe(),
            );
        }
    }

    // **No *ranked* crate may reach the fidelity oracle, in either dependency section.** The rule is
    // stricter than the one the corpus, the gate and the instrument live under — those are
    // legitimately `dev-dependencies` of shipped crates — because this crate depends on `mjx-paint`
    // at rank 5.5, so any edge into it drags a graphics stack into whatever declared it. A
    // `[dev-dependencies]` entry is no exemption: a test build that links Vulkan is still a test
    // build that links Vulkan.
    //
    // It is *looser* than the reference pack's rule, which admits no consumer at all, and the
    // looseness is deliberate rather than an oversight. **The oracle is meant to be consumed by the
    // crates above the graph**: `mjx-reference-pack` needs the authority vocabulary it owns — a
    // second copy of *"how much is this reference worth"* in one workspace would be one answer too
    // many — and MJXOFF-166's canvas harness is specified to reach its plate generator rather than
    // write a second PNG emitter. A rule that named `mjx-reference-pack` and nothing else would
    // force that child to re-litigate this file before it could start, which is exactly the shape
    // of hand-off this programme is trying not to leave.
    //
    // What the rule is actually about is therefore stated as what it is about: **a rank**. A crate
    // with one is in the document graph and may not link a graphics stack; a crate without one is
    // already above the whole graph and may.
    for member in &members {
        let source_has_a_rank = tier_of(&member.name).is_some_and(|tier| tier.rank().is_some());
        for (target, kind) in &member.edges {
            let target_tier =
                tier_of(target).expect("the target is a workspace member, so it has a row");
            if target_tier != Tier::RenderOracle {
                continue;
            }
            assert!(
                !source_has_a_rank,
                "`{}` declares `{target}` as {}, but `{target}` is {} and no crate with a rank may \
                 reach it in any section — a `[dev-dependencies]` entry included. It depends on \
                 `mjx-paint` at rank 5.5, so the edge would pull a graphics stack into `{}`'s own \
                 build. Only a crate that is itself above the whole document graph may consume it.",
                member.name,
                kind.describe(),
                target_tier.describe(),
                member.name,
            );
        }
    }

    // And the edge that must **exist**, so the exception above is not a hole nothing exercises. A
    // rule written for one consumer, with no consumer, is a rule that would go on passing if the
    // crate it governs were deleted.
    let pack = members
        .iter()
        .find(|member| member.name == "mjx-reference-pack")
        .expect("the workspace has a reference pack");
    assert!(
        pack.edges
            .iter()
            .any(|(target, _)| target == "mjx-render-oracle"),
        "`mjx-reference-pack` no longer depends on `mjx-render-oracle`, so the one exception above \
         is unexercised — and an unexercised exception is one nobody would notice going wrong"
    );

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
                !matches!(
                    target_tier,
                    Tier::TestCorpus | Tier::TestGate | Tier::TestInstrument
                ),
                "`{}` declares the test-only crate `{target}` as {} — it may only appear in a \
                 `[dev-dependencies]` section, or it stops being test-only",
                member.name,
                kind.describe(),
            );
        }
    }
}

/// Just enough JSON to read `cargo metadata`, shared with the design-token generator.
///
/// `xtask` carries no JSON dependency and nothing in the shipped graph wants one, so the reader
/// lives at `xtask/src/json.rs` rather than in the dependency graph. It used to be a private module
/// *here*, because this gate was its only consumer; MJXOFF-156's token generator became a second
/// one, and an integration test cannot reach a binary crate's private modules — so the one file is
/// pulled in by path rather than copied. A workspace with two JSON readers in it has one reader too
/// many, and the copy that is not exercised is the one that is wrong.
#[path = "../src/json.rs"]
mod json;
