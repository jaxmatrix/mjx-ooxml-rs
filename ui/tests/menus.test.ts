import { describe, expect, test } from 'vitest';

import { tokens, type ColorScheme } from '../tokens/tokens.ts';
import { bodyTextMinimum, contrastRatio } from '../dev/contrast.ts';
import {
  controlStateNames,
  controlStateSpecs,
  effectiveStatePaint,
} from '../src/controls/control-states.ts';
import { accessibleHitTargetMinimum, densityModes } from '../src/foundations/density.ts';
import { motionRoleClass } from '../src/foundations/motion.ts';
import { tabStripPickerAtOrBelow } from '../src/ribbon/ribbon-model.ts';
import { containerName } from '../src/harness/presets.ts';
import {
  ariaKeyShortcuts,
  focusManagementPatterns,
  focusRestoringCloseReasons,
  longPressDelay,
  longPressMoveTolerance,
  menuCloseReasons,
  menuFocusPattern,
  menuGlyphColumnUnits,
  menuGlyphSize,
  menuItemCss,
  menuItemKinds,
  menuItemRoles,
  menuItemStateCascade,
  menuItemStateNames,
  menuItemStates,
  menuItemStatesCss,
  menuActions,
  menuItemTypeRoles,
  menuKeyAction,
  menuMotionClass,
  menuPresentationAt,
  menuPresentationCss,
  menuPresentationOrder,
  menuPresentations,
  menuSecondaryTextIsSizeNotColour,
  menuSheetAtOrBelow,
  nextTypeaheadIndex,
  sheetBoundaryFraction,
  resolvedMenuItemFingerprint,
  submenuHoverCloseDelay,
  submenuHoverOpenDelay,
  submenuTravelGrace,
  typeaheadResetDelay,
  type MenuItemState,
} from '../src/menus/menu-model.ts';
import {
  insetRect,
  intersectRects,
  physicalSide,
  placeFloating,
  pointInTriangle,
  travellingToward,
  type Direction,
  type LogicalSide,
  type Rect,
} from '../src/overlay/floating.ts';

/**
 * The menu model, checked before a browser is involved.
 *
 * `tests/browser/menus.spec.ts` measures what a browser computed and drives real key presses and a
 * real pointer, which is the claim that matters. This tier proves the things a browser is a slow
 * and indirect way of asking about:
 *
 * * **the geometry**, which is arithmetic and should be tested as arithmetic — flipping, shifting,
 *   the RTL mirror and the safe triangle each get a case here rather than a screenshot;
 * * **the model is internally distinct and internally legible in both schemes**, so a palette
 *   re-seed that collapses two states or pushes a row's text under 4.5 : 1 is caught in a second;
 * * **the stylesheet cannot acquire the specificity accident** it was written to avoid, for the
 *   fourth child running.
 *
 * ⚠ No colour, size or duration is written in this file. Every expectation is derived from the
 * generated tokens or from the model itself.
 */

const schemes: readonly ColorScheme[] = ['light', 'dark'];

// ── the state table ──────────────────────────────────────────────────────────

describe('the menu state table', () => {
  test('every state names a control state, and every selector is a template over the row', () => {
    for (const state of menuItemStateNames) {
      const spec = menuItemStates[state];
      expect(controlStateNames, `${state} names a state the control table does not have`).toContain(
        spec.controlState,
      );
      expect(spec.description.trim(), `${state} has no description`).not.toBe('');
      for (const match of spec.matches) {
        expect(match, `${state} has a selector that never mentions the row`).toContain('%s');
      }
    }
  });

  test('a menu has no hard-disabled state, and no indeterminate one', () => {
    // The behavioural requirement, asserted rather than described: `disabled` takes an element out
    // of the tab order by the platform's own doing, and a menu item that did that would be a
    // command a person can never find out exists. `mixed` is a selection's summary, which a single
    // command does not have.
    const claimed = menuItemStateNames.map((state) => menuItemStates[state].controlState);
    expect(claimed).not.toContain('disabled');
    expect(claimed).not.toContain('mixed');
    expect(claimed).not.toContain('mixedHover');
    expect(claimed).toContain('unavailable');
    // …and the state that *is* claimed is the reachable one: full opacity, and a dashed edge.
    const unavailable = effectiveStatePaint('unavailable');
    expect(unavailable.opacity).toBe(1);
    expect(unavailable.borderStyle).toBe('dashed');
  });

  test('the cascade is exactly the states that paint, once each, with unavailable last', () => {
    const painting = menuItemStateNames.filter((state) => menuItemStates[state].matches.length > 0);
    expect([...menuItemStateCascade].sort()).toEqual([...painting].sort());
    expect(new Set(menuItemStateCascade).size).toBe(menuItemStateCascade.length);
    expect(menuItemStateCascade).not.toContain('focus');
    expect(menuItemStates.focus.matches).toEqual([]);
    expect(menuItemStateCascade[menuItemStateCascade.length - 1]).toBe('unavailable');
  });

  test('an open submenu reads as the highlighted row rather than as an eighth state', () => {
    expect(menuItemStates.hover.matches.some((match) => match.includes('data-submenu-open'))).toBe(
      true,
    );
    // …and it is *hover* it folds into, not `checked`: an item holding its submenu open is where
    // the pointer is, not a command that is on.
    expect(menuItemStates.hover.controlState).toBe('hover');
  });

  test('the three kinds carry the three ARIA roles, and only two of them are checkable', () => {
    expect([...menuItemKinds]).toEqual(['command', 'checkbox', 'radio']);
    expect(menuItemRoles.command).toBe('menuitem');
    expect(menuItemRoles.checkbox).toBe('menuitemcheckbox');
    expect(menuItemRoles.radio).toBe('menuitemradio');
  });
});

describe('pairwise distinctness, on the model', () => {
  for (const scheme of schemes) {
    test(`no two menu states paint alike in ${scheme}`, () => {
      const seen = new Map<string, MenuItemState>();
      for (const state of menuItemStateNames) {
        const fingerprint = resolvedMenuItemFingerprint(state, scheme);
        const previous = seen.get(fingerprint);
        expect(
          previous,
          `a menu renders '${state}' and '${String(previous)}' identically in ${scheme}:\n` +
            `  ${fingerprint}\n` +
            'Two states that render alike pass every "the state exists" check.',
        ).toBeUndefined();
        seen.set(fingerprint, state);
      }
      expect(seen.size).toBe(menuItemStateNames.length);
    });
  }
});

/**
 * The contrast rule, applied to the surface where it actually bites.
 *
 * `DESIGN_TOKENS.md` §2.2's rule is enforced on *tokens* by the generator. This is the same rule
 * applied to a **composition**: a row's text is painted on the row's own fill, and a pairing that
 * is legal on the menu's surface can be illegal on the hover fill. The second test below is the
 * measurement that decided `menuSecondaryTextIsSizeNotColour`, and it is written as an assertion so
 * that the day a re-seed makes the secondary colour legal, this fails and says so.
 */
describe('every state a row can be in is legible in it', () => {
  const surface = (scheme: ColorScheme): string => tokens.theme[scheme].surfaceRaised;

  for (const scheme of schemes) {
    test(`the row's text clears ${String(bodyTextMinimum)} : 1 in every state, in ${scheme}`, () => {
      for (const state of menuItemStateNames) {
        const paint = effectiveStatePaint(menuItemStates[state].controlState);
        const behind =
          paint.background === 'transparent'
            ? surface(scheme)
            : tokens.theme[scheme][paint.background];
        const text = tokens.theme[scheme][paint.text];
        const ratio = contrastRatio(text, behind) ?? 0;
        expect(
          ratio,
          `a menu row in '${state}' paints ${text} on ${behind} in ${scheme}, which is ` +
            `${ratio.toFixed(2)} : 1. A row a person cannot read in one of its states is a row ` +
            'they cannot read exactly when they are pointing at it.',
        ).toBeGreaterThanOrEqual(bodyTextMinimum);
      }
    });
  }

  test('the secondary colour is illegal on the hover fill, which is why the hint is sized not tinted', () => {
    const secondary = tokens.theme.light.textSecondary;
    const hoverFill = tokens.theme.light[effectiveStatePaint('hover').background as 'borderSubtle'];
    const ratio = contrastRatio(secondary, hoverFill) ?? 0;
    expect(
      ratio,
      'the secondary text colour now clears the body-text minimum against the hover fill. If the ' +
        'palette was re-seeded, a greyed shortcut hint has become legal and menuItemTypeRoles can ' +
        'stop telling the hint apart by size alone — which is the *reason* this test exists ' +
        'rather than a comment.',
    ).toBeLessThan(bodyTextMinimum);
    expect(menuSecondaryTextIsSizeNotColour).toBe(true);
    // The hierarchy is therefore in the type scale: the row is `control`, the hint is `dense`.
    expect(menuItemTypeRoles.row).toBe('control');
    expect(menuItemTypeRoles.shortcut).toBe('dense');
    expect(menuItemTypeRoles.description).toBe('dense');
  });

  for (const scheme of schemes) {
    test(`a section heading is legible on the menu's own surface, in ${scheme}`, () => {
      // The one piece of secondary text a menu has, and it is legal because a heading is never
      // hovered — it is not in the arrow-key sequence and has no state at all.
      const ratio = contrastRatio(tokens.theme[scheme].textSecondary, surface(scheme)) ?? 0;
      expect(ratio).toBeGreaterThanOrEqual(bodyTextMinimum);
      expect(menuItemTypeRoles.sectionHeading).toBe('label');
    });
  }
});

// ── the stylesheet ───────────────────────────────────────────────────────────

describe('the stylesheet cannot acquire the specificity accident', () => {
  const css = menuItemStatesCss('.item');

  test('every state rule is wrapped in :where(), so source order is the only thing deciding', () => {
    const selectors = [...css.matchAll(/^([^{\n]+)\{/gm)].map((match) => (match[1] ?? '').trim());
    expect(selectors.length).toBe(menuItemStateCascade.length);
    for (const selector of selectors) {
      expect(
        selector.startsWith(':where(') && selector.endsWith(')'),
        `'${selector}' is not entirely inside :where(). MJXOFF-183 shipped a rule at (0,4,0) ` +
          'against rules at (0,2,0) and the test asserting emission order stayed green, because ' +
          'emission order only arbitrates between rules of equal specificity.',
      ).toBe(true);
      expect(selector.slice(7, -1)).not.toContain(':where(');
    }
  });

  test('the rules are emitted in cascade order', () => {
    const positions = menuItemStateCascade.map((state) => {
      const first = menuItemStates[state].matches[0] ?? '';
      return css.indexOf(first.replace('%s', '.item'));
    });
    for (const position of positions) expect(position).toBeGreaterThanOrEqual(0);
    expect([...positions].sort((left, right) => left - right)).toEqual(positions);
  });

  test('the row rule declares none of the properties the state table owns', () => {
    const owned = [
      'background',
      'border-color',
      'border-style',
      'color',
      'font-weight',
      'opacity',
      'box-shadow',
    ];
    const base = menuItemCss.slice(
      menuItemCss.indexOf('.item {'),
      menuItemCss.indexOf('}', menuItemCss.indexOf('.item {')),
    );
    for (const property of owned) {
      expect(
        base,
        `.item declares '${property}', which the state table owns. At (0,1,0) it out-specifies ` +
          'every (0,0,0) state rule, so a row would render its resting paint in all seven states ' +
          'while every "the state exists" check passed.',
      ).not.toContain(`${property}:`);
    }
    // …and it does own the box, so the split is a division rather than an emptying.
    expect(base).toContain('grid-template-columns:');
    expect(base).toContain('border-radius:');
    expect(base).toContain('padding-inline:');
  });

  test('every colour in the state rules is a token reference', () => {
    const colours = [...css.matchAll(/(?:background|border-color|color|box-shadow):\s*([^;]+);/g)];
    expect(colours.length).toBeGreaterThan(0);
    for (const [, value = ''] of colours) {
      const trimmed = value.trim();
      if (trimmed === 'transparent' || trimmed === 'none') continue;
      expect(trimmed, `'${trimmed}' is not a token reference`).toContain('var(--theme-');
    }
  });
});

describe('the presentation rules', () => {
  const css = menuPresentationCss();

  test('every selector is wrapped, and the three blocks are emitted in refinement order', () => {
    const selectors = [...css.matchAll(/^\s*(\.?[^{\n@]*?)\s*\{/gm)]
      .map((match) => (match[1] ?? '').trim())
      .filter((selector) => selector !== '');
    expect(selectors.length).toBe(menuPresentationOrder.length);
    for (const selector of selectors) {
      expect(
        selector.startsWith(':where(') && selector.endsWith(')'),
        `'${selector}' is not entirely inside :where()`,
      ).toBe(true);
    }
    const base = css.indexOf(':where(.menu) {');
    const floating = css.indexOf(':where(.menu[data-floating]) {');
    const sheet = css.lastIndexOf(':where(.menu[data-floating]) {');
    expect(base).toBeGreaterThanOrEqual(0);
    expect(floating).toBeGreaterThan(base);
    expect(sheet).toBeGreaterThan(floating);
  });

  test('the sheet block is a container query on the harness frame, at the ribbon’s own width', () => {
    expect(css).toContain(`@container ${containerName} (width <= ${String(menuSheetAtOrBelow)}px)`);
    // Read, not restated: the width at which the tab strip becomes a picker is the width at which
    // a floating list becomes a sheet, and two numbers that must agree are written once.
    expect(menuSheetAtOrBelow).toBe(tabStripPickerAtOrBelow);
  });

  test('only the floating one is anchored, and only the two popups are fixed', () => {
    expect(menuPresentations.inline.position).toBe('static');
    expect(menuPresentations.floating.position).toBe('fixed');
    expect(menuPresentations.sheet.position).toBe('fixed');
    // A sheet is pinned to an edge of the boundary rather than placed beside an anchor — a
    // different question from *which side of this button*, and the one the component branches on.
    expect(menuPresentations.floating.anchored).toBe(true);
    expect(menuPresentations.sheet.anchored).toBe(false);
    expect(menuPresentations.inline.anchored).toBe(false);
    // …and a sheet may not cover the whole of what it is pinned to, or there is nothing left to
    // dismiss it by tapping.
    expect(sheetBoundaryFraction).toBeGreaterThan(0);
    expect(sheetBoundaryFraction).toBeLessThan(1);
  });

  test('each presentation wears the motion role §4 names for it', () => {
    expect(menuMotionClass('sheet')).toBe(motionRoleClass('sheetEnter'));
    expect(menuMotionClass('floating')).toBe(motionRoleClass('panelEnter'));
    for (const presentation of menuPresentationOrder) {
      // Nothing in a menu is attached to a document object, so overshoot is legal here — the rule
      // §4 states is about handles and selections, and `motion.ts` is what enforces it.
      expect(menuPresentations[presentation].motionRole).toBeDefined();
    }
  });

  test('a menu is a sheet exactly at and below the declared width', () => {
    expect(menuPresentationAt(1440, true)).toBe('floating');
    expect(menuPresentationAt(menuSheetAtOrBelow + 1, true)).toBe('floating');
    expect(menuPresentationAt(menuSheetAtOrBelow, true)).toBe('sheet');
    expect(menuPresentationAt(390, true)).toBe('sheet');
    // Without an invoker there is no popup to present, at any width.
    expect(menuPresentationAt(390, false)).toBe('inline');
    expect(menuPresentationAt(1440, false)).toBe('inline');
  });

  test('the mark gutter is the glyph box expressed in spacing units', () => {
    // `--spacing` is a rem value; the glyph is 16 CSS pixels; the column is the multiple that makes
    // the two the same at the default root size. If a re-seed moves `--spacing`, this says so.
    const spacingPixels = Number.parseFloat(tokens.spacing) * 16;
    expect(menuGlyphColumnUnits * spacingPixels).toBe(menuGlyphSize);
  });
});

// ── the keyboard ─────────────────────────────────────────────────────────────

describe('the ARIA menu keyboard model', () => {
  const ltr = { direction: 'ltr' as Direction, hasSubmenu: false, isSubmenu: false, typing: false };

  test('the movement keys', () => {
    expect(menuKeyAction('ArrowDown', ltr)).toBe('next');
    expect(menuKeyAction('ArrowUp', ltr)).toBe('previous');
    expect(menuKeyAction('Home', ltr)).toBe('first');
    expect(menuKeyAction('End', ltr)).toBe('last');
    expect(menuKeyAction('PageUp', ltr)).toBe('first');
    expect(menuKeyAction('PageDown', ltr)).toBe('last');
  });

  test('Tab leaves — a menu is not a focus trap', () => {
    expect(menuKeyAction('Tab', ltr)).toBe('leave');
    expect(menuKeyAction('Tab', { ...ltr, isSubmenu: true })).toBe('leave');
    // The rule that decides it, asserted so the two popups of MJXOFF-183 can be compared against it.
    expect(menuFocusPattern).toBe('roving');
    expect(focusManagementPatterns.roving.tabStops).toBe('one');
    expect(focusManagementPatterns.trap.tabStops).toBe('many');
  });

  test('Escape closes one level, wherever it is pressed', () => {
    expect(menuKeyAction('Escape', ltr)).toBe('close');
    expect(menuKeyAction('Escape', { ...ltr, isSubmenu: true })).toBe('close');
  });

  test('the inline arrows mirror under RTL, and do nothing when there is nothing to do', () => {
    const withSubmenu = { ...ltr, hasSubmenu: true };
    expect(menuKeyAction('ArrowRight', withSubmenu)).toBe('openSubmenu');
    expect(menuKeyAction('ArrowLeft', { ...withSubmenu, isSubmenu: true })).toBe('closeSubmenu');

    const rtl = { ...withSubmenu, direction: 'rtl' as Direction };
    expect(menuKeyAction('ArrowLeft', rtl)).toBe('openSubmenu');
    expect(menuKeyAction('ArrowRight', { ...rtl, isSubmenu: true })).toBe('closeSubmenu');

    // An item with no submenu, and a top-level menu with no level to close: neither key does
    // anything, and neither may be swallowed.
    expect(menuKeyAction('ArrowRight', ltr)).toBeUndefined();
    expect(menuKeyAction('ArrowLeft', ltr)).toBeUndefined();
  });

  test('Enter activates; Space activates unless a type-ahead is in flight', () => {
    expect(menuKeyAction('Enter', ltr)).toBe('activate');
    expect(menuKeyAction(' ', ltr)).toBe('activate');
    expect(menuKeyAction(' ', { ...ltr, typing: true })).toBe('typeahead');
  });

  test('every answer it can give is one the model names', () => {
    const keys = ['ArrowDown', 'ArrowUp', 'Home', 'End', 'PageUp', 'PageDown', 'Enter', ' ',
      'Escape', 'Tab', 'ArrowLeft', 'ArrowRight', 'p', 'Shift', 'F5'];
    for (const direction of ['ltr', 'rtl'] as Direction[]) {
      for (const hasSubmenu of [true, false]) {
        for (const isSubmenu of [true, false]) {
          for (const typing of [true, false]) {
            for (const key of keys) {
              const action = menuKeyAction(key, { direction, hasSubmenu, isSubmenu, typing });
              if (action === undefined) continue;
              expect(menuActions, `'${key}' produced '${action}'`).toContain(action);
            }
          }
        }
      }
    }
  });

  test('a printable character types; a named key does not', () => {
    expect(menuKeyAction('p', ltr)).toBe('typeahead');
    expect(menuKeyAction('7', ltr)).toBe('typeahead');
    expect(menuKeyAction('Shift', ltr)).toBeUndefined();
    expect(menuKeyAction('F5', ltr)).toBeUndefined();
    expect(menuKeyAction('Control', ltr)).toBeUndefined();
  });
});

describe('type-ahead', () => {
  const labels = ['Paste', 'Paste Special', 'Print', 'Cut', 'Copy'];

  test('a single character moves to the *next* match, and a growing prefix then stays put', () => {
    // The ARIA menu pattern's own wording: a character moves focus to the **next** item whose name
    // starts with it. So a first `p` pressed while the keyboard is on `Paste` finds `Paste
    // Special`, which is what every desktop menu has done since the Windows 95 shell.
    expect(nextTypeaheadIndex(labels, 'p', 0)).toBe(1);
    // …and once the buffer is a real prefix it must be able to stay where it is, or the second
    // letter of a word would walk off the item the first letter just found.
    expect(nextTypeaheadIndex(labels, 'pa', 1)).toBe(1);
    expect(nextTypeaheadIndex(labels, 'paste s', 1)).toBe(1);
    expect(nextTypeaheadIndex(labels, 'pr', 1)).toBe(2);
  });

  test('a repeated character cycles through the items that start with it', () => {
    expect(nextTypeaheadIndex(labels, 'p', 0)).toBe(1);
    expect(nextTypeaheadIndex(labels, 'pp', 1)).toBe(2);
    // …and wraps back round to the first.
    expect(nextTypeaheadIndex(labels, 'ppp', 2)).toBe(0);
    expect(nextTypeaheadIndex(labels, 'pppp', 0)).toBe(1);
  });

  test('it is case-insensitive, wraps, and reports no match honestly', () => {
    expect(nextTypeaheadIndex(labels, 'CU', 3)).toBe(3);
    expect(nextTypeaheadIndex(labels, 'c', 4)).toBe(3);
    expect(nextTypeaheadIndex(labels, 'z', 0)).toBe(-1);
    expect(nextTypeaheadIndex([], 'a', 0)).toBe(-1);
    expect(nextTypeaheadIndex(labels, '', 0)).toBe(-1);
  });

  test('the interaction timings are platform constants, not design tokens', () => {
    // Every one of these is a number of milliseconds in JavaScript rather than a CSS duration, and
    // that is deliberate: a re-seed of the palette must not change how long a person has to hold
    // still or how long they have to type the second letter of a command's name.
    expect(typeaheadResetDelay).toBeGreaterThan(0);
    expect(submenuHoverOpenDelay).toBeGreaterThan(0);
    expect(submenuHoverCloseDelay).toBeGreaterThan(0);
    expect(longPressDelay).toBeGreaterThan(submenuHoverOpenDelay);
    expect(longPressMoveTolerance).toBeGreaterThan(0);
    expect(submenuTravelGrace).toBeGreaterThan(0);
  });

  test('`Ctrl+V` is announced as `Control+V`', () => {
    expect(ariaKeyShortcuts('Ctrl+V')).toBe('Control+V');
    expect(ariaKeyShortcuts('Ctrl+Alt+V')).toBe('Control+Alt+V');
    expect(ariaKeyShortcuts('Cmd+Shift+P')).toBe('Meta+Shift+P');
    // Anything the table does not know is passed through rather than guessed at.
    expect(ariaKeyShortcuts('F12')).toBe('F12');
  });
});

// ── the geometry ─────────────────────────────────────────────────────────────

const boundary: Rect = { x: 0, y: 0, width: 1000, height: 600 };
const size = { width: 200, height: 300 };

function place(anchor: Rect, side: LogicalSide, direction: Direction = 'ltr'): ReturnType<typeof placeFloating> {
  return placeFloating({
    anchor,
    floating: size,
    boundary,
    side,
    align: 'start',
    direction,
    gap: 4,
  });
}

describe('placement', () => {
  test('the preferred side is taken when it fits', () => {
    const result = place({ x: 100, y: 100, width: 80, height: 30 }, 'blockEnd');
    expect(result.side).toBe('bottom');
    expect(result.flipped).toBe(false);
    expect(result.y).toBe(134);
    expect(result.x).toBe(100);
  });

  test('it flips in all four directions when the preferred side has no room', () => {
    // Down → up.
    expect(place({ x: 100, y: 550, width: 80, height: 30 }, 'blockEnd').side).toBe('top');
    // Up → down.
    expect(place({ x: 100, y: 10, width: 80, height: 30 }, 'blockStart').side).toBe('bottom');
    // End → start, in a left-to-right line.
    expect(place({ x: 900, y: 100, width: 80, height: 30 }, 'inlineEnd').side).toBe('left');
    // Start → end.
    expect(place({ x: 10, y: 100, width: 80, height: 30 }, 'inlineStart').side).toBe('right');
    for (const side of ['blockEnd', 'blockStart', 'inlineEnd', 'inlineStart'] as LogicalSide[]) {
      const anchor: Rect =
        side === 'blockEnd'
          ? { x: 100, y: 550, width: 80, height: 30 }
          : side === 'blockStart'
            ? { x: 100, y: 10, width: 80, height: 30 }
            : side === 'inlineEnd'
              ? { x: 900, y: 100, width: 80, height: 30 }
              : { x: 10, y: 100, width: 80, height: 30 };
      expect(place(anchor, side).flipped, `${side} did not flip`).toBe(true);
    }
  });

  test('a flip that buys nothing is not taken', () => {
    // Neither side has room for a 300px box in a 200px boundary, so the preferred side is kept and
    // the caller is told to scroll rather than being moved away from the thing it belongs to.
    const shallow: Rect = { x: 0, y: 0, width: 1000, height: 200 };
    const result = placeFloating({
      anchor: { x: 100, y: 100, width: 80, height: 30 },
      floating: size,
      boundary: shallow,
      side: 'blockEnd',
      align: 'start',
      direction: 'ltr',
      gap: 4,
    });
    expect(result.side).toBe('top');
    // …and either way it says so.
    expect(result.constrained).toBe(true);
    expect(result.maxBlockSize).toBeLessThan(size.height);
  });

  test('the cross axis shifts rather than overflowing, and says that it did', () => {
    const flush = place({ x: 100, y: 100, width: 80, height: 30 }, 'blockEnd');
    expect(flush.shifted).toBe(false);
    const near = place({ x: 950, y: 100, width: 40, height: 30 }, 'blockEnd');
    expect(near.shifted).toBe(true);
    expect(near.x + size.width).toBeLessThanOrEqual(boundary.x + boundary.width);
  });

  test('under RTL the inline sides mirror and `start` means the anchor’s right edge', () => {
    expect(physicalSide('inlineEnd', 'ltr')).toBe('right');
    expect(physicalSide('inlineEnd', 'rtl')).toBe('left');
    // The block axis does not mirror: a menu below its button is below it in every language.
    expect(physicalSide('blockEnd', 'rtl')).toBe('bottom');

    const anchor: Rect = { x: 400, y: 100, width: 120, height: 30 };
    const leftToRight = place(anchor, 'blockEnd', 'ltr');
    const rightToLeft = place(anchor, 'blockEnd', 'rtl');
    expect(leftToRight.x).toBe(anchor.x);
    expect(rightToLeft.x).toBe(anchor.x + anchor.width - size.width);
  });

  test('nothing ever leaves the boundary, wherever the anchor is', () => {
    for (const x of [-50, 0, 500, 990, 1200]) {
      for (const y of [-50, 0, 300, 590, 900]) {
        for (const side of ['blockEnd', 'blockStart', 'inlineEnd', 'inlineStart'] as LogicalSide[]) {
          const result = place({ x, y, width: 10, height: 10 }, side);
          expect(result.x).toBeGreaterThanOrEqual(boundary.x);
          expect(result.y).toBeGreaterThanOrEqual(boundary.y);
          expect(result.x + size.width).toBeLessThanOrEqual(boundary.x + boundary.width);
          expect(result.y + size.height).toBeLessThanOrEqual(boundary.y + boundary.height);
        }
      }
    }
  });

  test('a boundary is the overlap of what clips, pulled in by its inset', () => {
    expect(intersectRects({ x: 0, y: 0, width: 100, height: 100 }, { x: 50, y: 50, width: 100, height: 100 })).toEqual(
      { x: 50, y: 50, width: 50, height: 50 },
    );
    // No overlap is zero-sized rather than negative.
    expect(intersectRects({ x: 0, y: 0, width: 10, height: 10 }, { x: 90, y: 90, width: 10, height: 10 })).toEqual(
      { x: 90, y: 90, width: 0, height: 0 },
    );
    expect(insetRect({ x: 0, y: 0, width: 100, height: 100 }, 8)).toEqual({
      x: 8,
      y: 8,
      width: 84,
      height: 84,
    });
  });
});

/**
 * The safe triangle.
 *
 * MJXOFF-184 asks for this to be *"asserted with a simulated pointer path across a sibling item"*
 * and for it to be provable that it can fail. The browser gate drives a real pointer; this is the
 * arithmetic underneath it, where a path can be written down exactly.
 */
describe('diagonal travel toward a submenu', () => {
  /*
   * A menu 200 wide whose second row is at y 100–132, and the submenu that row opened, four pixels
   * to its right.
   *
   * The apex is **inside the parent row**, not on its edge, and that is the whole geometry rather
   * than a detail of the fixture: the pointer that is about to cross a sibling is still inside the
   * parent menu when it starts moving, so the wedge that has to be protected runs from where it was
   * *in the row* to the submenu's near edge. An apex taken on the row's outer edge would make the
   * triangle four pixels wide and the tolerance would protect nothing — which is exactly the way
   * this feature is usually written and does not work.
   */
  const submenu: Rect = { x: 204, y: 100, width: 180, height: 220 };
  const exit = { x: 120, y: 131 };

  test('a diagonal from the parent row to the submenu is travel', () => {
    for (const step of [
      { x: 150, y: 150 },
      { x: 180, y: 190 },
      { x: 199, y: 240 },
    ]) {
      expect(
        travellingToward(step, exit, submenu, 'right', submenuTravelGrace),
        `(${String(step.x)}, ${String(step.y)}) is on its way to the submenu and was read as ` +
          'leaving. This is the assertion the whole tolerance exists for.',
      ).toBe(true);
    }
  });

  test('a move straight down onto the sibling is not travel', () => {
    expect(travellingToward({ x: 120, y: 200 }, exit, submenu, 'right', submenuTravelGrace)).toBe(
      false,
    );
    expect(travellingToward({ x: 120, y: 320 }, exit, submenu, 'right', submenuTravelGrace)).toBe(
      false,
    );
  });

  test('moving away from the submenu is never travel toward it, whatever the triangle says', () => {
    expect(travellingToward({ x: 60, y: 150 }, exit, submenu, 'right', submenuTravelGrace)).toBe(
      false,
    );
  });

  test('a submenu with no box yet, and a pointer that has not moved, are both refused', () => {
    expect(
      travellingToward({ x: 260, y: 220 }, exit, { x: 0, y: 0, width: 0, height: 0 }, 'right', 8),
    ).toBe(false);
    expect(travellingToward(exit, exit, submenu, 'right', submenuTravelGrace)).toBe(false);
  });

  test('it mirrors for a submenu that opened to the left', () => {
    const mirrored: Rect = { x: -184, y: 100, width: 180, height: 220 };
    const leftExit = { x: 80, y: 131 };
    expect(travellingToward({ x: 0, y: 180 }, leftExit, mirrored, 'left', submenuTravelGrace)).toBe(
      true,
    );
    expect(travellingToward({ x: 80, y: 280 }, leftExit, mirrored, 'left', submenuTravelGrace)).toBe(
      false,
    );
  });

  test('the triangle test itself is a triangle test', () => {
    const a = { x: 0, y: 0 };
    const b = { x: 10, y: 0 };
    const c = { x: 0, y: 10 };
    expect(pointInTriangle({ x: 2, y: 2 }, a, b, c)).toBe(true);
    expect(pointInTriangle({ x: 9, y: 9 }, a, b, c)).toBe(false);
  });

  test('the grace is what saves a pointer aimed at the submenu’s very first row', () => {
    expect(submenuTravelGrace).toBeLessThan(submenu.height);
    const aimedAtTheFirstRow = { x: 200, y: 96 };
    const apex = { x: 120, y: 110 };
    // Slightly above the submenu's top edge, which is where a pointer aiming at its first item
    // actually goes — and with no grace at all the triangle's upper edge is below that point, so
    // the submenu would close on the way to the item the person is reaching for.
    expect(travellingToward(aimedAtTheFirstRow, apex, submenu, 'right', submenuTravelGrace)).toBe(
      true,
    );
    expect(travellingToward(aimedAtTheFirstRow, apex, submenu, 'right', 0)).toBe(false);
  });
});

// ── the contract the gates read ──────────────────────────────────────────────

describe('the catalogue contract', () => {
  test('the close reasons that restore focus are the three MJXOFF-184 names', () => {
    expect([...focusRestoringCloseReasons]).toEqual(['escape', 'activate', 'outside']);
    for (const reason of focusRestoringCloseReasons) expect(menuCloseReasons).toContain(reason);
    // `blur` never restores: focus has already gone somewhere a person chose.
    expect(focusRestoringCloseReasons).not.toContain('blur');
  });

  test('a compact menu still clears the accessible hit-target floor', () => {
    // The row wears `.mjx-hit-target`, which clamps to the WCAG minimum, and compact's own target
    // is above it — so the floor holds by construction *and* by the density mode's own number.
    expect(menuItemCss).toContain('padding-block');
    const compact = densityModes.compact.hitTargetUnits * Number.parseFloat(tokens.spacing) * 16;
    expect(compact).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
  });

  test('every menu state maps onto a control state that the shared table specifies', () => {
    for (const state of menuItemStateNames) {
      expect(controlStateSpecs[menuItemStates[state].controlState]).toBeDefined();
    }
  });
});
