/**
 * The menu specimens, and the two lists that are **derived rather than typed twice**.
 *
 * `matrix.ts` establishes the rule for the four ribbon archetypes and this follows it: the cells
 * come from `menuItemStateNames`, the captions from `menuItemStates`, and the token-dependency
 * list from the paints those states actually use. A state added to the model appears in the
 * catalogue, in the documentation and in the blast-radius list without anybody remembering.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import {
  controlStateSpecs,
  type ControlState,
} from '../../src/controls/control-states.ts';
import {
  menuItemStateNames,
  menuItemStates,
  type MenuItemState,
} from '../../src/menus/menu-model.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';
import type { TokenPath } from '../../src/tokens/resolver.ts';

/*
 * ⚠ `data-menu-state-cell` is written out below as an attribute *name*, and imported as
 * `menuStateCellAttribute` by the gate, because lit's `html` interpolates values and never names.
 * The gate asserts it found **exactly** as many cells as the model claims states, so a divergence
 * fails with a count rather than sweeping an empty list — MJXOFF-182's lesson, applied to a
 * selector for the second time.
 */
const captionStyle = 'color:var(--theme-text-secondary);margin:0;text-align:center';

const gridStyle =
  'display:grid;grid-template-columns:repeat(auto-fit,minmax(14rem,1fr));' +
  'gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2);' +
  'align-items:start';

/** A note above a specimen, so an auditor knows what they are looking at. */
export function note(text: string): TemplateResult {
  return html`
    <p
      class=${typeRoleClass('body')}
      style="margin:0;padding:calc(var(--mjx-density-gutter) * 2);color:var(--theme-text-primary);max-inline-size:60ch"
    >
      ${text}
    </p>
  `;
}

/** One item, forced into a state — the only way a static story can show `hover` or `active`. */
function cellItem(state: MenuItemState): TemplateResult {
  switch (state) {
    case 'rest':
      return html`<mjx-menu-item label="Paste" icon="folder-open" shortcut="Ctrl+V"></mjx-menu-item>`;
    case 'hover':
      return html`<mjx-menu-item
        label="Paste"
        icon="folder-open"
        shortcut="Ctrl+V"
        force-state="hover"
      ></mjx-menu-item>`;
    case 'active':
      return html`<mjx-menu-item
        label="Paste"
        icon="folder-open"
        shortcut="Ctrl+V"
        force-state="active"
      ></mjx-menu-item>`;
    case 'focus':
      // Never forced: `:focus-visible` is the browser's own judgement about how focus arrived, and
      // a story that asserted it into existence would be auditing a picture of a focus ring.
      return html`<mjx-menu-item label="Paste" icon="folder-open" shortcut="Ctrl+V"></mjx-menu-item>`;
    case 'unavailable':
      return html`<mjx-menu-item
        label="Paste Special"
        icon="folder-open"
        unavailable
        explanation="The clipboard holds nothing that can be pasted specially."
      ></mjx-menu-item>`;
    case 'checked':
      return html`<mjx-menu-item kind="checkbox" label="Ruler" checked></mjx-menu-item>`;
    case 'checkedHover':
      return html`<mjx-menu-item
        kind="checkbox"
        label="Ruler"
        checked
        force-state="hover"
      ></mjx-menu-item>`;
  }
}

/**
 * The matrix: one **inline** menu per state, each holding one row.
 *
 * One menu per cell rather than one menu of seven rows, and the reason is the `focus` cell: a menu
 * holds **one** roving tab stop, so seven rows in one menu would be one tab stop and `Tab` could
 * never reach the sixth. Seven menus are seven tab stops, and the gate walks them with the
 * browser's own sequential navigation exactly as `controls.spec.ts` does.
 */
export function menuStatesMatrix(): TemplateResult {
  return html`
    <div style=${gridStyle}>
      ${menuItemStateNames.map(
        (state) => html`
          <div
            data-menu-state-cell=${state}
            title=${menuItemStates[state].description}
            style="display:grid;gap:var(--mjx-density-step);justify-items:stretch"
          >
            <mjx-menu label=${`${state} specimen`}>${cellItem(state)}</mjx-menu>
            <p class=${typeRoleClass('dense')} style=${captionStyle}>${state}</p>
          </div>
        `,
      )}
    </div>
  `;
}

/** The states matrix as the story conventions want it. */
export function menuStatesFor(): readonly { name: string; description: string }[] {
  return menuItemStateNames.map((state) => ({
    name: state,
    description: menuItemStates[state].description,
  }));
}

const sharedTokenDependencies: readonly TokenPath[] = [
  'duration.transition',
  'ease.spring',
  'ease.outSoft',
  'fontWeight.bold',
  'fontWeight.medium',
  'leading.snug',
  'leading.tight',
  'radius.control',
  'radius.panel',
  'shadow.lift',
  'spacing',
  'text.sm',
  'text.xs',
];

/** The blast radius of a token change, **computed from the state table**. */
export function menuTokenDependencies(): readonly TokenPath[] {
  const members = new Set<string>();
  for (const state of menuItemStateNames) {
    const spec = controlStateSpecs[menuItemStates[state].controlState as ControlState];
    for (const paint of [spec.background, spec.borderColor, spec.text, spec.insetRing]) {
      if (paint !== undefined && paint !== 'transparent') members.add(paint);
    }
  }
  // The surface the menu is drawn on, the rule between its sections, and the focus ring.
  members.add('surfaceRaised');
  members.add('border');
  members.add('borderSubtle');
  members.add('textSecondary');
  members.add('accentPressed');
  const paths = [...members].map((member) => `theme.light.${member}` as TokenPath);
  return [...paths, ...sharedTokenDependencies].sort((left, right) => left.localeCompare(right));
}

/** The keyboard model, written out for the audit from the same vocabulary the component obeys. */
export const menuKeyboard = [
  { keys: 'Tab', does: 'Enters the menu at its one roving tab stop; from inside, leaves and closes it.' },
  { keys: 'Arrow Down / Arrow Up', does: 'Moves to the next or previous item, wrapping at both ends.' },
  { keys: 'Home / End', does: 'Jumps to the first or last item.' },
  { keys: 'Arrow Right', does: 'Opens the submenu and moves into it — Arrow Left under RTL.' },
  { keys: 'Arrow Left', does: 'Closes one submenu level and returns to the item that opened it — Arrow Right under RTL.' },
  { keys: 'Enter / Space', does: 'Activates the item. Space extends a type-ahead when one is in flight.' },
  { keys: 'A printable character', does: 'Type-ahead: selects the first item whose label starts with what was typed; a repeated character cycles.' },
  { keys: 'Escape', does: 'Closes exactly one level, and returns focus to the item or the invoker that opened it.' },
  { keys: 'Context Menu, or Shift + F10', does: 'Opens a context menu at the focused element.' },
];

/** What a screen reader announces. */
export const menuScreenReader =
  'The menu announces its name and that it is a menu, then the number of items. Each row ' +
  'announces its label, its role — menu item, check menu item or radio menu item — its checked ' +
  'state where it has one, its keyboard shortcut, and “has submenu, collapsed” where it opens one. ' +
  'An unavailable row announces “dimmed” and reads its reason; it is still reached by the arrow ' +
  'keys, which is the difference between a menu and a toolbar.';
