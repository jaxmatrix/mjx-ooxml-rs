#!/usr/bin/env bash
# Runs the whole check set in one pass and records each command with its exit code and its start
# and finish timestamps. The report is written to GATE_RESULTS_<child>.txt at the repository root.
#
# Not a substitute for `npm run check` — it is the same set, with the evidence written down. A gate
# result that predates the last edit is not a gate result, so this exists to make "when did this
# run" a fact rather than a memory.
#
# Usage: gate-run.sh [child] [title]
#   child   the suffix of the report file, e.g. U02. Defaults to U01, which is what MJXOFF-180 ran.
#   title   the heading written at the top of the report.
#
# Parameterised by MJXOFF-181, because two children in the same checkout writing to one report
# means the second overwrites the first's evidence — and evidence that was overwritten is evidence
# nobody has.
set -u

child="${1:-U01}"
title="${2:-MJXOFF-180 [U01] — the ui/ workspace, Storybook and its gates}"

ui="$(cd "$(dirname "$0")/.." && pwd)"
report="$(cd "$ui/.." && pwd)/GATE_RESULTS_${child}.txt"

: > "$report"
{
  echo "$title"
  echo "branch:  $(git -C "$ui/.." rev-parse --abbrev-ref HEAD)"
  echo "head:    $(git -C "$ui/.." rev-parse --short HEAD)"
  echo "node:    $(node --version)   npm: $(npm --version)"
  echo "started: $(date --iso-8601=seconds)"
  echo
} >> "$report"

failures=0

log="/tmp/mjx-gate-${child}.log"

# run <label> <working directory> <command…>
run() {
  label="$1"
  directory="$2"
  shift 2
  started="$(date --iso-8601=seconds)"
  ( cd "$directory" && "$@" ) > "$log" 2>&1
  status=$?
  finished="$(date --iso-8601=seconds)"
  {
    printf '%-24s exit=%-3d start=%s  finish=%s\n' "$label" "$status" "$started" "$finished"
    if [ "$status" -ne 0 ]; then
      echo "    --- last 30 lines ---"
      tail -30 "$log" | sed 's/^/    /'
    fi
  } >> "$report"
  if [ "$status" -ne 0 ]; then failures=$((failures + 1)); fi
  return 0
}

run "tokens:check"     "$ui" npm run --silent tokens:check
run "icons:check"      "$ui" npm run --silent icons:check
run "typecheck"        "$ui" npm run --silent typecheck
run "lint"             "$ui" npm run --silent lint
run "test:unit"        "$ui" npm run --silent test:unit
run "build-storybook"  "$ui" npm run --silent build-storybook
run "test:browser"     "$ui" npm run --silent test:browser

# The Rust-side sanity check: `ui/` is a Node workspace outside the rank graph, so `cargo metadata`
# must still succeed and must name no member under it. Cheap, and it is the only claim this child
# makes about the other track's tree.
run "cargo metadata"   "$ui/.." sh -c \
  'cargo metadata --no-deps --format-version 1 > /tmp/mjx-gate-metadata.json &&
   ! grep -q "\"manifest_path\":\"[^\"]*/ui/" /tmp/mjx-gate-metadata.json &&
   echo "cargo metadata: $(grep -o "\"name\":" /tmp/mjx-gate-metadata.json | wc -l) entries, none under ui/"'

{
  echo
  echo "finished: $(date --iso-8601=seconds)"
  echo "failures: $failures"
} >> "$report"

cat "$report"
exit "$failures"
