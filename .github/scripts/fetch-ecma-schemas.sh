#!/usr/bin/env bash
#
# Fetch the ECMA-376 XML schemas the schema-validity suite validates against, into the git-ignored
# `References/` tree that `crates/mjx-pptx/tests/schema_validity.rs` already looks in by default.
#
# Why fetch instead of committing the XSDs: `References/` is git-ignored by a standing rule of this
# repository, and whether ECMA-376 schema files may be redistributed inside the tree is a decision
# reserved for the repository owner. Downloading them, pinned by SHA-256, keeps the tree clean while
# still making the gate reproducible — a re-upload or a corrupted download cannot quietly change what
# we validate against.
#
# Usage:
#     .github/scripts/fetch-ecma-schemas.sh [references-dir]
#
# `references-dir` defaults to `References/` at the repository root — the same location a developer
# already keeps the spec in, and the default `schema_validity.rs` discovers with no environment
# variables set. Pass a directory to populate somewhere else; then point `MJX_SCHEMA_DIR` and
# `MJX_OPC_SCHEMA_DIR` at the two printed paths.
#
# The script is idempotent and safe to re-run: an archive already present is not re-downloaded, every
# archive is verified on every run (so a poisoned or truncated CI cache is caught, not trusted), and
# extraction overwrites in place. Only the archives named in the manifest are touched; any other
# ECMA part already sitting in `References/` is left alone.
#
# ## What each part costs, and why the figures are stated separately (MJXOFF-197)
#
# Part 1 was added so that two gates which had *never executed on CI* could execute: the sweep over
# the whole normative preset-shape corpus (`crates/mjx-dml/tests/guide_formula.rs`) and the
# freshness check on the committed geometry table (`xtask/src/codegen/geometry.rs`). Both read
# `presetShapeDefinitions.xml`, which ships only with Part 1, and both skipped silently without it —
# an absent corpus reads exactly like success.
#
# The cost is two different numbers and it matters which one a reader takes away:
#
#   * **the download and the CI cache grow by 42 MB** — the outer archive is atomic, and 35.3 MB of
#     it is the Part 1 PDF with a further 14.4 MB of `WordprocessingMLArtBorders`, neither of which
#     this repository has any use for;
#   * **the extracted tree grows by ~1.5 MB** — the two members actually wanted are 51 672 and
#     93 849 bytes compressed. The `-j`-plus-explicit-member extraction below is what keeps the
#     difference: the PDF is never written to disk at all.
#
# Requires: bash, curl, unzip, sha256sum.

set -euo pipefail

readonly BASE_URL="https://ecma-international.org/wp-content/uploads"

# `outer archive|member we need:a file that member must contain|member:marker|…`.
#
# The published archives nest: the outer zip holds the part's PDF plus further zips, and the XSDs are
# one level down. The marker is checked after extraction so a changed inner layout fails loudly
# rather than leaving an empty directory for the test suite to skip over.
#
# **One outer archive is exactly one entry, with a list of members after it.** Part 1 needs *two* of
# its six members, and the obvious shape — two entries naming the same outer — is wrong twice:
# `verify_archives` runs `sha256sum --check --strict` against a manifest that must carry each file
# once, so the two lists stop being in correspondence, and the next person adding a part cannot tell
# which of them is authoritative. Hence a member *list*, each member carrying its own marker.
readonly ARCHIVES=(
    "ECMA-376-4_5th_edition_december_2016.zip|OfficeOpenXML-XMLSchema-Transitional.zip:pml.xsd"
    "ECMA-376-2_5th_edition_december_2021.zip|OpenPackagingConventions-XMLSchema.zip:opc-relationships.xsd"
    # Part 1 carries THREE members this workspace reads, and they are one entry because they come
    # out of one 42 MB download:
    #   * `OfficeOpenXML-XMLSchema-Strict` — the Strict half of the namespace table the generator
    #     pairs against Transitional.
    #   * `presetShapeDefinitions.xml` — the normative preset-shape corpus.
    #     `xtask/tests/published_markup.rs` holds every published geometry name to
    #     `PresetShapeType`'s wire tokens (MJXOFF-250), and `mjx-geometry`'s table is generated from
    #     it (MJXOFF-202). Added to CI by MJXOFF-197, because two gates that read that corpus had
    #     never once executed there — they skipped, and an absent corpus reads exactly like success.
    #   * `presetCellStyles.xml` — not a schema: ECMA's own built-in cell and table styles, the only
    #     SpreadsheetML markup the standard publishes. `crates/mjx-sml/tests/theme_index.rs` derives
    #     the `@theme` position table from them rather than restating it (MJXOFF-246).
    # Only ~1.5 MB of the 42 is ever extracted; the rest is the part's PDF and 14.4 MB of Word art
    # borders. Each member carries its own marker, so a changed inner layout fails loudly.
    "ECMA-376-1_5th_edition_december_2016.zip|OfficeOpenXML-XMLSchema-Strict.zip:dml-main.xsd|OfficeOpenXML-DrawingMLGeometries.zip:presetShapeDefinitions.xml|OfficeOpenXML-SpreadsheetMLStyles.zip:presetCellStyles.xml"
)

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly repo_root
readonly manifest="$repo_root/.github/ecma-376-archives.sha256"

references_dir="${1:-$repo_root/References}"
mkdir -p "$references_dir"
references_dir="$(cd "$references_dir" && pwd)"
readonly references_dir

tmp_dir="$(mktemp -d)"
readonly tmp_dir
trap 'rm -rf "$tmp_dir"' EXIT

# Downloads every archive that is not already on disk. Writing to `.part` first means an interrupted
# run never leaves behind a truncated file that looks complete.
fetch_missing_archives() {
    local entry name
    for entry in "${ARCHIVES[@]}"; do
        name="${entry%%|*}"
        if [[ -f "$references_dir/$name" ]]; then
            echo "==> $name: already present"
            continue
        fi
        echo "==> $name: downloading from $BASE_URL"
        curl --location --fail --silent --show-error --retry 3 --retry-delay 2 \
            --output "$references_dir/$name.part" "$BASE_URL/$name"
        mv "$references_dir/$name.part" "$references_dir/$name"
    done
}

# Verifies every archive against the committed manifest. On mismatch the offending files are deleted
# — so the caller can re-fetch once — and a non-zero status is returned.
verify_archives() {
    local report="$tmp_dir/checksums.txt" line bad
    if (cd "$references_dir" && sha256sum --check --strict "$manifest") >"$report" 2>&1; then
        cat "$report"
        return 0
    fi
    cat "$report" >&2
    while IFS= read -r line; do
        case "$line" in
        *": FAILED"*)
            bad="${line%%: FAILED*}"
            echo "==> discarding $bad" >&2
            rm -f "$references_dir/$bad"
            ;;
        esac
    done <"$report"
    return 1
}

# `ARCHIVES` and the manifest are two lists that must name the same files. `sha256sum --check` only
# catches one direction — a manifest line whose file is absent — so an `ARCHIVES` entry with no
# manifest line would be downloaded and extracted *unverified*, which is the one thing this script
# exists to prevent. Checked before anything is fetched, so the failure names the entry rather than
# arriving as a surprise after a 42 MB download.
verify_manifest_covers_every_archive() {
    local entry name missing=0
    for entry in "${ARCHIVES[@]}"; do
        name="${entry%%|*}"
        if ! grep -q -F -- "  $name" "$manifest"; then
            echo "$name is in ARCHIVES but not in $manifest — it would be fetched unverified" >&2
            missing=1
        fi
    done
    return "$missing"
}

verify_manifest_covers_every_archive
fetch_missing_archives
if ! verify_archives; then
    echo "==> checksum mismatch; re-fetching the discarded archives once" >&2
    fetch_missing_archives
    if ! verify_archives; then
        echo "ECMA-376 archives do not match $manifest. If ECMA has republished them, confirm the" >&2
        echo "new contents by hand and update the manifest — do not weaken this check." >&2
        exit 1
    fi
fi

for entry in "${ARCHIVES[@]}"; do
    IFS='|' read -r -a fields <<<"$entry"
    outer="${fields[0]}"

    for spec in "${fields[@]:1}"; do
        member="${spec%%:*}"
        marker="${spec#*:}"
        dest="$references_dir/${outer%.zip}/${member%.zip}"

        mkdir -p "$dest"
        # `-j` and an explicit member: the outer archives also carry the part's PDF (35.3 MB for
        # Part 1 alone), a RELAX NG copy and — in Part 1 — 14.4 MB of Word art borders, none of
        # which this repository has any use for. Never writing them is what keeps the extracted
        # tree at ~1.5 MB per part while the download is 42 MB.
        unzip -o -q -j "$references_dir/$outer" "$member" -d "$tmp_dir"
        unzip -o -q "$tmp_dir/$member" -d "$dest"

        if [[ ! -f "$dest/$marker" ]]; then
            echo "$outer extracted $member without $marker — the archive layout changed" >&2
            exit 1
        fi
        echo "==> $dest"
    done
done
