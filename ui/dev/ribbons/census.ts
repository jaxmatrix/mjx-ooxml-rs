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
 * ## Which tabs carry commands yet
 *
 * **File and Home.** Unit 0 was the scaffold: the three Home tabs held the commands migrated out of
 * `stories/shell/*.stories.ts`, unchanged, and every other tab was a placeholder. Unit 1 authored
 * File in all three applications — see the *commands File shows* section below, which is also where
 * the reasoning about the census's control counts lives. Unit 2 authored **Home**, replacing that
 * migrated set with every command Office's Home tab shows and filling in the three groups the
 * shells had never carried: Word's Editor, PowerPoint's Slides, and Excel's Cells and Power
 * Options. Every remaining tab is still a placeholder until its own unit. That is why `commands`
 * is optional rather than required — an empty array would claim a tab had been authored and found
 * to hold nothing.
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
// The ribbon programme's **unit 2**, and the tab a person spends their day on. Unit 0 migrated four
// to seven commands per group out of `stories/shell/*.stories.ts`; what is below is every command
// Office's Home tab actually *shows*, in Office's own order, in all three applications.
//
// ## Why this is a fraction of what the census counts, again
//
// Excel's Home counts **233** controls and Word's 165, because the census counts every entry inside
// every gallery and every menu: Word's Font group alone is 43, which is the fifteen commands on its
// face plus the change-case menu, the underline-style menu, the highlight palette and the font
// colour palette. Decision 3 of the approved plan is the rule — **name what the tab shows; menus
// stay shallow** — so Font is fifteen commands and `controls: 43` stays beside it as checked data.
// `dev/word-tab-home.ts` remains the artefact that pads to the census count, and
// `tests/ribbons.test.ts` says so at the assertion.
//
// ## ⚠ Three toggles per group is a ceiling, and Office needs more than three
//
// `shell-parts.ts`'s `toggle()` emits `slot="essential"` unconditionally, and
// `essentialCommandLimit` is **3** — so a group may declare at most three toggles, ever. Office's
// Home tab exceeds that in four places:
//
// | Group | What Office toggles | What is drawn here |
// |---|---|---|
// | Word Font | Bold, Italic, Underline, Strikethrough, Subscript, Superscript | three toggles, three icon buttons |
// | Word Paragraph | Left, Centre, Right, Justify, Show/Hide ¶ | three toggles, two icon buttons |
// | PowerPoint Font | Bold, Italic, Underline, Text Shadow, Strikethrough | three toggles, two buttons |
// | Excel Alignment | Left, Centre, Right and Top, Middle, Bottom | three toggles, three icon buttons |
//
// The commands are all present and all reachable; what the fourth and later ones cannot do is
// **draw pressed**. That is a real loss — a person reading a ribbon learns the paragraph's
// alignment from which mark is filled, and Justify will never fill — and it is recorded here rather
// than smoothed over, because the fix is a change to `<mjx-ribbon-group>`'s ceiling or to
// `toggle()`'s unconditional slot, and neither is unit 2's to make. Which three a group spends its
// slots on is the same rule everywhere: **the three Office draws pressed most often**.
//
// ## `size: 'icon'` is the tab's default, which is new
//
// Unit 1's File tab was a column of labelled `small` buttons, because a backstage page is a list of
// destinations. Home is not: Office draws Word's Font group as two fields and eleven unlabelled
// glyphs, and Excel's Alignment group as eleven unlabelled glyphs and two labelled buttons. So a
// command here is `icon` when Office draws it icon-only and `small` when Office draws its name, and
// the accessible name is the label either way — `controlSizes.icon` draws it off-screen rather than
// dropping it.
//
// ## The commands that carry no icon, and why that is not an omission
//
// Unit 1's rule, unchanged: a wrong icon is worse than a missing one, because a person acts on it.
// PowerPoint's **Text Shadow** has no drawing in Fluent that is not either a square or a sparkle;
// Excel's **Comma Style**, **Increase Decimal** and **Decrease Decimal** are typographic marks
// (`,` and `.00`) that no icon set draws; and Excel's **Power Options** is discussed below. All
// five are `small`, so the label carries the whole command.

const wordHomeClipboard: readonly RibbonCommand[] = [
  { id: 'word.home.clipboard.paste', label: 'Paste', icon: 'clipboard-paste', size: 'large', essential: true },
  { id: 'word.home.clipboard.cut', label: 'Cut', icon: 'cut' },
  { id: 'word.home.clipboard.copy', label: 'Copy', icon: 'copy' },
  { id: 'word.home.clipboard.format-painter', label: 'Format Painter', icon: 'paint-brush' },
];

/**
 * Word's Font group: the two fields, the four size-and-case verbs, and the nine character formats.
 *
 * Bold, Italic and Underline take the group's three essential slots — they are what Office draws
 * pressed, and a collapsed Font group that kept anything else would be a Font group nobody could
 * read. Strikethrough, Subscript and Superscript are states too and are drawn as icon buttons; see
 * this section's header.
 */
const wordHomeFont: readonly RibbonCommand[] = [
  { id: 'word.home.font.name', label: 'Font' },
  { id: 'word.home.font.size', label: 'Font size' },
  { id: 'word.home.font.grow', label: 'Increase Font Size', icon: 'font-increase', size: 'icon' },
  { id: 'word.home.font.shrink', label: 'Decrease Font Size', icon: 'font-decrease', size: 'icon' },
  { id: 'word.home.font.change-case', label: 'Change Case', icon: 'text-change-case', size: 'icon' },
  { id: 'word.home.font.clear-formatting', label: 'Clear All Formatting', icon: 'clear-formatting', size: 'icon' },
  { id: 'word.home.font.bold', label: 'Bold', icon: 'text-bold', toggle: true },
  { id: 'word.home.font.italic', label: 'Italic', icon: 'text-italic', toggle: true, pressed: true },
  { id: 'word.home.font.underline', label: 'Underline', icon: 'text-underline', toggle: true },
  { id: 'word.home.font.strikethrough', label: 'Strikethrough', icon: 'text-strikethrough', size: 'icon' },
  { id: 'word.home.font.subscript', label: 'Subscript', icon: 'text-subscript', size: 'icon' },
  { id: 'word.home.font.superscript', label: 'Superscript', icon: 'text-superscript', size: 'icon' },
  { id: 'word.home.font.text-effects', label: 'Text Effects and Typography', icon: 'text-effects', size: 'icon' },
  { id: 'word.home.font.highlight', label: 'Text Highlight Colour', icon: 'highlight', size: 'icon' },
  { id: 'word.home.font.colour', label: 'Font colour' },
];

/**
 * Word's Paragraph group: the three lists, the two indents, Sort and Show/Hide, then the four
 * alignments, line spacing, Shading and Borders.
 *
 * Office lays this out as two rows of seven and this is that reading order. The alignment marks take
 * the three essential slots, and Justify is the fourth member that cannot — see the header.
 */
const wordHomeParagraph: readonly RibbonCommand[] = [
  { id: 'word.home.paragraph.bullets', label: 'Bullets', icon: 'text-bullet-list-ltr', size: 'icon' },
  { id: 'word.home.paragraph.numbering', label: 'Numbering', icon: 'text-number-list-ltr', size: 'icon' },
  { id: 'word.home.paragraph.multilevel-list', label: 'Multilevel List', icon: 'text-bullet-list-tree', size: 'icon' },
  { id: 'word.home.paragraph.decrease-indent', label: 'Decrease Indent', icon: 'text-indent-decrease', size: 'icon' },
  { id: 'word.home.paragraph.increase-indent', label: 'Increase Indent', icon: 'text-indent-increase', size: 'icon' },
  { id: 'word.home.paragraph.sort', label: 'Sort', icon: 'arrow-sort', size: 'icon' },
  { id: 'word.home.paragraph.show-marks', label: 'Show/Hide ¶', icon: 'text-paragraph', size: 'icon' },
  { id: 'word.home.paragraph.align-left', label: 'Align left', icon: 'text-align-left', toggle: true, pressed: true },
  { id: 'word.home.paragraph.centre', label: 'Centre', icon: 'text-align-center', toggle: true },
  { id: 'word.home.paragraph.align-right', label: 'Align right', icon: 'text-align-right', toggle: true },
  { id: 'word.home.paragraph.justify', label: 'Justify', icon: 'text-align-justify', size: 'icon' },
  { id: 'word.home.paragraph.line-spacing', label: 'Line and Paragraph Spacing', icon: 'text-line-spacing', size: 'icon' },
  { id: 'word.home.paragraph.shading', label: 'Shading', icon: 'color-fill', size: 'icon' },
  { id: 'word.home.paragraph.borders', label: 'Borders', icon: 'border-all', size: 'icon' },
];

/**
 * Word's Styles group is **one control**, and that is Office's own shape rather than a shortfall.
 *
 * The census counts five, which is the gallery plus the four commands inside its own menu — Create
 * a Style, Clear Formatting, Apply Styles, and the pane the dialog launcher opens. What Office
 * *draws* is the gallery and the launcher beside it, and the launcher is not a command: it is the
 * `launcher` option `stories/ribbons/word.ts` passes to `censusGroup`.
 */
const wordHomeStyles: readonly RibbonCommand[] = [
  { id: 'word.home.styles.gallery', label: 'Styles' },
];

const wordHomeEditing: readonly RibbonCommand[] = [
  { id: 'word.home.editing.find', label: 'Find', icon: 'search', essential: true },
  { id: 'word.home.editing.replace', label: 'Replace', icon: 'arrow-swap' },
  { id: 'word.home.editing.select', label: 'Select', icon: 'select-all-on' },
];

/**
 * Word's Editor group: one command, which is what the census counts and what Office draws.
 *
 * Declared since unit 0 and rendered by nothing until unit 2, because the shells never carried it.
 * A group whose label and whose only command are the same word looks like a mistake and is not:
 * Office does exactly this wherever a group holds one button.
 */
const wordHomeEditor: readonly RibbonCommand[] = [
  { id: 'word.home.editor.editor', label: 'Editor', icon: 'text-proofing-tools' },
];

const powerpointHomeClipboard: readonly RibbonCommand[] = [
  { id: 'powerpoint.home.clipboard.paste', label: 'Paste', icon: 'clipboard-paste', size: 'large', essential: true },
  { id: 'powerpoint.home.clipboard.cut', label: 'Cut', icon: 'cut' },
  { id: 'powerpoint.home.clipboard.copy', label: 'Copy', icon: 'copy' },
  { id: 'powerpoint.home.clipboard.format-painter', label: 'Format Painter', icon: 'paint-brush' },
];

/**
 * PowerPoint's Slides group — declared since unit 0, rendered by nothing until now.
 *
 * New Slide is the only `size: 'large'` command unit 2 adds. Section is drawn with
 * `slide-multiple`, which is a judgement: a section *is* a run of slides taken together, and
 * Fluent draws no divider-between-slides at twenty pixels.
 */
const powerpointHomeSlides: readonly RibbonCommand[] = [
  { id: 'powerpoint.home.slides.new-slide', label: 'New Slide', icon: 'slide-add', size: 'large', essential: true },
  { id: 'powerpoint.home.slides.layout', label: 'Layout', icon: 'slide-layout' },
  { id: 'powerpoint.home.slides.reset', label: 'Reset', icon: 'arrow-reset' },
  { id: 'powerpoint.home.slides.section', label: 'Section', icon: 'slide-multiple' },
];

/**
 * PowerPoint's Font group, which is Word's minus the scripts and plus Text Shadow and Character
 * Spacing — the two commands a deck needs and a document does not.
 *
 * **Text Shadow carries no icon**, and it is `small` for that reason: Fluent draws no shadowed
 * letter, and every candidate (`square-shadow`, `text-effects`) already names a different command
 * in this same subset.
 */
const powerpointHomeFont: readonly RibbonCommand[] = [
  { id: 'powerpoint.home.font.name', label: 'Font' },
  { id: 'powerpoint.home.font.size', label: 'Font size' },
  { id: 'powerpoint.home.font.grow', label: 'Increase Font Size', icon: 'font-increase', size: 'icon' },
  { id: 'powerpoint.home.font.shrink', label: 'Decrease Font Size', icon: 'font-decrease', size: 'icon' },
  { id: 'powerpoint.home.font.clear-formatting', label: 'Clear All Formatting', icon: 'clear-formatting', size: 'icon' },
  { id: 'powerpoint.home.font.bold', label: 'Bold', icon: 'text-bold', toggle: true, pressed: true },
  { id: 'powerpoint.home.font.italic', label: 'Italic', icon: 'text-italic', toggle: true },
  { id: 'powerpoint.home.font.underline', label: 'Underline', icon: 'text-underline', toggle: true },
  { id: 'powerpoint.home.font.text-shadow', label: 'Text Shadow' },
  { id: 'powerpoint.home.font.strikethrough', label: 'Strikethrough', icon: 'text-strikethrough', size: 'icon' },
  { id: 'powerpoint.home.font.character-spacing', label: 'Character Spacing', icon: 'font-space-tracking-out', size: 'icon' },
  { id: 'powerpoint.home.font.change-case', label: 'Change Case', icon: 'text-change-case', size: 'icon' },
  { id: 'powerpoint.home.font.colour', label: 'Font colour' },
];

/**
 * PowerPoint's Paragraph group. The list *levels* rather than Word's indents — in a deck an indent
 * is an outline level, and Office names the command accordingly even though the glyph is the same.
 *
 * The last three are the ones a document has no equivalent of: Text Direction rotates a
 * placeholder's text, Align Text is vertical alignment *inside* the placeholder, and Convert to
 * SmartArt turns a bullet list into a diagram.
 */
const powerpointHomeParagraph: readonly RibbonCommand[] = [
  { id: 'powerpoint.home.paragraph.bullets', label: 'Bullets', icon: 'text-bullet-list-ltr', size: 'icon' },
  { id: 'powerpoint.home.paragraph.numbering', label: 'Numbering', icon: 'text-number-list-ltr', size: 'icon' },
  { id: 'powerpoint.home.paragraph.decrease-list-level', label: 'Decrease List Level', icon: 'text-indent-decrease', size: 'icon' },
  { id: 'powerpoint.home.paragraph.increase-list-level', label: 'Increase List Level', icon: 'text-indent-increase', size: 'icon' },
  { id: 'powerpoint.home.paragraph.line-spacing', label: 'Line Spacing', icon: 'text-line-spacing', size: 'icon' },
  { id: 'powerpoint.home.paragraph.align-left', label: 'Align left', icon: 'text-align-left', toggle: true, pressed: true },
  { id: 'powerpoint.home.paragraph.centre', label: 'Centre', icon: 'text-align-center', toggle: true },
  { id: 'powerpoint.home.paragraph.align-right', label: 'Align right', icon: 'text-align-right', toggle: true },
  { id: 'powerpoint.home.paragraph.justify', label: 'Justify', icon: 'text-align-justify', size: 'icon' },
  { id: 'powerpoint.home.paragraph.columns', label: 'Columns', icon: 'text-column-two', size: 'icon' },
  { id: 'powerpoint.home.paragraph.text-direction', label: 'Text Direction', icon: 'text-direction-rotate-90-right' },
  { id: 'powerpoint.home.paragraph.align-text', label: 'Align Text', icon: 'align-center-vertical' },
  { id: 'powerpoint.home.paragraph.smart-art', label: 'Convert to SmartArt', icon: 'diagram' },
];

/**
 * PowerPoint's Drawing group — the census's largest Home group at 63 controls, and six commands on
 * its face.
 *
 * Sixty-three is the shapes gallery's entire catalogue plus three effect menus and the Arrange
 * menu's fourteen entries. What Office draws is the gallery, Arrange, Quick Styles and the three
 * shape formats. **Shapes is the group's survivor**: a collapsed Drawing group has room for one
 * verb, and *put something on the slide* is the one.
 *
 * `drawing.styles` keeps the label *Shape styles* rather than Office's *Quick Styles* because both
 * hosts bind a `<mjx-gallery>` over it and the gallery's own label is what a reader sees; renaming
 * the census entry would change nothing visible and would make the two disagree.
 */
const powerpointHomeDrawing: readonly RibbonCommand[] = [
  { id: 'powerpoint.home.drawing.shapes', label: 'Shapes', icon: 'shapes', essential: true },
  { id: 'powerpoint.home.drawing.arrange', label: 'Arrange', icon: 'layer' },
  { id: 'powerpoint.home.drawing.styles', label: 'Shape styles' },
  { id: 'powerpoint.home.drawing.fill', label: 'Shape Fill', icon: 'color-fill' },
  { id: 'powerpoint.home.drawing.outline', label: 'Shape Outline', icon: 'color-line' },
  { id: 'powerpoint.home.drawing.effects', label: 'Shape Effects', icon: 'square-shadow' },
];

const powerpointHomeEditing: readonly RibbonCommand[] = [
  { id: 'powerpoint.home.editing.find', label: 'Find', icon: 'search', essential: true },
  { id: 'powerpoint.home.editing.replace', label: 'Replace', icon: 'arrow-swap' },
  { id: 'powerpoint.home.editing.select', label: 'Select', icon: 'select-all-on' },
];

const excelHomeClipboard: readonly RibbonCommand[] = [
  { id: 'excel.home.clipboard.paste', label: 'Paste', icon: 'clipboard-paste', size: 'large', essential: true },
  { id: 'excel.home.clipboard.cut', label: 'Cut', icon: 'cut' },
  { id: 'excel.home.clipboard.copy', label: 'Copy', icon: 'copy' },
  { id: 'excel.home.clipboard.format-painter', label: 'Format Painter', icon: 'paint-brush' },
];

/**
 * Excel's Font group, which is the shortest of the three and the only one carrying **Borders**.
 *
 * A cell has an edge and a paragraph does not, which is why Word keeps Borders on Paragraph and
 * Excel keeps it here. Underline arrives in unit 2: the shell's migrated list had Bold and Italic
 * alone, which left Excel the one application whose Font group could not underline anything.
 */
const excelHomeFont: readonly RibbonCommand[] = [
  { id: 'excel.home.font.name', label: 'Font' },
  { id: 'excel.home.font.size', label: 'Font size' },
  { id: 'excel.home.font.grow', label: 'Increase Font Size', icon: 'font-increase', size: 'icon' },
  { id: 'excel.home.font.shrink', label: 'Decrease Font Size', icon: 'font-decrease', size: 'icon' },
  { id: 'excel.home.font.bold', label: 'Bold', icon: 'text-bold', toggle: true },
  { id: 'excel.home.font.italic', label: 'Italic', icon: 'text-italic', toggle: true },
  { id: 'excel.home.font.underline', label: 'Underline', icon: 'text-underline', toggle: true },
  { id: 'excel.home.font.borders', label: 'Borders', icon: 'border-all', size: 'icon' },
  { id: 'excel.home.font.fill', label: 'Fill colour' },
  { id: 'excel.home.font.font-colour', label: 'Font Colour', icon: 'text-color', size: 'icon' },
];

/**
 * Excel's Alignment group — eleven commands, three of which can be toggles.
 *
 * Office draws two rows: the three vertical alignments and Orientation above, the three horizontal
 * alignments, the two indents, Wrap Text and Merge & Centre below. The horizontal three take the
 * essential slots because they are the pair a person reads a sheet by; see this section's header on
 * what that costs Top, Middle and Bottom.
 */
const excelHomeAlignment: readonly RibbonCommand[] = [
  { id: 'excel.home.alignment.align-top', label: 'Top Align', icon: 'align-top', size: 'icon' },
  { id: 'excel.home.alignment.align-middle', label: 'Middle Align', icon: 'align-center-vertical', size: 'icon' },
  { id: 'excel.home.alignment.align-bottom', label: 'Bottom Align', icon: 'align-bottom', size: 'icon' },
  { id: 'excel.home.alignment.orientation', label: 'Orientation', icon: 'text-direction-rotate-90-right', size: 'icon' },
  { id: 'excel.home.alignment.align-left', label: 'Align left', icon: 'text-align-left', toggle: true },
  { id: 'excel.home.alignment.centre', label: 'Centre', icon: 'text-align-center', toggle: true, pressed: true },
  { id: 'excel.home.alignment.align-right', label: 'Align right', icon: 'text-align-right', toggle: true },
  { id: 'excel.home.alignment.decrease-indent', label: 'Decrease Indent', icon: 'text-indent-decrease', size: 'icon' },
  { id: 'excel.home.alignment.increase-indent', label: 'Increase Indent', icon: 'text-indent-increase', size: 'icon' },
  { id: 'excel.home.alignment.wrap', label: 'Wrap Text', icon: 'text-wrap' },
  { id: 'excel.home.alignment.merge', label: 'Merge & Centre', icon: 'table-cells-merge' },
];

/**
 * Excel's Number group: the format dropdown and the five one-press formats beside it.
 *
 * **Three of the six carry no icon, and two of those lost one in unit 2.** Increase Decimal and
 * Decrease Decimal were drawn with `add` and `subtract` — a plus and a minus sign, which is what
 * *insert* and *delete* mean everywhere else on this tab. Office draws them as `.00` with an arrow
 * and Fluent draws no such thing, so the label is the command. Comma Style is the same problem with
 * one character instead of three.
 */
const excelHomeNumber: readonly RibbonCommand[] = [
  { id: 'excel.home.number.format', label: 'Number format' },
  { id: 'excel.home.number.accounting', label: 'Accounting Number Format', icon: 'currency-dollar-euro', size: 'icon' },
  { id: 'excel.home.number.percent', label: 'Percent Style', icon: 'text-percent', size: 'icon' },
  { id: 'excel.home.number.comma', label: 'Comma Style' },
  { id: 'excel.home.number.increase-decimal', label: 'Increase Decimal' },
  { id: 'excel.home.number.decrease-decimal', label: 'Decrease Decimal' },
];

const excelHomeStyles: readonly RibbonCommand[] = [
  { id: 'excel.home.styles.conditional-formatting', label: 'Conditional Formatting', icon: 'table-lightning' },
  { id: 'excel.home.styles.format-as-table', label: 'Format as Table', icon: 'table-checker' },
  { id: 'excel.home.styles.gallery', label: 'Cell styles' },
];

/**
 * Excel's Cells group — declared since unit 0, rendered by nothing until now.
 *
 * Three split buttons covering thirty-nine census controls between them: Insert and Delete each
 * offer cells, rows, columns and a sheet, and Format offers row height, column width, visibility,
 * tab colour, protection and the Format Cells dialog. The faces are the three verbs.
 */
const excelHomeCells: readonly RibbonCommand[] = [
  { id: 'excel.home.cells.insert', label: 'Insert', icon: 'table-add' },
  { id: 'excel.home.cells.delete', label: 'Delete', icon: 'table-dismiss' },
  { id: 'excel.home.cells.format', label: 'Format', icon: 'table-settings' },
];

const excelHomeEditing: readonly RibbonCommand[] = [
  { id: 'excel.home.editing.autosum', label: 'AutoSum', icon: 'autosum', essential: true },
  { id: 'excel.home.editing.fill', label: 'Fill', icon: 'arrow-down' },
  { id: 'excel.home.editing.clear', label: 'Clear', icon: 'eraser' },
  { id: 'excel.home.editing.sort-filter', label: 'Sort & Filter', icon: 'arrow-sort' },
  { id: 'excel.home.editing.find-select', label: 'Find & Select', icon: 'search' },
];

/**
 * ⚠ **`GroupHomePowerOptions` is one control the census names and does not describe**, and this is
 * the honest reading of it.
 *
 * The TSV gives an id, a count of 1 and an in-scope flag; it carries no control names, and no
 * public Office documentation says what a group called *Power Options* on Excel's Home tab holds.
 * Every candidate — Analyze Data, Power Pivot, a sensitivity label — is a guess, and a guessed
 * command name on a ribbon face is exactly the drift the whole transcription exists to prevent. So
 * the command takes the group's own census-derived label and carries **no icon**, which is the same
 * decision `wordHomeEditor` records for a group that genuinely is one button with its group's name
 * on it. When somebody can say what Office puts here, this is a one-line change.
 *
 * Note that the census marks `GroupIdeas` — Excel's Analyze Data — **out of scope** two rows away,
 * which is the strongest available evidence that this group is *not* that one.
 */
const excelHomePowerOptions: readonly RibbonCommand[] = [
  { id: 'excel.home.power-options.power-options', label: 'Power Options' },
];

// ── the commands File shows ──────────────────────────────────────────────────
//
// The ribbon programme's unit 1, and the first tab authored after the scaffold. Decision 1 of the
// approved plan makes File an **ordinary ribbon tab**: its groups are the backstage destinations,
// so `TabRecent` is the Open group and `TabPublish` is the Export group, and a person reaches them
// the way they reach Bold.
//
// ## Why this is a fraction of what the census counts, and why that is the design
//
// The census counts Open at 67 controls and Save at 60, and neither number is a number of things a
// person sees. `ButtonTaskDynamicServiceProvider`, `CloudSkyDriveUpsellGroup` and their forty-odd
// relatives are the cloud-provider plumbing behind *Add a Place* — real controls in Office's own
// manifest, and no more part of the Open page's face than a file dialog's COM registration is.
// Decision 3 of the plan is the rule: **name what the tab shows; menus stay shallow.** So Open is
// the five destinations Office lists down the left of its Open page, and `controls: 67` stays
// beside it as *data* — checked against the TSV by `tests/ribbons.test.ts`, rendered by nothing.
// `dev/word-tab-home.ts` remains the artefact that pads to the census count, because a stress
// specimen is what a count is for.
//
// ## Four pages are the same page three times, so they are written once
//
// Office's Open, Save, Print and Help pages are identical in Word, PowerPoint and Excel — same
// destinations, same verbs, same order — and the only thing that differs is the id prefix a host
// binds an override to. Three copies of an identical list is three places for one of them to drift,
// which is the argument the whole census is written under, so these are functions of the
// application rather than transcribed three times. Info, Share and Export **are** transcribed per
// application, because those three genuinely differ: Office renames the noun (Protect *Document* /
// *Presentation* / *Workbook*), PowerPoint's Export carries video and packaging commands nothing
// else has, and Excel's Share is short because Excel has a Publish page beside it.
//
// ## The commands that carry no icon, and why that is not an omission
//
// Properties, Package Presentation for CD, Create Handouts and Publish to Power BI are drawn as
// `size="small"` with no `icon` at all. `<mjx-icon>`'s own rule is that a wrong icon is worse than
// a missing one because a person acts on it, and Fluent has no honest drawing for any of the four
// — there is no CD in the set at 20px, and a handout is not a landscape page. Properties is the
// one of them Office agrees with outright: it is a panel heading on the Info page rather than a
// picture button, so a glyph here would be an invention rather than a translation.

/**
 * Open, as Office lists it: the five places a document comes from.
 *
 * **Browse** is the group's survivor rather than Recent, and that is a deliberate reading of what
 * a collapsed group is for. Recent is the page's headline and is therefore the `large` button; but
 * a collapsed group has room for a verb, not for a list, and the verb here is *go and find one*.
 */
function fileOpenCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [
    { id: `${application}.file.open.recent`, label: 'Recent', icon: 'history', size: 'large' },
    { id: `${application}.file.open.shared-with-me`, label: 'Shared with Me', icon: 'people' },
    { id: `${application}.file.open.onedrive`, label: 'OneDrive', icon: 'cloud' },
    { id: `${application}.file.open.this-pc`, label: 'This PC', icon: 'desktop' },
    { id: `${application}.file.open.browse`, label: 'Browse', icon: 'folder-open', essential: true },
  ];
}

/**
 * Save, Save As, Save a Copy and AutoSave.
 *
 * **AutoSave is the File tab's only toggle**, in any of the three applications, and it is the
 * reason `arrow-sync` is requested in both variants: a toggle draws filled when it is pressed, and
 * a control that vanished the moment somebody turned it on would be the same defect as a blank
 * square arriving from the other direction. It is declared `pressed` because that is what Office
 * ships for a cloud document, and a resting state nobody has seen is a state nobody has audited.
 */
function fileSaveCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [
    { id: `${application}.file.save.save`, label: 'Save', icon: 'save', size: 'large', essential: true },
    { id: `${application}.file.save.save-as`, label: 'Save As', icon: 'save-edit' },
    { id: `${application}.file.save.save-a-copy`, label: 'Save a Copy', icon: 'save-copy' },
    { id: `${application}.file.save.autosave`, label: 'AutoSave', icon: 'arrow-sync', toggle: true, pressed: true },
  ];
}

/**
 * Print, and the two controls Office puts beside it.
 *
 * Printer and Copies carry **no icon** here and are not meant to render as buttons at all: they are
 * a dropdown and a number field, which `RibbonCommand` deliberately cannot express, so each host
 * binds them as a `TemplateResult` keyed by the id below. That is the seam
 * `stories/ribbons/ribbon-parts.ts` exists to draw, and Print is the first group in the census that
 * actually needs it for something other than a font list — *which printer* is this machine's
 * business, and a ribbon module has no way to know.
 */
function filePrintCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [
    { id: `${application}.file.print.print`, label: 'Print', icon: 'print', size: 'large', essential: true },
    { id: `${application}.file.print.printer`, label: 'Printer' },
    { id: `${application}.file.print.copies`, label: 'Copies' },
    { id: `${application}.file.print.settings`, label: 'Settings', icon: 'settings' },
  ];
}

/** Help, and the four places Office's Help page actually goes. */
function fileHelpCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [
    { id: `${application}.file.help.help`, label: 'Help', icon: 'question-circle', size: 'large', essential: true },
    { id: `${application}.file.help.contact-support`, label: 'Contact Support', icon: 'person-support' },
    { id: `${application}.file.help.feedback`, label: 'Feedback', icon: 'person-feedback' },
    { id: `${application}.file.help.whats-new`, label: 'What’s New', icon: 'megaphone' },
    { id: `${application}.file.help.about`, label: 'About', icon: 'info' },
  ];
}

/**
 * Info, whose four commands are one shape with the application's own noun in them.
 *
 * Office renames all three of the verbs — *Protect Document*, *Protect Presentation*, *Protect
 * Workbook* — and renaming them here rather than writing a generic *Protect* is the point: a
 * command whose label is not the one on the screen is a command a reviewer cannot check.
 * `additional` is Excel's Workbook Statistics, which the other two applications have no equivalent
 * of at all.
 */
function fileInfoCommands(
  application: RibbonApplication,
  noun: string,
  additional: readonly RibbonCommand[] = [],
): readonly RibbonCommand[] {
  return [
    { id: `${application}.file.info.protect`, label: `Protect ${noun}`, icon: 'document-lock', essential: true },
    { id: `${application}.file.info.check-for-issues`, label: 'Check for Issues', icon: 'document-search' },
    { id: `${application}.file.info.manage`, label: `Manage ${noun}`, icon: 'history' },
    ...additional,
    { id: `${application}.file.info.properties`, label: 'Properties' },
  ];
}

const wordFileShare: readonly RibbonCommand[] = [
  { id: 'word.file.share.share', label: 'Share', icon: 'share', size: 'large', essential: true },
  { id: 'word.file.share.email', label: 'Email', icon: 'mail' },
  { id: 'word.file.share.get-a-link', label: 'Get a Link', icon: 'link' },
  { id: 'word.file.share.present-online', label: 'Present Online', icon: 'presenter' },
];

const wordFileExport: readonly RibbonCommand[] = [
  { id: 'word.file.export.pdf', label: 'Create PDF/XPS Document', icon: 'document-pdf', essential: true },
  { id: 'word.file.export.change-file-type', label: 'Change File Type', icon: 'arrow-swap' },
];

const powerpointFileShare: readonly RibbonCommand[] = [
  { id: 'powerpoint.file.share.share', label: 'Share', icon: 'share', size: 'large', essential: true },
  { id: 'powerpoint.file.share.email', label: 'Email', icon: 'mail' },
  { id: 'powerpoint.file.share.get-a-link', label: 'Get a Link', icon: 'link' },
  { id: 'powerpoint.file.share.present-online', label: 'Present Online', icon: 'presenter' },
  { id: 'powerpoint.file.share.publish-slides', label: 'Publish Slides', icon: 'slide-multiple' },
];

/**
 * PowerPoint's Export page, which is the one place the three File tabs visibly diverge.
 *
 * Create a Video and Package Presentation for CD exist in no other application, and Create Handouts
 * exists because a deck is the only document that has a second shape to be printed in. The last two
 * carry no icon: see this section's header.
 */
const powerpointFileExport: readonly RibbonCommand[] = [
  { id: 'powerpoint.file.export.pdf', label: 'Create PDF/XPS Document', icon: 'document-pdf', essential: true },
  { id: 'powerpoint.file.export.video', label: 'Create a Video', icon: 'video' },
  { id: 'powerpoint.file.export.package-for-cd', label: 'Package Presentation for CD' },
  { id: 'powerpoint.file.export.handouts', label: 'Create Handouts' },
  { id: 'powerpoint.file.export.change-file-type', label: 'Change File Type', icon: 'arrow-swap' },
];

/** Excel's Share page is short because Excel has a Publish page beside it. See `excelFilePublish`. */
const excelFileShare: readonly RibbonCommand[] = [
  { id: 'excel.file.share.share', label: 'Share', icon: 'share', size: 'large', essential: true },
  { id: 'excel.file.share.email', label: 'Email', icon: 'mail' },
  { id: 'excel.file.share.get-a-link', label: 'Get a Link', icon: 'link' },
];

const excelFileExport: readonly RibbonCommand[] = [
  { id: 'excel.file.export.pdf', label: 'Create PDF/XPS Document', icon: 'document-pdf', essential: true },
  { id: 'excel.file.export.change-file-type', label: 'Change File Type', icon: 'arrow-swap' },
];

/**
 * Excel's Publish page — the group the other two applications have no row for at all.
 *
 * `Publish2Tab` is the census's own id and carries three controls, which is the one place in this
 * whole tab where what Office shows and what the census counts agree exactly. The headline carries
 * no icon: Fluent draws no Power BI mark, and every generic candidate — a chart, an upload arrow —
 * already means one of the two commands underneath it.
 */
const excelFilePublish: readonly RibbonCommand[] = [
  { id: 'excel.file.publish.power-bi', label: 'Publish to Power BI' },
  { id: 'excel.file.publish.upload-workbook', label: 'Upload Workbook', icon: 'arrow-upload' },
  { id: 'excel.file.publish.export-data', label: 'Export Workbook Data', icon: 'arrow-export' },
];

// ── the tabs ─────────────────────────────────────────────────────────────────

export const wordRibbonTabs: readonly RibbonTabEntry[] = [
  {
    id: 'file',
    label: 'File',
    appearance: 'always',
    source: { kind: 'backstage' },
    groups: [
      { id: 'TabInfo', label: 'Info', priority: 'standard', controls: 8, inScope: true, commands: fileInfoCommands('word', 'Document') },
      { id: 'TabRecent', label: 'Open', priority: 'primary', controls: 67, inScope: true, commands: fileOpenCommands('word') },
      { id: 'TabSave', label: 'Save', priority: 'primary', controls: 60, inScope: true, commands: fileSaveCommands('word') },
      { id: 'TabPrint', label: 'Print', priority: 'secondary', controls: 1, inScope: true, commands: filePrintCommands('word') },
      { id: 'TabShare', label: 'Share', priority: 'standard', controls: 21, inScope: true, commands: wordFileShare },
      { id: 'TabPublish', label: 'Export', priority: 'standard', controls: 8, inScope: true, commands: wordFileExport },
      { id: 'TabHelp', label: 'Help', priority: 'ancillary', controls: 6, inScope: true, commands: fileHelpCommands('word') },
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
      { id: 'GroupEditor', label: 'Editor', priority: 'secondary', controls: 1, inScope: true, commands: wordHomeEditor },
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
      { id: 'TabInfo', label: 'Info', priority: 'standard', controls: 8, inScope: true, commands: fileInfoCommands('powerpoint', 'Presentation') },
      { id: 'TabRecent', label: 'Open', priority: 'primary', controls: 67, inScope: true, commands: fileOpenCommands('powerpoint') },
      { id: 'TabSave', label: 'Save', priority: 'primary', controls: 60, inScope: true, commands: fileSaveCommands('powerpoint') },
      { id: 'TabPrint', label: 'Print', priority: 'secondary', controls: 1, inScope: true, commands: filePrintCommands('powerpoint') },
      { id: 'TabShare', label: 'Share', priority: 'standard', controls: 22, inScope: true, commands: powerpointFileShare },
      { id: 'TabPublish', label: 'Export', priority: 'standard', controls: 16, inScope: true, commands: powerpointFileExport },
      { id: 'TabHelp', label: 'Help', priority: 'ancillary', controls: 6, inScope: true, commands: fileHelpCommands('powerpoint') },
    ],
  },
  {
    id: 'home',
    label: 'Home',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabHome' },
    groups: [
      { id: 'GroupClipboard', label: 'Clipboard', priority: 'secondary', controls: 10, inScope: true, commands: powerpointHomeClipboard },
      { id: 'GroupSlides', label: 'Slides', priority: 'standard', controls: 16, inScope: true, commands: powerpointHomeSlides },
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
      {
        id: 'TabInfo',
        label: 'Info',
        priority: 'standard',
        controls: 9,
        inScope: true,
        commands: fileInfoCommands('excel', 'Workbook', [
          { id: 'excel.file.info.workbook-statistics', label: 'Workbook Statistics', icon: 'data-histogram' },
        ]),
      },
      { id: 'TabRecent', label: 'Open', priority: 'primary', controls: 67, inScope: true, commands: fileOpenCommands('excel') },
      { id: 'TabSave', label: 'Save', priority: 'primary', controls: 60, inScope: true, commands: fileSaveCommands('excel') },
      { id: 'TabShare', label: 'Share', priority: 'standard', controls: 19, inScope: true, commands: excelFileShare },
      { id: 'TabPublish', label: 'Export', priority: 'standard', controls: 6, inScope: true, commands: excelFileExport },
      { id: 'Publish2Tab', label: 'Publish', priority: 'secondary', controls: 3, inScope: true, commands: excelFilePublish },
      { id: 'TabHelp', label: 'Help', priority: 'ancillary', controls: 6, inScope: true, commands: fileHelpCommands('excel') },
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
      { id: 'GroupCells', label: 'Cells', priority: 'standard', controls: 39, inScope: true, commands: excelHomeCells },
      { id: 'GroupEditingExcel', label: 'Editing', priority: 'ancillary', controls: 45, inScope: true, commands: excelHomeEditing },
      { id: 'GroupHomePowerOptions', label: 'Power Options', priority: 'ancillary', controls: 1, inScope: true, commands: excelHomePowerOptions },
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
