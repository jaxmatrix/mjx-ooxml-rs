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
#   * the exact wire spelling `autoTitleDeleted`, which *is* identifier-shaped. It is the only such
#     token, so it is allow-listed by name below — and only in that exact casing, so
#     `auto_title_deleted` and `AutoTitleDeleted` still fail.
#   * `crates/mjx-ooxml-types/src/generated/`, which is generated from the XSDs and is nothing but
#     wire tokens.
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

# Two passes with the same pattern: the first finds candidate lines, then the one permitted
# identifier-shaped wire token is blanked out and the pattern is re-applied, so a line carrying both
# `autoTitleDeleted` and a real offender is still reported. (Blanking beats dropping the line.)
offenders=$(grep -rnEi "$pattern" "${targets[@]}" 2>/dev/null \
  | grep -v '^crates/mjx-ooxml-types/src/generated/' \
  | sed 's/autoTitleDeleted/<wire-token>/g' \
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
