/**
 * **The parts every ribbon tab is assembled from**, and the seam between a tab's *structure* and a
 * host's *bindings*.
 *
 * `dev/ribbons/census.ts` says which groups a tab has, in which order, at which priority, holding
 * which commands. It does not — and must not — say what this machine's font list is, which palette
 * this document carries, or which menu a paste button opens: those are application state, they
 * belong to whoever is assembling the ribbon, and a shell and a catalogue story legitimately answer
 * them differently. So a host passes `ControlOverrides`, a map from a command's stable id to a
 * `TemplateResult`, and `renderCommand` prefers the override over the generic control.
 *
 * That split is what lets the same three modules drive two surfaces without either becoming a
 * mock-up of the other — and it is also what keeps `tests/shell.test.ts`'s twenty-catalogued-element
 * floor honest, because the shell still *names* every component it composes.
 *
 * ## Why the generic control is only a button or a toggle
 *
 * The approved plan's architecture note, and it applies with full force here: a data union that
 * could express a split button's menu, a gallery's items, a colour picker's palette and a font
 * picker's list would be a renderer re-expressing every component's public API — a second API to
 * keep in step with the first. Two shapes cover the great majority of a ribbon's face; everything
 * else is an override, written as markup by the host that knows what to put in it.
 *
 * ## Everything a ribbon module needs comes from here
 *
 * `group()`, `toggle()`, `openDeclaredSurface` and the four in-ribbon widths live in
 * `stories/shell/shell-parts.ts`, because the shells needed them first and there must be one copy.
 * They are re-exported below so a ribbon module has one import rather than two, and so the day one
 * of them moves it moves in one place.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, nothing, type TemplateResult } from 'lit';

import {
  commandSurfaceId,
  ribbonGroup,
  strongestPriority,
  type RibbonCommand,
  type RibbonSurfaceHost,
  type RibbonTabEntry,
} from '../../dev/ribbons/census.ts';
import { group, toggle } from '../shell/shell-parts.ts';

export {
  group,
  openDeclaredSurface,
  ribbonColourFieldStyle,
  ribbonFieldStyle,
  ribbonGalleryStyle,
  ribbonNarrowFieldStyle,
  toggle,
} from '../shell/shell-parts.ts';

export {
  ribbonKeyboard,
  ribbonScreenReader,
  ribbonStatesMatrix,
  ribbonTokenDependencies,
  note,
} from '../ribbon/specimens.ts';

/**
 * A host's bindings, by command id.
 *
 * ⚠ A key that names no command is **silently ignored**, and that is deliberate rather than lax: a
 * host binds by id and a tab's command list grows unit by unit, so an override written for a
 * command that has not landed yet must not be an exception at render time. What catches the typo
 * is `tests/ribbons.test.ts`, which sweeps every override key the story files declare against the
 * census — a check, not a crash in front of an auditor.
 */
export type ControlOverrides = Readonly<Record<string, TemplateResult>>;

/** What every `<app><Tab>Tab()` function takes. */
export interface TabOptions {
  readonly controls?: ControlOverrides;
}

// ── what a host has to know that a census cannot ─────────────────────────────

/**
 * **This machine's printers**, and the counts a copies field offers.
 *
 * The File tab's Print group is the first group in the census whose bindings are not about a
 * document at all. A font list is at least *arguably* the document's; which printers are attached
 * to this computer is not, and no amount of ribbon data can say. So Printer and Copies are declared
 * as commands with no icon — see `dev/ribbons/census.ts` — and each host binds a real control over
 * them by id.
 *
 * The two lists live here rather than in each host because there are **four** of them for the same
 * two commands: `Ribbons/Word`, `Ribbons/PowerPoint` and the Word and PowerPoint shells. Four
 * copies of one list is four places for one of them to drift, which is the argument the census
 * itself is written under, applied one level up.
 *
 * ⚠ The names are deliberately **not** a real product's. A specimen that named a printer somebody
 * could buy would be a claim about what this platform has been tested against; these say *a machine
 * with three printers on it* and nothing more.
 */
export const printerList: readonly { readonly value: string; readonly label: string }[] = [
  { value: 'pdf', label: 'Print to PDF' },
  { value: 'hallway', label: 'Hallway Colour (Network)' },
  { value: 'studio', label: 'Studio Laser' },
];

/** What a copies field offers before somebody types their own. `allow-custom` is why it is short. */
export const copyCounts: readonly string[] = ['1', '2', '3', '4', '5'];

/**
 * **What Excel's Scale to Fit offers**: the page counts under Width and Height, and the percentages
 * under Scale.
 *
 * Here for `copyCounts`' reason, at a smaller scale: two hosts bind the same three fields. The page
 * counts are Office's own entries, Automatic then one page to four; the percentages are a short list
 * a combo box can extend, because Office's Scale is a spin box from 10% to 400% and the field takes any
 * number typed into it.
 */
export const fitPageCounts: readonly { readonly value: string; readonly label: string }[] = [
  { value: 'automatic', label: 'Automatic' },
  { value: '1', label: '1 page' },
  { value: '2', label: '2 pages' },
  { value: '3', label: '3 pages' },
  { value: '4', label: '4 pages' },
];

/** The Scale field's list. See `fitPageCounts`. */
export const scalePercentages: readonly string[] = ['100%', '90%', '75%', '50%', '25%'];

/**
 * **The fields References and Transitions bind** (unit 6), for `fitPageCounts`' reason: two hosts bind
 * each one.
 *
 * - `citationStyles`: a handful of Word's citation styles by Office's names. A British build starts
 *   on APA, as Word does.
 * - `transitionSounds`: PowerPoint's whole Sound list (unit 7 completed it).
 * - `durationSeconds`: seconds, written as Office writes them. 00.70 is Fade's own duration, and
 *   the hosts start the gallery on Fade. **Since unit 7, also Animations' Duration**, which starts on
 *   Fly In's 00.50: both are Office's seconds spin boxes, so the list is written once.
 * - `advanceAfterTimes`: minutes, seconds and hundredths, as Office writes them.
 *
 * Both time lists are short because each field is a combo box: Office's are spin boxes, and the field
 * takes any time typed into it.
 */
export const citationStyles: readonly { readonly value: string; readonly label: string }[] = [
  { value: 'apa', label: 'APA' },
  { value: 'chicago', label: 'Chicago' },
  { value: 'harvard-anglia', label: 'Harvard - Anglia' },
  { value: 'ieee', label: 'IEEE' },
  { value: 'iso-690', label: 'ISO 690 - Numerical Reference' },
  { value: 'mla', label: 'MLA' },
  { value: 'turabian', label: 'Turabian' },
];

/**
 * The Sound field's list: **every entry Office's Sound box offers**, in Office's order — the two bracketed
 * entries, the twenty built-in sounds, and *Other Sound…*, which opens a file picker in Office and is an
 * ordinary entry here. See `citationStyles`.
 */
export const transitionSounds: readonly { readonly value: string; readonly label: string }[] = [
  { value: 'no-sound', label: '[No Sound]' },
  { value: 'stop-previous-sound', label: '[Stop Previous Sound]' },
  { value: 'applause', label: 'Applause' },
  { value: 'arrow', label: 'Arrow' },
  { value: 'bomb', label: 'Bomb' },
  { value: 'breeze', label: 'Breeze' },
  { value: 'camera', label: 'Camera' },
  { value: 'cash-register', label: 'Cash Register' },
  { value: 'chime', label: 'Chime' },
  { value: 'click', label: 'Click' },
  { value: 'coin', label: 'Coin' },
  { value: 'drum-roll', label: 'Drum Roll' },
  { value: 'explosion', label: 'Explosion' },
  { value: 'hammer', label: 'Hammer' },
  { value: 'laser', label: 'Laser' },
  { value: 'push', label: 'Push' },
  { value: 'suction', label: 'Suction' },
  { value: 'typewriter', label: 'Typewriter' },
  { value: 'voltage', label: 'Voltage' },
  { value: 'whoosh', label: 'Whoosh' },
  { value: 'wind', label: 'Wind' },
  { value: 'other-sound', label: 'Other Sound…' },
];

/** The Duration field's list. See `citationStyles`. */
export const durationSeconds: readonly string[] = ['00.50', '00.70', '01.00', '01.50', '02.00'];

/** The Advance Slide After field's list. See `citationStyles`. */
export const advanceAfterTimes: readonly string[] = ['00:00.00', '00:02.00', '00:05.00', '00:10.00', '00:30.00'];

/**
 * **The fields Mailings and Animations bind** (unit 7), for `fitPageCounts`' reason: two hosts bind each.
 *
 * - `mergeRecordNumbers`: Mailings' *Go to Record*. Office's box takes any record number, so the list is
 *   the first five of a short recipient list and the combo box takes the rest.
 * - `animationStarts`: Animations' Start, Office's three entries.
 * - `animationDelays`: Animations' Delay, seconds as Office writes them, starting on 00.00. Duration
 *   reuses `durationSeconds`.
 */
export const mergeRecordNumbers: readonly string[] = ['1', '2', '3', '4', '5'];

/** The Start field's list. See `mergeRecordNumbers`. */
export const animationStarts: readonly { readonly value: string; readonly label: string }[] = [
  { value: 'on-click', label: 'On Click' },
  { value: 'with-previous', label: 'With Previous' },
  { value: 'after-previous', label: 'After Previous' },
];

/** The Delay field's list. See `mergeRecordNumbers`. */
export const animationDelays: readonly string[] = ['00.00', '00.25', '00.50', '01.00', '02.00'];

/**
 * **The field Word's Review tab binds** (unit 8), for `fitPageCounts`' reason: two hosts bind it.
 *
 * `displayForReviewModes` is Tracking's *Display for Review* box: **every** mode Office offers, in
 * Office's order. A host starts it on Simple Markup, a new document's mode since Word 2013.
 */
export const displayForReviewModes: readonly { readonly value: string; readonly label: string }[] = [
  { value: 'simple-markup', label: 'Simple Markup' },
  { value: 'all-markup', label: 'All Markup' },
  { value: 'no-markup', label: 'No Markup' },
  { value: 'original', label: 'Original' },
];

// ── the parts ────────────────────────────────────────────────────────────────

/** One tab of a ribbon. */
export function tab(id: string, label: string, ...groups: TemplateResult[]): TemplateResult {
  return html`<mjx-ribbon-tab tab-id=${id} label=${label}>${groups}</mjx-ribbon-tab>`;
}

/**
 * One command: the host's binding for it, or the generic control the census describes.
 *
 * **`essential` is read from the census and from nowhere else**, for a toggle and a button alike.
 * Until unit 2b a toggle was essential *because* it was a toggle — `toggle()` put every one of them
 * in `slot="essential"` — which capped a group at three state commands and made Justify, Subscript
 * and Excel's vertical alignments impossible to draw pressed. A state is not a survivor; a survivor
 * is declared.
 *
 * A toggle still goes through `shell-parts.ts`'s `toggle()` rather than being built here, so there
 * is one spelling of a ribbon toggle; what it no longer does is decide anything on the census's
 * behalf. **`exclusive` is carried the same way**: the census names the set, and the toggle writes it.
 * An override that draws a member writes the attribute itself, and `tests/ribbons.test.ts` holds it to
 * the census.
 *
 * ⚠ An override is drawn exactly as the host wrote it, `slot` included, which is why
 * `tests/ribbons.test.ts` refuses an override that claims `slot="essential"`: every override in
 * this catalogue is richer than a button — a split button, a picker, a gallery, a field — and so
 * fails demotion rule 1 before anyone has to ask.
 */
export function renderCommand(
  command: RibbonCommand,
  overrides: ControlOverrides = {},
): TemplateResult {
  const override = overrides[command.id];
  if (override !== undefined) return override;
  if (command.toggle === true) {
    return toggle(command.label, command.icon, {
      essential: command.essential === true,
      pressed: command.pressed === true,
      size: command.size ?? 'small',
      exclusive: command.exclusive,
    });
  }
  return html`<mjx-button
    slot=${command.essential === true ? 'essential' : nothing}
    label=${command.label}
    icon=${command.icon ?? nothing}
    size=${command.size ?? nothing}
  ></mjx-button>`;
}

/**
 * One group of a tab, drawn from the census: its English label, its priority and its commands.
 *
 * The label and the priority are **not** arguments, so a tab module cannot quietly disagree with
 * the table three gates read. What a tab module does decide is the *order* the groups appear in —
 * which is Office's, and is not in the census — and whether a group has a dialog launcher, which is
 * an application decision the census has no column for.
 */
export function censusGroup(
  entry: RibbonTabEntry,
  groupId: string,
  options: { readonly launcher?: string },
  overrides: ControlOverrides = {},
): TemplateResult {
  const declared = ribbonGroup(entry, groupId);
  return group(
    declared.label,
    declared.priority,
    options,
    ...(declared.commands ?? []).map((command) => renderCommand(command, overrides)),
  );
}

/**
 * **The menu a bound command opens**, with the id both hosts' bindings derive from the command id.
 *
 * The seam `openDeclaredSurface` reads: a binding carries `data-opens="<commandSurfaceId>"`, and a
 * press (the whole button, or a split button's arrow) opens the element with that id. The id is
 * never written by hand on this side — see `commandSurfaceId` — so the only way a binding and its
 * menu can disagree is a binding that names the wrong command, which `tests/ribbons.test.ts` refuses.
 *
 * ⚠ **The first argument is always spelt `host` at a call site**, and `tests/ribbons.test.ts` reads
 * the literal command id that follows it. A menu built from a computed id would be a menu the gate
 * cannot see.
 */
export function commandMenu(
  host: RibbonSurfaceHost,
  commandId: string,
  label: string,
  ...entries: TemplateResult[]
): TemplateResult {
  return html`<mjx-menu id=${commandSurfaceId(host, commandId)} label=${label} floating>
    ${entries}
  </mjx-menu>`;
}

/**
 * A tab whose unit has not landed yet: one group carrying the tab's name, and one honest button.
 *
 * ⚠ **The priority is the census's, not a constant**, and that is the whole point of the
 * placeholder being built from the entry rather than from a `stubTab(id, label, …)` call. A tab
 * that will hold a `primary` group when unit *N* authors it must not collapse earlier today than
 * it will then, or the collapse ladder a reviewer is looking at is a property of the scaffold
 * rather than of the ribbon. `strongestPriority` is what reads it.
 *
 * The button says *Not yet authored* rather than naming a plausible command, for the reason
 * `dev/word-tab-home.ts` gives about its own filler: a made-up command name is a worse lie than an
 * obvious placeholder, and a placeholder occupies exactly as much of the layout as a command does.
 */
export function placeholderTab(entry: RibbonTabEntry): TemplateResult {
  return tab(
    entry.id,
    entry.label,
    group(
      entry.label,
      strongestPriority(entry),
      {},
      html`<mjx-button label="Not yet authored" size="small"></mjx-button>`,
    ),
  );
}

/** Every tab of one application, in Office's order, filtered by where Office shows them. */
export function tabsFor(
  tabs: readonly RibbonTabEntry[],
  build: (entry: RibbonTabEntry) => TemplateResult,
  options: { readonly includeViewTabs?: boolean } = {},
): TemplateResult[] {
  return tabs
    .filter((entry) => options.includeViewTabs === true || entry.appearance === 'always')
    .map(build);
}
