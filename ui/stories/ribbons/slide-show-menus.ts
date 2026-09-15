/**
 * **The menus the Slide Show tab opens**, written once and rendered by both hosts.
 *
 * **PowerPoint's Slide Show tab**, authored under the one-tab-one-application rule, and the only application
 * with the tab: neither Word nor Excel has one, so `slideShowMenus` renders nothing for either, and that is a
 * fact about Office rather than a unit still to come. The pattern is `stories/ribbons/review-menus.ts`'s, for
 * its reasons. A binding lives in its host. The menu it opens is written here, with its id from
 * `commandSurfaceId(host, commandId)` through `commandMenu`. A host renders `slideShowMenus(application, host)`
 * once beside its ribbon. Every `commandMenu(host, '…'` call below spells its command id literally, so
 * `tests/ribbons.test.ts` can read it.
 *
 * ## Three menus and a field
 *
 * **Present Online** and **Custom Slide Show** are dropdowns, and **Record** is a split button whose arrow is a
 * menu. **Monitor** is a field, whose options are listed here (`slideShowMonitors`) so both hosts bind the same
 * list, as `view-menus.ts` lists Excel's sheet views.
 *
 * ## Complete, not shallow
 *
 * The rule since the rejected Transitions gallery: a popup carries **every entry Office's popup has**, by
 * Office's names and in Office's order. A submenu is flattened into a labelled section, as Record's *Clear* is.
 *
 * ⚠ `GUESS:` **every list here is from memory of PowerPoint 2016 and Microsoft 365**, not from a build this
 * project can cite, and `dev/ribbons/census.ts` records each disagreement:
 *
 * - **Present Online** lists Office Presentation Service and Skype for Business. Office shows the second only
 *   where that client is installed.
 * - **Custom Slide Show** lists Custom Shows… alone, because the catalogue's deck has saved no custom show.
 * - **Record** lists From Current Slide… and From Beginning…, then Clear's four entries.
 * - **Monitor** lists Automatic and Primary Monitor. Office lists every attached display by name, and a
 *   display's name is this machine's data rather than Office's vocabulary.
 *
 * **Nothing here dispatches a command.** A menu opens and an entry can be chosen; no show starts, because
 * command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, nothing, type TemplateResult } from 'lit';

import type { RibbonApplication, RibbonSurfaceHost } from '../../dev/ribbons/census.ts';
import { commandMenu } from './ribbon-parts.ts';

// ── the entries ──────────────────────────────────────────────────────────────

/** One entry. */
function item(label: string): TemplateResult {
  return html`<mjx-menu-item label=${label}></mjx-menu-item>`;
}

/** A labelled run of entries. */
function section(label: string, ...entries: TemplateResult[]): TemplateResult {
  return html`<mjx-menu-section label=${label}>${entries}</mjx-menu-section>`;
}

const separator = (): TemplateResult => html`<mjx-menu-separator></mjx-menu-separator>`;

// ── PowerPoint's Slide Show ──────────────────────────────────────────────────

/** Present Online: the two services PowerPoint 2016 offers. See this file's header. */
function presentOnlineEntries(): TemplateResult[] {
  return [item('Office Presentation Service'), item('Skype for Business')];
}

/**
 * Custom Slide Show: every custom show the deck has saved, then the dialog that edits them. The catalogue's
 * deck has saved none, so the dialog alone.
 */
function customSlideShowEntries(): TemplateResult[] {
  return [item('Custom Shows…')];
}

/** Record's arrow: where to start recording, then Office's *Clear* submenu, flattened. */
function recordEntries(): TemplateResult[] {
  return [
    item('From Current Slide…'),
    item('From Beginning…'),
    separator(),
    section(
      'Clear',
      item('Clear Timing on Current Slide'),
      item('Clear Timings on All Slides'),
      item('Clear Narration on Current Slide'),
      item('Clear Narrations on All Slides'),
    ),
  ];
}

/**
 * **The Monitor field's options**, which each PowerPoint host renders inside its `<mjx-dropdown>`: Automatic,
 * then every attached display. See this file's header on *Primary Monitor*.
 */
export const slideShowMonitors: readonly { readonly value: string; readonly label: string }[] = [
  { value: 'automatic', label: 'Automatic' },
  { value: 'primary-monitor', label: 'Primary Monitor' },
];

/**
 * Three menus. Monitor is a field over `slideShowMonitors`, and Play Narrations, Use Timings, Show Media Controls
 * and Use Presenter View are checkboxes the hosts bind.
 */
function powerpointSlideShowMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.slide-show.start-slide-show.present-online', 'Present Online', ...presentOnlineEntries())}
    ${commandMenu(host, 'powerpoint.slide-show.start-slide-show.custom-slide-show', 'Custom Slide Show', ...customSlideShowEntries())}
    ${commandMenu(host, 'powerpoint.slide-show.set-up.record', 'Record', ...recordEntries())}
  `;
}

// ── what a host renders ──────────────────────────────────────────────────────

/** PowerPoint alone, because Word and Excel have no Slide Show tab. */
const menusByApplication: Partial<Record<RibbonApplication, (host: RibbonSurfaceHost) => TemplateResult>> = {
  powerpoint: powerpointSlideShowMenus,
};

/**
 * Every menu one application's Slide Show tab opens, with ids for one host's page.
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, exactly as `viewMenus` is.
 */
export function slideShowMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult | typeof nothing {
  return menusByApplication[application]?.(host) ?? nothing;
}
