/**
 * **The menus the Draw tab's commands open** — written once, rendered by both hosts.
 *
 * The ribbon programme's unit 4. The pattern is `stories/ribbons/insert-menus.ts`'s, for its reasons:
 * a binding lives in its host, the menu it opens is written here, and `tests/ribbons.test.ts` reads
 * both. Each menu gets its id from `commandSurfaceId(host, commandId)` through `commandMenu`, a host
 * renders `drawMenus(application, host)` once beside its ribbon, and every `commandMenu(host, '…'`
 * call below spells its command id literally, so the gate can read it.
 *
 * ## Nearly everything is shared
 *
 * Draw is the same tab in all three applications, far more than Insert is. Add Pen, the pen styles,
 * Colour, Thickness and Touch/Mouse Mode open the same entries everywhere, so each is one entries
 * function, and an application's menus are a list of `commandMenu` calls over them. The one difference
 * is **Eraser**: Word and PowerPoint draw it as a split button with the eraser sizes behind its arrow,
 * and Excel draws a plain toggle, so Excel has no Eraser menu at all.
 *
 * ## Shallow, and real
 *
 * Decision 3 of the approved plan, as on Insert. The pen styles are a handful of Office's own
 * defaults by their own names (*Pen: Black, 0.5 mm*), not the gallery. The ink colours are a handful
 * of Office's colours and effects, and one entry reaches the rest.
 *
 * **Nothing here dispatches a command.** A menu opens and an entry can be chosen; no ink is drawn,
 * because command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import type { RibbonApplication, RibbonSurfaceHost } from '../../dev/ribbons/census.ts';
import { commandMenu } from './ribbon-parts.ts';

// ── the entries ──────────────────────────────────────────────────────────────

/** One entry. */
function item(label: string): TemplateResult {
  return html`<mjx-menu-item label=${label}></mjx-menu-item>`;
}

/** One entry of a set whose current member is checked. */
function choice(label: string, checked = false): TemplateResult {
  return html`<mjx-menu-item kind="radio" label=${label} ?checked=${checked}></mjx-menu-item>`;
}

/** A labelled run of entries. */
function section(label: string, ...entries: TemplateResult[]): TemplateResult {
  return html`<mjx-menu-section label=${label}>${entries}</mjx-menu-section>`;
}

const separator = (): TemplateResult => html`<mjx-menu-separator></mjx-menu-separator>`;

// ── what every application opens identically ────────────────────────────────

/** Add Pen: the three kinds of pen Office adds to the tray. */
function addPenEntries(): TemplateResult[] {
  return [item('Pen'), item('Pencil'), item('Highlighter')];
}

/** The pen styles, by Office's own names for its defaults. The current style is checked. */
function penStyleEntries(): TemplateResult[] {
  return [
    section('Pens', choice('Pen: Black, 0.5 mm', true), choice('Pen: Red, 0.5 mm'), choice('Pencil: Grey, 1 mm')),
    section('Highlighters', choice('Highlighter: Yellow, 6 mm'), choice('Highlighter: Green, 6 mm')),
  ];
}

/** Colour: a few of Office's ink colours, the effects beneath them, and the dialog. */
function colourEntries(): TemplateResult[] {
  return [
    section(
      'Colours',
      choice('Black', true),
      choice('White'),
      choice('Red'),
      choice('Yellow'),
      choice('Green'),
      choice('Blue'),
    ),
    section('Effects', choice('Rainbow'), choice('Galaxy'), choice('Gold'), choice('Silver')),
    separator(),
    item('More Colours…'),
  ];
}

/** Thickness: the pen widths Office offers, in millimetres. The current one is checked. */
function thicknessEntries(): TemplateResult[] {
  return [
    choice('0.25 mm'),
    choice('0.35 mm'),
    choice('0.5 mm', true),
    choice('1 mm'),
    choice('2 mm'),
    choice('3.5 mm'),
    choice('5 mm'),
  ];
}

/** Touch/Mouse Mode: the two ribbon densities. Mouse is the default. */
function touchMouseModeEntries(): TemplateResult[] {
  return [choice('Mouse', true), choice('Touch')];
}

/** Eraser's arrow, in Word and PowerPoint: a whole stroke at once, or a point eraser of three sizes. */
function eraserEntries(): TemplateResult[] {
  return [choice('Stroke Eraser', true), choice('Small Eraser'), choice('Medium Eraser'), choice('Large Eraser')];
}

// ── Word ─────────────────────────────────────────────────────────────────────

function wordDrawMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'word.draw.drawing-tools.add-pen', 'Add Pen', ...addPenEntries())}
    ${commandMenu(host, 'word.draw.pens.pens', 'Pens', ...penStyleEntries())}
    ${commandMenu(host, 'word.draw.pens.colour', 'Colour', ...colourEntries())}
    ${commandMenu(host, 'word.draw.pens.thickness', 'Thickness', ...thicknessEntries())}
    ${commandMenu(host, 'word.draw.write.eraser', 'Eraser', ...eraserEntries())}
    ${commandMenu(host, 'word.draw.input-mode.touch-mouse-mode', 'Touch/Mouse Mode', ...touchMouseModeEntries())}
  `;
}

// ── PowerPoint ───────────────────────────────────────────────────────────────

function powerpointDrawMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.draw.drawing-tools.add-pen', 'Add Pen', ...addPenEntries())}
    ${commandMenu(host, 'powerpoint.draw.pens.pens', 'Pens', ...penStyleEntries())}
    ${commandMenu(host, 'powerpoint.draw.pens.colour', 'Colour', ...colourEntries())}
    ${commandMenu(host, 'powerpoint.draw.pens.thickness', 'Thickness', ...thicknessEntries())}
    ${commandMenu(host, 'powerpoint.draw.write.eraser', 'Eraser', ...eraserEntries())}
    ${commandMenu(host, 'powerpoint.draw.input-mode.touch-mouse-mode', 'Touch/Mouse Mode', ...touchMouseModeEntries())}
  `;
}

// ── Excel ────────────────────────────────────────────────────────────────────

function excelDrawMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'excel.draw.drawing-tools.add-pen', 'Add Pen', ...addPenEntries())}
    ${commandMenu(host, 'excel.draw.pens.pens', 'Pens', ...penStyleEntries())}
    ${commandMenu(host, 'excel.draw.pens.colour', 'Colour', ...colourEntries())}
    ${commandMenu(host, 'excel.draw.pens.thickness', 'Thickness', ...thicknessEntries())}
    ${commandMenu(host, 'excel.draw.input-mode.touch-mouse-mode', 'Touch/Mouse Mode', ...touchMouseModeEntries())}
  `;
}

// ── what a host renders ──────────────────────────────────────────────────────

const menusByApplication: Readonly<Record<RibbonApplication, (host: RibbonSurfaceHost) => TemplateResult>> = {
  word: wordDrawMenus,
  powerpoint: powerpointDrawMenus,
  excel: excelDrawMenus,
};

/**
 * Every menu one application's Draw tab opens, with ids for one host's page.
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, exactly as `insertMenus` is.
 */
export function drawMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult {
  return menusByApplication[application](host);
}
