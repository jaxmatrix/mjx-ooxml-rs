//! MJXOFF-118 (E6) — "Done when" #5: **the shared-markup reachability table exists, every asymmetry
//! carries a written reason, and a test fails if the code and the table disagree.**
//!
//! The table is `crates/mjx-ooxml/docs/shared_markup_reachability.md`, rendered as
//! [`mjx_ooxml::shared_markup_reachability`]. This file re-derives both of its tables from the
//! workspace itself and compares them to what is written there.
//!
//! # Why a *derivation* rather than a second hand-written list
//!
//! A hand-written list of "which format reaches what" is the shape of claim this programme keeps
//! finding stale: it is written once, it is true once, and nothing tells anyone when it stops being
//! true. Every fact below is read out of a file that a change to the code has to touch anyway —
//! three `Cargo.toml`s and the facade's own `src/` — so the page cannot drift without this test
//! saying which row drifted.
//!
//! # What the derivation is, exactly
//!
//! 1. `src/lib.rs`'s `pub use mjx_dml::{…}` / `mjx_sml` / `mjx_chart` blocks are the shared-markup
//!    **type set**. `mjx_vml` and `mjx_omml` have no such block, and that absence is asserted.
//! 2. Every `pub fn` in the three surfaces' modules is read with its signature.
//! 3. A method is a **capability** when its signature names one of those types (`✓`); it is `≈` when
//!    the surface has a method of that name whose signature names none of them, and `—` when the
//!    surface has no such method at all.
//!
//! # The trap this file is written against
//!
//! A gate phrased "the table is covered and green" is green precisely when the derivation finds
//! nothing. So the first assertion is that the derivation found a *lot* — the counts are pinned, and
//! a parser that silently stopped matching `pub fn` (a rustfmt change, a moved module, a renamed
//! directory) fails here rather than passing an empty comparison.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// `crates/mjx-ooxml`.
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The workspace root — `crates/mjx-ooxml/../..`.
fn workspace_dir() -> PathBuf {
    crate_dir()
        .parent()
        .and_then(Path::parent)
        .expect("crates/mjx-ooxml sits two levels below the workspace root")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

// -------------------------------------------------------------------------------------------
// The page
// -------------------------------------------------------------------------------------------

fn page() -> String {
    read(&crate_dir().join("docs/shared_markup_reachability.md"))
}

/// The cells of a markdown table row, trimmed, or `None` for a line that is not one.
fn row_cells(line: &str) -> Option<Vec<&str>> {
    let line = line.trim();
    let inner = line.strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(str::trim).collect())
}

/// The text between the first pair of backticks, if the cell is exactly one code span.
fn code_span(cell: &str) -> Option<&str> {
    cell.strip_prefix('`')?.strip_suffix('`')
}

// -------------------------------------------------------------------------------------------
// Table 1 — which format crate models which shared markup
// -------------------------------------------------------------------------------------------

const SHARED_CRATES: [&str; 5] = ["mjx-dml", "mjx-sml", "mjx-chart", "mjx-vml", "mjx-omml"];
const FORMAT_CRATES: [&str; 3] = ["mjx-pptx", "mjx-docx", "mjx-xlsx"];

/// The shared-markup crates a format crate declares as a dependency, read from its `Cargo.toml`.
///
/// Both spellings the workspace uses are accepted — `mjx-dml.workspace = true` and
/// `mjx-vml = { workspace = true, optional = true }` — and a commented-out line is not one, which is
/// why the prefix is matched on a *trimmed* line rather than searched for anywhere in the file.
/// `mjx-pptx`'s `Cargo.toml` mentions `mjx-vml` five times in prose; only one of those is the
/// dependency.
fn modelled_shared_markup(format_crate: &str) -> BTreeSet<String> {
    let manifest = read(
        &workspace_dir()
            .join("crates")
            .join(format_crate)
            .join("Cargo.toml"),
    );
    let mut found = BTreeSet::new();
    for line in manifest.lines() {
        let line = line.trim();
        for shared in SHARED_CRATES {
            if line.starts_with(&format!("{shared}.workspace"))
                || line.starts_with(&format!("{shared} ="))
            {
                found.insert(shared.to_owned());
            }
        }
    }
    found
}

/// Table 1, as the page states it: format crate -> the shared crates its row ticks.
fn documented_dependency_grid(page: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut grid = BTreeMap::new();
    for line in page.lines() {
        let Some(cells) = row_cells(line) else {
            continue;
        };
        if cells.len() != SHARED_CRATES.len() + 1 {
            continue;
        }
        let Some(format_crate) = code_span(cells[0]) else {
            continue;
        };
        if !FORMAT_CRATES.contains(&format_crate) {
            continue;
        }
        let ticked = SHARED_CRATES
            .iter()
            .zip(&cells[1..])
            .filter(|(_, cell)| **cell == "✓")
            .map(|(shared, _)| (*shared).to_owned())
            .collect();
        grid.insert(format_crate.to_owned(), ticked);
    }
    grid
}

#[test]
fn the_dependency_grid_matches_the_three_format_crates_manifests() {
    let page = page();
    let documented = documented_dependency_grid(&page);
    assert_eq!(
        documented.len(),
        FORMAT_CRATES.len(),
        "table 1 of the page must carry one row per format crate; parsed {documented:?}"
    );
    for format_crate in FORMAT_CRATES {
        let derived = modelled_shared_markup(format_crate);
        assert!(
            !derived.is_empty(),
            "{format_crate} declares no shared-markup dependency at all — the manifest parser has \
             stopped matching, not the workspace"
        );
        assert_eq!(
            documented.get(format_crate),
            Some(&derived),
            "table 1's `{format_crate}` row disagrees with crates/{format_crate}/Cargo.toml"
        );
    }
}

// -------------------------------------------------------------------------------------------
// The shared-markup type set, out of the facade's own re-exports
// -------------------------------------------------------------------------------------------

/// Every name a `pub use <crate>::{…};` block re-exports, resolving `X as Y` to `Y`.
fn reexported_types(lib: &str, crate_ident: &str) -> BTreeSet<String> {
    let needle = format!("pub use {crate_ident}::{{");
    let Some(start) = lib.find(&needle) else {
        return BTreeSet::new();
    };
    let body_start = start + needle.len();
    let body_end = body_start
        + lib[body_start..]
            .find("};")
            .expect("a re-export block is closed by `};`");
    let mut names = BTreeSet::new();
    for token in lib[body_start..body_end].split(',') {
        // A `//` comment runs to the end of its line, and a block may carry several.
        let cleaned: String = token
            .lines()
            .map(|line| line.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join(" ");
        let words: Vec<&str> = cleaned.split_whitespace().collect();
        let name = match words.as_slice() {
            [name] => *name,
            [_, "as", alias] => *alias,
            _ => continue,
        };
        if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        {
            names.insert(name.to_owned());
        }
    }
    names
}

/// Type name -> the shared crate it belongs to.
fn shared_markup_types(lib: &str) -> BTreeMap<String, &'static str> {
    let mut owner = BTreeMap::new();
    for crate_ident in ["mjx_dml", "mjx_sml", "mjx_chart", "mjx_vml", "mjx_omml"] {
        let short: &'static str = match crate_ident {
            "mjx_dml" => "mjx-dml",
            "mjx_sml" => "mjx-sml",
            "mjx_chart" => "mjx-chart",
            "mjx_vml" => "mjx-vml",
            _ => "mjx-omml",
        };
        for name in reexported_types(lib, crate_ident) {
            owner.entry(name).or_insert(short);
        }
    }
    owner
}

// -------------------------------------------------------------------------------------------
// The three surfaces
// -------------------------------------------------------------------------------------------

/// Every `pub fn` of one surface, as `name -> the signature text` (`pub fn …` up to the body's
/// opening brace, so the return type is included).
///
/// Only items indented exactly one level are read, which is what a method of an `impl` block on this
/// surface looks like; a nested `pub fn` inside another item is deeper and a free function is not
/// indented at all.
fn surface_signatures(module: &str) -> BTreeMap<String, Vec<String>> {
    let mut files = vec![crate_dir().join(format!("src/{module}.rs"))];
    let mut nested: Vec<PathBuf> = std::fs::read_dir(crate_dir().join("src").join(module))
        .unwrap_or_else(|error| panic!("src/{module}: {error}"))
        .map(|entry| entry.expect("a readable directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    nested.sort();
    assert!(
        !nested.is_empty(),
        "src/{module}/ holds no .rs files — the module layout moved"
    );
    files.append(&mut nested);

    let mut signatures: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in files {
        let text = read(&file);
        let bytes: Vec<char> = text.chars().collect();
        let mut rest = text.as_str();
        let mut base = 0usize;
        while let Some(offset) = rest.find("\n    pub fn ") {
            let start = base + offset + 1; // the `p` of `pub`
            let after = start + "    pub fn ".len();
            let name: String = text[after..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            // Walk to the `)` that closes the parameter list, then on to the body's `{`.
            let mut depth = 0i32;
            let mut index = text[after + name.len()..]
                .char_indices()
                .find(|(_, c)| *c == '(')
                .map_or(after + name.len(), |(i, _)| after + name.len() + i);
            let chars_before = text[..index].chars().count();
            let mut position = chars_before;
            while position < bytes.len() {
                let c = bytes[position];
                if matches!(c, '(' | '[' | '<') {
                    depth += 1;
                } else if matches!(c, ')' | ']' | '>') {
                    depth -= 1;
                }
                if c == ')' && depth == 0 {
                    break;
                }
                position += 1;
            }
            index = text
                .char_indices()
                .nth(position)
                .map_or(text.len(), |(i, _)| i);
            let brace = text[index..].find('{').map_or(text.len(), |i| index + i);
            if !name.is_empty() {
                signatures
                    .entry(name)
                    .or_default()
                    .push(text[start..brace].to_owned());
            }
            base = after;
            rest = &text[base..];
        }
    }
    signatures
}

/// `✓` / `≈` / `—` for one capability on one surface.
fn mark(
    signatures: &BTreeMap<String, Vec<String>>,
    types: &BTreeMap<String, &str>,
    name: &str,
) -> (char, BTreeSet<String>) {
    let Some(all) = signatures.get(name) else {
        return ('—', BTreeSet::new());
    };
    let mut crates = BTreeSet::new();
    for signature in all {
        for (type_name, owner) in types {
            if names_type(signature, type_name) {
                crates.insert((*owner).to_owned());
            }
        }
    }
    if crates.is_empty() {
        ('≈', crates)
    } else {
        ('✓', crates)
    }
}

/// Whether `signature` names `type_name` as a whole word.
fn names_type(signature: &str, type_name: &str) -> bool {
    let mut from = 0;
    while let Some(offset) = signature[from..].find(type_name) {
        let at = from + offset;
        let before_ok = at == 0
            || !signature[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        let after = at + type_name.len();
        let after_ok = !signature[after..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        if before_ok && after_ok {
            return true;
        }
        from = at + 1;
    }
    false
}

/// One row of table 2, as derived from the source.
#[derive(Debug, PartialEq, Eq)]
struct Row {
    crates: String,
    marks: [char; 3],
}

fn derived_rows() -> BTreeMap<String, Row> {
    let lib = read(&crate_dir().join("src/lib.rs"));
    let types = shared_markup_types(&lib);
    assert!(
        types.len() > 100,
        "the facade re-exports {} shared-markup types, which is too few to be the real list — the \
         `pub use` parser has stopped matching",
        types.len()
    );
    for absent in ["mjx_vml", "mjx_omml"] {
        assert!(
            reexported_types(&lib, absent).is_empty(),
            "{absent} now has a `pub use` block in src/lib.rs — the `vml-omml` note on the page says \
             it has none, and that note is now wrong"
        );
    }

    let surfaces = [
        surface_signatures("deck"),
        surface_signatures("document"),
        surface_signatures("workbook"),
    ];
    for (module, signatures) in ["deck", "document", "workbook"].iter().zip(&surfaces) {
        assert!(
            signatures.len() > 100,
            "{module} yielded only {} public methods — the signature parser has stopped matching",
            signatures.len()
        );
    }

    let mut names: BTreeSet<String> = BTreeSet::new();
    for signatures in &surfaces {
        names.extend(signatures.keys().cloned());
    }

    let mut rows = BTreeMap::new();
    for name in names {
        let mut marks = ['—'; 3];
        let mut crates = BTreeSet::new();
        for (slot, signatures) in surfaces.iter().enumerate() {
            let (symbol, owners) = mark(signatures, &types, &name);
            marks[slot] = symbol;
            crates.extend(owners);
        }
        if !marks.contains(&'✓') {
            continue; // not a shared-markup capability on any surface
        }
        let crates = crates
            .iter()
            .map(|owner| format!("`{owner}`"))
            .collect::<Vec<_>>()
            .join(" + ");
        rows.insert(name, Row { crates, marks });
    }
    rows
}

/// Whether `text` is shaped like a note key: lowercase letters and single dashes, nothing else.
fn is_note_key(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Table 2 and the note headings, as the page states them.
fn documented_rows(page: &str) -> (BTreeMap<String, Row>, BTreeSet<String>, BTreeSet<String>) {
    let mut rows = BTreeMap::new();
    let mut cited = BTreeSet::new();
    let mut defined = BTreeSet::new();
    // A note is a `###` heading whose text is a bare key — `dml-shape`, `chart-range`. The other
    // `###` headings on the page are ordinary prose titles and are not notes.
    for line in page.lines() {
        if let Some(heading) = line.strip_prefix("### ") {
            let heading = heading.trim();
            if is_note_key(heading) {
                defined.insert(heading.to_owned());
            }
            continue;
        }
    }
    // A citation is any `](#key)` link to a note key, wherever on the page it appears — the two
    // crate-level notes are cited from table 1 and from prose rather than from a capability row.
    let mut rest = page;
    while let Some(offset) = rest.find("](#") {
        let after = &rest[offset + "](#".len()..];
        if let Some(end) = after.find(')') {
            if is_note_key(&after[..end]) {
                cited.insert(after[..end].to_owned());
            }
        }
        rest = after;
    }
    for line in page.lines() {
        let Some(cells) = row_cells(line) else {
            continue;
        };
        if cells.len() != 6 {
            continue;
        }
        let Some(name) = code_span(cells[0]) else {
            continue;
        };
        if !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            continue;
        }
        let mut marks = ['—'; 3];
        for (slot, cell) in cells[2..5].iter().enumerate() {
            marks[slot] = match *cell {
                "✓" => '✓',
                "≈" => '≈',
                "—" => '—',
                other => panic!("row `{name}` carries `{other}`, which is not one of ✓ ≈ —"),
            };
        }
        let reason = cells[5];
        if marks.iter().all(|mark| *mark == '✓') {
            assert_eq!(
                reason, "—",
                "row `{name}` is on all three surfaces, so its Reason cell must be `—`"
            );
        } else {
            let key = reason
                .split_once("](")
                .and_then(|(head, _)| head.strip_prefix('['))
                .filter(|key| is_note_key(key))
                .unwrap_or_else(|| {
                    panic!("row `{name}` is asymmetric and must cite a note, not `{reason}`")
                });
            assert!(
                defined.contains(key),
                "row `{name}` cites `{key}`, which the page defines no `### {key}` heading for"
            );
        }
        rows.insert(
            name.to_owned(),
            Row {
                crates: cells[1].to_owned(),
                marks,
            },
        );
    }
    (rows, cited, defined)
}

#[test]
fn the_capability_table_matches_the_facade_source() {
    let page = page();
    let (documented, _, _) = documented_rows(&page);
    let derived = derived_rows();

    assert!(
        derived.len() > 50,
        "only {} shared-markup capabilities were derived — too few to be the real surface",
        derived.len()
    );

    let mut complaint = String::new();
    for (name, row) in &derived {
        match documented.get(name) {
            None => {
                let _ = writeln!(
                    complaint,
                    "  MISSING from the page: `{name}` | {} | {} | {} | {}",
                    row.crates, row.marks[0], row.marks[1], row.marks[2]
                );
            }
            Some(stated) if stated != row => {
                let _ = writeln!(
                    complaint,
                    "  DISAGREES: `{name}` — page says {}/{}/{} ({}), source says {}/{}/{} ({})",
                    stated.marks[0],
                    stated.marks[1],
                    stated.marks[2],
                    stated.crates,
                    row.marks[0],
                    row.marks[1],
                    row.marks[2],
                    row.crates
                );
            }
            Some(_) => {}
        }
    }
    for name in documented.keys() {
        if !derived.contains_key(name) {
            let _ = writeln!(
                complaint,
                "  STALE on the page: `{name}` names no shared-markup type on any surface any more"
            );
        }
    }
    assert!(
        complaint.is_empty(),
        "docs/shared_markup_reachability.md and the facade's source disagree:\n{complaint}\n\
         Update the page — every row is a claim about what a caller can reach, and an asymmetry \
         needs a written reason beside it."
    );
}

#[test]
fn every_note_is_cited_and_every_citation_is_a_note() {
    let page = page();
    let (_, cited, defined) = documented_rows(&page);
    assert!(
        !cited.is_empty(),
        "no row cites a note — the parser is wrong"
    );
    let uncited: Vec<&String> = defined.difference(&cited).collect();
    let undefined: Vec<&String> = cited.difference(&defined).collect();
    assert!(
        undefined.is_empty(),
        "rows cite notes the page does not define: {undefined:?}"
    );
    assert!(
        uncited.is_empty(),
        "the page defines notes no row cites any more — the asymmetry they explained is gone, so \
         the note is now prose about nothing: {uncited:?}"
    );
}
