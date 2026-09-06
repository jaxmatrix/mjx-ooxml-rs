#!/usr/bin/env bash
# The format axis of the feature inventory: how much markup each format actually defines.
#
# Counts declared elements, complex types and simple types per ECMA-376 schema. This is the ceiling
# on rendering and editing work that comes from the file format itself — every element here either
# has a visual consequence, an editing consequence, or is preserved opaquely by decision.
#
# Reads the Transitional schemas, which are this project's primary target (see PLAN.md). Requires
# `References/` — see the ECMA-376 reference-schema setup notes; it is git-ignored and local-only.
set -euo pipefail

ROOT="${1:-References}"
DIR="$(find "$ROOT" -type d -name 'OfficeOpenXML-XMLSchema-Transitional' -print -quit)"
[ -n "$DIR" ] || { echo "no Transitional schema dir under $ROOT" >&2; exit 1; }

printf '%-32s %9s %13s %12s\n' SCHEMA elements complexTypes simpleTypes
printf '%-32s %9s %13s %12s\n' '--------------------------------' --------- ------------- ------------
total_e=0
for f in "$DIR"/*.xsd; do
  name="$(basename "$f" .xsd)"
  e=$(grep -c '<xsd:element name=' "$f" || true)
  c=$(grep -c '<xsd:complexType name=' "$f" || true)
  s=$(grep -c '<xsd:simpleType name=' "$f" || true)
  [ "$e" -eq 0 ] && [ "$c" -eq 0 ] && continue
  printf '%-32s %9s %13s %12s\n' "$name" "$e" "$c" "$s"
  total_e=$((total_e + e))
done
printf '%-32s %9s\n' 'TOTAL declared elements' "$total_e"
