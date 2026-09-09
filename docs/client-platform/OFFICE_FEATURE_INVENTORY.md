# The Office feature inventory — what "complete" actually contains

> The implementation surface for the client platform, derived rather than recalled. This is the list
> the parity ledger is built from and the list the stage-2 execution loop turns into tickets.
>
> Scope note: **application surface only.** Third-party, service and platform integrations are
> excluded by decision and named in §2 so the exclusion stays reviewable rather than silent.

---

## 1 · Method — why this list is not a feature list

Searching for "Microsoft Word features" returns marketing pages and tutorial blogs. They are useless
for this purpose: they are unranked, incomplete, undated, and they describe *benefits* rather than
*surfaces*. A list built from them would look complete and quietly omit a third of the work.

There are exactly two authoritative enumerations of what Office does, and both are machine-readable:

**The command axis — what a user can invoke.** Microsoft publishes every control identifier in every
Office application: ribbon tabs, groups, galleries, context menus, backstage, and the Quick Access
Toolbar. It is maintained per release channel in
[`OfficeDev/office-fluent-ui-command-identifiers`](https://github.com/OfficeDev/office-fluent-ui-command-identifiers).
`data/derive_command_inventory.py` reads the Microsoft 365 Current Channel workbooks and summarises
them into `data/command-surface.tsv`.

**The format axis — what a file can contain.** The ECMA-376 XSDs, already present locally in
`References/`. Every declared element either has a visual consequence, an editing consequence, or is
preserved opaquely by an explicit decision. `data/schema_element_census.sh` counts them into
`data/schema-census.txt`.

Neither axis alone is sufficient. The command axis misses everything with no button — line breaking,
IME, hit-testing, autosave. The format axis misses everything with no markup — find and replace,
spell check, the clipboard. The inventory is their union plus a third, hand-written axis (§6) for
behaviour that appears in neither.

### The numbers, measured

| Application | Controls published | Grouped | **In application scope** | Excluded (§2) |
|---|---:|---:|---:|---:|
| Word | 5,638 | 4,240 | **3,934** | 306 |
| Excel | 4,926 | 4,033 | **3,680** | 353 |
| PowerPoint | 5,230 | 4,367 | **4,255** | 112 |
| | | | **11,869** | 771 |

| Format axis | Elements | Complex types | Simple types |
|---|---:|---:|---:|
| WordprocessingML (`wml`) | 784 | 285 | 110 |
| SpreadsheetML (`sml`) | 628 | 367 | 96 |
| PresentationML (`pml`) | 315 | 149 | 58 |
| DrawingML core (`dml-main`) | 475 | 231 | 94 |
| Charts (`dml-chart`) | 444 | 136 | 68 |
| Diagrams / SmartArt (`dml-diagram`) | 133 | 58 | 66 |
| Math (`shared-math`) | 177 | 72 | 14 |
| Drawing anchors (`wordprocessing` + `spreadsheet`) | 119 | 37 | 10 |
| VML (legacy, five schemas) | 131 | 54 | 43 |
| Bibliography, doc properties, custom XML | 148 | 26 | 6 |
| **Total declared elements** | **3,404** | | |

The 11,869 figure counts each application's surface separately and therefore double-counts the shared
contextual tab sets — chart, picture, drawing, SmartArt, SVG and WordArt tools appear in all three
applications with near-identical command sets. That overlap is not waste; it is the argument for
building those subsystems **once**, in the shared tier, which is what §3 does.

---

## 2 · Excluded by decision — the third-party surfaces

Dropped from scope on instruction: these are integrations with services, tenants, add-in hosts and
automation runtimes, not the application's own document surface. Recorded so that a later reader
knows they were considered and set aside, not overlooked.

| Excluded surface | Examples from the published controls |
|---|---|
| Add-in host | `GroupOfficeExtension`, `GroupOfficeExtensionsAddinFlyout`, `TabAddIns` |
| Cloud intelligence | `GroupAIAssistance`, `GroupAIDemo`, `GroupIdeas`, `GroupDesignerOptions` |
| Cloud collaboration | `GroupCollaborate`, `GroupSharePointProperties`, `TabBroadcastPresentation`, `TabConflicts`, `TabMerge` |
| Tenant / licensing / labelling | `GroupClassifyLabelProtect`, `GroupActivation`, `TabSyntex` |
| External data & BI | `GroupPowerQuery*` (233 controls in Excel alone), `GroupPowerMap`, `GroupPowerBI`, `TabSetPowerQueryEdit` |
| Automation runtimes | `TabDeveloper`, `TabAutomate`, `GroupOfficeScripts`, `GroupPythonChunk`, `GroupMacros`, `GroupCode`, `GroupXml` |
| Speech & translation services | `GroupVoiceTools`, `GroupLiveSubtitles`, `GroupChineseTranslation`, `GroupSpeech` |
| External publishing | `TabBlogPost`, `TabBlogInsert`, `GroupInsertBarcode` |
| Help & community | `HelpTab`, `GroupHelpAndSupport`, `GroupExcelCommunity`, `GroupInsights`, `GroupResearch` |

**Two exclusions are worth revisiting later, and are noted rather than settled.** *Mail merge*
(`TabMailings`, 51 controls) is an application feature whose data source happens to be external — the
merge engine itself belongs to Word. *Ink* (`TabDrawInk`, 32–52 controls per app) is a genuine
document feature: ink is stored in the file, and a stylus is a first-class input on the mobile target.
Both are excluded from the first pass only because they are self-contained enough to add later
without disturbing anything.

---

## 3 · Shared subsystems — build once, three consumers

The largest efficiency available in the whole programme. These appear in all three applications with
substantially the same command surface and identical markup, so each is one work package and not
three.

### 3.1 · Text and typography
Font family, size, colour, highlight, bold/italic/underline (17 underline styles), strikethrough,
subscript/superscript, small caps, all caps, character spacing and scaling, kerning, position,
ligatures and number forms, text effects (shadow, glow, reflection, bevel), styles, clear formatting,
format painter. Underneath: shaping, bidirectional text, script itemisation, line breaking,
hyphenation, East Asian typography (ruby, vertical text, line-break rules), grapheme-cluster
navigation.
*Evidence:* `GroupFont` — 43 controls in Word, 39 in Excel, 18 in PowerPoint.

### 3.2 · Paragraphs
Alignment including justification variants, indentation, spacing before/after and line spacing,
tab stops of five kinds, bullets and multilevel numbered lists with restart and continuation, borders
and shading, text direction, pagination controls (widow/orphan, keep with next, keep lines together,
page break before), sorting, show formatting marks.
*Evidence:* `GroupParagraph` — 56 in Word, 27 in PowerPoint.

### 3.3 · Drawing and shapes — `TabSetDrawingTools` (121–161 controls per app)
The 187 preset geometries with their adjustment handles, custom freeform geometry, connectors and
routing, fills (solid, gradient with full stop and tile semantics, 54 preset patterns, picture,
texture), outlines (weight, dash, cap, join, compound, arrowheads), effects (`outerShdw`, `innerShdw`,
`glow`, `softEdge`, `reflection`, `blur`, and the effect DAG), 3-D rotation and extrusion
(`a:scene3d` / `a:sp3d`), shape styles and theme style references, WordArt (`TabSetWordArtTools`),
text boxes (`TabSetTextBoxTools`), grouping and ungrouping, z-order, alignment and distribution,
snapping and guides, rotation and flipping, size and position, locking, alt text.
*Evidence:* `GroupDrawing` in PowerPoint — 63 controls; `GroupArrange` — 65 in Word, 47 in Excel.

### 3.4 · Pictures — `TabSetPictureTools` (114–151 per app)
Insert, crop (including crop-to-shape and aspect fill), corrections (brightness, contrast,
sharpness), colour (saturation, tone, recolour, transparency, set transparent colour), artistic
effects, picture styles and borders, background removal (`TabBackgroundRemoval`), compression, reset,
change picture, alt text, and the anchoring models — inline, floating, one-cell, two-cell, absolute.

### 3.5 · Charts — `TabSetChartTools` (375–410 per app)
The single largest shared subsystem. ~73 chart types across 17 families, plot-area layout, axes
(category, value, date, secondary) with scales, ticks, and 40+ number-format behaviours, series and
data-point formatting, data labels, legends, gridlines, trendlines, error bars, up/down bars,
high/low lines, drop lines, chart styles and colour sets, the embedded workbook that backs the data
(`Edit Data`), chart templates, and combo charts. `dml-chart` is 444 elements.

### 3.6 · SmartArt / diagrams — `TabSetSmartArtTools` (145–177 per app)
~200 layouts across eight categories, the four-part model (data, layout, style, colours), layout
algorithm evaluation, text pane editing, promote/demote/reorder, shape and colour variants,
conversion to shapes or text. `dml-diagram` is 133 elements.

### 3.7 · Mathematics — `TabSetEquationTools`
`shared-math` is 177 elements: fractions, radicals, n-ary operators, integrals, matrices, delimiters,
accents, functions, limits, boxed and bordered expressions, equation arrays, professional/linear/
Unicode-math forms, and the layout engine to typeset them. Closer to TeX than to prose layout.

### 3.8 · Tables — `TabSetTableTools`
Structure (insert, delete, merge, split, split table), sizing (row height, column width, autofit,
distribute), alignment and cell margins, borders and shading with resolution precedence, table styles
with the six conditional-formatting bands, header-row repetition, text direction in cells, and
table-to-text conversion.

### 3.9 · Cross-cutting document surfaces
Themes (colour schemes, font schemes, effect schemes, theme variants), styles and style sets, the
clipboard with its four flavours and paste-special, find and replace including formatting and
wildcards, spell check and grammar, language and proofing settings, accessibility checking and alt
text, comments (both legacy and threaded), document properties and metadata, protection, versions and
autosave, print and page setup, zoom and view modes, undo and redo.
*Evidence:* `GroupClipboard` 10–12 per app; `GroupComments` 6–16; `GroupAccessibility` 6–9.

---

## 4 · Per-application surfaces

### 4.1 · Word — 3,934 commands in scope

| Tab | In scope | Principal groups |
|---|---:|---|
| `TabHome` | 165 | Paragraph 56, Font 43, Editing 23, Clipboard 12, Styles 5 |
| `TabInsert` | 160 | Header/Footer 32, Text 26, Illustrations 23, Pages 13, Tables 7, Symbols 7, Links 5 |
| `TabReviewWord` | 111 | Track Changes 22, Comments 16, Language 14, Changes 14, Compare 8, Proofing 8, Accessibility 6, Protect 4 |
| `TabPageLayoutWord` | 98 | Arrange 65, Page Setup 23, Paragraph Layout 7 |
| `TabReferences` | 44 | Footnotes 9, Citations & Bibliography 8, Table of Contents 6, Captions 4, Index 3, Table of Authorities 3 |
| `TabView` | 43 | Window 8, Zoom 6, Document Views 5, Show/Hide 4, Modes 3, Page Movement 2 |
| `TabWordDesign` | 27 | Style Set 16, Page Background 9 |
| `TabPrintPreview` | 24 | Page Setup 6, Zoom 6, Preview 6, Print 2 |
| `TabOutlining` | 24 | Outlining Tools 12, Master Document 8 |
| `TabHeaderAndFooterTools` | contextual | header/footer editing |

**The layout engine behind those commands** — the deepest work in the programme, and mostly invisible
in the command list:

- **Flow and pagination**: line breaking and justification, page and column breaks, widow/orphan
  control, keep-with-next and keep-lines-together, balanced columns.
- **Sections**: page size and orientation, margins and gutters, multiple columns, different
  first-page and odd/even headers and footers, line numbers, vertical alignment, section-scoped
  page numbering.
- **Floating objects and text wrapping**: `square`, `tight`, `through`, `topAndBottom`, `behind`,
  `inFrontOf`, with real wrap polygons, distance-from-text, and "wrap in the larger side only".
  Consistently the most under-estimated feature in any Word renderer.
- **Tables**: splitting across pages, repeating header rows, nested tables, row and column spans,
  fixed and autofit layout algorithms, floating tables with their own wrapping.
- **Footnotes and endnotes**: their own reflow, separators, continuation notices, numbering schemes,
  and the interaction with pagination that makes them genuinely hard.
- **Fields**: 90+ field types, calculated and updated, including `TOC`, `INDEX`, `REF`, `PAGE`,
  `SEQ`, `STYLEREF`, `IF`, `MERGEFIELD`, `CITATION`, cross-references and their update semantics.
- **Numbering**: multilevel lists, restart rules, list style inheritance, legal numbering.
- **Track changes**: insertions, deletions, formatting revisions, moves, and their effect on layout
  in each of the four display modes.
- **Content controls**, bookmarks, hyperlinks, drop caps, text frames, watermarks, hyphenation,
  and the East Asian typographic rule set.

### 4.2 · Excel — 3,680 commands in scope

| Tab | In scope | Principal groups |
|---|---:|---|
| `TabHome` | 233 | Editing 45, Font 39, Cells 39, Styles 37, Alignment 27, Clipboard 12, Number 11 |
| `TabInsert` | 121 | Illustrations 28, Charts 25, Tables 17, Text 10, Symbols 4, Sparklines 3, Slicers 2 |
| `TabFormulas` | 108 | Function Library 37, Formula Auditing 13, Named Cells 7, Calculation 7 |
| `TabPageLayoutExcel` | 91 | Arrange 47, Page Setup 16, Themes 9, Sheet Options 8, Scale to Fit 6 |
| `TabData` | ~79 in scope | Data Tools 10, Outline 10, Sort & Filter 7, Forecast 5 (Power Query excluded) |
| `TabReview` | 67 | Accessibility 9, Changes 8, Comments 6+7+5, Ink 5, Protect 4 |
| `TabView` | 44 | Window 10, Show/Hide 8, Named Sheet Views 5, Workbook Views 4, Zoom 3 |
| `TabSetPivotTableTools` | 136 | pivot tables |
| `TabSetPivotChartTools` | 434 | pivot charts |

**The engines behind those commands:**

- **The number-format engine** — `numFmt` to display string: all built-in formats plus arbitrary
  custom codes, four conditional sections, colour codes, locale and calendar awareness, both the
  1900 and 1904 date systems, fraction and scientific forms, text placeholders, and Excel's
  15-significant-digit display rule. A sub-project on its own.
- **The calculation engine** — see §5.
- **Grid layout** — row and column sizing including autofit, hidden rows and columns, merged regions,
  text overflow into empty neighbours and the rules that clip it, wrap and shrink-to-fit, frozen and
  split panes, right-to-left sheets.
- **Conditional formatting** — rule evaluation and precedence, data bars, colour scales, icon sets,
  top/bottom and above/below-average rules, duplicate rules, formula-driven rules, stop-if-true.
- **Tables, filters and sorting** — structured references, autofilter with all its criteria kinds,
  slicers, timelines, custom sorts, subtotals and outlining.
- **Data tools** — text to columns, flash fill, remove duplicates, data validation with all its
  types and input/error messages, consolidate, what-if analysis (goal seek, scenarios, data tables),
  forecast sheets.
- **Print layout** — page breaks and break preview, print areas, repeated rows and columns, scaling,
  headers and footers, sheet options.
- **Sparklines** (`TabSetSparkline`), cell drawings and their three anchor models, comments and notes,
  defined names, workbook and worksheet protection, sheet views.

### 4.3 · PowerPoint — 4,255 commands in scope

| Tab | In scope | Principal groups |
|---|---:|---|
| `TabHome` | 166 | Drawing 63, Paragraph 27, Font 18, Slides 16, Clipboard 10, Editing 8 |
| `TabSlideMasterHome` | 141 | the same surface, applied to masters |
| `TabInsert` | 93 | Illustrations 11, Images 10, Text 9, Media Clips 9, Links 7, Slides 7, Tables 4 |
| `TabReview` | 62 | Compare 13, Comments 12, Language 9, Accessibility 6, Ink 5 |
| `TabRecording` | 46 | Record 8, Edit 6, Media Auto-play 5, Content 3, Save 3, Export 2 |
| `TabSlideMaster` | 44 | Master Layout 15, Background 11, Edit Master 5, Theme 4, Slide Size 2 |
| `TabSlideShow` | 43 | Setup 16, Start 9, Monitors 2, Rehearse 1 |
| `TabView` | 37 | Window 6, Presentation Views 5, Show/Hide 5, Colour/Grayscale 4, Master Views 3 |
| `TabAnimations` | 36 | Animations 12, Custom Animation 11, Timing 6, Preview 3 |
| `TabHandoutMaster` | 36 | Page Setup 11, Background 11, Placeholders 4, Theme 4 |
| `TabNotesMaster` | 30 | Background 11, Placeholders 6, Theme 4, Page Setup 3 |
| `TabDesign` | 22 | Theme Variants 10, Themes 4, Customize 3 |
| `TabTransitions` | 14 | Transition Styles 8, Timing 2 |
| `TabSetVideoTools` / `TabSetAudioTools` | 127 / 156 | media playback, trimming, poster frames, bookmarks |

**The engines behind those commands:**

- **Animation and timing** (`p:timing`) — not modelled in the core today. Entrance, emphasis, exit and
  motion-path effects; the trigger and condition tree; sequence and parallel time nodes; the animation
  pane; effect options; timing (start, delay, duration, repeat, rewind); motion-path editing; and the
  runtime that evaluates the timeline into per-frame property deltas.
- **Transitions** — ~48 transition types with their effect options, duration, advance-on-click and
  advance-after, and sound.
- **The master/layout/notes/handout hierarchy** — placeholder inheritance, layout authoring, master
  editing, slide size and scaling, background styles and fills, per-layout theme overrides.
- **Slide show** — presenter view, rehearsal and recorded timings, custom shows, hidden slides,
  annotation ink during a show, monitor selection, laser pointer.
- **Media** — audio and video playback, trimming, fade, bookmarks, poster frames, looping, auto-play,
  and the recording surface.
- **Sections**, slide sorter, zoom links, morph, designer-free layout tooling, comparison and merge.

---

## 5 · The calculation engine

Its own programme, as decided. Dependency graph over cells and ranges with cycle detection and
iterative calculation; dirty propagation and minimal recalculation; the ~500 built-in functions of
`GroupFunctionLibrary` (37 controls, each a category menu); array formulas and modern dynamic arrays
with spill ranges and the implicit-intersection operator; volatile functions; cross-sheet and
cross-workbook references; defined names and structured table references; the full error lattice and
its propagation; and Excel's own numeric behaviour, which must be bug-compatible rather than merely
correct.

It gates Excel *editing* only. Rendering an unedited workbook uses the cached `<v>` already in the
file, so the viewer and the engine can proceed in parallel.

---

## 6 · The third axis — behaviour with neither markup nor a button

Absent from both enumerations, and roughly a third of the perceived quality of the product.

- **Selection and caret** — bidi-aware caret placement and movement, grapheme-cluster granularity,
  word and paragraph expansion, multi-range selection, selection across table cells and shapes,
  autoscroll at the viewport edge.
- **Hit testing** — spatial indexing, z-order resolution, tolerance zones, handle targets sized for
  touch versus pointer.
- **Input** — mouse, trackpad momentum and pinch, touch gestures, pen with pressure/tilt/palm
  rejection, the full keyboard shortcut map, and **IME composition** with a candidate window
  positioned from the caret.
- **Undo and redo** — command granularity, coalescing of consecutive keystrokes, selection restoration.
- **Editing feel** — drag with snapping and alignment guides, live resize and rotate previews,
  marching ants, drop indicators, smooth scrolling and fling, progressive render during a fling.
- **Document lifecycle** — open, autosave, crash recovery, dirty tracking, external-change detection,
  large-file progressive open.
- **Accessibility** — a tree derived from the fragment tree, screen-reader semantics, keyboard-only
  operation, focus management, reduced motion, high contrast.
- **Responsive chrome** — desktop, tablet and phone presentations of the same commands, and the
  command-demotion rules that decide what survives into a phone toolbar.
- **Localisation** — UI strings, RTL mirroring of the entire chrome, locale-aware numbers and dates,
  and locale-specific typographic rules.

---

## 7 · How this becomes the ledger

**It has.** [`PARITY_LEDGER.md`](PARITY_LEDGER.md) is that ledger, and it is **generated from this
workspace's own test suites** by `cargo run -p xtask -- ledger` (MJXOFF-179) — never written by
hand. A row there declares which suites are its evidence and nothing else; the state is derived from
what those suites contain, so a suite that is deleted, renamed or emptied moves the row rather than
leaving it stale. `ledger --check` refuses if the committed document is not what the tree produces,
and `xtask/tests/ledger.rs` runs that check.

Every row above is a ledger entry with one of four states: `implemented`, `partial`,
`preserved-not-rendered`, `not-started`. Three rules keep it honest:

1. **A state is produced by a test, never by assertion.** The fidelity oracle's suites generate the
   ledger; anything nothing tests is `not-started` regardless of what anyone believes.
2. **`preserved-not-rendered` is a legitimate, permanent state** for markup the core round-trips
   faithfully but the renderer does not draw. It is how "no holes" stays truthful without pretending
   every element is drawn on day one.
3. **The exclusions of §2 are ledger rows too**, in a fifth state — `out-of-scope` — with the reason
   attached, so a later reader can reopen the decision instead of rediscovering the gap.

Regenerate the two derived inputs with:

```sh
python3 docs/client-platform/data/derive_command_inventory.py --out docs/client-platform/data/command-surface.tsv
bash docs/client-platform/data/schema_element_census.sh References > docs/client-platform/data/schema-census.txt
```

## Sources

- [OfficeDev/office-fluent-ui-command-identifiers](https://github.com/OfficeDev/office-fluent-ui-command-identifiers) — the published control identifiers, Microsoft 365 Current Channel
- [Office 2016 Help Files: Office Fluent User Interface Control Identifiers](https://www.microsoft.com/en-us/download/details.aspx?id=50745) — the earlier published form of the same data
- [Overview of the Office Fluent ribbon](https://learn.microsoft.com/en-us/office/vba/library-reference/concepts/overview-of-the-office-fluent-ribbon) — the tab/group/control model the taxonomy above follows
- ECMA-376 5th edition Parts 1 and 4, local in `References/`
