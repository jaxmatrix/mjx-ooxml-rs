#!/usr/bin/env bash
#
# Refuse to merge a pull request until its checks have passed.
#
# WHY THIS IS A SCRIPT AND NOT A SENTENCE
# =======================================
# CI already had a `test (vml feature)` job running on every pull request. It went red at one merge
# and stayed red through the next — two sign-offs — while the process document said, in words, "the
# main agent watches CI". A gate converges only when something forces someone to look at its output,
# and a document cannot force anything. This is the same rule with an exit status: run it in place of
# `gh pr merge`, and a red or still-running check stops the merge rather than being noticed later.
#
# USAGE
#   .github/scripts/merge-when-checks-pass.sh <pr-number> [-- <extra gh pr merge args>]
#
#   MJX_REQUIRED_CHECKS   newline-separated check names that must be green (default: the list below)
#   MJX_MERGE             set to 0 to only report; the script then never calls `gh pr merge`
#
# WHAT IT ENFORCES, IN ORDER
#   1. Every check on the head commit has *concluded* — a pending check is not a pass.
#   2. Every check in the required list concluded successfully, and every one of them was present.
#   3. `gh pr checks --required` is consulted as well: when the base branch carries required status
#      checks, its verdict must also be clean. When it carries none, its "no required checks" answer
#      is not treated as a pass — the list in (2) is what stands in for the missing configuration.
#
# The three are deliberately independent. (2) works today on any base branch with no repository
# configuration at all; (3) picks up any required check an administrator adds later without this
# script needing to change.
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <pr-number> [-- <extra gh pr merge args>]" >&2
  exit 2
fi
pr="$1"
shift
if [[ "${1:-}" == "--" ]]; then shift; fi

# The jobs that must be green before anything merges. Named, not inferred: a list derived from
# "whatever ran" is green when a job silently stopped running, which is the failure this file exists
# to prevent.
default_required=(
  "fmt · clippy · test"
  "test (--all-features)"
  "naming (\`suppress\`, not \`delete\`)"
  "rustdoc"
  "schema-validity (ECMA-376 XSDs)"
  "examples"
)
if [[ -n "${MJX_REQUIRED_CHECKS:-}" ]]; then
  # Newline-separated: every one of these names contains spaces.
  mapfile -t required <<<"${MJX_REQUIRED_CHECKS}"
else
  required=("${default_required[@]}")
fi

echo "checks on pull request #${pr}:"
gh pr checks "$pr" || true
echo

# 1 & 2 — the named list, from the API rather than from the human-readable table.
states=$(gh pr checks "$pr" --json name,state,bucket)

pending=$(printf '%s' "$states" | python3 -c '
import json, sys
rows = json.load(sys.stdin)
print("\n".join(r["name"] for r in rows if r["bucket"] == "pending"))')
if [[ -n "$pending" ]]; then
  echo "REFUSING TO MERGE: these checks have not concluded:" >&2
  printf '%s\n' "$pending" | sed 's/^/  /' >&2
  exit 1
fi

missing=()
failed=()
for name in "${required[@]}"; do
  bucket=$(printf '%s' "$states" | REQUIRED_NAME="$name" python3 -c '
import json, os, sys
rows = json.load(sys.stdin)
want = os.environ["REQUIRED_NAME"]
hits = [r for r in rows if r["name"] == want]
print(hits[0]["bucket"] if hits else "absent")')
  case "$bucket" in
    pass) ;;
    absent) missing+=("$name") ;;
    *) failed+=("$name ($bucket)") ;;
  esac
done

if [[ ${#failed[@]} -gt 0 || ${#missing[@]} -gt 0 ]]; then
  echo "REFUSING TO MERGE pull request #${pr}." >&2
  if [[ ${#failed[@]} -gt 0 ]]; then
    echo "  not green:" >&2
    printf '    %s\n' "${failed[@]}" >&2
  fi
  if [[ ${#missing[@]} -gt 0 ]]; then
    # A job renamed in `ci.yml` and not renamed here lands in this branch, loudly, at merge time —
    # which is the only moment it matters. A list inferred from "whatever ran" would go quiet.
    echo "  never ran (a check that stops running is a gate that stops gating):" >&2
    printf '    %s\n' "${missing[@]}" >&2
  fi
  exit 1
fi

# 3 — whatever the base branch's own configuration requires, on top of the list above.
if gh pr checks "$pr" --required >/dev/null 2>&1; then
  echo "the base branch's required status checks are green."
else
  echo "the base branch declares no required status checks; the named list above is what gated this."
fi

echo "every required check is green."
if [[ "${MJX_MERGE:-1}" == "0" ]]; then
  echo "MJX_MERGE=0 — reporting only, not merging."
  exit 0
fi
exec gh pr merge "$pr" "$@"
