#!/usr/bin/env python3
"""Derive the Office application-surface command inventory from Microsoft's published control IDs.

The inventory in `../OFFICE_FEATURE_INVENTORY.md` is not hand-written and not sourced from feature
marketing pages. It is derived from the two authoritative sources:

  1. **The command axis** — `OfficeDev/office-fluent-ui-command-identifiers`, Microsoft's own list of
     every control in every Office application's ribbon, backstage, context menus and QAT. This
     script reads the Word, Excel and PowerPoint workbooks for the Microsoft 365 Current Channel and
     summarises them by tab and group.
  2. **The format axis** — the ECMA-376 XSDs already in `References/`, counted by
     `schema_element_census.sh` beside this file.

Only the *summary* is committed, not Microsoft's raw data. Re-run this to refresh it:

    python3 derive_command_inventory.py --out ../command-surface.tsv

Requires network access; no third-party packages (stdlib zipfile + re only, because a `.xlsx` is a
ZIP of XML and this repository of all places should not need a dependency to read one).
"""

from __future__ import annotations

import argparse
import collections
import re
import sys
import urllib.request
import zipfile
from io import BytesIO

BASE = (
    "https://raw.githubusercontent.com/OfficeDev/office-fluent-ui-command-identifiers"
    "/main/Microsoft%20365/Current%20Channel"
)
APPS = {"Word": "wordcontrols.xlsx", "Excel": "excelcontrols.xlsx", "PowerPoint": "powerpointcontrols.xlsx"}

# Surfaces excluded by decision: these are third-party, service or platform integrations rather than
# the application's own document surface. Recorded here rather than silently dropped, so the
# exclusion stays reviewable. Matched as substrings against the group name.
EXCLUDED_GROUPS = (
    "OfficeExtension",      # add-ins
    "AIAssistance", "AIDemo", "Ideas", "Designer",   # Copilot / cloud intelligence
    "Collaborate", "SharePoint", "Broadcast",        # cloud collaboration surfaces
    "ClassifyLabelProtect", "Activation", "Syntex",  # tenant / licensing / MIP
    "PowerQuery", "PowerMap", "PowerBI", "PowerAutomate", "MSForms",
    "OfficeScripts", "ScriptingTools", "PythonChunk",
    "VoiceTools", "Speech", "LiveSubtitles", "ChineseTranslation",
    "Addins", "Code", "Xml", "Macros",               # VBA / automation
    "HelpAndSupport", "ExcelCommunity", "Insights", "Research", "Lineage",
    "Barcode", "InkConvertOneNote", "PensOneNote",
)
EXCLUDED_TABS = (
    "TabBlogPost", "TabBlogInsert",       # publishing to an external blog service
    "TabBroadcastPresentation",
    "TabAddIns", "HelpTab", "TabAutomate", "TabDeveloper", "TabSyntex",
    "TabConflicts", "TabMerge",           # server-side co-authoring conflict UI
)


def fetch(name: str) -> bytes:
    with urllib.request.urlopen(f"{BASE}/{name}", timeout=90) as response:
        return response.read()


def rows_of(blob: bytes) -> list[dict[str, str]]:
    """A `.xlsx` is a ZIP of XML; read the one worksheet and its shared strings."""
    archive = zipfile.ZipFile(BytesIO(blob))
    shared = archive.read("xl/sharedStrings.xml").decode("utf8", "replace")
    strings = [re.sub(r"<[^>]+>", "", s) for s in re.findall(r"<si>(.*?)</si>", shared, re.S)]
    sheet = archive.read("xl/worksheets/sheet1.xml").decode("utf8", "replace")

    def cells(row: str) -> list[str]:
        out = []
        for match in re.finditer(r"<c\b([^>]*)>(.*?)</c>", row, re.S):
            attrs, body = match.group(1), match.group(2)
            kind = re.search(r't="(\w+)"', attrs)
            value = re.search(r"<v>(.*?)</v>", body, re.S)
            if not value:
                out.append("")
            elif kind and kind.group(1) == "s":
                out.append(strings[int(value.group(1))])
            else:
                out.append(value.group(1))
        return out

    raw = re.findall(r"<row[^>]*>(.*?)</row>", sheet, re.S)
    header = cells(raw[0])
    return [dict(zip(header, cells(r) + [""] * len(header))) for r in raw[1:]]


def is_in_scope(row: dict[str, str]) -> bool:
    group, tab = row.get("Group/Context Menu Name", ""), row.get("Tab", "")
    if any(token in tab for token in EXCLUDED_TABS):
        return False
    return not any(token in group for token in EXCLUDED_GROUPS)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", default="-", help="TSV destination, or - for stdout")
    args = parser.parse_args()

    sink = sys.stdout if args.out == "-" else open(args.out, "w", encoding="utf8")
    print("app\ttab_set\ttab\tgroup\tcontrols\tin_scope", file=sink)

    for app, filename in APPS.items():
        rows = rows_of(fetch(filename))
        buckets: dict[tuple[str, str, str], list[dict[str, str]]] = collections.defaultdict(list)
        for row in rows:
            buckets[(row["Tab Set"], row["Tab"], row["Group/Context Menu Name"])].append(row)
        total = in_scope = 0
        for (tab_set, tab, group), members in sorted(buckets.items()):
            if not tab or group.isdigit() or not group:
                continue      # unnamed legacy controls carry a numeric id and no group
            scoped = is_in_scope(members[0])
            total += len(members)
            in_scope += len(members) if scoped else 0
            print(f"{app}\t{tab_set}\t{tab}\t{group}\t{len(members)}\t{int(scoped)}", file=sink)
        print(f"# {app}: {len(rows)} controls, {total} grouped, {in_scope} in application scope",
              file=sys.stderr)

    if sink is not sys.stdout:
        sink.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
