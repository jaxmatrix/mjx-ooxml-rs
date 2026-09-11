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
  ribbonGroup,
  strongestPriority,
  type RibbonCommand,
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

// ── the parts ────────────────────────────────────────────────────────────────

/** One tab of a ribbon. */
export function tab(id: string, label: string, ...groups: TemplateResult[]): TemplateResult {
  return html`<mjx-ribbon-tab tab-id=${id} label=${label}>${groups}</mjx-ribbon-tab>`;
}

/**
 * One command: the host's binding for it, or the generic control the census describes.
 *
 * A toggle goes through `shell-parts.ts`'s `toggle()` rather than being built here, because that
 * function is what puts every toggle in `slot="essential"` — the single fact `essentialCommandLimit`
 * is counted against, and a second spelling of it here would be a second place for it to drift.
 */
export function renderCommand(
  command: RibbonCommand,
  overrides: ControlOverrides = {},
): TemplateResult {
  const override = overrides[command.id];
  if (override !== undefined) return override;
  if (command.toggle === true) {
    return toggle(command.label, command.icon ?? '', command.pressed === true);
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
