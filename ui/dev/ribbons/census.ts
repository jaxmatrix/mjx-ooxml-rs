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
 * **File, Home, Insert, Draw, the Design and Layout tabs, References, Transitions and Formulas, Mailings, Animations and Data, and Review in all three applications.** Unit 0 was the scaffold: the three Home tabs held the commands migrated
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
 * Formulas**; see the *commands References, Transitions and Formulas show* section. Unit 7 authored **Word's
 * Mailings, PowerPoint's Animations and Excel's Data**; see the *commands Mailings, Animations and Data show*
 * section. Unit 8 authored **Word's Review** alone, the first unit narrowed to one tab of one application;
 * see the *commands Review shows* section. **PowerPoint's Review** and **Excel's Review** followed, one tab of
 * one application each, in that section's *PowerPoint's Review* and *Excel's Review* parts. **Word's View**
 * followed, one tab of one application again; see the *commands View shows* section. **PowerPoint's View**
 * followed it, in that section's *PowerPoint's View* part, and **Excel's View** followed that, in its *Excel's View*
 * part. **PowerPoint's Slide Show** followed the three View tabs, one tab of one application again; see the
 * *commands Slide Show shows* section. **PowerPoint's Recording** followed Slide Show, one tab of one application
 * again; see the *commands Recording shows* section. **Word's Outlining** followed Recording, the first view tab
 * authored; see the *commands Outlining shows* section. **Word's Print Preview** followed Outlining, the second
 * view tab authored and the first with menus; see the *commands Print Preview shows* section. **Word's Background
 * Removal** followed Print Preview, the third view tab authored, its two groups written once as functions of the
 * application; see the *commands Background Removal shows* section. **PowerPoint's Background Removal**
 * followed, calling those two functions, in that section's *PowerPoint's Background Removal* part, and **Excel's
 * Background Removal** followed that, in its *Excel's Background Removal* part. **PowerPoint's Print Preview**
 * followed, PowerPoint's second view tab authored, in the *commands Print Preview shows* section's *PowerPoint's
 * Print Preview* part, and **Excel's Print Preview** followed that, Excel's second view tab authored, in its
 * *Excel's Print Preview* part. **PowerPoint's Slide Master** followed, PowerPoint's third view tab authored, its
 * Edit Theme, Background and Close groups written once as functions of the master view; see the *commands the
 * master views show* section. **PowerPoint's Slide Master Home** followed, PowerPoint's fourth view tab authored,
 * its Clipboard, Font, Paragraph, Drawing and Editing groups written once as functions of the Home tab they share;
 * see that section's *PowerPoint's Slide Master Home* part. **PowerPoint's Handout Master** followed, PowerPoint's
 * fifth view tab authored, calling Slide Master's Edit Theme, Background and Close functions; see that section's
 * *PowerPoint's Handout Master* part. **PowerPoint's Notes Master** followed, PowerPoint's sixth view tab
 * authored, calling the same three functions; see that section's *PowerPoint's Notes Master* part. **PowerPoint's
 * Black and White** followed, PowerPoint's seventh view tab authored, its Colour Mode and Close groups written once
 * as functions of the colour-mode tab it shares with Greyscale; see the *commands the colour modes show* section.
 * **PowerPoint's Greyscale** followed, PowerPoint's eighth and last view tab authored, calling the same two
 * functions; see that section's *PowerPoint's Greyscale* part. It was the last core or view placeholder, so every
 * in-scope core and view tab of the three applications now carries its commands. **The contextual tabs follow**:
 * see *The contextual tab sets* below, which declares their groups so each can be authored one tab of one
 * application at a time. **Word's Table Design** was the first, its menus and gallery art written once in
 * `stories/ribbons/table-tools-menus.ts` for PowerPoint's and Excel's Table Design units; see the *commands Table
 * Design shows* section. **Word's Table Layout** followed, the second, its Select and Delete lists written in that
 * file for PowerPoint's Table Layout to reuse; see the *commands Table Layout shows* section. **PowerPoint's Table
 * Design** followed, the third, reusing that file's gallery art and line weights and adding PowerPoint's own lists
 * there; its WordArt Styles group is written once as a function of the application and tab, for Shape Format and
 * Chart Format to call; see the *commands Table Design shows* section's *PowerPoint's Table Design* part.
 * **PowerPoint's Table Layout** followed, the fourth, calling Word's Select and Delete lists for PowerPoint and
 * `arrangeCommands` for a table, which it generalised to take the application, the tab and the object; see the
 * *commands Table Layout shows* section's *PowerPoint's Table Layout* part. `commands` stays optional rather than
 * required, because an empty array would claim a tab had been authored and found to hold nothing.
 *
 * ## The contextual tab sets
 *
 * A contextual tab is one Office shows only while something is selected — a table, a picture, a shape, a chart —
 * under a coloured band naming its **set**. The census writes them as rows whose `tab_set` is a `TabSet*` id rather
 * than `None (Core Tab)`, and a set is the unit Office publishes: `TabSetTableTools` is what appears, and its two tabs
 * are what it contains. So the model is a set holding tabs (`RibbonContextualSetEntry`), each tab an ordinary
 * `RibbonTabEntry` whose `source` is `{ kind: 'contextual', tabSet, tab }` and whose `appearance` is `contextual`.
 * `<mjx-contextual-tab-set>` is the same shape, which is why the rendering is a loop.
 *
 * **Only the four common sets are built, and that is the user's decision (2026-09-15)**:
 *
 * | Set | Word | PowerPoint | Excel |
 * |---|---|---|---|
 * | Table Tools | Table Design, Layout | Table Design, Layout | Table Design (`TabSetTableToolsExcel`) |
 * | Picture Tools | Picture Format | Picture Format | Picture Format |
 * | Drawing Tools | Shape Format | Shape Format | Shape Format |
 * | Chart Tools | Chart Design, Format | Chart Design, Format | Chart Design, Format |
 *
 * Every other in-scope `TabSet*` is recorded in `unbuiltContextualSets` with that reason, and
 * `tests/ribbons.test.ts` holds the declared tabs and the record together to the census as a plain equality, so a
 * set is either built or written down and there is no third state.
 *
 * ⚠ **Four things about the transcription, recorded rather than smoothed over:**
 *
 * 1. **Chart Tools is three generations in the census, and two tabs are built.** The set carries
 *    `TabChartToolsDesign`, `TabChartToolsFormat` and `TabChartToolsLayout` — Office 2007–2010's three chart tabs —
 *    beside `TabChartToolsDesignNew` and `TabChartToolsFormatNew`, which are Office 2013's Chart Design and Format.
 *    The user's decision names Chart Design and Format, so the two `New` tabs are declared and the three older ones
 *    are recorded in the set's own `unbuiltTabs` with their own reason. (The older Design tab is a subset of the new
 *    one; the older Layout tab's axes, labels and analysis groups moved into Office 2013's on-chart buttons.)
 * 2. **The group labels are Microsoft 365's, because the inventory names none of these groups** and several ids read
 *    wrongly on their own: `GroupTableLayout` sits on Word's *Table Design* tab and is its Table Style Options
 *    (`GUESS:` from the counts and the order), `GroupPictureTools` is Picture Format's Adjust, and
 *    `GroupTextStylesTable` is PowerPoint's WordArt Styles. `GroupAltText` is *Accessibility*, `GroupShapes` and
 *    `GroupShapesChart` are *Insert Shapes*, and `GroupTextbox` is *Text*. **`GroupImagePlay` is the one whose
 *    Office label is unknown**, so it is derived from the id (*Image Play*) — `GUESS:` that it is the animated-image
 *    play control, one control, drawn by Microsoft 365 only for a moving picture.
 * 3. **Office's tab labels are used, and two of them are ambiguous on purpose**: Table Tools' second tab and Chart
 *    Tools' second tab are both *Layout* and *Format* in Microsoft 365, and Word's core tab strip already has a
 *    *Layout*. `<mjx-ribbon>` announces a contextual tab with its set's name — *Layout, Table Tools* — which is what
 *    disambiguates them, so the labels are Office's rather than invented. The kebab ids are distinct.
 * 4. **The priorities follow the rubric**, and the one place it bites is the census's small counts: Word's and
 *    PowerPoint's Chart Styles counts 2 and Excel's Chart Data counts 2, so neither can be `standard` however much a
 *    person reaches for it. Both are `secondary`, because the chart's own on-canvas buttons reach them.
 *
 * **Four contextual tabs carry commands: Word's and PowerPoint's Table Design and Table Layout.** Each
 * per-tab unit authors one tab of one application,
 * exactly as the view tabs were; until then `stories/ribbons/<app>.ts` renders the tab through `placeholderTab`, at
 * the priority declared here. The two hosts draw different sets: `Ribbons/*` draws all four so each tab has a story, and `Shell/*` draws
 * the one set its document's selection would show (Word's and Excel's Table Tools, PowerPoint's Picture Tools),
 * because Office never shows four sets at once.
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
 *
 * **`contextual` is the third**: a tab Office shows only while its object is selected, under its set's band. It is
 * never in `<app>RibbonTabs`; it lives in a `RibbonContextualSetEntry`, and a host draws its sets separately.
 */
export const tabAppearanceNames = ['always', 'view', 'contextual'] as const;

/** One of the three. */
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
   * **The exclusive set this toggle belongs to**: Office holds exactly one member of the set, so
   * pressing one releases the others and pressing the one that holds keeps it. The name is
   * `<app>.<tab>.<group>.<set>`-shaped and shared by every member. `renderCommand` writes it onto the
   * generic toggle as `exclusive`, and a host that binds its own control writes the same attribute.
   *
   * Every member is a `toggle`, every member lives in one tab (the set is looked up in its tab), and
   * **exactly one member starts `pressed`**. `tests/ribbons.test.ts` holds all three and the host
   * bindings. The mechanism is `src/controls/exclusive-set.ts`.
   *
   * **A set may instead hold at most one**: see `exclusiveAllowsNone`.
   */
  readonly exclusive?: string;
  /**
   * **The exclusive set may hold none.** Pressing the member that holds releases it, so the set is empty,
   * and the set may start with no member pressed. Written as the boolean `exclusive-allows-none`.
   *
   * For a set with no member standing for *no tool*: Background Removal's two marking pencils, where the
   * ordinary pointer is not a command on the tab (the Draw tab's is Select Objects). Every member of the
   * set declares it or none does, it means nothing without `exclusive`, and a set that declares it starts
   * with **at most one** member pressed. `tests/ribbons.test.ts` holds all three.
   */
  readonly exclusiveAllowsNone?: boolean;
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
 * is what lets one test assert a genuine equality for all three shapes.
 *
 * The third is a **contextual** tab: a census tab inside a `TabSet*`. The set is part of the address even though no
 * contextual tab id repeats across sets within one application today, because a group's identity is only checkable
 * against rows whose two columns *both* agree: a tab moved to the wrong set would otherwise pass. The instrument test
 * in `tests/ribbons.test.ts` watches exactly that refusal fire.
 */
export type RibbonTabSource =
  /** One core tab. A group entry's `id` is a census group id inside it. */
  | { readonly kind: 'core'; readonly tab: string }
  /** The File tab. A group entry's `id` is a backstage census *tab* id. */
  | { readonly kind: 'backstage' }
  /** One contextual tab of one set. A group entry's `id` is a census group id inside it. */
  | { readonly kind: 'contextual'; readonly tabSet: string; readonly tab: string };

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

/** One census tab of a set that is deliberately not declared, and why. */
export interface UnbuiltContextualTab {
  /** The census's tab id — `TabChartToolsLayout`. */
  readonly tab: string;
  readonly reason: string;
}

/**
 * **One contextual tab set that is built** — its band and the tabs under it, in Office's order.
 *
 * `tabs` is what `<mjx-contextual-tab-set>` wraps. `unbuiltTabs` is every other in-scope census tab of the same
 * set, each with its reason, so the set is accounted for tab by tab: see the *contextual tab sets* section of this
 * file's header on why Chart Tools has three.
 */
export interface RibbonContextualSetEntry {
  /** Kebab case, and what a host names when it draws only some sets — `table-tools`. */
  readonly id: string;
  /** The census's set id — `TabSetTableTools`. Every tab's `source.tabSet` is this. */
  readonly tabSet: string;
  /** What the band over the set's tabs says, in English — *Table Tools*. */
  readonly label: string;
  readonly tabs: readonly RibbonTabEntry[];
  readonly unbuiltTabs: readonly UnbuiltContextualTab[];
}

/** **One in-scope contextual set that is not built**, with every in-scope census tab it carries. */
export interface UnbuiltContextualSet {
  /** The census's set id — `TabSetSmartArtTools`. */
  readonly tabSet: string;
  /** Its name in English, for a reader of the record. */
  readonly label: string;
  /** Every census tab of the set the census marks in scope. */
  readonly tabs: readonly string[];
  readonly reason: string;
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
  /** What every contextual set's `tab_set` starts with. */
  contextualTabSetPrefix: 'TabSet',
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

/**
 * **The two tabs PowerPoint calls Home**: the ordinary `TabHome`, and `TabSlideMasterHome`, the Home tab Slide
 * Master view shows.
 *
 * They share five of their six groups row for row, with the same ids and the same counts (Clipboard 10, Font 18,
 * Paragraph 27, Drawing 63, Editing 8), and Office draws the same commands in all five. So those five are
 * functions of this tab, exactly as `fileOpenCommands` is a function of the application: **written once**, and
 * every command id carries the tab, so the two tabs' ids stay distinct and a host binds each by its own. Home's
 * second group is Slides and Slide Master Home's is Master Slides; each is its own tab's constant. See the
 * *commands the master views show* section's *PowerPoint's Slide Master Home* part.
 */
type PowerpointHomeTab = 'home' | 'slide-master-home';

/**
 * PowerPoint's Clipboard group, on either Home tab. **No survivor**, for the reason this section's header gives.
 */
function powerpointHomeClipboardCommands(tab: PowerpointHomeTab): readonly RibbonCommand[] {
  return [
    { id: `powerpoint.${tab}.clipboard.paste`, label: 'Paste', icon: 'clipboard-paste', size: 'large' },
    { id: `powerpoint.${tab}.clipboard.cut`, label: 'Cut', icon: 'cut' },
    { id: `powerpoint.${tab}.clipboard.copy`, label: 'Copy', icon: 'copy' },
    { id: `powerpoint.${tab}.clipboard.format-painter`, label: 'Format Painter', icon: 'paint-brush' },
  ];
}

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
function powerpointHomeFontCommands(tab: PowerpointHomeTab): readonly RibbonCommand[] {
  return [
    { id: `powerpoint.${tab}.font.name`, label: 'Font' },
    { id: `powerpoint.${tab}.font.size`, label: 'Font size' },
    { id: `powerpoint.${tab}.font.grow`, label: 'Increase Font Size', icon: 'font-increase', size: 'icon' },
    { id: `powerpoint.${tab}.font.shrink`, label: 'Decrease Font Size', icon: 'font-decrease', size: 'icon' },
    { id: `powerpoint.${tab}.font.clear-formatting`, label: 'Clear All Formatting', icon: 'clear-formatting', size: 'icon' },
    { id: `powerpoint.${tab}.font.bold`, label: 'Bold', icon: 'text-bold', size: 'icon', toggle: true, pressed: true, essential: true },
    { id: `powerpoint.${tab}.font.italic`, label: 'Italic', icon: 'text-italic', size: 'icon', toggle: true, essential: true },
    { id: `powerpoint.${tab}.font.underline`, label: 'Underline', icon: 'text-underline', size: 'icon', toggle: true, essential: true },
    { id: `powerpoint.${tab}.font.text-shadow`, label: 'Text Shadow', size: 'small', toggle: true },
    { id: `powerpoint.${tab}.font.strikethrough`, label: 'Strikethrough', icon: 'text-strikethrough', size: 'icon', toggle: true },
    { id: `powerpoint.${tab}.font.character-spacing`, label: 'Character Spacing', icon: 'font-space-tracking-out', size: 'icon' },
    { id: `powerpoint.${tab}.font.change-case`, label: 'Change Case', icon: 'text-change-case', size: 'icon' },
    { id: `powerpoint.${tab}.font.colour`, label: 'Font colour' },
  ];
}

/**
 * PowerPoint's Paragraph group. The list *levels* rather than Word's indents — in a deck an indent
 * is an outline level, and Office names the command accordingly even though the glyph is the same.
 *
 * The last three are the ones a document has no equivalent of: Text Direction rotates a
 * placeholder's text, Align Text is vertical alignment *inside* the placeholder, and Convert to
 * SmartArt turns a bullet list into a diagram.
 */
function powerpointHomeParagraphCommands(tab: PowerpointHomeTab): readonly RibbonCommand[] {
  return [
    { id: `powerpoint.${tab}.paragraph.bullets`, label: 'Bullets', icon: 'text-bullet-list-ltr', size: 'icon' },
    { id: `powerpoint.${tab}.paragraph.numbering`, label: 'Numbering', icon: 'text-number-list-ltr', size: 'icon' },
    { id: `powerpoint.${tab}.paragraph.decrease-list-level`, label: 'Decrease List Level', icon: 'text-indent-decrease', size: 'icon' },
    { id: `powerpoint.${tab}.paragraph.increase-list-level`, label: 'Increase List Level', icon: 'text-indent-increase', size: 'icon' },
    { id: `powerpoint.${tab}.paragraph.line-spacing`, label: 'Line Spacing', icon: 'text-line-spacing', size: 'icon' },
    { id: `powerpoint.${tab}.paragraph.align-left`, label: 'Align left', icon: 'text-align-left', size: 'icon', toggle: true, pressed: true, essential: true },
    { id: `powerpoint.${tab}.paragraph.centre`, label: 'Centre', icon: 'text-align-center', size: 'icon', toggle: true, essential: true },
    { id: `powerpoint.${tab}.paragraph.align-right`, label: 'Align right', icon: 'text-align-right', size: 'icon', toggle: true, essential: true },
    { id: `powerpoint.${tab}.paragraph.justify`, label: 'Justify', icon: 'text-align-justify', size: 'icon', toggle: true },
    { id: `powerpoint.${tab}.paragraph.columns`, label: 'Columns', icon: 'text-column-two', size: 'icon' },
    { id: `powerpoint.${tab}.paragraph.text-direction`, label: 'Text Direction', icon: 'text-direction-rotate-90-right' },
    { id: `powerpoint.${tab}.paragraph.align-text`, label: 'Align Text', icon: 'align-center-vertical' },
    { id: `powerpoint.${tab}.paragraph.smart-art`, label: 'Convert to SmartArt', icon: 'diagram' },
  ];
}

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
function powerpointHomeDrawingCommands(tab: PowerpointHomeTab): readonly RibbonCommand[] {
  return [
    { id: `powerpoint.${tab}.drawing.shapes`, label: 'Shapes', icon: 'shapes' },
    { id: `powerpoint.${tab}.drawing.arrange`, label: 'Arrange', icon: 'layer' },
    { id: `powerpoint.${tab}.drawing.styles`, label: 'Shape styles' },
    { id: `powerpoint.${tab}.drawing.fill`, label: 'Shape Fill', icon: 'color-fill' },
    { id: `powerpoint.${tab}.drawing.outline`, label: 'Shape Outline', icon: 'color-line' },
    { id: `powerpoint.${tab}.drawing.effects`, label: 'Shape Effects', icon: 'square-shadow' },
  ];
}

/** No survivor: PowerPoint's Find opens a dialog, Replace is a split button and Select a menu. */
function powerpointHomeEditingCommands(tab: PowerpointHomeTab): readonly RibbonCommand[] {
  return [
    { id: `powerpoint.${tab}.editing.find`, label: 'Find', icon: 'search' },
    { id: `powerpoint.${tab}.editing.replace`, label: 'Replace', icon: 'arrow-swap' },
    { id: `powerpoint.${tab}.editing.select`, label: 'Select', icon: 'select-all-on' },
  ];
}

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
 * **One declaration, two tabs.** `GroupChunkCameoCamera` is a *chunk* in the census's own spelling: Office
 * places the same group on Insert and on Recording, so both call this with their tab id and differ in
 * nothing else. `stories/ribbons/insert-menus.ts`'s `cameoEntries` is the menu both open.
 *
 * **No survivor**: a split button, and the only command.
 */
function cameraCommands(tab: 'insert' | 'recording'): readonly RibbonCommand[] {
  return [{ id: `powerpoint.${tab}.camera.cameo`, label: 'Cameo', icon: 'camera', size: 'large' }];
}

const powerpointInsertCamera: readonly RibbonCommand[] = cameraCommands('insert');

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
 * All five are toggles, because Office draws the current tool pressed, and **one exclusive set**,
 * `<app>.draw.write.tools`, because Office holds one tool at a time: pressing Pen releases Select
 * Objects, and pressing the tool that holds keeps it. **Select Objects starts pressed**, since a person
 * arriving on the tab has not picked up a pen yet. Word's and PowerPoint's Eraser is a host's split
 * button, and each host writes the same `exclusive` on it; Excel's is the generic toggle.
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
  const tools = `${application}.draw.write.tools`;
  return [
    {
      id: `${application}.draw.write.select-objects`,
      label: 'Select Objects',
      icon: 'cursor',
      toggle: true,
      pressed: true,
      exclusive: tools,
      essential: true,
    },
    { id: `${application}.draw.write.lasso-select`, label: 'Lasso Select', icon: 'lasso', toggle: true, exclusive: tools },
    { id: `${application}.draw.write.pen`, label: 'Pen', icon: 'pen', size: 'large', toggle: true, exclusive: tools },
    { id: `${application}.draw.write.highlighter`, label: 'Highlighter', icon: 'highlight', size: 'large', toggle: true, exclusive: tools },
    { id: `${application}.draw.write.eraser`, label: 'Eraser', icon: 'eraser', size: 'large', toggle: true, exclusive: tools },
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
// (Since PowerPoint's Table Layout it also takes the tab and the object, and a table's Arrange has no
// Group or Rotate; see that unit's part of the *commands Table Layout shows* section.)
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
 * Arrange, in Word's Layout, Excel's Page Layout and PowerPoint's Table Layout: one function of the application, the
 * tab and the object arranged, because the shared commands carry the same names and glyphs everywhere. The Picture,
 * Shape and Chart Format units are expected to call it too.
 *
 * - **Word** leads with Position and Wrap Text, large, and draws the six small beside them.
 * - **Excel** has no Position or Wrap Text (a cell does not wrap around a picture), and draws Bring
 *   Forward and Send Backward large at the head. `GUESS:` Selection Pane small in Excel, where Office
 *   draws it large, because it has no glyph.
 * - **PowerPoint** has no Position or Wrap Text either (a slide has no running text to wrap), and draws the layer
 *   commands small, as Word does. `GUESS:` small.
 * - **A table** (`object: 'table'`, PowerPoint's Table Layout) has **no Group or Rotate**: PowerPoint neither groups a
 *   table with other objects nor rotates one, so Office's Arrange on that tab ends at Align.
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
function arrangeCommands(
  application: RibbonApplication,
  tab: string,
  object: 'drawing' | 'table' = 'drawing',
): readonly RibbonCommand[] {
  const layer: ControlSize = application === 'excel' ? 'large' : 'small';
  const wordOnly: readonly RibbonCommand[] =
    application === 'word'
      ? [
          { id: `word.${tab}.arrange.position`, label: 'Position' },
          { id: `word.${tab}.arrange.wrap-text`, label: 'Wrap Text', icon: 'text-position-square', size: 'large' },
        ]
      : [];
  const drawingOnly: readonly RibbonCommand[] =
    object === 'drawing'
      ? [
          { id: `${application}.${tab}.arrange.group`, label: 'Group', icon: 'group' },
          { id: `${application}.${tab}.arrange.rotate`, label: 'Rotate', icon: 'rotate-right' },
        ]
      : [];
  return [
    ...wordOnly,
    { id: `${application}.${tab}.arrange.bring-forward`, label: 'Bring Forward', icon: 'position-forward', size: layer },
    { id: `${application}.${tab}.arrange.send-backward`, label: 'Send Backward', icon: 'position-backward', size: layer },
    { id: `${application}.${tab}.arrange.selection-pane`, label: 'Selection Pane', toggle: true },
    { id: `${application}.${tab}.arrange.align`, label: 'Align', icon: 'align-left' },
    ...drawingOnly,
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
//    **six** commands and the *Advance Slide* caption (unit 7) where the census counts two, and that is
//    Office's face rather than padding.
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
 * **The gallery is in-ribbon**, and the hosts bind `<mjx-gallery>` over **every transition Office shows**:
 * None and forty-nine more under Subtle, Exciting and Dynamic Content. Its accessible name is Office's,
 * *Transition to This Slide*. **Effect Options is a dropdown whose entries follow the committed
 * transition**, as Office's do, and it is unavailable for a transition Office gives no options. The hosts
 * start the gallery on **Fade**, so it first offers Smoothly and Through Black. `GUESS:` Fade rather than
 * a new deck's None, where Office disables Effect Options and there would be nothing to audit, and many
 * transitions' options; see `stories/ribbons/references-transitions-formulas-menus.ts`.
 *
 * **No survivor**: a gallery and a menu.
 */
const powerpointTransitionsTransitionStyles: readonly RibbonCommand[] = [
  { id: 'powerpoint.transitions.transition-styles.transitions', label: 'Transition to This Slide' },
  { id: 'powerpoint.transitions.transition-styles.effect-options', label: 'Effect Options' },
];

/**
 * PowerPoint's `GroupTransitionToThisSlide`, labelled **Timing**: Sound, Duration, Apply To All, then
 * the *Advance Slide* heading, On Mouse Click, After and the time after which the slide advances.
 *
 * ⚠ **Seven entries where the census counts two.** See this section's header.
 *
 * **Office stacks the group in two columns** — Sound, Duration and Apply To All on the left, and the
 * Advance Slide heading over On Mouse Click and After on the right — and `<mjx-ribbon-group>` draws
 * every command in one row at full width (`groupPresentations.full.commandRows` is 1, for the whole
 * catalogue). So the group keeps Office's reading order rather than its columns. **Advance Slide is a
 * caption, not a command**: it is declared here so its place in that order is the census's, and both
 * hosts bind `<mjx-label>` over it, a label with no `for`, which is that component's caption. Unbound, a
 * host would draw it as a button, which is why both bind it. `GUESS:` that a declared caption is the
 * right way to carry a heading.
 *
 * **Sound is a dropdown field** starting on *[No Sound]*, over every entry Office's list has. **Duration is a combo box** of seconds,
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
  { id: 'powerpoint.transitions.timing.advance-slide', label: 'Advance Slide' },
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

// ── the commands Mailings, Animations and Data show ──────────────────────────
//
// The ribbon programme's **unit 7**: Word's **Mailings** (five groups), PowerPoint's **Animations**
// (four) and Excel's **Data** (nine). Mailings is a pipeline — pick a document, pick recipients, write
// fields, look at the results, merge. Animations is Transitions' shape again, applied to one object
// rather than one slide. Data is where a workbook meets the world outside it, and it is the tab where
// the census declares the most generations of Office at once.
//
// ## The shapes, decided by what Office's popup is
//
// Unit 6's three shapes, unchanged. **In-ribbon gallery**: Animations' *Animation Styles* and Data's
// *Data Types*. **Dropdown or split button** over a menu written once in
// `stories/ribbons/mailings-animations-data-menus.ts`: Start Mail Merge, Select Recipients, Rules,
// Finish & Merge, Effect Options, Add Animation, Trigger, From Other Sources and What-If Analysis are
// dropdowns; Insert Merge Field, Preview, Refresh All, Data Validation, Group and Ungroup are split
// buttons. **Field**: Mailings' *Go to Record* and Animations' Duration and Delay are `<mjx-combo-box>`
// (a record number and a time in seconds are not measures `measure.ts` knows, which is unit 6's
// Duration argument), and Animations' Start is `<mjx-dropdown>`. Their lists are in
// `stories/ribbons/ribbon-parts.ts`, because two hosts bind each.
//
// **Toggles**: Highlight Merge Fields, Preview Results, Animation Pane, Queries & Connections, Workbook
// Links and Filter. Office draws each pressed while it holds.
//
// ⚠ **Three of the brief's expected shapes are not Office's, and Office wins, as it did for unit 6's
// Insert Footnote.**
//
// - **Insert Merge Field is a split button**, where the brief expected a dropdown. Office's face opens
//   the Insert Merge Field dialog and its arrow lists the fields.
// - **Preview is a split button** (Preview, and AutoPreview behind the arrow), as on Office's
//   Animations tab. Transitions' Preview is a plain button because Office's is.
// - **Excel has no Get Data here.** See disagreement 4 below.
//
// ## ⚠ Where the census and Office disagree, recorded rather than smoothed over
//
// 1. **PowerPoint's `GroupAnimations` is labelled *Animations* here and *Animation* in Office**, and
//    **`GroupAnimationCustom` is *Custom Animation* here and *Advanced Animation* in Office.** The
//    census's labels win, as Insert's *Slicers* did. The commands carry Office's names.
// 2. **Mailings' Preview Results group is drawn as Office draws it, eight commands for eight
//    controls**: the toggle, four record buttons either side of the record number, Find Recipient and
//    Check for Errors. The census count and the face agree, which is rare enough to say.
// 3. **Excel's Data tab declares three generations of one group.** Office 2016's **Connections**
//    (Refresh All, Connections, Properties, Edit Links), Microsoft 365's **Queries & Connections**
//    (Refresh All, Queries & Connections, Properties) and the 365 variant that adds **Workbook Links**.
//    All three are in scope with nine controls each. Draw's rule applies: a group the census declares
//    is drawn, and each command is drawn **once**. So **Queries & Connections** holds Refresh All,
//    Queries & Connections and Properties; **Workbook Links** holds Workbook Links; **Connections**
//    holds the two commands neither 365 group has, the Workbook Connections dialog (*Connections*) and
//    *Edit Links*. `GUESS:` the whole reading, and in particular that Edit Links and Workbook Links are
//    two commands: one is a dialog and one is a pane, and Office has shipped both names.
// 4. **Excel's `GroupGetExternalData` is Office 2016's legacy group, and Get Data is out of scope.**
//    Microsoft 365's *Get & Transform Data* group — Get Data, From Text/CSV, From Web, From Table/Range
//    — is Power Query, and the census carries it as four `GroupPowerQuery*` rows marked **out of
//    scope** (46 to 74 controls each). The in-scope group counts **5**, which is exactly the legacy
//    face: From Access, From Web, From Text, From Other Sources and Existing Connections. The census
//    wins, so the brief's Get Data dropdown is not drawn and those five are. `GUESS:` that the count is
//    the legacy face and not five of 365's.
// 5. **Excel's `GroupLinkedEntityConvert` is labelled *Data Types* in both**, and holds Office's
//    in-ribbon gallery of linked data types. The census counts two controls; the gallery is one.
// 6. **The census's counts are larger than the faces**, and nothing is padded: Word's Start Mail Merge
//    is 13 (every document type and every recipient source) and draws three.
//
// ## Survivors: two on Mailings, two on Data, none on Animations
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Mailings' Previous Record and Next Record survive.** One press shows one record, and the other
//   press takes it back: the Select Objects standard from unit 4, because a record changes no
//   document and there is nothing for Ctrl+Z to undo. Office draws both icon-only, and a caret either
//   side of a group called *Preview Results* is the record navigator; `caret-left` and `caret-right`
//   are no other command's glyph. `GUESS:` rule 2, a judgement about glyphs. **First Record and Last
//   Record** pass rule 1 and fail rule 2: `previous` and `next` are a media player's skip-track marks,
//   which unlabelled say *track* rather than *record*. **Preview
//   Results** is refused on rule 2: an eye unlabelled is *show* or *hide*, not *show the merged data*.
// - **Data's Sort A to Z and Sort Z to A survive.** One press sorts, and Ctrl+Z restores the order. The
//   letters with an arrow are Office's own sort mark and no other command's glyph. `GUESS:` rule 1 for
//   a selection with data beside it, where Office asks whether to expand the selection first.
//   **Filter is refused on rule 2**, and by the census's own collision standard: the funnel is already
//   Slicer's glyph on Excel's Insert tab (`filter`), so an unlabelled funnel says two things.
// - **Animations keeps none.** Move Earlier and Move Later pass rule 1 (Ctrl+Z restores the order) and
//   fail rule 2: an arrow up or down unlabelled says *move*, *scroll* or *sort*, and `arrow-down` is
//   already Excel's Fill. Animation Painter arms a gesture, which Draw's rule 1 refuses.
//
// Everything else opens a menu, a gallery, a dialog or a pane, or is a field a host binds. Each group
// states its own reason below.
//
// ## Sizes, and the commands that carry no icon
//
// Unit 6's rule: `large` where Office draws it large **and** there is an honest glyph **and** the label
// wraps inside `largeControlWidthUnits`. Three tokens do not fit, so **Start Mail Merge**, **Edit
// Recipient List**, **Insert Merge Field**, **Check for Errors**, **Finish & Merge**, **Text to
// Columns**, **From Other Sources**, **What-If Analysis** and **Manage Data Model** are `small`.
// **Select Recipients** and **Existing Connections** are `small` because *Recipients* and
// *Connections* are the length of the *Recommended* unit 3 measured clipping.
//
// A wrong icon is worse than none. Fluent draws **no mailing label** (`tag` is a price tag), **no merge
// field** (`braces` is code), **no merge rule**, **no field matching**, **no merge result**, **no
// animation preview** (`slide-transition` is a slide leaving, and `play` is a video's), **no animation
// pane** (`panel-right` is any pane), **no Access database** (Access's mark is a product's), **no
// workbook connection**, **no workbook link** (`link` is a hyperlink), **no property sheet**, **no
// advanced filter**, **no text split into columns** (`text-column-two` is Columns), **no duplicate
// removal**, **no consolidation**, **no data model**, **no what-if**, **no ungroup of rows** (Fluent's
// dismissal glyph is shapes') and **no subtotal** (`autosum` is AutoSum). So **Labels**, **Highlight
// Merge Fields**, **Insert Merge Field**, **Rules**, **Match Fields**, **Update Labels**, **Finish &
// Merge**, **Preview**, **Effect Options**, **Animation Pane**, **From Access**, **Queries &
// Connections**, **Properties**, **Workbook Links**, **Connections**, **Edit Links**, **Advanced**,
// **Text to Columns**, **Remove Duplicates**, **Consolidate**, **Manage Data Model**, **What-If
// Analysis**, **Ungroup** and **Subtotal** are `small`, and the label is the command. The seven fields
// and the two galleries carry none either.

/**
 * Word's Create group: Envelopes, Labels.
 *
 * **Envelopes draws `mail`**, the envelope File's Share already draws for Email, and is large as in
 * Office. **Labels carries no icon** (see this section's header), so it is `small` where Office draws it
 * large. Both open the Envelopes and Labels dialog on its own page.
 *
 * **No survivor**: two dialogs.
 */
const wordMailingsCreate: readonly RibbonCommand[] = [
  { id: 'word.mailings.create.envelopes', label: 'Envelopes', icon: 'mail', size: 'large' },
  { id: 'word.mailings.create.labels', label: 'Labels' },
];

/**
 * Word's Start Mail Merge group: Start Mail Merge, Select Recipients, Edit Recipient List.
 *
 * **Start Mail Merge is a dropdown** of the document types, Normal Word Document checked, then the
 * wizard. **Select Recipients is a dropdown** of the three sources. **Edit Recipient List** opens the
 * Mail Merge Recipients dialog. All three are `small`; see this section's header. `GUESS:` the glyphs:
 * `mail-multiple` (many letters from one) for Start Mail Merge, `people-list` for Select Recipients
 * and `people-edit` for Edit Recipient List.
 *
 * **No survivor**: two menus and a dialog.
 */
const wordMailingsStartMailMerge: readonly RibbonCommand[] = [
  { id: 'word.mailings.start-mail-merge.start-mail-merge', label: 'Start Mail Merge', icon: 'mail-multiple' },
  { id: 'word.mailings.start-mail-merge.select-recipients', label: 'Select Recipients', icon: 'people-list' },
  { id: 'word.mailings.start-mail-merge.edit-recipient-list', label: 'Edit Recipient List', icon: 'people-edit' },
];

/**
 * Word's Write & Insert Fields group: Highlight Merge Fields, Address Block, Greeting Line, Insert Merge
 * Field, Rules, Match Fields, Update Labels.
 *
 * **Highlight Merge Fields is a toggle**: Office draws it pressed while fields are shaded. It carries no
 * icon: `highlight` is a marker stroke, and this shades a field grey. **Address Block draws
 * `contact-card`** and **Greeting Line draws `hand-wave`**, both large and both opening their dialogs.
 * `GUESS:` the address card. **Insert Merge Field is a split button** (see this section's header), and
 * **Rules is a dropdown** of Word's nine merge rules. **Match Fields** opens a dialog. **Update Labels**
 * copies the first label's layout to the rest, with no dialog.
 *
 * **No survivor**: Highlight Merge Fields and Update Labels have no glyph, and the rest open something.
 */
const wordMailingsWriteInsertFields: readonly RibbonCommand[] = [
  { id: 'word.mailings.write-insert-fields.highlight-merge-fields', label: 'Highlight Merge Fields', toggle: true },
  { id: 'word.mailings.write-insert-fields.address-block', label: 'Address Block', icon: 'contact-card', size: 'large' },
  { id: 'word.mailings.write-insert-fields.greeting-line', label: 'Greeting Line', icon: 'hand-wave', size: 'large' },
  { id: 'word.mailings.write-insert-fields.insert-merge-field', label: 'Insert Merge Field' },
  { id: 'word.mailings.write-insert-fields.rules', label: 'Rules' },
  { id: 'word.mailings.write-insert-fields.match-fields', label: 'Match Fields' },
  { id: 'word.mailings.write-insert-fields.update-labels', label: 'Update Labels' },
];

/**
 * Word's Preview Results group: Preview Results large, the record navigator, Find Recipient and Check for
 * Errors.
 *
 * **Preview Results is a large toggle drawing `eye`**, and starts **pressed**. `GUESS:` pressed rather
 * than a new merge's off, for unit 6's Fade reason: the navigator beside it is what there is to audit.
 * **The navigator is First Record (`previous`), Previous Record (`caret-left`), the record number, Next
 * Record (`caret-right`) and Last Record (`next`)**, icon-only as Office draws them. The record number is
 * a field, a combo box starting on 1. `GUESS:` its name, *Go to Record*, which is Office's tooltip; the
 * box has no visible name. **Find Recipient draws `person-search`** and **Check for Errors draws
 * `checkmark-circle-warning`**, Excel's Error Checking glyph, for the same check. Both open dialogs.
 *
 * **Survivors: Previous Record and Next Record.** See this section's header.
 */
const wordMailingsPreviewResults: readonly RibbonCommand[] = [
  { id: 'word.mailings.preview-results.preview-results', label: 'Preview Results', icon: 'eye', size: 'large', toggle: true, pressed: true },
  { id: 'word.mailings.preview-results.first-record', label: 'First Record', icon: 'previous', size: 'icon' },
  { id: 'word.mailings.preview-results.previous-record', label: 'Previous Record', icon: 'caret-left', size: 'icon', essential: true },
  { id: 'word.mailings.preview-results.go-to-record', label: 'Go to Record' },
  { id: 'word.mailings.preview-results.next-record', label: 'Next Record', icon: 'caret-right', size: 'icon', essential: true },
  { id: 'word.mailings.preview-results.last-record', label: 'Last Record', icon: 'next', size: 'icon' },
  { id: 'word.mailings.preview-results.find-recipient', label: 'Find Recipient', icon: 'person-search' },
  { id: 'word.mailings.preview-results.check-for-errors', label: 'Check for Errors', icon: 'checkmark-circle-warning' },
];

/**
 * Word's Finish group: Finish & Merge, a dropdown of the three ways to finish (edit, print, email).
 *
 * **No survivor**: a menu, and the only command.
 */
const wordMailingsFinish: readonly RibbonCommand[] = [
  { id: 'word.mailings.finish.finish-merge', label: 'Finish & Merge' },
];

/**
 * PowerPoint's Animations Preview group: one split button. The face plays the slide's animations, and the
 * arrow offers Preview and AutoPreview, checked.
 *
 * **No icon**, so `small` where Office draws it large; see this section's header.
 *
 * **No survivor**: a split button, and the only command.
 */
const powerpointAnimationsPreview: readonly RibbonCommand[] = [
  { id: 'powerpoint.animations.preview.preview', label: 'Preview' },
];

/**
 * PowerPoint's `GroupAnimations`, which Office labels **Animation**: the gallery and Effect Options.
 *
 * **The gallery is in-ribbon**, and the hosts bind `<mjx-gallery>` with Office's accessible name,
 * *Animation Styles*. It starts on **Fly In**, so **Effect Options** offers Fly In's eight directions
 * and the three sequences. `GUESS:` Fly In rather than a new shape's None, which disables Effect
 * Options: unit 6's Fade argument. Office's dialog launcher here opens the effect's own dialog.
 *
 * **No survivor**: a gallery and a menu.
 */
const powerpointAnimationsAnimations: readonly RibbonCommand[] = [
  { id: 'powerpoint.animations.animations.animation-styles', label: 'Animation Styles' },
  { id: 'powerpoint.animations.animations.effect-options', label: 'Effect Options' },
];

/**
 * PowerPoint's `GroupAnimationCustom`, which Office labels **Advanced Animation**: Add Animation large,
 * then Animation Pane, Trigger and Animation Painter.
 *
 * **Add Animation draws `star-add`**, a star with a plus, which is Office's own picture, and is a
 * dropdown of the effects the gallery holds, under the same four headings. **Animation Pane is a
 * toggle** with no glyph. **Trigger draws `flash`**, the lightning bolt Office draws, and is a dropdown
 * of the slide's shapes. `GUESS:` the bolt, which Excel's Flash Fill also draws. **Animation Painter
 * draws `paint-brush-sparkle`** and is a plain button, as Home's Format Painter is.
 *
 * **No survivor**: two menus, a pane, and a painter that arms a gesture.
 */
const powerpointAnimationsCustomAnimation: readonly RibbonCommand[] = [
  { id: 'powerpoint.animations.custom-animation.add-animation', label: 'Add Animation', icon: 'star-add', size: 'large' },
  { id: 'powerpoint.animations.custom-animation.animation-pane', label: 'Animation Pane', toggle: true },
  { id: 'powerpoint.animations.custom-animation.trigger', label: 'Trigger', icon: 'flash' },
  { id: 'powerpoint.animations.custom-animation.animation-painter', label: 'Animation Painter', icon: 'paint-brush-sparkle' },
];

/**
 * PowerPoint's Animations Timing group: Start, Duration, Delay, then Move Earlier and Move Later under
 * Office's *Reorder Animation* heading.
 *
 * **Start is a dropdown field** starting on On Click. **Duration and Delay are combo boxes** of seconds,
 * starting on Fly In's 00.50 and on 00.00. **Move Earlier draws `arrow-up`** and **Move Later draws
 * `arrow-down`**, labelled, as Office draws them. The census counts six; the heading is not a command.
 *
 * **No survivor**: three fields, and two arrows that fail rule 2. See this section's header.
 */
const powerpointAnimationsTiming: readonly RibbonCommand[] = [
  { id: 'powerpoint.animations.timing.start', label: 'Start' },
  { id: 'powerpoint.animations.timing.duration', label: 'Duration' },
  { id: 'powerpoint.animations.timing.delay', label: 'Delay' },
  { id: 'powerpoint.animations.timing.move-earlier', label: 'Move Earlier', icon: 'arrow-up' },
  { id: 'powerpoint.animations.timing.move-later', label: 'Move Later', icon: 'arrow-down' },
];

/**
 * Excel's `GroupGetExternalData`: Office 2016's Get External Data face. See disagreement 4 in this
 * section's header.
 *
 * **From Access** carries no icon. **From Web draws `globe`**, the web everywhere in Office, and **From
 * Text draws `document-text`**. **From Other Sources draws `database`** and is a dropdown of the legacy
 * wizards. **Existing Connections draws `plug-connected`**. `GUESS:` the plug. Every one opens a dialog
 * or a wizard, and all five are `small`: Office 2016 drew the first three in a column, and the last two
 * are long.
 *
 * **No survivor**: four dialogs and a menu.
 */
const excelDataGetExternalData: readonly RibbonCommand[] = [
  { id: 'excel.data.get-external-data.from-access', label: 'From Access' },
  { id: 'excel.data.get-external-data.from-web', label: 'From Web', icon: 'globe' },
  { id: 'excel.data.get-external-data.from-text', label: 'From Text', icon: 'document-text' },
  { id: 'excel.data.get-external-data.from-other-sources', label: 'From Other Sources', icon: 'database' },
  { id: 'excel.data.get-external-data.existing-connections', label: 'Existing Connections', icon: 'plug-connected' },
];

/**
 * Excel's Queries & Connections group: Refresh All large, Queries & Connections, Properties.
 *
 * **Refresh All draws `arrow-clockwise`**, the refresh arrow, and is a split button: the face refreshes
 * every connection, and the arrow offers Refresh, Refresh Status, Cancel Refresh and Connection
 * Properties. Not `arrow-sync`, which is AutoSave. **Queries & Connections is a toggle** that opens its
 * pane, and **Properties** opens the External Data Properties dialog. Neither has a glyph.
 *
 * **No survivor**: a split button, a pane and a dialog.
 */
const excelDataQueriesAndConnections: readonly RibbonCommand[] = [
  { id: 'excel.data.queries-connections.refresh-all', label: 'Refresh All', icon: 'arrow-clockwise', size: 'large' },
  { id: 'excel.data.queries-connections.queries-connections', label: 'Queries & Connections', toggle: true },
  { id: 'excel.data.queries-connections.properties', label: 'Properties' },
];

/**
 * Excel's `GroupDataQueriesAndConnectionsWorkbookLinks`: Workbook Links, the toggle that opens its pane.
 * See disagreement 3 in this section's header.
 *
 * **No survivor**: a pane, no glyph, and the only command.
 */
const excelDataWorkbookLinks: readonly RibbonCommand[] = [
  { id: 'excel.data.workbook-links.workbook-links', label: 'Workbook Links', toggle: true },
];

/**
 * Excel's `GroupConnections`: Office 2016's two dialogs that neither 365 group draws, Connections and
 * Edit Links. See disagreement 3 in this section's header.
 *
 * **No survivor**: two dialogs.
 */
const excelDataConnections: readonly RibbonCommand[] = [
  { id: 'excel.data.connections.connections', label: 'Connections' },
  { id: 'excel.data.connections.edit-links', label: 'Edit Links' },
];

/**
 * Excel's Data Types group: the in-ribbon gallery of linked data types, Stocks, Currencies and
 * Geography. Each picture is Office's glyph: `building-bank`, `money` and `map`.
 *
 * **No survivor**: a gallery, and the only command.
 */
const excelDataDataTypes: readonly RibbonCommand[] = [
  { id: 'excel.data.data-types.data-types', label: 'Data Types' },
];

/**
 * Excel's Sort & Filter group: Sort A to Z, Sort Z to A, Sort large, Filter large, then Clear, Reapply and
 * Advanced.
 *
 * **Sort A to Z and Sort Z to A draw `text-sort-ascending` and `text-sort-descending`**, icon-only as
 * Office draws them. **Sort draws `arrow-sort`**, Home's Sort & Filter glyph, because both open a surface
 * where the direction is chosen, and opens the Sort dialog. **Filter is a toggle drawing `filter`**:
 * Office draws it pressed while the range is filtered. **Clear draws `filter-dismiss`** and **Reapply
 * draws `filter-sync`**. **Advanced** opens the Advanced Filter dialog.
 *
 * **Survivors: Sort A to Z and Sort Z to A.** Filter is refused on rule 2. See this section's header.
 */
const excelDataSortFilter: readonly RibbonCommand[] = [
  { id: 'excel.data.sort-filter.sort-ascending', label: 'Sort A to Z', icon: 'text-sort-ascending', size: 'icon', essential: true },
  { id: 'excel.data.sort-filter.sort-descending', label: 'Sort Z to A', icon: 'text-sort-descending', size: 'icon', essential: true },
  { id: 'excel.data.sort-filter.sort', label: 'Sort', icon: 'arrow-sort', size: 'large' },
  { id: 'excel.data.sort-filter.filter', label: 'Filter', icon: 'filter', size: 'large', toggle: true },
  { id: 'excel.data.sort-filter.clear', label: 'Clear', icon: 'filter-dismiss' },
  { id: 'excel.data.sort-filter.reapply', label: 'Reapply', icon: 'filter-sync' },
  { id: 'excel.data.sort-filter.advanced', label: 'Advanced' },
];

/**
 * Excel's Data Tools group: Text to Columns, Flash Fill, Remove Duplicates, Data Validation, Consolidate,
 * Relationships, Manage Data Model.
 *
 * **Flash Fill draws `flash`**, the bolt Office draws, and fills with no dialog (Ctrl+E). **Data
 * Validation draws `table-simple-checkmark`** and is a split button (Data Validation, Circle Invalid Data,
 * Clear Validation Circles). **Relationships draws `table-link`**, two tables joined. `GUESS:` the table
 * link. Text to Columns, Remove Duplicates, Consolidate and Relationships open dialogs, and **Manage Data
 * Model** opens the Power Pivot window. `GUESS:` that it belongs on the face although Power Pivot is
 * out of scope: the census counts it in an in-scope group.
 *
 * **No survivor**: Flash Fill passes rule 1 and fails rule 2 (a bolt says nothing about filling a
 * pattern); the rest open something.
 */
const excelDataDataTools: readonly RibbonCommand[] = [
  { id: 'excel.data.data-tools.text-to-columns', label: 'Text to Columns' },
  { id: 'excel.data.data-tools.flash-fill', label: 'Flash Fill', icon: 'flash' },
  { id: 'excel.data.data-tools.remove-duplicates', label: 'Remove Duplicates' },
  { id: 'excel.data.data-tools.data-validation', label: 'Data Validation', icon: 'table-simple-checkmark' },
  { id: 'excel.data.data-tools.consolidate', label: 'Consolidate' },
  { id: 'excel.data.data-tools.relationships', label: 'Relationships', icon: 'table-link' },
  { id: 'excel.data.data-tools.manage-data-model', label: 'Manage Data Model' },
];

/**
 * Excel's Forecast group: What-If Analysis, a dropdown (Scenario Manager, Goal Seek, Data Table), and
 * Forecast Sheet large.
 *
 * **Forecast Sheet draws `data-trending`**, a line going up, and opens the Create Forecast Worksheet
 * dialog. `GUESS:` the glyph. Not `data-line`, which is Insert's Line chart.
 *
 * **No survivor**: a menu and a dialog.
 */
const excelDataForecast: readonly RibbonCommand[] = [
  { id: 'excel.data.forecast.what-if-analysis', label: 'What-If Analysis' },
  { id: 'excel.data.forecast.forecast-sheet', label: 'Forecast Sheet', icon: 'data-trending', size: 'large' },
];

/**
 * Excel's Outline group: Group large, Ungroup, Subtotal, Show Detail, Hide Detail.
 *
 * **Group draws `group-list`**, rows under one bracket, and **Group and Ungroup are split buttons** (Group,
 * Auto Outline; Ungroup, Clear Outline). Not `group`, which is Arrange's shapes. `GUESS:` the bracket.
 * **Subtotal** opens its dialog. **Show Detail and Hide Detail draw `add-square` and `subtract-square`**,
 * the outline's own marks. Office's dialog launcher here opens the outline Settings.
 *
 * **No survivor**: Show Detail and Hide Detail pass rule 1 and fail rule 2, `GUESS:`: a plus in a square
 * reads as *add* across Office, and the outline's own marks sit in the sheet margin where a person uses
 * them. The rest open something.
 */
const excelDataOutline: readonly RibbonCommand[] = [
  { id: 'excel.data.outline.group', label: 'Group', icon: 'group-list', size: 'large' },
  { id: 'excel.data.outline.ungroup', label: 'Ungroup' },
  { id: 'excel.data.outline.subtotal', label: 'Subtotal' },
  { id: 'excel.data.outline.show-detail', label: 'Show Detail', icon: 'add-square' },
  { id: 'excel.data.outline.hide-detail', label: 'Hide Detail', icon: 'subtract-square' },
];

// ── the commands Review shows ────────────────────────────────────────────────
//
// The ribbon programme's **unit 8**: **Word's Review tab**, all nine in-scope groups. ⚠ **Word alone.**
// The unit was narrowed to one tab of one application. PowerPoint's Review tab followed in its own unit,
// declared in the *PowerPoint's Review* part below, and Excel's after it in the *Excel's Review* part. Nothing is written as
// a function of the application. Proofing,
// Accessibility, Language, Comments and Ink carry the same group ids in all three census tabs, but their
// faces differ (Excel's Proofing is Spelling, Thesaurus and Workbook Statistics; Excel's Comments are
// three generations), so **whether any of them is one declaration is the next unit's question**, asked
// with the other two faces in front of it rather than answered here in advance.
//
// ## The shapes, decided by what Office's popup is
//
// Unit 7's shapes. **Dropdown or split button** over a menu written once in
// `stories/ribbons/review-menus.ts`: Translate, Language, Show Markup and Compare are dropdowns; Check
// Accessibility, Delete, Show Comments, Track Changes, Reviewing Pane, Accept, Reject, Block Authors and
// Hide Ink are split buttons. **Field**: Display for Review is `<mjx-dropdown>` over
// `ribbon-parts.ts`'s `displayForReviewModes`. **Toggle**: Restrict Editing, which Office draws pressed
// while its pane is open. **No gallery** on this tab.
//
// Every menu carries **every entry Office's menu has**, by Office's names and in Office's order — the
// user rejected a sampled Transitions gallery, and the rule since is that a popup is complete. Where
// Office nests a submenu (Show Markup's *Balloons* and *Specific People*, Compare's *Show Source
// Documents*) it is flattened into a labelled section, as every earlier unit flattened one.
//
// **Three of the brief's expected toggles are split buttons, because Office draws them so.** Each is
// declared `toggle: true` and bound as `<mjx-split-button toggle>`: the face draws pressed and the arrow
// stays a menu. `tests/ribbons.test.ts` holds every host's binding to that declaration, and the same
// component change is what lets Word's and PowerPoint's split Eraser draw pressed.
//
// - **Track Changes** is a split button, as the brief expected: the face turns tracking on and off, and
//   the arrow offers For Everyone, Just Mine and Lock Tracking. It starts unpressed.
// - **Show Comments** is `GUESS:` a split button in Microsoft 365's modern comments: the face shows the
//   comments, and the arrow chooses Contextual or List. Word 2016 drew a plain toggle. `GUESS:` it
//   starts pressed, because a document opened for review shows its comments.
// - **Show Ink is not a name Office's Review tab uses.** Microsoft 365's Ink group is **Hide Ink**, a
//   split button (`GUESS:` the shape) whose arrow holds Hide Ink and Delete All Ink in Document, and
//   whose face starts unpressed, since ink is shown until somebody hides it. Word
//   2016's Ink group held Start Inking instead, which is the Draw tab's job now.
//
// ## ⚠ Where the census and Office disagree, recorded rather than smoothed over
//
// 1. **The declaration puts Ink between Comments and Tracking; Microsoft 365 draws it after Protect.**
//    Office's Review tab reads Proofing, Speech, Accessibility, Language, Comments, Tracking, Changes,
//    Compare, Protect, Ink. The group *order* is the tab module's decision (`censusGroup`'s doc says so),
//    so `wordReviewTab` draws Office's order and the declaration below is left as transcribed. `GUESS:`
//    Ink's position, from Microsoft 365's face; no build this project can cite is checked.
// 2. **Office draws groups the census marks out of scope, and they are not drawn here**: **Speech**
//    (Read Aloud, between Proofing and Accessibility) and **Chinese Translation**. Office's **Resume**
//    (Resume Assistant) and **Linked Notes** (OneNote) have no census row at all. The census wins.
// 3. **Proofing is drawn as Spelling & Grammar, Thesaurus and Word Count.** Microsoft 365 builds between
//    Editor's arrival and its retirement drew **Editor** here in place of Spelling & Grammar. Editor is
//    already Home's `GroupEditor`, and Draw's rule applies: a command is drawn once. `GUESS:` the face.
// 4. **Office's face labels two commands *Previous* and two *Next*** (one pair in Comments, one in
//    Changes). A label here is also the accessible name, and four names for two meanings would be the
//    collision rule 2 exists to refuse, so the labels are **Office's own tooltips**: Previous Comment,
//    Next Comment, Previous Change, Next Change.
// 5. **The census's counts are larger than the faces**, and nothing is padded: Tracking is 22 (every
//    entry of every menu in the group) and draws four; Language is 14 and draws two.
//
// ## Survivors: Previous Comment and Next Comment, and nothing else
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Comments' Previous Comment and Next Comment survive.** One press moves to one comment, and the
//   other press moves back: the Mailings record-navigator standard, because moving between comments
//   changes no document. `comment-arrow-left` and `comment-arrow-right` are a speech bubble with an
//   arrow, which says *comment* and *which way* and is no other command's glyph. `GUESS:` rule 2.
// - **Changes' Previous Change and Next Change pass rule 1 and fail rule 2**, because they carry no
//   glyph: Fluent's page with an arrow (`document-arrow-up`, `document-arrow-down`) is *upload* and
//   *download* everywhere else, so it would be a wrong icon rather than a missing one.
// - **Restrict Editing** is a toggle that opens a pane, and a pane is something opened. **Word Count**
//   and **Spelling & Grammar** open a dialog and a pane. Everything else opens a menu, is a split
//   button, or is a field. Each group states its own reason below.
//
// ## Sizes, and the commands that carry no icon
//
// Unit 6's rule: `large` where Office draws it large **and** there is an honest glyph **and** the label
// wraps inside `largeControlWidthUnits`. **Spelling & Grammar** is three tokens and **Check
// Accessibility** holds a thirteen-letter word, so both are `small` where Office draws them large.
//
// A wrong icon is worse than none. Fluent draws **no markup view** (Office's page with a balloon), **no
// document comparison** (`column-double-compare` compares two columns, at 20 alone, and `document-copy`
// is Copy), **no previous or next change** (see above) and **no hidden ink** (`pen-off` is a pen that
// cannot be used, which is Draw's Stop Inking idea rather than ink hidden from the page). So **Show
// Markup**, **Compare**, **Previous Change**, **Next Change** and **Hide Ink** are `small`, and the label
// is the command. The Display for Review field carries none either.

/**
 * Word's Proofing group: Spelling & Grammar, Thesaurus, Word Count.
 *
 * **Spelling & Grammar draws `text-grammar-checkmark`**, text with a tick, Office's *ABC✓*, and opens the
 * proofing pane (F7). It is `small`: three tokens. `text-proofing-tools` is Home's Editor, the pane's
 * other door. **Thesaurus draws `book-search`**, a book being looked in, and opens the Thesaurus pane
 * (Shift+F7). `GUESS:` the book, which Excel's Lookup & Reference also draws on another application's
 * tab. **Word Count draws `text-word-count`**, Fluent's own, and opens the Word Count dialog
 * (Ctrl+Shift+G). See disagreement 3 in this section's header about Editor.
 *
 * **No survivor**: two panes and a dialog.
 */
const wordReviewProofing: readonly RibbonCommand[] = [
  { id: 'word.review.proofing.spelling-grammar', label: 'Spelling & Grammar', icon: 'text-grammar-checkmark' },
  { id: 'word.review.proofing.thesaurus', label: 'Thesaurus', icon: 'book-search' },
  { id: 'word.review.proofing.word-count', label: 'Word Count', icon: 'text-word-count' },
];

/**
 * Word's Accessibility group: Check Accessibility.
 *
 * **A split button drawing `accessibility-checkmark`**, Fluent's accessibility figure with a tick. The
 * face runs the checker and opens its pane; the arrow offers Check Accessibility, Alt Text, Navigation
 * Pane and Options: Accessibility. `small`, because *Accessibility* is thirteen letters.
 *
 * **No survivor**: a split button, and the only command.
 */
const wordReviewAccessibility: readonly RibbonCommand[] = [
  { id: 'word.review.accessibility.check-accessibility', label: 'Check Accessibility', icon: 'accessibility-checkmark' },
];

/**
 * Word's Language group: Translate and Language, both large dropdowns.
 *
 * **Translate draws `translate`** and offers Translate Selection and Translate Document. **Language draws
 * `local-language`** and offers Set Proofing Language and Language Preferences. Both glyphs are Fluent's
 * own for the idea.
 *
 * **No survivor**: two menus.
 */
const wordReviewLanguage: readonly RibbonCommand[] = [
  { id: 'word.review.language.translate', label: 'Translate', icon: 'translate', size: 'large' },
  { id: 'word.review.language.language', label: 'Language', icon: 'local-language', size: 'large' },
];

/**
 * Word's Comments group: New Comment, Delete, Previous Comment, Next Comment, Show Comments.
 *
 * **New Comment draws `comment-add`**, Insert's Comment glyph, because it is the same command
 * (Ctrl+Alt+M). **Delete draws `comment-dismiss`** and is a large split button (Delete, Delete All
 * Comments Shown, Delete All Comments in Document). **Previous Comment and Next Comment draw
 * `comment-arrow-left` and `comment-arrow-right`**, labelled, as Office draws them. **Show Comments
 * draws `comment-multiple`** and is `GUESS:` a split button whose face is a toggle, starting pressed; see
 * this section's header.
 *
 * **Survivors: Previous Comment and Next Comment.** See this section's header.
 */
const wordReviewComments: readonly RibbonCommand[] = [
  { id: 'word.review.comments.new-comment', label: 'New Comment', icon: 'comment-add', size: 'large' },
  { id: 'word.review.comments.delete', label: 'Delete', icon: 'comment-dismiss', size: 'large' },
  { id: 'word.review.comments.previous-comment', label: 'Previous Comment', icon: 'comment-arrow-left', essential: true },
  { id: 'word.review.comments.next-comment', label: 'Next Comment', icon: 'comment-arrow-right', essential: true },
  { id: 'word.review.comments.show-comments', label: 'Show Comments', icon: 'comment-multiple', toggle: true, pressed: true },
];

/**
 * Word's Ink group: Hide Ink. See this section's header on why it is not *Show Ink*.
 *
 * `GUESS:` **a split button**: the face hides every ink stroke in the document and shows them again, and
 * the arrow holds Hide Ink and Delete All Ink in Document. The face is a toggle, starting unpressed. It
 * carries no icon, so it is `small` where
 * Office draws it large.
 *
 * **No survivor**: a split button, no glyph, and the only command.
 */
const wordReviewInk: readonly RibbonCommand[] = [
  { id: 'word.review.ink.hide-ink', label: 'Hide Ink', toggle: true },
];

/**
 * Word's `GroupChangesTracking`, labelled **Tracking**: Track Changes large, then Display for Review, Show
 * Markup and Reviewing Pane in a column.
 *
 * **Track Changes draws `document-edit`**, a page with a pencil, and is a split button (Ctrl+Shift+E):
 * the face is a toggle that turns tracking on and off, starting unpressed, and the arrow offers For
 * Everyone, checked, Just Mine and Lock
 * Tracking. **Display for Review is a dropdown field** of Simple Markup, All Markup, No Markup and
 * Original, starting on Simple Markup. `GUESS:` that start, which is a new document's in Word 2013 and
 * later. **Show Markup is a dropdown** of every kind of markup, its balloon placement and its reviewers.
 * **Reviewing Pane draws `panel-left-text`**, a pane of text at the left where Word opens it, and is a
 * split button (Reviewing Pane Vertical, Reviewing Pane Horizontal). `GUESS:` the pane glyph. Office's
 * dialog launcher here opens Change Tracking Options.
 *
 * **No survivor**: three split buttons or menus and a field.
 */
const wordReviewTracking: readonly RibbonCommand[] = [
  { id: 'word.review.tracking.track-changes', label: 'Track Changes', icon: 'document-edit', size: 'large', toggle: true },
  { id: 'word.review.tracking.display-for-review', label: 'Display for Review' },
  { id: 'word.review.tracking.show-markup', label: 'Show Markup' },
  { id: 'word.review.tracking.reviewing-pane', label: 'Reviewing Pane', icon: 'panel-left-text' },
];

/**
 * Word's Changes group: Accept large, then Reject, Previous Change and Next Change in a column.
 *
 * **Accept draws `document-checkmark`** and **Reject draws `document-dismiss`**, a page ticked and a page
 * struck out, and both are split buttons. Accept's arrow offers Accept and Move to Next, Accept This
 * Change, Accept All Changes Shown, Accept All Changes and Accept All Changes and Stop Tracking; Reject's
 * is the same five for rejecting. **Previous Change and Next Change** carry no icon; see this section's
 * header.
 *
 * **No survivor**: two split buttons, and two navigators with no glyph.
 */
const wordReviewChanges: readonly RibbonCommand[] = [
  { id: 'word.review.changes.accept', label: 'Accept', icon: 'document-checkmark', size: 'large' },
  { id: 'word.review.changes.reject', label: 'Reject', icon: 'document-dismiss' },
  { id: 'word.review.changes.previous-change', label: 'Previous Change' },
  { id: 'word.review.changes.next-change', label: 'Next Change' },
];

/**
 * Word's Compare group: Compare, a dropdown of Compare, Combine and the Show Source Documents choices.
 *
 * **No icon**, so `small` where Office draws it large; see this section's header.
 *
 * **No survivor**: a menu, and the only command.
 */
const wordReviewCompare: readonly RibbonCommand[] = [
  { id: 'word.review.compare.compare', label: 'Compare' },
];

/**
 * Word's Protect group: Block Authors and Restrict Editing, both large.
 *
 * **Block Authors draws `person-lock`**, a person with a padlock, and is `GUESS:` a split button (Block
 * Authors, Release All of My Blocked Areas), as Word 2010 drew it. Office greys it for a document that is
 * not on a shared server. **Restrict Editing is a toggle drawing `document-lock`**, File's Protect
 * Document glyph, because Restrict Editing is what Protect Document's menu opens: Office draws it pressed
 * while the Restrict Editing pane is open.
 *
 * **No survivor**: a split button, and a toggle that opens a pane.
 */
const wordReviewProtect: readonly RibbonCommand[] = [
  { id: 'word.review.protect.block-authors', label: 'Block Authors', icon: 'person-lock', size: 'large' },
  { id: 'word.review.protect.restrict-editing', label: 'Restrict Editing', icon: 'document-lock', size: 'large', toggle: true },
];

// ── PowerPoint's Review ──────────────────────────────────────────────────────
//
// **PowerPoint's Review tab**, all seven in-scope groups, authored after Word's and under the same
// one-tab-one-application rule; Excel's followed in the part after this one. Word's section above is the
// pattern: the shapes, the whole-menu rule, the survivor standard and the icon rule are unchanged, and
// only what PowerPoint does differently is written here.
//
// **Not one declaration shared with Word.** Proofing, Accessibility, Language, Comments and Ink carry the
// same group ids in both census tabs, and the faces differ in every one of them: PowerPoint's Proofing is
// Spelling alone rather than Spelling & Grammar and has no Word Count, its Translate is a button, its
// Show Comments' arrow is the Comments Pane and Show Markup, and its Delete and Hide Ink say *slide* and
// *presentation*. One declaration would be a function of the application with every branch taken.
//
// ## The shapes
//
// **Dropdown or split button** over a menu written in `stories/ribbons/review-menus.ts`: Language is a
// dropdown; Check Accessibility, Delete, Accept and Reject are split buttons. **A split button whose face
// is a toggle** (`<mjx-split-button toggle>`): Show Comments and Hide Ink, both starting unpressed.
// **Toggle**: Reviewing Pane. **Plain button**: Spelling, Thesaurus, Translate, New Comment, Previous
// Comment, Next Comment, Compare, Previous Change, Next Change, End Review, Show Changes. **No field, no
// gallery, no dialog launcher**: Office puts none on PowerPoint's Review tab.
//
// ## ⚠ Where the census and Office disagree, recorded rather than smoothed over
//
// 1. **The declaration puts Ink fifth, between Comments and Compare; Microsoft 365 draws it last.** The
//    group order is the tab module's decision, so `powerpointReviewTab` draws Proofing, Accessibility,
//    Language, Comments, Compare, Activity, Ink, which is Word's argument about its own Ink group.
//    `GUESS:` Ink's and Activity's positions; no build this project can cite is checked.
// 2. **Office draws groups the census marks out of scope, and they are not drawn here**: **Insights**
//    (Smart Lookup, one control) and **Chinese Translation** (three). The census wins.
// 3. **Activity is `GUESS:` in its entirety.** The census names the group and counts two controls, and
//    says nothing else. It is drawn as **Show Changes** alone, the Microsoft 365 command that marks what
//    co-authors changed since the deck was last opened, as a plain small button with no icon. Nothing is
//    added to reach the count of two.
// 4. **Office's face labels two commands *Previous* and two *Next*** (one pair in Comments, one in
//    Compare), exactly as in Word, and the labels are Office's tooltips for Word's reason: Previous
//    Comment, Next Comment, Previous Change, Next Change.
// 5. **Office greys Accept, Reject, Previous Change, Next Change, Reviewing Pane and End Review until a
//    comparison is under way.** They are drawn available, so the two menus can be opened and audited;
//    Word's Block Authors was drawn available for the same reason. `disabled` is loop 2's, driven by a
//    real review state.
// 6. **Office's Ink group held Start Inking in PowerPoint 2016**, and the census's five controls may still
//    count it. Drawing is the Draw tab's job now, so the group is Hide Ink, as it is in Word.
// 7. **The census's counts are larger than the faces**, and nothing is padded: Compare is 13 and draws
//    seven; Comments is 12 and draws five; Language is 9 and draws two.
//
// ## Survivors: Previous Comment and Next Comment, and nothing else
//
// Word's judgement, on the same glyphs: a press moves to one comment and the other press moves back, and a
// speech bubble with an arrow is no other command's glyph. `GUESS:` rule 2. **Previous Change and Next
// Change** fail rule 2 for want of an honest glyph, as in Word. Every other command opens a pane, a
// dialog, a file picker or a menu, is a split button, or (End Review) cannot be undone.
//
// ## Sizes, and the commands that carry no icon
//
// `large` where Office draws it large **and** there is an honest glyph **and** the label wraps inside
// `largeControlWidthUnits`. **Spelling** is one word, so unlike Word's *Spelling & Grammar* it is large.
// **Check Accessibility** is small, for Word's reason. ⚠ **Previous Comment and Next Comment are small
// where Office draws them large**: they are the tab's survivors, and every survivor in this catalogue is
// small, so a large one in a collapsed group's survivor row is a presentation nobody has looked at.
// `GUESS:` that the trade is right.
//
// **Compare**, **Previous Change**, **Next Change**, **End Review**, **Show Changes** and **Hide Ink**
// carry no icon, so each is `small` and its label is the command. Compare, the two change navigators and
// Hide Ink have no honest glyph for Word's reasons. **End Review** would be `dismiss-circle`, which is
// Close everywhere else, and ending a review discards every change not yet accepted, which is not
// closing. **Show Changes** would be `history`, which is File's Version History in this subset.

/**
 * PowerPoint's Proofing group: Spelling and Thesaurus.
 *
 * **Spelling draws `text-grammar-checkmark`**, Word's *ABC✓*, and is `large`: one word fits, and Office
 * draws it large. It opens the Spelling pane (F7). **Thesaurus draws `book-search`**, as in Word, and
 * opens the Thesaurus pane (Shift+F7). Office's Word Count is Word's alone.
 *
 * **No survivor**: two panes.
 */
const powerpointReviewProofing: readonly RibbonCommand[] = [
  { id: 'powerpoint.review.proofing.spelling', label: 'Spelling', icon: 'text-grammar-checkmark', size: 'large' },
  { id: 'powerpoint.review.proofing.thesaurus', label: 'Thesaurus', icon: 'book-search' },
];

/**
 * PowerPoint's Accessibility group: Check Accessibility.
 *
 * **A split button drawing `accessibility-checkmark`**, as in Word. The face runs the checker and opens its
 * pane; the arrow offers Check Accessibility, Alt Text, Reading Order Pane and Options: Accessibility.
 * Reading Order Pane is PowerPoint's where Word's arrow has Navigation Pane. `small`, for Word's reason.
 *
 * **No survivor**: a split button, and the only command.
 */
const powerpointReviewAccessibility: readonly RibbonCommand[] = [
  { id: 'powerpoint.review.accessibility.check-accessibility', label: 'Check Accessibility', icon: 'accessibility-checkmark' },
];

/**
 * PowerPoint's Language group: Translate and Language, both large.
 *
 * **Translate is a plain button drawing `translate`**: it opens the Translator pane on the selection.
 * `GUESS:` that it has no arrow. Word's offers Translate Document, and a deck has no document translation.
 * **Language is a dropdown drawing `local-language`** over Word's own two entries, Set Proofing Language
 * and Language Preferences.
 *
 * **No survivor**: a pane and a menu.
 */
const powerpointReviewLanguage: readonly RibbonCommand[] = [
  { id: 'powerpoint.review.language.translate', label: 'Translate', icon: 'translate', size: 'large' },
  { id: 'powerpoint.review.language.language', label: 'Language', icon: 'local-language', size: 'large' },
];

/**
 * PowerPoint's Comments group: New Comment, Delete, Previous Comment, Next Comment, Show Comments.
 *
 * **New Comment draws `comment-add`** (Ctrl+Alt+M) and **Delete draws `comment-dismiss`**, a large split
 * button whose arrow offers Delete, Delete All Comments and Ink on This Slide, and Delete All Comments and
 * Ink in This Presentation. **Previous Comment and Next Comment draw `comment-arrow-left` and
 * `comment-arrow-right`**, small; see this section's header on their size. **Show Comments draws
 * `comment-multiple`**, large, and is a split button whose face is a toggle: the face opens and closes the
 * Comments pane, starting unpressed because the pane starts closed, and the arrow holds Comments Pane and
 * Show Markup, checked. `GUESS:` the arrow and the starting position.
 *
 * **Survivors: Previous Comment and Next Comment.** See this section's header.
 */
const powerpointReviewComments: readonly RibbonCommand[] = [
  { id: 'powerpoint.review.comments.new-comment', label: 'New Comment', icon: 'comment-add', size: 'large' },
  { id: 'powerpoint.review.comments.delete', label: 'Delete', icon: 'comment-dismiss', size: 'large' },
  { id: 'powerpoint.review.comments.previous-comment', label: 'Previous Comment', icon: 'comment-arrow-left', essential: true },
  { id: 'powerpoint.review.comments.next-comment', label: 'Next Comment', icon: 'comment-arrow-right', essential: true },
  { id: 'powerpoint.review.comments.show-comments', label: 'Show Comments', icon: 'comment-multiple', size: 'large', toggle: true },
];

/**
 * PowerPoint's `GroupReviewCompare`, labelled **Compare**: Compare, Accept, Reject, then Previous Change,
 * Next Change and Reviewing Pane in a column, then End Review.
 *
 * **Compare** opens the file picker that merges another copy of the deck into this one. No icon, for
 * Word's reason, so `small`. **Accept draws `document-checkmark` and Reject draws `document-dismiss`**,
 * both large split buttons, as Office draws them in PowerPoint (Word's Reject is small). Accept's arrow
 * offers Accept Change, Accept All Changes to This Slide and Accept All Changes to the Presentation;
 * Reject's is the same three for rejecting. **Previous Change and Next Change** carry no icon.
 * **Reviewing Pane is a toggle drawing `panel-right`**, a pane at the right, where PowerPoint docks its
 * Revisions pane. `GUESS:` the glyph; Word's pane opens at the left and draws `panel-left-text`, and Fluent
 * has no right-hand drawing with text in it. **End Review** ends the comparison and discards every change
 * not yet accepted, after a confirmation. No icon; see this section's header.
 *
 * **No survivor**: a file picker, two split buttons, two navigators with no glyph, a toggle that opens a
 * pane, and a command that cannot be undone.
 */
const powerpointReviewCompare: readonly RibbonCommand[] = [
  { id: 'powerpoint.review.compare.compare', label: 'Compare' },
  { id: 'powerpoint.review.compare.accept', label: 'Accept', icon: 'document-checkmark', size: 'large' },
  { id: 'powerpoint.review.compare.reject', label: 'Reject', icon: 'document-dismiss', size: 'large' },
  { id: 'powerpoint.review.compare.previous-change', label: 'Previous Change' },
  { id: 'powerpoint.review.compare.next-change', label: 'Next Change' },
  { id: 'powerpoint.review.compare.reviewing-pane', label: 'Reviewing Pane', icon: 'panel-right', toggle: true },
  { id: 'powerpoint.review.compare.end-review', label: 'End Review' },
];

/**
 * PowerPoint's Activity group: Show Changes. `GUESS:` the whole group; see disagreement 3 in this
 * section's header.
 *
 * **No survivor**: no glyph, and the only command.
 */
const powerpointReviewActivity: readonly RibbonCommand[] = [
  { id: 'powerpoint.review.activity.show-changes', label: 'Show Changes' },
];

/**
 * PowerPoint's Ink group: Hide Ink.
 *
 * Word's shape: `GUESS:` **a split button whose face is a toggle**, hiding every ink stroke in the deck and
 * showing them again, starting unpressed. The arrow holds Hide Ink and Delete All Ink in Presentation.
 * `GUESS:` the second entry's wording. No icon, so `small`.
 *
 * **No survivor**: a split button, no glyph, and the only command.
 */
const powerpointReviewInk: readonly RibbonCommand[] = [
  { id: 'powerpoint.review.ink.hide-ink', label: 'Hide Ink', toggle: true },
];

// ── Excel's Review ───────────────────────────────────────────────────────────
//
// **Excel's Review tab**, all eleven in-scope groups, the third Review unit and the last under the
// one-tab-one-application rule. Word's section above is the pattern: the shapes, the whole-menu rule, the
// survivor standard and the icon rule are unchanged, and only what Excel does differently is written here.
//
// **Not one declaration shared with Word or PowerPoint**, which answers the question Word's section left
// open. Proofing, Accessibility, Language, Comments and Ink carry the same group ids in all three census
// tabs. Excel's Proofing adds Workbook Statistics, its Check Accessibility arrow has no pane, its Comments
// are three generations of Office and its Hide Ink says *sheet*. Translate is the one face that matches
// PowerPoint's command for command, and one shared declaration of one button would be a function of the
// application for the sake of a single line.
//
// ## The shapes
//
// **Dropdown or split button** over a menu written in `stories/ribbons/review-menus.ts`: Notes and Track
// Changes are dropdowns; Check Accessibility is a split button. **A split button whose face is a toggle**
// (`<mjx-split-button toggle>`): Hide Ink, starting unpressed. **Toggle**: Show Comments, Show/Hide
// Comment, Show All Comments and Protect Workbook. **Plain button**: Spelling, Thesaurus, Workbook
// Statistics, Check Performance, Translate, New Comment, Delete, Previous Comment, Next Comment, Protect
// Sheet, Allow Edit Ranges, Unshare Workbook, Share Workbook, Protect and Share Workbook, Debug. **No field,
// no gallery, no dialog launcher**: Office puts none on Excel's Review tab.
//
// ## ⚠ Where the census and Office disagree, recorded rather than smoothed over
//
// 1. **The declaration puts Performance tenth; Microsoft 365 draws Check Performance second, right after
//    Proofing.** It also puts Ink before Protect and Changes, where Microsoft 365 draws Ink last of the
//    groups it shows. The group order is the tab module's decision, so `excelReviewTab` draws Proofing,
//    Performance, Accessibility, Language, Threaded Comments, Comments, Notes, Protect, Changes, Ink,
//    Debug. Debug stays last because nothing says where Office puts it. `GUESS:` Performance's and Ink's
//    positions; no build this project can cite is checked.
// 2. **Office draws groups the census marks out of scope, and they are not drawn here**: **Insights**
//    (Smart Lookup, one control) and **Lineage** (one control). The census wins.
// 3. **Comments, Threaded Comments and Notes are one Office group in three generations**, as Data's three
//    Connections groups are. **Comments** (6) is Office 2016's face: New Comment, Delete, Previous, Next,
//    Show/Hide Comment and Show All Comments, all acting on what Microsoft 365 calls a note. **Threaded
//    Comments** (5) is Microsoft 365's Comments face: New Comment, Delete, Previous Comment, Next Comment,
//    Show Comments. **Notes** (7) is Microsoft 365's Notes dropdown and its six entries, exactly. Data's
//    rule applies, and a face command is drawn **once**. So Threaded Comments draws its five; Comments
//    draws the two face toggles neither 365 group draws on its face, **Show/Hide Comment** and **Show All
//    Comments**; Notes is the one dropdown. ⚠ Those two toggles and Notes' *Show/Hide Note* and *Show All
//    Notes* entries are the same two commands under two generations' names, one on a face and one in a
//    menu. That repetition is recorded, not removed: the brief requires the Notes menu whole, and Comments
//    would otherwise be a group the census declares and nothing draws. `GUESS:` the whole reading.
// 4. **Changes is Office 2016's group, and Microsoft 365's names differ.** The census counts **8**, which
//    is 2016's face exactly: Protect Sheet, Protect Workbook, Share Workbook, Protect and Share Workbook,
//    Allow Users to Edit Ranges and Track Changes, whose menu holds Highlight Changes and Accept/Reject
//    Changes. Protect Sheet, Protect Workbook and Allow Users to Edit Ranges are Protect's (Microsoft 365
//    calls the last *Allow Edit Ranges*), so Changes draws the other three. Microsoft 365 hides all three
//    and offers them only through ribbon customisation, as *Share Workbook (Legacy)*, *Protect Sharing
//    (Legacy)* and *Track Changes (Legacy)*. The census's group label is 2016's, so the commands carry
//    2016's names. `GUESS:` that reading. **Microsoft 365's Show Changes** has no census row on this tab
//    and is not drawn.
// 5. **Debug is `GUESS:` in its entirety.** The census names the group and counts one control, and says
//    nothing else. It is drawn as Power Options is on Home: one small button carrying the group's own
//    label, and no icon.
// 6. **Office greys Unshare Workbook** unless the workbook is a legacy shared workbook, and greys Allow
//    Edit Ranges while the sheet is protected. Both are drawn available; `disabled` is loop 2's.
// 7. **The census's counts are larger than the faces**, and nothing is padded: Accessibility is 9 and
//    draws one; Ink is 5 and draws one; Language is 3 and draws one.
//
// ## Survivors: Previous Comment and Next Comment, and nothing else
//
// Word's judgement, on the same glyphs, in Threaded Comments. **Show/Hide Comment and Show All Comments**
// pass rule 1 and fail rule 2 for want of an honest glyph: `comment-multiple` is Show Comments beside
// them. **Protect Workbook** is a toggle that asks for a password first. Every other command opens a
// pane, a dialog or a menu, is a split button, or (Unshare Workbook) cannot be undone.
//
// ## Sizes, and the commands that carry no icon
//
// `large` where Office draws it large **and** there is an honest glyph **and** the label wraps inside
// `largeControlWidthUnits`. **Spelling**, **Translate**, **New Comment**, **Show Comments**, **Notes**,
// **Protect Sheet** and **Protect Workbook** are large. **Check Accessibility** is small, for Word's
// reason, and **Check Performance** for the same one: *Performance* is eleven letters, the length of the
// *Recommended* that unit 3 measured clipping. **Allow Edit Ranges** is three tokens. Previous Comment and
// Next Comment are small survivors, as in PowerPoint.
//
// **Show/Hide Comment**, **Show All Comments**, **Allow Edit Ranges**, **Unshare Workbook**, **Share
// Workbook**, **Protect and Share Workbook** and **Debug** carry no icon, so each is `small` and its label
// is the command. The two comment toggles are refused above. Fluent draws **no edit range**: `table-edit`
// is editing a table, and `edit-lock` is editing locked, which is the opposite of allowing it. It draws
// **no legacy shared workbook**: `share` is File's Share, which is a different command, and `people` is
// Open's Shared with Me. **Unshare** would be a share mark struck out, which Fluent does not draw.

/**
 * Excel's Proofing group: Spelling large, then Thesaurus and Workbook Statistics in a column.
 *
 * **Spelling draws `text-grammar-checkmark`**, as in PowerPoint, and opens the Spelling dialog (F7).
 * **Thesaurus draws `book-search`**, as in Word and PowerPoint, and opens the Thesaurus pane (Shift+F7).
 * **Workbook Statistics draws `data-histogram`**, which is File's Workbook Statistics on the Info page
 * because it is the same dialog.
 *
 * **No survivor**: a dialog, a pane and a dialog.
 */
const excelReviewProofing: readonly RibbonCommand[] = [
  { id: 'excel.review.proofing.spelling', label: 'Spelling', icon: 'text-grammar-checkmark', size: 'large' },
  { id: 'excel.review.proofing.thesaurus', label: 'Thesaurus', icon: 'book-search' },
  { id: 'excel.review.proofing.workbook-statistics', label: 'Workbook Statistics', icon: 'data-histogram' },
];

/**
 * Excel's Performance group: Check Performance.
 *
 * **Draws `top-speed`**, a speedometer, which is Office's own picture of a gauge for this pane. It opens
 * the Workbook Performance pane, which finds formatted empty cells and offers to clear them. `small`; see
 * this section's header. `GUESS:` the glyph.
 *
 * **No survivor**: a pane, and the only command.
 */
const excelReviewPerformance: readonly RibbonCommand[] = [
  { id: 'excel.review.performance.check-performance', label: 'Check Performance', icon: 'top-speed' },
];

/**
 * Excel's Accessibility group: Check Accessibility.
 *
 * **A split button drawing `accessibility-checkmark`**, as in Word and PowerPoint. The face runs the
 * checker and opens its pane; the arrow offers Check Accessibility, Alt Text and Options: Accessibility.
 * Excel's arrow has neither Word's Navigation Pane nor PowerPoint's Reading Order Pane. `GUESS:` the list.
 *
 * **No survivor**: a split button, and the only command.
 */
const excelReviewAccessibility: readonly RibbonCommand[] = [
  { id: 'excel.review.accessibility.check-accessibility', label: 'Check Accessibility', icon: 'accessibility-checkmark' },
];

/**
 * Excel's Language group: Translate.
 *
 * **A plain large button drawing `translate`**, as in PowerPoint: it opens the Translator pane on the
 * selected cells. `GUESS:` that it has no arrow. Excel has no Language dropdown here.
 *
 * **No survivor**: a pane, and the only command.
 */
const excelReviewLanguage: readonly RibbonCommand[] = [
  { id: 'excel.review.language.translate', label: 'Translate', icon: 'translate', size: 'large' },
];

/**
 * Excel's `GroupThreadedComments`, Microsoft 365's Comments: New Comment large, then Delete, Previous
 * Comment and Next Comment in a column, then Show Comments large.
 *
 * **New Comment draws `comment-add`**, Insert's Comment glyph, because it is the same command. **Delete
 * draws `comment-dismiss`** and is a plain button: it deletes the selected cell's thread.
 * `GUESS:` that Excel's has no arrow, unlike PowerPoint's. **Previous Comment and Next Comment draw
 * `comment-arrow-left` and `comment-arrow-right`**. **Show Comments is a toggle drawing
 * `comment-multiple`**: Office draws it pressed while the Comments pane is open, and it starts unpressed.
 * `GUESS:` the sizes, and that Show Comments has no arrow.
 *
 * **Survivors: Previous Comment and Next Comment.** See this section's header.
 */
const excelReviewThreadedComments: readonly RibbonCommand[] = [
  { id: 'excel.review.threaded-comments.new-comment', label: 'New Comment', icon: 'comment-add', size: 'large' },
  { id: 'excel.review.threaded-comments.delete', label: 'Delete', icon: 'comment-dismiss' },
  { id: 'excel.review.threaded-comments.previous-comment', label: 'Previous Comment', icon: 'comment-arrow-left', essential: true },
  { id: 'excel.review.threaded-comments.next-comment', label: 'Next Comment', icon: 'comment-arrow-right', essential: true },
  { id: 'excel.review.threaded-comments.show-comments', label: 'Show Comments', icon: 'comment-multiple', size: 'large', toggle: true },
];

/**
 * Excel's `GroupComments`, Office 2016's: Show/Hide Comment and Show All Comments.
 *
 * Both are toggles. **Show/Hide Comment** keeps the selected cell's comment on screen, and **Show All
 * Comments** keeps every one on screen. Office draws each pressed while it holds, and both start
 * unpressed. No icon; see this section's header. See disagreement 3 on why the other four commands of
 * Office 2016's face are not here.
 *
 * **No survivor**: two toggles with no glyph.
 */
const excelReviewComments: readonly RibbonCommand[] = [
  { id: 'excel.review.comments.show-hide-comment', label: 'Show/Hide Comment', toggle: true },
  { id: 'excel.review.comments.show-all-comments', label: 'Show All Comments', toggle: true },
];

/**
 * Excel's `GroupCommentsLegacy`, labelled **Notes**: one large dropdown.
 *
 * **Notes draws `note`**, a sticky note, Office's own picture for a note. The menu holds New Note,
 * Previous Note, Next Note, Show/Hide Note, Show All Notes and Convert to Comments. Its six entries and
 * the dropdown are the census's seven controls.
 *
 * **No survivor**: a menu, and the only command.
 */
const excelReviewNotes: readonly RibbonCommand[] = [
  { id: 'excel.review.notes.notes', label: 'Notes', icon: 'note', size: 'large' },
];

/**
 * Excel's `GroupProtectExcel`, labelled **Protect**: Protect Sheet, Protect Workbook, Allow Edit Ranges,
 * Unshare Workbook.
 *
 * **Protect Sheet draws `table-lock`**, a grid with a padlock, and opens the Protect Sheet dialog. Office
 * relabels it *Unprotect Sheet* once the sheet is protected rather than drawing it pressed. `GUESS:` the
 * glyph. **Protect Workbook is a toggle drawing `document-lock`**, File's Protect Workbook glyph, because
 * protecting the workbook's structure is what that menu's *Protect Workbook Structure* opens. Office draws
 * it pressed while the structure is protected, and it starts unpressed. **Allow Edit Ranges** opens the
 * dialog of ranges a protected sheet still lets people edit. **Unshare Workbook** stops legacy sharing.
 * Neither carries an icon; see this section's header.
 *
 * **No survivor**: two dialogs, a toggle that asks for a password, and a command that cannot be undone.
 */
const excelReviewProtect: readonly RibbonCommand[] = [
  { id: 'excel.review.protect.protect-sheet', label: 'Protect Sheet', icon: 'table-lock', size: 'large' },
  { id: 'excel.review.protect.protect-workbook', label: 'Protect Workbook', icon: 'document-lock', size: 'large', toggle: true },
  { id: 'excel.review.protect.allow-edit-ranges', label: 'Allow Edit Ranges' },
  { id: 'excel.review.protect.unshare-workbook', label: 'Unshare Workbook' },
];

/**
 * Excel's `GroupChangesExcel`, Office 2016's **Changes**: Share Workbook, Protect and Share Workbook, Track
 * Changes. See disagreement 4 in this section's header.
 *
 * **Share Workbook** opens the legacy Share Workbook dialog, and **Protect and Share Workbook** opens the
 * dialog that shares it with change tracking locked on. Neither carries an icon. **Track Changes is a
 * dropdown drawing `document-edit`**, Word's Track Changes glyph, because it is the same idea: its menu
 * holds Highlight Changes and Accept/Reject Changes. Excel's Track Changes has no face of its own, so it
 * is not a toggle here as Word's is.
 *
 * **No survivor**: two dialogs and a menu.
 */
const excelReviewChanges: readonly RibbonCommand[] = [
  { id: 'excel.review.changes.share-workbook', label: 'Share Workbook' },
  { id: 'excel.review.changes.protect-and-share-workbook', label: 'Protect and Share Workbook' },
  { id: 'excel.review.changes.track-changes', label: 'Track Changes', icon: 'document-edit' },
];

/**
 * Excel's Ink group: Hide Ink.
 *
 * PowerPoint's shape: `GUESS:` **a split button whose face is a toggle**, hiding every ink stroke on the
 * sheet and showing them again, starting unpressed. The arrow holds Hide Ink and Delete All Ink on Sheet.
 * `GUESS:` the second entry's wording. No icon, so `small`.
 *
 * **No survivor**: a split button, no glyph, and the only command.
 */
const excelReviewInk: readonly RibbonCommand[] = [
  { id: 'excel.review.ink.hide-ink', label: 'Hide Ink', toggle: true },
];

/**
 * Excel's Debug group. `GUESS:` the whole group; see disagreement 5 in this section's header.
 *
 * **No survivor**: no glyph, and the only command.
 */
const excelReviewDebug: readonly RibbonCommand[] = [
  { id: 'excel.review.debug.debug', label: 'Debug' },
];

// ── the commands View shows ──────────────────────────────────────────────────
//
// The ribbon programme's unit after the three Review tabs: **Word's View tab**, all seven in-scope groups,
// under the one-tab-one-application rule. PowerPoint's View and Excel's View followed in the *PowerPoint's
// View* and *Excel's View* parts below, and nothing here is written as a function of the application. Zoom and Window carry the same census ids in
// all three, and whether any of them is one declaration is their own units' question.
//
// ## The shapes, decided by what Office's popup is
//
// Almost nothing on this tab opens anything, and that is the tab's character: it changes how the document
// is *looked at*, never the document. **Toggles**: the five views, Focus, the two page movements, View Side
// by Side, Synchronous Scrolling and Switch Modes. **Checkboxes a host binds**: Ruler, Gridlines and
// Navigation Pane, which Office draws as three ticks. **One dropdown**, Switch Windows, over a menu written
// once in `stories/ribbons/view-menus.ts`. **Buttons**: Immersive Reader, Zoom (a dialog), 100%, One Page,
// Multiple Pages, Page Width, New Window, Arrange All, Split and Reset Window Position. **No field, no
// gallery, no dialog launcher**: Office puts none on Word's View tab.
//
// **Two exclusive sets.** Word is always in exactly one of Read Mode, Print Layout, Web Layout, Outline and
// Draft, and scrolls in exactly one of Vertical and Side to Side. Pressing the view it is already in keeps
// it. So the five views are the set `word.view.document-views`, Print Layout pressed, and the two page
// movements are `word.view.page-movement`, Vertical pressed: pressing Web Layout releases Print Layout, and
// pressing Print Layout while it holds keeps it. The mechanism is `src/controls/exclusive-set.ts`, the
// same one the Draw tab's tools use.
//
// ## ⚠ Where the census and Office disagree, recorded rather than smoothed over
//
// 1. **The declaration puts Page Movement sixth; Microsoft 365 draws it third**, after Immersive. Office's
//    View tab reads Views, Immersive, Page Movement, Show, Zoom, Window, Macros, SharePoint. The group
//    order is the tab module's decision, so `wordViewTab` draws Document Views, Modes, Page Movement, Show,
//    Zoom, Window, Night Mode. **Night Mode stays last** because nothing this project can cite says where
//    Office puts it. `GUESS:` both positions.
// 2. **The census's group labels are not Microsoft 365's**, and the census's are drawn, because a label is
//    not an argument `censusGroup` accepts. **Document Views** is Word 2010's label; Microsoft 365 says
//    **Views**. **Modes** (`GroupModes`) is the group Microsoft 365 labels **Immersive**. **Night Mode**
//    (`GroupNightMode`) is the group Microsoft 365 labels **Dark Mode**. `GUESS:` the last two readings.
// 3. **There is no separate Immersive group in the census.** `GroupModes` counts three controls, and
//    Microsoft 365's Immersive group draws two, Focus and Immersive Reader, so both are declared here.
//    `GUESS:` that `GroupModes` is that group. Word 2016's Learning Tools button, which Immersive Reader
//    replaced, is not drawn beside it: a command is drawn once.
// 4. **Office draws groups the census marks out of scope, and they are not drawn here**: **Macros** (four
//    controls, the automation runtime this project excludes) and **SharePoint** (`GroupSharePointProperties`,
//    one control). The census wins.
// 5. **Office shows Switch Modes only while Office's theme is Black**, and draws the group nowhere
//    otherwise. The census declares the group in scope, so it is drawn always. `GUESS:` the command's name
//    and its shape as a toggle.
// 6. **Split is a plain button.** Office relabels it *Remove Split* while the window is split rather than
//    drawing it pressed, as Excel's Protect Sheet becomes Unprotect Sheet. **Synchronous Scrolling and
//    Reset Window Position** are greyed in Office until View Side by Side is on; both are drawn available,
//    because `disabled` is loop 2's.
// 7. **The census's counts are larger than the faces**, and nothing is padded: Show is 4 and draws three;
//    Zoom is 6 and draws five; Window is 8 and draws seven; Modes is 3 and draws two.
//
// ## Survivors: 100%, One Page and Page Width, and nothing else
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Zoom's 100%, One Page and Page Width survive.** Each sets the zoom to one value in one press, and
//   the zoom before it is one press away: the Mailings record-navigator standard, because a zoom changes
//   no document. Each glyph is its own: `ratio-one-to-one` is *1:1*, actual size; `document-fit` is a page
//   inside fit corners; `auto-fit-width` is a width between two stops. That is three, the ceiling, and Zoom
//   and Multiple Pages stay in the popup. `GUESS:` rule 2 on all three glyphs.
// - **Multiple Pages passes rule 1 and fails rule 2**: it carries no glyph. Fluent's
//   `document-one-page-multiple` is a stack of pages, which reads as *several documents* or *copies*,
//   where the command lays pages side by side.
// - **The views and Focus are modes, not presses.** Read Mode and Focus take over the whole window and hide
//   the ribbon, and Outline opens the Outlining tab. A press on a view is not taken back by a second press on
//   it, only by pressing another view, which rule 1 does not allow. **Page Movement** is two commands with no
//   glyph, and a survivor of each would leave its collapsed popup empty.
// - **Show's three are checkboxes a host binds**, which the gate refuses as survivors, and none has a glyph.
// - **Window**: New Window opens a window, Arrange All and Split rearrange every window (Split then arms a
//   bar the pointer places), View Side by Side asks which document when more than two are open, and Switch
//   Windows is a menu. Synchronous Scrolling passes rule 1 and has no glyph. **Night Mode** and **Immersive
//   Reader** (a tab of its own) fail on their own grounds; each group states its reason below.
//
// ## Sizes, and the commands that carry no icon
//
// Unit 6's rule: `large` where Office draws it large **and** there is an honest glyph **and** the label
// wraps inside `largeControlWidthUnits`. **Read Mode, Print Layout, Web Layout, Focus, Immersive Reader,
// 100%, New Window, Split, Switch Windows and Switch Modes** are large.
//
// **Outline and Draft carry an icon and stay small**, where Office draws them in a column beside the three
// large views. **Outline draws `list-bar-tree-offset`**, bars stepping one level deeper each, which is the
// heading hierarchy the view shows. It is deliberately not `text-bullet-list-tree`, the nearer picture,
// because that is **Multilevel List** on Home's Paragraph group and would make the view look like a list
// command. **Draft draws `drafts`**, lines of text with a pencil and no page around them, which is the
// view: the text, edited, with the page layout taken away. Plain lines would be every alignment glyph, and
// a page of lines would be Print Layout's `document-one-page`. `GUESS:` both glyphs.
//
// A wrong icon is worse than none, and this tab is where Fluent's gaps show most:
//
// - **Vertical and Side to Side**: Fluent's vertical scroll marks are a phone and a dual screen, and its open
//   book is Read Mode beside them. No page with an arrow down, no pages turning sideways.
// - **Ruler, Gridlines and Navigation Pane** are checkboxes, which draw no glyph.
// - **Zoom**: the plain magnifier is `search`, Find, and `zoom-in` is *Zoom In*, which the dialog is not.
//   **Multiple Pages**: see its survivor reason above.
// - **Arrange All**: `layout-row-two` is one window divided, which is Split's meaning rather than two windows
//   stacked, and it would sit beside Split's `split-horizontal`.
// - **Synchronous Scrolling**: `arrow-sync` is AutoSave's. **Reset Window Position**: `arrow-reset` is
//   PowerPoint's Reset.
//
// So those ten are `small`, and the label is the command.

/**
 * Word's Document Views group: Read Mode, Print Layout and Web Layout large, then Outline and Draft in a
 * column.
 *
 * **Five toggles and one exclusive set, Print Layout pressed**, because Print Layout is a new document's
 * view and Word is in exactly one view at a time; see this section's header. **Read Mode draws
 * `book-open`**, an open book, Office's own picture for the reading view. **Print Layout draws
 * `document-one-page`**, a printed page. **Web Layout draws `globe`**, Excel's From Web glyph on another
 * application's tab, because Office's own is a page with a globe on it. **Outline draws
 * `list-bar-tree-offset`** and **Draft draws `drafts`**, both small; see this section's header for why
 * neither is the nearer-looking glyph. `GUESS:` all five glyphs.
 *
 * **No survivor**: five modes, one of which hides the ribbon and one of which opens a tab.
 */
const wordViewDocumentViews: readonly RibbonCommand[] = [
  { id: 'word.view.document-views.read-mode', label: 'Read Mode', icon: 'book-open', size: 'large', toggle: true, exclusive: 'word.view.document-views' },
  { id: 'word.view.document-views.print-layout', label: 'Print Layout', icon: 'document-one-page', size: 'large', toggle: true, pressed: true, exclusive: 'word.view.document-views' },
  { id: 'word.view.document-views.web-layout', label: 'Web Layout', icon: 'globe', size: 'large', toggle: true, exclusive: 'word.view.document-views' },
  { id: 'word.view.document-views.outline', label: 'Outline', icon: 'list-bar-tree-offset', toggle: true, exclusive: 'word.view.document-views' },
  { id: 'word.view.document-views.draft', label: 'Draft', icon: 'drafts', toggle: true, exclusive: 'word.view.document-views' },
];

/**
 * Word's `GroupModes`, Microsoft 365's **Immersive**: Focus and Immersive Reader, both large. See
 * disagreements 2 and 3 in this section's header.
 *
 * **Focus is a toggle drawing `full-screen-maximize`**, four corners pushed outward: Focus fills the screen
 * with the page and hides everything else, and Esc or a second press leaves it. `GUESS:` the glyph, and
 * that Office draws it pressed. **Immersive Reader draws `immersive-reader`**, Fluent's own mark for the
 * product, and opens the Immersive Reader tab. It is a plain button: the tab's Close Immersive Reader is
 * how it ends.
 *
 * **No survivor**: a mode that hides the ribbon, and a command that opens a tab.
 */
const wordViewModes: readonly RibbonCommand[] = [
  { id: 'word.view.modes.focus', label: 'Focus', icon: 'full-screen-maximize', size: 'large', toggle: true },
  { id: 'word.view.modes.immersive-reader', label: 'Immersive Reader', icon: 'immersive-reader', size: 'large' },
];

/**
 * Word's Page Movement group: Vertical and Side to Side.
 *
 * **Two toggles and one exclusive set, Vertical pressed**, because a new document scrolls vertically and
 * one movement holds at a time in Office. Neither carries an icon, so both are `small` where Office draws
 * them large; see this section's header.
 *
 * **No survivor**: no glyph, and a survivor of each would leave the popup empty.
 */
const wordViewPageMovement: readonly RibbonCommand[] = [
  { id: 'word.view.page-movement.vertical', label: 'Vertical', toggle: true, pressed: true, exclusive: 'word.view.page-movement' },
  { id: 'word.view.page-movement.side-to-side', label: 'Side to Side', toggle: true, exclusive: 'word.view.page-movement' },
];

/**
 * Word's `GroupViewShowHide`, labelled **Show**: Ruler, Gridlines and Navigation Pane.
 *
 * **Three toggles, drawn as checkboxes**, as Excel's Sheet Options are: Office draws three ticks, and each
 * host binds an `<mjx-checkbox>`. **Navigation Pane starts ticked**, because the assembled shell draws the
 * pane open beside the page. Ruler and Gridlines start unticked, as in a new document since Word 2013.
 * `GUESS:` Navigation Pane's start, which follows the shell rather than a new document.
 *
 * **No survivor, although all three pass rule 1**: none carries a glyph, and each is a checkbox a host
 * binds.
 */
const wordViewShow: readonly RibbonCommand[] = [
  { id: 'word.view.show.ruler', label: 'Ruler', toggle: true },
  { id: 'word.view.show.gridlines', label: 'Gridlines', toggle: true },
  { id: 'word.view.show.navigation-pane', label: 'Navigation Pane', toggle: true, pressed: true },
];

/**
 * Word's Zoom group: Zoom, 100% large, then One Page, Multiple Pages and Page Width in a column.
 *
 * **Zoom** opens the Zoom dialog and carries no icon, so it is `small` where Office draws it large. **100%
 * draws `ratio-one-to-one`**, *1:1*, and sets the zoom to actual size. **One Page draws `document-fit`**, a
 * page inside four fit corners, and fits one whole page in the window. **Multiple Pages** fits as many
 * pages as the window holds side by side, and carries no icon. **Page Width draws `auto-fit-width`**, a
 * width between two stops, and fits the page's width to the window. `GUESS:` the three glyphs.
 *
 * **Survivors: 100%, One Page and Page Width.** See this section's header.
 */
const wordViewZoom: readonly RibbonCommand[] = [
  { id: 'word.view.zoom.zoom', label: 'Zoom' },
  { id: 'word.view.zoom.one-hundred-percent', label: '100%', icon: 'ratio-one-to-one', size: 'large', essential: true },
  { id: 'word.view.zoom.one-page', label: 'One Page', icon: 'document-fit', essential: true },
  { id: 'word.view.zoom.multiple-pages', label: 'Multiple Pages' },
  { id: 'word.view.zoom.page-width', label: 'Page Width', icon: 'auto-fit-width', essential: true },
];

/**
 * Word's Window group: New Window, Arrange All and Split, then View Side by Side, Synchronous Scrolling and
 * Reset Window Position in a column, then Switch Windows large.
 *
 * **New Window draws `window-new`**, a window with an arrow leaving it, and opens a second window on the
 * same document. **Arrange All** tiles every open Word window, and carries no icon. **Split draws
 * `split-horizontal`**, a window cut across, and is a plain button; see disagreement 6. **View Side by Side
 * is a toggle drawing `column-double-compare`**, two panes set beside each other to be compared, which is
 * what it does to two documents. Fluent draws it at 20 alone, and the command is small. **Synchronous
 * Scrolling is a toggle** with no icon; Office presses it as soon as View Side by Side is on, and it starts
 * unpressed here because View Side by Side does. **Reset Window Position** shares the screen equally again,
 * and carries no icon. **Switch Windows is a large dropdown drawing `window-multiple`**, windows overlapping,
 * and lists every open window. `GUESS:` Split's and Switch Windows' glyphs, and the sizes.
 *
 * **No survivor**: see this section's header.
 */
const wordViewWindow: readonly RibbonCommand[] = [
  { id: 'word.view.window.new-window', label: 'New Window', icon: 'window-new', size: 'large' },
  { id: 'word.view.window.arrange-all', label: 'Arrange All' },
  { id: 'word.view.window.split', label: 'Split', icon: 'split-horizontal', size: 'large' },
  { id: 'word.view.window.view-side-by-side', label: 'View Side by Side', icon: 'column-double-compare', toggle: true },
  { id: 'word.view.window.synchronous-scrolling', label: 'Synchronous Scrolling', toggle: true },
  { id: 'word.view.window.reset-window-position', label: 'Reset Window Position' },
  { id: 'word.view.window.switch-windows', label: 'Switch Windows', icon: 'window-multiple', size: 'large' },
];

/**
 * Word's `GroupNightMode`, Microsoft 365's **Dark Mode**: Switch Modes. See disagreements 2 and 5 in this
 * section's header.
 *
 * **A large toggle drawing `dark-theme`**, a circle half dark: Office draws it pressed while the page is
 * drawn light inside a dark Office, and the press turns the page dark again. It starts unpressed.
 * `GUESS:` the whole group.
 *
 * **No survivor**: the only command.
 */
const wordViewNightMode: readonly RibbonCommand[] = [
  { id: 'word.view.night-mode.switch-modes', label: 'Switch Modes', icon: 'dark-theme', size: 'large', toggle: true },
];

// ── PowerPoint's View ────────────────────────────────────────────────────────
//
// **PowerPoint's View tab**, the second View tab, one tab of one application again and to Word's pattern:
// all seven in-scope groups, twenty-four commands. Excel's View tab followed it. **Nothing is
// declared once with Word's.** Zoom and Window carry the same census ids in both, and their faces differ:
// PowerPoint's Zoom is two commands where Word's is five, and its Window has Cascade and Move Split where
// Word's has Split and the side-by-side comparison. A function of the application would be two
// declarations behind one name. What *is* shared is glyphs (Reading View, Outline View, New Window, Switch
// Windows) and the Switch Windows menu's shape in `stories/ribbons/view-menus.ts`.
//
// ## The shapes, decided by what Office's popup is
//
// **Toggles in three exclusive sets**, because PowerPoint holds exactly one member of each:
//
// - `powerpoint.view.presentation-views`: Normal (pressed), Outline View, Slide Sorter, Notes Page, Reading View.
// - `powerpoint.view.colour-greyscale`: Colour (pressed), Greyscale, Black and White.
// - `powerpoint.view.view-direction`: Left-to-Right (pressed), Right-to-Left. See disagreement 4.
//
// **One plain toggle**, Notes, which shows the notes pane under the slide and hides it again. It starts
// unpressed, because the assembled shell's status bar reads *Notes: Hidden*. **Checkboxes a host binds**:
// Ruler, Gridlines and Guides, which Office draws as three ticks. **One dropdown**, Switch Windows, over a menu
// in `stories/ribbons/view-menus.ts`. **Buttons**: Slide Master, Handout Master, Notes Master, Zoom (a
// dialog), Fit to Window, New Window, Arrange All, Cascade and Move Split. **One dialog launcher**, on Show,
// where Office opens its Grid and Guides dialog; `powerpointViewTab` names it *Grid Settings*. `GUESS:` the
// name. **No field, no gallery, no split button**: nothing on this tab is a state with a menu behind it, so
// the split toggle Word's and PowerPoint's Review tabs use has no candidate here.
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **The census's spelling wins over Office's.** Office writes *Color/Grayscale*, *Color* and *Grayscale*;
//    the census labels the group *Colour/Greyscale*, and this catalogue's own view tab is *Greyscale*. So the
//    commands are **Colour**, **Greyscale** and **Black and White**.
// 2. **Greyscale and Black and White open view tabs in Office** (`TabGrayscale`, `TabBlackAndWhite`), whose
//    *Back To Color View* returns the deck to colour. Here the three are one set and pressing Colour releases
//    the other two; the two view tabs are their own stories, both authored in *the commands the colour
//    modes show*. `GUESS:` that Office draws Colour pressed while the deck is in colour.
// 3. **Slide Master, Handout Master and Notes Master open view tabs too**, and Office leaves them by *Close
//    Master View*. They are buttons here, because nothing on this tab takes them back.
// 4. **View Direction is the census's alone.** The census names `GroupViewDirection`, counts three controls
//    and describes none, and no Microsoft 365 build this project can cite shows it on an ordinary View tab.
//    `GUESS:` that it is the group Office adds while a right-to-left editing language is enabled, holding
//    **Left-to-Right** and **Right-to-Left**, which flip which side the window's panes sit on; that the two are
//    one exclusive set; that Left-to-Right starts pressed; and both labels. It is drawn last, where the
//    declaration puts it, because nothing says where Office puts it.
// 5. **Macros** (`GroupMacros`, one control) is out of scope in the census and is not drawn.
// 6. **The census's counts are larger than three faces**, and nothing is padded: Show is 5 and draws four
//    commands and a launcher; Colour/Greyscale is 4 and draws three; Window is 6 and draws five; View Direction
//    is 3 and draws two.
// 7. **Office greys Arrange All and Cascade with one window open, and Move Split outside Normal view.** All
//    three are drawn available, because `disabled` is loop 2's.
// 8. **Ruler starts ticked**, following the shell's own slide context menu, which shows Ruler checked, where a
//    new deck in Office starts with it off. `GUESS:`, and the same call Word's Navigation Pane makes.
//
// ## Survivors: Fit to Window, and nothing else
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Fit to Window survives.** One press fits the slide to the window and changes no deck, which is the
//   standard Word's zoom presets pass. `page-fit` is a landscape frame inside fit corners, no other command's
//   glyph in this subset, and the slide-shaped sibling of Word's One Page. Zoom stays in the popup, so the
//   popup is never empty. `GUESS:` rule 2.
// - **Zoom** opens a dialog: rule 1.
// - **Presentation Views, Colour/Greyscale and View Direction** are exclusive sets. A press is taken back only
//   by pressing another member, which rule 1 does not allow; Reading View also takes over the window, and
//   Greyscale and Black and White open tabs.
// - **Master Views** each open a view tab: rule 1.
// - **Show**: Ruler, Gridlines and Guides are checkboxes a host binds, which the gate refuses. **Notes passes
//   rule 1 and fails rule 2**: `panel-bottom` says *a pane at the foot of a window*, not *notes*, once the label
//   is gone. It is on the status bar at every width besides.
// - **Window**: New Window opens a window, Arrange All and Cascade rearrange every window, Move Split arms a
//   keyboard mode the arrow keys and Enter end, and Switch Windows is a menu.
//
// ## Sizes, and every glyph
//
// Word's rule: `large` where Office draws it large **and** Fluent draws its glyph at 24 **and** the label wraps
// inside `largeControlWidthUnits`. **Normal, Slide Sorter, Notes Page, Reading View, Slide Master, Handout
// Master, Zoom, Fit to Window, New Window and Switch Windows** are large. **Outline View, Notes Master and
// Notes are small where Office draws them large**, because Fluent draws `list-bar-tree-offset`, `notepad-edit`
// and `panel-bottom` at 20 alone. The colour modes, the view directions, Arrange All, Cascade and Move Split
// are small, as Office draws them.
//
// **Every command on the tab's face carries a glyph except Ruler, Gridlines and Guides**, which are
// checkboxes and draw a tick box rather than an icon. Every glyph below is `GUESS:`, judged from Fluent's
// drawings rather than from a build this project can cite:
//
// - **Normal draws `panel-left`**, a window with a narrow pane at its left: the thumbnails beside the slide,
//   which is Normal view. Not `panel-left-text`, Word's Reviewing Pane, nor `slide-layout`, Layout's.
// - **Outline View draws `list-bar-tree-offset`**, Word's Outline, because it is the same view: headings a
//   level deeper each.
// - **Slide Sorter draws `slide-grid`**, slides in a grid, which is the view.
// - **Notes Page draws `notepad`**, a page of lines. **Notes Master draws `notepad-edit`**, the same page with a
//   pencil, as **Slide Master draws `slide-text-edit`**, a slide with a pencil: a master is that kind of page,
//   edited. **Handout Master draws `document-one-page-multiple`**, pages stacked. Word refused it for Multiple
//   Pages because it reads as *copies*, and copies to hand out are what a handout is.
// - **Reading View draws `book-open`**, Word's Read Mode, because it is the same picture for the same idea.
// - **Notes draws `panel-bottom`**, the notes pane at the slide's foot. Not `note`, which is Excel's Notes.
// - **Zoom draws `zoom-in`**, a magnifier. Word's Zoom carries none, because `zoom-in` reads *Zoom In*; no Zoom
//   In command is in this subset, and a label-only Zoom beside a glyph-led Fit to Window reads as a gap. A
//   disagreement with Word's unit, which this unit does not reopen.
// - **Fit to Window draws `page-fit`**; see the survivors above.
// - **Colour draws `color`**, the palette Word's Design Colours draws, because both are colour. **Greyscale
//   draws `color-off`**, that palette struck through: the colour taken out. **Black and White draws
//   `circle-half-fill`**, a circle half black and half white, and not Word's Switch Modes' `dark-theme`, which
//   halves the other way.
// - **New Window and Switch Windows** draw Word's `window-new` and `window-multiple`.
// - **Arrange All draws `layout-column-two`**, two windows side by side, which is how PowerPoint tiles them.
//   Word's Arrange All carries none because it sits beside Split; PowerPoint's Window has no Split. Move
//   Split's panes are inside one window, and a reader could still take the glyph for a split. **Cascade draws
//   `stack`**, squares offset down a diagonal, the nearest glyph to Switch Windows' overlap on this tab.
//   **Move Split draws `arrow-move`**, the four arrows whose keys move the split bars.
// - **Left-to-Right and Right-to-Left draw `text-direction-horizontal-ltr` and `text-direction-horizontal-rtl`**,
//   a letter and an arrow each way.

/**
 * PowerPoint's Presentation Views: Normal, Outline View, Slide Sorter, Notes Page and Reading View.
 *
 * **Five toggles and one exclusive set, Normal pressed**, because a deck opens in Normal and PowerPoint is in
 * exactly one view at a time. All large but Outline View, whose glyph Fluent draws at 20 alone. See this
 * section's header for every glyph.
 *
 * **No survivor**: an exclusive set, which a second press does not take back.
 */
const powerpointViewPresentationViews: readonly RibbonCommand[] = [
  { id: 'powerpoint.view.presentation-views.normal', label: 'Normal', icon: 'panel-left', size: 'large', toggle: true, pressed: true, exclusive: 'powerpoint.view.presentation-views' },
  { id: 'powerpoint.view.presentation-views.outline-view', label: 'Outline View', icon: 'list-bar-tree-offset', toggle: true, exclusive: 'powerpoint.view.presentation-views' },
  { id: 'powerpoint.view.presentation-views.slide-sorter', label: 'Slide Sorter', icon: 'slide-grid', size: 'large', toggle: true, exclusive: 'powerpoint.view.presentation-views' },
  { id: 'powerpoint.view.presentation-views.notes-page', label: 'Notes Page', icon: 'notepad', size: 'large', toggle: true, exclusive: 'powerpoint.view.presentation-views' },
  { id: 'powerpoint.view.presentation-views.reading-view', label: 'Reading View', icon: 'book-open', size: 'large', toggle: true, exclusive: 'powerpoint.view.presentation-views' },
];

/**
 * PowerPoint's Master Views: Slide Master, Handout Master and Notes Master.
 *
 * **Three buttons**, each opening its view tab; see disagreement 3. Slide Master and Handout Master are large,
 * Notes Master small because `notepad-edit` has no 24 drawing.
 *
 * **No survivor**: each opens a tab.
 */
const powerpointViewMasterViews: readonly RibbonCommand[] = [
  { id: 'powerpoint.view.master-views.slide-master', label: 'Slide Master', icon: 'slide-text-edit', size: 'large' },
  { id: 'powerpoint.view.master-views.handout-master', label: 'Handout Master', icon: 'document-one-page-multiple', size: 'large' },
  { id: 'powerpoint.view.master-views.notes-master', label: 'Notes Master', icon: 'notepad-edit' },
];

/**
 * PowerPoint's `GroupViewShowHide`, labelled **Show**: Ruler, Gridlines, Guides, then Notes.
 *
 * **Ruler, Gridlines and Guides are toggles drawn as checkboxes**, bound by each host as `<mjx-checkbox>`;
 * Ruler starts ticked (disagreement 8). **Notes is a plain toggle drawing `panel-bottom`**, starting
 * unpressed, small because the glyph has no 24 drawing. The group's dialog launcher is `powerpointViewTab`'s.
 *
 * **No survivor**: three checkboxes, and Notes fails rule 2.
 */
const powerpointViewShow: readonly RibbonCommand[] = [
  { id: 'powerpoint.view.show.ruler', label: 'Ruler', toggle: true, pressed: true },
  { id: 'powerpoint.view.show.gridlines', label: 'Gridlines', toggle: true },
  { id: 'powerpoint.view.show.guides', label: 'Guides', toggle: true },
  { id: 'powerpoint.view.show.notes', label: 'Notes', icon: 'panel-bottom', toggle: true },
];

/**
 * PowerPoint's Zoom group: Zoom and Fit to Window, both large.
 *
 * **Zoom draws `zoom-in`** and opens the Zoom dialog. **Fit to Window draws `page-fit`** and fits the slide to
 * the window in one press.
 *
 * **Survivor: Fit to Window.** One press, no deck changed, a glyph no other command has.
 */
const powerpointViewZoom: readonly RibbonCommand[] = [
  { id: 'powerpoint.view.zoom.zoom', label: 'Zoom', icon: 'zoom-in', size: 'large' },
  { id: 'powerpoint.view.zoom.fit-to-window', label: 'Fit to Window', icon: 'page-fit', size: 'large', essential: true },
];

/**
 * PowerPoint's `GroupColorGrayscale`, labelled **Colour/Greyscale**: Colour, Greyscale and Black and White,
 * small, in a column.
 *
 * **Three toggles and one exclusive set, Colour pressed.** See disagreements 1 and 2.
 *
 * **No survivor**: an exclusive set, and two of its members open tabs.
 */
const powerpointViewColourGreyscale: readonly RibbonCommand[] = [
  { id: 'powerpoint.view.colour-greyscale.colour', label: 'Colour', icon: 'color', toggle: true, pressed: true, exclusive: 'powerpoint.view.colour-greyscale' },
  { id: 'powerpoint.view.colour-greyscale.greyscale', label: 'Greyscale', icon: 'color-off', toggle: true, exclusive: 'powerpoint.view.colour-greyscale' },
  { id: 'powerpoint.view.colour-greyscale.black-and-white', label: 'Black and White', icon: 'circle-half-fill', toggle: true, exclusive: 'powerpoint.view.colour-greyscale' },
];

/**
 * PowerPoint's Window group: New Window large, Arrange All, Cascade and Move Split in a column, then Switch
 * Windows large.
 *
 * **New Window** opens a second window on the same deck. **Arrange All** tiles every open PowerPoint window side
 * by side, and **Cascade** overlaps them down a diagonal. **Move Split** lets the arrow keys move the bars
 * between Normal view's panes. **Switch Windows is a dropdown** a host binds, listing every open window.
 *
 * **No survivor**: see this section's header.
 */
const powerpointViewWindow: readonly RibbonCommand[] = [
  { id: 'powerpoint.view.window.new-window', label: 'New Window', icon: 'window-new', size: 'large' },
  { id: 'powerpoint.view.window.arrange-all', label: 'Arrange All', icon: 'layout-column-two' },
  { id: 'powerpoint.view.window.cascade', label: 'Cascade', icon: 'stack' },
  { id: 'powerpoint.view.window.move-split', label: 'Move Split', icon: 'arrow-move' },
  { id: 'powerpoint.view.window.switch-windows', label: 'Switch Windows', icon: 'window-multiple', size: 'large' },
];

/**
 * PowerPoint's View Direction: Left-to-Right and Right-to-Left. `GUESS:` the whole group; see disagreement 4.
 *
 * **Two toggles and one exclusive set, Left-to-Right pressed**, small.
 *
 * **No survivor**: an exclusive set, which a second press does not take back.
 */
const powerpointViewViewDirection: readonly RibbonCommand[] = [
  { id: 'powerpoint.view.view-direction.left-to-right', label: 'Left-to-Right', icon: 'text-direction-horizontal-ltr', toggle: true, pressed: true, exclusive: 'powerpoint.view.view-direction' },
  { id: 'powerpoint.view.view-direction.right-to-left', label: 'Right-to-Left', icon: 'text-direction-horizontal-rtl', toggle: true, exclusive: 'powerpoint.view.view-direction' },
];

// ── Excel's View ─────────────────────────────────────────────────────────────
//
// **Excel's View tab**, the third View tab, one tab of one application again and to Word's and PowerPoint's
// pattern: all seven in-scope groups, twenty-eight commands. **Nothing is declared once with Word's or
// PowerPoint's.** Zoom, Window, Show and Night Mode carry the same census ids in two or three of them, and
// their faces differ: Excel's Zoom is Zoom, 100% and Zoom to Selection; its Window has Freeze Panes, Hide and
// Unhide; its Show has Formula Bar and Headings. What *is* shared is glyphs (Zoom, 100%, New Window, View
// Side by Side, Switch Windows, Switch Modes, Page Layout's printed page) and the Switch Windows list's
// shape in `stories/ribbons/view-menus.ts`.
//
// ## The shapes, decided by what Office's popup is
//
// **Toggles in one exclusive set**, `excel.view.workbook-views`: Normal (pressed), Page Break Preview and
// Page Layout, because Excel shows a sheet in exactly one of the three, which its status bar's three view
// buttons also say. **Plain toggles**: Split, View Side by Side, Synchronous Scrolling and Switch Modes.
// **Checkboxes a host binds**: Ruler, Gridlines, Formula Bar and Headings, all ticked. **One field a host
// binds**: the Sheet View dropdown, reading *Default*. **Two dropdowns a host binds**, Freeze Panes and
// Switch Windows, over menus in `stories/ribbons/view-menus.ts`. **Buttons**: Keep, Exit, New, Options,
// Custom Views, Zoom, 100%, Zoom to Selection, New Window, Arrange All, Hide, Unhide, Reset Window Position
// and Debug. **No gallery, no split button, no dialog launcher**: Office puts none on Excel's View tab.
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **The declaration puts Sheet View fifth; Microsoft 365 draws it first**, before Workbook Views. Office's
//    View tab reads Sheet View, Workbook Views, Show, Zoom, Window, Macros. The group order is the tab
//    module's decision, as it was for Word's Page Movement, so `excelViewTab` draws Sheet View, Workbook
//    Views, Show, Zoom, Window, Night Mode, Debug. **Night Mode and Debug stay last**, in the declaration's
//    order, because nothing this project can cite says where Office puts either. `GUESS:` both positions.
// 2. **Macros** (`GroupMacros`, four controls) is out of scope in the census and is not drawn.
// 3. **Sheet View is a Microsoft 365 group for a workbook saved to OneDrive or SharePoint**, and Office
//    greys all of it but New for any other workbook, and greys Keep and Exit while *Default* is showing.
//    Everything is drawn available, because `disabled` is loop 2's. The dropdown lists **Default** and every
//    sheet view the workbook has saved; the catalogue's workbook has saved none, so it lists Default alone.
//    `GUESS:` the dropdown's accessible name, *Sheet View*, which is the group's.
// 4. **Night Mode is Word's reading, repeated.** The census names `GroupNightMode` and counts one control.
//    It is drawn as Word's is, one large **Switch Modes** toggle, unpressed. `GUESS:` the whole group,
//    including that Excel draws it at all outside a dark Office theme.
// 5. **Debug is `GUESS:` in its entirety**, as Excel Review's Debug is: `GroupViewDebug` counts one control
//    and says nothing else. One small button carrying the group's own label, and no icon.
// 6. **Gridlines and Headings are Page Layout's View Gridlines and View Headings under another name**: one
//    sheet option each, on two faces. Both are drawn, ticked in both places, because the census declares
//    both groups; the repetition is recorded, not removed, as Excel Review's comment toggles are.
// 7. **Ruler is ticked and greyed in Office outside Page Layout view**, where it has no ruler to show. It is
//    drawn ticked and available.
// 8. **Office relabels Freeze Panes' first entry *Unfreeze Panes* while panes are frozen.** The catalogue's
//    sheet has none frozen, so the menu reads Freeze Panes, Freeze Top Row, Freeze First Column, each with
//    Office's one-line description. `GUESS:` the descriptions' wording, from memory of Microsoft 365.
// 9. **Office greys Unhide while no window is hidden, and Synchronous Scrolling and Reset Window Position
//    until View Side by Side is on.** All three are drawn available.
// 10. **The census's counts are larger than the faces**, and nothing is padded: Show is 8 and draws four;
//     Workbook Views is 4, Zoom is 3, Window is 10 and Sheet View is 5, and each draws exactly that many.
//
// ## Survivors: 100% and Zoom to Selection, and nothing else
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Zoom's 100% and Zoom to Selection survive.** Each sets the zoom in one press and changes no workbook,
//   the standard Word's zoom presets and PowerPoint's Fit to Window pass. `ratio-one-to-one` is *1:1*, actual
//   size, and Word's 100%; `zoom-fit` is a magnifier inside fit corners, the selection fitted to the window,
//   and no other command's glyph in this subset. Zoom stays in the popup, so the popup is never empty.
//   `GUESS:` rule 2 on both glyphs.
// - **Zoom** opens a dialog: rule 1.
// - **Sheet View**: the dropdown is a field, which the gate refuses. **Keep** saves a temporary view into the
//   workbook, **New** creates one and **Exit** leaves one, and none is taken back by a second press or an
//   undo: rule 1. **Options** opens a dialog.
// - **Workbook Views** are an exclusive set, which a second press does not take back; **Custom Views** opens
//   a dialog.
// - **Show**'s four are checkboxes a host binds, which the gate refuses.
// - **Window**: New Window opens a window, Arrange All opens the Arrange Windows dialog, Freeze Panes and
//   Switch Windows are menus, Unhide is a dialog, and **Hide** is taken back only through that dialog. View
//   Side by Side asks which workbook when more than two are open, and **Reset Window Position** is undone by
//   nothing. **Split passes rule 1 and fails rule 2**: a press splits the window into four panes at the
//   active cell and a second press removes them, but `split-horizontal` draws one cut across, so with no
//   label it says a different split from the one it makes. **Synchronous Scrolling passes rule 1 and fails
//   rule 2**: `arrow-bidirectional-up-down` is *scroll*, and says nothing of *together*.
// - **Night Mode** and **Debug** are each their group's only command, so a survivor would leave the collapsed
//   popup empty; Debug has no glyph besides.
//
// ## Sizes, and every glyph
//
// Word's rule: `large` where Office draws it large **and** Fluent draws its glyph at 24 **and** the label
// wraps inside `largeControlWidthUnits`. **Normal, Page Break Preview, Page Layout, Zoom, 100%, Zoom to
// Selection, New Window, Arrange All, Freeze Panes, Switch Windows and Switch Modes** are large. **Custom
// Views is small where Office draws it large**, because Fluent draws `window-bullet-list` at 20 alone. Sheet
// View's five, Split, Hide, Unhide, View Side by Side, Synchronous Scrolling, Reset Window Position and Debug
// are small, as Office draws them. `GUESS:` that *Page Break Preview*, three words, wraps to two lines
// without clipping; `Ribbons/Excel → View` names it as the first thing to look at.
//
// **Every command on the tab's face carries a glyph except the Sheet View dropdown, the four Show
// checkboxes and Debug.** The dropdown is a field and draws its value; the checkboxes draw a tick box; Debug
// has no described behaviour, so any glyph would claim one (`bug` would say *debugger*, which nothing in the
// census says the command opens). Every glyph below is `GUESS:`, judged from Fluent's drawings rather than
// from a build this project can cite:
//
// - **Keep draws `save`**, the disk: Office's tooltip says Keep *saves* the temporary view. It is File's Save
//   glyph, which matters only to a survivor, and Keep is not one. **Exit draws `arrow-exit`**, an arrow
//   leaving a frame, out of the view. **New draws `add`**, a plus, a new view. **Options draws `settings`**, the
//   cog every Options command in this subset draws.
// - **Normal draws `grid`**, cells in rows and columns: the sheet, with nothing laid over it. Not `table`,
//   which is Insert's Table. **Page Break Preview draws `document-page-break`**, Breaks' two page edges and the
//   gap between them, which is what the view paints over the sheet. **Page Layout draws `document-one-page`**,
//   Word's Print Layout, because it is the same idea: the sheet as printed pages.
// - **Custom Views draws `window-bullet-list`**, a window holding a list: the named views the dialog keeps.
// - **Zoom draws `zoom-in`**, PowerPoint's Zoom; **100% draws `ratio-one-to-one`**, Word's; **Zoom to Selection
//   draws `zoom-fit`**; see the survivors above.
// - **New Window and Switch Windows** draw Word's `window-new` and `window-multiple`. **View Side by Side**
//   draws Word's `column-double-compare`.
// - **Arrange All draws `layout-cell-four`**, four windows tiled, which is Excel's first arrangement in the
//   dialog it opens. Not PowerPoint's `layout-column-two`, which Reset Window Position draws here.
// - **Freeze Panes draws `table-freeze-column-and-row`**, a grid with its top row and first column held,
//   Fluent's own drawing of the command; its three menu entries draw `table-freeze-column-and-row`,
//   `table-freeze-row` and `table-freeze-column`.
// - **Split draws `split-horizontal`**, Word's Split, filled while the window is split.
// - **Hide draws `eye-off`** and **Unhide draws `eye`**, a window put out of sight and brought back. `eye` is
//   Word's Preview Results, another application's tab.
// - **Synchronous Scrolling draws `arrow-bidirectional-up-down`**, an arrow up and down: scrolling. Word's
//   Synchronous Scrolling carries none, because `arrow-sync` is AutoSave's; this glyph is not, and a
//   disagreement with Word's unit that this unit does not reopen.
// - **Reset Window Position draws `layout-column-two`**, two equal columns: the two windows sharing the
//   screen equally again, which is what the command does.
// - **Switch Modes draws `dark-theme`**, Word's.

/**
 * Excel's `GroupNamedSheetView`, labelled **Sheet View**: the view dropdown, then Keep, Exit, New and Options.
 *
 * **The dropdown is a field** each host binds as `<mjx-dropdown>` reading *Default*; see disagreement 3.
 * **Keep, Exit, New and Options are small buttons**: Keep saves the temporary view, Exit returns to Default,
 * New starts a temporary view, and Options opens Sheet View Options. See this section's header for the glyphs.
 *
 * **No survivor**: a field, three commands no undo takes back, and a dialog.
 */
const excelViewSheetView: readonly RibbonCommand[] = [
  { id: 'excel.view.sheet-view.sheet-view', label: 'Sheet View' },
  { id: 'excel.view.sheet-view.keep', label: 'Keep', icon: 'save' },
  { id: 'excel.view.sheet-view.exit', label: 'Exit', icon: 'arrow-exit' },
  { id: 'excel.view.sheet-view.new', label: 'New', icon: 'add' },
  { id: 'excel.view.sheet-view.options', label: 'Options', icon: 'settings' },
];

/**
 * Excel's Workbook Views: Normal, Page Break Preview and Page Layout large, then Custom Views.
 *
 * **Three toggles and one exclusive set, Normal pressed**, because a workbook opens in Normal and Excel shows
 * a sheet in exactly one view. **Custom Views is a button** opening the Custom Views dialog, small because
 * `window-bullet-list` has no 24 drawing.
 *
 * **No survivor**: an exclusive set, and a dialog.
 */
const excelViewWorkbookViews: readonly RibbonCommand[] = [
  { id: 'excel.view.workbook-views.normal', label: 'Normal', icon: 'grid', size: 'large', toggle: true, pressed: true, exclusive: 'excel.view.workbook-views' },
  { id: 'excel.view.workbook-views.page-break-preview', label: 'Page Break Preview', icon: 'document-page-break', size: 'large', toggle: true, exclusive: 'excel.view.workbook-views' },
  { id: 'excel.view.workbook-views.page-layout', label: 'Page Layout', icon: 'document-one-page', size: 'large', toggle: true, exclusive: 'excel.view.workbook-views' },
  { id: 'excel.view.workbook-views.custom-views', label: 'Custom Views', icon: 'window-bullet-list' },
];

/**
 * Excel's `GroupViewShowHide`, labelled **Show**: Ruler, Gridlines, Formula Bar and Headings.
 *
 * **Four toggles drawn as checkboxes**, bound by each host as `<mjx-checkbox>`, **all ticked**: a new
 * workbook shows gridlines, the formula bar and headings, and the assembled shell draws all three. Office
 * draws Ruler ticked and greyed outside Page Layout view; see disagreements 6 and 7.
 *
 * **No survivor**: four checkboxes.
 */
const excelViewShow: readonly RibbonCommand[] = [
  { id: 'excel.view.show.ruler', label: 'Ruler', toggle: true, pressed: true },
  { id: 'excel.view.show.gridlines', label: 'Gridlines', toggle: true, pressed: true },
  { id: 'excel.view.show.formula-bar', label: 'Formula Bar', toggle: true, pressed: true },
  { id: 'excel.view.show.headings', label: 'Headings', toggle: true, pressed: true },
];

/**
 * Excel's Zoom group: Zoom, 100% and Zoom to Selection, all large.
 *
 * **Zoom draws `zoom-in`** and opens the Zoom dialog. **100% draws `ratio-one-to-one`** and sets the zoom to
 * actual size. **Zoom to Selection draws `zoom-fit`** and zooms until the selected cells fill the window.
 *
 * **Survivors: 100% and Zoom to Selection.** See this section's header.
 */
const excelViewZoom: readonly RibbonCommand[] = [
  { id: 'excel.view.zoom.zoom', label: 'Zoom', icon: 'zoom-in', size: 'large' },
  { id: 'excel.view.zoom.one-hundred-percent', label: '100%', icon: 'ratio-one-to-one', size: 'large', essential: true },
  { id: 'excel.view.zoom.zoom-to-selection', label: 'Zoom to Selection', icon: 'zoom-fit', size: 'large', essential: true },
];

/**
 * Excel's Window group: New Window, Arrange All and Freeze Panes large; Split, Hide and Unhide in a column;
 * View Side by Side, Synchronous Scrolling and Reset Window Position in a column; then Switch Windows large.
 *
 * **New Window** opens a second window on the workbook. **Arrange All** opens Arrange Windows (Tiled,
 * Horizontal, Vertical, Cascade). **Freeze Panes is a dropdown** a host binds, over three entries. **Split is
 * a toggle**, drawn pressed while the window is split; Excel does not relabel it, as Word does. **Hide** hides
 * the window and **Unhide** opens a list of hidden windows. **View Side by Side and Synchronous Scrolling are
 * toggles**, both unpressed, and **Reset Window Position** shares the screen equally again. **Switch Windows
 * is a dropdown** a host binds, listing every open window. See disagreements 8 and 9.
 *
 * **No survivor**: see this section's header.
 */
const excelViewWindow: readonly RibbonCommand[] = [
  { id: 'excel.view.window.new-window', label: 'New Window', icon: 'window-new', size: 'large' },
  { id: 'excel.view.window.arrange-all', label: 'Arrange All', icon: 'layout-cell-four', size: 'large' },
  { id: 'excel.view.window.freeze-panes', label: 'Freeze Panes', icon: 'table-freeze-column-and-row', size: 'large' },
  { id: 'excel.view.window.split', label: 'Split', icon: 'split-horizontal', toggle: true },
  { id: 'excel.view.window.hide', label: 'Hide', icon: 'eye-off' },
  { id: 'excel.view.window.unhide', label: 'Unhide', icon: 'eye' },
  { id: 'excel.view.window.view-side-by-side', label: 'View Side by Side', icon: 'column-double-compare', toggle: true },
  { id: 'excel.view.window.synchronous-scrolling', label: 'Synchronous Scrolling', icon: 'arrow-bidirectional-up-down', toggle: true },
  { id: 'excel.view.window.reset-window-position', label: 'Reset Window Position', icon: 'layout-column-two' },
  { id: 'excel.view.window.switch-windows', label: 'Switch Windows', icon: 'window-multiple', size: 'large' },
];

/**
 * Excel's `GroupNightMode`: Switch Modes, Word's reading. `GUESS:` the whole group; see disagreement 4.
 *
 * **A large toggle drawing `dark-theme`**, unpressed.
 *
 * **No survivor**: the only command.
 */
const excelViewNightMode: readonly RibbonCommand[] = [
  { id: 'excel.view.night-mode.switch-modes', label: 'Switch Modes', icon: 'dark-theme', size: 'large', toggle: true },
];

/**
 * Excel's `GroupViewDebug`, labelled **Debug**. `GUESS:` the whole group; see disagreement 5.
 *
 * **No survivor**: no glyph, and the only command.
 */
const excelViewDebug: readonly RibbonCommand[] = [
  { id: 'excel.view.debug.debug', label: 'Debug' },
];

// ── the commands Slide Show shows ────────────────────────────────────────────
//
// The ribbon programme's unit after the three View tabs: **PowerPoint's Slide Show tab**, all four in-scope
// groups and fourteen commands, one tab of one application. It is PowerPoint's alone: neither Word nor Excel
// has a Slide Show tab, so nothing here is a candidate for sharing, and `stories/ribbons/slide-show-menus.ts`
// renders nothing for either.
//
// ## The shapes, decided by what Office's popup is
//
// The tab plays the deck and says how it is played. **Buttons**: From Beginning, From Current Slide,
// Rehearse with Coach, Set Up Slide Show and Rehearse Timings, each of which starts a show, a rehearsal or a
// dialog. **One toggle**, Hide Slide, drawn pressed while the current slide is hidden; it starts unpressed,
// because the catalogue's deck hides nothing. **Two dropdowns a host binds**, Present Online and Custom Slide
// Show, and **one split button a host binds**, Record, whose face records from the current slide and whose
// arrow chooses where to start and what to clear. All three open menus in
// `stories/ribbons/slide-show-menus.ts`. **One field a host binds**, Monitor, over `slideShowMonitors`.
// **Checkboxes a host binds**: Play Narrations, Use Timings, Show Media Controls and Use Presenter View, all
// ticked, as a new deck in Office has them. **No exclusive set, no split toggle, no gallery, no dialog
// launcher**: Office puts no launcher on this tab, and nothing on it holds one of several states.
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **Rehearse is drawn second, between Start Slide Show and Set Up.** The declaration puts it third, after
//    Set Up. Microsoft 365 draws Rehearse with Coach right after Custom Slide Show, so `powerpointSlideShowTab`
//    draws Start Slide Show, Rehearse, Set Up, Monitors. `GUESS:` that position, from memory of Microsoft 365.
// 2. **Rehearse's one command is Rehearse with Coach**, Speaker Coach's door. The census names
//    `GroupRehearse`, counts one control and names none. `GUESS:` the command and its label. It is not
//    Rehearse Timings, which Office keeps in Set Up.
// 3. **Captions & Subtitles is out of scope.** The census's row is `GroupLiveSubtitles`, ten controls,
//    `in_scope = 0` (Office's Always Use Subtitles and Subtitle Settings), so it has no declaration here and
//    is not drawn.
// 4. **Record is Microsoft 365's face**, a large split button labelled *Record*. PowerPoint 2016 labelled it
//    *Record Slide Show*. Its arrow holds From Current Slide…, From Beginning… and Office's *Clear* submenu,
//    flattened into a labelled section as every earlier unit flattened one. `GUESS:` the entries' wording.
// 5. **Present Online lists Office Presentation Service and Skype for Business**, PowerPoint 2016's two.
//    Office shows the second only where the Skype for Business client is installed, and Microsoft 365 has
//    been retiring both services. `GUESS:` that both are still what the dropdown lists.
// 6. **Custom Slide Show lists Custom Shows… alone.** Office lists every custom show the deck has saved above
//    it; the catalogue's deck has saved none, which is Excel's Sheet View argument.
// 7. **Monitor lists Automatic and Primary Monitor.** Office lists Automatic and then every attached display
//    by name, and a display name is this machine's data rather than Office's vocabulary, which is the printer
//    list's argument. `GUESS:` *Primary Monitor* as the one display.
// 8. **The census's counts are larger than the faces**, and nothing is padded: Start Slide Show is 9 and draws
//    four; Set Up is 16 and draws seven. Rehearse is 1 and Monitors is 2, and each draws exactly that many.
// 9. **Office greys Rehearse with Coach while offline, and Show Media Controls in a deck with no media.** Both
//    are drawn available, because `disabled` is loop 2's.
//
// ## Survivors: Hide Slide, and nothing else
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Hide Slide survives.** One press hides the current slide from the show, and a second press or an undo
//   takes it back, the standard Bold passes. `slide-hide` is a slide drawn in a dashed outline, the picture
//   Office itself uses for a hidden slide in the thumbnail rail, and no other command's glyph in this subset.
//   Set Up Slide Show, Rehearse Timings and Record stay in the popup, so it is never empty. `GUESS:` rule 2.
// - **From Beginning and From Current Slide** take over the whole screen until the show ends, which no undo
//   takes back: rule 1, the reason Reading View does not survive on View.
// - **Present Online and Custom Slide Show** are menus, **Record** is a split button, and **Set Up Slide Show**
//   opens a dialog: rule 1. **Rehearse Timings** starts a timed rehearsal that takes over the screen.
// - **Rehearse with Coach** opens Speaker Coach's rehearsal, and it is its group's only command, so a survivor
//   would leave the popup empty.
// - **Monitor** is a field and the four checkboxes are checkboxes a host binds, which the gate refuses.
//
// ## Sizes, and every glyph
//
// Word's View rule: `large` where Office draws it large **and** Fluent draws its glyph at 24. Every button, the
// toggle, both dropdowns and the split button are large, as Office draws them; the checkboxes and the field
// have no size to choose.
//
// **Every command on the tab's face carries a glyph except Monitor and the four checkboxes**: a field draws its
// value and a checkbox its tick box. Every glyph below is `GUESS:`, judged from Fluent's drawings rather than
// from a build this project can cite:
//
// - **From Beginning draws `slide-multiple-arrow-right`**, stacked slides with an arrow forward: the whole deck,
//   played through. **From Current Slide draws `slide-play`**, one slide with a play mark: this slide, played.
//   The two differ in exactly what the commands differ in. Not `previous`, which is Mailings' First Record.
// - **Present Online draws `presenter`**, File's Present Online, because it is the same command; a 24 drawing
//   is added, since it is large here.
// - **Custom Slide Show draws `slide-text-multiple`**, slides stacked, some of the deck chosen. Not
//   `slide-multiple`, which is File's Publish Slides.
// - **Rehearse with Coach draws `person-voice`**, a figure speaking, which is what Speaker Coach listens to.
// - **Set Up Slide Show draws `slide-settings`**, a slide with a cog: the show's settings.
// - **Hide Slide draws `slide-hide`**; see the survivors above.
// - **Rehearse Timings draws `timer`**, a stopwatch, which is the timer Office's rehearsal toolbar runs.
// - **Record draws `slide-record`**, a slide with the record mark. Not `record`, which is Insert's Screen
//   Recording and records the screen rather than the slides.

/**
 * PowerPoint's `GroupSlideShowStart`, labelled **Start Slide Show**: From Beginning, From Current Slide, Present
 * Online and Custom Slide Show, all large.
 *
 * **From Beginning** (F5) plays the deck from its first slide and **From Current Slide** (Shift+F5) from the one
 * selected. **Present Online and Custom Slide Show are dropdowns** a host binds; see disagreements 5 and 6.
 *
 * **No survivor**: two commands take over the screen, and two are menus.
 */
const powerpointSlideShowStart: readonly RibbonCommand[] = [
  { id: 'powerpoint.slide-show.start-slide-show.from-beginning', label: 'From Beginning', icon: 'slide-multiple-arrow-right', size: 'large' },
  { id: 'powerpoint.slide-show.start-slide-show.from-current-slide', label: 'From Current Slide', icon: 'slide-play', size: 'large' },
  { id: 'powerpoint.slide-show.start-slide-show.present-online', label: 'Present Online', icon: 'presenter', size: 'large' },
  { id: 'powerpoint.slide-show.start-slide-show.custom-slide-show', label: 'Custom Slide Show', icon: 'slide-text-multiple', size: 'large' },
];

/**
 * PowerPoint's `GroupRehearse`, labelled **Rehearse**: Rehearse with Coach, large. `GUESS:` the command and the
 * group's position; see disagreements 1 and 2.
 *
 * **No survivor**: it opens a rehearsal, and it is the only command.
 */
const powerpointSlideShowRehearse: readonly RibbonCommand[] = [
  { id: 'powerpoint.slide-show.rehearse.rehearse-with-coach', label: 'Rehearse with Coach', icon: 'person-voice', size: 'large' },
];

/**
 * PowerPoint's `GroupSlideShowSetup`, labelled **Set Up**: Set Up Slide Show, Hide Slide, Rehearse Timings and
 * Record large, then Play Narrations, Use Timings and Show Media Controls in a column.
 *
 * **Set Up Slide Show** opens its dialog. **Hide Slide is a toggle**, unpressed. **Rehearse Timings** starts a
 * rehearsal that records each slide's time. **Record is a split button** a host binds; see disagreement 4.
 * **The three checkboxes** are toggles a host binds as `<mjx-checkbox>`, all ticked.
 *
 * **Survivor: Hide Slide.** One press, one undo, a glyph no other command has.
 */
const powerpointSlideShowSetUp: readonly RibbonCommand[] = [
  { id: 'powerpoint.slide-show.set-up.set-up-slide-show', label: 'Set Up Slide Show', icon: 'slide-settings', size: 'large' },
  { id: 'powerpoint.slide-show.set-up.hide-slide', label: 'Hide Slide', icon: 'slide-hide', size: 'large', toggle: true, essential: true },
  { id: 'powerpoint.slide-show.set-up.rehearse-timings', label: 'Rehearse Timings', icon: 'timer', size: 'large' },
  { id: 'powerpoint.slide-show.set-up.record', label: 'Record', icon: 'slide-record', size: 'large' },
  { id: 'powerpoint.slide-show.set-up.play-narrations', label: 'Play Narrations', toggle: true, pressed: true },
  { id: 'powerpoint.slide-show.set-up.use-timings', label: 'Use Timings', toggle: true, pressed: true },
  { id: 'powerpoint.slide-show.set-up.show-media-controls', label: 'Show Media Controls', toggle: true, pressed: true },
];

/**
 * PowerPoint's `GroupMonitors`, labelled **Monitors**: the Monitor field over Automatic and Primary Monitor, then
 * Use Presenter View.
 *
 * **Monitor is a dropdown field** a host binds, reading *Automatic*; see disagreement 7. **Use Presenter View is
 * a toggle a host binds as `<mjx-checkbox>`**, ticked, as Office has it since PowerPoint 2013.
 *
 * **No survivor**: a field and a checkbox.
 */
const powerpointSlideShowMonitors: readonly RibbonCommand[] = [
  { id: 'powerpoint.slide-show.monitors.monitor', label: 'Monitor' },
  { id: 'powerpoint.slide-show.monitors.use-presenter-view', label: 'Use Presenter View', toggle: true, pressed: true },
];

// ── the commands Recording shows ─────────────────────────────────────────────
//
// The ribbon programme's unit after Slide Show: **PowerPoint's Recording tab**, all ten in-scope groups and
// fifteen commands, one tab of one application. It is PowerPoint's alone, so `stories/ribbons/recording-menus.ts`
// renders nothing for Word or Excel.
//
// ## ⚠ The census declares two generations of one tab, and that is the reading everything below rests on
//
// Five of the ten group ids end in `TabRecord`: `GroupRecordTabRecord`, `GroupEditTabRecord`,
// `GroupExportTabRecord`, `GroupPreviewTabRecord`, `GroupHelpTabRecord`. The other five do not: `GroupRecord`,
// `GroupContentRecording`, `GroupAutoPlayMediaRecording`, `GroupSaveRecording`, and the `GroupChunkCameoCamera`
// chunk that Insert also carries. `GUESS:` **the census dump merged two tabs under `TabRecording`**:
//
// - **The Recording tab** Microsoft 365 has shipped since 2017, turned on in Customize Ribbon: Record, Content,
//   Auto-Play Media, Save, with Camera (Cameo) added later.
// - **The Record tab** of the newer recording experience, whose window has a Record screen and an Export
//   screen: a Record button, Clear Recording and Reset to Cameo, Export, a preview of the recording, help.
//
// Office never draws both at once. A group the census declares in scope is a group this catalogue draws, so
// the tab draws all ten, which is Draw's answer to Draw's two generations, and **draws each command once
// wherever the two shapes are the same command**. The one exception is Record; see disagreement 3.
//
// ## The shapes, decided by what Office's popup is
//
// **Buttons**: From Beginning, From Current Slide, Screen Recording, Save as Show, Export to Video, Preview and
// Help. **Two split buttons a host binds**: Record, whose arrow chooses where to start, and Cameo, Insert's own.
// **Five dropdowns a host binds**: Screenshot, Video and Audio (Insert's own menus), Clear Recording, Reset to
// Cameo and Export. All seven menus are written in `stories/ribbons/recording-menus.ts`, and the four Insert
// already had are *called* from `stories/ribbons/insert-menus.ts` rather than copied. **No toggle, no exclusive
// set, no split toggle, no gallery, no field, no checkbox, no dialog launcher**: nothing on this tab holds a
// state, and Office puts no launcher here.
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **The order is `GUESS:` the declaration's**: Record, Recording, Content, Camera, Auto-play Media, Edit,
//    Save, Export, Preview, Help. No Office build draws the union. The declaration keeps the Recording tab's
//    own four in Office's order (Record, Content, Auto-Play Media, Save), which is what the brief lists.
// 2. **`GroupRecord` counts eight and draws two**: From Beginning and From Current Slide, as Microsoft 365
//    draws them and the brief lists them. Eight is exactly PowerPoint 2016's *Record Slide Show* split
//    (its face, From Current Slide…, From Beginning…, Clear, and Clear's four), which Slide Show's Record
//    already carries. Nothing is padded.
// 3. **`GroupRecordTabRecord` is labelled *Recording* by the census**, and Office's new tab calls its group
//    Record. Its command is **Record**, the large split button Microsoft 365 renamed from *Record Slide Show*:
//    the face records from the current slide, and the arrow holds From Current Slide… and From Beginning….
//    It is the same verb as the Record group's two buttons in a different shape, so it is **the one place
//    this tab draws a command twice**. Dropping either would leave a declared in-scope group empty. `GUESS:`
//    the command and its entries.
// 4. **Content is Screenshot and Screen Recording**, as the brief lists. The 2017 Recording tab also held
//    *Apps and Quizzes*, Office Mix's door, which Microsoft retired in 2018, and is not drawn. Screenshot's menu
//    is Insert's (Screen Clipping and no windows, for Insert's reason).
// 5. **Camera is Insert's Cameo, declared once.** `cameraCommands` above is called by both tabs, and the
//    menu is Insert's `cameoEntries`.
// 6. **`GroupAutoPlayMediaRecording` is labelled *Auto-play Media* by the census**, and Office writes
//    *Auto-Play Media*. The census's label is kept. Video and Audio open Insert's own menus. The 2017 tab also
//    put Screen Recording here; Microsoft 365 and the brief put it in Content, and it is drawn once, there.
//    The census counts five and the face is two.
// 7. **Edit is Clear Recording and Reset to Cameo**, each a dropdown over *on Current Slide* and *on All
//    Slides*, in Microsoft's support wording for the record window. Two menus of a face and two entries is
//    exactly the census's six. `GUESS:` that these are the group's commands. Office's record window also
//    has an *Edit* button that returns to the deck; the ribbon's Edit group is not that.
// 8. **Save is Save as Show and Export to Video**, as the brief lists; the census counts three.
// 9. **Export is one dropdown, Export, over Export Video and Customize Export**, the two things the record
//    window's Export screen offers. `GUESS:` all of it. It is not Save's Export to Video, which opens File's
//    Create a Video page, rather than the recorder's own export.
// 10. **Preview is one button, Preview**, which plays the current slide's recording without leaving the record
//     window, and **Help is one button, Help**. The census names each group, counts one control and names
//     none. `GUESS:` both labels.
//
// ## Survivors: none
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws. Nothing here does.
//
// - **From Beginning, From Current Slide and Record** take over the screen with the record window, which no
//   undo takes back: rule 1, and Slide Show's From Beginning.
// - **Screenshot, Video, Audio, Clear Recording, Reset to Cameo and Export** are menus, **Record and Cameo** are
//   split buttons, and **Save as Show** opens the Save As dialog: rule 1.
// - **Screen Recording** opens the recording dock, which is Insert's reason. **Export to Video** opens File's
//   Create a Video page.
// - **Preview and Help** are each their group's only command, so a survivor would leave the popup empty. Preview
//   also plays media, which is not a state an undo returns.
//
// ## Sizes, and every glyph
//
// Every command is `large`, as Office draws the whole tab, and every label wraps inside two lines (*From
// Current Slide* and *Screen Recording* are already drawn large on Slide Show and Insert).
//
// **Every command carries a glyph.** Eight are reused, because the command is the same one: Screenshot
// (`screenshot`), Screen Recording (`record`), Cameo (`camera`), Video (`video`), Audio (`speaker-2`), Help
// (`question-circle`), and the three record commands below. Every glyph judgement is `GUESS:`, from Fluent's
// drawings rather than from a build this project can cite:
//
// - **From Beginning draws `slide-multiple-arrow-right` and From Current Slide draws `slide-play`**, Slide Show's
//   own two. They start the same show from the same place, with the recorder on, and the tab says the
//   recorder is on. Fluent draws no stack of slides with a record mark, so the record mark is not what tells
//   the two apart.
// - **Record draws `slide-record`**, Slide Show's Record, because it is that command.
// - **Clear Recording draws `delete`**, the bin the record window's own Delete button draws. It gains a 24.
// - **Reset to Cameo draws `arrow-reset`**, Home's Reset, because the verb is the same: put the slide back to what
//   it was made with. It gains a 24.
// - **Save as Show draws `save-arrow-right`**, a save carried onward, a copy saved to open straight into the
//   show. Not `save-edit`, which is File's Save As.
// - **Export to Video draws `video-clip`**, the clip the export writes. Not `video`, which is Video two groups
//   along on this same tab and inserts one.
// - **Export draws `arrow-export`**, the arrow leaving a box, Excel's Export Workbook Data. It gains a 24.
// - **Preview draws `play-circle`**, a play mark. Not `slide-transition`, Transitions' Preview, which previews a
//   transition rather than a recording.

/**
 * PowerPoint's `GroupRecord`, labelled **Record**: From Beginning and From Current Slide, large. See
 * disagreement 2 on the census's eight.
 *
 * **No survivor**: both take over the screen.
 */
const powerpointRecordingRecord: readonly RibbonCommand[] = [
  { id: 'powerpoint.recording.record.from-beginning', label: 'From Beginning', icon: 'slide-multiple-arrow-right', size: 'large' },
  { id: 'powerpoint.recording.record.from-current-slide', label: 'From Current Slide', icon: 'slide-play', size: 'large' },
];

/**
 * PowerPoint's `GroupRecordTabRecord`, labelled **Recording** by the census: Record, a large split button a host
 * binds. `GUESS:` the command; see disagreement 3.
 *
 * **No survivor**: a split button, and the only command.
 */
const powerpointRecordingRecording: readonly RibbonCommand[] = [
  { id: 'powerpoint.recording.recording.record', label: 'Record', icon: 'slide-record', size: 'large' },
];

/**
 * PowerPoint's `GroupContentRecording`, labelled **Content**: Screenshot, a dropdown a host binds over Insert's
 * menu, and Screen Recording, both large. See disagreement 4.
 *
 * **No survivor**: a menu, and a command that opens the recording dock.
 */
const powerpointRecordingContent: readonly RibbonCommand[] = [
  { id: 'powerpoint.recording.content.screenshot', label: 'Screenshot', icon: 'screenshot', size: 'large' },
  { id: 'powerpoint.recording.content.screen-recording', label: 'Screen Recording', icon: 'record', size: 'large' },
];

/** PowerPoint's `GroupChunkCameoCamera` on Recording: Insert's Cameo. See `cameraCommands` and disagreement 5. */
const powerpointRecordingCamera: readonly RibbonCommand[] = cameraCommands('recording');

/**
 * PowerPoint's `GroupAutoPlayMediaRecording`, labelled **Auto-play Media**: Video and Audio, large dropdowns a
 * host binds over Insert's menus. See disagreement 6.
 *
 * **No survivor**: both are menus.
 */
const powerpointRecordingAutoPlayMedia: readonly RibbonCommand[] = [
  { id: 'powerpoint.recording.auto-play-media.video', label: 'Video', icon: 'video', size: 'large' },
  { id: 'powerpoint.recording.auto-play-media.audio', label: 'Audio', icon: 'speaker-2', size: 'large' },
];

/**
 * PowerPoint's `GroupEditTabRecord`, labelled **Edit**: Clear Recording and Reset to Cameo, large dropdowns a
 * host binds. `GUESS:` both; see disagreement 7.
 *
 * **No survivor**: both are menus.
 */
const powerpointRecordingEdit: readonly RibbonCommand[] = [
  { id: 'powerpoint.recording.edit.clear-recording', label: 'Clear Recording', icon: 'delete', size: 'large' },
  { id: 'powerpoint.recording.edit.reset-to-cameo', label: 'Reset to Cameo', icon: 'arrow-reset', size: 'large' },
];

/**
 * PowerPoint's `GroupSaveRecording`, labelled **Save**: Save as Show and Export to Video, large. See
 * disagreement 8.
 *
 * **No survivor**: a dialog, and File's Create a Video page.
 */
const powerpointRecordingSave: readonly RibbonCommand[] = [
  { id: 'powerpoint.recording.save.save-as-show', label: 'Save as Show', icon: 'save-arrow-right', size: 'large' },
  { id: 'powerpoint.recording.save.export-to-video', label: 'Export to Video', icon: 'video-clip', size: 'large' },
];

/**
 * PowerPoint's `GroupExportTabRecord`, labelled **Export**: Export, a large dropdown a host binds. `GUESS:`; see
 * disagreement 9.
 *
 * **No survivor**: a menu, and the only command.
 */
const powerpointRecordingExport: readonly RibbonCommand[] = [
  { id: 'powerpoint.recording.export.export', label: 'Export', icon: 'arrow-export', size: 'large' },
];

/**
 * PowerPoint's `GroupPreviewTabRecord`, labelled **Preview**: Preview, large. `GUESS:`; see disagreement 10.
 *
 * **No survivor**: the only command.
 */
const powerpointRecordingPreview: readonly RibbonCommand[] = [
  { id: 'powerpoint.recording.preview.preview', label: 'Preview', icon: 'play-circle', size: 'large' },
];

/**
 * PowerPoint's `GroupHelpTabRecord`, labelled **Help**: Help, large. `GUESS:`; see disagreement 10.
 *
 * **No survivor**: the only command.
 */
const powerpointRecordingHelp: readonly RibbonCommand[] = [
  { id: 'powerpoint.recording.help.help', label: 'Help', icon: 'question-circle', size: 'large' },
];

// ── the commands Outlining shows ─────────────────────────────────────────────
//
// The ribbon programme's unit after Recording: **Word's Outlining tab**, all three in-scope groups and
// twenty-one commands, one tab of one application, and **the first `appearance: 'view'` tab authored**. Word
// alone has it: PowerPoint's outline is a pane of Normal view, not a tab.
//
// ## A view tab renders in the catalogue alone
//
// Office shows Outlining only inside Outline view, and View's Outline button is the way in. `tabsFor` leaves
// every `appearance: 'view'` tab out of a strip unless `includeViewTabs` is asked for, and **only
// `Ribbons/Word` asks**. `Shell/Word` draws the default strip, so it never renders this tab and binds none of
// its commands. The four host bindings are in `stories/ribbons/word.stories.ts` alone, and a binding written
// in the shell would be a binding to nothing.
//
// ## The shapes, decided by what Office's popup is
//
// **Two fields a host binds**: Outline Level (Level 1 to Level 9, then Body Text, reading *Body Text*) and Show
// Level (Level 1 to Level 9, then All Levels, reading *All Levels*), both `<mjx-dropdown>` over lists written
// once in `stories/ribbons/outlining-menus.ts`. **Two checkboxes a host binds**: Show Text Formatting (ticked)
// and Show First Line Only. **Three toggles**: Show Document (pressed), Collapse Subdocuments and Lock
// Document. **Buttons**: the four level arrows, Move Up, Move Down, Expand, Collapse, Create, Insert, Unlink,
// Merge, Split and Close Outline View. **No menu, no gallery, no split button, no exclusive set, no dialog
// launcher**: nothing on Word's Outlining tab opens a popup but its two fields' own lists.
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **The first group's label is the census's, *Outlining Tools*; Office writes *Outline Tools*.** A label is
//    not an argument `censusGroup` accepts, so the census's is drawn, as Design's *Style Set* is.
// 2. **The Outline Level field is third, between Promote and Demote, where Office draws it**: the top row reads
//    Promote to Heading 1, Promote, the field, Demote, Demote to Body Text. The brief lists the field first.
//    `GUESS:` Office's order, from memory of Word 2010 to Microsoft 365, which all draw that row alike.
// 3. **Outline Level lists Level 1 to Level 9 and then Body Text; Word's Paragraph dialog lists Body Text
//    first.** The ribbon's order is kept, and the field reads *Body Text* because a new document's paragraph
//    is Normal text. `GUESS:` the list's order. Show Level lists Level 1 to Level 9 and then All Levels, and
//    reads *All Levels*, Outline view's start.
// 4. **Show Text Formatting starts ticked** and Show First Line Only unticked, as a new Outline view does.
// 5. **Office shows Master Document as Show Document and Collapse Subdocuments alone** until Show Document is
//    pressed, and only then draws Create, Insert, Unlink, Merge, Split and Lock Document. The catalogue draws
//    all eight, so **Show Document starts pressed**: the face shows what the pressed state shows. Nothing
//    hides the six when it is released, because that is command dispatch, which is loop 2's.
// 6. **Collapse Subdocuments is a toggle, as the brief lists it**, starting unpressed. `GUESS:` Office may
//    relabel it *Expand Subdocuments* while collapsed rather than drawing it pressed, which is why Word View's
//    Split is a plain button; the brief's shape is kept and the doubt is recorded. **Lock Document** is a
//    toggle Office draws pressed while the subdocument is locked.
// 7. **Office greys** Collapse Subdocuments, Unlink, Merge, Split and Lock Document in a document with no
//    subdocuments, which the catalogue's is. All are drawn available, because `disabled` is loop 2's.
// 8. **The census's counts equal the faces**, and nothing is padded: Outlining Tools is 12, Master Document 8,
//    Close 1.
//
// ## Survivors: Promote and Demote, and nothing else
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Promote and Demote survive.** Each moves the selected paragraph one level out or in with one press
//   (Alt+Shift+Left and Alt+Shift+Right), and one undo takes it back. `arrow-left` and `arrow-right` are no
//   other command's glyph in this subset, and in an outline an arrow out and an arrow in are the verbs.
//   Two, under the ceiling, and ten commands stay in the popup. `GUESS:` rule 2 on both glyphs, since a bare
//   left arrow is *back* in a browser.
// - **Promote to Heading 1 and Demote to Body Text pass rule 1 and fail rule 2**: `arrow-previous` and
//   `arrow-next` are an arrow to a stop, and unlabelled they read as *to the start* and *to the end*, the
//   neighbours of Mailings' First Record and Last Record.
// - **Move Up and Move Down pass rule 1 and fail rule 2**: `arrow-down` is already Excel's Fill. Keeping only
//   Move Up would split a pair.
// - **Expand and Collapse** change what the outline shows, not the document, and pass rule 1 as View's zoom
//   presets do; their plus and minus in a box are Excel's Show Detail and Hide Detail, the same meaning. They
//   are not kept because two more would reach four, over the ceiling, and one would split a pair. `GUESS:`
//   choosing the level verbs over them.
// - **Outline Level and Show Level** are fields, and **Show Text Formatting and Show First Line Only** are
//   checkboxes, which the gate refuses.
// - **Master Document**: Show Document changes the group's own face, which a survivor row cannot show.
//   Collapse Subdocuments asks to save the master document first. Insert opens a file picker. Create, Unlink,
//   Merge and Split restructure which files the document is saved as. Lock Document passes rule 1 and fails
//   rule 2: an unlabelled padlock reads as *protect*, which is Review's Restrict Editing and Info's Protect Document.
// - **Close Outline View** leaves the view and takes the tab with it, and it is its group's only command.
//
// ## Sizes, and every glyph
//
// **Size follows Office's shape.** The four level arrows, Move Up, Move Down, Expand and Collapse are
// `icon`, as Office draws them: icon-only, two rows beside the field. Show Document and Close Outline View
// are `large`. **Collapse Subdocuments is small where Office draws it large**, because *Subdocuments* is one
// twelve-letter word that cannot wrap inside `largeControlWidthUnits`, which is Review's Check Accessibility.
// Create, Insert, Unlink, Merge, Split and Lock Document are small, in two columns, as Office draws them.
// `GUESS:` that *Close Outline View* wraps to two lines without an ellipsis; `Ribbons/Word → Outlining` names
// it as a thing to look at.
//
// **Every command on the tab's face carries a glyph except the two fields and the two checkboxes**: a field
// draws its value and a checkbox its tick box. Every glyph below is `GUESS:`, judged from Fluent's drawings
// rather than from a build this project can cite:
//
// - **Promote draws `arrow-left` and Demote `arrow-right`**: out one level, in one level, the way the text
//   moves. **Promote to Heading 1 draws `arrow-previous` and Demote to Body Text `arrow-next`**: the same
//   arrows stopped at a wall, all the way out and all the way in. Office draws a double arrow; Fluent's
//   `chevron-double-left` is a panel's collapse chevron, which would break the family of four.
// - **Move Up draws `arrow-up` and Move Down `arrow-down`**, PowerPoint's Move Earlier and Move Later: the
//   paragraph moves up or down the list, the same verb.
// - **Expand draws `add-square` and Collapse `subtract-square`**, Excel's Show Detail and Hide Detail: the plus
//   and minus in a box an outline draws in its margin, the same verb.
// - **Show Document draws `document-multiple`**, pages stacked: a master document and its subdocuments. It is
//   filled while pressed.
// - **Collapse Subdocuments draws `arrow-collapse-all`**, lines drawn in towards each other, every subdocument
//   folded to its link. Filled while pressed.
// - **Create draws `document-add`**, a new subdocument. **Insert draws `document-arrow-left`**, a document with
//   an arrow, an existing file brought in. **Unlink draws `link-dismiss`**, a link struck off: the subdocument's
//   text is copied in and its file let go. **Merge draws `merge`**, two paths joining. **Split draws
//   `arrow-split`**, one path dividing; not View's `split-horizontal`, which splits a window.
// - **Lock Document draws `lock-closed`**, a padlock, filled while locked. Not `document-lock`, which is
//   Protect Document and Restrict Editing.
// - **Close Outline View draws `dismiss-square`**, a cross in a square, Office's own picture of the command.
//   Not the plain `dismiss`, which closes a panel, a chip or a dialog.

/**
 * Word's `GroupOutliningTools`, labelled **Outlining Tools** (Office: Outline Tools): the four level arrows
 * round the Outline Level field, Move Up, Move Down, Expand and Collapse, then the Show Level field and two
 * checkboxes. See disagreements 1 to 4.
 *
 * **The two fields are bound by each host as `<mjx-dropdown>`**, over `stories/ribbons/outlining-menus.ts`.
 * **Show Text Formatting and Show First Line Only are toggles drawn as checkboxes**, bound as
 * `<mjx-checkbox>`, the first ticked. Everything else is an icon-only button.
 *
 * **Survivors: Promote and Demote.** See this section's header.
 */
const wordOutliningOutlineTools: readonly RibbonCommand[] = [
  { id: 'word.outlining.outlining-tools.promote-to-heading-1', label: 'Promote to Heading 1', icon: 'arrow-previous', size: 'icon' },
  { id: 'word.outlining.outlining-tools.promote', label: 'Promote', icon: 'arrow-left', size: 'icon', essential: true },
  { id: 'word.outlining.outlining-tools.outline-level', label: 'Outline Level' },
  { id: 'word.outlining.outlining-tools.demote', label: 'Demote', icon: 'arrow-right', size: 'icon', essential: true },
  { id: 'word.outlining.outlining-tools.demote-to-body-text', label: 'Demote to Body Text', icon: 'arrow-next', size: 'icon' },
  { id: 'word.outlining.outlining-tools.move-up', label: 'Move Up', icon: 'arrow-up', size: 'icon' },
  { id: 'word.outlining.outlining-tools.move-down', label: 'Move Down', icon: 'arrow-down', size: 'icon' },
  { id: 'word.outlining.outlining-tools.expand', label: 'Expand', icon: 'add-square', size: 'icon' },
  { id: 'word.outlining.outlining-tools.collapse', label: 'Collapse', icon: 'subtract-square', size: 'icon' },
  { id: 'word.outlining.outlining-tools.show-level', label: 'Show Level' },
  { id: 'word.outlining.outlining-tools.show-text-formatting', label: 'Show Text Formatting', toggle: true, pressed: true },
  { id: 'word.outlining.outlining-tools.show-first-line-only', label: 'Show First Line Only', toggle: true },
];

/**
 * Word's Master Document group: Show Document large, Collapse Subdocuments, then Create, Insert and Unlink
 * in a column and Merge, Split and Lock Document in another.
 *
 * **Show Document is a large toggle, pressed**, because the catalogue draws the six commands Office shows only
 * while it is; see disagreement 5. **Collapse Subdocuments and Lock Document are small toggles**, unpressed;
 * see disagreement 6. **Create, Insert, Unlink, Merge and Split are small buttons**: Create makes the selected
 * heading a subdocument, Insert opens a file picker for an existing one, Unlink copies a subdocument's text in
 * and lets its file go, Merge joins the selected subdocuments, and Split divides one at the selection.
 *
 * **No survivor**: see this section's header.
 */
const wordOutliningMasterDocument: readonly RibbonCommand[] = [
  { id: 'word.outlining.master-document.show-document', label: 'Show Document', icon: 'document-multiple', size: 'large', toggle: true, pressed: true },
  { id: 'word.outlining.master-document.collapse-subdocuments', label: 'Collapse Subdocuments', icon: 'arrow-collapse-all', toggle: true },
  { id: 'word.outlining.master-document.create', label: 'Create', icon: 'document-add' },
  { id: 'word.outlining.master-document.insert', label: 'Insert', icon: 'document-arrow-left' },
  { id: 'word.outlining.master-document.unlink', label: 'Unlink', icon: 'link-dismiss' },
  { id: 'word.outlining.master-document.merge', label: 'Merge', icon: 'merge' },
  { id: 'word.outlining.master-document.split', label: 'Split', icon: 'arrow-split' },
  { id: 'word.outlining.master-document.lock-document', label: 'Lock Document', icon: 'lock-closed', toggle: true },
];

/**
 * Word's `GroupOutliningClose`, labelled **Close**: Close Outline View, large, which returns to the view the
 * document was in and takes the tab away.
 *
 * **No survivor**: nothing a press can take back, and the only command.
 */
const wordOutliningClose: readonly RibbonCommand[] = [
  { id: 'word.outlining.close.close-outline-view', label: 'Close Outline View', icon: 'dismiss-square', size: 'large' },
];

// ── the commands Print Preview shows ─────────────────────────────────────────
//
// The ribbon programme's unit after Outlining: **Word's Print Preview tab**, all four in-scope groups and
// seventeen commands, one tab of one application, and the second `appearance: 'view'` tab authored. It is
// Word's classic Print Preview (Word 2007 and 2010, and Microsoft 365's *Print Preview Edit Mode*): the
// document drawn as it will print, with the page setup and the zoom a person needs to judge it.
//
// ## A view tab renders in the catalogue alone
//
// Office shows the tab only inside Print Preview. `tabsFor` leaves every `appearance: 'view'` tab out of a
// strip unless `includeViewTabs` is asked for, and **only `Ribbons/Word` asks**, so the five host bindings
// and the three menus are rendered in `stories/ribbons/word.stories.ts` alone. `Shell/Word` never draws the
// tab. **The surface gate knew nothing of that until this unit**: it required every declared menu to be
// opened by both hosts, which for a view tab's menu would have demanded a binding to nothing in the shell.
// `tests/ribbons.test.ts` now requires a view tab's menu of the host that draws view tabs alone, refuses the
// shell opening one, and holds the claim about which host draws them to the hosts' own source.
//
// ## The shapes, decided by what Office's popup is
//
// **Three dropdowns a host binds**, Margins, Orientation and Size, each opening **Layout's own list**
// (`marginEntries('word')`, `orientationEntries()`, `sizeEntries()`, exported from
// `stories/ribbons/design-layout-menus.ts` for this) through `stories/ribbons/print-preview-menus.ts`, which
// declares the menu under this tab's command ids. **Two checkboxes a host binds**: Show Ruler and Magnifier
// (ticked). **Buttons**: Print, Options, Zoom, 100%, One Page, Two Pages, Page Width, Shrink One Page, Next
// Page, Previous Page and Close Print Preview. **One dialog launcher**, on Page Setup, which opens the Page
// Setup dialog as Layout's does. **No gallery, no split button, no exclusive set, no toggle button.**
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **The census counts more than Office draws, and nothing is padded.** Print is 2 and draws 2. Page Setup
//    is 6 and draws 3 and a launcher: Margins, Orientation, Size. Zoom is 6 and draws 5. Preview is 6 and draws
//    6. The census's extra controls are not named by it, and a command made up to reach a count would be a
//    worse lie than a short group.
// 2. **Zoom is `GroupZoom`, the same census id as View's Zoom**, because Office reuses the group. The group
//    is found by id inside this tab's entry, so the two never meet, and the command ids differ by their tab
//    segment (`word.print-preview.zoom.*` against `word.view.zoom.*`).
// 3. **Magnifier is a checkbox, where the brief lists it as a toggle.** Office draws Show Ruler and Magnifier
//    as two ticks stacked above Shrink One Page, and Magnifier starts ticked: the pointer is a magnifier that
//    switches the page between 100% and the whole page, and clearing the tick is how a person edits in the
//    preview. It is still a `toggle` in this census, as every checkbox is; what differs is the face. The
//    shape also settles a glyph clash: a magnifier toggle beside Zoom's magnifier would be two identical
//    pictures for two different commands. `GUESS:` the shape and the start, from memory of Word 2007 and 2010.
// 4. **Two Pages, not Multiple Pages.** The classic tab lays exactly two pages side by side; View's Microsoft
//    365 Zoom group renamed its neighbour Multiple Pages. The brief and Office agree on Two Pages.
// 5. **Zoom draws `zoom-in` here and nothing on Word's View.** Word's View refused `zoom-in` because it reads
//    as Zoom In; PowerPoint's and Excel's View units then drew it for the same Zoom dialog, because no Zoom In
//    command exists in the subset. This tab follows the later two, so every face command with an honest glyph
//    carries one, and records the disagreement with Word's View rather than re-deciding it.
// 6. **Show Ruler starts unticked**, following Word's View, where Ruler starts unticked; Office carries the
//    same setting into the preview.
// 7. **The Page Setup menus are Layout's, completed by this unit, and Layout's tab changes with them**, as
//    intended. Margins: Normal, Narrow, Moderate, Wide, Mirrored and Office 2003 Default, then *Custom
//    Margins…*, with *Last Custom Setting* leading the list only once custom margins have been set (a new
//    document, which the catalogue draws, has none). Orientation: Portrait and Landscape. Size: Letter,
//    Legal, Executive, A3, A4 (checked), A5, B4 (JIS), B5 (JIS), Tabloid, Statement, Envelope #10, Envelope
//    DL, Envelope C5, Envelope B5 and Envelope Monarch, then *More Paper Sizes…*. ⚠ Office's real Size list
//    comes from the selected printer's driver; this is Word's standard list, standing in for no printer.
//    `GUESS:` the Size order.
// 8. **Office greys Next Page and Previous Page** at the last and the first page, and Shrink One Page when
//    the document cannot lose a page. All are drawn available, because `disabled` is loop 2's.
//
// ## Survivors: Next Page and Previous Page, 100%, One Page and Page Width
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Print**: none. Print opens the Print dialog and Options opens Word Options; both fail rule 1.
// - **Page Setup**: none. Margins, Orientation and Size each open a menu.
// - **Zoom: 100%, One Page and Page Width survive**, for View's reason, word for word: each sets the zoom in
//   one press, the zoom before it is one press away, and a zoom changes no document. Zoom opens a dialog.
//   **Two Pages passes rule 1 and fails rule 2**: it carries no glyph (see *every glyph* below), and a fourth
//   would pass the ceiling of three.
// - **Preview: Next Page and Previous Page survive.** Each moves the preview one page with one press and the
//   other takes it back: the Mailings record-navigator standard. `document-arrow-down` and
//   `document-arrow-up` are a page and the way it moves, and no other command's glyph in this subset. Two,
//   under the ceiling, and four commands stay in the popup. `GUESS:` rule 2 on both, since a page with an
//   arrow down can read as *download*. **Shrink One Page passes rule 1** (one undo restores the font sizes)
//   **and fails rule 2**: arrows squeezed together read as *minimise*. **Show Ruler and Magnifier** are
//   checkboxes, which the gate refuses. **Close Print Preview** leaves the view and takes the tab with it,
//   as Close Outline View does.
//
// ## Sizes, and every glyph
//
// **Size follows Office's shape**: Print, Options, Margins, Orientation, Zoom, 100% and Close Print Preview
// are `large`; One Page, Two Pages, Page Width, Shrink One Page, Next Page and Previous Page are small, in the
// columns Office stacks them in. **Size is small where Office draws it large**, because it carries no glyph.
//
// Every glyph below is `GUESS:`, judged from Fluent's drawings rather than from a build this project can cite:
//
// - **Print draws `print`**, File's Print. **Options draws `settings`**, a cog: it opens Word Options. It is
//   large here, so `settings` gains a 24.
// - **Margins draws `document-margins` and Orientation `orientation`**, Layout's own, for the same commands.
// - **Zoom draws `zoom-in`**, PowerPoint's and Excel's Zoom; see disagreement 5. **100% draws
//   `ratio-one-to-one`, One Page `document-fit` and Page Width `auto-fit-width`**, View's three.
// - **Shrink One Page draws `arrow-minimize-vertical`**, two arrows pressed together from above and below:
//   the document squeezed until its last page is gone.
// - **Next Page draws `document-arrow-down` and Previous Page `document-arrow-up`**: a page, and the direction
//   Word's pages run. Not `caret-down`, which reads as a dropdown's arrow, nor `chevron-double-down`, the
//   gallery's More.
// - **Close Print Preview draws `dismiss-square`**, Close Outline View's cross in a square: the same verb,
//   leaving a view.
//
// **Three commands carry no glyph, and say why.** **Size**: Fluent draws no page size, which is Layout's
// reason (`resize` says *resize this object*). **Two Pages**: every two-page picture Fluent draws is already
// another command in this subset — `book-open` is Read Mode, `layout-column-two` is Arrange All,
// `text-column-two` is Columns and `document-one-page-multiple` is Handout Master, which Word's View refused
// for Multiple Pages. **Show Ruler and Magnifier** are checkboxes, which draw their tick box.

/**
 * Word's `GroupPrintPreviewPrint`, labelled **Print**: Print and Options, both large.
 *
 * **Print** opens the Print dialog; **Options** opens Word Options. `GUESS:` that Options opens it on its
 * Display page, where the printing options are.
 *
 * **No survivor**: both open a dialog.
 */
const wordPrintPreviewPrint: readonly RibbonCommand[] = [
  { id: 'word.print-preview.print.print', label: 'Print', icon: 'print', size: 'large' },
  { id: 'word.print-preview.print.options', label: 'Options', icon: 'settings', size: 'large' },
];

/**
 * Word's `GroupPrintPreviewPageSetup`, labelled **Page Setup**: Margins, Orientation and Size, and the Page
 * Setup dialog launcher. See disagreements 1 and 7.
 *
 * **All three are dropdowns a host binds**, over Layout's own lists through
 * `stories/ribbons/print-preview-menus.ts`. Size is small, having no glyph.
 *
 * **No survivor**: all three open a menu.
 */
const wordPrintPreviewPageSetup: readonly RibbonCommand[] = [
  { id: 'word.print-preview.page-setup.margins', label: 'Margins', icon: 'document-margins', size: 'large' },
  { id: 'word.print-preview.page-setup.orientation', label: 'Orientation', icon: 'orientation', size: 'large' },
  { id: 'word.print-preview.page-setup.size', label: 'Size' },
];

/**
 * Word's `GroupZoom` on Print Preview, labelled **Zoom**: Zoom and 100% large, then One Page, Two Pages and
 * Page Width in a column. See disagreements 2, 4 and 5.
 *
 * **Zoom** opens the Zoom dialog. **100%**, **One Page** and **Page Width** are View's, glyphs included.
 * **Two Pages** lays two whole pages side by side, and carries no glyph.
 *
 * **Survivors: 100%, One Page and Page Width.** See this section's header.
 */
const wordPrintPreviewZoom: readonly RibbonCommand[] = [
  { id: 'word.print-preview.zoom.zoom', label: 'Zoom', icon: 'zoom-in', size: 'large' },
  { id: 'word.print-preview.zoom.one-hundred-percent', label: '100%', icon: 'ratio-one-to-one', size: 'large', essential: true },
  { id: 'word.print-preview.zoom.one-page', label: 'One Page', icon: 'document-fit', essential: true },
  { id: 'word.print-preview.zoom.two-pages', label: 'Two Pages' },
  { id: 'word.print-preview.zoom.page-width', label: 'Page Width', icon: 'auto-fit-width', essential: true },
];

/**
 * Word's `GroupPrintPreviewPreview`, labelled **Preview**: Show Ruler, Magnifier and Shrink One Page in a
 * column, Next Page and Previous Page in another, then Close Print Preview large. See disagreements 3, 6
 * and 8.
 *
 * **Show Ruler and Magnifier are toggles drawn as checkboxes**, bound as `<mjx-checkbox>`, Magnifier ticked.
 * **Shrink One Page** shrinks the text until the document is one page shorter. **Next Page** and **Previous
 * Page** move the preview a page. **Close Print Preview** returns to the view the document was in.
 *
 * **Survivors: Next Page and Previous Page.** See this section's header.
 */
const wordPrintPreviewPreview: readonly RibbonCommand[] = [
  { id: 'word.print-preview.preview.show-ruler', label: 'Show Ruler', toggle: true },
  { id: 'word.print-preview.preview.magnifier', label: 'Magnifier', toggle: true, pressed: true },
  { id: 'word.print-preview.preview.shrink-one-page', label: 'Shrink One Page', icon: 'arrow-minimize-vertical' },
  { id: 'word.print-preview.preview.next-page', label: 'Next Page', icon: 'document-arrow-down', essential: true },
  { id: 'word.print-preview.preview.previous-page', label: 'Previous Page', icon: 'document-arrow-up', essential: true },
  { id: 'word.print-preview.preview.close-print-preview', label: 'Close Print Preview', icon: 'dismiss-square', size: 'large' },
];

// ## PowerPoint's Print Preview
//
// The unit after Excel's Background Removal, one tab of one application: **PowerPoint's Print Preview tab**,
// all four in-scope groups and ten commands, and PowerPoint's second view tab authored. It is PowerPoint 2007's
// Print Preview, the last PowerPoint with the tab (2010 folded it into File → Print): the deck as it will
// print, in whichever of PowerPoint's printout shapes is chosen. **Nothing is declared once with Word's.** The
// four group ids are the same, and every face differs: PowerPoint has no Margins, Size, Show Ruler, Magnifier,
// One Page, Two Pages, Page Width or Shrink One Page, and Word has no Print What or Colour/Greyscale. A
// function of the application would be two declarations behind one name, which is View's reason too.
//
// ## The shapes, decided by what Office's popup is
//
// **Two fields a host binds**, Print What and Colour/Greyscale, over option lists in
// `stories/ribbons/print-preview-menus.ts` (`powerpointPrintWhat`, `powerpointPrintColourModes`). **Two
// dropdowns a host binds**: Options, over PowerPoint's own printing options, and Orientation, over **Layout's
// own list** (`orientationEntries()`), declared there under this tab's ids. **Buttons**: Print, Zoom, Fit to
// Window, Next Page, Previous Page and Close Print Preview. **No dialog launcher, no gallery, no split button,
// no toggle, no exclusive set, no checkbox.**
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **Page Setup counts 5 and draws 3**: Print What, Orientation and Colour/Greyscale. `GUESS:` the reading
//    that the census counts each field's caption (*Print What:*, *Color/Grayscale:*) as a control of its own,
//    so two fields are four and Orientation is the fifth. Nothing is padded. Print (2), Zoom (2) and Preview
//    (3) draw exactly their counts.
// 2. **Colour/Greyscale is in Page Setup, as the brief places it; Office 2007 is remembered with it inside
//    Options' menu** as a *Color/Grayscale* submenu. The brief and the census's count of 5 both put a second
//    field in Page Setup, so it is drawn there, and **Options' menu leaves the submenu out** rather than carry
//    one setting in two places. `GUESS:` both Office positions, from memory of PowerPoint 2007.
// 3. **The census's spelling wins over Office's.** Office writes *Color/Grayscale*, *Color* and *Grayscale*;
//    PowerPoint's View group is labelled *Colour/Greyscale* in the census and its view tab *Greyscale*. So the
//    field is **Colour/Greyscale** over **Colour**, **Greyscale** and **Pure Black and White**, and the brief's
//    American spelling is not followed.
// 4. **Options is a dropdown, where Word's Options is a button.** Word's opens Word Options; PowerPoint 2007's
//    opens a menu of printing options: Header and Footer…, Scale to Fit Paper, Frame Slides, Print Comments
//    and Ink Markup, Print Order (Horizontal, Vertical) and Print Hidden Slides. Office's *Print Order* submenu
//    is flattened into a labelled section, as Review's are. `GUESS:` the entries, their order and every
//    starting tick, from memory.
// 5. **Orientation opens Layout's list unchanged**: Portrait (checked) and Landscape. It sets the orientation
//    of handouts, notes pages and the outline, which a new deck prints in portrait; a slide prints in the
//    deck's own orientation, and **Office greys the button while Print What is Slides**. It is drawn available,
//    because `disabled` is loop 2's. `GUESS:` that PowerPoint's labels are Word's.
// 6. **Zoom is `GroupZoom`, View's census id**, as on Word's Print Preview, and holds View's two commands
//    under this tab's ids, glyphs included. The census's priority here is `secondary` where Word's Print
//    Preview Zoom is `standard`; the census wins.
// 7. **Office greys Next Page and Previous Page** at the last and the first page, and Print Hidden Slides with
//    no slide hidden. All are drawn available, because `disabled` is loop 2's.
// 8. **No dialog launcher on Page Setup**, where Word's has one. `GUESS:` that PowerPoint 2007's tab drew none;
//    Design's Slide Size is where a deck's page setup lives.
//
// ## Survivors: Fit to Window, Next Page and Previous Page
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Print**: none. Print opens the Print dialog (rule 1) and Options opens a menu (rule 1).
// - **Page Setup**: none. Print What and Colour/Greyscale are fields a host binds, which the gate refuses, and
//   Orientation opens a menu.
// - **Zoom: Fit to Window survives**, for View's reason word for word: one press fits the page to the window,
//   no deck changes, and `page-fit` is no other command's glyph on the tab. Zoom opens a dialog and stays in
//   the popup, so the popup is never empty. `GUESS:` rule 2.
// - **Preview: Next Page and Previous Page survive**, for Word's Print Preview's reason: each moves the
//   preview one page and the other takes it back, two under the ceiling, and Close Print Preview stays in the
//   popup. `GUESS:` that the glyphs read as pages rather than *download* and *upload*. **Close Print Preview**
//   leaves the view and takes the tab with it.
//
// ## Sizes, and every glyph
//
// **Size follows Office's shape**: Print, Options, Zoom, Fit to Window and Close Print Preview are `large`;
// Orientation, Next Page and Previous Page are small, in the columns Office stacks them in, Orientation under
// the two fields. `GUESS:` Orientation's size. Every glyph is a glyph this subset already carries for the same
// command, and every one is `GUESS:`:
//
// - **Print draws `print`** and **Options `settings`**, Word's Print Preview's: a printer, and a cog for a set
//   of options.
// - **Orientation draws `orientation`**, Layout's.
// - **Zoom draws `zoom-in` and Fit to Window `page-fit`**, PowerPoint's View's.
// - **Next Page draws `document-arrow-down` and Previous Page `document-arrow-up`**, Word's Print Preview's.
// - **Close Print Preview draws `dismiss-square`**, Word's Close Print Preview and Close Outline View.
//
// **Two commands carry no glyph, and say why**: Print What and Colour/Greyscale are fields, which draw their
// value.

/**
 * PowerPoint's `GroupPrintPreviewPrint`, labelled **Print**: Print and Options, both large. See disagreement 4.
 *
 * **Print** opens the Print dialog. **Options is a dropdown** a host binds, over the menu in
 * `stories/ribbons/print-preview-menus.ts`.
 *
 * **No survivor**: a dialog and a menu.
 */
const powerpointPrintPreviewPrint: readonly RibbonCommand[] = [
  { id: 'powerpoint.print-preview.print.print', label: 'Print', icon: 'print', size: 'large' },
  { id: 'powerpoint.print-preview.print.options', label: 'Options', icon: 'settings', size: 'large' },
];

/**
 * PowerPoint's `GroupPrintPreviewPageSetup`, labelled **Page Setup**: Print What, Orientation and
 * Colour/Greyscale, in a column. See disagreements 1, 2, 3, 5 and 8.
 *
 * **Print What** is a field of PowerPoint's nine printout shapes, starting on Slides. **Orientation** is a
 * dropdown over Layout's list. **Colour/Greyscale** is a field of three, starting on Colour. `GUESS:` the start.
 *
 * **No survivor**: two fields and a menu.
 */
const powerpointPrintPreviewPageSetup: readonly RibbonCommand[] = [
  { id: 'powerpoint.print-preview.page-setup.print-what', label: 'Print What' },
  { id: 'powerpoint.print-preview.page-setup.orientation', label: 'Orientation', icon: 'orientation' },
  { id: 'powerpoint.print-preview.page-setup.colour-greyscale', label: 'Colour/Greyscale' },
];

/**
 * PowerPoint's `GroupZoom` on Print Preview, labelled **Zoom**: Zoom and Fit to Window, both large, View's two.
 * See disagreement 6.
 *
 * **Survivor: Fit to Window.** See this part's header.
 */
const powerpointPrintPreviewZoom: readonly RibbonCommand[] = [
  { id: 'powerpoint.print-preview.zoom.zoom', label: 'Zoom', icon: 'zoom-in', size: 'large' },
  { id: 'powerpoint.print-preview.zoom.fit-to-window', label: 'Fit to Window', icon: 'page-fit', size: 'large', essential: true },
];

/**
 * PowerPoint's `GroupPrintPreviewPreview`, labelled **Preview**: Next Page and Previous Page in a column, then
 * Close Print Preview large. See disagreement 7.
 *
 * **Survivors: Next Page and Previous Page.** See this part's header.
 */
const powerpointPrintPreviewPreview: readonly RibbonCommand[] = [
  { id: 'powerpoint.print-preview.preview.next-page', label: 'Next Page', icon: 'document-arrow-down', essential: true },
  { id: 'powerpoint.print-preview.preview.previous-page', label: 'Previous Page', icon: 'document-arrow-up', essential: true },
  { id: 'powerpoint.print-preview.preview.close-print-preview', label: 'Close Print Preview', icon: 'dismiss-square', size: 'large' },
];

// ## Excel's Print Preview
//
// The unit after PowerPoint's Print Preview, one tab of one application: **Excel's Print Preview tab**, all three
// in-scope groups and seven commands, and Excel's second view tab authored. It is Excel 2007's Print Preview, the
// last Excel with the tab (2010 folded it into File → Print): the sheet as it will print, a page at a time.
// **Nothing is declared once with Word's or PowerPoint's.** Excel shares two group ids with them, and its Zoom is
// `GroupPrintPreviewZoom`, not View's `GroupZoom`. Print, Next Page, Previous Page and Close Print Preview share
// faces with the other two tabs, but those tabs also declare Word's and PowerPoint's commands. A function of the
// application would be three different lists behind one name, which is View's reason too.
//
// ## The shapes, decided by what Office's popup is
//
// **One checkbox a host binds**: Show Margins, unticked. **Buttons**: Print, Page Setup, Zoom, Next Page, Previous
// Page and Close Print Preview. **No menu, no field, no dialog launcher, no gallery, no split button, no toggle
// button, no exclusive set.** `printPreviewMenus('excel', …)` renders an authored empty set, which
// `stories/ribbons/print-preview-menus.ts` records as a finding.
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **Group order: drawn Print, Zoom, Preview, which is Office's; the census declares Print, Preview, Zoom.** The
//    row below keeps the census's order, because it is a transcription. `excelPrintPreviewTab` draws Office's,
//    as Excel's Review and View do. The ladder collapses by priority rather than position, so Zoom, `ancillary`,
//    still gives way first wherever it is drawn.
// 2. **Show Margins is a checkbox, where the brief lists a toggle.** Excel 2007 draws *Show Margins* as a tick
//    under Next Page and Previous Page. When it is ticked, the preview draws margin and column handles that a person
//    drags on the page, which is canvas and not chrome. It is still `toggle: true` here, as every checkbox is.
//    What differs is the face, exactly as Word's Magnifier's does. It starts unticked. `GUESS:` the shape, the
//    position and the start, from memory of Excel 2007.
// 3. **Zoom is a plain button and opens no dialog.** On this tab it switches the preview between the whole page
//    and a magnified page, where View's Zoom opens the Zoom dialog. The brief lists it as a button, and it is
//    drawn as one. `GUESS:` that Office does not draw it pressed while magnified. If Office does, it is a toggle,
//    and `zoom-in` would need a filled drawing.
// 4. **Page Setup is a large face command, not a dialog launcher**: the group's second button. It opens the same
//    Page Setup dialog that Page Layout's three launchers open.
// 5. **The counts agree**: Print 2, Preview 4, Zoom 1, each drawn exactly. Nothing is padded or short.
// 6. **Office greys Next Page and Previous Page** at the last and the first page, and both on a one-page sheet.
//    Both are drawn available, because `disabled` is loop 2's.
//
// ## Survivors: Next Page and Previous Page
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Print**: none. Print opens the Print dialog and Page Setup the Page Setup dialog; both fail rule 1.
// - **Zoom**: none. Zoom passes rule 1: one press magnifies, the next undoes it, and no workbook changes. But it is
//   the group's only command, so a survivor would leave the collapsed popup empty. That is the gate's *opens
//   empty*, and Editor's reason.
// - **Preview: Next Page and Previous Page survive**, for Word's and PowerPoint's Print Preview's reason. Each moves
//   the preview one page with one press, and the other takes it back. They are two, under the ceiling, and two
//   commands stay in the popup. `GUESS:` that the glyphs read as pages rather than *download* and *upload*.
//   **Show Margins** is a checkbox, which the gate refuses. **Close Print Preview** leaves the view and takes the
//   tab with it.
//
// ## Sizes, and every glyph
//
// **Size follows Office's shape**: Print, Page Setup, Zoom and Close Print Preview are `large`. Next Page, Previous
// Page and Show Margins are small, stacked in one column. `GUESS:` Zoom's size. Every glyph is `GUESS:`:
//
// - **Print draws `print`**, Word's and PowerPoint's Print Preview's.
// - **Page Setup draws `settings`**, the cog Word's and PowerPoint's Options draw in the same slot of the same
//   group. Fluent draws no page with a cog. It draws `slide-settings` and `table-settings`, and no document
//   equivalent. Every page glyph this subset carries is already another command's: `document-margins` is Margins,
//   `orientation` is Orientation, `document-page-break` is Breaks and `document-one-page` is Page Layout view. A
//   cog over the printed page's settings is the honest picture left, and Excel's tab has no Options to confuse it
//   with.
// - **Zoom draws `zoom-in`**: a magnifier, which is what the press does.
// - **Next Page draws `document-arrow-down` and Previous Page `document-arrow-up`**, the other two tabs'.
// - **Close Print Preview draws `dismiss-square`**, the other two tabs' cross in a square.
//
// **One command carries no glyph, and says why**: Show Margins is a checkbox, which draws its tick box.

/**
 * Excel's `GroupPrintPreviewPrint`, labelled **Print**: Print and Page Setup, both large. See disagreement 4.
 *
 * **Print** opens the Print dialog; **Page Setup** opens the Page Setup dialog on its Page page.
 *
 * **No survivor**: both open a dialog.
 */
const excelPrintPreviewPrint: readonly RibbonCommand[] = [
  { id: 'excel.print-preview.print.print', label: 'Print', icon: 'print', size: 'large' },
  { id: 'excel.print-preview.print.page-setup', label: 'Page Setup', icon: 'settings', size: 'large' },
];

/**
 * Excel's `GroupPrintPreviewPreview`, labelled **Preview**: Next Page, Previous Page and Show Margins in a column,
 * then Close Print Preview large. See disagreements 2 and 6.
 *
 * **Show Margins is a toggle drawn as a checkbox**, bound as `<mjx-checkbox>`, unticked.
 *
 * **Survivors: Next Page and Previous Page.** See this part's header.
 */
const excelPrintPreviewPreview: readonly RibbonCommand[] = [
  { id: 'excel.print-preview.preview.next-page', label: 'Next Page', icon: 'document-arrow-down', essential: true },
  { id: 'excel.print-preview.preview.previous-page', label: 'Previous Page', icon: 'document-arrow-up', essential: true },
  { id: 'excel.print-preview.preview.show-margins', label: 'Show Margins', toggle: true },
  { id: 'excel.print-preview.preview.close-print-preview', label: 'Close Print Preview', icon: 'dismiss-square', size: 'large' },
];

/**
 * Excel's `GroupPrintPreviewZoom`, labelled **Zoom**: Zoom alone, large. See disagreements 1 and 3.
 *
 * **No survivor**: the group's only command, and a survivor would leave the popup empty.
 */
const excelPrintPreviewZoom: readonly RibbonCommand[] = [
  { id: 'excel.print-preview.zoom.zoom', label: 'Zoom', icon: 'zoom-in', size: 'large' },
];

// ── the commands Background Removal shows ────────────────────────────────────
//
// The ribbon programme's unit after Print Preview: **Word's Background Removal tab**, both in-scope groups
// and four commands, one tab of one application, and the third `appearance: 'view'` tab authored. Office
// shows it only while a picture's background is being removed: the picture is drawn with what will go
// shaded, and the tab holds the two pencils that correct the guess and the two ways out.
//
// ## Written once, as functions of the application
//
// **PowerPoint's and Excel's census rows are the same two groups, with the same counts (3 and 2)**, and
// Office draws the same four commands in all three. So both groups are `backgroundRemovalRefineCommands` and
// `backgroundRemovalCloseCommands`, keyed by application exactly as Draw's eight shared groups are. **Word's
// entry called them first**; PowerPoint's and Excel's now do too (see *PowerPoint's Background Removal* and
// *Excel's Background Removal* below), each adding `commands:` to its two rows and authoring its tab module, and
// otherwise this code. No host binds anything on this tab, so those units wrote no binding either.
//
// ## The shapes
//
// **Two large toggles in one exclusive set that may hold none**: Mark Areas to Keep and Mark Areas to Remove,
// `<app>.background-removal.refine`, neither pressed. **Two large buttons**: Discard All Changes and Keep
// Changes. **No menu, no gallery, no split button, no field, no checkbox, no dialog launcher.**
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **Refine counts 3 and draws 2: Delete Mark is not drawn.** Office 2010, 2013 and 2016 drew a third
//    large button, Delete Mark, which removed one of the straight marks those versions drew. Microsoft 365
//    (and Office 2019) replaced the marks with free-form pencil strokes and dropped Delete Mark from the
//    tab: a stroke is taken back with Undo. The census's count still carries it. This catalogue draws the
//    Microsoft 365 tab it draws everywhere else, so Delete Mark is **left out rather than drawn**, and
//    nothing is padded in its place. `GUESS:` that Microsoft 365 no longer draws it, from memory of the
//    tab rather than a build this project can cite.
// 2. **The two pencils are one exclusive set that may hold none**, a variant of the mechanism this unit
//    added (`exclusiveAllowsNone`, `exclusive-allows-none`). Office holds at most one pencil: arming Mark
//    Areas to Remove puts Mark Areas to Keep down, and pressing the armed pencil again gives back the
//    ordinary pointer. **The tab starts with neither pressed**, because Office arms no pencil on entry.
//    The set of *exactly* one that Word's views and the Draw tools use could not say either of those
//    things: the Draw tab's *no tool* is Select Objects, a command on the tab, and this tab has no such
//    command. `GUESS:` the release on a second press, from Office's other arm-a-gesture commands (Format
//    Painter, Draw Table), and that neither starts pressed. `src/controls/exclusive-set.ts` records the
//    three designs rejected.
// 3. **Close is drawn Discard All Changes, then Keep Changes**, the brief's order. `GUESS:` that it is
//    Office's, which also puts the command that keeps the work nearest the picture's edge of the ribbon.
// 4. **The group is labelled *Refine*, and its census id is `GroupBackgroundRemovalMode`.** Office writes
//    *Refine*; the id is the census's own, used as written.
// 5. **Office's two Close commands both leave the view and take the tab with them**, as Close Outline View
//    and Close Print Preview do; nothing here dispatches, because command dispatch is loop 2.
//
// ## No survivors, on the whole tab
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Refine: none.** Both pencils arm a gesture, and Draw's standard applies word for word: arming a
//   gesture is not one press doing one thing, so both fail rule 1. Unlike Draw's Write, the set has no
//   Select Objects to keep instead.
// - **Close: none.** Discard All Changes throws every mark away and leaves the view: not reversible by one
//   undo, rule 1. Keep Changes leaves the view and takes the tab with it, Close Outline View's reason.
//
// ## Sizes, and every glyph
//
// **All four are `large`**, as Office draws them. Every glyph below is `GUESS:`, judged from Fluent's drawings
// rather than from a build this project can cite:
//
// - **Mark Areas to Keep draws `add-circle` and Mark Areas to Remove `subtract-circle`.** Office draws a
//   pencil with a plus and a pencil with a minus; Fluent draws no pencil with either (`pen-add` and
//   `pen-subtract` do not exist), so the glyph keeps the half that tells the two apart. Both are toggles, so
//   both carry `filled` for the pressed state. Not `add-square` and `subtract-square`, which are already
//   Expand and Collapse, and Show Detail and Hide Detail.
// - **Discard All Changes draws `dismiss-circle` and Keep Changes `checkmark-circle`**: a verdict each, the
//   pair Office draws as a picture with a cross and a picture with a tick. Not `document-dismiss` and
//   `document-checkmark`, which are Review's Reject and Accept on a page; not `dismiss-square`, which is
//   Close Outline View and Close Print Preview leaving a view with nothing discarded; not `arrow-undo`, which
//   is Undo, one step rather than every mark.
//
// ⚠ **Four circles in a row.** The two pairs share an outline and differ by their mark, which is the weakest
// visual choice on the tab: a plus beside a tick may be read as the same kind of command.
//
// ## PowerPoint's Background Removal
//
// The unit after Word's, one tab of one application. **PowerPoint's entry calls the same two functions with
// `'powerpoint'`**, so its ids are `powerpoint.background-removal.*` and its pencils are their own set,
// `powerpoint.background-removal.refine`, looked up in PowerPoint's tab and never Word's. Nothing was written
// for it beyond the two `commands:` and its tab module; no glyph, no binding, no menu.
//
// **No PowerPoint-specific disagreement.** The census's PowerPoint row is Word's row to the field: the same two
// group ids, labels, priorities (`primary`, `secondary`) and counts (3, 2), and Microsoft 365's PowerPoint draws
// the same four commands in the same two groups. Disagreements 1 to 5 above apply to it word for word, Delete
// Mark's absence and the pencils' empty start among them, and are not restated.
//
// ## Excel's Background Removal
//
// The unit after PowerPoint's, one tab of one application, and Excel's first view tab authored. **Excel's entry
// calls the same two functions with `'excel'`**, so its ids are `excel.background-removal.*` and its pencils are
// their own set, `excel.background-removal.refine`, looked up in Excel's tab and never Word's or PowerPoint's.
// Nothing was written for it beyond the two `commands:` and its tab module; no glyph, no binding, no menu.
//
// **No Excel-specific disagreement.** The census's Excel row is Word's and PowerPoint's row to the field: the
// same two group ids, labels, priorities (`primary`, `secondary`) and counts (3, 2), and Microsoft 365's Excel
// draws the same four commands in the same two groups for a picture placed on a sheet. Disagreements 1 to 5
// above apply to it word for word and are not restated.

/**
 * `GroupBackgroundRemovalMode`, labelled **Refine**: Mark Areas to Keep and Mark Areas to Remove, large, in
 * every application. See disagreements 1 and 2.
 *
 * **One exclusive set that may hold none**, `<app>.background-removal.refine`, neither pressed: each arms a
 * pencil that marks what the picture keeps or loses, pressing one puts the other down, and pressing the
 * armed one again puts it down.
 *
 * **No survivor**: both arm a gesture.
 */
function backgroundRemovalRefineCommands(application: RibbonApplication): readonly RibbonCommand[] {
  const refine = `${application}.background-removal.refine`;
  return [
    {
      id: `${application}.background-removal.refine.mark-areas-to-keep`,
      label: 'Mark Areas to Keep',
      icon: 'add-circle',
      size: 'large',
      toggle: true,
      exclusive: refine,
      exclusiveAllowsNone: true,
    },
    {
      id: `${application}.background-removal.refine.mark-areas-to-remove`,
      label: 'Mark Areas to Remove',
      icon: 'subtract-circle',
      size: 'large',
      toggle: true,
      exclusive: refine,
      exclusiveAllowsNone: true,
    },
  ];
}

/**
 * `GroupBackgroundRemovalClose`, labelled **Close**: Discard All Changes and Keep Changes, large, in every
 * application. See disagreements 3 and 5.
 *
 * **Discard All Changes** leaves the picture as it was before the tab opened; **Keep Changes** removes the
 * shaded background. Both leave the view.
 *
 * **No survivor**: one is irreversible, and both leave the view.
 */
function backgroundRemovalCloseCommands(application: RibbonApplication): readonly RibbonCommand[] {
  return [
    {
      id: `${application}.background-removal.close.discard-all-changes`,
      label: 'Discard All Changes',
      icon: 'dismiss-circle',
      size: 'large',
    },
    { id: `${application}.background-removal.close.keep-changes`, label: 'Keep Changes', icon: 'checkmark-circle', size: 'large' },
  ];
}

// ── the commands the master views show ───────────────────────────────────────
//
// ## PowerPoint's Slide Master
//
// The unit after Excel's Print Preview, one tab of one application: **PowerPoint's Slide Master tab**, all six
// in-scope groups and eighteen commands, and PowerPoint's third view tab authored. Office shows it only in Slide
// Master view, which View's *Slide Master* opens: the master and its layouts in the thumbnail pane, and this tab
// to add, rename and preserve them, choose what placeholders a layout carries, and change the theme and the
// background every slide inherits. *Close Master View* takes the deck back to Normal.
//
// ## Three groups written once, for three master views
//
// **`GroupMasterEditTheme`, `GroupBackground` and `GroupMasterClose` are the same three rows, with the same
// counts (4, 11 and 1), on `TabSlideMaster`, `TabHandoutMaster` and `TabNotesMaster`**, and Office draws the
// same commands in all three. So they are `masterEditThemeCommands`, `masterBackgroundCommands` and
// `masterCloseCommands`, functions of the master tab exactly as Background Removal's two groups are functions
// of the application. **Slide Master's entry calls them first**; the Handout Master and Notes Master units add
// `commands:` to their own three rows with their own tab id and write no second declaration. Edit Master,
// Master Layout and Size are Slide Master's alone and are declared as constants.
//
// ## The shapes, decided by what Office's popup is
//
// **One toggle**, Preserve, which keeps a master in the deck when no slide uses it, starting unpressed.
// **One split button a host binds**, Insert Placeholder: its face inserts a Content placeholder and its arrow
// opens Office's ten. **Three checkboxes a host binds**: Title and Footers, ticked, and Hide Background
// Graphics, unticked. **Six dropdowns a host binds**: Themes, Colours, Fonts, Effects, Background Styles and
// Slide Size, over the menus in `stories/ribbons/slide-master-menus.ts`, **every list reused from
// `stories/ribbons/design-layout-menus.ts`** rather than written again. **Buttons**: Insert Slide Master, Insert
// Layout, Delete, Rename, Master Layout and Close Master View. **One dialog launcher**, on Background, which
// opens the Format Background pane. **No field, no gallery, no exclusive set, no split toggle.**
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **Colours, Fonts and Effects are in Edit Theme, as the brief and PowerPoint 2010 place them.** Microsoft
//    365 draws Edit Theme as Themes alone and moves Colours, Fonts and Effects into Background, beside Background
//    Styles. The census's count of 4 for Edit Theme fits the brief's four exactly, and nothing in the census says
//    where the three belong, so the brief is followed. `GUESS:` Microsoft 365's placement, from memory.
// 2. **Master Layout counts 15 and draws 4.** `GUESS:` the reading that the census counts Insert Placeholder's
//    two halves and its ten entries as twelve controls, and Master Layout, Title and Footers as the other three.
//    Nothing is padded.
// 3. **Background counts 11 and draws 2 and a launcher.** `GUESS:` that the census reads Microsoft 365's group,
//    which holds Colours, Fonts and Effects too (disagreement 1), and counts some of their menus' footers
//    (Customise Colours…, Customise Fonts…, Format Background…, Reset Slide Background). No reading this unit
//    can cite reaches 11 exactly, and nothing is padded.
// 4. **Size counts 2 and draws 1.** `GUESS:` the census counts Slide Size's face and its arrow.
// 5. **Edit Master (5), Edit Theme (4) and Close (1) draw exactly their counts.**
// 6. **Every theme list is Design's, and this unit completed three of them.** Office's Colours, Fonts and
//    Effects menus and its Background Styles gallery carry more entries than Design's did, so
//    `themeColourEntries`, `themeFontEntries`, `themeEffectEntries` and `backgroundStyleEntries` now carry
//    Office's whole lists, and **Word's Design, Excel's Page Layout and PowerPoint's Variants footer draw the
//    same lists**. PowerPoint's Themes list is the Design gallery's `themes`, completed to Office's built-in set
//    and drawn here as a menu (`powerpointThemeEntries`). `GUESS:` every list, from memory of Microsoft 365.
// 7. **The census's spelling wins**: *Colours*, *Customise Colours…*, *Greyscale*, where Office writes *Colors*.
// 8. **Office greys** Delete and Rename while the master's in-use layouts are selected, Title and Footers while
//    the master itself is selected, Hide Background Graphics on the master, and Reset Slide Background in master
//    view. All are drawn available, because `disabled` is loop 2's.
// 9. **Preserve starts unpressed.** `GUESS:` that a new deck's one master is not preserved; PowerPoint
//    preserves a master only once a second is inserted or a person presses it.
//
// ## Survivors: none, and why each group keeps none
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Edit Master**: none. Insert Slide Master and Insert Layout pass rule 1, but `slide-text-title-add` and
//   `slide-layout` read as New Slide and Layout once the label is gone (rule 2's standard); Rename opens a dialog;
//   Delete removes a master or a layout; and **un-pressing Preserve on a master no slide uses asks whether to
//   delete it**, so Preserve fails rule 1.
// - **Master Layout**: none. Master Layout opens a dialog, Insert Placeholder is a split button, and Title and
//   Footers are checkboxes, which the gate refuses.
// - **Edit Theme**: none. All four open menus.
// - **Background**: none. Background Styles opens a menu and Hide Background Graphics is a checkbox.
// - **Size**: none. Slide Size opens a menu, and it is the group's only command.
// - **Close**: none. Close Master View leaves the view and takes the tab with it, and it is the only command.
//
// ## Sizes, and every glyph
//
// **Size follows PowerPoint 2010's shape**: Insert Slide Master, Insert Layout, Master Layout, Insert Placeholder,
// Themes, Slide Size and Close Master View are `large`. Delete, Rename and Preserve are small in a column, as are
// Colours, Fonts and Effects, and Background Styles. Every glyph is `GUESS:`:
//
// - **Insert Slide Master draws `slide-text-title-add`**, a slide with a title bar and a plus: a master is the
//   slide carrying the title placeholder, added. Not `slide-add`, which is New Slide's.
// - **Insert Layout draws `slide-layout`**, Home's Layout glyph, a slide with a layout inside it, now at 24 too:
//   the command adds a layout. Fluent draws no layout with a plus.
// - **Delete draws `delete`**, the bin. **Rename draws `rename`**, a text cursor in a field.
// - **Preserve draws `pin`**, the pushpin Office itself draws for Preserve, filled while pressed.
// - **Master Layout draws `slide-text-title-checkmark`**, a slide with a title and a tick: the dialog is a list of
//   ticks for the placeholders a master carries.
// - **Insert Placeholder draws `slide-content`**, a slide holding a picture and lines, which is the Content
//   placeholder its face inserts.
// - **Themes draws `style-guide`**, a fanned swatch book: a set of colours, fonts and effects chosen together.
//   Not `color`, which Colours draws beside it, nor `design-ideas`, Designer's. Word's and Excel's Themes carry
//   none, and this is the first Themes command to carry a glyph.
// - **Colours, Fonts and Effects draw `color`, `text-font` and `square-shadow`**, Design's and Page Layout's.
// - **Background Styles draws `color-background`**, the glyph the Variants footer draws for it.
// - **Slide Size draws `slide-size`**, Design's. **Close Master View draws `dismiss-square`**, the cross in a
//   square every Close view command draws.
//
// **Three commands carry no glyph, and say why**: Title, Footers and Hide Background Graphics are checkboxes,
// which draw their tick box.

/** A master view: the three tabs whose Edit Theme, Background and Close groups are one declaration. */
type MasterViewTab = 'slide-master' | 'handout-master' | 'notes-master';

/**
 * PowerPoint's `GroupMasterEdit`, labelled **Edit Master**: Insert Slide Master and Insert Layout large, then
 * Delete, Rename and Preserve in a column. See disagreements 5, 8 and 9.
 *
 * **Preserve is a toggle**, unpressed. The rest are buttons: Rename opens the Rename Layout dialog.
 *
 * **No survivor**: two glyphs that read as other commands, a dialog, a deletion, and a toggle whose release can
 * ask to delete.
 */
const powerpointSlideMasterEditMaster: readonly RibbonCommand[] = [
  { id: 'powerpoint.slide-master.edit-master.insert-slide-master', label: 'Insert Slide Master', icon: 'slide-text-title-add', size: 'large' },
  { id: 'powerpoint.slide-master.edit-master.insert-layout', label: 'Insert Layout', icon: 'slide-layout', size: 'large' },
  { id: 'powerpoint.slide-master.edit-master.delete', label: 'Delete', icon: 'delete' },
  { id: 'powerpoint.slide-master.edit-master.rename', label: 'Rename', icon: 'rename' },
  { id: 'powerpoint.slide-master.edit-master.preserve', label: 'Preserve', icon: 'pin', toggle: true },
];

/**
 * PowerPoint's `GroupMasterLayout`, labelled **Master Layout**: Master Layout and Insert Placeholder large, then
 * Title and Footers in a column. See disagreement 2.
 *
 * **Master Layout** opens the Master Layout dialog. **Insert Placeholder is a split button** a host binds, over
 * Office's ten placeholders. **Title and Footers are toggles drawn as checkboxes**, both ticked.
 *
 * **No survivor**: a dialog, a split button and two checkboxes.
 */
const powerpointSlideMasterMasterLayout: readonly RibbonCommand[] = [
  { id: 'powerpoint.slide-master.master-layout.master-layout', label: 'Master Layout', icon: 'slide-text-title-checkmark', size: 'large' },
  { id: 'powerpoint.slide-master.master-layout.insert-placeholder', label: 'Insert Placeholder', icon: 'slide-content', size: 'large' },
  { id: 'powerpoint.slide-master.master-layout.title', label: 'Title', toggle: true, pressed: true },
  { id: 'powerpoint.slide-master.master-layout.footers', label: 'Footers', toggle: true, pressed: true },
];

/**
 * `GroupMasterEditTheme`, labelled **Edit Theme**, on any master view: Themes large, then Colours, Fonts and
 * Effects in a column. See disagreements 1, 5 and 6.
 *
 * **All four are dropdowns** a host binds, over Design's lists.
 *
 * **No survivor**: four menus.
 */
function masterEditThemeCommands(tab: MasterViewTab): readonly RibbonCommand[] {
  return [
    { id: `powerpoint.${tab}.edit-theme.themes`, label: 'Themes', icon: 'style-guide', size: 'large' },
    { id: `powerpoint.${tab}.edit-theme.colours`, label: 'Colours', icon: 'color' },
    { id: `powerpoint.${tab}.edit-theme.fonts`, label: 'Fonts', icon: 'text-font' },
    { id: `powerpoint.${tab}.edit-theme.effects`, label: 'Effects', icon: 'square-shadow' },
  ];
}

/**
 * `GroupBackground`, labelled **Background**, on any master view: Background Styles, then Hide Background
 * Graphics, and the Format Background launcher the tab module passes. See disagreements 3 and 8.
 *
 * **Background Styles is a dropdown** a host binds; **Hide Background Graphics is a toggle drawn as a
 * checkbox**, unticked.
 *
 * **No survivor**: a menu and a checkbox.
 */
function masterBackgroundCommands(tab: MasterViewTab): readonly RibbonCommand[] {
  return [
    { id: `powerpoint.${tab}.background.background-styles`, label: 'Background Styles', icon: 'color-background' },
    { id: `powerpoint.${tab}.background.hide-background-graphics`, label: 'Hide Background Graphics', toggle: true },
  ];
}

/**
 * `GroupMasterClose`, labelled **Close**, on any master view: Close Master View, large.
 *
 * **No survivor**: it leaves the view, and it is the only command.
 */
function masterCloseCommands(tab: MasterViewTab): readonly RibbonCommand[] {
  return [{ id: `powerpoint.${tab}.close.close-master-view`, label: 'Close Master View', icon: 'dismiss-square', size: 'large' }];
}

/**
 * PowerPoint's `GroupSlideSize` on Slide Master, labelled **Size**: Slide Size, large, Design's command under this
 * tab's id. See disagreement 4.
 *
 * **No survivor**: a menu, and the only command.
 */
const powerpointSlideMasterSize: readonly RibbonCommand[] = [
  { id: 'powerpoint.slide-master.size.slide-size', label: 'Slide Size', icon: 'slide-size', size: 'large' },
];

// ## PowerPoint's Slide Master Home
//
// The unit after Slide Master, one tab of one application: **PowerPoint's `TabSlideMasterHome`**, the Home tab
// Slide Master view shows beside Slide Master, all six in-scope groups, and PowerPoint's fourth view tab authored.
// Office labels it *Home*, and the ordinary Home tab is never on screen with it; this catalogue renders both, which
// is the one place the two labels collide.
//
// ## Five groups written once, with Home
//
// **Clipboard, Font, Paragraph, Drawing and Editing are Home's rows, with Home's ids and Home's counts** (10, 18,
// 27, 63 and 8), so they are `powerpointHomeClipboardCommands`, `powerpointHomeFontCommands`,
// `powerpointHomeParagraphCommands`, `powerpointHomeDrawingCommands` and `powerpointHomeEditingCommands`, functions
// of the Home tab in the *commands Home shows* section. Home calls them with `'home'` and draws exactly what it drew
// before; this tab calls them with `'slide-master-home'`, so every id is `powerpoint.slide-master-home.<group>.<command>`
// and a host binds each tab's controls separately. **Every shape, size, glyph, toggle and survivor in those five
// groups is Home's**, and so is every reason: Bold, Italic and Underline survive in Font, Align Left, Centre and
// Align Right in Paragraph, and Clipboard, Drawing and Editing keep none. Only **Master Slides** is this tab's own.
//
// ## Master Slides, Office's master-view counterpart to Home's Slides
//
// Where Home's second group adds, lays out and sections slides, this one adds **masters and layouts**: Insert Slide
// Master and Insert Layout, large, then Layout, Reset and Section in a column. **Insert Slide Master and Insert
// Layout are Slide Master's own commands**, drawn with Slide Master's glyphs and sizes under this tab's ids; Reset
// is Home's. **Layout and Section are dropdowns a host binds**, over the menus in
// `stories/ribbons/slide-master-menus.ts`: Layout lists the Office Theme's eleven layouts, and Section lists Office's
// six section commands.
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **Master Slides counts 9 and draws 5.** `GUESS:` the reading that the census counts the five commands and four
//    of Section's entries, the four that change the deck's sections (Add, Rename, Remove, Remove All), leaving out
//    Collapse All and Expand All, which only fold the thumbnail pane. The menus draw Office's whole lists, and no
//    reading of what is drawn reaches 9 exactly. Nothing is padded.
// 2. **Layout draws `layout-row-two-split-top`, not Home's `slide-layout`.** Insert Layout already draws
//    `slide-layout`, as it does on Slide Master, in the same group; two commands in one group wearing one glyph is
//    the defect unit 2 removed from Arrange. Insert Layout's entry is identical to Slide Master's and keeps its glyph;
//    Layout's is not identical to Home's (Home draws a button, this a dropdown) and takes a new one, a frame divided
//    into a title row and two panes. `GUESS:`.
// 3. **Layout and Section are dropdowns here and generic buttons on Home.** Home's Slides group records Office's
//    Layout as a gallery and Section as a menu, and draws both as buttons; Home is unchanged by this unit, so the two
//    tabs draw one command two ways. The dropdown is the shape the brief and Office give.
// 4. **Layout's list is not Insert's New Slide list.** New Slide lists seven Office Theme layouts; Office's Layout
//    gallery lists all eleven, Title and Vertical Text and Vertical Title and Text included, so the two are not the
//    same list and neither is reused. `GUESS:` the eleven and their order, from memory of Microsoft 365, and that no
//    entry is ticked while a master, rather than a layout, is selected.
// 5. **Office greys Section, and Reset, in master view** (`GUESS:`), because a master has no sections and Reset acts
//    on a slide. Both are drawn available, because `disabled` is loop 2's.
// 6. **Every other count agrees with Home's**, row for row.
// 7. **The dialog launchers are Home's four** (Clipboard settings, Font settings, Paragraph settings, Shape
//    settings), and Master Slides has none, as Slides has none. `GUESS:` that master view keeps all four.
//
// ## Survivors: Home's, and none in Master Slides
//
// Font keeps Bold, Italic and Underline, and Paragraph Align Left, Centre and Align Right, exactly as on Home.
// **Master Slides keeps none**: Layout and Section open menus, Insert Slide Master's and Insert Layout's glyphs read
// as New Slide and Layout without a label, and Reset's loop fails rule 2, as on Home.
//
// ## Sizes, and every glyph
//
// Master Slides follows Home's Slides: the two insert commands large, the rest small in a column. Every glyph but
// Layout's is one the subset already draws for the same command: `slide-text-title-add` and `slide-layout` (Slide
// Master's Insert Slide Master and Insert Layout), `arrow-reset` and `slide-multiple` (Home's Reset and Section).
// **Layout draws `layout-row-two-split-top`, new** (disagreement 2). No command in Master Slides lacks a glyph.

/**
 * PowerPoint's `GroupMasterSlides` on Slide Master Home, labelled **Master Slides**: Insert Slide Master and Insert
 * Layout large, then Layout, Reset and Section in a column. See disagreements 1 to 5.
 *
 * **Layout and Section are dropdowns** a host binds; the rest are buttons.
 *
 * **No survivor**: two menus, two glyphs that read as other commands, and Reset's loop.
 */
const powerpointSlideMasterHomeMasterSlides: readonly RibbonCommand[] = [
  { id: 'powerpoint.slide-master-home.master-slides.insert-slide-master', label: 'Insert Slide Master', icon: 'slide-text-title-add', size: 'large' },
  { id: 'powerpoint.slide-master-home.master-slides.insert-layout', label: 'Insert Layout', icon: 'slide-layout', size: 'large' },
  { id: 'powerpoint.slide-master-home.master-slides.layout', label: 'Layout', icon: 'layout-row-two-split-top' },
  { id: 'powerpoint.slide-master-home.master-slides.reset', label: 'Reset', icon: 'arrow-reset' },
  { id: 'powerpoint.slide-master-home.master-slides.section', label: 'Section', icon: 'slide-multiple' },
];

// ## PowerPoint's Handout Master
//
// The unit after Slide Master Home, one tab of one application: **PowerPoint's `TabHandoutMaster`**, all five
// in-scope groups and fourteen commands, and PowerPoint's fifth view tab authored. Office shows it only in Handout
// Master view, which View's *Handout Master* opens: one printed handout page, the slide frames laid out on it, and
// the header, date, footer and page number around them. *Close Master View* takes the deck back to Normal.
//
// ## Three groups are Slide Master's, called rather than written
//
// **Edit Theme, Background and Close are `masterEditThemeCommands('handout-master')`,
// `masterBackgroundCommands('handout-master')` and `masterCloseCommands('handout-master')`**, the functions Slide
// Master's part wrote for this unit. The rows' ids and counts (4, 11 and 1) are Slide Master's, and so are every
// shape, size, glyph and reason, disagreements 1, 3, 6 and 8 of that part included. Every id is
// `powerpoint.handout-master.<group>.<command>`, so a host binds this tab's controls apart from Slide Master's.
// **Page Setup and Placeholders are this tab's own** and are declared as constants.
//
// ## The shapes, decided by what Office's popup is
//
// **Three dropdowns a host binds in Page Setup**: Handout Orientation, over Layout's own `orientationEntries()`;
// Slide Size, over Design's `slideSizeEntries()`; and Slides Per Page, over Office's seven handout layouts, written
// in `stories/ribbons/slide-master-menus.ts` because no other tab opens it. **Four checkboxes a host binds in
// Placeholders**: Header, Date, Footer and Page Number, all ticked. The shared groups add **five dropdowns**
// (Themes, Colours, Fonts, Effects, Background Styles), **one checkbox** (Hide Background Graphics), **one button**
// (Close Master View) and **one dialog launcher**, Format Background. **No field, no gallery, no split button, no
// toggle, no exclusive set.**
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **Page Setup counts 11 and draws 3.** `GUESS:` the reading that the census counts **the eleven choices the
//    group's three menus offer**: Portrait and Landscape (2), Standard (4:3) and Widescreen (16:9) (2), and 1, 2, 3,
//    4, 6 and 9 Slides and Outline (7), leaving out the faces and Slide Size's *Custom Slide Size…*, which opens a
//    dialog rather than choosing. It is the one reading this unit found that reaches 11 exactly. Nothing is padded.
// 2. **Placeholders counts 4 and draws 4**, one checkbox per placeholder. **Edit Theme (4) and Close (1) draw
//    their counts; Background counts 11 and draws 2 and a launcher**, Slide Master's disagreement 3, unchanged.
// 3. **PowerPoint 2010's Page Setup is not this one.** 2010 drew *Page Setup* (a dialog), *Handout Orientation*,
//    *Slide Orientation* and *Slides Per Page*; Microsoft 365 draws Handout Orientation, Slide Size and Slides Per
//    Page. The brief names 365's three and they are drawn. `GUESS:` both, from memory.
// 4. **Handout Orientation opens Layout's list unchanged**, Portrait checked, as PowerPoint's Print Preview's
//    Orientation does: a handout prints on a portrait page by default. `GUESS:` that Office's labels are Word's.
// 5. **Slides Per Page starts on 6 Slides**, a new deck's handout master. `GUESS:` the start, and that Office's
//    labels are *1 Slide*, *2 Slides* … *9 Slides* and *Outline*, in that order, with no separator before Outline.
//    They are the shapes PowerPoint's Print Preview's Print What lists as *Handouts (n Slides Per Page)* and
//    *Outline View*; the two lists name one set of layouts in two voices, so neither is reused.
// 6. **Header, Date, Footer and Page Number start ticked**: a new deck's handout master carries all four
//    placeholders. `GUESS:`.
// 7. **Office greys Themes on Handout Master** (`GUESS:`), because a handout master cannot take a theme of its
//    own. It is drawn available, because `disabled` is loop 2's, and so is Hide Background Graphics, which Office
//    may also grey here.
// 8. **The launcher at Background's corner is Format Background**, as on Slide Master. `GUESS:` that Handout
//    Master keeps it.
//
// ## Survivors: none, and why each group keeps none
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Page Setup**: none. Handout Orientation, Slide Size and Slides Per Page each open a menu, which fails rule 1.
// - **Placeholders**: none. All four are checkboxes, which the gate refuses.
// - **Edit Theme**: none. All four open menus, as on Slide Master.
// - **Background**: none. Background Styles opens a menu and Hide Background Graphics is a checkbox.
// - **Close**: none. Close Master View leaves the view and takes the tab with it, and it is the only command.
//
// ## Sizes, and every glyph
//
// **Size follows Microsoft 365's shape**: Handout Orientation, Slide Size and Slides Per Page are `large`, side by
// side, and the four checkboxes are stacked in two columns. The shared groups keep Slide Master's sizes. Every
// glyph is one this subset already carries, and every one is `GUESS:`:
//
// - **Handout Orientation draws `orientation`**, Layout's and both Print Previews': a portrait page turning to
//   landscape, the same choice.
// - **Slide Size draws `slide-size`**, Design's and Slide Master's.
// - **Slides Per Page draws `layout-cell-four`**, a page divided into four frames: slides laid out on one handout.
//   Excel's Arrange All draws it for four windows tiled; the two are never on one ribbon. Not `slide-grid`, which
//   is Slide Sorter, nor `document-one-page-multiple`, which is Handout Master itself on View.
// - **Themes, Colours, Fonts, Effects, Background Styles and Close Master View** draw Slide Master's `style-guide`,
//   `color`, `text-font`, `square-shadow`, `color-background` and `dismiss-square`.
//
// **Five commands carry no glyph, and say why**: Header, Date, Footer, Page Number and Hide Background Graphics
// are checkboxes, which draw their tick box.

/**
 * PowerPoint's `GroupPageSetupHandoutMaster`, labelled **Page Setup**: Handout Orientation, Slide Size and Slides
 * Per Page, all large. See disagreements 1, 3, 4 and 5.
 *
 * **All three are dropdowns** a host binds, over the menus in `stories/ribbons/slide-master-menus.ts`.
 *
 * **No survivor**: three menus.
 */
const powerpointHandoutMasterPageSetup: readonly RibbonCommand[] = [
  { id: 'powerpoint.handout-master.page-setup.handout-orientation', label: 'Handout Orientation', icon: 'orientation', size: 'large' },
  { id: 'powerpoint.handout-master.page-setup.slide-size', label: 'Slide Size', icon: 'slide-size', size: 'large' },
  { id: 'powerpoint.handout-master.page-setup.slides-per-page', label: 'Slides Per Page', icon: 'layout-cell-four', size: 'large' },
];

/**
 * PowerPoint's `GroupPlaceholdersHandoutMaster`, labelled **Placeholders**: Header and Date, then Footer and Page
 * Number, in two columns. See disagreements 2 and 6.
 *
 * **All four are toggles drawn as checkboxes**, ticked, which a host binds.
 *
 * **No survivor**: four checkboxes.
 */
const powerpointHandoutMasterPlaceholders: readonly RibbonCommand[] = [
  { id: 'powerpoint.handout-master.placeholders.header', label: 'Header', toggle: true, pressed: true },
  { id: 'powerpoint.handout-master.placeholders.date', label: 'Date', toggle: true, pressed: true },
  { id: 'powerpoint.handout-master.placeholders.footer', label: 'Footer', toggle: true, pressed: true },
  { id: 'powerpoint.handout-master.placeholders.page-number', label: 'Page Number', toggle: true, pressed: true },
];

// ## PowerPoint's Notes Master
//
// The unit after Handout Master, one tab of one application: **PowerPoint's `TabNotesMaster`**, all five in-scope
// groups and fifteen commands, and PowerPoint's sixth view tab authored. Office shows it only in Notes Master view,
// which View's *Notes Master* opens: one printed notes page, the slide image at its top, the notes body under it,
// and the header, date, footer and page number around them. *Close Master View* takes the deck back to Normal.
//
// ## Three groups are Slide Master's, called rather than written
//
// **Edit Theme, Background and Close are `masterEditThemeCommands('notes-master')`,
// `masterBackgroundCommands('notes-master')` and `masterCloseCommands('notes-master')`**, as Handout Master's are.
// The rows' ids and counts (4, 11 and 1) are Slide Master's, and so are every shape, size, glyph and reason,
// disagreements 1, 3, 6 and 8 of that part included. Every id is `powerpoint.notes-master.<group>.<command>`, so a
// host binds this tab's controls apart from Slide Master's and Handout Master's. **Page Setup and Placeholders are
// this tab's own** and are declared as constants.
//
// ## The shapes, decided by what Office's popup is
//
// **Two dropdowns a host binds in Page Setup**: Notes Page Orientation, over Layout's own `orientationEntries()`,
// and Slide Size, over Design's `slideSizeEntries()`. Neither list is written again. **Six checkboxes a host binds
// in Placeholders**: Header, Slide Image, Footer, Date, Body and Page Number, all ticked. The shared groups add
// **five dropdowns** (Themes, Colours, Fonts, Effects, Background Styles), **one checkbox** (Hide Background
// Graphics), **one button** (Close Master View) and **one dialog launcher**, Format Background. **No field, no
// gallery, no split button, no toggle, no exclusive set**, and no list of this tab's own.
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **Page Setup counts 3 and draws 2.** `GUESS:` the reading that the census counts **Slide Size's face and its
//    arrow as two**, exactly as Slide Master's Size counts 2 for the same one command (that part's disagreement 4),
//    and Notes Page Orientation as the third. A second reading also reaches 3: **PowerPoint 2010's group**, *Page
//    Setup* (a dialog), *Notes Page Orientation* and *Slide Orientation* (disagreement 4 below). **Handout Master's
//    reading does not reach it**: counting the choices the menus offer gives 4 here (Portrait and Landscape,
//    Standard and Widescreen), so the census's two Page Setup counts cannot both be read one way, and this unit
//    records that rather than choosing a reading that fits both. Nothing is padded.
// 2. **Placeholders counts 6 and draws 6**, one checkbox per placeholder. **Edit Theme (4) and Close (1) draw their
//    counts; Background counts 11 and draws 2 and a launcher**, Slide Master's disagreement 3, unchanged.
// 3. **Background is `primary` and Page Setup `standard` on this tab**, the census's priorities, kept. They are the
//    reverse of Handout Master's, where Page Setup is `primary` and Background `standard`, although the two tabs'
//    Background rows are the same group with the same commands. So as the ribbon narrows here Page Setup,
//    Placeholders and Edit Theme collapse before Background does, and on Handout Master Background collapses
//    before Page Setup. The census wins; no reason for the difference is recorded in it.
// 4. **PowerPoint 2010's Page Setup is not this one.** 2010 drew *Page Setup*, *Notes Page Orientation* and *Slide
//    Orientation*; Microsoft 365 draws Notes Page Orientation and Slide Size. The brief names 365's two and they
//    are drawn. `GUESS:` both, from memory.
// 5. **Notes Page Orientation opens Layout's list unchanged**, Portrait checked, as Handout Orientation does: a
//    notes page prints portrait by default. `GUESS:` that Office's labels are Word's.
// 6. **All six placeholders start ticked**: a new deck's notes master carries all six. `GUESS:`. The order is the
//    brief's, Header, Slide Image and Footer, then Date, Body and Page Number, which is Office's two columns of
//    three read down each column. `GUESS:` that column order.
// 7. **Office greys Themes on Notes Master** (`GUESS:`), because a notes master cannot take a theme of its own. It
//    is drawn available, because `disabled` is loop 2's, and so is Hide Background Graphics.
// 8. **The launcher at Background's corner is Format Background**, as on Slide Master and Handout Master.
//    `GUESS:` that Notes Master keeps it.
//
// ## Survivors: none, and why each group keeps none
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Page Setup**: none. Notes Page Orientation and Slide Size each open a menu, which fails rule 1.
// - **Placeholders**: none. All six are checkboxes, which the gate refuses.
// - **Edit Theme**: none. All four open menus, as on Slide Master.
// - **Background**: none. Background Styles opens a menu and Hide Background Graphics is a checkbox. Its `primary`
//   priority changes when the group collapses, not what may survive it.
// - **Close**: none. Close Master View leaves the view and takes the tab with it, and it is the only command.
//
// ## Sizes, and every glyph
//
// **Size follows Microsoft 365's shape**: Notes Page Orientation and Slide Size are `large`, side by side, and the
// six checkboxes are small. The shared groups keep Slide Master's sizes. Every glyph is one this subset already
// carries, and every one is `GUESS:`:
//
// - **Notes Page Orientation draws `orientation`**, Layout's, both Print Previews' and Handout Orientation's: a
//   portrait page turning to landscape, the same choice.
// - **Slide Size draws `slide-size`**, Design's, Slide Master's and Handout Master's.
// - **Themes, Colours, Fonts, Effects, Background Styles and Close Master View** draw Slide Master's `style-guide`,
//   `color`, `text-font`, `square-shadow`, `color-background` and `dismiss-square`.
//
// **Seven commands carry no glyph, and say why**: Header, Slide Image, Footer, Date, Body, Page Number and Hide
// Background Graphics are checkboxes, which draw their tick box.

/**
 * PowerPoint's `GroupPageSetupNotesMaster`, labelled **Page Setup**: Notes Page Orientation and Slide Size, both
 * large. See disagreements 1, 4 and 5.
 *
 * **Both are dropdowns** a host binds, over the menus in `stories/ribbons/slide-master-menus.ts`.
 *
 * **No survivor**: two menus.
 */
const powerpointNotesMasterPageSetup: readonly RibbonCommand[] = [
  { id: 'powerpoint.notes-master.page-setup.notes-page-orientation', label: 'Notes Page Orientation', icon: 'orientation', size: 'large' },
  { id: 'powerpoint.notes-master.page-setup.slide-size', label: 'Slide Size', icon: 'slide-size', size: 'large' },
];

/**
 * PowerPoint's `GroupPlaceholdersNotesMaster`, labelled **Placeholders**: Header, Slide Image and Footer, then Date,
 * Body and Page Number. See disagreements 2 and 6.
 *
 * **All six are toggles drawn as checkboxes**, ticked, which a host binds.
 *
 * **No survivor**: six checkboxes.
 */
const powerpointNotesMasterPlaceholders: readonly RibbonCommand[] = [
  { id: 'powerpoint.notes-master.placeholders.header', label: 'Header', toggle: true, pressed: true },
  { id: 'powerpoint.notes-master.placeholders.slide-image', label: 'Slide Image', toggle: true, pressed: true },
  { id: 'powerpoint.notes-master.placeholders.footer', label: 'Footer', toggle: true, pressed: true },
  { id: 'powerpoint.notes-master.placeholders.date', label: 'Date', toggle: true, pressed: true },
  { id: 'powerpoint.notes-master.placeholders.body', label: 'Body', toggle: true, pressed: true },
  { id: 'powerpoint.notes-master.placeholders.page-number', label: 'Page Number', toggle: true, pressed: true },
];

// ── the commands the colour modes show ───────────────────────────────────────
//
// ## PowerPoint's Black and White
//
// The unit after Notes Master, one tab of one application: **PowerPoint's `TabBlackAndWhite`**, both in-scope
// groups and eleven commands, and PowerPoint's seventh view tab authored. Office shows it only while the deck is
// previewed in black and white, which View's *Black and White* opens. The slides are drawn as they would print on a
// black-and-white printer, and this tab chooses how the **selected object** is drawn in that preview. *Back To
// Colour View* takes the deck back to colour. It is a view-only tab: nothing on it changes a slide's colours.
//
// ## Two groups written once, for two colour modes
//
// **`GroupColorModeSetting` and `GroupColorModeClose` are the same two rows, with the same labels, priorities and
// counts (10 and 1), on `TabBlackAndWhite` and `TabGrayscale`**, and Office draws the same eleven commands on both.
// So they are `colourModeSettingCommands` and `colourModeCloseCommands`, functions of the colour-mode tab, exactly
// as the master views' shared groups are functions of the master tab. **Black and White's entry called them
// first**; the Greyscale unit added `commands:` to its own two rows with `'greyscale'` and wrote no second
// declaration (see *PowerPoint's Greyscale* below).
// Every id is `powerpoint.<tab>.<group>.<command>`, so a host binds each tab's controls apart, and each tab's
// settings are **their own exclusive set**, `powerpoint.<tab>.colour-mode`, looked up in its own tab.
//
// ## The shape: large toggles in one exclusive set, not a gallery
//
// **Ten toggles and one exclusive set, Automatic pressed.** The brief offered two shapes, and the toggles win for
// four reasons:
//
// - **Office draws buttons, not a gallery.** The ten are a row of labelled commands on the ribbon's face, with no
//   scroll arrows, no *More* expander and no popup grid, which are what `<mjx-gallery>` draws.
// - **The census counts 10, one per setting.** A gallery is one control that draws ten items, so it would count
//   1. The toggles draw the count exactly.
// - **The choice is exclusive and has a current value.** Office highlights the setting the selection carries, one
//   of ten, and pressing another replaces it. That is what `exclusive` holds (see `src/controls/exclusive-set.ts`),
//   and each toggle keeps `aria-pressed`, one tab stop per setting, as every other set on the ribbon does.
// - **A gallery's items are pictures of the result.** Every gallery in this subset previews what a press produces
//   (themes, transitions, animations). A picture of *grey outline on a white fill* needs a colour swatch, which the
//   subset refuses (see *every glyph* below). A gallery of ten labelled but pictureless items is a menu drawn as a
//   strip.
//
// `GUESS:` that the settings map one to one onto DrawingML's `ST_BlackWhiteMode` (the `bwMode` attribute on a
// shape's properties, read by `mjx-dml`): `auto`, `gray`, `ltGray`, `invGray`, `grayWhite`, `blackGray`,
// `blackWhite`, `black`, `white` and `hidden`, which is the type's eleven values without `clr`. From memory of
// ECMA-376 Part 1, not read from `References/`.
//
// **One button**, Back To Colour View. **No menu, no field, no checkbox, no split button, no dialog launcher**,
// and nothing a host binds: every command is the generic toggle or button `renderCommand` draws.
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **Office labels the group *Change Selected Object*; the census labels it *Colour Mode*.** The census wins,
//    as it does for every label. The brief names both.
// 2. **The census's spelling wins over Office's**, as on View (that part's disagreement 1). Office writes
//    *Grayscale*, *Gray* and *Back To Color View*. So the commands are **Greyscale**, **Light Greyscale**,
//    **Inverse Greyscale**, **Grey with White Fill**, **Black with Greyscale Fill** and **Back To Colour View**.
//    `GUESS:` Office's capitals (*Back To*, lower-case *with*), from memory.
// 3. **The counts agree**: Colour Mode counts 10 and draws 10; Close counts 1 and draws 1. Nothing is padded or
//    left out.
// 4. **Office's set reflects a selection; this one always holds one.** With nothing selected, Office greys the ten
//    (`GUESS:`), and with a selection whose objects carry different settings it highlights none. Here the set
//    starts on **Automatic**, the setting a new shape carries, and always holds exactly one, because the census
//    gate requires one pressed and `disabled` is loop 2's. `GUESS:` Automatic.
// 5. **A second press keeps the setting.** The set does not declare `exclusiveAllowsNone`: pressing Black while
//    Black holds leaves the object black, as pressing Normal on View leaves the deck in Normal. `GUESS:`.
// 6. **The two colour-mode tabs are two sets here.** `GUESS:` that Office keeps the black-and-white setting and
//    the greyscale setting of an object apart. If the file holds one `bwMode` per shape, as `mjx-dml` reads it,
//    Office may show one tab's choice on the other, and a document-backed host would press the same member on
//    both. This catalogue draws two independent sets, because a set is looked up in its own tab.
// 7. **Ten commands and one row.** Office draws all ten large. Five are `small` here, for the reasons under
//    *Sizes*, so the row is three large toggles between small columns, and Office's order is kept.
//
// ## Survivors: none, and why each group keeps none
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Colour Mode**: none. It is an exclusive set: a press is taken back only by pressing another member, which
//   rule 1 does not allow, as for View's sets. Five members carry no glyph besides, which fails rule 3.
// - **Close**: none. Back To Colour View leaves the view and takes the tab with it, and it is the only command.
//
// ## Sizes, and every glyph
//
// A command is `large` where Office draws it large **and** it has an honest glyph **and** its label wraps inside
// `largeControlWidthUnits`. **Greyscale, Light Greyscale, Inverse Greyscale and Don't Show are large**: two tokens,
// no word longer than *Greyscale* and *Inverse*, and every line shorter than *Outline View*, which fits on Word's
// Close Outline View. **Back To Colour View is large** for the same reason: it wraps as *Back To* over *Colour View*,
// eleven letters, one fewer than *Outline View*. `GUESS:` both, unmeasured. Every glyph is `GUESS:`:
//
// - **Greyscale draws `color-off`**, View's Greyscale: the palette struck through, the colour taken out. It gains a
//   24.
// - **Light Greyscale draws `brightness-high`**, new: a sun with long rays, greyscale lightened. Not
//   `weather-sunny`, which reads as weather.
// - **Inverse Greyscale draws `dark-theme`**, Word's and Excel's Switch Modes: a circle half dark, light and dark
//   exchanged. Never on one ribbon with Word's or Excel's. Not `circle-half-fill`, which View's *Black and White*
//   draws on this same ribbon.
// - **Don't Show draws `eye-off`**, Excel's Hide: an eye struck through, the object not drawn. It gains a 24 and
//   the filled drawing, because it is a toggle here.
// - **Back To Colour View draws `dismiss-square`**, the verb every view tab's close draws: leaving a view. Not
//   `color`, which View's *Colour* toggle and Edit Theme's *Colours* draw.
//
// **Five commands carry no glyph, and say why.** **Automatic** is a rule, not a look (each object's kind decides
// how it is drawn), and Fluent's *auto* glyphs are a camera flash (`flash-auto`) and a translation
// (`translate-auto`); `sparkle` reads as *the computer will decide* in Copilot's sense. **Grey with White Fill,
// Black with Greyscale Fill, Black with White Fill, Black and White** name the colours an object's line and fill
// print in, and no glyph in this subset can say that honestly, for two reasons. A glyph is drawn in **one tint from
// a token**, so it has ink and paper but no grey, and its ink is light in the dark theme, so a *black* square would
// draw white. And a toggle draws Fluent's **filled** drawing while it holds, so on a command whose meaning *is* its
// fill, pressing it would change what it says: `square` for Black with White Fill would become Black's solid square
// the moment it held. Office draws coloured shape swatches here, and the subset refuses `_color` drawings. All five
// are `small`, because a large button without a glyph is a label in a tall box.
//
// ## PowerPoint's Greyscale
//
// The unit after Black and White, one tab of one application: **PowerPoint's `TabGrayscale`**, both in-scope groups
// and eleven commands, and PowerPoint's eighth and last view tab authored. Office shows it only while the deck is
// previewed in greyscale, which View's *Greyscale* opens, and the tab chooses how the **selected object** is drawn in
// that preview. It is a view-only tab: nothing on it changes a slide's colours.
//
// **Its two rows call the two functions above with `'greyscale'`**, so every label, size, glyph, priority and count
// is Black and White's, and every disagreement above holds as written. Its ids are `powerpoint.greyscale.<group>.
// <command>`, and its settings are **their own exclusive set**, `powerpoint.greyscale.colour-mode`, Automatic
// pressed, apart from `powerpoint.black-and-white.colour-mode` (disagreement 6). Nothing else is written.
//
// **No Greyscale-specific difference from Office is known.** Office draws the same *Change Selected Object* group,
// with the same ten settings in the same order, and the same *Back To Color View*. Two things could differ, and
// neither is drawn:
//
// - **The start.** `GUESS:` that Office starts a new shape on Automatic in greyscale as in black and white, so both
//   sets start on Automatic. From memory, not observed.
// - **The shared setting.** If Office keeps one `bwMode` per shape (disagreement 6), pressing a setting here would
//   press it on Black and White too. The two sets are independent here.

/** A colour-mode view: the two tabs whose Colour Mode and Close groups are one declaration. */
type ColourModeTab = 'black-and-white' | 'greyscale';

/**
 * `GroupColorModeSetting`, labelled **Colour Mode** (Office's *Change Selected Object*), on either colour-mode tab:
 * Automatic, Greyscale, Light Greyscale, Inverse Greyscale, Grey with White Fill, Black with Greyscale Fill, Black
 * with White Fill, Black, White and Don't Show. See disagreements 1, 2 and 4 to 7.
 *
 * **Ten toggles and one exclusive set**, `powerpoint.<tab>.colour-mode`, Automatic pressed. Each sets how the selected
 * object is drawn in this colour mode.
 *
 * **No survivor**: an exclusive set, and five members carry no glyph.
 */
function colourModeSettingCommands(tab: ColourModeTab): readonly RibbonCommand[] {
  const setting = `powerpoint.${tab}.colour-mode`;
  return [
    { id: `${setting}.automatic`, label: 'Automatic', toggle: true, pressed: true, exclusive: setting },
    { id: `${setting}.greyscale`, label: 'Greyscale', icon: 'color-off', size: 'large', toggle: true, exclusive: setting },
    {
      id: `${setting}.light-greyscale`,
      label: 'Light Greyscale',
      icon: 'brightness-high',
      size: 'large',
      toggle: true,
      exclusive: setting,
    },
    {
      id: `${setting}.inverse-greyscale`,
      label: 'Inverse Greyscale',
      icon: 'dark-theme',
      size: 'large',
      toggle: true,
      exclusive: setting,
    },
    { id: `${setting}.grey-with-white-fill`, label: 'Grey with White Fill', toggle: true, exclusive: setting },
    { id: `${setting}.black-with-greyscale-fill`, label: 'Black with Greyscale Fill', toggle: true, exclusive: setting },
    { id: `${setting}.black-with-white-fill`, label: 'Black with White Fill', toggle: true, exclusive: setting },
    { id: `${setting}.black`, label: 'Black', toggle: true, exclusive: setting },
    { id: `${setting}.white`, label: 'White', toggle: true, exclusive: setting },
    { id: `${setting}.dont-show`, label: "Don't Show", icon: 'eye-off', size: 'large', toggle: true, exclusive: setting },
  ];
}

/**
 * `GroupColorModeClose`, labelled **Close**, on either colour-mode tab: Back To Colour View, large. See disagreement 2.
 *
 * **No survivor**: it leaves the view, and it is the only command.
 */
function colourModeCloseCommands(tab: ColourModeTab): readonly RibbonCommand[] {
  return [
    { id: `powerpoint.${tab}.close.back-to-colour-view`, label: 'Back To Colour View', icon: 'dismiss-square', size: 'large' },
  ];
}

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
      { id: 'GroupArrange', label: 'Arrange', priority: 'standard', controls: 65, inScope: true, commands: arrangeCommands('word', 'layout') },
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
      { id: 'GroupEnvelopeLabelCreate', label: 'Create', priority: 'standard', controls: 10, inScope: true, commands: wordMailingsCreate },
      { id: 'GroupMailMergeStart', label: 'Start Mail Merge', priority: 'primary', controls: 13, inScope: true, commands: wordMailingsStartMailMerge },
      { id: 'GroupMailMergeWriteInsertFields', label: 'Write & Insert Fields', priority: 'primary', controls: 11, inScope: true, commands: wordMailingsWriteInsertFields },
      { id: 'GroupMailMergePreviewResults', label: 'Preview Results', priority: 'standard', controls: 8, inScope: true, commands: wordMailingsPreviewResults },
      { id: 'GroupMailMergeFinish', label: 'Finish', priority: 'standard', controls: 4, inScope: true, commands: wordMailingsFinish },
    ],
  },
  {
    id: 'review',
    label: 'Review',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabReviewWord' },
    groups: [
      { id: 'GroupProofing', label: 'Proofing', priority: 'standard', controls: 8, inScope: true, commands: wordReviewProofing },
      { id: 'GroupAccessibility', label: 'Accessibility', priority: 'standard', controls: 6, inScope: true, commands: wordReviewAccessibility },
      { id: 'GroupLanguage', label: 'Language', priority: 'standard', controls: 14, inScope: true, commands: wordReviewLanguage },
      { id: 'GroupComments', label: 'Comments', priority: 'primary', controls: 16, inScope: true, commands: wordReviewComments },
      { id: 'GroupInk', label: 'Ink', priority: 'standard', controls: 4, inScope: true, commands: wordReviewInk },
      { id: 'GroupChangesTracking', label: 'Tracking', priority: 'primary', controls: 22, inScope: true, commands: wordReviewTracking },
      { id: 'GroupChanges', label: 'Changes', priority: 'standard', controls: 14, inScope: true, commands: wordReviewChanges },
      { id: 'GroupCompare', label: 'Compare', priority: 'standard', controls: 8, inScope: true, commands: wordReviewCompare },
      { id: 'GroupProtect', label: 'Protect', priority: 'standard', controls: 4, inScope: true, commands: wordReviewProtect },
    ],
  },
  {
    id: 'view',
    label: 'View',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabView' },
    groups: [
      { id: 'GroupDocumentViews', label: 'Document Views', priority: 'primary', controls: 5, inScope: true, commands: wordViewDocumentViews },
      { id: 'GroupModes', label: 'Modes', priority: 'standard', controls: 3, inScope: true, commands: wordViewModes },
      { id: 'GroupViewShowHide', label: 'Show', priority: 'standard', controls: 4, inScope: true, commands: wordViewShow },
      { id: 'GroupZoom', label: 'Zoom', priority: 'standard', controls: 6, inScope: true, commands: wordViewZoom },
      { id: 'GroupWindow', label: 'Window', priority: 'standard', controls: 8, inScope: true, commands: wordViewWindow },
      { id: 'GroupPageMovement', label: 'Page Movement', priority: 'ancillary', controls: 2, inScope: true, commands: wordViewPageMovement },
      { id: 'GroupNightMode', label: 'Night Mode', priority: 'ancillary', controls: 1, inScope: true, commands: wordViewNightMode },
    ],
  },
  {
    id: 'outlining',
    label: 'Outlining',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabOutlining' },
    groups: [
      { id: 'GroupOutliningTools', label: 'Outlining Tools', priority: 'primary', controls: 12, inScope: true, commands: wordOutliningOutlineTools },
      { id: 'GroupMasterDocument', label: 'Master Document', priority: 'standard', controls: 8, inScope: true, commands: wordOutliningMasterDocument },
      { id: 'GroupOutliningClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true, commands: wordOutliningClose },
    ],
  },
  {
    id: 'print-preview',
    label: 'Print Preview',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabPrintPreview' },
    groups: [
      { id: 'GroupPrintPreviewPrint', label: 'Print', priority: 'secondary', controls: 2, inScope: true, commands: wordPrintPreviewPrint },
      { id: 'GroupPrintPreviewPageSetup', label: 'Page Setup', priority: 'standard', controls: 6, inScope: true, commands: wordPrintPreviewPageSetup },
      { id: 'GroupZoom', label: 'Zoom', priority: 'standard', controls: 6, inScope: true, commands: wordPrintPreviewZoom },
      { id: 'GroupPrintPreviewPreview', label: 'Preview', priority: 'primary', controls: 6, inScope: true, commands: wordPrintPreviewPreview },
    ],
  },
  {
    id: 'background-removal',
    label: 'Background Removal',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabBackgroundRemoval' },
    groups: [
      { id: 'GroupBackgroundRemovalMode', label: 'Refine', priority: 'primary', controls: 3, inScope: true, commands: backgroundRemovalRefineCommands('word') },
      { id: 'GroupBackgroundRemovalClose', label: 'Close', priority: 'secondary', controls: 2, inScope: true, commands: backgroundRemovalCloseCommands('word') },
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
      { id: 'GroupClipboard', label: 'Clipboard', priority: 'secondary', controls: 10, inScope: true, commands: powerpointHomeClipboardCommands('home') },
      { id: 'GroupSlides', label: 'Slides', priority: 'standard', controls: 16, inScope: true, commands: powerpointHomeSlides },
      { id: 'GroupFont', label: 'Font', priority: 'primary', controls: 18, inScope: true, commands: powerpointHomeFontCommands('home') },
      { id: 'GroupParagraph', label: 'Paragraph', priority: 'primary', controls: 27, inScope: true, commands: powerpointHomeParagraphCommands('home') },
      { id: 'GroupDrawing', label: 'Drawing', priority: 'standard', controls: 63, inScope: true, commands: powerpointHomeDrawingCommands('home') },
      { id: 'GroupEditing', label: 'Editing', priority: 'ancillary', controls: 8, inScope: true, commands: powerpointHomeEditingCommands('home') },
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
      { id: 'GroupPreview', label: 'Preview', priority: 'standard', controls: 3, inScope: true, commands: powerpointAnimationsPreview },
      { id: 'GroupAnimations', label: 'Animations', priority: 'primary', controls: 12, inScope: true, commands: powerpointAnimationsAnimations },
      { id: 'GroupAnimationCustom', label: 'Custom Animation', priority: 'primary', controls: 11, inScope: true, commands: powerpointAnimationsCustomAnimation },
      { id: 'GroupAnimationTiming', label: 'Timing', priority: 'standard', controls: 6, inScope: true, commands: powerpointAnimationsTiming },
    ],
  },
  {
    id: 'slide-show',
    label: 'Slide Show',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabSlideShow' },
    groups: [
      { id: 'GroupSlideShowStart', label: 'Start Slide Show', priority: 'primary', controls: 9, inScope: true, commands: powerpointSlideShowStart },
      { id: 'GroupSlideShowSetup', label: 'Set Up', priority: 'primary', controls: 16, inScope: true, commands: powerpointSlideShowSetUp },
      { id: 'GroupRehearse', label: 'Rehearse', priority: 'ancillary', controls: 1, inScope: true, commands: powerpointSlideShowRehearse },
      { id: 'GroupMonitors', label: 'Monitors', priority: 'secondary', controls: 2, inScope: true, commands: powerpointSlideShowMonitors },
    ],
  },
  {
    id: 'recording',
    label: 'Recording',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabRecording' },
    groups: [
      { id: 'GroupRecord', label: 'Record', priority: 'primary', controls: 8, inScope: true, commands: powerpointRecordingRecord },
      { id: 'GroupRecordTabRecord', label: 'Recording', priority: 'standard', controls: 4, inScope: true, commands: powerpointRecordingRecording },
      { id: 'GroupContentRecording', label: 'Content', priority: 'standard', controls: 3, inScope: true, commands: powerpointRecordingContent },
      { id: 'GroupChunkCameoCamera', label: 'Camera', priority: 'standard', controls: 3, inScope: true, commands: powerpointRecordingCamera },
      { id: 'GroupAutoPlayMediaRecording', label: 'Auto-play Media', priority: 'standard', controls: 5, inScope: true, commands: powerpointRecordingAutoPlayMedia },
      { id: 'GroupEditTabRecord', label: 'Edit', priority: 'standard', controls: 6, inScope: true, commands: powerpointRecordingEdit },
      { id: 'GroupSaveRecording', label: 'Save', priority: 'standard', controls: 3, inScope: true, commands: powerpointRecordingSave },
      { id: 'GroupExportTabRecord', label: 'Export', priority: 'ancillary', controls: 2, inScope: true, commands: powerpointRecordingExport },
      { id: 'GroupPreviewTabRecord', label: 'Preview', priority: 'ancillary', controls: 1, inScope: true, commands: powerpointRecordingPreview },
      { id: 'GroupHelpTabRecord', label: 'Help', priority: 'ancillary', controls: 1, inScope: true, commands: powerpointRecordingHelp },
    ],
  },
  {
    id: 'review',
    label: 'Review',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabReview' },
    groups: [
      { id: 'GroupProofing', label: 'Proofing', priority: 'secondary', controls: 2, inScope: true, commands: powerpointReviewProofing },
      { id: 'GroupAccessibility', label: 'Accessibility', priority: 'standard', controls: 6, inScope: true, commands: powerpointReviewAccessibility },
      { id: 'GroupLanguage', label: 'Language', priority: 'standard', controls: 9, inScope: true, commands: powerpointReviewLanguage },
      { id: 'GroupComments', label: 'Comments', priority: 'primary', controls: 12, inScope: true, commands: powerpointReviewComments },
      { id: 'GroupInk', label: 'Ink', priority: 'standard', controls: 5, inScope: true, commands: powerpointReviewInk },
      { id: 'GroupReviewCompare', label: 'Compare', priority: 'primary', controls: 13, inScope: true, commands: powerpointReviewCompare },
      { id: 'GroupActivity', label: 'Activity', priority: 'secondary', controls: 2, inScope: true, commands: powerpointReviewActivity },
    ],
  },
  {
    id: 'view',
    label: 'View',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabView' },
    groups: [
      { id: 'GroupPresentationViews', label: 'Presentation Views', priority: 'primary', controls: 5, inScope: true, commands: powerpointViewPresentationViews },
      { id: 'GroupMasterViews', label: 'Master Views', priority: 'standard', controls: 3, inScope: true, commands: powerpointViewMasterViews },
      { id: 'GroupViewShowHide', label: 'Show', priority: 'standard', controls: 5, inScope: true, commands: powerpointViewShow },
      { id: 'GroupZoom', label: 'Zoom', priority: 'secondary', controls: 2, inScope: true, commands: powerpointViewZoom },
      { id: 'GroupColorGrayscale', label: 'Colour/Greyscale', priority: 'standard', controls: 4, inScope: true, commands: powerpointViewColourGreyscale },
      { id: 'GroupWindow', label: 'Window', priority: 'standard', controls: 6, inScope: true, commands: powerpointViewWindow },
      { id: 'GroupViewDirection', label: 'View Direction', priority: 'standard', controls: 3, inScope: true, commands: powerpointViewViewDirection },
    ],
  },
  {
    id: 'slide-master',
    label: 'Slide Master',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabSlideMaster' },
    groups: [
      { id: 'GroupMasterEdit', label: 'Edit Master', priority: 'standard', controls: 5, inScope: true, commands: powerpointSlideMasterEditMaster },
      { id: 'GroupMasterLayout', label: 'Master Layout', priority: 'primary', controls: 15, inScope: true, commands: powerpointSlideMasterMasterLayout },
      { id: 'GroupMasterEditTheme', label: 'Edit Theme', priority: 'standard', controls: 4, inScope: true, commands: masterEditThemeCommands('slide-master') },
      { id: 'GroupBackground', label: 'Background', priority: 'standard', controls: 11, inScope: true, commands: masterBackgroundCommands('slide-master') },
      { id: 'GroupSlideSize', label: 'Size', priority: 'secondary', controls: 2, inScope: true, commands: powerpointSlideMasterSize },
      { id: 'GroupMasterClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true, commands: masterCloseCommands('slide-master') },
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
      { id: 'GroupClipboard', label: 'Clipboard', priority: 'secondary', controls: 10, inScope: true, commands: powerpointHomeClipboardCommands('slide-master-home') },
      { id: 'GroupMasterSlides', label: 'Master Slides', priority: 'standard', controls: 9, inScope: true, commands: powerpointSlideMasterHomeMasterSlides },
      { id: 'GroupFont', label: 'Font', priority: 'primary', controls: 18, inScope: true, commands: powerpointHomeFontCommands('slide-master-home') },
      { id: 'GroupParagraph', label: 'Paragraph', priority: 'primary', controls: 27, inScope: true, commands: powerpointHomeParagraphCommands('slide-master-home') },
      { id: 'GroupDrawing', label: 'Drawing', priority: 'standard', controls: 63, inScope: true, commands: powerpointHomeDrawingCommands('slide-master-home') },
      { id: 'GroupEditing', label: 'Editing', priority: 'ancillary', controls: 8, inScope: true, commands: powerpointHomeEditingCommands('slide-master-home') },
    ],
  },
  {
    id: 'handout-master',
    label: 'Handout Master',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabHandoutMaster' },
    groups: [
      { id: 'GroupPageSetupHandoutMaster', label: 'Page Setup', priority: 'primary', controls: 11, inScope: true, commands: powerpointHandoutMasterPageSetup },
      { id: 'GroupPlaceholdersHandoutMaster', label: 'Placeholders', priority: 'standard', controls: 4, inScope: true, commands: powerpointHandoutMasterPlaceholders },
      { id: 'GroupMasterEditTheme', label: 'Edit Theme', priority: 'standard', controls: 4, inScope: true, commands: masterEditThemeCommands('handout-master') },
      { id: 'GroupBackground', label: 'Background', priority: 'standard', controls: 11, inScope: true, commands: masterBackgroundCommands('handout-master') },
      { id: 'GroupMasterClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true, commands: masterCloseCommands('handout-master') },
    ],
  },
  {
    id: 'notes-master',
    label: 'Notes Master',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabNotesMaster' },
    groups: [
      { id: 'GroupPageSetupNotesMaster', label: 'Page Setup', priority: 'standard', controls: 3, inScope: true, commands: powerpointNotesMasterPageSetup },
      { id: 'GroupPlaceholdersNotesMaster', label: 'Placeholders', priority: 'standard', controls: 6, inScope: true, commands: powerpointNotesMasterPlaceholders },
      { id: 'GroupMasterEditTheme', label: 'Edit Theme', priority: 'standard', controls: 4, inScope: true, commands: masterEditThemeCommands('notes-master') },
      { id: 'GroupBackground', label: 'Background', priority: 'primary', controls: 11, inScope: true, commands: masterBackgroundCommands('notes-master') },
      { id: 'GroupMasterClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true, commands: masterCloseCommands('notes-master') },
    ],
  },
  {
    id: 'black-and-white',
    label: 'Black and White',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabBlackAndWhite' },
    groups: [
      { id: 'GroupColorModeSetting', label: 'Colour Mode', priority: 'primary', controls: 10, inScope: true, commands: colourModeSettingCommands('black-and-white') },
      { id: 'GroupColorModeClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true, commands: colourModeCloseCommands('black-and-white') },
    ],
  },
  {
    id: 'greyscale',
    label: 'Greyscale',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabGrayscale' },
    groups: [
      { id: 'GroupColorModeSetting', label: 'Colour Mode', priority: 'primary', controls: 10, inScope: true, commands: colourModeSettingCommands('greyscale') },
      { id: 'GroupColorModeClose', label: 'Close', priority: 'ancillary', controls: 1, inScope: true, commands: colourModeCloseCommands('greyscale') },
    ],
  },
  {
    id: 'print-preview',
    label: 'Print Preview',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabPrintPreview' },
    groups: [
      { id: 'GroupPrintPreviewPrint', label: 'Print', priority: 'secondary', controls: 2, inScope: true, commands: powerpointPrintPreviewPrint },
      { id: 'GroupPrintPreviewPageSetup', label: 'Page Setup', priority: 'standard', controls: 5, inScope: true, commands: powerpointPrintPreviewPageSetup },
      { id: 'GroupZoom', label: 'Zoom', priority: 'secondary', controls: 2, inScope: true, commands: powerpointPrintPreviewZoom },
      { id: 'GroupPrintPreviewPreview', label: 'Preview', priority: 'primary', controls: 3, inScope: true, commands: powerpointPrintPreviewPreview },
    ],
  },
  {
    id: 'background-removal',
    label: 'Background Removal',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabBackgroundRemoval' },
    groups: [
      { id: 'GroupBackgroundRemovalMode', label: 'Refine', priority: 'primary', controls: 3, inScope: true, commands: backgroundRemovalRefineCommands('powerpoint') },
      { id: 'GroupBackgroundRemovalClose', label: 'Close', priority: 'secondary', controls: 2, inScope: true, commands: backgroundRemovalCloseCommands('powerpoint') },
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
      { id: 'GroupArrange', label: 'Arrange', priority: 'standard', controls: 47, inScope: true, commands: arrangeCommands('excel', 'page-layout') },
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
      { id: 'GroupGetExternalData', label: 'Get External Data', priority: 'standard', controls: 5, inScope: true, commands: excelDataGetExternalData },
      { id: 'GroupDataQueriesAndConnections', label: 'Queries & Connections', priority: 'standard', controls: 9, inScope: true, commands: excelDataQueriesAndConnections },
      { id: 'GroupDataQueriesAndConnectionsWorkbookLinks', label: 'Workbook Links', priority: 'standard', controls: 9, inScope: true, commands: excelDataWorkbookLinks },
      { id: 'GroupConnections', label: 'Connections', priority: 'standard', controls: 9, inScope: true, commands: excelDataConnections },
      { id: 'GroupLinkedEntityConvert', label: 'Data Types', priority: 'ancillary', controls: 2, inScope: true, commands: excelDataDataTypes },
      { id: 'GroupSortFilter', label: 'Sort & Filter', priority: 'primary', controls: 7, inScope: true, commands: excelDataSortFilter },
      { id: 'GroupDataTools', label: 'Data Tools', priority: 'primary', controls: 10, inScope: true, commands: excelDataDataTools },
      { id: 'GroupForecast', label: 'Forecast', priority: 'standard', controls: 5, inScope: true, commands: excelDataForecast },
      { id: 'GroupOutline', label: 'Outline', priority: 'standard', controls: 10, inScope: true, commands: excelDataOutline },
    ],
  },
  {
    id: 'review',
    label: 'Review',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabReview' },
    groups: [
      { id: 'GroupProofing', label: 'Proofing', priority: 'standard', controls: 3, inScope: true, commands: excelReviewProofing },
      { id: 'GroupAccessibility', label: 'Accessibility', priority: 'standard', controls: 9, inScope: true, commands: excelReviewAccessibility },
      { id: 'GroupLanguage', label: 'Language', priority: 'standard', controls: 3, inScope: true, commands: excelReviewLanguage },
      { id: 'GroupThreadedComments', label: 'Threaded Comments', priority: 'primary', controls: 5, inScope: true, commands: excelReviewThreadedComments },
      { id: 'GroupComments', label: 'Comments', priority: 'standard', controls: 6, inScope: true, commands: excelReviewComments },
      { id: 'GroupCommentsLegacy', label: 'Notes', priority: 'standard', controls: 7, inScope: true, commands: excelReviewNotes },
      { id: 'GroupInk', label: 'Ink', priority: 'standard', controls: 5, inScope: true, commands: excelReviewInk },
      { id: 'GroupProtectExcel', label: 'Protect', priority: 'primary', controls: 4, inScope: true, commands: excelReviewProtect },
      { id: 'GroupChangesExcel', label: 'Changes', priority: 'standard', controls: 8, inScope: true, commands: excelReviewChanges },
      { id: 'GroupPerformance', label: 'Performance', priority: 'ancillary', controls: 1, inScope: true, commands: excelReviewPerformance },
      { id: 'GroupDebug', label: 'Debug', priority: 'ancillary', controls: 1, inScope: true, commands: excelReviewDebug },
    ],
  },
  {
    id: 'view',
    label: 'View',
    appearance: 'always',
    source: { kind: 'core', tab: 'TabView' },
    groups: [
      { id: 'GroupWorkbookViews', label: 'Workbook Views', priority: 'primary', controls: 4, inScope: true, commands: excelViewWorkbookViews },
      { id: 'GroupViewShowHide', label: 'Show', priority: 'standard', controls: 8, inScope: true, commands: excelViewShow },
      { id: 'GroupZoom', label: 'Zoom', priority: 'standard', controls: 3, inScope: true, commands: excelViewZoom },
      { id: 'GroupWindow', label: 'Window', priority: 'standard', controls: 10, inScope: true, commands: excelViewWindow },
      { id: 'GroupNamedSheetView', label: 'Sheet View', priority: 'standard', controls: 5, inScope: true, commands: excelViewSheetView },
      { id: 'GroupNightMode', label: 'Night Mode', priority: 'ancillary', controls: 1, inScope: true, commands: excelViewNightMode },
      { id: 'GroupViewDebug', label: 'Debug', priority: 'ancillary', controls: 1, inScope: true, commands: excelViewDebug },
    ],
  },
  {
    id: 'print-preview',
    label: 'Print Preview',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabPrintPreview' },
    groups: [
      { id: 'GroupPrintPreviewPrint', label: 'Print', priority: 'secondary', controls: 2, inScope: true, commands: excelPrintPreviewPrint },
      { id: 'GroupPrintPreviewPreview', label: 'Preview', priority: 'primary', controls: 4, inScope: true, commands: excelPrintPreviewPreview },
      { id: 'GroupPrintPreviewZoom', label: 'Zoom', priority: 'ancillary', controls: 1, inScope: true, commands: excelPrintPreviewZoom },
    ],
  },
  {
    id: 'background-removal',
    label: 'Background Removal',
    appearance: 'view',
    source: { kind: 'core', tab: 'TabBackgroundRemoval' },
    groups: [
      { id: 'GroupBackgroundRemovalMode', label: 'Refine', priority: 'primary', controls: 3, inScope: true, commands: backgroundRemovalRefineCommands('excel') },
      { id: 'GroupBackgroundRemovalClose', label: 'Close', priority: 'secondary', controls: 2, inScope: true, commands: backgroundRemovalCloseCommands('excel') },
    ],
  },
];

/** Every application's tabs, by kebab id. */
export const ribbonCensus: Readonly<Record<RibbonApplication, readonly RibbonTabEntry[]>> = {
  word: wordRibbonTabs,
  powerpoint: powerpointRibbonTabs,
  excel: excelRibbonTabs,
};

// ── the commands Table Design shows ──────────────────────────────────────────
//
// ## Word's Table Design
//
// The unit after the contextual sets were declared, one tab of one application: **Word's `TabTableToolsDesign`, in
// `TabSetTableTools`**, all three in-scope groups and fourteen commands, and **the first contextual tab authored**.
// Office shows it under the *Table Tools* band while the insertion point is in a table: which parts of the table its
// style sets apart, which style it wears, and the pen the Borders group draws borders with.
//
// ## Office's three groups, read onto the census's three
//
// | Census group (count) | Label | What it holds here |
// |---|---|---|
// | `GroupTableLayout` (13) | Table Style Options | Header Row, Total Row, Banded Rows, First Column, Last Column, Banded Columns |
// | `GroupTableStylesWord` (6) | Table Styles | the Table Styles gallery, Shading |
// | `GroupTableBorders` (13) | Borders | Border Styles, Line Style, Line Weight, Pen Colour, Borders, Border Painter; the Borders and Shading launcher |
//
// **Every id, label and priority is the contextual unit's, unchanged.** `GroupTableLayout` is still the misleading
// id: it names a layout and sits on the Design tab, where the only group its position and count fit is Table Style
// Options. `GUESS:` that reading, as the header's item 2 records it.
//
// ## The shapes, decided by what Office's popup is
//
// **Six toggles drawn as checkboxes**, bound by both hosts. **One in-ribbon gallery**, Table Styles, over Word's 105
// built-in styles with three footer commands. **Two colour pickers**, Shading and Pen Colour, over the document's
// palette. **Two fields**, Line Style and Line Weight. **One dropdown**, Border Styles. **One split button**, Borders.
// **One toggle**, Border Painter, the generic toggle, unpressed. **One dialog launcher**, Borders and Shading, on
// Borders. Every list is in `stories/ribbons/table-tools-menus.ts`, **written for PowerPoint's and Excel's Table
// Design to reuse where their entries are Word's**: the table picture, the gallery item builder and the nine line
// weights. **No exclusive set, no split toggle, no survivor.**
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **The counts.** **Table Style Options counts 13 and draws 6.** `GUESS:` no reading this unit found reaches 13 (a
//    box and a label per checkbox is 12), and nothing is padded. **Table Styles counts 6 and draws 2.** `GUESS:` the
//    gallery, its two scroll arrows, its More button, and the two halves of Shading make six. **Borders counts 13 and
//    draws 6 and a launcher.** `GUESS:` the census counts some of the Borders menu; no reading reaches 13 exactly.
// 2. **The six checkboxes are declared down each column**: Header Row, Total Row, Banded Rows, then First Column, Last
//    Column, Banded Columns. That is Office's two columns of three read down each column, as Notes Master's
//    placeholders are. The brief lists them across the rows. `GUESS:` the columns.
// 3. **Header Row, First Column and Banded Rows start ticked**, and the other three unticked. That is the table look
//    Word writes on an inserted table, `w:tblLook w:val="04A0"`: first row, first column, and no vertical banding.
//    `GUESS:` that the checkboxes show exactly that look.
// 4. **The gallery starts on Table Grid**, Insert Table's style, and draws Word's 105 built-in styles by Word's own
//    names: *Plain Tables* (7), *Grid Tables* (49) and *List Tables* (49), seven families each drawn without an accent
//    and in the six accents. **The style names keep Word's spelling**, *Colorful*, where the census writes *Colour*: a
//    style name is data a document carries (`w:style/w:name`). The pictures are drawn in the **document's palette**,
//    so a style tinted by Accent 2 is the document's second accent. `GUESS:` each picture's look, the footer's order
//    (Modify Table Style…, Clear, New Table Style…), and that no *Custom* section shows for a document with none.
// 5. **The census's spelling wins everywhere else**: *Pen Colour*, where Office writes *Pen Color*.
// 6. **Shading and Pen Colour are drawn as the catalogue's colour picker**, a field with a swatch. Office draws Shading
//    as a large split button with a paint bucket, and Pen Color as a small dropdown with a pen, each opening a colour
//    grid. The picker is the catalogue's one colour control and the one that reads a document's palette, so it is
//    used for both, as Home's Font Colour and Design's Page Colour already are. Shading offers *No Colour*, Pen
//    Colour *Automatic*, and both carry **More Colours…** beneath the palette, the picker's slotted entries.
//    `GUESS:` that Word's carry nothing else (no Eyedropper, Picture, Gradient or Texture).
// 7. **Line Style and Line Weight carry names.** Office draws each entry as a picture of the line with no text. The
//    style names are `GUESS:`, from the Borders and Shading dialog, and each value is the style's `ST_Border` token.
//    Line Style starts on Single and Line Weight on ½ pt, a new table's border.
// 8. **Office presses Border Painter itself** once a border style, line style, weight or colour is chosen. Nothing
//    here dispatches a command, so it stays where a person leaves it; that is loop 2's.
//
// ## Survivors: none, and why each group keeps none
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Table Style Options**: none. All six are checkboxes, which the gate refuses.
// - **Table Styles**: none. The gallery and Shading's colour grid both fail rule 1.
// - **Borders**: none. Border Styles opens a menu, Line Style, Line Weight and Pen Colour open lists and Borders is a
//   split button, all rule 1.
//   **Border Painter passes rule 1 and fails rule 2**: unlabelled, its brush is Format Painter's, a different command
//   a person reaches for on Home.
//
// ## Sizes, and every glyph
//
// **Size follows Microsoft 365's shape**: Border Styles, Borders and Border Painter are `large`; Line Style, Line
// Weight and Pen Colour are stacked in a column; the gallery is in-ribbon; the six checkboxes are two columns of three.
// Three commands carry a glyph, and every glyph is `GUESS:`:
//
// - **Border Styles draws `line-style`**, three lines of different dashes, new: a border style is a line's style,
//   weight and colour chosen together. Not `border-all`, which Borders draws beside it.
// - **Borders draws `border-all`**, Home's Borders glyph, now at 24 too: the same command, the same resting face.
// - **Border Painter draws `paint-brush`**, Format Painter's brush, now at 24 and filled while pressed. Office's own
//   Border Painter glyph is a brush over a border. See the survivors on why that shared brush keeps it off the
//   survivor row.
//
// **Eleven commands carry no glyph, and say why**: the six checkboxes draw their tick box; the gallery is its
// pictures; Shading and Pen Colour are colour pickers, which draw a swatch; Line Style and Line Weight are fields.

/**
 * Word's `GroupTableLayout` on Table Design, labelled **Table Style Options**: six checkboxes, two columns of three
 * read down each column. See disagreements 1, 2 and 3.
 *
 * **All six are toggles drawn as checkboxes** a host binds; Header Row, First Column and Banded Rows start ticked.
 *
 * **No survivor**: six checkboxes.
 */
const wordTableDesignTableStyleOptions: readonly RibbonCommand[] = [
  { id: 'word.table-design.table-style-options.header-row', label: 'Header Row', toggle: true, pressed: true },
  { id: 'word.table-design.table-style-options.total-row', label: 'Total Row', toggle: true },
  { id: 'word.table-design.table-style-options.banded-rows', label: 'Banded Rows', toggle: true, pressed: true },
  { id: 'word.table-design.table-style-options.first-column', label: 'First Column', toggle: true, pressed: true },
  { id: 'word.table-design.table-style-options.last-column', label: 'Last Column', toggle: true },
  { id: 'word.table-design.table-style-options.banded-columns', label: 'Banded Columns', toggle: true },
];

/**
 * Word's `GroupTableStylesWord`, labelled **Table Styles**: the gallery and Shading. See disagreements 1, 4 and 6.
 *
 * **The gallery** and **Shading**, a colour picker, carry no glyph. Both are bound by a host.
 *
 * **No survivor**: a gallery and a colour grid.
 */
const wordTableDesignTableStyles: readonly RibbonCommand[] = [
  { id: 'word.table-design.table-styles.gallery', label: 'Table Styles' },
  { id: 'word.table-design.table-styles.shading', label: 'Shading' },
];

/**
 * Word's `GroupTableBorders`, labelled **Borders**, as Microsoft 365 draws it: Border Styles large and first, then Line
 * Style, Line Weight and Pen Colour in a column, then Borders and Border Painter large, and the Borders and Shading
 * launcher the tab module passes. See disagreements 1, 6, 7 and 8.
 *
 * **Border Styles is a large dropdown** a host binds. **Line Style and Line Weight are fields** and **Pen Colour a
 * colour picker**, bound by a host. **Borders is a split button** a host binds. **Border Painter is the generic
 * toggle**, unpressed.
 *
 * **No survivor**: a menu, three lists, a split button, and a brush that reads as Format Painter.
 */
const wordTableDesignBorders: readonly RibbonCommand[] = [
  { id: 'word.table-design.borders.border-styles', label: 'Border Styles', icon: 'line-style', size: 'large' },
  { id: 'word.table-design.borders.line-style', label: 'Line Style' },
  { id: 'word.table-design.borders.line-weight', label: 'Line Weight' },
  { id: 'word.table-design.borders.pen-colour', label: 'Pen Colour' },
  { id: 'word.table-design.borders.borders', label: 'Borders', icon: 'border-all', size: 'large' },
  { id: 'word.table-design.borders.border-painter', label: 'Border Painter', icon: 'paint-brush', size: 'large', toggle: true },
];

// ## PowerPoint's Table Design
//
// The unit after Word's Table Layout, one tab of one application: **PowerPoint's `TabTableToolsDesign`, in
// `TabSetTableTools`**, all four in-scope groups and nineteen commands, and **the third contextual tab authored**.
// Office shows it under the *Table Tools* band while a table on a slide is selected: which parts of the table its
// style sets apart, which style it wears and what effects its cells carry, how the text in it is dressed, and the pen
// Draw Table draws borders with.
//
// ## Office's four groups, read onto the census's four
//
// | Census group (count) | Label | What it holds here |
// |---|---|---|
// | `GroupTableStyleOptionsPowerPoint` (6) | Table Style Options | Header Row, Total Row, Banded Rows, First Column, Last Column, Banded Columns |
// | `GroupTableStylesPowerPoint` (33) | Table Styles | the Table Styles gallery, Shading, Borders, Effects |
// | `GroupTextStylesTable` (33) | WordArt Styles | Quick Styles, Text Fill, Text Outline, Text Effects; the Format Text Effects launcher |
// | `GroupDrawBorders` (6) | Draw Borders | Pen Style, Pen Weight, Pen Colour, Draw Table, Eraser; the Format Shape launcher |
//
// **Every id, label and priority is the contextual unit's, unchanged**, and every group's identity is plain from its id
// and count except `GroupTextStylesTable`, whose label the contextual unit already read as WordArt Styles (its header's
// item 2). Table Style Options is the one group whose count matches what is drawn (6).
//
// ## The shapes, decided by what Office's popup is
//
// **Six toggles drawn as checkboxes**, bound by a host. **Two in-ribbon galleries**: Table Styles, over PowerPoint's
// 74 built-in table styles, and Quick Styles, over the twenty WordArt styles. **Four colour pickers**: Shading, Text
// Fill, Text Outline and Pen Colour, over the document's palette. **Two fields**, Pen Style and Pen Weight. **Two
// dropdowns**, Effects and Text Effects, each a menu of submenus. **One split button**, Borders. **Two toggles in one
// exclusive set that may hold none**, Draw Table and Eraser. **Two dialog launchers**, Format Text Effects on WordArt
// Styles and Format Shape on Draw Borders.
//
// Where PowerPoint's entries are Word's, the lists are Word's: the table picture and gallery item builder and the nine
// line weights, in `stories/ribbons/table-tools-menus.ts`, which now also carries PowerPoint's own lists (the 74
// styles, Pen Style, Borders' twelve entries). **WordArt Styles is written once**, as `wordArtStylesCommands`, a
// function of the application and the tab, because Shape Format and Chart Format repeat it in all three
// applications; its gallery and its effect menus are in `stories/ribbons/wordart-styles-menus.ts`, and Table Design's
// Effects menu reuses that file's Bevel, Shadow and Reflection lists.
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **The counts.** **Table Styles counts 33 and draws 4.** `GUESS:` no reading this unit found reaches 33: the
//    gallery, its two scroll arrows and its More button, the two halves of Shading and of Borders, and Effects make 9.
//    **WordArt Styles counts 33 and draws 4 and a launcher.** `GUESS:` the same: the gallery's four parts, the two
//    halves of Text Fill and of Text Outline, Text Effects and the launcher make 10. **Draw Borders counts 6 and draws
//    5 and a launcher**, which is the reading that makes 6 and the reason the launcher is drawn. `GUESS:` that
//    reading. Nothing is padded.
// 2. **The six checkboxes are declared down each column**, as Word's are: Header Row, Total Row, Banded Rows, then
//    First Column, Last Column, Banded Columns. The brief lists them across the rows (Header Row, First Column, …).
//    `GUESS:` the columns.
// 3. **Header Row and Banded Rows start ticked**, and the other four unticked. That is the look PowerPoint writes on
//    an inserted table, `<a:tblPr firstRow="1" bandRow="1">`, and it differs from Word's, which also sets the first
//    column apart. `GUESS:` that the checkboxes show exactly that look.
// 4. **The gallery starts on Medium Style 2 - Accent 1**, the style PowerPoint inserts a table in
//    (`{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}`), and draws PowerPoint's 74 built-in styles by their own names in the
//    brief's four sections. **Best Match for Document holds the fourteen *Themed* and *No Style* styles**, the ones
//    drawn only from the document's theme, and **Light begins at Light Style 1**. Office may repeat styles under Best
//    Match; a gallery that listed one value twice would select two cells, so no style is listed twice here. `GUESS:`
//    that reading of Best Match, every picture, the order inside each section, and the footer (Clear Table).
// 5. **The census's spelling wins**: *Pen Colour*, where Office writes *Pen Color*, and *colour* in the WordArt style
//    names, as Insert's WordArt menu already spells them. Style names that are document data keep PowerPoint's
//    spelling; none of the 74 has a *colour* in it.
// 6. **Shading, Text Fill, Text Outline and Pen Colour are drawn as the catalogue's colour picker**, as Word's Shading
//    and Pen Colour are. Office draws each as a small split button or dropdown with a coloured bar, opening a colour
//    grid with entries beneath it, which the picker carries as a slotted menu (`stories/ribbons/colour-picker-entries.ts`):
//    Eyedropper and More Colours on all four; Picture, Gradient, Texture and Table Background on Shading; Picture,
//    Gradient and Texture on Text Fill; Weight, Sketched and Dashes on Text Outline. Shading and Text Fill name the
//    picker's *none* chip *No Fill*, Text Outline *No Outline*; Pen Colour starts on Text 1. `GUESS:` which entries
//    each carries and every label. **Table Background's own colour grid is not drawn**: a menu holds commands, not
//    swatches, so its submenu lists No Fill, More Fill Colours…, Eyedropper and Picture… alone.
// 7. **Pen Style and Pen Weight carry names.** Office draws each entry as a picture of the line. The style names are
//    PowerPoint's dash names and each value is its `ST_PresetLineDashVal` token, except *No Border*, which is not a
//    dash (`none`). Pen Style starts on Solid and Pen Weight on 1 pt, an inserted table's border. `GUESS:` both starts.
// 8. **Draw Table and Eraser are one exclusive set that may hold none**, `powerpoint.table-design.draw-borders.tools`,
//    neither pressed, as Word's pair on Table Layout is. **Office presses Draw Table itself** once a pen style, weight
//    or colour is chosen; nothing here dispatches, so it stays where a person leaves it.
// 9. **Effects and Text Effects are menus of submenus**, Office's shape: Cell Bevel, Shadow and Reflection; and
//    Shadow, Reflection, Glow, Bevel, 3-D Rotation and Transform. Each submenu carries Office's whole preset list and
//    its Options entry, which opens the Format Shape pane in Office and nothing here. `GUESS:` every preset's name, and
//    that a table cell's Shadow list is the same as text's.
// 10. **The WordArt style names are the Office theme's** (*Blue, Accent colour 1*), as Insert's WordArt menu names
//    them. Office names each from the document's theme colours; the pictures are drawn in the document's palette.
//    `GUESS:` the twenty names, their order, and the footer (Clear WordArt).
//
// ## Survivors: none, and why each group keeps none
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Table Style Options**: none. All six are checkboxes, which the gate refuses.
// - **Table Styles**: none. The gallery and Shading open a gallery and a colour grid, Borders is a split button and
//   Effects a menu, all rule 1.
// - **WordArt Styles**: none. Quick Styles is a gallery, Text Fill and Text Outline open colour grids and Text Effects
//   a menu, all rule 1.
// - **Draw Borders**: none. Pen Style, Pen Weight and Pen Colour open lists, rule 1, and Draw Table and Eraser arm a
//   gesture rather than doing one thing in one press, as on Word's Table Layout.
//
// ## Sizes, and every glyph
//
// **Size follows Microsoft 365's shape**: the six checkboxes are two columns of three; the Table Styles gallery is
// in-ribbon with Shading, Borders and Effects small in a column; the Quick Styles gallery is in-ribbon with Text Fill,
// Text Outline and Text Effects small in a column; Pen Style, Pen Weight and Pen Colour are stacked in a column, then
// Draw Table and Eraser large. **Five commands carry a glyph, every one reused and every one `GUESS:`**:
//
// - **Borders draws `border-all`**, Home's Borders and Word's Table Design Borders, small.
// - **Effects draws `square-shadow`**, Home's Shape Effects: a shape with a shadow, one of the three effects the menu
//   offers, the same idea applied to cells.
// - **Text Effects draws `text-effects`**, Word's Text Effects and Typography and Insert's WordArt: an outlined,
//   shadowed letter, which is what the menu makes.
// - **Draw Table draws `table-edit`** and **Eraser draws `eraser`**, Word's Table Layout's pair, large, filled while
//   pressed.
//
// **Fourteen commands carry no glyph, and say why**: the six checkboxes draw their tick box; the two galleries are
// their pictures; Shading, Text Fill, Text Outline and Pen Colour are colour pickers, which draw a swatch; Pen Style
// and Pen Weight are fields.

/**
 * **WordArt Styles, as Office draws it on PowerPoint's Table Design and on every Shape Format and Chart Format**:
 * Quick Styles, Text Fill, Text Outline and Text Effects, under `<application>.<tab>.wordart-styles`.
 *
 * Written once because the group repeats: `GroupTextStylesTable` here, `GroupWordArtStyles` on the Drawing Tools and
 * Chart Tools tabs of all three applications. The census counts Word's and Excel's 30 and PowerPoint's 33; the four
 * commands Office draws are the same four. Each unit that calls it records its own disagreements.
 *
 * **Quick Styles is a gallery**, over `stories/ribbons/wordart-styles-menus.ts`'s styles. **Text Fill and Text Outline
 * are colour pickers**, and **Text Effects is a dropdown** over that file's effect submenus; all four are bound by a
 * host. Text Effects draws `text-effects`.
 *
 * **No survivor**: a gallery, two colour grids and a menu.
 */
export function wordArtStylesCommands(application: RibbonApplication, tab: string): readonly RibbonCommand[] {
  return [
    { id: `${application}.${tab}.wordart-styles.quick-styles`, label: 'Quick Styles' },
    { id: `${application}.${tab}.wordart-styles.text-fill`, label: 'Text Fill' },
    { id: `${application}.${tab}.wordart-styles.text-outline`, label: 'Text Outline' },
    { id: `${application}.${tab}.wordart-styles.text-effects`, label: 'Text Effects', icon: 'text-effects' },
  ];
}

/**
 * PowerPoint's `GroupTableStyleOptionsPowerPoint` on Table Design, labelled **Table Style Options**: six checkboxes,
 * two columns of three read down each column. See disagreements 2 and 3.
 *
 * **All six are toggles drawn as checkboxes** a host binds; Header Row and Banded Rows start ticked.
 *
 * **No survivor**: six checkboxes.
 */
const powerpointTableDesignTableStyleOptions: readonly RibbonCommand[] = [
  { id: 'powerpoint.table-design.table-style-options.header-row', label: 'Header Row', toggle: true, pressed: true },
  { id: 'powerpoint.table-design.table-style-options.total-row', label: 'Total Row', toggle: true },
  { id: 'powerpoint.table-design.table-style-options.banded-rows', label: 'Banded Rows', toggle: true, pressed: true },
  { id: 'powerpoint.table-design.table-style-options.first-column', label: 'First Column', toggle: true },
  { id: 'powerpoint.table-design.table-style-options.last-column', label: 'Last Column', toggle: true },
  { id: 'powerpoint.table-design.table-style-options.banded-columns', label: 'Banded Columns', toggle: true },
];

/**
 * PowerPoint's `GroupTableStylesPowerPoint`, labelled **Table Styles**: the gallery in-ribbon, then Shading, Borders
 * and Effects small in a column. See disagreements 1, 4, 6 and 9.
 *
 * **The gallery** and **Shading**, a colour picker, carry no glyph. **Borders is a split button** and **Effects a
 * dropdown**. All four are bound by a host.
 *
 * **No survivor**: a gallery, a colour grid, a split button and a menu.
 */
const powerpointTableDesignTableStyles: readonly RibbonCommand[] = [
  { id: 'powerpoint.table-design.table-styles.gallery', label: 'Table Styles' },
  { id: 'powerpoint.table-design.table-styles.shading', label: 'Shading' },
  { id: 'powerpoint.table-design.table-styles.borders', label: 'Borders', icon: 'border-all' },
  { id: 'powerpoint.table-design.table-styles.effects', label: 'Effects', icon: 'square-shadow' },
];

/** The set Draw Table and Eraser share on PowerPoint's Table Design. It may hold none. See disagreement 8. */
const powerpointTableDesignDrawTools = 'powerpoint.table-design.draw-borders.tools';

/**
 * PowerPoint's `GroupDrawBorders`, labelled **Draw Borders**: Pen Style, Pen Weight and Pen Colour in a column, then
 * Draw Table and Eraser large, and the Format Shape launcher the tab module passes. See disagreements 1, 5, 6, 7 and 8.
 *
 * **Pen Style and Pen Weight are fields** and **Pen Colour a colour picker**, bound by a host. **Draw Table and
 * Eraser are one exclusive set that may hold none**, neither pressed, both the generic toggle.
 *
 * **No survivor**: three lists and two gestures.
 */
const powerpointTableDesignDrawBorders: readonly RibbonCommand[] = [
  { id: 'powerpoint.table-design.draw-borders.pen-style', label: 'Pen Style' },
  { id: 'powerpoint.table-design.draw-borders.pen-weight', label: 'Pen Weight' },
  { id: 'powerpoint.table-design.draw-borders.pen-colour', label: 'Pen Colour' },
  { id: 'powerpoint.table-design.draw-borders.draw-table', label: 'Draw Table', icon: 'table-edit', size: 'large', toggle: true, exclusive: powerpointTableDesignDrawTools, exclusiveAllowsNone: true },
  { id: 'powerpoint.table-design.draw-borders.eraser', label: 'Eraser', icon: 'eraser', size: 'large', toggle: true, exclusive: powerpointTableDesignDrawTools, exclusiveAllowsNone: true },
];

// ── the commands Table Layout shows ──────────────────────────────────────────
//
// ## Word's Table Layout
//
// The unit after Word's Table Design, one tab of one application: **Word's `TabTableToolsLayout`, in
// `TabSetTableTools`**, all seven in-scope groups and thirty-three commands, and **the second contextual tab
// authored**. Office shows it beside Table Design while the insertion point is in a table: the table's structure
// (which cells, rows and columns it has, and how they are merged), its cells' sizes, where text sits in a cell, and
// the table's data. Its label is Office's *Layout*, and the band and the accessible name *Layout, Table Tools* tell it
// from the core Layout tab.
//
// ## Office's seven groups, read onto the census's seven
//
// | Census group (count) | Label | What it holds here |
// |---|---|---|
// | `GroupTable` (7) | Table | Select, View Gridlines, Properties |
// | `GroupTableDraw` (2) | Draw | Draw Table, Eraser |
// | `GroupTableRowsAndColumns` (10) | Rows & Columns | Delete, Insert Above, Insert Below, Insert Left, Insert Right; the Insert Cells launcher |
// | `GroupTableMerge` (3) | Merge | Merge Cells, Split Cells, Split Table |
// | `GroupTableCellSize` (9) | Cell Size | AutoFit, Height, Width, Distribute Rows, Distribute Columns; the Table Properties launcher |
// | `GroupTableAlignment` (21) | Alignment | the nine cell alignments, Text Direction, Cell Margins |
// | `GroupTableData` (4) | Data | Sort, Repeat Header Rows, Convert to Text, Formula |
//
// **Every id, label and priority is the contextual unit's, unchanged**, and every group's identity is plain from its
// id, so unlike Table Design's `GroupTableLayout` no reading here is a guess. Draw, Merge and Data are the three
// whose counts match what is drawn (2, 3 and 4).
//
// ## The shapes, decided by what Office's popup is
//
// **Three dropdowns** a host binds, each opening its menu from `stories/ribbons/table-tools-menus.ts`: Select, Delete
// and AutoFit. **Two fields**, Height and Width, measure inputs in centimetres. **Thirteen toggles** in two exclusive
// sets and one on its own: Draw Table and Eraser, **one set that may hold none**, neither pressed; the nine cell
// alignments, **one set of exactly one**, Align Top Left pressed; and View Gridlines and Repeat Header Rows, plain
// toggles. **Fifteen plain buttons**, and **two dialog launchers**, Insert Cells on Rows & Columns and Table
// Properties on Cell Size. **No gallery, no split button, no split toggle, no colour picker, no checkbox.**
//
// Select's and Delete's lists are **written for PowerPoint's Table Layout to reuse** (`tableSelectEntries` and
// `tableDeleteEntries`, which take the application); AutoFit's is Word's.
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **The counts.** **Table counts 7 and draws 3.** `GUESS:` the three commands and Select's four entries make 7.
//    **Rows & Columns counts 10 and draws 5 and a launcher.** `GUESS:` the five, Delete's four entries and the
//    launcher make 10. **Cell Size counts 9 and draws 5 and a launcher.** `GUESS:` the five, AutoFit's three entries
//    and the launcher make 9. **Alignment counts 21 and draws 11.** `GUESS:` no reading this unit found reaches 21.
//    Nothing is padded anywhere.
// 2. **The nine alignments are one exclusive set of exactly one**, `word.table-layout.alignment.cell-alignment`.
//    Office presses the alignment the selected cells are in, and a cell is always in one: pressing Align Centre
//    releases Align Top Left, and pressing the one that holds keeps it. **Align Top Left starts pressed**: an inserted
//    table's cells align top (`w:vAlign` absent) and its paragraphs left. `GUESS:` that start.
// 3. **The nine are declared down each column**, as Table Design's checkboxes and Notes Master's placeholders are:
//    Top Left, Centre Left, Bottom Left, then the centre column, then the right. Office draws a grid of three by
//    three; `GUESS:` the column order.
// 4. **The census's spelling wins**: *Align Top Centre*, *Align Centre Left*, *Align Centre*, *Align Centre Right*
//    and *Align Bottom Centre*, where Office writes *Center*.
// 5. **Draw Table and Eraser are one exclusive set that may hold none**, `word.table-layout.draw.tools`, neither
//    pressed. Each arms a gesture: Draw Table turns the pointer into a pencil that draws cell borders, Eraser into an
//    eraser that removes them. Office arms at most one, and pressing the armed one gives back the pointer, as
//    Background Removal's pencils do; the tab has no Select Objects to stand for *no tool*. `GUESS:` that the two
//    release each other, and that neither starts pressed. **Eraser is a plain toggle here**, where Draw's Eraser is a
//    split toggle: this one has no sizes behind it.
// 6. **Text Direction is a plain button**, as Word draws it: each press turns the selected cells' text a quarter
//    further (horizontal, then down, then up). PowerPoint's Text Direction opens a list; Word's does not. `GUESS:`.
// 7. **Height and Width start on 0.5 cm and 3.18 cm**, a row of an inserted five-column table on an A4 page with
//    2.54 cm margins. Office shows the selected cell's size; `GUESS:` both numbers, and the 0.1 cm step.
// 8. **View Gridlines starts pressed**, which Table Design's Borders menu also ticks. `GUESS:` that a new install
//    shows gridlines. The two are one state in Office; nothing here dispatches, so ticking one does not press the
//    other, which is loop 2's.
// 9. **Split Cells, Properties, Sort, Convert to Text, Formula and Cell Margins open dialogs in Office**, and the two
//    launchers do too. They are plain buttons here, because no dialog is wired on this tab, as none was on Table
//    Design. `GUESS:` that Cell Margins opens Table Options, and that Rows & Columns' launcher is Insert Cells.
//
// ## Survivors, judged group by group
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Table: none.** Select opens a menu and Properties a dialog, rule 1. **View Gridlines passes rule 1 and fails
//   rule 2**: unlabelled, its dashed box crossed by solid lines is Inside Borders, a border a person reaches for on
//   Table Design. `GUESS:`.
// - **Draw: none.** Draw Table and Eraser arm a gesture, which is not one press doing one thing: Draw's standard.
// - **Rows & Columns: Insert Above, Insert Below and Insert Right.** Each inserts one row or column in one press, one
//   undo takes it back, and each draws a glyph no other command draws. Delete opens a menu. **Insert Left is the
//   ceiling's cost**: all four pass and only three survive, and a row or column added after the selection is the one
//   reached for most, so Left gives way as Justify does on Home. `GUESS:` which of the four.
// - **Merge: Merge Cells and Split Table.** Both are one press and one undo. Merge Cells' glyph is also Excel's Merge
//   & Centre, the same act in another application, which rule 2's standard does not refuse. Split Cells opens a
//   dialog.
// - **Cell Size: Distribute Rows and Distribute Columns.** One press, one undo, and three equal bars no other command
//   draws. AutoFit opens a menu, and Height and Width are fields.
// - **Alignment: Align Top Left, Align Top Centre and Align Top Right**, the top row. Each is one press and one undo,
//   unlike a view set, whose presses no undo takes back. The nine glyphs are drawn by no other command. **Six pass
//   and give way to the ceiling**, as Justify does; the top row is the vertical alignment an inserted table is in,
//   so it is the row a person changes horizontally. `GUESS:` which three. Text Direction passes too and gives way to
//   the ceiling; Cell Margins opens a dialog.
// - **Data: Repeat Header Rows.** One press, one undo, a table with a loop that no other command draws. Sort, Convert
//   to Text and Formula open dialogs.
//
// ## Sizes, and every glyph
//
// **Size follows Microsoft 365's shape**: Select, View Gridlines and Properties small in a column; Draw Table and
// Eraser large; Delete and Insert Above large, then Insert Below, Left and Right small; Merge's three small; AutoFit
// large, then Height and Width, then the two Distribute commands small; the nine alignments icon-only in a grid, then
// Text Direction and Cell Margins large; Sort large, then Repeat Header Rows, Convert to Text and Formula small.
// **Thirty-one of the thirty-three commands carry a glyph**, every one `GUESS:`:
//
// - **Select draws `table-cursor`**, a table with a pointer, Office's picture. Not `select-all-on`, Home's Select.
// - **View Gridlines draws `border-inside`**, a dashed box with its inside lines: the lines that show where a
//   borderless table's cells are. Filled while pressed.
// - **Properties draws `table-settings`**, a table with a cog, also Excel's Format.
// - **Draw Table draws `table-edit`**, a table with a pencil, and **Eraser draws `eraser`**, Draw's.
// - **Delete draws `table-dismiss`**, the broad table-with-a-cross Excel's Delete draws, now large: the menu deletes
//   cells, columns, rows or the table, so no one of those glyphs is honest on the face.
// - **Insert Above, Below, Left and Right draw `table-stack-above`, `-below`, `-left` and `-right`**, a table with a
//   new line on that side.
// - **Merge Cells draws `table-cells-merge`**, Excel's Merge & Centre, **Split Cells `table-cells-split`** and
//   **Split Table `table-split`**, a table cut in two.
// - **AutoFit draws `arrow-autofit-content`**, content with arrows to its edges. Not `auto-fit-width`, View's Page
//   Width.
// - **Distribute Rows draws `align-space-evenly-vertical`** and **Distribute Columns
//   `align-space-evenly-horizontal`**, three equal bars.
// - **The nine alignments draw `textbox-align-top-left` to `textbox-align-bottom-right`**, a box with two lines where
//   the text sits; Align Centre is `textbox-align-center`. Filled while pressed.
// - **Text Direction draws `text-direction-rotate-90-right`**, PowerPoint's Text Direction, now large.
// - **Cell Margins draws `padding-left`**, an edge, a dashed inner guide and the space between them. Not
//   `document-margins`, which is a page's. The weakest glyph on the tab.
// - **Sort draws `arrow-sort`**, Home's Sort, now large here as on Excel's Data tab.
// - **Repeat Header Rows draws `table-arrow-repeat-all`**, a table with a loop. Not `table-freeze-row`, Excel's
//   Freeze Top Row, which holds a row still rather than repeating it on each page.
// - **Convert to Text draws `table-switch`**, a table with a turn arrow.
// - **Formula draws `math-formula`**, the *fx* of Equation and Insert Function.
//
// **Two commands carry no glyph, and say why**: Height and Width are fields.

/**
 * Word's `GroupTable` on Table Layout, labelled **Table**: Select, View Gridlines and Properties, small in a column.
 * See disagreements 1, 8 and 9.
 *
 * **Select is a dropdown** a host binds. **View Gridlines is the generic toggle**, pressed. **Properties is a plain
 * button**, whose dialog is not wired.
 *
 * **No survivor**: a menu, a dialog, and a glyph that reads as Inside Borders.
 */
const wordTableLayoutTable: readonly RibbonCommand[] = [
  { id: 'word.table-layout.table.select', label: 'Select', icon: 'table-cursor' },
  { id: 'word.table-layout.table.view-gridlines', label: 'View Gridlines', icon: 'border-inside', toggle: true, pressed: true },
  { id: 'word.table-layout.table.properties', label: 'Properties', icon: 'table-settings' },
];

/** The set Draw Table and Eraser share. It may hold none. See disagreement 5. */
const wordTableLayoutDrawTools = 'word.table-layout.draw.tools';

/**
 * Word's `GroupTableDraw`, labelled **Draw**: Draw Table and Eraser, large. See disagreement 5.
 *
 * **One exclusive set that may hold none**, neither pressed, both the generic toggle.
 *
 * **No survivor**: each arms a gesture.
 */
const wordTableLayoutDraw: readonly RibbonCommand[] = [
  { id: 'word.table-layout.draw.draw-table', label: 'Draw Table', icon: 'table-edit', size: 'large', toggle: true, exclusive: wordTableLayoutDrawTools, exclusiveAllowsNone: true },
  { id: 'word.table-layout.draw.eraser', label: 'Eraser', icon: 'eraser', size: 'large', toggle: true, exclusive: wordTableLayoutDrawTools, exclusiveAllowsNone: true },
];

/**
 * Word's `GroupTableRowsAndColumns`, labelled **Rows & Columns**: Delete and Insert Above large, then Insert Below,
 * Insert Left and Insert Right small in a column, and the Insert Cells launcher the tab module passes. See
 * disagreements 1 and 9.
 *
 * **Delete is a large dropdown** a host binds. The four inserts are plain buttons.
 *
 * **Three survivors**: Insert Above, Insert Below and Insert Right. Insert Left is the ceiling's cost.
 */
const wordTableLayoutRowsAndColumns: readonly RibbonCommand[] = [
  { id: 'word.table-layout.rows-and-columns.delete', label: 'Delete', icon: 'table-dismiss', size: 'large' },
  { id: 'word.table-layout.rows-and-columns.insert-above', label: 'Insert Above', icon: 'table-stack-above', size: 'large', essential: true },
  { id: 'word.table-layout.rows-and-columns.insert-below', label: 'Insert Below', icon: 'table-stack-below', essential: true },
  { id: 'word.table-layout.rows-and-columns.insert-left', label: 'Insert Left', icon: 'table-stack-left' },
  { id: 'word.table-layout.rows-and-columns.insert-right', label: 'Insert Right', icon: 'table-stack-right', essential: true },
];

/**
 * Word's `GroupTableMerge`, labelled **Merge**: Merge Cells, Split Cells and Split Table, small in a column. See
 * disagreement 9.
 *
 * **Three plain buttons.** Split Cells' dialog is not wired.
 *
 * **Two survivors**: Merge Cells and Split Table. Split Cells opens a dialog.
 */
const wordTableLayoutMerge: readonly RibbonCommand[] = [
  { id: 'word.table-layout.merge.merge-cells', label: 'Merge Cells', icon: 'table-cells-merge', essential: true },
  { id: 'word.table-layout.merge.split-cells', label: 'Split Cells', icon: 'table-cells-split' },
  { id: 'word.table-layout.merge.split-table', label: 'Split Table', icon: 'table-split', essential: true },
];

/**
 * Word's `GroupTableCellSize`, labelled **Cell Size**: AutoFit large, then Height and Width, then Distribute Rows and
 * Distribute Columns small, and the Table Properties launcher the tab module passes. See disagreements 1 and 7.
 *
 * **AutoFit is a large dropdown** a host binds. **Height and Width are measure fields** a host binds. The two
 * Distribute commands are plain buttons.
 *
 * **Two survivors**: Distribute Rows and Distribute Columns.
 */
const wordTableLayoutCellSize: readonly RibbonCommand[] = [
  { id: 'word.table-layout.cell-size.autofit', label: 'AutoFit', icon: 'arrow-autofit-content', size: 'large' },
  { id: 'word.table-layout.cell-size.height', label: 'Height' },
  { id: 'word.table-layout.cell-size.width', label: 'Width' },
  { id: 'word.table-layout.cell-size.distribute-rows', label: 'Distribute Rows', icon: 'align-space-evenly-vertical', essential: true },
  { id: 'word.table-layout.cell-size.distribute-columns', label: 'Distribute Columns', icon: 'align-space-evenly-horizontal', essential: true },
];

/** The set the nine cell alignments share. It holds exactly one. See disagreement 2. */
const wordTableLayoutCellAlignment = 'word.table-layout.alignment.cell-alignment';

/**
 * Word's `GroupTableAlignment`, labelled **Alignment**: the nine cell alignments icon-only in a grid, declared down
 * each column, then Text Direction and Cell Margins large. See disagreements 1 to 4, 6 and 9.
 *
 * **One exclusive set of exactly one**, the nine generic toggles, Align Top Left pressed. **Text Direction and Cell
 * Margins are plain buttons.**
 *
 * **Three survivors**: the top row, Align Top Left, Align Top Centre and Align Top Right.
 */
const wordTableLayoutAlignment: readonly RibbonCommand[] = [
  { id: 'word.table-layout.alignment.align-top-left', label: 'Align Top Left', icon: 'textbox-align-top-left', size: 'icon', toggle: true, pressed: true, exclusive: wordTableLayoutCellAlignment, essential: true },
  { id: 'word.table-layout.alignment.align-centre-left', label: 'Align Centre Left', icon: 'textbox-align-middle-left', size: 'icon', toggle: true, exclusive: wordTableLayoutCellAlignment },
  { id: 'word.table-layout.alignment.align-bottom-left', label: 'Align Bottom Left', icon: 'textbox-align-bottom-left', size: 'icon', toggle: true, exclusive: wordTableLayoutCellAlignment },
  { id: 'word.table-layout.alignment.align-top-centre', label: 'Align Top Centre', icon: 'textbox-align-top-center', size: 'icon', toggle: true, exclusive: wordTableLayoutCellAlignment, essential: true },
  { id: 'word.table-layout.alignment.align-centre', label: 'Align Centre', icon: 'textbox-align-center', size: 'icon', toggle: true, exclusive: wordTableLayoutCellAlignment },
  { id: 'word.table-layout.alignment.align-bottom-centre', label: 'Align Bottom Centre', icon: 'textbox-align-bottom-center', size: 'icon', toggle: true, exclusive: wordTableLayoutCellAlignment },
  { id: 'word.table-layout.alignment.align-top-right', label: 'Align Top Right', icon: 'textbox-align-top-right', size: 'icon', toggle: true, exclusive: wordTableLayoutCellAlignment, essential: true },
  { id: 'word.table-layout.alignment.align-centre-right', label: 'Align Centre Right', icon: 'textbox-align-middle-right', size: 'icon', toggle: true, exclusive: wordTableLayoutCellAlignment },
  { id: 'word.table-layout.alignment.align-bottom-right', label: 'Align Bottom Right', icon: 'textbox-align-bottom-right', size: 'icon', toggle: true, exclusive: wordTableLayoutCellAlignment },
  { id: 'word.table-layout.alignment.text-direction', label: 'Text Direction', icon: 'text-direction-rotate-90-right', size: 'large' },
  { id: 'word.table-layout.alignment.cell-margins', label: 'Cell Margins', icon: 'padding-left', size: 'large' },
];

/**
 * Word's `GroupTableData`, labelled **Data**: Sort large, then Repeat Header Rows, Convert to Text and Formula small
 * in a column. See disagreement 9.
 *
 * **Repeat Header Rows is the generic toggle**, unpressed. The other three are plain buttons, whose dialogs are not
 * wired.
 *
 * **One survivor**: Repeat Header Rows.
 */
const wordTableLayoutData: readonly RibbonCommand[] = [
  { id: 'word.table-layout.data.sort', label: 'Sort', icon: 'arrow-sort', size: 'large' },
  { id: 'word.table-layout.data.repeat-header-rows', label: 'Repeat Header Rows', icon: 'table-arrow-repeat-all', toggle: true, essential: true },
  { id: 'word.table-layout.data.convert-to-text', label: 'Convert to Text', icon: 'table-switch' },
  { id: 'word.table-layout.data.formula', label: 'Formula', icon: 'math-formula' },
];

// ## PowerPoint's Table Layout
//
// The unit after PowerPoint's Table Design, one tab of one application: **PowerPoint's `TabTableToolsLayout`, in
// `TabSetTableTools`**, all seven in-scope groups and twenty-eight commands, and **the fourth contextual tab
// authored**. Office shows it beside Table Design while a table on a slide is selected: which rows, columns and cells
// the table has, how big its cells and the table are, where text sits in a cell, and where the table sits among the
// slide's other objects. A slide table is a shape on a canvas rather than a run of text, which is why it has Table
// Size and Arrange where Word's has Draw and Data.
//
// ## Office's seven groups, read onto the census's seven
//
// | Census group (count) | Label | What it holds here |
// |---|---|---|
// | `GroupTable` (5) | Table | Select, View Gridlines |
// | `GroupTableRowsAndColumns` (8) | Rows & Columns | Delete, Insert Above, Insert Below, Insert Left, Insert Right |
// | `GroupMerge` (2) | Merge | Merge Cells, Split Cells |
// | `GroupTableCellSize` (4) | Cell Size | Height, Width, Distribute Rows, Distribute Columns |
// | `GroupAlignment` (12) | Alignment | Align Left, Align Centre, Align Right; Align Top, Centre Vertically, Align Bottom; Text Direction, Cell Margins |
// | `GroupTableSize` (3) | Table Size | Height, Width, Lock Aspect Ratio |
// | `GroupArrange` (46) | Arrange | Bring Forward, Send Backward, Selection Pane, Align |
//
// **Every id, label and priority is the contextual unit's, unchanged**, and every group's identity is plain from its
// id and label. `GroupMerge` and `GroupAlignment` are PowerPoint's own ids for what Word calls `GroupTableMerge` and
// `GroupTableAlignment`; `GroupArrange` is the id Word's Layout and Excel's Page Layout use.
//
// ## The shapes, decided by what Office's popup is
//
// **Seven dropdowns** a host binds: Select, Delete, Text Direction, Cell Margins and Align, and **two split buttons**,
// Bring Forward and Send Backward. **Four fields**, the two Height and Width pairs, measure inputs in centimetres.
// **One checkbox**, Lock Aspect Ratio. **Eight toggles**: the six alignments in **two exclusive sets of exactly one**,
// and View Gridlines and Selection Pane on their own. **Seven plain buttons**: the four inserts, Merge Cells, Split
// Cells and the two Distribute commands. **No gallery, no colour picker, no dialog launcher.**
//
// **Reused, not rewritten**: Select's and Delete's lists are Word's Table Layout's `tableSelectEntries` and
// `tableDeleteEntries`, called for PowerPoint. **Arrange is `arrangeCommands`**, which this unit generalised from Word
// and Excel to take the application, the tab and the object, so Picture, Shape and Chart Format can call it; for a
// table it drops Group and Rotate. Its Bring Forward, Send Backward and Align menus reuse
// `stories/ribbons/design-layout-menus.ts`' Arrange lists, which now take PowerPoint too. Text Direction's and Cell
// Margins' lists are PowerPoint's own, in `stories/ribbons/table-tools-menus.ts`.
//
// ## ⚠ Where the census, the brief and Office disagree, recorded rather than smoothed over
//
// 1. **The counts.** Five of the seven are met by a reading, and each is `GUESS:`. **Table counts 5**: Select, its
//    three entries and View Gridlines. **Rows & Columns counts 8**: Delete, its three entries and the four inserts.
//    **Merge counts 2**, **Cell Size 4** and **Table Size 3**, what is drawn. Those five readings are the census's
//    evidence for the two shared lists, which Word's unit wrote as `GUESS:` for PowerPoint: three entries each, no
//    cell. **Alignment counts 12 and draws 8.** `GUESS:` the eight and Cell Margins' four presets make 12; Text
//    Direction's four directions would make 12 too, so which list the census counts is not known. **Arrange counts 46
//    and draws 4.** `GUESS:` no reading this unit found reaches 46: the census appears to count the whole Arrange
//    menu of a drawing (Group, Rotate and every alignment and order entry), which a table does not carry. Nothing is
//    padded anywhere.
// 2. **Horizontal and vertical alignment are two exclusive sets of exactly one**,
//    `powerpoint.table-layout.alignment.horizontal` and `powerpoint.table-layout.alignment.vertical`. Office presses
//    one of each for the selected cells: pressing Align Centre releases Align Left and leaves Align Top pressed.
//    **Align Left and Align Top start pressed**: an inserted table's paragraphs write no `algn` and its cells no
//    `anchor`, whose defaults are left and top. `GUESS:` both starts.
// 3. **The six are declared across Office's two rows**, the horizontal three and then the vertical three. Office draws
//    them as two rows of three, icons only, beside a large Text Direction and Cell Margins. `GUESS:` that the order
//    reads as Office's grid at every presentation; the ribbon stacks small commands into columns.
// 4. **The census's spelling wins**: *Align Centre* and *Centre Vertically*, where Office writes *Center*.
// 5. **Text Direction is a dropdown**, PowerPoint's shape, where Word's is a plain button: Horizontal, Rotate all text
//    90°, Rotate all text 270° and Stacked, Horizontal checked, then More Options…. `GUESS:` the list and its names.
// 6. **Cell Margins is a dropdown**, where Word's opens a dialog: Normal (checked), None, Narrow and Wide, each with its
//    four measures as a second line, then Custom Margins…. `GUESS:` the measures and that the current preset is ticked.
// 7. **The two Height and Width pairs start on 1.02 cm by 5.84 cm and 2.04 cm by 29.21 cm**: a five-by-two table in the
//    widescreen Office theme's content placeholder. The cell pair and the table pair describe one table and are one
//    state in Office; nothing dispatches, so changing one does not move the other. `GUESS:` every number and the 0.01
//    cm step. **Lock Aspect Ratio starts unticked**, `GUESS:`.
// 8. **View Gridlines starts pressed**, as on Word's Table Layout. `GUESS:` that PowerPoint shows a borderless table's
//    gridlines by default.
// 9. **Arrange has no Group and no Rotate**, which the census's `arrangeCommands` draws for a drawing: PowerPoint neither
//    groups nor rotates a table. **Bring Forward and Send Backward are split buttons** whose arrows add Bring to Front
//    and Send to Back, and **Align** lists the six alignments, the two distributions, then Align to Slide (ticked, as
//    for one selected object) and Align Selected Objects. `GUESS:` PowerPoint's Align entries and the tick.
// 10. **Split Cells, More Options… and Custom Margins… open dialogs in Office**, and Selection Pane opens a pane. They
//    open nothing here: no dialog is wired on the Table Tools tabs.
// 11. **Sizes where PowerPoint differs from Word**: Select and View Gridlines are large (Word's are small), and so are
//    Merge Cells and Split Cells, the only two commands in their group. `GUESS:` all four. **No group has a dialog
//    launcher**, `GUESS:`.
//
// ## Survivors, judged group by group
//
// A survivor passes **all four** of `demotionRules`, judged on the shape Office draws.
//
// - **Table: none.** Select opens a menu, rule 1. **View Gridlines passes rule 1 and fails rule 2**, as on Word's: its
//   dashed box crossed by lines reads as Inside Borders. `GUESS:`.
// - **Rows & Columns: Insert Above, Insert Below and Insert Right**, Word's three, for Word's reason: each inserts in one
//   press and one undo takes it back, each glyph is its own, and **Insert Left is the ceiling's cost**. Delete opens a
//   menu. `GUESS:` which three.
// - **Merge: Merge Cells.** One press, one undo, and the glyph is Merge & Centre's, the same act in another
//   application, which rule 2 does not refuse. Split Cells opens a dialog.
// - **Cell Size: Distribute Rows and Distribute Columns.** One press, one undo, three equal bars no other command
//   draws. Height and Width are fields.
// - **Alignment: Align Left, Align Centre and Align Right**, the horizontal set. Each is one press and one undo, and
//   each glyph is Home's paragraph alignment, which is the same act on the cell's text. **The vertical three pass and
//   give way to the ceiling**: a cell's text is aligned across far more often than up and down, as Home's Paragraph
//   keeps its horizontal alignments. `GUESS:` which three. Text Direction and Cell Margins open menus.
// - **Table Size: none.** Two fields and a checkbox, which the gate refuses as bound.
// - **Arrange: none.** Two split buttons and a menu, rule 1, and Selection Pane opens a pane and carries no glyph.
//
// ## Sizes, and every glyph
//
// **Size follows Microsoft 365's shape**: Select and View Gridlines large; Delete and Insert Above large, then Insert
// Below, Left and Right small; Merge Cells and Split Cells large; Height and Width, then the two Distribute commands
// small; the six alignments icon-only, then Text Direction and Cell Margins large; Height, Width and Lock Aspect
// Ratio in a column; Bring Forward, Send Backward, Selection Pane and Align small. **Twenty-two of the twenty-eight
// commands carry a glyph**, every one reused and every one `GUESS:`:
//
// - **Select draws `table-cursor`** and **View Gridlines `border-inside`**, Word's Table Layout's, now at 24 too.
// - **Delete draws `table-dismiss`** and **Insert Above, Below, Left and Right `table-stack-above`, `-below`, `-left`
//   and `-right`**, Word's Table Layout's, at the same sizes.
// - **Merge Cells draws `table-cells-merge`** and **Split Cells `table-cells-split`**, Word's, now at 24 too.
// - **Distribute Rows draws `align-space-evenly-vertical`** and **Distribute Columns `align-space-evenly-horizontal`**,
//   Word's.
// - **Align Left, Align Centre and Align Right draw `text-align-left`, `text-align-center` and `text-align-right`**,
//   Home's paragraph alignment, filled while pressed.
// - **Align Top, Centre Vertically and Align Bottom draw `align-top`, `align-center-vertical` and `align-bottom`**,
//   Excel's Home vertical alignment; `align-center-vertical` is also PowerPoint's Home Align Text, the same act.
//   Filled while pressed.
// - **Text Direction draws `text-direction-rotate-90-right`** and **Cell Margins `padding-left`**, Word's Table
//   Layout's, large. **Cell Margins' `padding-left` is still the weakest glyph on the tab.**
// - **Bring Forward, Send Backward and Align draw `position-forward`, `position-backward` and `align-left`**,
//   `arrangeCommands`' own.
//
// **Six commands carry no glyph, and say why**: the four Height and Width fields and the Lock Aspect Ratio checkbox,
// and **Selection Pane**, for `arrangeCommands`' reason: Fluent draws no selection pane, and `panel-right` is any pane.

/**
 * PowerPoint's `GroupTable` on Table Layout, labelled **Table**: Select and View Gridlines, large. See disagreements 1,
 * 8 and 11.
 *
 * **Select is a large dropdown** a host binds. **View Gridlines is the generic toggle**, pressed.
 *
 * **No survivor**: a menu, and a glyph that reads as Inside Borders.
 */
const powerpointTableLayoutTable: readonly RibbonCommand[] = [
  { id: 'powerpoint.table-layout.table.select', label: 'Select', icon: 'table-cursor', size: 'large' },
  { id: 'powerpoint.table-layout.table.view-gridlines', label: 'View Gridlines', icon: 'border-inside', size: 'large', toggle: true, pressed: true },
];

/**
 * PowerPoint's `GroupTableRowsAndColumns`, labelled **Rows & Columns**: Delete and Insert Above large, then Insert
 * Below, Insert Left and Insert Right small in a column. See disagreement 1.
 *
 * **Delete is a large dropdown** a host binds, over Word's shared list called for PowerPoint. The four inserts are
 * plain buttons.
 *
 * **Three survivors**: Insert Above, Insert Below and Insert Right. Insert Left is the ceiling's cost.
 */
const powerpointTableLayoutRowsAndColumns: readonly RibbonCommand[] = [
  { id: 'powerpoint.table-layout.rows-and-columns.delete', label: 'Delete', icon: 'table-dismiss', size: 'large' },
  { id: 'powerpoint.table-layout.rows-and-columns.insert-above', label: 'Insert Above', icon: 'table-stack-above', size: 'large', essential: true },
  { id: 'powerpoint.table-layout.rows-and-columns.insert-below', label: 'Insert Below', icon: 'table-stack-below', essential: true },
  { id: 'powerpoint.table-layout.rows-and-columns.insert-left', label: 'Insert Left', icon: 'table-stack-left' },
  { id: 'powerpoint.table-layout.rows-and-columns.insert-right', label: 'Insert Right', icon: 'table-stack-right', essential: true },
];

/**
 * PowerPoint's `GroupMerge`, labelled **Merge**: Merge Cells and Split Cells, large. See disagreements 10 and 11.
 *
 * **Two plain buttons.** Split Cells' dialog is not wired.
 *
 * **One survivor**: Merge Cells. Split Cells opens a dialog.
 */
const powerpointTableLayoutMerge: readonly RibbonCommand[] = [
  { id: 'powerpoint.table-layout.merge.merge-cells', label: 'Merge Cells', icon: 'table-cells-merge', size: 'large', essential: true },
  { id: 'powerpoint.table-layout.merge.split-cells', label: 'Split Cells', icon: 'table-cells-split', size: 'large' },
];

/**
 * PowerPoint's `GroupTableCellSize`, labelled **Cell Size**: Height and Width, then Distribute Rows and Distribute
 * Columns small. See disagreement 7.
 *
 * **Height and Width are measure fields** a host binds. The two Distribute commands are plain buttons.
 *
 * **Two survivors**: Distribute Rows and Distribute Columns.
 */
const powerpointTableLayoutCellSize: readonly RibbonCommand[] = [
  { id: 'powerpoint.table-layout.cell-size.height', label: 'Height' },
  { id: 'powerpoint.table-layout.cell-size.width', label: 'Width' },
  { id: 'powerpoint.table-layout.cell-size.distribute-rows', label: 'Distribute Rows', icon: 'align-space-evenly-vertical', essential: true },
  { id: 'powerpoint.table-layout.cell-size.distribute-columns', label: 'Distribute Columns', icon: 'align-space-evenly-horizontal', essential: true },
];

/** The set the three horizontal alignments share. It holds exactly one. See disagreement 2. */
const powerpointTableLayoutHorizontal = 'powerpoint.table-layout.alignment.horizontal';

/** The set the three vertical alignments share. It holds exactly one. See disagreement 2. */
const powerpointTableLayoutVertical = 'powerpoint.table-layout.alignment.vertical';

/**
 * PowerPoint's `GroupAlignment`, labelled **Alignment**: the six alignments icon-only, horizontal then vertical, then
 * Text Direction and Cell Margins large. See disagreements 1 to 6.
 *
 * **Two exclusive sets of exactly one**, the six generic toggles, Align Left and Align Top pressed. **Text Direction
 * and Cell Margins are large dropdowns** a host binds.
 *
 * **Three survivors**: Align Left, Align Centre and Align Right.
 */
const powerpointTableLayoutAlignment: readonly RibbonCommand[] = [
  { id: 'powerpoint.table-layout.alignment.align-left', label: 'Align Left', icon: 'text-align-left', size: 'icon', toggle: true, pressed: true, exclusive: powerpointTableLayoutHorizontal, essential: true },
  { id: 'powerpoint.table-layout.alignment.align-centre', label: 'Align Centre', icon: 'text-align-center', size: 'icon', toggle: true, exclusive: powerpointTableLayoutHorizontal, essential: true },
  { id: 'powerpoint.table-layout.alignment.align-right', label: 'Align Right', icon: 'text-align-right', size: 'icon', toggle: true, exclusive: powerpointTableLayoutHorizontal, essential: true },
  { id: 'powerpoint.table-layout.alignment.align-top', label: 'Align Top', icon: 'align-top', size: 'icon', toggle: true, pressed: true, exclusive: powerpointTableLayoutVertical },
  { id: 'powerpoint.table-layout.alignment.centre-vertically', label: 'Centre Vertically', icon: 'align-center-vertical', size: 'icon', toggle: true, exclusive: powerpointTableLayoutVertical },
  { id: 'powerpoint.table-layout.alignment.align-bottom', label: 'Align Bottom', icon: 'align-bottom', size: 'icon', toggle: true, exclusive: powerpointTableLayoutVertical },
  { id: 'powerpoint.table-layout.alignment.text-direction', label: 'Text Direction', icon: 'text-direction-rotate-90-right', size: 'large' },
  { id: 'powerpoint.table-layout.alignment.cell-margins', label: 'Cell Margins', icon: 'padding-left', size: 'large' },
];

/**
 * PowerPoint's `GroupTableSize`, labelled **Table Size**: Height, Width and Lock Aspect Ratio in a column. See
 * disagreement 7.
 *
 * **Height and Width are measure fields** and **Lock Aspect Ratio a toggle drawn as a checkbox**, all three bound by a
 * host; Lock Aspect Ratio starts unticked.
 *
 * **No survivor**: two fields and a checkbox.
 */
const powerpointTableLayoutTableSize: readonly RibbonCommand[] = [
  { id: 'powerpoint.table-layout.table-size.height', label: 'Height' },
  { id: 'powerpoint.table-layout.table-size.width', label: 'Width' },
  { id: 'powerpoint.table-layout.table-size.lock-aspect-ratio', label: 'Lock Aspect Ratio', toggle: true },
];

// ── the contextual tab sets ──────────────────────────────────────────────────
//
// See the *contextual tab sets* section of this file's header: the four common sets, their groups transcribed from
// the census's `TabSet*` rows with Microsoft 365's labels, and no commands. Every other in-scope set is recorded
// below them, with its reason, and `tests/ribbons.test.ts` holds the two together to the census.

/** **Why every other in-scope contextual set is not built.** Stated once, so every record carries the same words. */
export const unbuiltContextualSetReason =
  'Only the four common sets (Table, Picture, Drawing, Chart) are built; decided by the user, 2026-09-15.';

/** Why Chart Tools' three older tabs are not built beside the two that are. See the header's item 1. */
export const legacyChartTabReason =
  'Office 2007–2010’s chart tab, which Office 2013 replaced with TabChartToolsDesignNew and ' +
  'TabChartToolsFormatNew; only Chart Design and Format are built, decided by the user, 2026-09-15.';

/** Chart Tools' three older tabs, which the census carries for all three applications. */
const legacyChartTabs: readonly UnbuiltContextualTab[] = [
  { tab: 'TabChartToolsDesign', reason: legacyChartTabReason },
  { tab: 'TabChartToolsFormat', reason: legacyChartTabReason },
  { tab: 'TabChartToolsLayout', reason: legacyChartTabReason },
];

export const wordRibbonContextualSets: readonly RibbonContextualSetEntry[] = [
  {
    id: 'table-tools',
    tabSet: 'TabSetTableTools',
    label: 'Table Tools',
    tabs: [
      {
        id: 'table-design',
        label: 'Table Design',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetTableTools', tab: 'TabTableToolsDesign' },
        groups: [
          { id: 'GroupTableLayout', label: 'Table Style Options', priority: 'standard', controls: 13, inScope: true, commands: wordTableDesignTableStyleOptions },
          { id: 'GroupTableStylesWord', label: 'Table Styles', priority: 'primary', controls: 6, inScope: true, commands: wordTableDesignTableStyles },
          { id: 'GroupTableBorders', label: 'Borders', priority: 'primary', controls: 13, inScope: true, commands: wordTableDesignBorders },
        ],
      },
      {
        id: 'table-layout',
        label: 'Layout',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetTableTools', tab: 'TabTableToolsLayout' },
        groups: [
          { id: 'GroupTable', label: 'Table', priority: 'secondary', controls: 7, inScope: true, commands: wordTableLayoutTable },
          { id: 'GroupTableDraw', label: 'Draw', priority: 'ancillary', controls: 2, inScope: true, commands: wordTableLayoutDraw },
          { id: 'GroupTableRowsAndColumns', label: 'Rows & Columns', priority: 'primary', controls: 10, inScope: true, commands: wordTableLayoutRowsAndColumns },
          { id: 'GroupTableMerge', label: 'Merge', priority: 'standard', controls: 3, inScope: true, commands: wordTableLayoutMerge },
          { id: 'GroupTableCellSize', label: 'Cell Size', priority: 'standard', controls: 9, inScope: true, commands: wordTableLayoutCellSize },
          { id: 'GroupTableAlignment', label: 'Alignment', priority: 'primary', controls: 21, inScope: true, commands: wordTableLayoutAlignment },
          { id: 'GroupTableData', label: 'Data', priority: 'standard', controls: 4, inScope: true, commands: wordTableLayoutData },
        ],
      },
    ],
    unbuiltTabs: [],
  },
  {
    id: 'picture-tools',
    tabSet: 'TabSetPictureTools',
    label: 'Picture Tools',
    tabs: [
      {
        id: 'picture-format',
        label: 'Picture Format',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetPictureTools', tab: 'TabPictureToolsFormat' },
        groups: [
          { id: 'GroupPictureTools', label: 'Adjust', priority: 'primary', controls: 29, inScope: true },
          { id: 'GroupPictureStyles', label: 'Picture Styles', priority: 'primary', controls: 28, inScope: true },
          { id: 'GroupAltText', label: 'Accessibility', priority: 'secondary', controls: 1, inScope: true },
          { id: 'GroupArrangeWith3DEditor', label: 'Arrange', priority: 'standard', controls: 65, inScope: true },
          { id: 'GroupPictureSize', label: 'Size', priority: 'standard', controls: 20, inScope: true },
          { id: 'GroupImagePlay', label: 'Image Play', priority: 'ancillary', controls: 1, inScope: true },
        ],
      },
    ],
    unbuiltTabs: [],
  },
  {
    id: 'drawing-tools',
    tabSet: 'TabSetDrawingTools',
    label: 'Drawing Tools',
    tabs: [
      {
        id: 'shape-format',
        label: 'Shape Format',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetDrawingTools', tab: 'TabDrawingToolsFormat' },
        groups: [
          { id: 'GroupShapes', label: 'Insert Shapes', priority: 'standard', controls: 12, inScope: true },
          { id: 'GroupShapeStyles', label: 'Shape Styles', priority: 'primary', controls: 37, inScope: true },
          { id: 'GroupWordArtStyles', label: 'WordArt Styles', priority: 'standard', controls: 30, inScope: true },
          { id: 'GroupTextbox', label: 'Text', priority: 'standard', controls: 5, inScope: true },
          { id: 'GroupAltText', label: 'Accessibility', priority: 'secondary', controls: 1, inScope: true },
          { id: 'GroupArrangeWith3DEditor', label: 'Arrange', priority: 'standard', controls: 65, inScope: true },
          { id: 'GroupSize', label: 'Size', priority: 'standard', controls: 3, inScope: true },
        ],
      },
    ],
    unbuiltTabs: [],
  },
  {
    id: 'chart-tools',
    tabSet: 'TabSetChartTools',
    label: 'Chart Tools',
    tabs: [
      {
        id: 'chart-design',
        label: 'Chart Design',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetChartTools', tab: 'TabChartToolsDesignNew' },
        groups: [
          { id: 'GroupChartLayouts', label: 'Chart Layouts', priority: 'primary', controls: 23, inScope: true },
          { id: 'GroupChartStyles', label: 'Chart Styles', priority: 'secondary', controls: 2, inScope: true },
          { id: 'GroupChartData', label: 'Data', priority: 'standard', controls: 6, inScope: true },
          { id: 'GroupChartType', label: 'Type', priority: 'secondary', controls: 1, inScope: true },
        ],
      },
      {
        id: 'chart-format',
        label: 'Format',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetChartTools', tab: 'TabChartToolsFormatNew' },
        groups: [
          { id: 'GroupChartCurrentSelection', label: 'Current Selection', priority: 'standard', controls: 3, inScope: true },
          { id: 'GroupShapesChart', label: 'Insert Shapes', priority: 'ancillary', controls: 2, inScope: true },
          { id: 'GroupChartShapeStyles', label: 'Shape Styles', priority: 'primary', controls: 35, inScope: true },
          { id: 'GroupWordArtStyles', label: 'WordArt Styles', priority: 'standard', controls: 30, inScope: true },
          { id: 'GroupAltText', label: 'Accessibility', priority: 'secondary', controls: 1, inScope: true },
          { id: 'GroupArrange', label: 'Arrange', priority: 'standard', controls: 65, inScope: true },
          { id: 'GroupSize', label: 'Size', priority: 'standard', controls: 3, inScope: true },
        ],
      },
    ],
    unbuiltTabs: legacyChartTabs,
  },
];

export const powerpointRibbonContextualSets: readonly RibbonContextualSetEntry[] = [
  {
    id: 'table-tools',
    tabSet: 'TabSetTableTools',
    label: 'Table Tools',
    tabs: [
      {
        id: 'table-design',
        label: 'Table Design',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetTableTools', tab: 'TabTableToolsDesign' },
        groups: [
          { id: 'GroupTableStyleOptionsPowerPoint', label: 'Table Style Options', priority: 'standard', controls: 6, inScope: true, commands: powerpointTableDesignTableStyleOptions },
          { id: 'GroupTableStylesPowerPoint', label: 'Table Styles', priority: 'primary', controls: 33, inScope: true, commands: powerpointTableDesignTableStyles },
          { id: 'GroupTextStylesTable', label: 'WordArt Styles', priority: 'standard', controls: 33, inScope: true, commands: wordArtStylesCommands('powerpoint', 'table-design') },
          { id: 'GroupDrawBorders', label: 'Draw Borders', priority: 'standard', controls: 6, inScope: true, commands: powerpointTableDesignDrawBorders },
        ],
      },
      {
        id: 'table-layout',
        label: 'Layout',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetTableTools', tab: 'TabTableToolsLayout' },
        groups: [
          { id: 'GroupTable', label: 'Table', priority: 'secondary', controls: 5, inScope: true, commands: powerpointTableLayoutTable },
          { id: 'GroupTableRowsAndColumns', label: 'Rows & Columns', priority: 'primary', controls: 8, inScope: true, commands: powerpointTableLayoutRowsAndColumns },
          { id: 'GroupMerge', label: 'Merge', priority: 'secondary', controls: 2, inScope: true, commands: powerpointTableLayoutMerge },
          { id: 'GroupTableCellSize', label: 'Cell Size', priority: 'standard', controls: 4, inScope: true, commands: powerpointTableLayoutCellSize },
          { id: 'GroupAlignment', label: 'Alignment', priority: 'primary', controls: 12, inScope: true, commands: powerpointTableLayoutAlignment },
          { id: 'GroupTableSize', label: 'Table Size', priority: 'standard', controls: 3, inScope: true, commands: powerpointTableLayoutTableSize },
          { id: 'GroupArrange', label: 'Arrange', priority: 'standard', controls: 46, inScope: true, commands: arrangeCommands('powerpoint', 'table-layout', 'table') },
        ],
      },
    ],
    unbuiltTabs: [],
  },
  {
    id: 'picture-tools',
    tabSet: 'TabSetPictureTools',
    label: 'Picture Tools',
    tabs: [
      {
        id: 'picture-format',
        label: 'Picture Format',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetPictureTools', tab: 'TabPictureToolsFormat' },
        groups: [
          { id: 'GroupPictureTools', label: 'Adjust', priority: 'primary', controls: 31, inScope: true },
          { id: 'GroupPictureStyles', label: 'Picture Styles', priority: 'primary', controls: 30, inScope: true },
          { id: 'GroupAltText', label: 'Accessibility', priority: 'secondary', controls: 1, inScope: true },
          { id: 'GroupArrangeWith3DEditor', label: 'Arrange', priority: 'standard', controls: 24, inScope: true },
          { id: 'GroupPictureSize', label: 'Size', priority: 'standard', controls: 20, inScope: true },
          { id: 'GroupImagePlay', label: 'Image Play', priority: 'ancillary', controls: 1, inScope: true },
        ],
      },
    ],
    unbuiltTabs: [],
  },
  {
    id: 'drawing-tools',
    tabSet: 'TabSetDrawingTools',
    label: 'Drawing Tools',
    tabs: [
      {
        id: 'shape-format',
        label: 'Shape Format',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetDrawingTools', tab: 'TabDrawingToolsFormat' },
        groups: [
          { id: 'GroupShapes', label: 'Insert Shapes', priority: 'standard', controls: 13, inScope: true },
          { id: 'GroupShapeStyles', label: 'Shape Styles', priority: 'primary', controls: 40, inScope: true },
          { id: 'GroupWordArtStyles', label: 'WordArt Styles', priority: 'standard', controls: 33, inScope: true },
          { id: 'GroupAltText', label: 'Accessibility', priority: 'secondary', controls: 1, inScope: true },
          { id: 'GroupArrangeWith3DEditor', label: 'Arrange', priority: 'standard', controls: 24, inScope: true },
          { id: 'GroupSize', label: 'Size', priority: 'standard', controls: 3, inScope: true },
        ],
      },
    ],
    unbuiltTabs: [],
  },
  {
    id: 'chart-tools',
    tabSet: 'TabSetChartTools',
    label: 'Chart Tools',
    tabs: [
      {
        id: 'chart-design',
        label: 'Chart Design',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetChartTools', tab: 'TabChartToolsDesignNew' },
        groups: [
          { id: 'GroupChartLayouts', label: 'Chart Layouts', priority: 'primary', controls: 23, inScope: true },
          { id: 'GroupChartStyles', label: 'Chart Styles', priority: 'secondary', controls: 2, inScope: true },
          { id: 'GroupChartData', label: 'Data', priority: 'standard', controls: 6, inScope: true },
          { id: 'GroupChartType', label: 'Type', priority: 'secondary', controls: 1, inScope: true },
        ],
      },
      {
        id: 'chart-format',
        label: 'Format',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetChartTools', tab: 'TabChartToolsFormatNew' },
        groups: [
          { id: 'GroupChartCurrentSelection', label: 'Current Selection', priority: 'standard', controls: 3, inScope: true },
          { id: 'GroupShapesChart', label: 'Insert Shapes', priority: 'ancillary', controls: 2, inScope: true },
          { id: 'GroupChartShapeStyles', label: 'Shape Styles', priority: 'primary', controls: 38, inScope: true },
          { id: 'GroupWordArtStyles', label: 'WordArt Styles', priority: 'standard', controls: 33, inScope: true },
          { id: 'GroupAltText', label: 'Accessibility', priority: 'secondary', controls: 1, inScope: true },
          { id: 'GroupArrange', label: 'Arrange', priority: 'standard', controls: 46, inScope: true },
          { id: 'GroupSize', label: 'Size', priority: 'standard', controls: 3, inScope: true },
        ],
      },
    ],
    unbuiltTabs: legacyChartTabs,
  },
];

/**
 * ⚠ **Excel's Table Tools is a different census set**, `TabSetTableToolsExcel`, with one tab, `TabTableToolsDesignExcel`:
 * a worksheet table has no Layout tab, because its rows and columns are the sheet's.
 */
export const excelRibbonContextualSets: readonly RibbonContextualSetEntry[] = [
  {
    id: 'table-tools',
    tabSet: 'TabSetTableToolsExcel',
    label: 'Table Tools',
    tabs: [
      {
        id: 'table-design',
        label: 'Table Design',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetTableToolsExcel', tab: 'TabTableToolsDesignExcel' },
        groups: [
          { id: 'GroupTableProperties', label: 'Properties', priority: 'standard', controls: 3, inScope: true },
          { id: 'GroupTableTools', label: 'Tools', priority: 'standard', controls: 4, inScope: true },
          { id: 'GroupTableExternalData', label: 'External Table Data', priority: 'secondary', controls: 13, inScope: true },
          { id: 'GroupTableStyleOptions', label: 'Table Style Options', priority: 'primary', controls: 7, inScope: true },
          { id: 'GroupTableStylesExcel', label: 'Table Styles', priority: 'primary', controls: 3, inScope: true },
        ],
      },
    ],
    unbuiltTabs: [],
  },
  {
    id: 'picture-tools',
    tabSet: 'TabSetPictureTools',
    label: 'Picture Tools',
    tabs: [
      {
        id: 'picture-format',
        label: 'Picture Format',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetPictureTools', tab: 'TabPictureToolsFormat' },
        groups: [
          { id: 'GroupPictureTools', label: 'Adjust', priority: 'primary', controls: 30, inScope: true },
          { id: 'GroupPictureStyles', label: 'Picture Styles', priority: 'primary', controls: 28, inScope: true },
          { id: 'GroupAltText', label: 'Accessibility', priority: 'secondary', controls: 1, inScope: true },
          { id: 'GroupArrangeWith3DEditor', label: 'Arrange', priority: 'standard', controls: 47, inScope: true },
          { id: 'GroupPictureSize', label: 'Size', priority: 'standard', controls: 20, inScope: true },
          { id: 'GroupImagePlay', label: 'Image Play', priority: 'ancillary', controls: 1, inScope: true },
        ],
      },
    ],
    unbuiltTabs: [],
  },
  {
    id: 'drawing-tools',
    tabSet: 'TabSetDrawingTools',
    label: 'Drawing Tools',
    tabs: [
      {
        id: 'shape-format',
        label: 'Shape Format',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetDrawingTools', tab: 'TabDrawingToolsFormat' },
        groups: [
          { id: 'GroupShapes', label: 'Insert Shapes', priority: 'standard', controls: 12, inScope: true },
          { id: 'GroupShapeStyles', label: 'Shape Styles', priority: 'primary', controls: 37, inScope: true },
          { id: 'GroupWordArtStyles', label: 'WordArt Styles', priority: 'standard', controls: 30, inScope: true },
          { id: 'GroupAltText', label: 'Accessibility', priority: 'secondary', controls: 1, inScope: true },
          { id: 'GroupArrangeWith3DEditor', label: 'Arrange', priority: 'standard', controls: 47, inScope: true },
          { id: 'GroupSize', label: 'Size', priority: 'standard', controls: 3, inScope: true },
        ],
      },
    ],
    unbuiltTabs: [],
  },
  {
    id: 'chart-tools',
    tabSet: 'TabSetChartTools',
    label: 'Chart Tools',
    tabs: [
      {
        id: 'chart-design',
        label: 'Chart Design',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetChartTools', tab: 'TabChartToolsDesignNew' },
        groups: [
          { id: 'GroupChartLayouts', label: 'Chart Layouts', priority: 'primary', controls: 23, inScope: true },
          { id: 'GroupChartStyles', label: 'Chart Styles', priority: 'primary', controls: 3, inScope: true },
          { id: 'GroupChartData', label: 'Data', priority: 'secondary', controls: 2, inScope: true },
          { id: 'GroupChartType', label: 'Type', priority: 'secondary', controls: 1, inScope: true },
          { id: 'GroupChartLocation', label: 'Location', priority: 'ancillary', controls: 1, inScope: true },
        ],
      },
      {
        id: 'chart-format',
        label: 'Format',
        appearance: 'contextual',
        source: { kind: 'contextual', tabSet: 'TabSetChartTools', tab: 'TabChartToolsFormatNew' },
        groups: [
          { id: 'GroupChartCurrentSelection', label: 'Current Selection', priority: 'standard', controls: 3, inScope: true },
          { id: 'GroupShapesChart', label: 'Insert Shapes', priority: 'ancillary', controls: 2, inScope: true },
          { id: 'GroupChartShapeStyles', label: 'Shape Styles', priority: 'primary', controls: 35, inScope: true },
          { id: 'GroupWordArtStyles', label: 'WordArt Styles', priority: 'standard', controls: 30, inScope: true },
          { id: 'GroupAltText', label: 'Accessibility', priority: 'secondary', controls: 1, inScope: true },
          { id: 'GroupArrange', label: 'Arrange', priority: 'standard', controls: 47, inScope: true },
          { id: 'GroupSize', label: 'Size', priority: 'standard', controls: 3, inScope: true },
        ],
      },
    ],
    unbuiltTabs: legacyChartTabs,
  },
];

/** Every application's built contextual sets, in the order a host draws them. */
export const ribbonContextualSets: Readonly<Record<RibbonApplication, readonly RibbonContextualSetEntry[]>> = {
  word: wordRibbonContextualSets,
  powerpoint: powerpointRibbonContextualSets,
  excel: excelRibbonContextualSets,
};

/** One unbuilt set, with the user's reason. */
function unbuilt(tabSet: string, label: string, tabs: readonly string[]): UnbuiltContextualSet {
  return { tabSet, label, tabs, reason: unbuiltContextualSetReason };
}

/**
 * **Every in-scope contextual set that is not built, per application**, with every in-scope census tab it carries.
 *
 * Transcribed from the census exactly as the built sets are, and held to it by the same test: the declared tabs,
 * each built set's `unbuiltTabs` and these records together must be every in-scope `TabSet*` tab, each exactly once.
 * Excel's `TabSetPowerQueryEdit` is absent because the census marks every one of its rows out of scope, and Word's
 * `TabSetInkTools` is present because one of its rows is in scope. The labels are Office's names for the sets, for a
 * reader; nothing reads them. The two `(classic)` sets are separate census sets, Office 2007's picture and drawing
 * tabs for a document in compatibility mode, and the decision covers them like any other.
 */
export const unbuiltContextualSets: Readonly<Record<RibbonApplication, readonly UnbuiltContextualSet[]>> = {
  word: [
    unbuilt('TabSet3DModelTools', '3D Model Tools', ['Tab3DModelToolsFormat']),
    unbuilt('TabSetDiagramTools', 'Diagram Tools', ['TabDiagramToolsFormatClassic']),
    unbuilt('TabSetDrawingToolsClassic', 'Drawing Tools (classic)', ['TabDrawingToolsFormatClassic']),
    unbuilt('TabSetEquationTools', 'Equation Tools', ['TabEquationToolsDesign']),
    unbuilt('TabSetHeaderAndFooterTools', 'Header & Footer Tools', ['TabHeaderAndFooterToolsDesign']),
    unbuilt('TabSetInkTools', 'Ink Tools', ['TabInkToolsPens']),
    unbuilt('TabSetLearningTools', 'Learning Tools', ['TabLearningTools']),
    unbuilt('TabSetOrganizationChartTools', 'Organization Chart Tools', ['TabOrganizationChartToolsFormat']),
    unbuilt('TabSetPictureToolsClassic', 'Picture Tools (classic)', ['TabPictureToolsFormatClassic']),
    unbuilt('TabSetSmartArtTools', 'SmartArt Tools', ['TabSmartArtToolsDesign', 'TabSmartArtToolsFormat']),
    unbuilt('TabSetSVGTools', 'Graphics Tools', ['TabGraphicsToolsFormat']),
    unbuilt('TabSetTextBoxTools', 'Text Box Tools', ['TabTextBoxToolsFormat']),
    unbuilt('TabSetWordArtTools', 'WordArt Tools', ['TabWordArtToolsFormat']),
  ],
  powerpoint: [
    unbuilt('TabSet3DModelTools', '3D Model Tools', ['Tab3DModelToolsFormat']),
    unbuilt('TabSetAccessibleAuthoring', 'Accessibility', ['TabAccessibleAuthoring']),
    unbuilt('TabSetAudioTools', 'Audio Tools', ['TabAudioToolsFormat', 'TabAudioToolsEdit']),
    unbuilt('TabSetCameoTools', 'Cameo Tools', ['TabDesignCameo']),
    unbuilt('TabSetCDAudioTools', 'CD Audio Tools', ['TabCDAudioToolsOptions']),
    unbuilt('TabSetEquationTools', 'Equation Tools', ['TabEquationToolsDesign']),
    unbuilt('TabSetInkTools', 'Ink Tools', ['TabInkToolsPens']),
    unbuilt('TabSetInteractiveIndexTools', 'Zoom Tools', ['TabInteractiveIndexToolsFormat']),
    unbuilt('TabSetMovieTools', 'Movie Tools', ['TabMovieToolsOptions']),
    unbuilt('TabSetSmartArtTools', 'SmartArt Tools', ['TabSmartArtToolsDesign', 'TabSmartArtToolsFormat']),
    unbuilt('TabSetSoundTools', 'Sound Tools', ['TabSoundToolsOptions']),
    unbuilt('TabSetSVGTools', 'Graphics Tools', ['TabGraphicsToolsFormat']),
    unbuilt('TabSetVideoTools', 'Video Tools', ['TabVideoToolsDesign', 'TabVideoToolsEdit']),
  ],
  excel: [
    unbuilt('TabSet3DModelTools', '3D Model Tools', ['Tab3DModelToolsFormat']),
    unbuilt('TabSetAccessibleAuthoring', 'Accessibility', ['TabAccessibleAuthoring']),
    unbuilt('TabSetBIVisualTools', 'BI Visual Tools', ['TabBIVisualTools']),
    unbuilt('TabSetEquationTools', 'Equation Tools', ['TabEquationToolsDesign']),
    unbuilt('TabSetHeaderAndFooterTools', 'Header & Footer Tools', ['TabHeaderAndFooterToolsDesign']),
    unbuilt('TabSetInkTools', 'Ink Tools', ['TabInkToolsPens']),
    unbuilt('TabSetPivotChartTools', 'PivotChart Tools', [
      'TabPivotChartToolsAnalyze',
      'TabChartToolsDesignPivotChart',
      'TabChartToolsFormatPivotChart',
      'TabPivotChartToolsDesign',
      'TabPivotChartToolsLayout',
      'TabPivotChartToolsFormat',
    ]),
    unbuilt('TabSetPivotTableTools', 'PivotTable Tools', [
      'TabPivotTableToolsOptions',
      'TabPivotTableToolsDesign',
      'TabPivotTableToolsDesignDeprecated',
    ]),
    unbuilt('TabSetSlicerTools', 'Slicer Tools', ['TabSlicerDesign']),
    unbuilt('TabSetSmartArtTools', 'SmartArt Tools', ['TabSmartArtToolsDesign', 'TabSmartArtToolsFormat']),
    unbuilt('TabSetSparkline', 'Sparkline Tools', ['TabSparklineDesign']),
    unbuilt('TabSetSVGTools', 'Graphics Tools', ['TabGraphicsToolsFormat']),
    unbuilt('TabSetTimeSlicerTools', 'Timeline Tools', ['TabTimeSlicerDesign']),
  ],
};

// ── reading it ───────────────────────────────────────────────────────────────

/**
 * **Every tab one application declares**: its core, view and File tabs in Office's order, then each built
 * contextual set's tabs in set order. For the gates that hold every tab to one rule, and for `ribbonTab`.
 */
export function everyRibbonTab(application: RibbonApplication): readonly RibbonTabEntry[] {
  return [
    ...ribbonCensus[application],
    ...ribbonContextualSets[application].flatMap((set) => set.tabs),
  ];
}

/**
 * One tab, by application and kebab id — a contextual tab included, since a tab id is unique across both.
 *
 * @throws {Error} when the tab is not declared — a module asking for a tab that is not in the
 * census has a typo, and a silent `undefined` would become an empty ribbon nobody could explain.
 */
export function ribbonTab(application: RibbonApplication, id: string): RibbonTabEntry {
  const entry = everyRibbonTab(application).find((tab) => tab.id === id);
  if (entry === undefined) {
    throw new Error(`${application} declares no '${id}' tab in dev/ribbons/census.ts`);
  }
  return entry;
}

/**
 * One built contextual set, by application and kebab id.
 *
 * @throws {Error} for the same reason `ribbonTab` does: a host naming a set that is not built has a typo, or is
 * asking for one of the sets `unbuiltContextualSets` records.
 */
export function ribbonContextualSet(application: RibbonApplication, id: string): RibbonContextualSetEntry {
  const entry = ribbonContextualSets[application].find((set) => set.id === id);
  if (entry === undefined) {
    throw new Error(`${application} declares no built '${id}' contextual set in dev/ribbons/census.ts`);
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

/** Every command declared anywhere in the census, contextual tabs included, for the gates that sweep all of them. */
export function everyRibbonCommand(): readonly RibbonCommand[] {
  return ribbonApplicationNames.flatMap((application) =>
    everyRibbonTab(application).flatMap((tab) =>
      tab.groups.flatMap((group) => group.commands ?? []),
    ),
  );
}
