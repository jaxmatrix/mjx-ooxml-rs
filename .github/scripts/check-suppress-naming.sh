#!/usr/bin/env bash
#
# Fail the build if a public identifier spells "this thing is switched off" as `delete`.
#
# The concept is `c:delete` / `c:autoTitleDeleted` in the chart part: markup that says *draw nothing
# here*. The project's rule is that a public identifier must be self-explanatory to a reader who does
# not have ECMA-376 open, and `delete_chart_data_labels` sat directly beside
# `remove_chart_data_labels` — near-synonyms in English for two genuinely different operations
# (write a `c:delete`, versus remove the element and inherit again). The family is therefore spelled
# `suppress_*` / `suppressed`, and this script is what keeps it that way. A claim CI does not check
# is a claim that quietly stops being true, so this checks it.
#
# PERMITTED spellings of `delete`, and why:
#
#   * the wire token in a string literal — `flag("delete")`, `chart_val_leaf(interner, "delete", "1")`,
#     `self.scalar(interner, "autoTitleDeleted")`. The token is the file format's, never ours, and it
#     is preserved exactly. A bare quoted `delete` is never adjacent to an identifier character, so
#     the pattern below cannot see it.
#   * the element name in prose — `c:delete`, `<c:delete val="1"/>`, `c:dLbls`. Same reason: `:` and
#     `<` are not identifier characters.
#   * the exact wire spelling `autoTitleDeleted`, which *is* identifier-shaped — allow-listed by
#     name below, and only in that exact casing, so `auto_title_deleted` and `AutoTitleDeleted`
#     still fail.
#   * **WordprocessingML's tracked-change deletion vocabulary.** `w:delText` ("Deleted Text",
#     ECMA-376 Part 1 §17.3.3.7), `w:delInstrText` ("Deleted Field Code", §17.16.13), `w:del`
#     ("Deletion", §17.13.5.15 — the tracked-change wrapper a paragraph mark's own run properties
#     carry, MJXOFF-96) and the `Deleted*`/bare `Deleted` Rust names that model them. This gate
#     exists for the *chart* concept `c:delete` — "this label tier is switched off" — which was
#     renamed because `delete_*` sat beside `remove_*` and read as its synonym. A tracked deletion is
#     not that concept: it is a real deletion, the spec's own caption is "Deletion" or "Deleted
#     Text", and `Suppressed` would be wrong rather than clearer. Allow-listed by exact token and by
#     the `Deleted`-prefixed identifiers derived from them, so an unrelated `deleted` still fails.
#     The **bare** `Deleted` — which `CT_ParaRPr`'s tracked-change wrapper needs as an enum variant —
#     is allowed **only under `crates/mjx-docx/` and `crates/mjx-omml/`**, the two crates whose
#     schemas carry tracked changes. Allowing it workspace-wide would let the chart concept return
#     as `enum ChartLabelTier { Deleted }` inside `mjx-chart` itself, which is the one place this
#     gate exists to police; scoping it by path closes that while leaving Word's variant legal.
#   * **Every other delete-family identifier MJXOFF-126 (revision marks) adds under those same two
#     crates** — `CellDeleted`/`cell_deleted` (`w:cellDel`, "a tracked-deleted table cell"),
#     `MarkerDeleted` (the enumeration a bare tracked-deletion marker reports),
#     `deleted`/`set_deleted` (a paragraph mark's or row's own `w:del`), `deleted_zone`/
#     `deleted_run_text` (reading `w:delText` rather than `w:t` while resolving a `w:del`'s own
#     content). None of these is the chart's "switched off" concept either — `mjx-docx` and
#     `mjx-omml` carry no such concept to confuse it with — so rather than enumerate every future
#     compound one exact token at a time, the whole `delete` family is permitted, case-insensitively,
#     under those two paths (a case-insensitive substitution, `gI`, since the FORBIDDEN pattern above
#     is itself matched case-insensitively).
#   * **SpreadsheetML's data-table input cells** (MJXOFF-115) — `first_input_cell_deleted` and
#     `second_input_cell_deleted`, the accessors for `CT_CellFormula`'s `@del1` and `@del2`. ECMA-376
#     Part 1 §18.3.1.40's own captions are "Input 1 Deleted" and "Input 2 Deleted", and the
#     description is *"Whether the first input cell for data table has been deleted"* — a **real
#     deletion of a cell**, reported as a fact about the file, which is the same situation as
#     WordprocessingML's tracked-change vocabulary two bullets above and not the chart concept this
#     gate exists for. `first_input_cell_suppressed` would be wrong rather than clearer: nothing is
#     switched off, a cell is gone. Allow-listed by **exact token and by path** — only
#     `first_input_cell_deleted` / `second_input_cell_deleted`, and only under
#     `crates/mjx-sml/src/formula/`, so an unrelated `deleted` anywhere in `mjx-sml` still fails and
#     a chart-shaped `delete_*` planted in that very file still fails (probed, both ways, in
#     MJXOFF-115's pull request).
#   * **SpreadsheetML's sheet grid** (MJXOFF-117) — four identifiers under
#     `crates/mjx-sml/src/worksheet/`, each a real deletion rather than the chart's "switched off",
#     and each named from ECMA-376 Part 1's own prose:
#       - `locks_deleting_columns` / `locks_deleting_rows`, for `CT_SheetProtection`'s
#         `@deleteColumns` and `@deleteRows`. §18.3.1.85's captions are "Delete Columns Locked" and
#         "Delete Rows Locked", and the descriptions are *"If 1 or true then deleting columns should
#         not be allowed when the sheet is protected"*. The thing being locked **is** a deletion of
#         columns; `locks_suppressing_columns` would describe a different operation, and dropping the
#         verb entirely would lose which of `formatColumns`/`insertColumns`/`deleteColumns` is meant.
#       - `input_cell_was_deleted` / `deletion_was_undone`, for `CT_InputCells`'s `@deleted` and
#         `@undone`. §18.3.1.52 spells them out in full — *"Input cell was deleted. This input cell
#         shall be present in the file format, but shall not be presented to the user"* and *"Cell's
#         deletion was undone"* — and the accessors are those sentences. This is the same situation as
#         the data-table `@del1`/`@del2` bullet above: a cell is gone, and `suppressed` would be
#         wrong rather than clearer.
#     Allow-listed by **exact token and by path** — only those four, and only under
#     `crates/mjx-sml/src/worksheet/`, so an unrelated `deleted` anywhere else in `mjx-sml` still
#     fails and a chart-shaped `delete_chart_data_labels` planted in `protection.rs` or
#     `scenarios.rs` still fails. Probed both ways before this entry was written.
#
#     The three **wire tokens** these accessors are declared against — `"deleteColumns"`,
#     `"deleteRows"`, `"deleted"` — are permitted on the same path, and only **inside their double
#     quotes**. `deleteColumns` is identifier-shaped, so unlike the chart's bare `flag("delete")` the
#     quotes alone do not save it; requiring them is what keeps the exemption to a string literal,
#     which is where this script has always said a wire token belongs. A bare `deleteColumns`
#     identifier under the same path still fails.
#   * `crates/mjx-ooxml-types/src/generated/`, which is generated from the XSDs and is nothing but
#     wire tokens.
#   * **The two bindings' own projection of `RevisionKind`** (MJXOFF-139) — `Deleted` and
#     `MarkerDeleted`, in `bindings/mjx-python/src/enums.rs`, `bindings/mjx-wasm/src/enums.rs` and
#     the committed `.pyi` stub. `RevisionKind` is `mjx_docx`'s own tracked-change vocabulary (the
#     bullet two above this one), reprojected member-for-member by `sealed_enums!`/`open_enums!` —
#     the same identifiers, not a new naming decision, so the reasoning that already permits `Deleted`
#     under `crates/mjx-docx/` applies unchanged to the classes that mirror it. **File scoping alone
#     was tried first and rejected**: a probe planting `ChartLabelTierProbe { Deleted }` elsewhere in
#     `bindings/mjx-python/src/enums.rs` (a file that also carries `mjx-chart`'s own enumerations)
#     passed the gate when the exemption blanked every `Deleted`/`MarkerDeleted` token in that file
#     regardless of which enum declared it — the same "excuses what it might" failure MJXOFF-138's own
#     namespace allow-list names. So the exemption is scoped **twice**: by **line range**, computed
#     fresh on every run from each file's own `RevisionKind { … }` (or, for the stub,
#     `class RevisionKind:` … the next blank line) block boundaries below — not hand-copied numbers
#     that could drift out of sync with the source — and by the **exact line shape**
#     `sealed_enums!`/`open_enums!`/the stub generator produce for these two variants specifically
#     (`        Deleted,`, `    Deleted: RevisionKind`, and their `MarkerDeleted` counterparts), never
#     a bare substring match. Two probes proved both halves matter: `ChartLabelTierProbe { Deleted }`
#     planted elsewhere in the same file (outside the line range) still fails the gate, and a
#     `DeletedSomethingElse` variant planted *inside* `RevisionKind`'s own block (in range, but not one
#     of the two exact permitted lines) still fails it too — MJXOFF-139's own commit message pastes
#     both.
#   * **The same `RevisionKind` variant, a third time, in wasm-bindgen's own generated output**
#     (MJXOFF-139) — `bindings/mjx-wasm/npm/dist/{web,bundler}/mjx_ooxml.js` (glue code; for the
#     `bundler` target this lives in `mjx_ooxml_bg.js` instead, since that target splits the glue
#     from the re-exporting wrapper) and the matching `.d.ts` in each. This is not a new naming
#     decision either — it is wasm-bindgen re-emitting the identifiers the bullet above already
#     permits, once as a frozen JS object (`Deleted: 1, "1": "Deleted",`) and once as a TypeScript
#     `enum` member (`Deleted = 1,`) — but it was **missed on first landing**: the local check only
#     ever ran against `wasm-pack test --node`, which builds a test harness and never touches
#     `npm/dist`, so this arm was never exercised until CI's *second* invocation of this script
#     (`check-suppress-naming.sh bindings/mjx-wasm/npm/dist`, run only after `build-npm.sh`) caught
#     it. Scoped exactly like the source projection — by a freshly computed line range per file
#     (`export const RevisionKind = Object.freeze({` … `});` for JS, `export enum RevisionKind {` …
#     `}` for TypeScript) and by the exact generated line shape, never a blanket exemption for
#     `npm/dist` as a whole, which would reopen precisely the hole the two probes above already
#     closed once. A generated enum's own discriminant number is not fixed in these patterns
#     (`[0-9]+`, not `1`/`16`) since it is derived from variant order, not a naming decision this
#     gate polices. **The general lesson, for the next generated enum that adds a permitted
#     identifier here**: a source-scoped exemption is not automatically dist-scoped too — any build
#     step that re-emits an already-permitted identifier into a generated artifact this script's
#     `targets` can reach needs its own line-range-and-shape entry, and the local dry run must
#     actually produce that artifact (`build-npm.sh`, not just `wasm-pack test`) before trusting a
#     green result.
#
# FORBIDDEN: anything identifier-shaped — `delete_x`, `x_delete`, `deleted`, `deleteX`, `Deleted`,
# `delete(`. That is the whole rule.
#
# SCOPE. The default scan is the public surface: `crates/*/src`, `bindings/*/src`, and the committed
# Python stub. Test and example call sites are deliberately not scanned — a test that calls a renamed
# method does not compile, so the compiler is already a stricter gate there than grep is, and test
# fixtures are full of legitimate `c:delete` markup. The generated TypeScript declarations are not in
# the repository (`bindings/mjx-wasm/npm/dist` is git-ignored); CI passes them to this script by hand
# after `build-npm.sh` has produced them, which is why the paths are arguments rather than hard-coded.
#
# Usage:
#     .github/scripts/check-suppress-naming.sh [path ...]
#
# With no arguments it scans the default surface listed above, from the repository root.

set -euo pipefail

cd "$(dirname "$0")/../.."

if [ "$#" -gt 0 ]; then
  targets=("$@")
else
  targets=(crates/*/src bindings/*/src bindings/mjx-python/python/mjx_ooxml/__init__.pyi)
fi

# `delete` with an identifier character on either side, or standing alone as a call. Matched
# case-insensitively, so `Deleted` and `deleteChartDataLabels` are caught as well as `deleted`.
pattern='(delete[a-zA-Z0-9_]|[a-zA-Z0-9_]delete|(^|[^a-zA-Z0-9_])delete[[:space:]]*\()'

# The line range of `RevisionKind`'s own block in one file, as "start,end" for a `sed` address —
# computed fresh from the file every run, not hand-copied line numbers that could silently stop
# matching the source they describe. `start_pattern` finds the block's own opening line;
# `end_pattern` is the first line at or after it that closes the block.
#
# Guarded against a missing `file`, not just an absent block within one: the four wasm dist ranges
# below name paths under `bindings/mjx-wasm/npm/dist`, which is git-ignored and exists only after
# `build-npm.sh` has run. CI invokes this script twice — once over the default source surface, once
# with the dist as an explicit argument (see the SCOPE comment above) — and the first of those runs
# before the dist exists, as does every plain local run that never built it. Checked, not assumed:
# `start=$(failing pipeline)` here does not actually trip `errexit` on its own (bash does not treat
# a command substitution's exit status as the assignment statement's own when the assignment is not
# the last thing the enclosing function does), so an unguarded version of this function still
# returns "0,0" for a missing file rather than aborting the script — verified by running an
# unguarded copy with the dist directory moved aside. The guard exists to keep that case *quiet*:
# without it, `grep` prints a "No such file or directory" line to stderr for each of the four dist
# paths on every ordinary source-only run, which reads as a real problem it is not.
revision_kind_range() {
  local file="$1" start_pattern="$2" end_pattern="$3"
  local start end
  if [ ! -f "$file" ]; then
    echo "0,0" # this invocation never named this file — nothing to scope
    return
  fi
  start=$(grep -nE "$start_pattern" "$file" | head -1 | cut -d: -f1)
  if [ -z "$start" ]; then
    echo "0,0" # no such block in this file — the substitution below then matches nothing
    return
  fi
  end=$(tail -n "+$start" "$file" | grep -nE "$end_pattern" | head -1 | cut -d: -f1)
  echo "$start,$((start + end - 1))"
}

py_enums_range=$(revision_kind_range bindings/mjx-python/src/enums.rs '^    RevisionKind \{$' '^    \}$')
wasm_enums_range=$(revision_kind_range bindings/mjx-wasm/src/enums.rs '^    RevisionKind \{$' '^    \}$')
pyi_stub_range=$(revision_kind_range bindings/mjx-python/python/mjx_ooxml/__init__.pyi '^class RevisionKind:$' '^$')

# The same variant, reprojected a second time by wasm-bindgen itself into the npm package's
# generated JS glue and `.d.ts` — not hand-written, not covered by the source-scoped exemption
# above, and only reachable once `build-npm.sh` has produced `bindings/mjx-wasm/npm/dist`. Two
# build targets (`web`, a single bundled file; `bundler`, glue split into `..._bg.js`) each emit the
# enum once in JS and once in TypeScript, so four files, each scoped by its own freshly computed
# line range exactly as the source files are above — never a blanket exemption for the dist
# directory, which would reopen the same "excuses what it might" hole the two probes in the bullet
# above this already found and closed once for the source files.
wasm_dist_web_js_range=$(revision_kind_range bindings/mjx-wasm/npm/dist/web/mjx_ooxml.js \
  '^export const RevisionKind = Object\.freeze\(\{$' '^\}\);$')
wasm_dist_web_dts_range=$(revision_kind_range bindings/mjx-wasm/npm/dist/web/mjx_ooxml.d.ts \
  '^export enum RevisionKind \{$' '^\}$')
wasm_dist_bundler_js_range=$(revision_kind_range bindings/mjx-wasm/npm/dist/bundler/mjx_ooxml_bg.js \
  '^export const RevisionKind = Object\.freeze\(\{$' '^\}\);$')
wasm_dist_bundler_dts_range=$(revision_kind_range bindings/mjx-wasm/npm/dist/bundler/mjx_ooxml.d.ts \
  '^export enum RevisionKind \{$' '^\}$')

# Two passes with the same pattern: the first finds candidate lines, then every permitted spelling is
# blanked out and the pattern is re-applied, so a line carrying both a permitted token and a real
# offender is still reported. (Blanking beats dropping the line.)
#
# The `///`/`//!` arm is deliberately narrow. It blanks *only* the standalone English words
# `deleted`/`deletion`/`delete` on a documentation line — never `delete_x`, `x_delete` or `delete(`,
# which is what a reference to a renamed API looks like. So `delete_chart_data_labels` in a doc
# comment is still caught, exactly as MJXOFF-89 intended, while ECMA-376's own captions ("Deleted
# Text", "Deleted Field Code") and ordinary English about a tracked deletion are not. A doc comment
# declares no public identifier, and this gate's subject is public identifiers.
offenders=$(grep -rnEi "$pattern" "${targets[@]}" 2>/dev/null \
  | grep -v '^crates/mjx-ooxml-types/src/generated/' \
  | sed -E '/^[^:]+:[0-9]+:[[:space:]]*(\/\/\/|\/\/!)/ s/\b([Dd]elet(ed|ion)|[Dd]elete)\b/<prose>/g' \
  | sed -E -e 's/autoTitleDeleted/<wire-token>/g' \
        -e 's/delInstrText/<wire-token>/g' \
        -e 's/delText/<wire-token>/g' \
        -e 's/DeletedFieldCode/<wml-revision>/g' \
        -e 's/DeletedText/<wml-revision>/g' \
        -e '/^crates\/(mjx-docx|mjx-omml)\//Is/delet(e|ed|ing|ion)[A-Za-z0-9_]*/<wml-revision>/gI' \
        -e '/^crates\/mjx-sml\/src\/formula\//s/(first|second)_input_cell_deleted/<data-table-input>/g' \
        -e '/^crates\/mjx-sml\/src\/worksheet\//s/locks_deleting_(columns|rows)/<sheet-protection-lock>/g' \
        -e '/^crates\/mjx-sml\/src\/worksheet\//s/input_cell_was_deleted|deletion_was_undone/<scenario-input-cell>/g' \
        -e '/^crates\/mjx-sml\/src\/worksheet\//s/"(deleteColumns|deleteRows|deleted)"/<wire-token>/g' \
  | awk -F: -v py_range="$py_enums_range" -v wasm_range="$wasm_enums_range" -v pyi_range="$pyi_stub_range" \
        -v dist_web_js_range="$wasm_dist_web_js_range" -v dist_web_dts_range="$wasm_dist_web_dts_range" \
        -v dist_bundler_js_range="$wasm_dist_bundler_js_range" -v dist_bundler_dts_range="$wasm_dist_bundler_dts_range" '
      # Portable (POSIX awk, no GNU \< \> word-boundary extension) — matches the *exact* line shape
      # `sealed_enums!`/`open_enums!` (Rust), the stub generator (`.pyi`), or wasm-bindgen itself
      # (the generated JS/TS below) produce for a bare variant, never a substring, so this cannot
      # blank `Deleted` inside some other identifier that happens to contain it.
      function in_range(line, range,   parts, n) {
        n = split(range, parts, ",")
        return n == 2 && line + 0 >= parts[1] + 0 && line + 0 <= parts[2] + 0
      }
      {
        file = $1; line = $2; rest = $0
        sub(/^[^:]*:[^:]*:/, "", rest)
        scoped = 0
        is_dist_js = 0
        is_dist_ts = 0
        if (file == "bindings/mjx-python/src/enums.rs" && in_range(line, py_range)) scoped = 1
        if (file == "bindings/mjx-wasm/src/enums.rs" && in_range(line, wasm_range)) scoped = 1
        if (file == "bindings/mjx-python/python/mjx_ooxml/__init__.pyi" && in_range(line, pyi_range)) scoped = 1
        if (file == "bindings/mjx-wasm/npm/dist/web/mjx_ooxml.js" && in_range(line, dist_web_js_range)) { scoped = 1; is_dist_js = 1 }
        if (file == "bindings/mjx-wasm/npm/dist/bundler/mjx_ooxml_bg.js" && in_range(line, dist_bundler_js_range)) { scoped = 1; is_dist_js = 1 }
        if (file == "bindings/mjx-wasm/npm/dist/web/mjx_ooxml.d.ts" && in_range(line, dist_web_dts_range)) { scoped = 1; is_dist_ts = 1 }
        if (file == "bindings/mjx-wasm/npm/dist/bundler/mjx_ooxml.d.ts" && in_range(line, dist_bundler_dts_range)) { scoped = 1; is_dist_ts = 1 }
        if (scoped) {
          if (rest ~ /^[ \t]*Deleted,[ \t]*$/) rest = "<wml-revision>"
          if (rest ~ /^[ \t]*MarkerDeleted,[ \t]*$/) rest = "<wml-revision>"
          if (rest ~ /^[ \t]*Deleted: RevisionKind[ \t]*$/) rest = "<wml-revision>"
          if (rest ~ /^[ \t]*MarkerDeleted: RevisionKind[ \t]*$/) rest = "<wml-revision>"
        }
        if (is_dist_js) {
          if (rest ~ /^[ \t]*Deleted: [0-9]+, "[0-9]+": "Deleted",[ \t]*$/) rest = "<wml-revision>"
          if (rest ~ /^[ \t]*MarkerDeleted: [0-9]+, "[0-9]+": "MarkerDeleted",[ \t]*$/) rest = "<wml-revision>"
        }
        if (is_dist_ts) {
          if (rest ~ /^[ \t]*Deleted = [0-9]+,[ \t]*$/) rest = "<wml-revision>"
          if (rest ~ /^[ \t]*MarkerDeleted = [0-9]+,[ \t]*$/) rest = "<wml-revision>"
        }
        print file ":" line ":" rest
      }
    ' \
  | grep -Ei "$pattern" \
  || true)

if [ -n "$offenders" ]; then
  echo "an identifier spells this concept \`delete\`:"
  echo "$offenders"
  echo
  echo "This concept is spelled \`suppress_*\` / \`suppressed\` throughout — \`delete\` read as a"
  echo "synonym of \`remove\`, which is a different operation on the same markup. The wire token"
  echo "stays exact in string literals and in each item's doc comment; only identifiers change."
  echo "If a genuinely new spelling must be permitted, add it to the allow-list in this script"
  echo "with its reason, so the exception is reviewed rather than assumed."
  exit 1
fi

echo "no identifier spells this concept \`delete\` in: ${targets[*]}"
