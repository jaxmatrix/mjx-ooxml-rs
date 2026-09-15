/**
 * **The menus the Recording tab opens**, written once and rendered by both hosts.
 *
 * **PowerPoint's Recording tab**, authored under the one-tab-one-application rule, and the only application
 * with the tab: neither Word nor Excel has one, so `recordingMenus` renders nothing for either, and that is a
 * fact about Office rather than a unit still to come. The pattern is `stories/ribbons/slide-show-menus.ts`'s,
 * for its reasons. A binding lives in its host. The menu it opens is written here, with its id from
 * `commandSurfaceId(host, commandId)` through `commandMenu`. A host renders `recordingMenus(application, host)`
 * once beside its ribbon. Every `commandMenu(host, '…'` call below spells its command id literally, so
 * `tests/ribbons.test.ts` can read it.
 *
 * ## Seven menus, four of them Insert's
 *
 * **Screenshot, Cameo, Video and Audio are the commands Insert already draws**, placed on a second tab, so
 * their entries are *called* from `stories/ribbons/insert-menus.ts` rather than written again: two lists of
 * one menu would be two places for it to drift. Each still gets its own menu element here, because the
 * element's id is derived from the command id and the Recording tab's command id is its own.
 *
 * **Record, Clear Recording, Reset to Cameo and Export** are this tab's alone.
 *
 * ## Complete, not shallow
 *
 * The rule since the rejected Transitions gallery: a popup carries **every entry Office's popup has**, by
 * Office's names and in Office's order.
 *
 * ⚠ `GUESS:` **the four lists written here are from Microsoft's support wording for the record window**, not
 * from a build this project can cite, and `dev/ribbons/census.ts` records each disagreement:
 *
 * - **Record** lists From Current Slide… and From Beginning…, the two places Microsoft 365's Record starts.
 *   Office's *Clear* is not here: the Edit group carries the newer recorder's Clear Recording.
 * - **Clear Recording** lists Clear Recording on Current Slide and Clear Recording on All Slides.
 * - **Reset to Cameo** lists Reset to Cameo on Current Slide and Reset to Cameo on All Slides.
 * - **Export** lists Export Video and Customize Export, the record window's Export screen.
 *
 * **Nothing here dispatches a command.** A menu opens and an entry can be chosen; no recording starts, because
 * command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, nothing, type TemplateResult } from 'lit';

import type { RibbonApplication, RibbonSurfaceHost } from '../../dev/ribbons/census.ts';
import { cameoEntries, powerpointAudioEntries, powerpointVideoEntries, screenshotEntries } from './insert-menus.ts';
import { commandMenu } from './ribbon-parts.ts';

// ── the entries ──────────────────────────────────────────────────────────────

/** One entry. */
function item(label: string): TemplateResult {
  return html`<mjx-menu-item label=${label}></mjx-menu-item>`;
}

// ── PowerPoint's Recording ───────────────────────────────────────────────────

/** Record's arrow: where the recording starts. */
function recordEntries(): TemplateResult[] {
  return [item('From Current Slide…'), item('From Beginning…')];
}

/** Clear Recording: this slide's recording, or every slide's. */
function clearRecordingEntries(): TemplateResult[] {
  return [item('Clear Recording on Current Slide'), item('Clear Recording on All Slides')];
}

/** Reset to Cameo: this slide's recorded video back to the live camera, or every slide's. */
function resetToCameoEntries(): TemplateResult[] {
  return [item('Reset to Cameo on Current Slide'), item('Reset to Cameo on All Slides')];
}

/** Export: the record window's Export screen. */
function exportEntries(): TemplateResult[] {
  return [item('Export Video'), item('Customize Export')];
}

/** Seven menus: four of Insert's lists, and the three groups this tab alone draws. */
function powerpointRecordingMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.recording.recording.record', 'Record', ...recordEntries())}
    ${commandMenu(host, 'powerpoint.recording.content.screenshot', 'Screenshot', ...screenshotEntries())}
    ${commandMenu(host, 'powerpoint.recording.camera.cameo', 'Cameo', ...cameoEntries())}
    ${commandMenu(host, 'powerpoint.recording.auto-play-media.video', 'Video', ...powerpointVideoEntries())}
    ${commandMenu(host, 'powerpoint.recording.auto-play-media.audio', 'Audio', ...powerpointAudioEntries())}
    ${commandMenu(host, 'powerpoint.recording.edit.clear-recording', 'Clear Recording', ...clearRecordingEntries())}
    ${commandMenu(host, 'powerpoint.recording.edit.reset-to-cameo', 'Reset to Cameo', ...resetToCameoEntries())}
    ${commandMenu(host, 'powerpoint.recording.export.export', 'Export', ...exportEntries())}
  `;
}

// ── what a host renders ──────────────────────────────────────────────────────

/** PowerPoint alone, because Word and Excel have no Recording tab. */
const menusByApplication: Partial<Record<RibbonApplication, (host: RibbonSurfaceHost) => TemplateResult>> = {
  powerpoint: powerpointRecordingMenus,
};

/**
 * Every menu one application's Recording tab opens, with ids for one host's page.
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, exactly as `slideShowMenus` is.
 */
export function recordingMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult | typeof nothing {
  return menusByApplication[application]?.(host) ?? nothing;
}
