/**
 * **Every tab of every ribbon, as the committed command surface records it** — the table the
 * `Ribbons/*` catalogue and the three shells are both built from.
 *
 * ## Why this file exists, and why it is checked against the source
 *
 * The same doctrine `dev/word-tab-home.ts` is written under, applied to all three applications at
 * once: a structure somebody typed out by hand is a structure somebody *chose*, and it drifts the
 * moment the census is re-derived. So the tab ids, the group ids and the control counts below are
 * transcribed from `docs/client-platform/data/command-surface.tsv`, and `tests/ribbons.test.ts`
 * **reads that file and fails if they disagree**. The transcription exists at all because `ui/` is
 * a Node workspace that must not grow a build step reading a document tree, and because a story
 * cannot read a TSV at render time without one.
 *
 * ## The two sources, and which one wins
 *
 * The TSV says *which groups exist* and *how many controls each holds*; `OFFICE_FEATURE_INVENTORY.md`
 * §4.1–4.3 says what to **call** them in English. Where the inventory names a group, that name is
 * used verbatim; where it does not, the census id is self-describing and the label is derived from
 * it (`GroupPens2` → Pens, `GroupWrite` → Write). The counts in the inventory's prose disagree with
 * the TSV in several places and **the TSV wins**, exactly as `dev/word-tab-home.ts` records.
 *
 * ## ⚠ Three places the census and Office disagree, recorded rather than smoothed over
 *
 * 1. **Excel's File tab has no Print.** Word and PowerPoint both carry a backstage `TabPrint` row
 *    (`GroupPrintSettings`, one control); Excel carries none, and carries a `Publish2Tab` the
 *    other two do not. Excel obviously *has* a File → Print page, so this is a gap in the census
 *    dump rather than a fact about Excel — but the census is the checked source and inventing a
 *    row here would be the drift this file exists to prevent. So Excel's File tab is Info, Open,
 *    Save, Share, Export, Publish, Help, and this paragraph is the record.
 * 2. **The approved plan's scale counts are approximate.** It says *"Word 12 tabs / 66 groups,
 *    PowerPoint 14 / 82, Excel 10 / 67"*. Filtering the TSV on `tab_set = "None (Core Tab)"` and
 *    `in_scope = 1` gives **Word 12 / 68, PowerPoint 18 / 96, Excel 10 / 67**. The filter is the
 *    thing a test can run, so the filter wins and every in-scope core tab is declared here.
 * 3. **Two of Word's Draw groups carry Excel's ids** (`GroupEditingExcel`), and PowerPoint's
 *    Recording tab carries both a `GroupRecord` and a `GroupRecordTabRecord`. Both are the
 *    census's own spellings, transcribed unchanged; only the English labels disambiguate.
 *
 * ## The priority rubric — and it is a rubric rather than a per-group opinion
 *
 * There are ~230 groups here, and a priority decided ad hoc 230 times is not a design. Each group
 * declares one of `mjx_ribbon`'s four priorities by this rule:
 *
 * | Priority | The group it is for |
 * |---|---|
 * | `primary` | **The group the tab exists for.** Word's Font and Paragraph on Home. At most two per tab, or the word means nothing. |
 * | `standard` | The default. A group reached for often but not constantly. |
 * | `secondary` | **Reachable elsewhere** — the keyboard, a context menu, the status bar, the mini toolbar. Clipboard is the archetype: its four verbs are on the keyboard anyway. |
 * | `ancillary` | Two controls or fewer, or an add-in / Copilot / telemetry group, or a group the census marks out of scope. Never anybody's reason for opening the tab. |
 *
 * `tests/ribbons.test.ts` asserts three consequences of that table rather than trusting it: no tab
 * declares more than two `primary` groups, no group of two controls or fewer is `primary` or
 * `standard`, and an out-of-scope group is `ancillary`. The third binds nothing today — the
 * scaffold declares only the rows the census marks in scope — and it is written because the rule
 * has to be in force *before* the first Copilot group arrives, not after.
 *
 * ## What a command is here, and what it deliberately is not
 *
 * The approved plan is explicit that the ribbons are **template-driven, not data-driven**: a data
 * union covering split buttons, galleries, colour pickers and font pickers would be a renderer
 * re-expressing every component's public API, which is a second API to keep in step.
 * `RibbonCommand` below is therefore the *small* half — a button or a toggle, and nothing else.
 * Anything richer is supplied by the host as a `TemplateResult` keyed by the command's `id`
 * (`stories/ribbons/ribbon-parts.ts`'s `ControlOverrides`), which is how the shells keep their own
 * bindings — this machine's font list, this document's palette, the paste menu's id — while the
 * structure lives here.
 *
 * The `id` is the seam and it is stable: `<app>.<tab>.<group>.<command>`, lower case, so a shell
 * and a catalogue story can bind the same command two different ways without either of them
 * knowing what the other did.
 *
 * ## Only Home carries commands yet
 *
 * Unit 0 is the scaffold. The three Home tabs hold the commands migrated out of
 * `stories/shell/*.stories.ts`, unchanged; every other tab is a placeholder until its own unit.
 * That is why `commands` is optional rather than required — an empty array would claim a tab had
 * been authored and found to hold nothing.
 *
 * ## Node-importable
 *
 * Data only, for the reason `src/harness/presets.ts` states: `tests/ribbons.test.ts` runs in Node.
 */

import type { ControlSize } from '../../src/controls/control-states.ts';
import type { GroupPriority } from '../../src/ribbon/ribbon-model.ts';

// ── the vocabulary ───────────────────────────────────────────────────────────

/** The three applications, in the order the `Ribbons` section shows them. */
export const ribbonApplicationNames = ['word', 'powerpoint', 'excel'] as const;

/** One of the three. */
export type RibbonApplication = (typeof ribbonApplicationNames)[number];

/**
 * Whether Office shows a tab in the ordinary strip, or only inside a particular view.
 *
 * ⚠ **The census's "core tabs" are not all of them tabs a person ever sees at once.**
 * `TabSlideMaster`, `TabSlideMasterHome`, `TabHandoutMaster`, `TabNotesMaster`, `TabBlackAndWhite`
 * and `TabGrayscale` appear only in the view they name; `TabPrintPreview` and
 * `TabBackgroundRemoval` do the same in all three applications. A shell that put PowerPoint's eight
 * into its default strip would be showing a ribbon Office never shows, so `<app>Tabs()` takes a
 * parameter and the shells ask for `always` alone. The catalogue still gives each of them a story,
 * because a tab nobody can look at cannot be audited.
 */
export const tabAppearanceNames = ['always', 'view'] as const;

/** One of the two. */
export type TabAppearance = (typeof tabAppearanceNames)[number];

/**
 * One command on a ribbon's face — **a button or a toggle, and nothing else.**
 *
 * See the header: anything that opens a menu, shows a gallery or carries a palette is a host's
 * `TemplateResult` override keyed by `id`, not a wider union here.
 */
export interface RibbonCommand {
  /** `<app>.<tab>.<group>.<command>`. The key a host binds an override to. Stable. */
  readonly id: string;
  /** The command's name, as Office writes it. Also the accessible name at `size: 'icon'`. */
  readonly label: string;
  /** A name from `src/icons/manifest.ts`, or `undefined` for a text-only command. */
  readonly icon?: string;
  /** Defaults to `small`, which is what a group's secondary commands use. */
  readonly size?: ControlSize;
  /** A state rather than a verb. Rendered as `<mjx-toggle-button>`, always `slot="essential"`. */
  readonly toggle?: boolean;
  /** A toggle that starts on, so the ribbon shows a pressed state without a pointer. */
  readonly pressed?: boolean;
  /** A one-shot command that survives a collapse. See `demotionRules`. Ignored for a toggle. */
  readonly essential?: boolean;
}

/** One group of one tab. */
export interface RibbonGroupEntry {
  /** The group's id in the census — `GroupFont`. For the File tab, a backstage *tab* id. */
  readonly id: string;
  /** What the ribbon draws. */
  readonly label: string;
  /** See the rubric in this file's header. */
  readonly priority: GroupPriority;
  /** The census's control count. The modules do **not** render this many — see `tests/ribbons.test.ts`. */
  readonly controls: number;
  /** Whether the census marks the group in scope for this project. */
  readonly inScope: boolean;
  /** The commands the group shows, when its unit has authored them. */
  readonly commands?: readonly RibbonCommand[];
}

/**
 * Where a tab's groups come from in the census.
 *
 * Two shapes because the File tab is two shapes: **decision 1 of the approved plan makes File an
 * ordinary ribbon tab** rather than a backstage surface, so its *groups* are the backstage
 * *destinations* — Info, Open (`TabRecent`), Save, Print, Share, Export (`TabPublish`), Help — and
 * a group entry's `id` therefore names a census tab rather than a census group. The discriminant
 * is what lets one test assert a genuine equality for both.
 */
export type RibbonTabSource =
  /** One core tab. A group entry's `id` is a census group id inside it. */
  | { readonly kind: 'core'; readonly tab: string }
  /** The File tab. A group entry's `id` is a backstage census *tab* id. */
  | { readonly kind: 'backstage' };

/** One tab of one application's ribbon. */
export interface RibbonTabEntry {
  /** Kebab case, and the `tab-id` the ribbon is driven by — `page-layout`. */
  readonly id: string;
  /** What the tab strip draws. */
  readonly label: string;
  readonly appearance: TabAppearance;
  readonly source: RibbonTabSource;
  readonly groups: readonly RibbonGroupEntry[];
}

/** Which file the rows below were transcribed from, and under which filters. */
export const ribbonCensusSource = {
  file: 'docs/client-platform/data/command-surface.tsv',
  coreTabSet: 'None (Core Tab)',
  backstageTabSet: 'None (Backstage View)',
  /** The census's spelling of each application's name, which is not the kebab id. */
  app: { word: 'Word', powerpoint: 'PowerPoint', excel: 'Excel' },
} as const;

// ── the commands Home shows ──────────────────────────────────────────────────
//
// Migrated out of `stories/shell/*.stories.ts` unchanged, in the order the shells rendered them.
// Anything absent from a list below is absent from the shell today too — Word's Editor group,
// PowerPoint's Slides, Excel's Cells — and belongs to unit 2, which authors Home properly.

const wordHomeClipboard: readonly RibbonCommand[] = [
  { id: 'word.home.clipboard.paste', label: 'Paste', icon: 'clipboard-paste', size: 'large', essential: true },
  { id: 'word.home.clipboard.cut', label: 'Cut', icon: 'cut' },
  { id: 'word.home.clipboard.copy', label: 'Copy', icon: 'copy' },
  { id: 'word.home.clipboard.format-painter', label: 'Format Painter', icon: 'settings' },
];

const wordHomeFont: readonly RibbonCommand[] = [
  { id: 'word.home.font.name', label: 'Font' },
  { id: 'word.home.font.size', label: 'Font size' },
  { id: 'word.home.font.bold', label: 'Bold', icon: 'text-bold', toggle: true },
  { id: 'word.home.font.italic', label: 'Italic', icon: 'text-italic', toggle: true, pressed: true },
  { id: 'word.home.font.underline', label: 'Underline', icon: 'text-underline', toggle: true },
  { id: 'word.home.font.colour', label: 'Font colour' },
];

const wordHomeParagraph: readonly RibbonCommand[] = [
  { id: 'word.home.paragraph.align-left', label: 'Align left', icon: 'text-align-left', toggle: true, pressed: true },
  { id: 'word.home.paragraph.centre', label: 'Centre', icon: 'text-align-center', toggle: true },
  { id: 'word.home.paragraph.align-right', label: 'Align right', icon: 'text-align-right', toggle: true },
  { id: 'word.home.paragraph.bullets', label: 'Bullets', icon: 'add' },
  { id: 'word.home.paragraph.numbering', label: 'Numbering', icon: 'subtract' },
  { id: 'word.home.paragraph.borders', label: 'Borders', icon: 'table' },
];

const wordHomeStyles: readonly RibbonCommand[] = [
  { id: 'word.home.styles.gallery', label: 'Styles' },
];

const wordHomeEditing: readonly RibbonCommand[] = [
  { id: 'word.home.editing.find', label: 'Find', icon: 'search', essential: true },
  { id: 'word.home.editing.replace', label: 'Replace', icon: 'arrow-redo' },
  { id: 'word.home.editing.select', label: 'Select', icon: 'checkmark' },
];

const powerpointHomeClipboard: readonly RibbonCommand[] = [
  { id: 'powerpoint.home.clipboard.paste', label: 'Paste', icon: 'clipboard-paste', size: 'large', essential: true },
  { id: 'powerpoint.home.clipboard.cut', label: 'Cut', icon: 'cut' },
  { id: 'powerpoint.home.clipboard.copy', label: 'Copy', icon: 'copy' },
  { id: 'powerpoint.home.clipboard.format-painter', label: 'Format Painter', icon: 'settings' },
];

const powerpointHomeFont: readonly RibbonCommand[] = [
  { id: 'powerpoint.home.font.name', label: 'Font' },
  { id: 'powerpoint.home.font.size', label: 'Font size' },
  { id: 'powerpoint.home.font.bold', label: 'Bold', icon: 'text-bold', toggle: true, pressed: true },
  { id: 'powerpoint.home.font.italic', label: 'Italic', icon: 'text-italic', toggle: true },
  { id: 'powerpoint.home.font.underline', label: 'Underline', icon: 'text-underline', toggle: true },
  { id: 'powerpoint.home.font.colour', label: 'Font colour' },
];

const powerpointHomeParagraph: readonly RibbonCommand[] = [
  { id: 'powerpoint.home.paragraph.align-left', label: 'Align left', icon: 'text-align-left', toggle: true, pressed: true },
  { id: 'powerpoint.home.paragraph.centre', label: 'Centre', icon: 'text-align-center', toggle: true },
  { id: 'powerpoint.home.paragraph.align-right', label: 'Align right', icon: 'text-align-right', toggle: true },
  { id: 'powerpoint.home.paragraph.bullets', label: 'Bullets', icon: 'add' },
  { id: 'powerpoint.home.paragraph.numbering', label: 'Numbering', icon: 'subtract' },
  { id: 'powerpoint.home.paragraph.smart-art', label: 'Convert to SmartArt', icon: 'table' },
];

const powerpointHomeDrawing: readonly RibbonCommand[] = [
  { id: 'powerpoint.home.drawing.styles', label: 'Shape styles' },
  { id: 'powerpoint.home.drawing.arrange', label: 'Arrange', icon: 'slide-layout' },
];

const powerpointHomeEditing: readonly RibbonCommand[] = [
  { id: 'powerpoint.home.editing.find', label: 'Find', icon: 'search', essential: true },
  { id: 'powerpoint.home.editing.select', label: 'Select', icon: 'checkmark' },
];

const excelHomeClipboard: readonly RibbonCommand[] = [
  { id: 'excel.home.clipboard.paste', label: 'Paste', icon: 'clipboard-paste', size: 'large', essential: true },
  { id: 'excel.home.clipboard.cut', label: 'Cut', icon: 'cut' },
  { id: 'excel.home.clipboard.copy', label: 'Copy', icon: 'copy' },
];

const excelHomeFont: readonly RibbonCommand[] = [
  { id: 'excel.home.font.name', label: 'Font' },
  { id: 'excel.home.font.size', label: 'Font size' },
  { id: 'excel.home.font.bold', label: 'Bold', icon: 'text-bold', toggle: true },
  { id: 'excel.home.font.italic', label: 'Italic', icon: 'text-italic', toggle: true },
  { id: 'excel.home.font.fill', label: 'Fill colour' },
];

const excelHomeAlignment: readonly RibbonCommand[] = [
  { id: 'excel.home.alignment.align-left', label: 'Align left', icon: 'text-align-left', toggle: true },
  { id: 'excel.home.alignment.centre', label: 'Centre', icon: 'text-align-center', toggle: true, pressed: true },
  { id: 'excel.home.alignment.align-right', label: 'Align right', icon: 'text-align-right', toggle: true },
  { id: 'excel.home.alignment.merge', label: 'Merge & Centre', icon: 'table' },
  { id: 'excel.home.alignment.wrap', label: 'Wrap Text', icon: 'arrow-down-right' },
];

const excelHomeNumber: readonly RibbonCommand[] = [
  { id: 'excel.home.number.format', label: 'Number format' },
  { id: 'excel.home.number.increase-decimal', label: 'Increase decimal', icon: 'add' },
  { id: 'excel.home.number.decrease-decimal', label: 'Decrease decimal', icon: 'subtract' },
];

const excelHomeStyles: readonly RibbonCommand[] = [
  { id: 'excel.home.styles.gallery', label: 'Cell styles' },
];

const excelHomeEditing: readonly RibbonCommand[] = [
  { id: 'excel.home.editing.autosum', label: 'AutoSum', icon: 'add', essential: true },
  { id: 'excel.home.editing.sort-filter', label: 'Sort & Filter', icon: 'arrow-down-right' },
  { id: 'excel.home.editing.find-select', label: 'Find & Select', icon: 'search' },
];

// ── the tabs ─────────────────────────────────────────────────────────────────

export const wordRibbonTabs: readonly RibbonTabEntry[] = [
  {
    id: 'file',
    label: 'File',
    appearance: 'always',
    source: { kind: 'backstage' },
    groups: [
      { id: 'TabInfo', label: 'Info', priority: 'standard', controls: 8, inScope: true },
      { id: 'TabRecent', label: 'Open', priority: 'primary', controls: 67, inScope: true },
      { id: 'TabSave', label: 'Save', priority: 'primary', controls: 60, inScope: true },
      { id: 'TabPrint', label: 'Print', priority: 'secondary', controls: 1, inScope: true },
      { id: 'TabShare', label: 'Share', priority: 'standard', controls: 21, inScope: true },
      { id: 'TabPublish', label: 'Export', priority: 'standard', controls: 8, inScope: true },
      { id: 'TabHelp', label: 'Help', priority: 'ancillary', controls: 6, inScope: true },
    ],
  },
  {
    id: 'home',
    label: 'Home',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabHome' },
    groups: [
      { id: 'GroupClipboard', label: 'Clipboard', priority: 'secondary', controls: 12, inScope: true, commands: wordHomeClipboard },
      { id: 'GroupFont', label: 'Font', priority: 'primary', controls: 43, inScope: true, commands: wordHomeFont },
      { id: 'GroupParagraph', label: 'Paragraph', priority: 'primary', controls: 56, inScope: true, commands: wordHomeParagraph },
      { id: 'GroupStyles', label: 'Styles', priority: 'standard', controls: 5, inScope: true, commands: wordHomeStyles },
      { id: 'GroupEditing', label: 'Editing', priority: 'ancillary', controls: 23, inScope: true, commands: wordHomeEditing },
      { id: 'GroupEditor', label: 'Editor', priority: 'secondary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'insert',
    label: 'Insert',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabInsert' },
    groups: [
      { id: 'GroupInsertPages', label: 'Pages', priority: 'standard', controls: 13, inScope: true },
      { id: 'GroupInsertTables', label: 'Tables', priority: 'primary', controls: 7, inScope: true },
      { id: 'GroupInsertIllustrations', label: 'Illustrations', priority: 'primary', controls: 23, inScope: true },
      { id: 'GroupMedia', label: 'Media', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupInsertLinks', label: 'Links', priority: 'standard', controls: 5, inScope: true },
      { id: 'GroupInsertComments', label: 'Comments', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupHeaderFooter', label: 'Header & Footer', priority: 'standard', controls: 32, inScope: true },
      { id: 'GroupInsertText', label: 'Text', priority: 'standard', controls: 26, inScope: true },
      { id: 'GroupInsertSymbols', label: 'Symbols', priority: 'standard', controls: 7, inScope: true },
    ],
  },
  {
    id: 'draw',
    label: 'Draw',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabDrawInk' },
    groups: [
      { id: 'GroupDrawingTools', label: 'Drawing Tools', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupPens2', label: 'Pens', priority: 'primary', controls: 6, inScope: true },
      { id: 'GroupWrite', label: 'Write', priority: 'primary', controls: 10, inScope: true },
      { id: 'GroupStencils', label: 'Stencils', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupEditingExcel', label: 'Editing', priority: 'standard', controls: 6, inScope: true },
      { id: 'GroupInsertDrawingCanvas', label: 'Drawing Canvas', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupInputMode', label: 'Input Mode', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupDrawWithTouch', label: 'Draw with Touch', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'InkReplay', label: 'Replay', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupPenAndInkHelp', label: 'Help', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupInkClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'design',
    label: 'Design',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabWordDesign' },
    groups: [
      { id: 'GroupStyleSet', label: 'Style Set', priority: 'primary', controls: 16, inScope: true },
      { id: 'GroupPageBackground', label: 'Page Background', priority: 'standard', controls: 9, inScope: true },
    ],
  },
  {
    id: 'layout',
    label: 'Layout',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabPageLayoutWord' },
    groups: [
      { id: 'GroupPageLayoutSetup', label: 'Page Setup', priority: 'primary', controls: 23, inScope: true },
      { id: 'GroupParagraphLayout', label: 'Paragraph', priority: 'standard', controls: 7, inScope: true },
      { id: 'GroupArrange', label: 'Arrange', priority: 'standard', controls: 65, inScope: true },
    ],
  },
  {
    id: 'references',
    label: 'References',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabReferences' },
    groups: [
      { id: 'GroupTableOfContents', label: 'Table of Contents', priority: 'primary', controls: 6, inScope: true },
      { id: 'GroupFootnotes', label: 'Footnotes', priority: 'primary', controls: 9, inScope: true },
      { id: 'GroupCitationsAndBibliography', label: 'Citations & Bibliography', priority: 'standard', controls: 8, inScope: true },
      { id: 'GroupCaptions', label: 'Captions', priority: 'standard', controls: 4, inScope: true },
      { id: 'GroupIndex', label: 'Index', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupTableOfAuthorities', label: 'Table of Authorities', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupAcronyms', label: 'Acronyms', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'mailings',
    label: 'Mailings',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabMailings' },
    groups: [
      { id: 'GroupEnvelopeLabelCreate', label: 'Create', priority: 'standard', controls: 10, inScope: true },
      { id: 'GroupMailMergeStart', label: 'Start Mail Merge', priority: 'primary', controls: 13, inScope: true },
      { id: 'GroupMailMergeWriteInsertFields', label: 'Write & Insert Fields', priority: 'primary', controls: 11, inScope: true },
      { id: 'GroupMailMergePreviewResults', label: 'Preview Results', priority: 'standard', controls: 8, inScope: true },
      { id: 'GroupMailMergeFinish', label: 'Finish', priority: 'standard', controls: 4, inScope: true },
    ],
  },
  {
    id: 'review',
    label: 'Review',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabReviewWord' },
    groups: [
      { id: 'GroupProofing', label: 'Proofing', priority: 'standard', controls: 8, inScope: true },
      { id: 'GroupAccessibility', label: 'Accessibility', priority: 'standard', controls: 6, inScope: true },
      { id: 'GroupLanguage', label: 'Language', priority: 'standard', controls: 14, inScope: true },
      { id: 'GroupComments', label: 'Comments', priority: 'primary', controls: 16, inScope: true },
      { id: 'GroupInk', label: 'Ink', priority: 'standard', controls: 4, inScope: true },
      { id: 'GroupChangesTracking', label: 'Tracking', priority: 'primary', controls: 22, inScope: true },
      { id: 'GroupChanges', label: 'Changes', priority: 'standard', controls: 14, inScope: true },
      { id: 'GroupCompare', label: 'Compare', priority: 'standard', controls: 8, inScope: true },
      { id: 'GroupProtect', label: 'Protect', priority: 'standard', controls: 4, inScope: true },
    ],
  },
  {
    id: 'view',
    label: 'View',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabView' },
    groups: [
      { id: 'GroupDocumentViews', label: 'Document Views', priority: 'primary', controls: 5, inScope: true },
      { id: 'GroupModes', label: 'Modes', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupViewShowHide', label: 'Show', priority: 'standard', controls: 4, inScope: true },
      { id: 'GroupZoom', label: 'Zoom', priority: 'standard', controls: 6, inScope: true },
      { id: 'GroupWindow', label: 'Window', priority: 'standard', controls: 8, inScope: true },
      { id: 'GroupPageMovement', label: 'Page Movement', priority: 'ancillary', controls: 2, inScope: true },
      { id: 'GroupNightMode', label: 'Night Mode', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'outlining',
    label: 'Outlining',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabOutlining' },
    groups: [
      { id: 'GroupOutliningTools', label: 'Outlining Tools', priority: 'primary', controls: 12, inScope: true },
      { id: 'GroupMasterDocument', label: 'Master Document', priority: 'standard', controls: 8, inScope: true },
      { id: 'GroupOutliningClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'print-preview',
    label: 'Print Preview',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabPrintPreview' },
    groups: [
      { id: 'GroupPrintPreviewPrint', label: 'Print', priority: 'secondary', controls: 2, inScope: true },
      { id: 'GroupPrintPreviewPageSetup', label: 'Page Setup', priority: 'standard', controls: 6, inScope: true },
      { id: 'GroupZoom', label: 'Zoom', priority: 'standard', controls: 6, inScope: true },
      { id: 'GroupPrintPreviewPreview', label: 'Preview', priority: 'primary', controls: 6, inScope: true },
    ],
  },
  {
    id: 'background-removal',
    label: 'Background Removal',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabBackgroundRemoval' },
    groups: [
      { id: 'GroupBackgroundRemovalMode', label: 'Refine', priority: 'primary', controls: 3, inScope: true },
      { id: 'GroupBackgroundRemovalClose', label: 'Close', priority: 'secondary', controls: 2, inScope: true },
    ],
  },
];

export const powerpointRibbonTabs: readonly RibbonTabEntry[] = [
  {
    id: 'file',
    label: 'File',
    appearance: 'always',
    source: { kind: 'backstage' },
    groups: [
      { id: 'TabInfo', label: 'Info', priority: 'standard', controls: 8, inScope: true },
      { id: 'TabRecent', label: 'Open', priority: 'primary', controls: 67, inScope: true },
      { id: 'TabSave', label: 'Save', priority: 'primary', controls: 60, inScope: true },
      { id: 'TabPrint', label: 'Print', priority: 'secondary', controls: 1, inScope: true },
      { id: 'TabShare', label: 'Share', priority: 'standard', controls: 22, inScope: true },
      { id: 'TabPublish', label: 'Export', priority: 'standard', controls: 16, inScope: true },
      { id: 'TabHelp', label: 'Help', priority: 'ancillary', controls: 6, inScope: true },
    ],
  },
  {
    id: 'home',
    label: 'Home',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabHome' },
    groups: [
      { id: 'GroupClipboard', label: 'Clipboard', priority: 'secondary', controls: 10, inScope: true, commands: powerpointHomeClipboard },
      { id: 'GroupSlides', label: 'Slides', priority: 'standard', controls: 16, inScope: true },
      { id: 'GroupFont', label: 'Font', priority: 'primary', controls: 18, inScope: true, commands: powerpointHomeFont },
      { id: 'GroupParagraph', label: 'Paragraph', priority: 'primary', controls: 27, inScope: true, commands: powerpointHomeParagraph },
      { id: 'GroupDrawing', label: 'Drawing', priority: 'standard', controls: 63, inScope: true, commands: powerpointHomeDrawing },
      { id: 'GroupEditing', label: 'Editing', priority: 'ancillary', controls: 8, inScope: true, commands: powerpointHomeEditing },
    ],
  },
  {
    id: 'insert',
    label: 'Insert',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabInsert' },
    groups: [
      { id: 'GroupSlides2', label: 'Slides', priority: 'standard', controls: 7, inScope: true },
      { id: 'GroupInsertTables', label: 'Tables', priority: 'standard', controls: 4, inScope: true },
      { id: 'GroupImages', label: 'Images', priority: 'primary', controls: 10, inScope: true },
      { id: 'GroupInsertIllustrations', label: 'Illustrations', priority: 'primary', controls: 11, inScope: true },
      { id: 'GroupChunkCameoCamera', label: 'Camera', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupInsertLinks', label: 'Links', priority: 'standard', controls: 7, inScope: true },
      { id: 'GroupInsertComments', label: 'Comments', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupInsertText', label: 'Text', priority: 'standard', controls: 9, inScope: true },
      { id: 'GroupInsertSymbols', label: 'Symbols', priority: 'standard', controls: 4, inScope: true },
      { id: 'GroupInsertMediaClips', label: 'Media Clips', priority: 'standard', controls: 9, inScope: true },
      { id: 'GroupContent', label: 'Content', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'draw',
    label: 'Draw',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabDrawInk' },
    groups: [
      { id: 'GroupDrawingTools', label: 'Drawing Tools', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupPens2', label: 'Pens', priority: 'primary', controls: 7, inScope: true },
      { id: 'GroupWrite', label: 'Write', priority: 'primary', controls: 10, inScope: true },
      { id: 'GroupStencils', label: 'Stencils', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupInputMode', label: 'Input Mode', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupDrawWithTouch', label: 'Draw with Touch', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'InkReplay', label: 'Replay', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupPenAndInkHelp', label: 'Help', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupInkClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'design',
    label: 'Design',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabDesign' },
    groups: [
      { id: 'GroupSlideThemes', label: 'Themes', priority: 'primary', controls: 4, inScope: true },
      { id: 'GroupThemeVariants', label: 'Variants', priority: 'primary', controls: 10, inScope: true },
      { id: 'GroupCustomizeThemeOptions', label: 'Customise', priority: 'standard', controls: 3, inScope: true },
    ],
  },
  {
    id: 'transitions',
    label: 'Transitions',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabTransitions' },
    groups: [
      { id: 'GroupPreviewTransitions', label: 'Preview', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupTransitionStyles', label: 'Transition Styles', priority: 'primary', controls: 8, inScope: true },
      { id: 'GroupTransitionToThisSlide', label: 'Timing', priority: 'secondary', controls: 2, inScope: true },
    ],
  },
  {
    id: 'animations',
    label: 'Animations',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabAnimations' },
    groups: [
      { id: 'GroupPreview', label: 'Preview', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupAnimations', label: 'Animations', priority: 'primary', controls: 12, inScope: true },
      { id: 'GroupAnimationCustom', label: 'Custom Animation', priority: 'primary', controls: 11, inScope: true },
      { id: 'GroupAnimationTiming', label: 'Timing', priority: 'standard', controls: 6, inScope: true },
    ],
  },
  {
    id: 'slide-show',
    label: 'Slide Show',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabSlideShow' },
    groups: [
      { id: 'GroupSlideShowStart', label: 'Start Slide Show', priority: 'primary', controls: 9, inScope: true },
      { id: 'GroupSlideShowSetup', label: 'Set Up', priority: 'primary', controls: 16, inScope: true },
      { id: 'GroupRehearse', label: 'Rehearse', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupMonitors', label: 'Monitors', priority: 'secondary', controls: 2, inScope: true },
    ],
  },
  {
    id: 'recording',
    label: 'Recording',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabRecording' },
    groups: [
      { id: 'GroupRecord', label: 'Record', priority: 'primary', controls: 8, inScope: true },
      { id: 'GroupRecordTabRecord', label: 'Recording', priority: 'standard', controls: 4, inScope: true },
      { id: 'GroupContentRecording', label: 'Content', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupChunkCameoCamera', label: 'Camera', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupAutoPlayMediaRecording', label: 'Auto-play Media', priority: 'standard', controls: 5, inScope: true },
      { id: 'GroupEditTabRecord', label: 'Edit', priority: 'standard', controls: 6, inScope: true },
      { id: 'GroupSaveRecording', label: 'Save', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupExportTabRecord', label: 'Export', priority: 'ancillary', controls: 2, inScope: true },
      { id: 'GroupPreviewTabRecord', label: 'Preview', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupHelpTabRecord', label: 'Help', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'review',
    label: 'Review',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabReview' },
    groups: [
      { id: 'GroupProofing', label: 'Proofing', priority: 'secondary', controls: 2, inScope: true },
      { id: 'GroupAccessibility', label: 'Accessibility', priority: 'standard', controls: 6, inScope: true },
      { id: 'GroupLanguage', label: 'Language', priority: 'standard', controls: 9, inScope: true },
      { id: 'GroupComments', label: 'Comments', priority: 'primary', controls: 12, inScope: true },
      { id: 'GroupInk', label: 'Ink', priority: 'standard', controls: 5, inScope: true },
      { id: 'GroupReviewCompare', label: 'Compare', priority: 'primary', controls: 13, inScope: true },
      { id: 'GroupActivity', label: 'Activity', priority: 'secondary', controls: 2, inScope: true },
    ],
  },
  {
    id: 'view',
    label: 'View',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabView' },
    groups: [
      { id: 'GroupPresentationViews', label: 'Presentation Views', priority: 'primary', controls: 5, inScope: true },
      { id: 'GroupMasterViews', label: 'Master Views', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupViewShowHide', label: 'Show', priority: 'standard', controls: 5, inScope: true },
      { id: 'GroupZoom', label: 'Zoom', priority: 'secondary', controls: 2, inScope: true },
      { id: 'GroupColorGrayscale', label: 'Colour/Greyscale', priority: 'standard', controls: 4, inScope: true },
      { id: 'GroupWindow', label: 'Window', priority: 'standard', controls: 6, inScope: true },
      { id: 'GroupViewDirection', label: 'View Direction', priority: 'standard', controls: 3, inScope: true },
    ],
  },
  {
    id: 'slide-master',
    label: 'Slide Master',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabSlideMaster' },
    groups: [
      { id: 'GroupMasterEdit', label: 'Edit Master', priority: 'standard', controls: 5, inScope: true },
      { id: 'GroupMasterLayout', label: 'Master Layout', priority: 'primary', controls: 15, inScope: true },
      { id: 'GroupMasterEditTheme', label: 'Edit Theme', priority: 'standard', controls: 4, inScope: true },
      { id: 'GroupBackground', label: 'Background', priority: 'standard', controls: 11, inScope: true },
      { id: 'GroupSlideSize', label: 'Size', priority: 'secondary', controls: 2, inScope: true },
      { id: 'GroupMasterClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    // ⚠ Labelled `Home`, like the ordinary Home tab, because that is what Office calls it. The two
    // are never on screen together there — this one exists only in Slide Master view — and a
    // catalogue story that shows every tab at once is the one place they collide.
    id: 'slide-master-home',
    label: 'Home',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabSlideMasterHome' },
    groups: [
      { id: 'GroupClipboard', label: 'Clipboard', priority: 'secondary', controls: 10, inScope: true },
      { id: 'GroupMasterSlides', label: 'Master Slides', priority: 'standard', controls: 9, inScope: true },
      { id: 'GroupFont', label: 'Font', priority: 'primary', controls: 18, inScope: true },
      { id: 'GroupParagraph', label: 'Paragraph', priority: 'primary', controls: 27, inScope: true },
      { id: 'GroupDrawing', label: 'Drawing', priority: 'standard', controls: 63, inScope: true },
      { id: 'GroupEditing', label: 'Editing', priority: 'ancillary', controls: 8, inScope: true },
    ],
  },
  {
    id: 'handout-master',
    label: 'Handout Master',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabHandoutMaster' },
    groups: [
      { id: 'GroupPageSetupHandoutMaster', label: 'Page Setup', priority: 'primary', controls: 11, inScope: true },
      { id: 'GroupPlaceholdersHandoutMaster', label: 'Placeholders', priority: 'standard', controls: 4, inScope: true },
      { id: 'GroupMasterEditTheme', label: 'Edit Theme', priority: 'standard', controls: 4, inScope: true },
      { id: 'GroupBackground', label: 'Background', priority: 'standard', controls: 11, inScope: true },
      { id: 'GroupMasterClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'notes-master',
    label: 'Notes Master',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabNotesMaster' },
    groups: [
      { id: 'GroupPageSetupNotesMaster', label: 'Page Setup', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupPlaceholdersNotesMaster', label: 'Placeholders', priority: 'standard', controls: 6, inScope: true },
      { id: 'GroupMasterEditTheme', label: 'Edit Theme', priority: 'standard', controls: 4, inScope: true },
      { id: 'GroupBackground', label: 'Background', priority: 'primary', controls: 11, inScope: true },
      { id: 'GroupMasterClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'black-and-white',
    label: 'Black and White',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabBlackAndWhite' },
    groups: [
      { id: 'GroupColorModeSetting', label: 'Colour Mode', priority: 'primary', controls: 10, inScope: true },
      { id: 'GroupColorModeClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'greyscale',
    label: 'Greyscale',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabGrayscale' },
    groups: [
      { id: 'GroupColorModeSetting', label: 'Colour Mode', priority: 'primary', controls: 10, inScope: true },
      { id: 'GroupColorModeClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'print-preview',
    label: 'Print Preview',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabPrintPreview' },
    groups: [
      { id: 'GroupPrintPreviewPrint', label: 'Print', priority: 'secondary', controls: 2, inScope: true },
      { id: 'GroupPrintPreviewPageSetup', label: 'Page Setup', priority: 'standard', controls: 5, inScope: true },
      { id: 'GroupZoom', label: 'Zoom', priority: 'secondary', controls: 2, inScope: true },
      { id: 'GroupPrintPreviewPreview', label: 'Preview', priority: 'primary', controls: 3, inScope: true },
    ],
  },
  {
    id: 'background-removal',
    label: 'Background Removal',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabBackgroundRemoval' },
    groups: [
      { id: 'GroupBackgroundRemovalMode', label: 'Refine', priority: 'primary', controls: 3, inScope: true },
      { id: 'GroupBackgroundRemovalClose', label: 'Close', priority: 'secondary', controls: 2, inScope: true },
    ],
  },
];

export const excelRibbonTabs: readonly RibbonTabEntry[] = [
  {
    // ⚠ No Print group, and that is the census rather than a transcription slip. See the header.
    id: 'file',
    label: 'File',
    appearance: 'always',
    source: { kind: 'backstage' },
    groups: [
      { id: 'TabInfo', label: 'Info', priority: 'standard', controls: 9, inScope: true },
      { id: 'TabRecent', label: 'Open', priority: 'primary', controls: 67, inScope: true },
      { id: 'TabSave', label: 'Save', priority: 'primary', controls: 60, inScope: true },
      { id: 'TabShare', label: 'Share', priority: 'standard', controls: 19, inScope: true },
      { id: 'TabPublish', label: 'Export', priority: 'standard', controls: 6, inScope: true },
      { id: 'Publish2Tab', label: 'Publish', priority: 'secondary', controls: 3, inScope: true },
      { id: 'TabHelp', label: 'Help', priority: 'ancillary', controls: 6, inScope: true },
    ],
  },
  {
    id: 'home',
    label: 'Home',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabHome' },
    groups: [
      { id: 'GroupClipboard', label: 'Clipboard', priority: 'secondary', controls: 12, inScope: true, commands: excelHomeClipboard },
      { id: 'GroupFont', label: 'Font', priority: 'primary', controls: 39, inScope: true, commands: excelHomeFont },
      { id: 'GroupAlignmentExcel', label: 'Alignment', priority: 'primary', controls: 27, inScope: true, commands: excelHomeAlignment },
      { id: 'GroupNumber', label: 'Number', priority: 'standard', controls: 11, inScope: true, commands: excelHomeNumber },
      { id: 'GroupStyles', label: 'Styles', priority: 'standard', controls: 37, inScope: true, commands: excelHomeStyles },
      { id: 'GroupCells', label: 'Cells', priority: 'standard', controls: 39, inScope: true },
      { id: 'GroupEditingExcel', label: 'Editing', priority: 'ancillary', controls: 45, inScope: true, commands: excelHomeEditing },
      { id: 'GroupHomePowerOptions', label: 'Power Options', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'insert',
    label: 'Insert',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabInsert' },
    groups: [
      { id: 'GroupInsertTablesExcel', label: 'Tables', priority: 'primary', controls: 17, inScope: true },
      { id: 'GroupInsertIllustrations', label: 'Illustrations', priority: 'standard', controls: 28, inScope: true },
      { id: 'GroupInsertChartsExcel', label: 'Charts', priority: 'primary', controls: 25, inScope: true },
      { id: 'GroupSparklinesInsert', label: 'Sparklines', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupSlicerInsert', label: 'Slicers', priority: 'ancillary', controls: 2, inScope: true },
      { id: 'GroupInsertLinks', label: 'Links', priority: 'ancillary', controls: 2, inScope: true },
      { id: 'GroupInsertComments', label: 'Comments', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupInsertText', label: 'Text', priority: 'standard', controls: 10, inScope: true },
      { id: 'GroupInsertSymbols', label: 'Symbols', priority: 'standard', controls: 4, inScope: true },
      { id: 'GroupCellControls', label: 'Cell Controls', priority: 'standard', controls: 3, inScope: true },
    ],
  },
  {
    id: 'draw',
    label: 'Draw',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabDrawInk' },
    groups: [
      { id: 'GroupDrawingTools', label: 'Drawing Tools', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupPens2', label: 'Pens', priority: 'primary', controls: 6, inScope: true },
      { id: 'GroupWrite', label: 'Write', priority: 'primary', controls: 5, inScope: true },
      { id: 'GroupInputMode', label: 'Input Mode', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupDrawWithTouch', label: 'Draw with Touch', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'InkReplay', label: 'Replay', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupPenAndInkHelp', label: 'Help', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupInkClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'page-layout',
    label: 'Page Layout',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabPageLayoutExcel' },
    groups: [
      { id: 'GroupThemesExcel', label: 'Themes', priority: 'standard', controls: 9, inScope: true },
      { id: 'GroupPageSetup', label: 'Page Setup', priority: 'primary', controls: 16, inScope: true },
      { id: 'GroupPageLayoutScaleToFit', label: 'Scale to Fit', priority: 'standard', controls: 6, inScope: true },
      { id: 'GroupPageLayoutSheetOptions', label: 'Sheet Options', priority: 'standard', controls: 8, inScope: true },
      { id: 'GroupArrange', label: 'Arrange', priority: 'standard', controls: 47, inScope: true },
    ],
  },
  {
    id: 'formulas',
    label: 'Formulas',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabFormulas' },
    groups: [
      { id: 'GroupFunctionLibrary', label: 'Function Library', priority: 'primary', controls: 37, inScope: true },
      { id: 'GroupNamedCells', label: 'Named Cells', priority: 'standard', controls: 7, inScope: true },
      { id: 'GroupFormulaAuditing', label: 'Formula Auditing', priority: 'standard', controls: 13, inScope: true },
      { id: 'GroupCalculation', label: 'Calculation', priority: 'standard', controls: 7, inScope: true },
    ],
  },
  {
    id: 'data',
    label: 'Data',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabData' },
    groups: [
      { id: 'GroupGetExternalData', label: 'Get External Data', priority: 'standard', controls: 5, inScope: true },
      { id: 'GroupDataQueriesAndConnections', label: 'Queries & Connections', priority: 'standard', controls: 9, inScope: true },
      { id: 'GroupDataQueriesAndConnectionsWorkbookLinks', label: 'Workbook Links', priority: 'standard', controls: 9, inScope: true },
      { id: 'GroupConnections', label: 'Connections', priority: 'standard', controls: 9, inScope: true },
      { id: 'GroupLinkedEntityConvert', label: 'Data Types', priority: 'ancillary', controls: 2, inScope: true },
      { id: 'GroupSortFilter', label: 'Sort & Filter', priority: 'primary', controls: 7, inScope: true },
      { id: 'GroupDataTools', label: 'Data Tools', priority: 'primary', controls: 10, inScope: true },
      { id: 'GroupForecast', label: 'Forecast', priority: 'standard', controls: 5, inScope: true },
      { id: 'GroupOutline', label: 'Outline', priority: 'standard', controls: 10, inScope: true },
    ],
  },
  {
    id: 'review',
    label: 'Review',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabReview' },
    groups: [
      { id: 'GroupProofing', label: 'Proofing', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupAccessibility', label: 'Accessibility', priority: 'standard', controls: 9, inScope: true },
      { id: 'GroupLanguage', label: 'Language', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupThreadedComments', label: 'Threaded Comments', priority: 'primary', controls: 5, inScope: true },
      { id: 'GroupComments', label: 'Comments', priority: 'standard', controls: 6, inScope: true },
      { id: 'GroupCommentsLegacy', label: 'Notes', priority: 'standard', controls: 7, inScope: true },
      { id: 'GroupInk', label: 'Ink', priority: 'standard', controls: 5, inScope: true },
      { id: 'GroupProtectExcel', label: 'Protect', priority: 'primary', controls: 4, inScope: true },
      { id: 'GroupChangesExcel', label: 'Changes', priority: 'standard', controls: 8, inScope: true },
      { id: 'GroupPerformance', label: 'Performance', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupDebug', label: 'Debug', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'view',
    label: 'View',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabView' },
    groups: [
      { id: 'GroupWorkbookViews', label: 'Workbook Views', priority: 'primary', controls: 4, inScope: true },
      { id: 'GroupViewShowHide', label: 'Show', priority: 'standard', controls: 8, inScope: true },
      { id: 'GroupZoom', label: 'Zoom', priority: 'standard', controls: 3, inScope: true },
      { id: 'GroupWindow', label: 'Window', priority: 'standard', controls: 10, inScope: true },
      { id: 'GroupNamedSheetView', label: 'Sheet View', priority: 'standard', controls: 5, inScope: true },
      { id: 'GroupNightMode', label: 'Night Mode', priority: 'ancillary', controls: 1, inScope: true },
      { id: 'GroupViewDebug', label: 'Debug', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'print-preview',
    label: 'Print Preview',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabPrintPreview' },
    groups: [
      { id: 'GroupPrintPreviewPrint', label: 'Print', priority: 'secondary', controls: 2, inScope: true },
      { id: 'GroupPrintPreviewPreview', label: 'Preview', priority: 'primary', controls: 4, inScope: true },
      { id: 'GroupPrintPreviewZoom', label: 'Zoom', priority: 'ancillary', controls: 1, inScope: true },
    ],
  },
  {
    id: 'background-removal',
    label: 'Background Removal',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabBackgroundRemoval' },
    groups: [
      { id: 'GroupBackgroundRemovalMode', label: 'Refine', priority: 'primary', controls: 3, inScope: true },
      { id: 'GroupBackgroundRemovalClose', label: 'Close', priority: 'secondary', controls: 2, inScope: true },
    ],
  },
];

/** Every application's tabs, by kebab id. */
export const ribbonCensus: Readonly<Record<RibbonApplication, readonly RibbonTabEntry[]>> = {
  word: wordRibbonTabs,
  powerpoint: powerpointRibbonTabs,
  excel: excelRibbonTabs,
};

// ── reading it ───────────────────────────────────────────────────────────────

/**
 * One tab, by application and kebab id.
 *
 * @throws {Error} when the tab is not declared — a module asking for a tab that is not in the
 * census has a typo, and a silent `undefined` would become an empty ribbon nobody could explain.
 */
export function ribbonTab(application: RibbonApplication, id: string): RibbonTabEntry {
  const entry = ribbonCensus[application].find((tab) => tab.id === id);
  if (entry === undefined) {
    throw new Error(`${application} declares no '${id}' tab in dev/ribbons/census.ts`);
  }
  return entry;
}

/**
 * One group of one tab, by census id.
 *
 * @throws {Error} for the same reason `ribbonTab` does.
 */
export function ribbonGroup(tab: RibbonTabEntry, id: string): RibbonGroupEntry {
  const entry = tab.groups.find((group) => group.id === id);
  if (entry === undefined) {
    throw new Error(`the '${tab.id}' tab declares no '${id}' group in dev/ribbons/census.ts`);
  }
  return entry;
}

/**
 * The priority of the tab's least-willing-to-give-way group.
 *
 * What a placeholder tab is drawn with, so a tab that will hold a `primary` group when its unit
 * lands does not collapse earlier today than it will then. `groupPriorityNames` is ordered
 * first-to-give-way-**last**, so the strongest is the earliest member present.
 */
export function strongestPriority(tab: RibbonTabEntry): GroupPriority {
  const order: readonly GroupPriority[] = ['primary', 'standard', 'secondary', 'ancillary'];
  return order.find((priority) => tab.groups.some((group) => group.priority === priority)) ?? 'standard';
}

/**
 * The commands a group shows that survive a collapse.
 *
 * A toggle always does — `shell-parts.ts`'s `toggle()` emits `slot="essential"` unconditionally —
 * and a one-shot command does when it says so. `essentialCommandLimit` is the ceiling, and
 * `tests/ribbons.test.ts` holds it.
 */
export function essentialCommands(group: RibbonGroupEntry): readonly RibbonCommand[] {
  return (group.commands ?? []).filter(
    (command) => command.toggle === true || command.essential === true,
  );
}

/** Every command declared anywhere in the census, for the gates that sweep all of them. */
export function everyRibbonCommand(): readonly RibbonCommand[] {
  return ribbonApplicationNames.flatMap((application) =>
    ribbonCensus[application].flatMap((tab) =>
      tab.groups.flatMap((group) => group.commands ?? []),
    ),
  );
}
