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
# extraction overwrites in place. Only the two archives named in the manifest are touched; any other
# ECMA part already sitting in `References/` is left alone.
#
# Requires: bash, curl, unzip, sha256sum.

set -euo pipefail

readonly BASE_URL="https://ecma-international.org/wp-content/uploads"

# `outer archive|the one member we need|a file that member must contain`.
# The published archives nest: the outer zip holds the part's PDF plus further zips, and the XSDs are
# one level down. The marker is checked after extraction so a changed inner layout fails loudly
# rather than leaving an empty directory for the test suite to skip over.
readonly ARCHIVES=(
    "ECMA-376-4_5th_edition_december_2016.zip|OfficeOpenXML-XMLSchema-Transitional.zip|pml.xsd"
    "ECMA-376-2_5th_edition_december_2021.zip|OpenPackagingConventions-XMLSchema.zip|opc-relationships.xsd"
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
    outer="${entry%%|*}"
    rest="${entry#*|}"
    member="${rest%%|*}"
    marker="${rest##*|}"
    dest="$references_dir/${outer%.zip}/${member%.zip}"

    mkdir -p "$dest"
    # `-j` and an explicit member: the outer archive also carries a 10 MB PDF and a RELAX NG copy we
    # have no use for, and never writing them keeps the CI cache and the disk footprint small.
    unzip -o -q -j "$references_dir/$outer" "$member" -d "$tmp_dir"
    unzip -o -q "$tmp_dir/$member" -d "$dest"

    if [[ ! -f "$dest/$marker" ]]; then
        echo "$outer extracted without $marker — the archive layout changed" >&2
        exit 1
    fi
    echo "==> $dest"
done
