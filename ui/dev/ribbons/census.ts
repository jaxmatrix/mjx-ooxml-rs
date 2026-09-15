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
 * 3. **One of Word's Draw groups carries Excel's id** (`GroupEditingExcel`), and PowerPoint's
 *    Recording tab carries both a `GroupRecord` and a `GroupRecordTabRecord`. Both are the
 *    census's own spellings, transcribed unchanged; only the English labels disambiguate. (This
 *    item said *two* Draw groups until unit 4 authored the tab and counted: the TSV has one.)
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
 * **File, Home, Insert, Draw, the Design and Layout tabs, and References, Transitions and Formulas.** Unit 0 was the scaffold: the three Home tabs held the commands migrated
 * out of `stories/shell/*.stories.ts`, unchanged, and every other tab was a placeholder. Unit 1
 * authored File in all three applications — see the *commands File shows* section below, which is
 * also where the reasoning about the census's control counts lives. Unit 2 authored **Home**,
 * replacing that migrated set with every command Office's Home tab shows and filling in the three
 * groups the shells had never carried: Word's Editor, PowerPoint's Slides, and Excel's Cells and
 * Power Options. Unit 3 authored **Insert** — thirty groups across the three applications, and the
 * first tab where nearly every command opens something, so the first where most of a tab's face is
 * bound by its hosts rather than drawn generically; see the *commands Insert shows* section. Unit 4
 * authored **Draw**, whose census declares two generations of Office's ink tools on one tab; see the
 * *commands Draw shows* section. Unit 5 authored **Word's Design and Layout, PowerPoint's Design and Excel's
 * Page Layout**, the tabs about the whole document rather than a selection; see the *commands Design and
 * Layout show* section. Unit 6 authored **Word's References, PowerPoint's Transitions and Excel's
 * Formulas**; see the *commands References, Transitions and Formulas show* section. Every remaining tab is still a placeholder until its own unit. That is why `commands` is optional rather
 * than required — an empty array would claim a tab had been authored and found to hold nothing.
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
  /** Defaults to `small`, which is what a group's secondary commands use — toggles included. */
  readonly size?: ControlSize;
  /**
   * A state rather than a verb — Office draws it pressed while it holds. Rendered as
   * `<mjx-toggle-button>`, and **not** thereby essential: see `essential`.
   */
  readonly toggle?: boolean;
  /** A toggle that starts on, so the ribbon shows a pressed state without a pointer. */
  readonly pressed?: boolean;
  /**
   * **This command survives its group's collapse** — declared, never inferred, for a toggle and a
   * button alike. It must pass every one of `demotionRules`, and it says nothing about *position*:
   * a survivor draws where it is declared at every width with room for the group, and beside the
   * trigger once the group collapses. See `survivorPlacement`.
   */
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

/**
 * The two kinds of page that assemble a ribbon and bind its commands: the `Ribbons/*` catalogue and
 * the `Shell/*` assemblies.
 */
export const ribbonSurfaceHostNames = ['ribbons', 'shell'] as const;

/** One of the two. */
export type RibbonSurfaceHost = (typeof ribbonSurfaceHostNames)[number];

/**
 * **The element id of the surface a bound command opens**, on one host's page —
 * `ribbons-word-insert-tables-table`.
 *
 * Derived from the command id rather than chosen, because the menu and the binding that opens it are
 * written in two different files (`stories/ribbons/insert-menus.ts`, and each host's bindings) and a
 * binding whose `data-opens` names a menu that does not exist is a control that silently does
 * nothing: `openDeclaredSurface` finds no element and returns. `tests/ribbons.test.ts` reads both
 * files and requires every `data-opens` to resolve, and to name its own command's menu.
 */
export function commandSurfaceId(host: RibbonSurfaceHost, commandId: string): string {
  return `${host}-${commandId.replaceAll('.', '-')}`;
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
// ## A state is a toggle; a survivor is declared — and the two are independent
//
// Until unit 2b, `shell-parts.ts`'s `toggle()` emitted `slot="essential"` unconditionally and
// `essentialCommands()` counted every toggle, so *draws pressed* and *survives a collapse* were one
// fact and `essentialCommandLimit` (3) capped every group at three state commands. Four groups on
// this tab paid for it — Word's Font and Paragraph, PowerPoint's Font, Excel's Alignment — and their
// fourth and later states were drawn as plain buttons that could never fill. **They are toggles
// now**, every one Office draws pressed:
//
// | Group | Toggles | Survivors |
// |---|---|---|
// | Word Font | Bold, Italic, Underline, Strikethrough, Subscript, Superscript | Bold, Italic |
// | Word Paragraph | Show/Hide ¶, Align Left, Centre, Align Right, Justify | Align Left, Centre, Align Right |
// | PowerPoint Font | Bold, Italic, Underline, Text Shadow, Strikethrough | Bold, Italic, Underline |
// | PowerPoint Paragraph | Align Left, Centre, Align Right, Justify | Align Left, Centre, Align Right |
// | Excel Font | Bold, Italic, Underline | Bold, Italic |
// | Excel Alignment | Top, Middle, Bottom Align, Wrap Text, Align Left, Centre, Align Right | Align Left, Centre, Align Right |
//
// Excel's **Wrap Text** is on that list although the old table did not name it: Office draws it
// pressed while the cell wraps, so it was a state command drawn as a verb for the same reason.
// **Merge & Centre** is not, because it is a split button — it too highlights on a merged cell,
// but its arrow opens a menu, and `RibbonCommand` is deliberately a button or a toggle.
//
// ## Which commands survive a collapse, and why most groups keep none
//
// A command is `essential` only if it passes **all four** of `demotionRules`, judged on the shape
// **Office** draws it in rather than on the simpler shape this catalogue happens to render: nothing
// that opens a menu, a gallery, a dialog or a file picker; nothing irreversible; an icon a person
// recognises with no label; at most three. Most groups have none, and that is the rule working:
//
// - **Clipboard keeps none, in all three applications.** Paste is a split button in Office, and in
//   every host here — rule 1 — which is why it no longer claims the survivor slot it held since unit
//   0. Cut, Copy and Format Painter pass the rules, and are on the keyboard anyway
//   (Ctrl+X, Ctrl+C, Ctrl+Shift+C) — the reason `dev/word-tab-home.ts` has always given for its own
//   Clipboard keeping none, and the reason the group is `secondary`.
// - **Font keeps the character formats Office draws as plain toggles; Paragraph (Alignment in
//   Excel) keeps three alignments.** PowerPoint's Font keeps Bold, Italic and Underline. **Word's
//   and Excel's keep Bold and Italic only**, because their Underline is a split button in Office —
//   Word's arrow opens the underline styles and the underline colour, Excel's offers Underline and
//   Double Underline — and rule 1 refuses anything with a menu behind it, however the catalogue
//   draws it today. Justify is the fourth alignment and is the one a collapsed group gives up.
// - **Word's Editing keeps none.** Find is a split button in Office (Find, Advanced Find, Go To),
//   Replace opens a dialog, Select opens a menu. **PowerPoint's** Find opens a dialog, and **Excel's**
//   five are all menus or split buttons. The survivors unit 2 declared there — Find, and AutoSum —
//   were rule-1 failures this catalogue could not see, because it draws both as plain buttons.
// - **Slides, Drawing, Styles, Number, Cells, Editor and Power Options keep none.** New Slide,
//   Shapes, Arrange and every shape format open a gallery or a menu; Reset passes rule 1 and fails
//   rule 2, by the standard in the next item.
//   Percent Style passes both, and is not kept, because nothing says Office keeps it: `GUESS:` the
//   one judgement in this list with no Office behaviour behind it either way. Editor's only command
//   is its group's name, and a survivor there would leave the collapsed popup empty.
// - **Rule 2 is judged on the glyph alone, and by one standard: an unlabelled glyph fails when it
//   is already the glyph of a *different* command a person reaches for**, because pressing it would
//   do something they did not ask for. PowerPoint's **Reset** draws `arrow-reset`, the loop that
//   means *undo* and *refresh* everywhere else, so it fails. **AutoSave** draws `arrow-sync`, the
//   glyph of the cloud sync AutoSave *is*, and as a toggle it draws filled while it is on, so it
//   passes. A survivor keeps its own size in the survivor row, so a `small` survivor such as
//   AutoSave also keeps its label there — but the label is never what lets a command pass, or every
//   labelled command would. `GUESS:` a judgement about glyphs, stated as one.
//
// **Where** a survivor draws is not this file's concern any more: every command draws in the order
// below at every width with room for its group, and the survivors move beside the trigger only when
// the group collapses. `survivorPlacement` in `src/ribbon/ribbon-model.ts` is that decision.
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
  { id: 'word.home.clipboard.paste', label: 'Paste', icon: 'clipboard-paste', size: 'large' },
  { id: 'word.home.clipboard.cut', label: 'Cut', icon: 'cut' },
  { id: 'word.home.clipboard.copy', label: 'Copy', icon: 'copy' },
  { id: 'word.home.clipboard.format-painter', label: 'Format Painter', icon: 'paint-brush' },
];

/**
 * Word's Font group: the two fields, the four size-and-case verbs, and the nine character formats.
 *
 * All six character formats are toggles, because Office draws all six pressed. **Bold and Italic
 * are the group's survivors.** They sit **seventh and eighth**, after the two fields and the four
 * size-and-case verbs, and that position is exactly why `survivorPlacement` exists: the old survivor
 * row drew them first.
 *
 * **Underline is not a survivor**, although it is the third format Office draws pressed most often:
 * in Word it is a split button whose arrow opens the underline styles and the underline colour, and
 * demotion rule 1 refuses anything with a menu behind it. It stays a toggle here, because that is
 * the shape this unit draws; the split button is a later unit's.
 */
const wordHomeFont: readonly RibbonCommand[] = [
  { id: 'word.home.font.name', label: 'Font' },
  { id: 'word.home.font.size', label: 'Font size' },
  { id: 'word.home.font.grow', label: 'Increase Font Size', icon: 'font-increase', size: 'icon' },
  { id: 'word.home.font.shrink', label: 'Decrease Font Size', icon: 'font-decrease', size: 'icon' },
  { id: 'word.home.font.change-case', label: 'Change Case', icon: 'text-change-case', size: 'icon' },
  { id: 'word.home.font.clear-formatting', label: 'Clear All Formatting', icon: 'clear-formatting', size: 'icon' },
  { id: 'word.home.font.bold', label: 'Bold', icon: 'text-bold', size: 'icon', toggle: true, essential: true },
  { id: 'word.home.font.italic', label: 'Italic', icon: 'text-italic', size: 'icon', toggle: true, pressed: true, essential: true },
  { id: 'word.home.font.underline', label: 'Underline', icon: 'text-underline', size: 'icon', toggle: true },
  { id: 'word.home.font.strikethrough', label: 'Strikethrough', icon: 'text-strikethrough', size: 'icon', toggle: true },
  { id: 'word.home.font.subscript', label: 'Subscript', icon: 'text-subscript', size: 'icon', toggle: true },
  { id: 'word.home.font.superscript', label: 'Superscript', icon: 'text-superscript', size: 'icon', toggle: true },
  { id: 'word.home.font.text-effects', label: 'Text Effects and Typography', icon: 'text-effects', size: 'icon' },
  { id: 'word.home.font.highlight', label: 'Text Highlight Colour', icon: 'highlight', size: 'icon' },
  { id: 'word.home.font.colour', label: 'Font colour' },
];

/**
 * Word's Paragraph group: the three lists, the two indents, Sort and Show/Hide, then the four
 * alignments, line spacing, Shading and Borders.
 *
 * Office lays this out as two rows of seven and this is that reading order. Show/Hide ¶ and all four
 * alignments are toggles; Align Left, Centre and Align Right survive a collapse, and Justify — the
 * fourth alignment and the least used — is the one the collapsed group gives up. It still draws
 * pressed on a justified paragraph, which it could not before unit 2b.
 */
const wordHomeParagraph: readonly RibbonCommand[] = [
  { id: 'word.home.paragraph.bullets', label: 'Bullets', icon: 'text-bullet-list-ltr', size: 'icon' },
  { id: 'word.home.paragraph.numbering', label: 'Numbering', icon: 'text-number-list-ltr', size: 'icon' },
  { id: 'word.home.paragraph.multilevel-list', label: 'Multilevel List', icon: 'text-bullet-list-tree', size: 'icon' },
  { id: 'word.home.paragraph.decrease-indent', label: 'Decrease Indent', icon: 'text-indent-decrease', size: 'icon' },
  { id: 'word.home.paragraph.increase-indent', label: 'Increase Indent', icon: 'text-indent-increase', size: 'icon' },
  { id: 'word.home.paragraph.sort', label: 'Sort', icon: 'arrow-sort', size: 'icon' },
  { id: 'word.home.paragraph.show-marks', label: 'Show/Hide ¶', icon: 'text-paragraph', size: 'icon', toggle: true },
  { id: 'word.home.paragraph.align-left', label: 'Align left', icon: 'text-align-left', size: 'icon', toggle: true, pressed: true, essential: true },
  { id: 'word.home.paragraph.centre', label: 'Centre', icon: 'text-align-center', size: 'icon', toggle: true, essential: true },
  { id: 'word.home.paragraph.align-right', label: 'Align right', icon: 'text-align-right', size: 'icon', toggle: true, essential: true },
  { id: 'word.home.paragraph.justify', label: 'Justify', icon: 'text-align-justify', size: 'icon', toggle: true },
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

/**
 * Word's Editing group keeps **no** survivor. Office draws Find as a split button (Find, Advanced
 * Find, Go To), Replace opens a dialog and Select opens a menu — three rule-1 failures, the first of
 * which unit 2 declared essential because this catalogue draws it as a plain button.
 */
const wordHomeEditing: readonly RibbonCommand[] = [
  { id: 'word.home.editing.find', label: 'Find', icon: 'search' },
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
  { id: 'powerpoint.home.clipboard.paste', label: 'Paste', icon: 'clipboard-paste', size: 'large' },
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
 *
 * **No survivor.** New Slide is a split button in Office whose arrow is the layout gallery, Layout
 * is a gallery and Section a menu; Reset is the one that opens nothing, and it fails rule 2: its
 * glyph is the loop that means *undo* and *refresh* elsewhere. The standard is stated once, in this
 * section's header, beside AutoSave's.
 */
const powerpointHomeSlides: readonly RibbonCommand[] = [
  { id: 'powerpoint.home.slides.new-slide', label: 'New Slide', icon: 'slide-add', size: 'large' },
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
 * in this same subset. It is a toggle all the same — Office draws it pressed on shadowed text — so it
 * is the one *labelled* toggle on the tab, and it can never be a survivor, because a survivor has no
 * room for a label.
 *
 * Bold, Italic and Underline survive a collapse; Text Shadow and Strikethrough draw pressed and do
 * not. **Underline survives here and not in Word or Excel**: PowerPoint's is a plain toggle, where
 * theirs are split buttons with a menu behind them.
 */
const powerpointHomeFont: readonly RibbonCommand[] = [
  { id: 'powerpoint.home.font.name', label: 'Font' },
  { id: 'powerpoint.home.font.size', label: 'Font size' },
  { id: 'powerpoint.home.font.grow', label: 'Increase Font Size', icon: 'font-increase', size: 'icon' },
  { id: 'powerpoint.home.font.shrink', label: 'Decrease Font Size', icon: 'font-decrease', size: 'icon' },
  { id: 'powerpoint.home.font.clear-formatting', label: 'Clear All Formatting', icon: 'clear-formatting', size: 'icon' },
  { id: 'powerpoint.home.font.bold', label: 'Bold', icon: 'text-bold', size: 'icon', toggle: true, pressed: true, essential: true },
  { id: 'powerpoint.home.font.italic', label: 'Italic', icon: 'text-italic', size: 'icon', toggle: true, essential: true },
  { id: 'powerpoint.home.font.underline', label: 'Underline', icon: 'text-underline', size: 'icon', toggle: true, essential: true },
  { id: 'powerpoint.home.font.text-shadow', label: 'Text Shadow', size: 'small', toggle: true },
  { id: 'powerpoint.home.font.strikethrough', label: 'Strikethrough', icon: 'text-strikethrough', size: 'icon', toggle: true },
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
  { id: 'powerpoint.home.paragraph.align-left', label: 'Align left', icon: 'text-align-left', size: 'icon', toggle: true, pressed: true, essential: true },
  { id: 'powerpoint.home.paragraph.centre', label: 'Centre', icon: 'text-align-center', size: 'icon', toggle: true, essential: true },
  { id: 'powerpoint.home.paragraph.align-right', label: 'Align right', icon: 'text-align-right', size: 'icon', toggle: true, essential: true },
  { id: 'powerpoint.home.paragraph.justify', label: 'Justify', icon: 'text-align-justify', size: 'icon', toggle: true },
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
 * shape formats. **No survivor**: every one of the six opens a gallery or a menu. Unit 2 declared
 * Shapes essential, which put a gallery inside the collapsed group's popup — rule 1's exact case,
 * invisible here only because this catalogue draws Shapes as a plain button.
 *
 * `drawing.styles` keeps the label *Shape styles* rather than Office's *Quick Styles* because both
 * hosts bind a `<mjx-gallery>` over it and the gallery's own label is what a reader sees; renaming
 * the census entry would change nothing visible and would make the two disagree.
 */
const powerpointHomeDrawing: readonly RibbonCommand[] = [
  { id: 'powerpoint.home.drawing.shapes', label: 'Shapes', icon: 'shapes' },
  { id: 'powerpoint.home.drawing.arrange', label: 'Arrange', icon: 'layer' },
  { id: 'powerpoint.home.drawing.styles', label: 'Shape styles' },
  { id: 'powerpoint.home.drawing.fill', label: 'Shape Fill', icon: 'color-fill' },
  { id: 'powerpoint.home.drawing.outline', label: 'Shape Outline', icon: 'color-line' },
  { id: 'powerpoint.home.drawing.effects', label: 'Shape Effects', icon: 'square-shadow' },
];

/** No survivor: PowerPoint's Find opens a dialog, Replace is a split button and Select a menu. */
const powerpointHomeEditing: readonly RibbonCommand[] = [
  { id: 'powerpoint.home.editing.find', label: 'Find', icon: 'search' },
  { id: 'powerpoint.home.editing.replace', label: 'Replace', icon: 'arrow-swap' },
  { id: 'powerpoint.home.editing.select', label: 'Select', icon: 'select-all-on' },
];

const excelHomeClipboard: readonly RibbonCommand[] = [
  { id: 'excel.home.clipboard.paste', label: 'Paste', icon: 'clipboard-paste', size: 'large' },
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
 *
 * **Bold and Italic survive a collapse; Underline does not.** Excel's Underline is a split button in
 * Office — Underline and Double Underline — and demotion rule 1 refuses anything with a menu behind
 * it, exactly as it refuses AutoSum two groups along.
 */
const excelHomeFont: readonly RibbonCommand[] = [
  { id: 'excel.home.font.name', label: 'Font' },
  { id: 'excel.home.font.size', label: 'Font size' },
  { id: 'excel.home.font.grow', label: 'Increase Font Size', icon: 'font-increase', size: 'icon' },
  { id: 'excel.home.font.shrink', label: 'Decrease Font Size', icon: 'font-decrease', size: 'icon' },
  { id: 'excel.home.font.bold', label: 'Bold', icon: 'text-bold', size: 'icon', toggle: true, essential: true },
  { id: 'excel.home.font.italic', label: 'Italic', icon: 'text-italic', size: 'icon', toggle: true, essential: true },
  { id: 'excel.home.font.underline', label: 'Underline', icon: 'text-underline', size: 'icon', toggle: true },
  { id: 'excel.home.font.borders', label: 'Borders', icon: 'border-all', size: 'icon' },
  { id: 'excel.home.font.fill', label: 'Fill colour' },
  { id: 'excel.home.font.font-colour', label: 'Font Colour', icon: 'text-color', size: 'icon' },
];

/**
 * Excel's Alignment group — eleven commands, seven of them states.
 *
 * Office draws two rows: the three vertical alignments, Orientation and Wrap Text above, the three
 * horizontal alignments, the two indents and Merge & Centre below — and this is that reading order,
 * which unit 2 had wrong for Wrap Text. Top, Middle and Bottom Align, Wrap Text and the three
 * horizontal alignments are all toggles; the horizontal three survive a collapse, because they are
 * what a person reads a sheet's columns by.
 *
 * **Bottom Align starts pressed**, beside Centre: Excel's default vertical alignment is bottom, so a
 * cell nobody has formatted is bottom-aligned, and a resting state nobody has seen is a state
 * nobody has audited.
 */
const excelHomeAlignment: readonly RibbonCommand[] = [
  { id: 'excel.home.alignment.align-top', label: 'Top Align', icon: 'align-top', size: 'icon', toggle: true },
  { id: 'excel.home.alignment.align-middle', label: 'Middle Align', icon: 'align-center-vertical', size: 'icon', toggle: true },
  { id: 'excel.home.alignment.align-bottom', label: 'Bottom Align', icon: 'align-bottom', size: 'icon', toggle: true, pressed: true },
  { id: 'excel.home.alignment.orientation', label: 'Orientation', icon: 'text-direction-rotate-90-right', size: 'icon' },
  { id: 'excel.home.alignment.wrap', label: 'Wrap Text', icon: 'text-wrap', toggle: true },
  { id: 'excel.home.alignment.align-left', label: 'Align left', icon: 'text-align-left', size: 'icon', toggle: true, essential: true },
  { id: 'excel.home.alignment.centre', label: 'Centre', icon: 'text-align-center', size: 'icon', toggle: true, pressed: true, essential: true },
  { id: 'excel.home.alignment.align-right', label: 'Align right', icon: 'text-align-right', size: 'icon', toggle: true, essential: true },
  { id: 'excel.home.alignment.decrease-indent', label: 'Decrease Indent', icon: 'text-indent-decrease', size: 'icon' },
  { id: 'excel.home.alignment.increase-indent', label: 'Increase Indent', icon: 'text-indent-increase', size: 'icon' },
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

/** No survivor: AutoSum is a split button in Office, and Fill, Clear, Sort & Filter and Find & Select are menus. */
const excelHomeEditing: readonly RibbonCommand[] = [
  { id: 'excel.home.editing.autosum', label: 'AutoSum', icon: 'autosum' },
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

// ── the commands Insert shows ────────────────────────────────────────────────
//
// The ribbon programme's **unit 3**: Insert, in all three applications — nine groups in Word, eleven
// in PowerPoint, ten in Excel — carrying every command Office's Insert tab shows on its face, in
// Office's own order.
//
// ## Nearly every command on this tab opens something, and that decides three things at once
//
// Home is a tab of *verbs on a selection*: Bold does one thing to the text you have. Insert is a tab
// of *nouns you have not chosen yet* — which table, which shape, which header, which chart — so the
// choice comes after the press, in a gallery, a menu, a dialog or a picker. Of the ninety-odd
// commands below, **fifty-one** are a dropdown or a split button in Office. That decides:
//
// 1. **The bindings.** `RibbonCommand` is a button or a toggle, so each of the fifty-one is declared
//    here with the label, icon and size Office draws it with, and both hosts bind a real control
//    over it by id — `<mjx-split-button>` where Office draws two hit regions, `<mjx-button>` where it
//    draws one — opening an `<mjx-menu>` that `stories/ribbons/insert-menus.ts` writes once for both
//    hosts. The declaration keeps its icon and size, as Home's Paste does, so `tests/ribbons.test.ts`
//    still resolves the glyph the binding draws; a label-only declaration would put every one of
//    those fifty-one glyphs outside the icon gate.
// 2. **The menus stay shallow.** The census counts Word's Header & Footer at 32 and Excel's Charts
//    at 25 because it counts every built-in in every gallery. Decision 3 of the approved plan is
//    unchanged: *name what the tab shows; menus stay shallow*. A menu here carries a handful of real
//    Office entries — Austin, Banded and Facet from the header gallery; Edit Header and Remove Header
//    under them — and the long tail is represented by the control that opens it, never transcribed.
// 3. **No survivor anywhere on the tab.** See the next section.
//
// ## No group on this tab keeps a survivor, and each says why
//
// A survivor must pass **all four** of `demotionRules`, judged on the shape Office draws. On Insert,
// rule 1 alone refuses almost everything: a gallery, a menu, a dialog, a picker, and — less obviously
// — a *drawing mode*. PowerPoint's and Excel's **Text Box** open nothing, and are still not
// immediate: the press arms a gesture and the text box arrives when the pointer is dragged, so one
// press does not do one thing. The three commands that do pass rule 1 are refused on other grounds,
// and each is written beside its group:
//
// - **Word's Page Break** passes all four and is not kept, for the reason Clipboard's Cut and Copy
//   give on Home: it is on the keyboard (Ctrl+Enter) at every width, so a collapsed group spends its
//   one row of room on nothing a person could not already reach.
// - **Word's Blank Page** fails rule 2 by the standard stated once in the Home section's header: a
//   page with a plus on it is the glyph of *New document*, a different command a person reaches for
//   many times a day. `GUESS:` a judgement about a glyph.
// - **Excel's Checkbox** passes rules 1 and 2 and is its group's only command, so a survivor there
//   would leave the collapsed popup empty — the ceiling gate's own refusal, and `wordHomeEditor`'s
//   argument.
//
// ## Three groups are the same group in all three applications, and two of them are written once
//
// **Comments** is one command, *Comment*, in all three — same name, same glyph, same size — so it is
// `insertCommentsCommands(application)`. **Symbols** is Equation and Symbol in all three with the same
// names and sizes, so it is `insertSymbolsCommands(application)`; what differs is the *shape* Office
// draws Symbol in (Word's opens a grid of recent symbols, PowerPoint's and Excel's open the Symbol
// dialog directly), and a shape is a binding, not a declaration. **Links is not shared**, although it
// looks like a candidate: Word's is Link, Bookmark and Cross-reference, PowerPoint's is Zoom, Link and
// Action, and Excel's is Link alone.
//
// ## ⚠ Where the census and Office disagree, recorded rather than smoothed over
//
// 1. **Excel's `GroupSlicerInsert` is labelled *Slicers* here and *Filters* in Office.** Office's group
//    holds Slicer and Timeline, and a timeline is not a slicer; the census's own id is the source of
//    the label, and renaming a group is exactly what `tests/ribbons.test.ts` exists to stop. The
//    commands carry Office's names.
// 2. **PowerPoint's `GroupInsertMediaClips` is labelled *Media Clips* here and *Media* in Office.**
//    Same reasoning.
// 3. **PowerPoint's `GroupContent` is one control the census names and does not describe** —
//    `GroupHomePowerOptions`'s case exactly. Office's PowerPoint Insert tab has no group a person
//    would call *Content*; its neighbours in the census (`GroupMSForms`, `GroupPowerBI`,
//    `GroupOfficeExtension`) are all out of scope, which is the best available evidence that this is
//    a fourth host for external content rather than anything on the classic face. So it is one
//    command carrying the group's own label and no icon, and a one-line change when somebody can
//    say what Office puts there.
// 4. **Office draws groups the census marks out of scope, and they are not drawn here**: Word's
//    Add-ins (between Illustrations and Media) and Barcode, PowerPoint's Forms, Power BI and Add-ins,
//    Excel's Add-ins and Tours (3D Map). The census wins.
// 5. **Three of the tab orders are `GUESS:`, because the census has no order column and Office has
//    moved these groups between releases.** Word's order is Office's and matches the declaration.
//    PowerPoint's **Camera** is drawn after Illustrations and **Content** last, and Excel's **Cell
//    Controls** last — each where the declaration puts it, because no Office build this project can
//    cite puts it anywhere else.
//
// ## Sizes: what Office draws, bounded by what fits
//
// Office's Insert tab is mostly large buttons, and a command is `large` here where Office draws it
// large **and** its label wraps inside `largeControlWidthUnits` — about eighty pixels, two lines. Three
// labels Office draws large are `small` here for the second reason: **Header & Footer** is three
// tokens (PowerPoint and Excel), **Recommended PivotTables** carries a thirteen-letter word, and
// **Recommended Charts** was drawn large and *measured*: in the built catalogue its label needed 58
// pixels of height in a 39-pixel, two-line box, because *Recommended* alone does not fit the width
// and broke onto a third line that was clipped. Every other large label on the tab was measured in
// the same pass and fits. Where
// Office draws a column of labelled commands beside its large ones — Word's Pages, Links, Header &
// Footer and most of Text — they are `small`. Excel's eight chart families are `icon`, which is the
// one place on the tab Office draws glyphs alone, and their accessible names are Office's tooltips.
//
// ## The commands that carry no icon, and why that is not an omission
//
// Unit 1's rule, unchanged: a wrong icon is worse than a missing one, because a person acts on it.
// Fluent draws no cover page, no cross-reference, no Quick Parts building block, no drop cap, no
// OLE object, no Ω, no zoom-to-slide, no pivot, no combo chart and no win/loss mark. So **Cover
// Page**, **Cross-reference**, **Quick Parts**, **Drop Cap** and **Object** (Word); **Reuse Slides**,
// **Zoom**, **Object** and **Content** (PowerPoint); **PivotTable**, **Recommended PivotTables**,
// **Insert Combo Chart**, **PivotChart**, **Win/Loss** and **Object** (Excel); and **Symbol** in all
// three are `small`, and the label is the command. PivotTable is the one that costs something: it is
// the headline of Excel's Insert tab, and it is drawn as a labelled small button rather than as a
// table with arrows somebody would read as *refresh*.

/**
 * Comments: one command, the same in all three applications — `Comment`, drawn large.
 *
 * **No survivor.** New Comment opens a comment card and puts the caret in it — a surface, one press
 * away from the command — and it is the group's only command, so a survivor would leave the collapsed
 * popup empty.
 */
function insertCommentsCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [
    { id: `${application}.insert.comments.comment`, label: 'Comment', icon: 'comment-add', size: 'large' },
  ];
}

/**
 * Symbols: Equation and Symbol, with the same names and sizes in all three applications.
 *
 * Equation is a split button everywhere — the face inserts a new equation, the arrow opens the
 * built-in equations — and it is bound as one by every host. **Symbol** is where the three differ, and
 * they differ in shape only: Word's opens a grid of recently used symbols, so Word's hosts bind a
 * dropdown over it; PowerPoint's and Excel's open the Symbol dialog, so their hosts draw the generic
 * button. It carries no icon in any of them — Fluent's `symbols` is an ampersand and a percent sign,
 * and Office's is Ω.
 *
 * **No survivor**: a split button with a gallery behind it, and a grid or a dialog.
 */
function insertSymbolsCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [
    { id: `${application}.insert.symbols.equation`, label: 'Equation', icon: 'math-formula', size: 'large' },
    { id: `${application}.insert.symbols.symbol`, label: 'Symbol' },
  ];
}

/**
 * Word's Pages group: three small labelled commands in a column, which is how Office draws it.
 *
 * **No survivor.** Cover Page is a gallery. Page Break passes all four rules and is on the keyboard
 * (Ctrl+Enter) at every width, which is the reason Home's Clipboard keeps none. Blank Page fails rule
 * 2: a page with a plus is *New document*'s glyph. See this section's header.
 */
const wordInsertPages: readonly RibbonCommand[] = [
  { id: 'word.insert.pages.cover-page', label: 'Cover Page' },
  { id: 'word.insert.pages.blank-page', label: 'Blank Page', icon: 'document-one-page-add' },
  { id: 'word.insert.pages.page-break', label: 'Page Break', icon: 'document-page-break' },
];

/**
 * Word's Tables group is one control, and the census's seven are what is behind it: the grid, Insert
 * Table, Draw Table, Convert Text to Table, Excel Spreadsheet and Quick Tables.
 *
 * **No survivor**: the grid is a picker.
 */
const wordInsertTables: readonly RibbonCommand[] = [
  { id: 'word.insert.tables.table', label: 'Table', icon: 'table', size: 'large' },
];

/**
 * Word's Illustrations group: four large commands and a column of three small ones.
 *
 * **No survivor**: Pictures, Shapes, 3D Models and Screenshot open a menu or a gallery; Icons, SmartArt
 * and Chart open a dialog.
 */
const wordInsertIllustrations: readonly RibbonCommand[] = [
  { id: 'word.insert.illustrations.pictures', label: 'Pictures', icon: 'image', size: 'large' },
  { id: 'word.insert.illustrations.shapes', label: 'Shapes', icon: 'shapes', size: 'large' },
  { id: 'word.insert.illustrations.icons', label: 'Icons', icon: 'icons', size: 'large' },
  { id: 'word.insert.illustrations.3d-models', label: '3D Models', icon: 'cube', size: 'large' },
  { id: 'word.insert.illustrations.smartart', label: 'SmartArt', icon: 'diagram' },
  { id: 'word.insert.illustrations.chart', label: 'Chart', icon: 'data-bar-vertical' },
  { id: 'word.insert.illustrations.screenshot', label: 'Screenshot', icon: 'screenshot' },
];

/** Word's Media group: one command, and the census counts one. **No survivor**: a dialog, and the only command. */
const wordInsertMedia: readonly RibbonCommand[] = [
  { id: 'word.insert.media.online-videos', label: 'Online Videos', icon: 'video', size: 'large' },
];

/**
 * Word's Links group: Link, Bookmark and Cross-reference.
 *
 * `GUESS:` **small, in a column.** Word 2016 drew all three large and Microsoft 365 draws Link as the
 * group's headline at some widths; the column is the conservative shape, because it keeps all three
 * names at the width Word's Insert tab most often has.
 *
 * **No survivor**: Link is a split button with recent items behind its arrow, and Bookmark and
 * Cross-reference open dialogs.
 */
const wordInsertLinks: readonly RibbonCommand[] = [
  { id: 'word.insert.links.link', label: 'Link', icon: 'link' },
  { id: 'word.insert.links.bookmark', label: 'Bookmark', icon: 'bookmark' },
  { id: 'word.insert.links.cross-reference', label: 'Cross-reference' },
];

/**
 * Word's Header & Footer group — **32** census controls, three commands on its face.
 *
 * Thirty-two is the built-in header gallery, the built-in footer gallery and the page-number
 * positions, each counted entry by entry. `GUESS:` **small, in a column**, as Microsoft 365 draws
 * them at a wide window; Word 2013 drew them large.
 *
 * **No survivor**: all three are galleries.
 */
const wordInsertHeaderFooter: readonly RibbonCommand[] = [
  { id: 'word.insert.header-footer.header', label: 'Header', icon: 'document-header' },
  { id: 'word.insert.header-footer.footer', label: 'Footer', icon: 'document-footer' },
  { id: 'word.insert.header-footer.page-number', label: 'Page Number', icon: 'document-page-number' },
];

/**
 * Word's Text group: Text Box large, then two columns of three.
 *
 * **WordArt draws `text-effects`**, which is also Home's *Text Effects and Typography*, and that is
 * the same idea twice rather than a collision: both put an outlined, shadowed, glowing letter on the
 * page, one on text that exists and one in a new box.
 *
 * **No survivor**: Text Box, WordArt and Drop Cap are galleries, Quick Parts is a menu, Signature Line
 * and Object are split buttons, Date & Time opens a dialog.
 */
const wordInsertText: readonly RibbonCommand[] = [
  { id: 'word.insert.text.text-box', label: 'Text Box', icon: 'textbox', size: 'large' },
  { id: 'word.insert.text.quick-parts', label: 'Quick Parts' },
  { id: 'word.insert.text.wordart', label: 'WordArt', icon: 'text-effects' },
  { id: 'word.insert.text.drop-cap', label: 'Drop Cap' },
  { id: 'word.insert.text.signature-line', label: 'Signature Line', icon: 'signature' },
  { id: 'word.insert.text.date-time', label: 'Date & Time', icon: 'calendar-clock' },
  { id: 'word.insert.text.object', label: 'Object' },
];

/**
 * PowerPoint's Slides group on Insert — New Slide again, and Reuse Slides beside it.
 *
 * `powerpoint.home.slides.new-slide` and this are **two commands with one name**, as they are in
 * Office: the same split button on two tabs, which is why their ids differ by tab and a host binds
 * each where it draws it.
 *
 * **No survivor**: New Slide is a split button whose arrow is the layout gallery, and Reuse Slides
 * opens a pane.
 */
const powerpointInsertSlides: readonly RibbonCommand[] = [
  { id: 'powerpoint.insert.slides.new-slide', label: 'New Slide', icon: 'slide-add', size: 'large' },
  { id: 'powerpoint.insert.slides.reuse-slides', label: 'Reuse Slides' },
];

/** PowerPoint's Tables group: the grid and its three commands behind one button. **No survivor**: a picker. */
const powerpointInsertTables: readonly RibbonCommand[] = [
  { id: 'powerpoint.insert.tables.table', label: 'Table', icon: 'table', size: 'large' },
];

/**
 * PowerPoint's Images group — the group Word and Excel fold into Illustrations. All three large.
 *
 * **No survivor**: Pictures is a menu, Screenshot a gallery, Photo Album a split button.
 */
const powerpointInsertImages: readonly RibbonCommand[] = [
  { id: 'powerpoint.insert.images.pictures', label: 'Pictures', icon: 'image', size: 'large' },
  { id: 'powerpoint.insert.images.screenshot', label: 'Screenshot', icon: 'screenshot', size: 'large' },
  { id: 'powerpoint.insert.images.photo-album', label: 'Photo Album', icon: 'image-multiple', size: 'large' },
];

/**
 * PowerPoint's Illustrations group: five large commands, where Word draws four large and three small —
 * a deck's Insert tab has the room Word's spends on Pages and Header & Footer.
 *
 * **No survivor**: Shapes and 3D Models open a gallery or a menu; Icons, SmartArt and Chart a dialog.
 */
const powerpointInsertIllustrations: readonly RibbonCommand[] = [
  { id: 'powerpoint.insert.illustrations.shapes', label: 'Shapes', icon: 'shapes', size: 'large' },
  { id: 'powerpoint.insert.illustrations.icons', label: 'Icons', icon: 'icons', size: 'large' },
  { id: 'powerpoint.insert.illustrations.3d-models', label: '3D Models', icon: 'cube', size: 'large' },
  { id: 'powerpoint.insert.illustrations.smartart', label: 'SmartArt', icon: 'diagram', size: 'large' },
  { id: 'powerpoint.insert.illustrations.chart', label: 'Chart', icon: 'data-bar-vertical', size: 'large' },
];

/**
 * PowerPoint's Camera group: Cameo, which puts a live camera feed on a slide.
 *
 * `GUESS:` **a split button** — the face inserts the feed on this slide, the arrow offers This Slide
 * and All Slides. The census counts three, which is that shape.
 *
 * **No survivor**: a split button, and the only command.
 */
const powerpointInsertCamera: readonly RibbonCommand[] = [
  { id: 'powerpoint.insert.camera.cameo', label: 'Cameo', icon: 'camera', size: 'large' },
];

/**
 * PowerPoint's Links group: Zoom, Link and Action.
 *
 * **Zoom carries no icon.** It inserts a live thumbnail that jumps to a slide or a section, and
 * Fluent's `zoom-in` is a magnifier — the *view* zoom on the status bar, a different command a person
 * reaches for constantly.
 *
 * **No survivor**: Zoom is a menu (Summary, Section, Slide), Link a split button, Action a dialog.
 */
const powerpointInsertLinks: readonly RibbonCommand[] = [
  { id: 'powerpoint.insert.links.zoom', label: 'Zoom' },
  { id: 'powerpoint.insert.links.link', label: 'Link', icon: 'link', size: 'large' },
  { id: 'powerpoint.insert.links.action', label: 'Action', icon: 'cursor-click', size: 'large' },
];

/**
 * PowerPoint's Text group: three large commands and a column of three small ones, where Office draws
 * Header & Footer large too — see the section header on why three tokens do not fit.
 *
 * **Slide Number draws `number-symbol-square`**, a number sign in a frame, where Word's Page Number
 * draws a numbered page: one command in two vocabularies.
 *
 * **No survivor**: Text Box arms a drawing mode, WordArt is a gallery, and Header & Footer, Date &
 * Time, Slide Number and Object all open a dialog — the first three the same one.
 */
const powerpointInsertText: readonly RibbonCommand[] = [
  { id: 'powerpoint.insert.text.text-box', label: 'Text Box', icon: 'textbox', size: 'large' },
  { id: 'powerpoint.insert.text.header-footer', label: 'Header & Footer', icon: 'document-header-footer' },
  { id: 'powerpoint.insert.text.wordart', label: 'WordArt', icon: 'text-effects', size: 'large' },
  { id: 'powerpoint.insert.text.date-time', label: 'Date & Time', icon: 'calendar-clock' },
  { id: 'powerpoint.insert.text.slide-number', label: 'Slide Number', icon: 'number-symbol-square' },
  { id: 'powerpoint.insert.text.object', label: 'Object' },
];

/**
 * PowerPoint's Media group — `GroupInsertMediaClips`, labelled *Media Clips* by the census. See the
 * section header.
 *
 * **No survivor**: Video and Audio are menus, and Screen Recording opens the recording dock and cannot
 * be taken back by one press.
 */
const powerpointInsertMediaClips: readonly RibbonCommand[] = [
  { id: 'powerpoint.insert.media-clips.video', label: 'Video', icon: 'video', size: 'large' },
  { id: 'powerpoint.insert.media-clips.audio', label: 'Audio', icon: 'speaker-2', size: 'large' },
  { id: 'powerpoint.insert.media-clips.screen-recording', label: 'Screen Recording', icon: 'record', size: 'large' },
];

/**
 * ⚠ **`GroupContent` is one control the census names and does not describe.** See the section header,
 * and `excelHomePowerOptions`, whose reasoning this is.
 *
 * **No survivor**: nothing is known about what it does, and it is the only command.
 */
const powerpointInsertContent: readonly RibbonCommand[] = [
  { id: 'powerpoint.insert.content.content', label: 'Content' },
];

/**
 * Excel's Tables group: PivotTable, Recommended PivotTables and Table.
 *
 * **Only Table carries an icon.** Fluent draws no pivot table, and the nearest pictures — a table with
 * circling arrows, a table with a switch — say *refresh* and *swap*. See the section header on what
 * that costs the headline of Excel's Insert tab.
 *
 * **No survivor**: PivotTable is a split button (From Table/Range, From External Data Source, From
 * Data Model), and Recommended PivotTables and Table open dialogs.
 */
const excelInsertTables: readonly RibbonCommand[] = [
  { id: 'excel.insert.tables.pivottable', label: 'PivotTable' },
  { id: 'excel.insert.tables.recommended-pivottables', label: 'Recommended PivotTables' },
  { id: 'excel.insert.tables.table', label: 'Table', icon: 'table', size: 'large' },
];

/**
 * Excel's Illustrations group: Word's, without Chart — a workbook has a whole Charts group beside it.
 *
 * `GUESS:` **Pictures, Shapes and Icons large and the rest small**, as Excel draws them beside a wide
 * Charts group; at narrower windows Office folds the whole group into one Illustrations button,
 * which is this catalogue's collapse ladder rather than a declaration.
 *
 * **No survivor**: Pictures, Shapes, 3D Models and Screenshot open menus or galleries; Icons and
 * SmartArt open dialogs.
 */
const excelInsertIllustrations: readonly RibbonCommand[] = [
  { id: 'excel.insert.illustrations.pictures', label: 'Pictures', icon: 'image', size: 'large' },
  { id: 'excel.insert.illustrations.shapes', label: 'Shapes', icon: 'shapes', size: 'large' },
  { id: 'excel.insert.illustrations.icons', label: 'Icons', icon: 'icons', size: 'large' },
  { id: 'excel.insert.illustrations.3d-models', label: '3D Models', icon: 'cube' },
  { id: 'excel.insert.illustrations.smartart', label: 'SmartArt', icon: 'diagram' },
  { id: 'excel.insert.illustrations.screenshot', label: 'Screenshot', icon: 'screenshot' },
];

/**
 * Excel's Charts group — Recommended Charts, eight chart families drawn as glyphs, Maps and PivotChart.
 *
 * **Recommended Charts is `small`, although Office draws it large**: its label was measured clipped
 * at `large` in the built catalogue. See the section header.
 *
 * The eight are the one place on any Insert tab where Office draws icons alone, so they are `icon`
 * and each accessible name is Office's tooltip, *Insert Column or Bar Chart* and the rest. They are
 * in reading order: Office lays them out as a three-row block — Column or Bar, Hierarchy, Waterfall;
 * Line or Area, Statistic, Combo; Pie or Doughnut, Scatter — and a group's flex row reads that
 * left to right, top to bottom.
 *
 * **Insert Combo Chart carries no icon**, and is therefore the one labelled command in a row of
 * glyphs — PowerPoint's Text Shadow on Home, again. Fluent draws columns and lines, never both in one
 * picture.
 *
 * **No survivor**: Recommended Charts opens a dialog and the other ten are galleries or a split button.
 */
const excelInsertCharts: readonly RibbonCommand[] = [
  { id: 'excel.insert.charts.recommended-charts', label: 'Recommended Charts', icon: 'chart-multiple' },
  { id: 'excel.insert.charts.column-bar', label: 'Insert Column or Bar Chart', icon: 'data-bar-vertical', size: 'icon' },
  { id: 'excel.insert.charts.hierarchy', label: 'Insert Hierarchy Chart', icon: 'data-treemap', size: 'icon' },
  { id: 'excel.insert.charts.waterfall', label: 'Insert Waterfall, Funnel, Stock, Surface or Radar Chart', icon: 'data-waterfall', size: 'icon' },
  { id: 'excel.insert.charts.line-area', label: 'Insert Line or Area Chart', icon: 'data-line', size: 'icon' },
  { id: 'excel.insert.charts.statistic', label: 'Insert Statistic Chart', icon: 'data-histogram', size: 'icon' },
  { id: 'excel.insert.charts.combo', label: 'Insert Combo Chart' },
  { id: 'excel.insert.charts.pie-doughnut', label: 'Insert Pie or Doughnut Chart', icon: 'data-pie', size: 'icon' },
  { id: 'excel.insert.charts.scatter-bubble', label: 'Insert Scatter (X, Y) or Bubble Chart', icon: 'data-scatter', size: 'icon' },
  { id: 'excel.insert.charts.maps', label: 'Maps', icon: 'map', size: 'large' },
  { id: 'excel.insert.charts.pivotchart', label: 'PivotChart' },
];

/**
 * Excel's Sparklines group: Line, Column, Win/Loss — a chart the size of a cell.
 *
 * Line and Column draw the same glyphs as the Line and Column chart families one group to the left,
 * and that is the same idea twice rather than a collision: a sparkline *is* that chart, drawn in a
 * cell. Win/Loss has no Fluent drawing.
 *
 * **No survivor**: all three open the Create Sparklines dialog, which asks for the data range.
 */
const excelInsertSparklines: readonly RibbonCommand[] = [
  { id: 'excel.insert.sparklines.line', label: 'Line', icon: 'data-line', size: 'large' },
  { id: 'excel.insert.sparklines.column', label: 'Column', icon: 'data-bar-vertical', size: 'large' },
  { id: 'excel.insert.sparklines.win-loss', label: 'Win/Loss' },
];

/**
 * Excel's Filters group — `GroupSlicerInsert`, labelled *Slicers* by the census. See the section
 * header. **No survivor**: both open a dialog listing the fields to filter by.
 */
const excelInsertSlicers: readonly RibbonCommand[] = [
  { id: 'excel.insert.slicers.slicer', label: 'Slicer', icon: 'filter', size: 'large' },
  { id: 'excel.insert.slicers.timeline', label: 'Timeline', icon: 'timeline', size: 'large' },
];

/**
 * Excel's Links group: Link, and the census's second control is its recent-items menu.
 * **No survivor**: a split button, and the only command.
 */
const excelInsertLinks: readonly RibbonCommand[] = [
  { id: 'excel.insert.links.link', label: 'Link', icon: 'link', size: 'large' },
];

/**
 * Excel's Text group: Text Box and WordArt large, Header & Footer, Signature Line and Object small.
 *
 * **No survivor**: Text Box arms a drawing mode, Header & Footer switches the sheet to Page Layout
 * view, WordArt is a gallery, Signature Line a split button and Object a dialog.
 */
const excelInsertText: readonly RibbonCommand[] = [
  { id: 'excel.insert.text.text-box', label: 'Text Box', icon: 'textbox', size: 'large' },
  { id: 'excel.insert.text.header-footer', label: 'Header & Footer', icon: 'document-header-footer' },
  { id: 'excel.insert.text.wordart', label: 'WordArt', icon: 'text-effects', size: 'large' },
  { id: 'excel.insert.text.signature-line', label: 'Signature Line', icon: 'signature' },
  { id: 'excel.insert.text.object', label: 'Object' },
];

/**
 * Excel's Cell Controls group: Checkbox, which turns the selected cells into checkboxes.
 *
 * **No survivor, although it passes rules 1 and 2**: one press, one undo, no popup, and a glyph that
 * is exactly the control it makes. It is the group's only command, so a survivor would leave the
 * collapsed popup empty. See the section header.
 */
const excelInsertCellControls: readonly RibbonCommand[] = [
  { id: 'excel.insert.cell-controls.checkbox', label: 'Checkbox', icon: 'checkbox-checked', size: 'large' },
];

// ── the commands Draw shows ──────────────────────────────────────────────────
//
// The ribbon programme's **unit 4**: Draw, in all three applications — eleven groups in Word, nine in
// PowerPoint, eight in Excel — carrying the commands Office shows on the face of each group the census
// declares, and nothing added to reach the census's counts.
//
// ## ⚠ The census's Draw tab is two generations of Office at once, and the census wins
//
// `TabDrawInk` declares groups that no single Office build draws side by side:
//
// - **Write, Pens and Close** are the three groups of Office 2013's *Ink Tools | Pens* contextual tab,
//   where inking lived before it had a tab of its own. Write held the tools (Select Objects, Lasso
//   Select, Pen, Highlighter, Eraser), Pens held the pen styles with Colour and Thickness, and Close
//   held Stop Inking. The census's counts fit that reading: Excel's `GroupWrite` is **5**, exactly those
//   five tools, and Word's and PowerPoint's are **10**, which is those five plus the eraser sizes behind
//   a split Eraser.
// - **Drawing Tools, Stencils, Drawing Canvas, Replay, Help and Draw with Touch** are the Microsoft 365
//   Draw tab's.
//
// A group the census declares in scope is a group this catalogue draws, so the tab draws both. It
// draws each command **once**, though. Microsoft 365's Drawing Tools tray repeats Select, Lasso Select,
// Eraser and the pens that Write already holds, so **Drawing Tools keeps the one command Write does
// not have: Add Pen**, which matches the census's count of one. `GUESS:` the whole reading. No Office
// build this project can cite draws the union.
//
// ## ⚠ Where the census and Office disagree, recorded rather than smoothed over
//
// 1. **Word's `GroupEditingExcel` is Excel's Home Editing id, on Word's Draw tab.** The census spells
//    it that way and the id is transcribed unchanged; only the label, *Editing*, is this file's. This
//    file's header said *two* of Word's Draw groups carry Excel's ids until this unit counted: the TSV
//    has one. What Word's Draw tab has and the other two lack is **Ink Editor**, which edits the text
//    under the pen by gesture, so it is the group's command. `GUESS:`. The census counts six controls
//    here, and six cannot be named, so none are padded in.
// 2. **Input Mode is not a label Office draws.** Its three controls fit exactly one control Office
//    ships: **Touch/Mouse Mode**, a dropdown holding Mouse and Touch. `GUESS:`.
// 3. **Two labels differ from Office's.** Word's `GroupInsertDrawingCanvas` is labelled *Drawing
//    Canvas* here and *Insert* in Office. `GroupPenAndInkHelp` is *Help* in both, and its command is
//    `GUESS:` **Pen Help**.
// 4. **Office draws groups the census marks out of scope, and they are not drawn here.**
//    `GroupInkConvertOneNote` is Office's Convert group (Ink to Shape, Ink to Math, and Ink to Text in
//    PowerPoint), and `GroupPensOneNote` is its pen tray. The census wins.
// 5. **The group order is `GUESS:` the declaration's**, in all three applications. The union has no
//    Office order to follow, and the declaration keeps Office 2013's Pens and Write adjacent.
//
// ## One survivor on the whole tab, in all three applications
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws. On Draw, the
// deciding rule is rule 1, and unit 3's Text Box sets its standard: **arming a gesture is not one press
// doing one thing.** Pressing Pen, Highlighter, Lasso Select or Eraser does nothing until the pointer
// is dragged, so all four fail, whether they are plain toggles (Excel's Eraser) or split buttons (Word's
// and PowerPoint's). **Select Objects is the exception, and it is the one survivor.** It arms nothing:
// one press puts the pointer back, and one press on a pen takes that back. Its arrow is recognisable
// with no label and is no other command's glyph. It is one of Write's five, so the collapsed popup
// keeps the rest.
//
// The other plain toggles pass rules 1 and 2 and are still not kept. **Ruler** (one press shows the
// stencil, one hides it) and **Draw with Touch** are each their group's only command, so a survivor
// would leave the collapsed popup empty — `excelInsertCellControls`' argument. Pens, Colour, Thickness,
// Add Pen, Touch/Mouse Mode and Word's split Eraser open menus. Ink Replay plays the ink back, Pen Help
// opens a pane, and Stop Inking closes the tools; each is its group's only command.
//
// ## Eight groups are written once, as functions of the application
//
// **Drawing Tools, Pens, Write, Input Mode, Draw with Touch, Replay, Help and Close** declare the same
// commands with the same names, icons and sizes in all three applications. **Stencils** is shared by
// Word and PowerPoint, and Excel's census has no Stencils. What differs is a *shape*, not a
// declaration. Word's and PowerPoint's Eraser is a split button, with the eraser sizes behind its
// arrow, and their hosts bind one. Excel's is the plain toggle its census count of five says it is, so
// Excel's hosts bind nothing over it. That is the Symbol argument from unit 3, again.
//
// ## Sizes, and the commands that carry no icon
//
// Office 2013 drew Pen, Highlighter and Eraser large, with Select Objects and Lasso Select in a small
// column. Microsoft 365 draws Ruler, Ink Replay, Pen Help and Ink Editor large. Those are `large`
// here. **Draw with Touch** is `small` for the reason unit 3 gives about *Header & Footer*: three tokens
// do not wrap inside `largeControlWidthUnits`. Pens is `large` because Office draws it as an in-ribbon
// gallery, the widest thing in its group. Colour and Thickness beside it are `small`.
//
// Unit 1's rule, unchanged: a wrong icon is worse than a missing one. **Add Pen** has no Fluent
// drawing, since Fluent ships no pen with a plus. **Touch/Mouse Mode** is a hand over a pointer in
// Office, and Fluent's tapping finger names only one of the two modes. **Drawing Canvas** is a frame
// with shapes in it in Office, and Fluent's nearest drawings are a whiteboard and a shape being drawn.
// All three are `small`, and the label is the command.

/**
 * Drawing Tools: **Add Pen**, the one command in Microsoft 365's tray that Write does not already hold.
 *
 * Office draws it large, as a pen with a plus. Fluent draws no such thing, so it is `small` and
 * labelled. It is a dropdown in Office (Pen, Pencil, Highlighter), and every host binds one.
 *
 * **No survivor**: a menu, and the only command.
 */
function drawDrawingToolsCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [{ id: `${application}.draw.drawing-tools.add-pen`, label: 'Add Pen' }];
}

/**
 * Pens: the pen styles, Colour and Thickness, which is Office 2013's Pens group.
 *
 * `GUESS:` **the gallery's label is *Pens*.** Office draws the styles as an in-ribbon gallery with no
 * visible name, and a host binds it as a dropdown of named styles, for decision 3 of the approved plan.
 * `inking-tool` is Fluent's own pen nib. **Colour draws `color-line`**, a pen over a stroke of colour,
 * which is also PowerPoint's Shape Outline. That is the same idea twice rather than a collision: both
 * set the colour of a drawn line.
 *
 * The census counts six in Word and Excel and seven in PowerPoint. All of that count is behind these
 * three controls, and none is padded in.
 *
 * **No survivor**: all three open a menu.
 */
function drawPensCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [
    { id: `${application}.draw.pens.pens`, label: 'Pens', icon: 'inking-tool', size: 'large' },
    { id: `${application}.draw.pens.colour`, label: 'Colour', icon: 'color-line' },
    { id: `${application}.draw.pens.thickness`, label: 'Thickness', icon: 'line-thickness' },
  ];
}

/**
 * Write: Select Objects, Lasso Select, Pen, Highlighter, Eraser — Office 2013's tool set, in its order.
 *
 * All five are toggles, because Office draws the current tool pressed. **Select Objects starts
 * pressed**, since a person arriving on the tab has not picked up a pen yet.
 *
 * **Select Objects is the tab's one survivor.** See the section header for why the other four arm a
 * gesture and fail rule 1. **Highlighter draws `highlight`**, Word's Text Highlight Colour glyph. That
 * is the same marker for the same stroke, and Highlighter is not a survivor, so rule 2's collision
 * standard never applies to it. **Eraser draws `eraser`**, Excel's Clear glyph, for the same reason.
 *
 * `GUESS:` **Highlighter at `large`.** *Highlighter* is eleven letters in one word, the same length as
 * the *Recommended* unit 3 measured clipping. Its letters are narrower, and nobody has measured it.
 */
function drawWriteCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [
    {
      id: `${application}.draw.write.select-objects`,
      label: 'Select Objects',
      icon: 'cursor',
      toggle: true,
      pressed: true,
      essential: true,
    },
    { id: `${application}.draw.write.lasso-select`, label: 'Lasso Select', icon: 'lasso', toggle: true },
    { id: `${application}.draw.write.pen`, label: 'Pen', icon: 'pen', size: 'large', toggle: true },
    { id: `${application}.draw.write.highlighter`, label: 'Highlighter', icon: 'highlight', size: 'large', toggle: true },
    { id: `${application}.draw.write.eraser`, label: 'Eraser', icon: 'eraser', size: 'large', toggle: true },
  ];
}

/**
 * Stencils: **Ruler**, in Word and PowerPoint. Excel's census declares no Stencils group.
 *
 * A toggle: Office draws it pressed while the ruler lies on the page.
 *
 * **No survivor, although it passes rules 1 and 2**: it is the group's only command.
 */
function drawStencilsCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [{ id: `${application}.draw.stencils.ruler`, label: 'Ruler', icon: 'ruler', size: 'large', toggle: true }];
}

/**
 * Input Mode: `GUESS:` **Touch/Mouse Mode**, a dropdown of Mouse and Touch. See the section header.
 *
 * **No survivor**: a menu, and the only command.
 */
function drawInputModeCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [{ id: `${application}.draw.input-mode.touch-mouse-mode`, label: 'Touch/Mouse Mode' }];
}

/**
 * Draw with Touch: one toggle, drawn pressed while a finger inks instead of scrolling.
 *
 * `small`, because three tokens do not wrap inside a large button. `hand-draw` is a finger tracing a
 * stroke, which is the command.
 *
 * **No survivor, although it passes rules 1 and 2**: it is the group's only command.
 */
function drawWithTouchCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [
    { id: `${application}.draw.draw-with-touch.draw-with-touch`, label: 'Draw with Touch', icon: 'hand-draw', toggle: true },
  ];
}

/**
 * Replay: **Ink Replay**, which plays the ink back stroke by stroke and shows its playback controls.
 *
 * **No survivor**: a playback that must be stopped rather than one thing done, and the only command.
 */
function drawReplayCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [{ id: `${application}.draw.replay.ink-replay`, label: 'Ink Replay', icon: 'replay', size: 'large' }];
}

/**
 * Help: `GUESS:` **Pen Help**. `question-circle` is the File tab's Help glyph, for the same kind of page.
 *
 * **No survivor**: it opens the Help pane, and it is the only command.
 */
function drawHelpCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [{ id: `${application}.draw.help.pen-help`, label: 'Pen Help', icon: 'question-circle', size: 'large' }];
}

/**
 * Close: **Stop Inking**, Office 2013's own last group, which puts the pen down and leaves ink mode.
 *
 * `pen-dismiss` is a pen with a cross, Office's picture of the command.
 *
 * **No survivor**: nothing a press can take back, and the only command.
 */
function drawCloseCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [{ id: `${application}.draw.close.stop-inking`, label: 'Stop Inking', icon: 'pen-dismiss', size: 'large' }];
}

/**
 * ⚠ Word's `GroupEditingExcel`, labelled *Editing*: `GUESS:` **Ink Editor**. See the section header.
 *
 * A toggle, drawn pressed while the editing pen is held. `text-edit-style` is a letter beside a pen,
 * which is what the command does: it edits text with a pen.
 *
 * **No survivor**: it arms a gesture, and it is the only command.
 */
const wordDrawEditing: readonly RibbonCommand[] = [
  { id: 'word.draw.editing.ink-editor', label: 'Ink Editor', icon: 'text-edit-style', size: 'large', toggle: true },
];

/**
 * Word's Drawing Canvas group, which Office labels *Insert*: one command, which inserts a canvas.
 *
 * **Carries no icon**, and is therefore `small`. See the section header.
 *
 * **No survivor**: one press and one undo, so rules 1 and 4 hold, but there is no glyph and it is the
 * group's only command.
 */
const wordDrawDrawingCanvas: readonly RibbonCommand[] = [
  { id: 'word.draw.drawing-canvas.drawing-canvas', label: 'Drawing Canvas' },
];

// ── the commands Design and Layout show ──────────────────────────────────────
//
// The ribbon programme's **unit 5**: four tabs that say what the *whole* document looks like rather
// than what a selection does. Word's **Design** (two groups) and **Layout** (three), PowerPoint's
// **Design** (three), and Excel's **Page Layout** (five). Each carries the commands Office shows on
// the face of each group the census declares, and nothing added to reach the census's counts.
//
// ## Three shapes, decided by what Office's popup is
//
// Almost every command here opens something, and the brief sorts them into three shapes:
//
// 1. **An in-ribbon gallery** where Office draws a strip of pictures in the group itself: Word's
//    **Style Set**, and PowerPoint's **Themes** and **Variants**. Both hosts bind `<mjx-gallery>`, whose
//    items are written once in `stories/ribbons/design-layout-menus.ts`.
// 2. **A dropdown** where Office draws one button with a list or a grid behind it: Margins,
//    Orientation, Size, Columns, Breaks, Colours, Fonts, Watermark and the rest. Both hosts bind a
//    `<mjx-button>` (or `<mjx-split-button>` where Office draws two hit regions) over a menu written
//    once in the same file.
// 3. **A field** where Office draws a value you type or pick: Word's Indent and Spacing are
//    `<mjx-measure-input>`, Excel's Width and Height are `<mjx-dropdown>`, Excel's Scale is an
//    `<mjx-combo-box>` (a percentage is not a measure `measure.ts` knows), Word's Page Colour is the
//    `<mjx-color-picker>` Home's Font Colour already is, and Excel's four sheet options are
//    `<mjx-checkbox>`. Each reuses the `ribbon*FieldStyle` width Home settled.
//
// ⚠ **Word's and Excel's Themes are a dropdown, although the brief lists Themes among the galleries.**
// Office draws them as a large button whose *popup* is a grid of themes. `<mjx-gallery>` has three
// presentations — the in-ribbon strip, the flyout and the sheet — and none of them is a button, so
// binding a gallery would draw a strip Office does not draw. The popup is therefore a shallow menu of
// named themes, which is Insert's rule for a gallery Office opens from a button (Cover Page, Header).
// PowerPoint's Themes **is** an in-ribbon strip in Office, and it is bound as one. `GUESS:` that this
// reading of the brief is the intended one.
//
// ## ⚠ Where the census and Office disagree, recorded rather than smoothed over
//
// 1. **Word's `GroupStyleSet` is labelled *Style Set* here and *Document Formatting* in Office.** The
//    census's id is the source of the label, as Insert's *Slicers* and *Media Clips* are. The group
//    holds Office's commands: Themes, the Style Set gallery, Colours, Fonts, Paragraph Spacing,
//    Effects, Set as Default.
// 2. **PowerPoint's `GroupDesignerOptions` is out of scope**, so Office's Designer group, after
//    Customise, is not drawn. The census wins.
// 3. **Variants' four submenus are not on the face.** Office opens Colours, Fonts, Effects and
//    Background Styles from the bottom of the Variants gallery, not from the group, so they are
//    **gallery footer buttons** here, each opening a menu. They are not census commands, because the
//    census declares the face. `GUESS:` that a footer button inside an open flyout is a place a menu
//    can be opened from without the flyout closing under it; nobody has watched it happen.
// 4. **The census's counts are much larger than the faces**, and nothing is padded: Word's Arrange is
//    65 (every wrap and position preset), Excel's is 47, and PowerPoint's Themes is 4 for one strip.
//
// ## Three groups are written once
//
// **Arrange** is one function of the application, `arrangeCommands`. Excel's Arrange is Word's
// without Position and Wrap Text, and the six they share carry the same names and glyphs in both.
// What differs is **size**: Excel draws Bring Forward and Send Backward large at the head of its
// group, and Word draws them small beside a large Wrap Text. PowerPoint's Home Arrange (`layer`) is
// **not** shared: it is one button standing for the whole menu, on a different tab, and Office names
// the six commands only where they are spelled out.
//
// **Themes, Colours, Fonts and Effects** open the same four menus in Word's Design and Excel's Page
// Layout, and the same menus again from the foot of PowerPoint's Variants gallery. The declarations are
// not shared, because Word draws Colours and Fonts large and Excel draws them small; the menus are,
// each as one entries function.
//
// **Indent and Spacing** share nothing with Home's Paragraph group, although both live under a
// *Paragraph* label: Home's are one-press verbs (Increase Indent), and Layout's are the measures
// themselves. They reuse `ribbonNarrowFieldStyle` and the measure input, not the Home commands.
//
// ## No survivor on any of the four tabs
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws. Nearly every
// command here fails rule 1: a gallery, a dropdown, a split button, a dialog (Page Borders, Set as
// Default, Background, Print Titles), or a pane (Format Background, Selection Pane). The commands that
// do not open anything are fields and checkboxes, which a host binds, and `tests/ribbons.test.ts`
// refuses a bound survivor. Each group states its own reason below.
//
// ## Sizes, and the commands that carry no icon
//
// Office draws most of these tabs large. A command is `large` here where Office draws it large **and**
// Fluent draws an honest glyph for it; every command with no glyph is `small`, and its label is the
// command. Unit 1's rule is unchanged: a wrong icon is worse than a missing one.
//
// Fluent draws **no theme** (Office's is a page of *Aa* over colour bars; `dark-theme` is dark mode and
// `design-ideas` is PowerPoint's Designer), **no watermark**, **no page size** (`resize` says *resize
// this object*), **no line number** (`number-row` is two digits in boxes, and reads as a count), **no
// hyphen**, **no position preset**, **no selection pane** (`panel-right` is any pane), **no print
// area**, **no sheet background** (`image` is Pictures) and **no print title**. So **Themes** (Word,
// Excel), **Set as Default**, **Watermark**, **Page Colour**, **Size**, **Line Numbers**,
// **Hyphenation**, **Position**, **Selection Pane**, **Print Area**, **Background**, **Print Titles**,
// and the seven fields are drawn without one.

/**
 * Arrange, in Word's Layout and Excel's Page Layout: one function, because the shared six carry the same
 * names and glyphs in both.
 *
 * - **Word** leads with Position and Wrap Text, large, and draws the six small beside them.
 * - **Excel** has no Position or Wrap Text (a cell does not wrap around a picture), and draws Bring
 *   Forward and Send Backward large at the head. `GUESS:` Selection Pane small in Excel, where Office
 *   draws it large, because it has no glyph.
 *
 * Bring Forward and Send Backward are **split buttons** in Office (the face moves one layer, the arrow
 * offers Bring to Front and, in Word, Bring in Front of Text), and every host binds one. Position, Wrap
 * Text, Align, Group and Rotate are dropdowns. **Selection Pane is a toggle**: Office draws it pressed
 * while the pane is open.
 *
 * **Wrap Text draws `text-position-square`**, text around a square, which is the wrap Office's own menu
 * leads with. It is not `text-wrap`, which is Excel's Home Wrap Text: text wrapping inside a cell, a
 * different command a person reaches for on the same application's ribbon. **Align draws `align-left`**,
 * shapes lined up against an edge, which is not `text-align-left`'s ragged lines.
 *
 * **No survivor**: Selection Pane opens a pane and carries no glyph, and the other seven open a menu.
 */
function arrangeCommands(application: 'word' | 'excel'): readonly RibbonCommand[] {
  const tab = application === 'word' ? 'layout' : 'page-layout';
  const layer: ControlSize = application === 'word' ? 'small' : 'large';
  const wordOnly: readonly RibbonCommand[] =
    application === 'word'
      ? [
          { id: 'word.layout.arrange.position', label: 'Position' },
          { id: 'word.layout.arrange.wrap-text', label: 'Wrap Text', icon: 'text-position-square', size: 'large' },
        ]
      : [];
  return [
    ...wordOnly,
    { id: `${application}.${tab}.arrange.bring-forward`, label: 'Bring Forward', icon: 'position-forward', size: layer },
    { id: `${application}.${tab}.arrange.send-backward`, label: 'Send Backward', icon: 'position-backward', size: layer },
    { id: `${application}.${tab}.arrange.selection-pane`, label: 'Selection Pane', toggle: true },
    { id: `${application}.${tab}.arrange.align`, label: 'Align', icon: 'align-left' },
    { id: `${application}.${tab}.arrange.group`, label: 'Group', icon: 'group' },
    { id: `${application}.${tab}.arrange.rotate`, label: 'Rotate', icon: 'rotate-right' },
  ];
}

/**
 * Word's `GroupStyleSet`, which Office labels **Document Formatting**: Themes, the Style Set gallery,
 * Colours, Fonts, Paragraph Spacing, Effects, Set as Default, in Office's order.
 *
 * Themes, Colours, Fonts, Paragraph Spacing and Effects are dropdowns, and the Style Set is an in-ribbon
 * gallery; the hosts bind all six. **Colours draws `color`**, a painter's palette, and **Fonts draws
 * `text-font`**, two letters of two sizes: a heading font and a body font. Both are large, as Word draws
 * them. **Paragraph Spacing draws `text-line-spacing`**, Home's *Line and Paragraph Spacing* glyph, which
 * is the same idea at the scale of the document. **Effects draws `square-shadow`**, PowerPoint's Shape
 * Effects: theme effects *are* the shape effects a theme hands to every shape. `GUESS:` Colours and Fonts
 * large, as Word 2013 to 365 draw them at a wide window.
 *
 * **No survivor**: six of the seven open a gallery or a menu, and Set as Default opens a confirmation.
 */
const wordDesignStyleSet: readonly RibbonCommand[] = [
  { id: 'word.design.style-set.themes', label: 'Themes' },
  { id: 'word.design.style-set.style-set', label: 'Style Set' },
  { id: 'word.design.style-set.colours', label: 'Colours', icon: 'color', size: 'large' },
  { id: 'word.design.style-set.fonts', label: 'Fonts', icon: 'text-font', size: 'large' },
  { id: 'word.design.style-set.paragraph-spacing', label: 'Paragraph Spacing', icon: 'text-line-spacing' },
  { id: 'word.design.style-set.effects', label: 'Effects', icon: 'square-shadow' },
  { id: 'word.design.style-set.set-as-default', label: 'Set as Default' },
];

/**
 * Word's Page Background: Watermark, Page Colour, Page Borders.
 *
 * Watermark is a dropdown gallery (Confidential, Draft, Custom Watermark), and it carries no icon.
 * **Page Colour is the colour picker Home's Font Colour is**, with *No Colour* in its popup; Office draws
 * a large button with the same palette behind it, and the picker is the component this catalogue has for
 * that palette. **Page Borders draws `document-border`**, a page with a border inside its edge, and opens
 * the Borders and Shading dialog, so every host draws the generic button.
 *
 * **No survivor**: a gallery, a picker and a dialog.
 */
const wordDesignPageBackground: readonly RibbonCommand[] = [
  { id: 'word.design.page-background.watermark', label: 'Watermark' },
  { id: 'word.design.page-background.page-colour', label: 'Page Colour' },
  { id: 'word.design.page-background.page-borders', label: 'Page Borders', icon: 'document-border', size: 'large' },
];

/**
 * Word's Page Setup: Margins, Orientation, Size, Columns large, then Breaks, Line Numbers and Hyphenation
 * in a column, which is how Office draws it.
 *
 * All seven are dropdowns. **Margins draws `document-margins`**, a page with its margins dashed in;
 * **Orientation draws `orientation`**, a portrait page turning to landscape; **Columns draws
 * `text-column-two`**, PowerPoint's Home Columns, which is the same command on a text box; **Breaks
 * draws `document-page-break`**, Insert's Page Break, because a page break is the first entry in
 * Breaks. **Size** carries no icon and is therefore small between large neighbours: Office draws it
 * large.
 *
 * **No survivor**: all seven open a menu.
 */
const wordLayoutPageSetup: readonly RibbonCommand[] = [
  { id: 'word.layout.page-setup.margins', label: 'Margins', icon: 'document-margins', size: 'large' },
  { id: 'word.layout.page-setup.orientation', label: 'Orientation', icon: 'orientation', size: 'large' },
  { id: 'word.layout.page-setup.size', label: 'Size' },
  { id: 'word.layout.page-setup.columns', label: 'Columns', icon: 'text-column-two', size: 'large' },
  { id: 'word.layout.page-setup.breaks', label: 'Breaks', icon: 'document-page-break' },
  { id: 'word.layout.page-setup.line-numbers', label: 'Line Numbers' },
  { id: 'word.layout.page-setup.hyphenation', label: 'Hyphenation' },
];

/**
 * Word's Paragraph group on Layout: Indent Left and Indent Right, Spacing Before and Spacing After.
 *
 * **Four measures, not four verbs.** Office draws two spin boxes under *Indent* and two under
 * *Spacing*, and each host binds a `<mjx-measure-input>` over each: centimetres for an indent, points
 * for spacing, which is how Word writes them. The names are Office's tooltips. The census counts seven,
 * which is these four and their spin arrows; nothing is padded in.
 *
 * **No survivor**: a field is richer than a button, and a host binds every one.
 */
const wordLayoutParagraph: readonly RibbonCommand[] = [
  { id: 'word.layout.paragraph.indent-left', label: 'Indent Left' },
  { id: 'word.layout.paragraph.indent-right', label: 'Indent Right' },
  { id: 'word.layout.paragraph.spacing-before', label: 'Spacing Before' },
  { id: 'word.layout.paragraph.spacing-after', label: 'Spacing After' },
];

/**
 * PowerPoint's Themes group: **one in-ribbon gallery**, which is the whole group in Office.
 *
 * The theme names are Office's. Browse for Themes and Save Current Theme sit in the gallery's footer,
 * where Office puts them, and are not face commands.
 *
 * **No survivor**: a gallery, and the only command.
 */
const powerpointDesignThemes: readonly RibbonCommand[] = [
  { id: 'powerpoint.design.themes.themes', label: 'Themes' },
];

/**
 * PowerPoint's Variants group: **one in-ribbon gallery** of the current theme's variants.
 *
 * Colours, Fonts, Effects and Background Styles are the gallery's footer in Office, not face commands;
 * see this section's header.
 *
 * **No survivor**: a gallery, and the only command.
 */
const powerpointDesignVariants: readonly RibbonCommand[] = [
  { id: 'powerpoint.design.variants.variants', label: 'Variants' },
];

/**
 * PowerPoint's Customise group: Slide Size and Format Background, both large.
 *
 * **Slide Size draws `slide-size`**, a frame with a diagonal arrow, and is a dropdown (Standard,
 * Widescreen, Custom Slide Size). **Format Background draws `color-background`**, a paint bucket over a
 * frame, and opens the Format Background pane, so every host draws the generic button. `GUESS:` that
 * *Background*, ten letters, fits `largeControlWidthUnits` where *Recommended* did not; nobody has
 * measured it.
 *
 * **No survivor**: a menu, and a pane.
 */
const powerpointDesignCustomise: readonly RibbonCommand[] = [
  { id: 'powerpoint.design.customise.slide-size', label: 'Slide Size', icon: 'slide-size', size: 'large' },
  { id: 'powerpoint.design.customise.format-background', label: 'Format Background', icon: 'color-background', size: 'large' },
];

/**
 * Excel's Themes group: Themes, then Colours, Fonts and Effects in a column.
 *
 * Word's four theme commands, drawn the size Excel draws them. **Themes** carries no icon, so it is small
 * where Office draws it large. The glyphs are Word's, for Word's reasons.
 *
 * **No survivor**: all four open a menu.
 */
const excelPageLayoutThemes: readonly RibbonCommand[] = [
  { id: 'excel.page-layout.themes.themes', label: 'Themes' },
  { id: 'excel.page-layout.themes.colours', label: 'Colours', icon: 'color' },
  { id: 'excel.page-layout.themes.fonts', label: 'Fonts', icon: 'text-font' },
  { id: 'excel.page-layout.themes.effects', label: 'Effects', icon: 'square-shadow' },
];

/**
 * Excel's Page Setup: Margins, Orientation, Size, Print Area, Breaks, Background, Print Titles.
 *
 * Office draws all seven large. **Margins and Orientation are large here**, with Word's glyphs; the other
 * five are small. Size, Print Area, Background and Print Titles carry no icon, and `GUESS:` Breaks is
 * small beside them rather than the one large command in a column of labels.
 *
 * Margins, Orientation, Size, Print Area and Breaks are dropdowns. **Background** opens the Insert
 * Pictures dialog and **Print Titles** opens Page Setup on its Sheet page, so every host draws the
 * generic button for both.
 *
 * **No survivor**: five menus and two dialogs.
 */
const excelPageLayoutPageSetup: readonly RibbonCommand[] = [
  { id: 'excel.page-layout.page-setup.margins', label: 'Margins', icon: 'document-margins', size: 'large' },
  { id: 'excel.page-layout.page-setup.orientation', label: 'Orientation', icon: 'orientation', size: 'large' },
  { id: 'excel.page-layout.page-setup.size', label: 'Size' },
  { id: 'excel.page-layout.page-setup.print-area', label: 'Print Area' },
  { id: 'excel.page-layout.page-setup.breaks', label: 'Breaks', icon: 'document-page-break' },
  { id: 'excel.page-layout.page-setup.background', label: 'Background' },
  { id: 'excel.page-layout.page-setup.print-titles', label: 'Print Titles' },
];

/**
 * Excel's Scale to Fit: Width, Height and Scale.
 *
 * **Three fields.** Width and Height are dropdowns (Automatic, 1 page, 2 pages …), and each host binds an
 * `<mjx-dropdown>`. Scale is a percentage, which `<mjx-measure-input>` does not carry (its units are
 * lengths), so each host binds the `<mjx-combo-box>` that File's Copies already is: a short list of
 * values, and room to type another. The census counts six, which is these three and their spin arrows.
 *
 * **No survivor**: three fields, all bound.
 */
const excelPageLayoutScaleToFit: readonly RibbonCommand[] = [
  { id: 'excel.page-layout.scale-to-fit.width', label: 'Width' },
  { id: 'excel.page-layout.scale-to-fit.height', label: 'Height' },
  { id: 'excel.page-layout.scale-to-fit.scale', label: 'Scale' },
];

/**
 * Excel's Sheet Options: View and Print under Gridlines, then View and Print under Headings.
 *
 * **Four toggles, drawn as checkboxes.** Office draws four ticks under two headings, and each host binds
 * an `<mjx-checkbox>`. They are declared as toggles because each is a state. The names are Office's
 * tooltips, so the accessible name says which View. **View Gridlines and View Headings start on**, as
 * they are in a new workbook; the two Print options start off.
 *
 * **No survivor, although all four pass rule 1**: one press ticks, one press unticks. None carries a
 * glyph (rule 2), and each is a checkbox a host binds, which the gate refuses as a survivor.
 */
const excelPageLayoutSheetOptions: readonly RibbonCommand[] = [
  { id: 'excel.page-layout.sheet-options.view-gridlines', label: 'View Gridlines', toggle: true, pressed: true },
  { id: 'excel.page-layout.sheet-options.print-gridlines', label: 'Print Gridlines', toggle: true },
  { id: 'excel.page-layout.sheet-options.view-headings', label: 'View Headings', toggle: true, pressed: true },
  { id: 'excel.page-layout.sheet-options.print-headings', label: 'Print Headings', toggle: true },
];

// ── the commands References, Transitions and Formulas show ──────────────────
//
// The ribbon programme's **unit 6**: Word's **References** (seven groups), PowerPoint's **Transitions**
// (three) and Excel's **Formulas** (four). Three tabs with nothing in common but their place in the
// programme, and one shape of problem each: References is a tab of *generated* content (tables of
// contents, indexes, bibliographies) and the commands that keep it current; Transitions is one gallery
// and the timing beside it; Formulas is a library of functions sorted into menus.
//
// ## The shapes, decided by what Office's popup is
//
// Unit 5's three shapes, unchanged. **In-ribbon gallery**: Transitions' Transition to This Slide.
// **Dropdown or split button** over a menu written once in
// `stories/ribbons/references-transitions-formulas-menus.ts`: Table of Contents, Add Text, Next
// Footnote, Insert Citation, Bibliography, Effect Options, AutoSum, the eight function categories,
// Define Name, Use in Formula, Remove Arrows, Error Checking, Calculation Options. **Field**: Word's
// citation Style and PowerPoint's Sound are `<mjx-dropdown>`, Duration and the advance time are
// `<mjx-combo-box>`, On Mouse Click and After are `<mjx-checkbox>`. The lists the fields offer are in
// `stories/ribbons/ribbon-parts.ts`, because two hosts bind each.
//
// ⚠ **Duration is a combo box, not a measure input.** `<mjx-measure-input>` carries six lengths
// (`measureUnitNames`) and a duration is seconds, which is Excel Scale's case from unit 5 exactly: a
// short list of Office's own values, and room to type another. `GUESS:` that this is what the brief's
// *measure field* meant.
//
// ⚠ **Insert Footnote is a plain button.** The brief expected it among the dropdowns and split buttons.
// Office draws References' Footnotes group as one large Insert Footnote with no arrow, then Insert
// Endnote, Next Footnote (the split button) and Show Notes in a column. Office's shape wins, and every
// host draws the generic button over it.
//
// ## ⚠ Where the census and Office disagree, recorded rather than smoothed over
//
// 1. **PowerPoint's `GroupTransitionToThisSlide` is labelled *Timing*, and its id names Office's other
//    group.** Office's Transitions tab is Preview, *Transition to This Slide* (the gallery and Effect
//    Options) and *Timing* (Sound, Duration, Apply To All, On Mouse Click, After). The census carries
//    three rows, and `OFFICE_FEATURE_INVENTORY.md` labels them *Transition Styles 8, Timing 2*. The
//    label is what a person reads, so **Transition Styles holds the gallery and Effect Options, and
//    Timing holds Office's Timing face**. `GUESS:` the reading. The counts argue the other way:
//    `GroupTransitionToThisSlide`'s 2 is exactly the gallery and Effect Options. Timing therefore draws
//    **six** commands where the census counts two, and that is Office's face rather than padding.
// 2. **Excel's `GroupNamedCells` is labelled *Named Cells* here and *Defined Names* in Office.** The
//    census's label wins, as Insert's *Slicers* did. The commands carry Office's names.
// 3. **Office draws groups the census marks out of scope, and they are not drawn here**: Word's
//    Research (Search and Researcher, between Footnotes and Citations & Bibliography), and Excel's four
//    Python groups. The census wins.
// 4. **Word's Acronyms is `GUESS:` last**, where the declaration puts it. It is new in Microsoft 365,
//    and no build this project can cite fixes its position. Its one command opens the Acronyms pane.
// 5. **The census's counts are larger than the faces**, and nothing is padded: Function Library is 37
//    (every function category's menu) and draws ten.
//
// ## No survivor on any of the three tabs
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws. The candidates
// that open nothing, and why each is refused:
//
// - **Insert Footnote** passes rules 1 to 3 in Print Layout. It is refused on unit 3's New Comment
//   argument: it moves the caret out of the text into the note, and in Draft view it opens the notes
//   pane, so one press puts a person somewhere else. It is also Alt+Ctrl+F at every width.
// - **Insert Endnote** is the same command, Alt+Ctrl+D.
// - **Update Index**, and Update Table where it updates without asking, pass rule 1 and fail rule 2:
//   four commands on References draw the one refresh glyph, so unlabelled it does not say which table.
// - **Apply To All** fails rule 2, because it has no glyph. **Show Formulas** is a toggle with no glyph
//   (Ctrl+`). **Trace Precedents** and **Trace Dependents** draw arrows that Ctrl+Z does not remove,
//   so rule 1 fails. **Calculate Now** and **Calculate Sheet** are F9 and Shift+F9, and a manual
//   recalculation cannot be undone.
//
// Everything else opens a menu, a gallery, a dialog or a pane, or is a field a host binds.
//
// ## Sizes, and the commands that carry no icon
//
// A command is `large` where Office draws it large **and** it has an honest glyph **and** its label
// wraps inside `largeControlWidthUnits`. Unit 3's measured rule applies: three tokens do not fit, so
// **Table of Contents**, **Date & Time**, **Lookup & Reference** and **Math & Trig** are `small`, and
// **Calculation Options** is `small` because *Calculation* is eleven letters, *Recommended*'s length.
//
// **Fluent draws Office's own function library.** `book-star`, `book-coins`, `book-question-mark`,
// `book-letter`, `book-clock`, `book-search` and `book-theta` are Recently Used, Financial, Logical,
// Text, Date & Time, Lookup & Reference and Math & Trig, the books Excel draws. More Functions draws the
// plain `book` the other seven are variations of.
//
// A wrong icon is worse than none. Fluent draws **no caption** (`image-alt-text` is Alt Text), **no
// index mark**, **no authority**, **no cross-reference**, **no next note**, **no notes page**, **no
// bibliography** (`book-open` is Read Mode), **no acronym**, **no transition effect option** (Office's
// changes with the transition), **no apply-to-all**, **no trace arrow**, **no formula view**, **no
// formula evaluation** and **no watch window** (`window` is New Window). So **Add Text**, **Next
// Footnote**, **Show Notes**, **Bibliography**, **Insert Caption**, **Insert Table of Figures**,
// **Cross-reference**, **Mark Entry**, **Insert Index**, **Mark Citation**, **Insert Table of
// Authorities**, **Acronyms**, **Effect Options**, **Apply To All**, **Use in Formula**, **Create from
// Selection**, **Trace Precedents**, **Trace Dependents**, **Remove Arrows**, **Show Formulas**,
// **Evaluate Formula** and **Watch Window** are `small`, and the label is the command. The six fields
// carry none either.

/**
 * Word's Table of Contents group: Table of Contents, Add Text, Update Table.
 *
 * **Table of Contents draws `document-bullet-list`**, a page holding a list, and is a dropdown of the
 * built-in tables. It is `small`, although Office draws it large: three tokens. **Add Text** is a
 * dropdown of the levels. **Update Table draws `document-sync`**, a page with a refresh on it, and
 * opens Office's *update page numbers or the entire table* dialog, so every host draws the generic
 * button.
 *
 * **No survivor**: two menus and a dialog.
 */
const wordReferencesTableOfContents: readonly RibbonCommand[] = [
  { id: 'word.references.table-of-contents.table-of-contents', label: 'Table of Contents', icon: 'document-bullet-list' },
  { id: 'word.references.table-of-contents.add-text', label: 'Add Text' },
  { id: 'word.references.table-of-contents.update-table', label: 'Update Table', icon: 'document-sync' },
];

/**
 * Word's Footnotes group: Insert Footnote large, then Insert Endnote, Next Footnote and Show Notes.
 *
 * **Insert Footnote draws `text-footnote`**, *Ab* with a superscript 1, which is Office's own picture.
 * **Insert Endnote draws `document-endnote`**, a page with a mark at its foot. **Next Footnote is a
 * split button**: the face moves to the next footnote, and the arrow offers Previous Footnote, Next
 * Endnote and Previous Endnote. **Show Notes** jumps to the notes, or asks which when a document has
 * both, so it is the generic button.
 *
 * **No survivor**: see this section's header on Insert Footnote.
 */
const wordReferencesFootnotes: readonly RibbonCommand[] = [
  { id: 'word.references.footnotes.insert-footnote', label: 'Insert Footnote', icon: 'text-footnote', size: 'large' },
  { id: 'word.references.footnotes.insert-endnote', label: 'Insert Endnote', icon: 'document-endnote' },
  { id: 'word.references.footnotes.next-footnote', label: 'Next Footnote' },
  { id: 'word.references.footnotes.show-notes', label: 'Show Notes' },
];

/**
 * Word's Citations & Bibliography group: Insert Citation large, then Manage Sources, Style and
 * Bibliography.
 *
 * **Insert Citation draws `text-quote`**, and is a dropdown (Add New Source, Add New Placeholder).
 * `GUESS:` the glyph: a citation marks words taken from a source, and Office's picture is a page with a
 * mark on it that Fluent does not draw. **Manage Sources draws `library`**, books on a shelf, and opens
 * the Source Manager dialog. **Style is a field**, a dropdown of citation styles starting on APA, as
 * Word's is. **Bibliography** is a dropdown of the built-in bibliographies.
 *
 * **No survivor**: two menus, a dialog and a field.
 */
const wordReferencesCitationsBibliography: readonly RibbonCommand[] = [
  { id: 'word.references.citations-bibliography.insert-citation', label: 'Insert Citation', icon: 'text-quote', size: 'large' },
  { id: 'word.references.citations-bibliography.manage-sources', label: 'Manage Sources', icon: 'library' },
  { id: 'word.references.citations-bibliography.style', label: 'Style' },
  { id: 'word.references.citations-bibliography.bibliography', label: 'Bibliography' },
];

/**
 * Word's Captions group: Insert Caption, Insert Table of Figures, Update Table, Cross-reference.
 *
 * Insert Caption, Insert Table of Figures and Cross-reference open dialogs, so every host draws the
 * generic buttons. **Update Table** is Table of Contents' command for the table of figures, with the
 * same glyph and the same dialog. **Insert Caption carries no icon**, so it is `small` where Office
 * draws it large. The census counts four, and the face is four.
 *
 * **No survivor**: three dialogs, and an Update Table whose glyph three other commands share.
 */
const wordReferencesCaptions: readonly RibbonCommand[] = [
  { id: 'word.references.captions.insert-caption', label: 'Insert Caption' },
  { id: 'word.references.captions.insert-table-of-figures', label: 'Insert Table of Figures' },
  { id: 'word.references.captions.update-table', label: 'Update Table', icon: 'document-sync' },
  { id: 'word.references.captions.cross-reference', label: 'Cross-reference' },
];

/**
 * Word's Index group: Mark Entry, Insert Index, Update Index. The census counts three.
 *
 * Mark Entry and Insert Index open dialogs. **Update Index draws `document-sync`** and updates the index
 * with no dialog.
 *
 * **No survivor**: Update Index passes rule 1 and fails rule 2. See this section's header.
 */
const wordReferencesIndex: readonly RibbonCommand[] = [
  { id: 'word.references.index.mark-entry', label: 'Mark Entry' },
  { id: 'word.references.index.insert-index', label: 'Insert Index' },
  { id: 'word.references.index.update-index', label: 'Update Index', icon: 'document-sync' },
];

/**
 * Word's Table of Authorities group: Mark Citation, Insert Table of Authorities, Update Table.
 *
 * Index's shape for a legal document. Mark Citation and Insert Table of Authorities open dialogs.
 * **Update Table** draws the shared refresh glyph.
 *
 * **No survivor**: two dialogs, and rule 2 refuses Update Table.
 */
const wordReferencesTableOfAuthorities: readonly RibbonCommand[] = [
  { id: 'word.references.table-of-authorities.mark-citation', label: 'Mark Citation' },
  { id: 'word.references.table-of-authorities.insert-table-of-authorities', label: 'Insert Table of Authorities' },
  { id: 'word.references.table-of-authorities.update-table', label: 'Update Table', icon: 'document-sync' },
];

/**
 * Word's Acronyms group: one command, which opens the Acronyms pane. `GUESS:` the group sits last. See
 * this section's header.
 *
 * **No survivor**: a pane, no glyph, and the only command.
 */
const wordReferencesAcronyms: readonly RibbonCommand[] = [
  { id: 'word.references.acronyms.acronyms', label: 'Acronyms' },
];

/**
 * PowerPoint's Preview group: one command, which plays this slide's transition.
 *
 * **Preview draws `slide-transition`**, a slide sliding off its neighbours: the thing being previewed.
 * Not `play`, which is a video's, and not `slide-play`, which is From Current Slide.
 *
 * **No survivor**: the only command.
 */
const powerpointTransitionsPreview: readonly RibbonCommand[] = [
  { id: 'powerpoint.transitions.preview.preview', label: 'Preview', icon: 'slide-transition', size: 'large' },
];

/**
 * PowerPoint's `GroupTransitionStyles`, which Office labels **Transition to This Slide**: the gallery
 * and Effect Options.
 *
 * **The gallery is in-ribbon**, and the hosts bind `<mjx-gallery>`. Its accessible name is Office's,
 * *Transition to This Slide*. **Effect Options is a dropdown** whose entries depend on the transition;
 * the hosts start the gallery on **Fade**, so Effect Options offers Smoothly and Through Black.
 * `GUESS:` Fade rather than a new deck's None, where Office disables Effect Options and there would be
 * nothing to audit.
 *
 * **No survivor**: a gallery and a menu.
 */
const powerpointTransitionsTransitionStyles: readonly RibbonCommand[] = [
  { id: 'powerpoint.transitions.transition-styles.transitions', label: 'Transition to This Slide' },
  { id: 'powerpoint.transitions.transition-styles.effect-options', label: 'Effect Options' },
];

/**
 * PowerPoint's `GroupTransitionToThisSlide`, labelled **Timing**: Sound, Duration, Apply To All, then
 * On Mouse Click, After and the time after which the slide advances.
 *
 * ⚠ **Six commands where the census counts two.** See this section's header.
 *
 * **Sound is a dropdown field** starting on *[No Sound]*. **Duration is a combo box** of seconds,
 * starting on Fade's 00.70. **Apply To All** is a button. **On Mouse Click and After are checkboxes**,
 * declared as toggles because each is a state: On Mouse Click starts ticked, as in a new deck.
 * **Advance Slide After is a combo box** of times. `GUESS:` its name, which joins Office's *Advance
 * Slide* heading to the checkbox beside it, because Office's spin box has no visible name of its own.
 *
 * **No survivor**: Apply To All has no glyph, and the other five are fields a host binds.
 */
const powerpointTransitionsTiming: readonly RibbonCommand[] = [
  { id: 'powerpoint.transitions.timing.sound', label: 'Sound' },
  { id: 'powerpoint.transitions.timing.duration', label: 'Duration' },
  { id: 'powerpoint.transitions.timing.apply-to-all', label: 'Apply To All' },
  { id: 'powerpoint.transitions.timing.on-mouse-click', label: 'On Mouse Click', toggle: true, pressed: true },
  { id: 'powerpoint.transitions.timing.after', label: 'After', toggle: true },
  { id: 'powerpoint.transitions.timing.advance-after', label: 'Advance Slide After' },
];

/**
 * Excel's Function Library: Insert Function, AutoSum, Recently Used, the six categories, More Functions.
 *
 * **Insert Function draws `math-formula`**, *fx*, Office's own mark, and opens the Insert Function
 * dialog. **AutoSum draws `autosum`** and is a split button (Sum, Average, Count Numbers, Max, Min),
 * as on Home. The other eight are dropdowns of function names. See this section's header for the
 * books, and for why three categories are `small`. `GUESS:` More Functions fits a large button;
 * *Functions* is nine letters and nobody has measured it.
 *
 * **No survivor**: a dialog, a split button and eight menus.
 */
const excelFormulasFunctionLibrary: readonly RibbonCommand[] = [
  { id: 'excel.formulas.function-library.insert-function', label: 'Insert Function', icon: 'math-formula', size: 'large' },
  { id: 'excel.formulas.function-library.autosum', label: 'AutoSum', icon: 'autosum', size: 'large' },
  { id: 'excel.formulas.function-library.recently-used', label: 'Recently Used', icon: 'book-star', size: 'large' },
  { id: 'excel.formulas.function-library.financial', label: 'Financial', icon: 'book-coins', size: 'large' },
  { id: 'excel.formulas.function-library.logical', label: 'Logical', icon: 'book-question-mark', size: 'large' },
  { id: 'excel.formulas.function-library.text', label: 'Text', icon: 'book-letter', size: 'large' },
  { id: 'excel.formulas.function-library.date-time', label: 'Date & Time', icon: 'book-clock' },
  { id: 'excel.formulas.function-library.lookup-reference', label: 'Lookup & Reference', icon: 'book-search' },
  { id: 'excel.formulas.function-library.math-trig', label: 'Math & Trig', icon: 'book-theta' },
  { id: 'excel.formulas.function-library.more-functions', label: 'More Functions', icon: 'book', size: 'large' },
];

/**
 * Excel's `GroupNamedCells`, which Office labels **Defined Names**: Name Manager large, then Define
 * Name, Use in Formula and Create from Selection.
 *
 * **Name Manager draws `tag-multiple`**, a stack of name tags, and opens its dialog. **Define Name draws
 * `tag-add`** and is a split button (Define Name, Apply Names). **Use in Formula** is a dropdown of
 * the workbook's names, the ones the formula bar's name box lists. **Create from Selection** opens a
 * dialog. `GUESS:` both tag glyphs; Office draws a tag over a table.
 *
 * **No survivor**: two dialogs, a split button and a menu.
 */
const excelFormulasNamedCells: readonly RibbonCommand[] = [
  { id: 'excel.formulas.named-cells.name-manager', label: 'Name Manager', icon: 'tag-multiple', size: 'large' },
  { id: 'excel.formulas.named-cells.define-name', label: 'Define Name', icon: 'tag-add' },
  { id: 'excel.formulas.named-cells.use-in-formula', label: 'Use in Formula' },
  { id: 'excel.formulas.named-cells.create-from-selection', label: 'Create from Selection' },
];

/**
 * Excel's Formula Auditing: Trace Precedents, Trace Dependents, Remove Arrows, then Show Formulas,
 * Error Checking, Evaluate Formula, then Watch Window.
 *
 * **Remove Arrows and Error Checking are split buttons.** Remove Arrows offers the precedent and
 * dependent arrows alone. Error Checking offers Trace Error and Circular References, and **draws
 * `checkmark-circle-warning`**, a check that found a warning. **Show Formulas is a toggle**: Office
 * draws it pressed while the sheet shows formulas. Evaluate Formula opens a dialog and Watch Window a
 * window, so both are the generic button. **Watch Window carries no icon**, so it is `small` where
 * Office draws it large.
 *
 * **No survivor**: see this section's header on the trace arrows and Show Formulas.
 */
const excelFormulasFormulaAuditing: readonly RibbonCommand[] = [
  { id: 'excel.formulas.formula-auditing.trace-precedents', label: 'Trace Precedents' },
  { id: 'excel.formulas.formula-auditing.trace-dependents', label: 'Trace Dependents' },
  { id: 'excel.formulas.formula-auditing.remove-arrows', label: 'Remove Arrows' },
  { id: 'excel.formulas.formula-auditing.show-formulas', label: 'Show Formulas', toggle: true },
  { id: 'excel.formulas.formula-auditing.error-checking', label: 'Error Checking', icon: 'checkmark-circle-warning' },
  { id: 'excel.formulas.formula-auditing.evaluate-formula', label: 'Evaluate Formula' },
  { id: 'excel.formulas.formula-auditing.watch-window', label: 'Watch Window' },
];

/**
 * Excel's Calculation group: Calculation Options, Calculate Now, Calculate Sheet.
 *
 * **Calculation Options draws `calculator`** and is a dropdown (Automatic, Automatic Except for Data
 * Tables, Manual). It is `small`: see this section's header. `GUESS:` the middle entry's name; recent
 * builds call it *Partial*. **Calculate Now draws `calculator-arrow-clockwise`**, a calculator running
 * again, and **Calculate Sheet draws `table-calculator`**, a calculator over a sheet.
 *
 * **No survivor**: a menu, and two recalculations that cannot be undone.
 */
const excelFormulasCalculation: readonly RibbonCommand[] = [
  { id: 'excel.formulas.calculation.calculation-options', label: 'Calculation Options', icon: 'calculator' },
  { id: 'excel.formulas.calculation.calculate-now', label: 'Calculate Now', icon: 'calculator-arrow-clockwise' },
  { id: 'excel.formulas.calculation.calculate-sheet', label: 'Calculate Sheet', icon: 'table-calculator' },
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
// ## One survivor on the whole tab, and it is AutoSave
//
// Unit 1 gave every File group a survivor — Protect, Browse, Save, Print, Share, Create PDF/XPS,
// Help — and unit 2b re-judged all seven against `demotionRules` on the shape Office draws them in.
// **Six of them fail rule 1.** A File page is a list of *destinations*: Protect is a menu, Browse
// the system file dialog, Share and Create PDF/XPS dialogs, Help a pane, and Save and Print are
// irreversible — nothing takes back a save that wrote over the file, or a page that left the
// printer. **AutoSave passes all four**: one press, one more press takes it back, a glyph that is
// the sync it performs (rule 2's standard is stated once, in the Home section's header, beside
// Reset's refusal), and it is a state Office keeps in the title bar at every width. So in all three applications
// Save is the only File group with a survivor, and every other File group collapses to its trigger
// alone — which `demotionRules` allows and which a collapsed destination list genuinely is.
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
 * **No survivor.** Unit 1 declared Browse, and Browse opens the system's file dialog — rule 1's
 * dialog, one level further out. Recent, Shared with Me, OneDrive and This PC each open a list to
 * choose from, so on this page the choice always comes *after* the press, and nothing here is a
 * command that does one thing on its own.
 */
function fileOpenCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [
    { id: `${application}.file.open.recent`, label: 'Recent', icon: 'history', size: 'large' },
    { id: `${application}.file.open.shared-with-me`, label: 'Shared with Me', icon: 'people' },
    { id: `${application}.file.open.onedrive`, label: 'OneDrive', icon: 'cloud' },
    { id: `${application}.file.open.this-pc`, label: 'This PC', icon: 'desktop' },
    { id: `${application}.file.open.browse`, label: 'Browse', icon: 'folder-open' },
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
 *
 * **AutoSave is also the File tab's only survivor**, and Save is not — the reversal of unit 1.
 * Demotion rule 1 asks for *immediate and reversible*: pressing AutoSave again takes it back, and
 * nothing takes back a save, which writes over the file on disk (and on a document never saved,
 * opens Save As). Save As and Save a Copy open a dialog.
 *
 * AutoSave is `small`, as it was before it survived anything: the File page draws its name, and a
 * survivor keeps its own size in the survivor row, so it keeps its name there too.
 */
function fileSaveCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [
    { id: `${application}.file.save.save`, label: 'Save', icon: 'save', size: 'large' },
    { id: `${application}.file.save.save-as`, label: 'Save As', icon: 'save-edit' },
    { id: `${application}.file.save.save-a-copy`, label: 'Save a Copy', icon: 'save-copy' },
    { id: `${application}.file.save.autosave`, label: 'AutoSave', icon: 'arrow-sync', toggle: true, pressed: true, essential: true },
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
 *
 * **No survivor.** Print is the only verb and it is irreversible — no undo takes a page back out of
 * a printer — so it fails rule 1; Printer and Copies are fields and Settings opens a list.
 */
function filePrintCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [
    { id: `${application}.file.print.print`, label: 'Print', icon: 'print', size: 'large' },
    { id: `${application}.file.print.printer`, label: 'Printer' },
    { id: `${application}.file.print.copies`, label: 'Copies' },
    { id: `${application}.file.print.settings`, label: 'Settings', icon: 'settings' },
  ];
}

/**
 * Help, and the four places Office's Help page actually goes.
 *
 * **No survivor**: all five open somewhere — a pane, a form, a page, a dialog — which is what a
 * destination is, and none of them is a command that does one thing in place.
 */
function fileHelpCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [
    { id: `${application}.file.help.help`, label: 'Help', icon: 'question-circle', size: 'large' },
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
 *
 * **No survivor.** Protect, Check for Issues and Manage are each a menu in Office — Protect's holds
 * Always Open Read-Only, Encrypt with Password and four more — and Properties has no icon. Unit 1
 * declared Protect essential, which put a menu inside the collapsed group's popup.
 */
function fileInfoCommands(
  application: RibbonApplication,
  noun: string,
  additional: readonly RibbonCommand[] = [],
): readonly RibbonCommand[] {
  return [
    { id: `${application}.file.info.protect`, label: `Protect ${noun}`, icon: 'document-lock' },
    { id: `${application}.file.info.check-for-issues`, label: 'Check for Issues', icon: 'document-search' },
    { id: `${application}.file.info.manage`, label: `Manage ${noun}`, icon: 'history' },
    ...additional,
    { id: `${application}.file.info.properties`, label: 'Properties' },
  ];
}

const wordFileShare: readonly RibbonCommand[] = [
  { id: 'word.file.share.share', label: 'Share', icon: 'share', size: 'large' },
  { id: 'word.file.share.email', label: 'Email', icon: 'mail' },
  { id: 'word.file.share.get-a-link', label: 'Get a Link', icon: 'link' },
  { id: 'word.file.share.present-online', label: 'Present Online', icon: 'presenter' },
];

const wordFileExport: readonly RibbonCommand[] = [
  { id: 'word.file.export.pdf', label: 'Create PDF/XPS Document', icon: 'document-pdf' },
  { id: 'word.file.export.change-file-type', label: 'Change File Type', icon: 'arrow-swap' },
];

const powerpointFileShare: readonly RibbonCommand[] = [
  { id: 'powerpoint.file.share.share', label: 'Share', icon: 'share', size: 'large' },
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
  { id: 'powerpoint.file.export.pdf', label: 'Create PDF/XPS Document', icon: 'document-pdf' },
  { id: 'powerpoint.file.export.video', label: 'Create a Video', icon: 'video' },
  { id: 'powerpoint.file.export.package-for-cd', label: 'Package Presentation for CD' },
  { id: 'powerpoint.file.export.handouts', label: 'Create Handouts' },
  { id: 'powerpoint.file.export.change-file-type', label: 'Change File Type', icon: 'arrow-swap' },
];

/** Excel's Share page is short because Excel has a Publish page beside it. See `excelFilePublish`. */
const excelFileShare: readonly RibbonCommand[] = [
  { id: 'excel.file.share.share', label: 'Share', icon: 'share', size: 'large' },
  { id: 'excel.file.share.email', label: 'Email', icon: 'mail' },
  { id: 'excel.file.share.get-a-link', label: 'Get a Link', icon: 'link' },
];

const excelFileExport: readonly RibbonCommand[] = [
  { id: 'excel.file.export.pdf', label: 'Create PDF/XPS Document', icon: 'document-pdf' },
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
      { id: 'GroupInsertPages', label: 'Pages', priority: 'standard', controls: 13, inScope: true, commands: wordInsertPages },
      { id: 'GroupInsertTables', label: 'Tables', priority: 'primary', controls: 7, inScope: true, commands: wordInsertTables },
      { id: 'GroupInsertIllustrations', label: 'Illustrations', priority: 'primary', controls: 23, inScope: true, commands: wordInsertIllustrations },
      { id: 'GroupMedia', label: 'Media', priority: 'ancillary', controls: 1, inScope: true, commands: wordInsertMedia },
      { id: 'GroupInsertLinks', label: 'Links', priority: 'standard', controls: 5, inScope: true, commands: wordInsertLinks },
      { id: 'GroupInsertComments', label: 'Comments', priority: 'ancillary', controls: 1, inScope: true, commands: insertCommentsCommands('word') },
      { id: 'GroupHeaderFooter', label: 'Header & Footer', priority: 'standard', controls: 32, inScope: true, commands: wordInsertHeaderFooter },
      { id: 'GroupInsertText', label: 'Text', priority: 'standard', controls: 26, inScope: true, commands: wordInsertText },
      { id: 'GroupInsertSymbols', label: 'Symbols', priority: 'standard', controls: 7, inScope: true, commands: insertSymbolsCommands('word') },
    ],
  },
  {
    id: 'draw',
    label: 'Draw',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabDrawInk' },
    groups: [
      { id: 'GroupDrawingTools', label: 'Drawing Tools', priority: 'ancillary', controls: 1, inScope: true, commands: drawDrawingToolsCommands('word') },
      { id: 'GroupPens2', label: 'Pens', priority: 'primary', controls: 6, inScope: true, commands: drawPensCommands('word') },
      { id: 'GroupWrite', label: 'Write', priority: 'primary', controls: 10, inScope: true, commands: drawWriteCommands('word') },
      { id: 'GroupStencils', label: 'Stencils', priority: 'ancillary', controls: 1, inScope: true, commands: drawStencilsCommands('word') },
      { id: 'GroupEditingExcel', label: 'Editing', priority: 'standard', controls: 6, inScope: true, commands: wordDrawEditing },
      { id: 'GroupInsertDrawingCanvas', label: 'Drawing Canvas', priority: 'ancillary', controls: 1, inScope: true, commands: wordDrawDrawingCanvas },
      { id: 'GroupInputMode', label: 'Input Mode', priority: 'standard', controls: 3, inScope: true, commands: drawInputModeCommands('word') },
      { id: 'GroupDrawWithTouch', label: 'Draw with Touch', priority: 'ancillary', controls: 1, inScope: true, commands: drawWithTouchCommands('word') },
      { id: 'InkReplay', label: 'Replay', priority: 'ancillary', controls: 1, inScope: true, commands: drawReplayCommands('word') },
      { id: 'GroupPenAndInkHelp', label: 'Help', priority: 'ancillary', controls: 1, inScope: true, commands: drawHelpCommands('word') },
      { id: 'GroupInkClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true, commands: drawCloseCommands('word') },
    ],
  },
  {
    id: 'design',
    label: 'Design',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabWordDesign' },
    groups: [
      { id: 'GroupStyleSet', label: 'Style Set', priority: 'primary', controls: 16, inScope: true, commands: wordDesignStyleSet },
      { id: 'GroupPageBackground', label: 'Page Background', priority: 'standard', controls: 9, inScope: true, commands: wordDesignPageBackground },
    ],
  },
  {
    id: 'layout',
    label: 'Layout',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabPageLayoutWord' },
    groups: [
      { id: 'GroupPageLayoutSetup', label: 'Page Setup', priority: 'primary', controls: 23, inScope: true, commands: wordLayoutPageSetup },
      { id: 'GroupParagraphLayout', label: 'Paragraph', priority: 'standard', controls: 7, inScope: true, commands: wordLayoutParagraph },
      { id: 'GroupArrange', label: 'Arrange', priority: 'standard', controls: 65, inScope: true, commands: arrangeCommands('word') },
    ],
  },
  {
    id: 'references',
    label: 'References',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabReferences' },
    groups: [
      { id: 'GroupTableOfContents', label: 'Table of Contents', priority: 'primary', controls: 6, inScope: true, commands: wordReferencesTableOfContents },
      { id: 'GroupFootnotes', label: 'Footnotes', priority: 'primary', controls: 9, inScope: true, commands: wordReferencesFootnotes },
      { id: 'GroupCitationsAndBibliography', label: 'Citations & Bibliography', priority: 'standard', controls: 8, inScope: true, commands: wordReferencesCitationsBibliography },
      { id: 'GroupCaptions', label: 'Captions', priority: 'standard', controls: 4, inScope: true, commands: wordReferencesCaptions },
      { id: 'GroupIndex', label: 'Index', priority: 'standard', controls: 3, inScope: true, commands: wordReferencesIndex },
      { id: 'GroupTableOfAuthorities', label: 'Table of Authorities', priority: 'standard', controls: 3, inScope: true, commands: wordReferencesTableOfAuthorities },
      { id: 'GroupAcronyms', label: 'Acronyms', priority: 'ancillary', controls: 1, inScope: true, commands: wordReferencesAcronyms },
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
      { id: 'GroupSlides2', label: 'Slides', priority: 'standard', controls: 7, inScope: true, commands: powerpointInsertSlides },
      { id: 'GroupInsertTables', label: 'Tables', priority: 'standard', controls: 4, inScope: true, commands: powerpointInsertTables },
      { id: 'GroupImages', label: 'Images', priority: 'primary', controls: 10, inScope: true, commands: powerpointInsertImages },
      { id: 'GroupInsertIllustrations', label: 'Illustrations', priority: 'primary', controls: 11, inScope: true, commands: powerpointInsertIllustrations },
      { id: 'GroupChunkCameoCamera', label: 'Camera', priority: 'standard', controls: 3, inScope: true, commands: powerpointInsertCamera },
      { id: 'GroupInsertLinks', label: 'Links', priority: 'standard', controls: 7, inScope: true, commands: powerpointInsertLinks },
      { id: 'GroupInsertComments', label: 'Comments', priority: 'ancillary', controls: 1, inScope: true, commands: insertCommentsCommands('powerpoint') },
      { id: 'GroupInsertText', label: 'Text', priority: 'standard', controls: 9, inScope: true, commands: powerpointInsertText },
      { id: 'GroupInsertSymbols', label: 'Symbols', priority: 'standard', controls: 4, inScope: true, commands: insertSymbolsCommands('powerpoint') },
      { id: 'GroupInsertMediaClips', label: 'Media Clips', priority: 'standard', controls: 9, inScope: true, commands: powerpointInsertMediaClips },
      { id: 'GroupContent', label: 'Content', priority: 'ancillary', controls: 1, inScope: true, commands: powerpointInsertContent },
    ],
  },
  {
    id: 'draw',
    label: 'Draw',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabDrawInk' },
    groups: [
      { id: 'GroupDrawingTools', label: 'Drawing Tools', priority: 'ancillary', controls: 1, inScope: true, commands: drawDrawingToolsCommands('powerpoint') },
      { id: 'GroupPens2', label: 'Pens', priority: 'primary', controls: 7, inScope: true, commands: drawPensCommands('powerpoint') },
      { id: 'GroupWrite', label: 'Write', priority: 'primary', controls: 10, inScope: true, commands: drawWriteCommands('powerpoint') },
      { id: 'GroupStencils', label: 'Stencils', priority: 'ancillary', controls: 1, inScope: true, commands: drawStencilsCommands('powerpoint') },
      { id: 'GroupInputMode', label: 'Input Mode', priority: 'standard', controls: 3, inScope: true, commands: drawInputModeCommands('powerpoint') },
      { id: 'GroupDrawWithTouch', label: 'Draw with Touch', priority: 'ancillary', controls: 1, inScope: true, commands: drawWithTouchCommands('powerpoint') },
      { id: 'InkReplay', label: 'Replay', priority: 'ancillary', controls: 1, inScope: true, commands: drawReplayCommands('powerpoint') },
      { id: 'GroupPenAndInkHelp', label: 'Help', priority: 'ancillary', controls: 1, inScope: true, commands: drawHelpCommands('powerpoint') },
      { id: 'GroupInkClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true, commands: drawCloseCommands('powerpoint') },
    ],
  },
  {
    id: 'design',
    label: 'Design',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabDesign' },
    groups: [
      { id: 'GroupSlideThemes', label: 'Themes', priority: 'primary', controls: 4, inScope: true, commands: powerpointDesignThemes },
      { id: 'GroupThemeVariants', label: 'Variants', priority: 'primary', controls: 10, inScope: true, commands: powerpointDesignVariants },
      { id: 'GroupCustomizeThemeOptions', label: 'Customise', priority: 'standard', controls: 3, inScope: true, commands: powerpointDesignCustomise },
    ],
  },
  {
    id: 'transitions',
    label: 'Transitions',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabTransitions' },
    groups: [
      { id: 'GroupPreviewTransitions', label: 'Preview', priority: 'ancillary', controls: 1, inScope: true, commands: powerpointTransitionsPreview },
      { id: 'GroupTransitionStyles', label: 'Transition Styles', priority: 'primary', controls: 8, inScope: true, commands: powerpointTransitionsTransitionStyles },
      { id: 'GroupTransitionToThisSlide', label: 'Timing', priority: 'secondary', controls: 2, inScope: true, commands: powerpointTransitionsTiming },
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
      { id: 'GroupInsertTablesExcel', label: 'Tables', priority: 'primary', controls: 17, inScope: true, commands: excelInsertTables },
      { id: 'GroupInsertIllustrations', label: 'Illustrations', priority: 'standard', controls: 28, inScope: true, commands: excelInsertIllustrations },
      { id: 'GroupInsertChartsExcel', label: 'Charts', priority: 'primary', controls: 25, inScope: true, commands: excelInsertCharts },
      { id: 'GroupSparklinesInsert', label: 'Sparklines', priority: 'standard', controls: 3, inScope: true, commands: excelInsertSparklines },
      { id: 'GroupSlicerInsert', label: 'Slicers', priority: 'ancillary', controls: 2, inScope: true, commands: excelInsertSlicers },
      { id: 'GroupInsertLinks', label: 'Links', priority: 'ancillary', controls: 2, inScope: true, commands: excelInsertLinks },
      { id: 'GroupInsertComments', label: 'Comments', priority: 'ancillary', controls: 1, inScope: true, commands: insertCommentsCommands('excel') },
      { id: 'GroupInsertText', label: 'Text', priority: 'standard', controls: 10, inScope: true, commands: excelInsertText },
      { id: 'GroupInsertSymbols', label: 'Symbols', priority: 'standard', controls: 4, inScope: true, commands: insertSymbolsCommands('excel') },
      { id: 'GroupCellControls', label: 'Cell Controls', priority: 'standard', controls: 3, inScope: true, commands: excelInsertCellControls },
    ],
  },
  {
    id: 'draw',
    label: 'Draw',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabDrawInk' },
    groups: [
      { id: 'GroupDrawingTools', label: 'Drawing Tools', priority: 'ancillary', controls: 1, inScope: true, commands: drawDrawingToolsCommands('excel') },
      { id: 'GroupPens2', label: 'Pens', priority: 'primary', controls: 6, inScope: true, commands: drawPensCommands('excel') },
      { id: 'GroupWrite', label: 'Write', priority: 'primary', controls: 5, inScope: true, commands: drawWriteCommands('excel') },
      { id: 'GroupInputMode', label: 'Input Mode', priority: 'standard', controls: 3, inScope: true, commands: drawInputModeCommands('excel') },
      { id: 'GroupDrawWithTouch', label: 'Draw with Touch', priority: 'ancillary', controls: 1, inScope: true, commands: drawWithTouchCommands('excel') },
      { id: 'InkReplay', label: 'Replay', priority: 'ancillary', controls: 1, inScope: true, commands: drawReplayCommands('excel') },
      { id: 'GroupPenAndInkHelp', label: 'Help', priority: 'ancillary', controls: 1, inScope: true, commands: drawHelpCommands('excel') },
      { id: 'GroupInkClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true, commands: drawCloseCommands('excel') },
    ],
  },
  {
    id: 'page-layout',
    label: 'Page Layout',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabPageLayoutExcel' },
    groups: [
      { id: 'GroupThemesExcel', label: 'Themes', priority: 'standard', controls: 9, inScope: true, commands: excelPageLayoutThemes },
      { id: 'GroupPageSetup', label: 'Page Setup', priority: 'primary', controls: 16, inScope: true, commands: excelPageLayoutPageSetup },
      { id: 'GroupPageLayoutScaleToFit', label: 'Scale to Fit', priority: 'standard', controls: 6, inScope: true, commands: excelPageLayoutScaleToFit },
      { id: 'GroupPageLayoutSheetOptions', label: 'Sheet Options', priority: 'standard', controls: 8, inScope: true, commands: excelPageLayoutSheetOptions },
      { id: 'GroupArrange', label: 'Arrange', priority: 'standard', controls: 47, inScope: true, commands: arrangeCommands('excel') },
    ],
  },
  {
    id: 'formulas',
    label: 'Formulas',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabFormulas' },
    groups: [
      { id: 'GroupFunctionLibrary', label: 'Function Library', priority: 'primary', controls: 37, inScope: true, commands: excelFormulasFunctionLibrary },
      { id: 'GroupNamedCells', label: 'Named Cells', priority: 'standard', controls: 7, inScope: true, commands: excelFormulasNamedCells },
      { id: 'GroupFormulaAuditing', label: 'Formula Auditing', priority: 'standard', controls: 13, inScope: true, commands: excelFormulasFormulaAuditing },
      { id: 'GroupCalculation', label: 'Calculation', priority: 'standard', controls: 7, inScope: true, commands: excelFormulasCalculation },
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
 * The commands a group shows that survive a collapse, in declared order.
 *
 * **Declared, never inferred.** This used to count every toggle as essential, because
 * `shell-parts.ts`'s `toggle()` put every toggle in the survivor slot; a state command is not a
 * survivor, and the two are now independent. `essentialCommandLimit` is the ceiling, and
 * `tests/ribbons.test.ts` holds it.
 */
export function essentialCommands(group: RibbonGroupEntry): readonly RibbonCommand[] {
  return (group.commands ?? []).filter((command) => command.essential === true);
}

/** Every command declared anywhere in the census, for the gates that sweep all of them. */
export function everyRibbonCommand(): readonly RibbonCommand[] {
  return ribbonApplicationNames.flatMap((application) =>
    ribbonCensus[application].flatMap((tab) =>
      tab.groups.flatMap((group) => group.commands ?? []),
    ),
  );
}
