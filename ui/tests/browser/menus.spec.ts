import { expect, test, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { tokens, type ColorScheme } from '../../tokens/tokens.ts';
import { accessibleHitTargetMinimum } from '../../src/foundations/density.ts';
import {
  focusManagementPatterns,
  menuFocusPattern,
  menuItemRoles,
  menuItemStateNames,
  menuItemPaint,
  menuItemParts,
  menuPresentationAt,
  menuPresentations,
  menuSheetAtOrBelow,
  menuStateCellAttribute,
  menuStatesMatrixStoryName,
  menuStoryTitles,
  submenuHoverCloseDelay,
  submenuHoverOpenDelay,
  type MenuItemState,
} from '../../src/menus/menu-model.ts';
import { placeFloating, type Align, type Direction, type LogicalSide, type Placement, type Rect } from '../../src/overlay/floating.ts';

/**
 * Menus, measured and driven in a browser.
 *
 * MJXOFF-184 names the trap and it is the reason almost nothing below is a snapshot:
 *
 * > Every menu behaviour that matters is **temporal or positional**, and both are invisible to a
 * > static story. A menu that opens instantly on hover, never flips at an edge, and drops focus on
 * > close will look perfect in a screenshot and be unpleasant to use. **So the gates are
 * > interaction tests** — hover intent with timing, edge flipping at a constrained container, and
 * > focus restoration — not appearance snapshots.
 *
 * And MJXOFF-182's rule decides the shape of the two that *are* about appearance:
 *
 * > **A distinctness gate proves no two states are the same; it does not prove any of them is
 * > right.**
 *
 * So the paint suite asserts **correspondence** — every cell's computed paint equals
 * `effectiveStatePaint()`, the same model function `<mjx-button>`'s gate reads — and distinctness
 * separately. And the placement suite re-runs `placeFloating()` **in Node** over the component's
 * own recorded inputs and requires the same answer, which is the only comparison that can catch a
 * placement that is wrong in the component and in the model at once.
 */

/*
 * The titles come from the model, and the story files spell them out again as literals — CSF is
 * indexed statically and refuses a computed `title`. `open()` fails with the name rather than with
 * `undefined` when the two diverge, which is the same arrangement `controls.spec.ts` uses.
 */
const menu = menuStoryTitles.menu;
const context = menuStoryTitles.contextMenu;

const matrixStory = { title: menu, name: menuStatesMatrixStoryName } as const;
const anatomyStory = { title: menu, name: 'The Item Anatomy' } as const;
const travelStory = { title: menu, name: 'Submenus And Diagonal Travel' } as const;
const placementStory = { title: menu, name: 'Positioning And Flipping' } as const;
const rtlStory = { title: menu, name: 'Under Right To Left' } as const;
const compactStory = { title: menu, name: 'In Compact Density' } as const;
const splitStory = { title: menu, name: 'Opened By A Split Button' } as const;
const canvasStory = { title: context, name: 'On A Canvas' } as const;
const phoneStory = { title: context, name: 'On A Phone' } as const;

const schemes: readonly ColorScheme[] = ['light', 'dark'];
const transitionMilliseconds = Number.parseFloat(tokens.duration.transition);

async function open(
  page: Page,
  story: { readonly title: string; readonly name: string },
  options: { theme?: ColorScheme; containerPreset?: string } = {},
): Promise<void> {
  const entry = builtStories().find(
    (candidate) => candidate.title === story.title && candidate.name === story.name,
  );
  expect(entry, `${story.title} · ${story.name} is missing from the catalogue`).toBeDefined();
  if (entry === undefined) return;
  await openStory(page, entry.id, options);
  await page.waitForFunction(() => customElements.get('mjx-menu') !== undefined);
  await settle(page);
}

async function settle(page: Page): Promise<void> {
  await page.evaluate(async () => {
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
}

/** Two frames is a layout change; a paint change needs the transition the token declares. */
async function settlePaint(page: Page): Promise<void> {
  await settle(page);
  await page.waitForTimeout(transitionMilliseconds * 2 + 100);
}

/** `#2e9e63` → `rgb(46, 158, 99)`; `transparent` → what a browser reports for it. */
function expectedColor(member: string | undefined, scheme: ColorScheme): string {
  if (member === undefined || member === 'transparent') return 'rgba(0, 0, 0, 0)';
  const hex = tokens.theme[scheme][member as keyof (typeof tokens.theme)['light']];
  const number = Number.parseInt(hex.slice(1, 7), 16);
  return `rgb(${String((number >> 16) & 0xff)}, ${String((number >> 8) & 0xff)}, ${String(number & 0xff)})`;
}

interface Paint {
  background: string;
  borderColor: string;
  borderStyle: string;
  color: string;
  weight: string;
  opacity: string;
  shadow: string;
  outline: string;
  minBlock: number;
  labelWeight: string;
}

const readPaint = `(element) => {
  const style = getComputedStyle(element);
  return {
    background: style.backgroundColor,
    borderColor: style.borderTopColor,
    borderStyle: style.borderTopStyle,
    color: style.color,
    weight: style.fontWeight,
    opacity: style.opacity,
    shadow: style.boxShadow,
    outline: style.outlineStyle + ' ' + style.outlineColor + ' ' + style.outlineWidth,
    minBlock: parseFloat(style.minBlockSize) || 0,
    /*
     * ⚠ The weight of the **label**, not only of the row it sits in.
     *
     * Reading it off the row alone leaves a hole big enough to drive MJXOFF-182's accident through
     * a fourth time: a type-role class on the label declares its own font-weight, so the row can
     * report the state's bold while the words a person actually reads render medium. This gate was
     * green under exactly that break until the label was measured too.
     */
    labelWeight: getComputedStyle(element.querySelector('.label')).fontWeight,
  };
}`;

/**
 * Every matrix cell, and the paint of the row inside it.
 *
 * ⚠ The selector is `[data-menu-state-cell]` and the reader throws when a cell holds no row, so an
 * empty sweep fails loudly rather than passing with nothing — MJXOFF-182's byte-budget lesson,
 * which measured 342 bytes of a 14 kB payload and reported success.
 */
async function readCells(page: Page): Promise<{ state: MenuItemState; paint: Paint }[]> {
  return page.evaluate(([reader, attribute]) => {
    const read = new Function(`return ${reader}`)() as (element: Element) => unknown;
    const cells = [...document.querySelectorAll(`[${attribute}]`)];
    if (cells.length === 0) throw new Error('the states matrix marked no cells');
    return cells.map((cell) => {
      const host = cell.querySelector('mjx-menu-item');
      const row = host?.shadowRoot?.querySelector('.item');
      if (row === null || row === undefined) {
        throw new Error(`the cell for '${cell.getAttribute(attribute) ?? '?'}' has no row`);
      }
      // ⚠ The focus ring is on the **host**: the row inside the shadow root is presentational and
      // the thing that holds the roving tabindex is `<mjx-menu-item>` itself. Reading the outline
      // off the row would report `none` for every state and make `focus` indistinguishable from
      // `rest` — which is exactly what this gate found on its first run.
      const paint = read(row) as Record<string, unknown>;
      const hostStyle = getComputedStyle(host as HTMLElement);
      paint['outline'] =
        hostStyle.outlineStyle + ' ' + hostStyle.outlineColor + ' ' + hostStyle.outlineWidth;
      return {
        state: cell.getAttribute(attribute) as never,
        paint: paint as never,
      };
    });
  }, [readPaint, menuStateCellAttribute] as const);
}

const activeDescriptor = `() => {
  let element = document.activeElement;
  while (element && element.shadowRoot && element.shadowRoot.activeElement) {
    element = element.shadowRoot.activeElement;
  }
  const cell = element && element.closest ? element.closest('[data-menu-state-cell]') : null;
  return {
    tag: element ? element.tagName.toLowerCase() : null,
    id: element ? element.id : '',
    label: element ? element.getAttribute('label') : null,
    role: element ? element.getAttribute('role') : null,
    tabIndex: element instanceof HTMLElement ? element.tabIndex : null,
    checked: element ? element.getAttribute('aria-checked') : null,
    disabled: element ? element.getAttribute('aria-disabled') : null,
    cell: cell ? cell.getAttribute('data-menu-state-cell') : null,
    menuLabel: element && element.closest ? (element.closest('mjx-menu')?.getAttribute('label') ?? null) : null,
  };
}`;

interface Active {
  tag: string | null;
  id: string;
  label: string | null;
  role: string | null;
  tabIndex: number | null;
  checked: string | null;
  disabled: string | null;
  cell: string | null;
  menuLabel: string | null;
}

async function focused(page: Page): Promise<Active> {
  return page.evaluate(
    (source) => (new Function(`return ${source}`)() as () => unknown)() as never,
    activeDescriptor,
  );
}

async function tabUntil(
  page: Page,
  predicate: (active: Active) => boolean,
  limit = 24,
): Promise<{ reached: boolean }> {
  for (let step = 0; step < limit; step += 1) {
    await page.keyboard.press('Tab');
    await settle(page);
    if (predicate(await focused(page))) return { reached: true };
  }
  return { reached: false };
}

// ── the paint ────────────────────────────────────────────────────────────────

test.describe('the states matrix, on what the browser computed', () => {
  for (const scheme of schemes) {
    test(`${scheme}: what it computed is what the model says`, async ({ page }) => {
      await open(page, matrixStory, { theme: scheme });
      const cells = await readCells(page);
      expect(
        cells.map((cell) => cell.state),
        'the matrix does not show the states the model claims. If this found nothing at all, the ' +
          'story stopped marking its cells and the gate was about to sweep an empty list.',
      ).toEqual([...menuItemStateNames]);

      for (const cell of cells) {
        // `focus` paints nothing of its own — it is the foundations' ring, asserted separately.
        if (cell.state === 'focus') continue;
        const model = menuItemPaint(cell.state);
        const where = `${cell.state} · ${scheme}`;
        expect(cell.paint.background, `${where}: fill`).toBe(expectedColor(model.background, scheme));
        expect(cell.paint.borderColor, `${where}: edge`).toBe(
          expectedColor(model.borderColor, scheme),
        );
        expect(cell.paint.borderStyle, `${where}: edge style`).toBe(model.borderStyle);
        expect(cell.paint.color, `${where}: text`).toBe(expectedColor(model.text, scheme));
        expect(
          cell.paint.weight,
          `${where}: font-weight is ${cell.paint.weight}, model says ${model.weight}. This is the ` +
            'assertion MJXOFF-182 broke on purpose to prove a distinctness gate is not enough.',
        ).toBe(String(model.weight === 'bold' ? tokens.fontWeight.bold : tokens.fontWeight.medium));
        expect(
          cell.paint.labelWeight,
          `${where}: the row is ${cell.paint.weight} and the label inside it is ` +
            `${cell.paint.labelWeight}. The words a person reads are the ones that have to carry ` +
            'the state.',
        ).toBe(cell.paint.weight);
        expect(Number.parseFloat(cell.paint.opacity), `${where}: opacity`).toBeCloseTo(
          model.opacity,
          2,
        );
        if (model.insetRing === undefined) {
          expect(cell.paint.shadow, `${where}: inset ring`).toBe('none');
        } else {
          expect(cell.paint.shadow, `${where}: inset ring`).toContain(
            expectedColor(model.insetRing, scheme),
          );
        }
      }
    });

    test(`${scheme}: every pair of states differs`, async ({ page }) => {
      await open(page, matrixStory, { theme: scheme });
      const cells = await readCells(page);

      // `focus` cannot be forced: it is produced by pressing Tab, through the browser's own input
      // path, so `:focus-visible` makes its own judgement rather than being asserted into being.
      await page.locator('body').click({ position: { x: 1, y: 1 } });
      const { reached } = await tabUntil(page, (active) => active.cell === 'focus');
      expect(reached, 'Tab never reached the focus cell').toBe(true);
      const focusedCells = await readCells(page);

      const seen = new Map<string, MenuItemState>();
      for (const [index, cell] of cells.entries()) {
        const paint = cell.state === 'focus' ? focusedCells[index]?.paint : cell.paint;
        expect(paint, `no paint for '${cell.state}'`).toBeDefined();
        if (paint === undefined) continue;
        const print = [
          paint.background,
          paint.borderColor,
          paint.borderStyle,
          paint.color,
          paint.weight,
          paint.opacity,
          paint.shadow,
          paint.outline,
        ].join(' | ');
        const previous = seen.get(print);
        expect(
          previous,
          `a menu renders '${cell.state}' and '${String(previous)}' identically in ${scheme}:\n` +
            `  ${print}`,
        ).toBeUndefined();
        seen.set(print, cell.state);
      }
      expect(seen.size).toBe(menuItemStateNames.length);
    });
  }

  test('the keyboard highlight is the foundations’ ring and nothing of the menu’s own', async ({
    page,
  }) => {
    await open(page, matrixStory);
    await page.locator('body').click({ position: { x: 1, y: 1 } });
    const { reached } = await tabUntil(page, (active) => active.cell === 'focus');
    expect(reached).toBe(true);
    const ring = await page.evaluate(() => {
      const cell = document.querySelector('[data-menu-state-cell="focus"] mjx-menu-item');
      if (!(cell instanceof HTMLElement)) return null;
      const style = getComputedStyle(cell);
      return { style: style.outlineStyle, colour: style.outlineColor, width: style.outlineWidth };
    });
    expect(ring, 'the focus cell has no row').not.toBeNull();
    expect(ring?.style, 'a keyboard-focused menu row has no ring').not.toBe('none');
    expect(Number.parseFloat(ring?.width ?? '0')).toBeGreaterThan(0);
    // The colour is the foundations' — not a UA default and not one the menu invented.
    expect(ring?.colour).toBe(expectedColor('accentPressed', 'light'));
  });
});

// ── the anatomy and the ARIA ─────────────────────────────────────────────────

test.describe('the item anatomy and the ARIA menu roles', () => {
  test('every kind carries its role, its state, and the mark that goes with it', async ({
    page,
  }) => {
    await open(page, anatomyStory);
    const reading = await page.evaluate(() => {
      const menu = document.querySelector('mjx-menu');
      const box = menu?.shadowRoot?.querySelector('.menu');
      const rows = [...(menu?.querySelectorAll('mjx-menu-item') ?? [])];
      const sections = [...(menu?.querySelectorAll('mjx-menu-section') ?? [])];
      const separators = [...(menu?.querySelectorAll('mjx-menu-separator') ?? [])];
      return {
        menuRole: box?.getAttribute('role') ?? null,
        menuName: box?.getAttribute('aria-label') ?? null,
        orientation: box?.getAttribute('aria-orientation') ?? null,
        sections: sections.map((section) => ({
          role: section.getAttribute('role'),
          label: section.getAttribute('aria-label'),
          headingHidden:
            section.shadowRoot?.querySelector('.heading')?.getAttribute('aria-hidden') ?? null,
        })),
        separators: separators.map((rule) => rule.getAttribute('role')),
        rows: rows.map((row) => ({
          label: row.getAttribute('label'),
          role: row.getAttribute('role'),
          checked: row.getAttribute('aria-checked'),
          disabled: row.getAttribute('aria-disabled'),
          haspopup: row.getAttribute('aria-haspopup'),
          expanded: row.getAttribute('aria-expanded'),
          keys: row.getAttribute('aria-keyshortcuts'),
          name: row.getAttribute('aria-label'),
          mark: (row.shadowRoot?.querySelector('.mark mjx-icon')?.getAttribute('name') ?? null),
          arrow: (row.shadowRoot?.querySelector('.arrow mjx-icon')?.getAttribute('name') ?? null),
          shortcutText: row.shadowRoot?.querySelector('.shortcut')?.textContent ?? '',
          descriptionText: row.shadowRoot?.querySelector('.description')?.textContent ?? '',
        })),
      };
    });

    expect(reading.menuRole).toBe('menu');
    expect(reading.menuName).toBe('Edit');
    expect(reading.orientation).toBe('vertical');
    expect(reading.separators).toEqual(['separator', 'separator']);
    expect(reading.sections).toEqual([
      { role: 'group', label: 'Paste Options', headingHidden: 'true' },
    ]);

    const by = (label: string): (typeof reading.rows)[number] | undefined =>
      reading.rows.find((row) => row.label === label);

    const cut = by('Cut');
    expect(cut?.role).toBe(menuItemRoles.command);
    expect(cut?.checked, 'a plain command must not announce a checked state it does not have').toBeNull();
    expect(cut?.keys, 'the visible hint is Ctrl+X; ARIA spells it Control+X').toBe('Control+X');
    expect(cut?.shortcutText).toBe('Ctrl+X');

    const special = by('Paste Special');
    expect(special?.disabled, 'an unavailable row announces aria-disabled').toBe('true');
    expect(special?.name).toContain('The clipboard holds nothing');

    const ruler = by('Ruler');
    expect(ruler?.role).toBe(menuItemRoles.checkbox);
    expect(ruler?.checked).toBe('true');
    expect(ruler?.mark, 'a checked checkbox draws a check mark').toBe('checkmark');

    const gridlines = by('Gridlines');
    expect(gridlines?.checked).toBe('false');
    expect(gridlines?.mark, 'an unchecked row draws nothing in the gutter').toBeNull();

    const keep = by('Keep Source Formatting');
    expect(keep?.role).toBe(menuItemRoles.radio);
    expect(keep?.checked).toBe('true');
    // A radio's mark is a bullet, not a check: *this one, of these* rather than *this is on*.
    expect(keep?.mark).toBe('radio-button');
    expect(keep?.descriptionText).toContain('Keeps the fonts');

    const merge = by('Merge Formatting');
    expect(merge?.checked).toBe('false');
    expect(merge?.mark).toBeNull();

    // Every part the model names is present on a fully-featured row: an anatomy that is *listed*
    // and not *rendered* is exactly what one specimen's screenshot hides.
    const parts = await page.evaluate(() => {
      const row = document.querySelector('mjx-menu-item[label="Keep Source Formatting"]');
      return [...(row?.shadowRoot?.querySelectorAll('[part]') ?? [])].map((node) =>
        node.getAttribute('part'),
      );
    });
    for (const part of menuItemParts) {
      if (part === 'arrow' || part === 'shortcut') continue;
      expect(parts, `the row does not expose a '${part}' part`).toContain(part);
    }

    const more = by('More Options');
    expect(more?.haspopup).toBe('menu');
    expect(more?.expanded, 'a closed submenu announces itself collapsed').toBe('false');
    expect(more?.arrow).toBe('chevron-right');
  });

  test('checking a box and choosing a radio move aria-checked, in both directions', async ({
    page,
  }) => {
    await open(page, anatomyStory);
    const state = async (): Promise<Record<string, string | null>> =>
      page.evaluate(() =>
        Object.fromEntries(
          [...document.querySelectorAll('mjx-menu-item')].map((row) => [
            row.getAttribute('label') ?? '',
            row.getAttribute('aria-checked'),
          ]),
        ),
      );

    expect((await state())['Gridlines']).toBe('false');
    await page.locator('mjx-menu-item[label="Gridlines"]').click();
    await settle(page);
    expect((await state())['Gridlines'], 'a checkbox that never turns on has never been rendered on').toBe('true');
    await page.locator('mjx-menu-item[label="Gridlines"]').click();
    await settle(page);
    expect((await state())['Gridlines']).toBe('false');

    // A radio group is exactly one member, and choosing another clears the first.
    expect((await state())['Keep Source Formatting']).toBe('true');
    await page.locator('mjx-menu-item[label="Merge Formatting"]').click();
    await settle(page);
    const after = await state();
    expect(after['Merge Formatting']).toBe('true');
    expect(after['Keep Source Formatting']).toBe('false');
    expect(after['Keep Text Only']).toBe('false');
    // …and the checkbox outside the section is untouched by the radio group.
    expect(after['Ruler']).toBe('true');
  });

  test('the mark gutter is reserved once for the whole menu, so the labels line up', async ({
    page,
  }) => {
    await open(page, anatomyStory);
    const reading = await page.evaluate(() => {
      // ⚠ This menu's **own** rows. A `mjx-menu-item` descendant selector also matches the rows of
      // the closed submenu, which are inside `display: none` and measure zero — an offset that
      // would fail this assertion for a reason that has nothing to do with the gutter.
      const menu = document.querySelector('mjx-menu[label="Edit"]');
      const rows = [
        ...(menu?.querySelectorAll(':scope > mjx-menu-item, :scope > mjx-menu-section > mjx-menu-item') ??
          []),
      ];
      const offsets = rows.map((row) => {
        const lines = row.shadowRoot?.querySelector('.lines');
        return lines instanceof HTMLElement ? Math.round(lines.getBoundingClientRect().left) : -1;
      });
      const box = document.querySelector('mjx-menu')?.shadowRoot?.querySelector('.menu');
      return {
        offsets,
        markColumn:
          box instanceof HTMLElement
            ? getComputedStyle(box).getPropertyValue('--mjx-menu-mark-column').trim()
            : '',
      };
    });
    expect(reading.offsets.length).toBeGreaterThan(5);
    expect(
      new Set(reading.offsets).size,
      'the labels of this menu start at more than one x. The gutter is the menu’s decision ' +
        'precisely so that a menu whose third row is the only checkable one still lines up.',
    ).toBe(1);
    expect(reading.markColumn, 'a menu with checkable rows reserves the mark gutter').not.toBe(
      '0px',
    );
  });
});

// ── the keyboard ─────────────────────────────────────────────────────────────

test.describe('the full keyboard model', () => {
  test('arrows move and wrap, Home and End jump, and type-ahead selects', async ({ page }) => {
    await open(page, travelStory);
    await page.evaluate(() => {
      const menu = document.querySelector('#travel-menu') as unknown as {
        focusItem(index: number): void;
      };
      menu.focusItem(0);
    });
    expect((await focused(page)).label).toBe('Paste');

    await page.keyboard.press('ArrowDown');
    expect((await focused(page)).label).toBe('Paste Options');
    await page.keyboard.press('ArrowUp');
    expect((await focused(page)).label).toBe('Paste');
    // Wrapping at both ends: a menu is a ring, not a list with a floor.
    await page.keyboard.press('ArrowUp');
    expect((await focused(page)).label).toBe('Paste As Hyperlink');
    await page.keyboard.press('ArrowDown');
    expect((await focused(page)).label).toBe('Paste');

    await page.keyboard.press('End');
    expect((await focused(page)).label).toBe('Paste As Hyperlink');
    await page.keyboard.press('Home');
    expect((await focused(page)).label).toBe('Paste');

    // Type-ahead: `Paste As…` is the only row starting with `paste a`.
    await page.keyboard.type('paste a');
    expect(
      (await focused(page)).label,
      'type-ahead did not select by prefix — the single most-used way of reaching a long menu',
    ).toBe('Paste As Hyperlink');
  });

  test('only one row is a tab stop, and Tab leaves the menu rather than cycling in it', async ({
    page,
  }) => {
    await open(page, travelStory);
    const stops = await page.evaluate(() =>
      [...document.querySelectorAll('#travel-menu > mjx-menu-item')].map((row) =>
        row instanceof HTMLElement ? row.tabIndex : null,
      ),
    );
    expect(
      stops.filter((stop) => stop === 0).length,
      'a menu holds exactly one roving tab stop; that is what lets Tab mean *leave*',
    ).toBe(1);
    expect(stops.filter((stop) => stop === -1).length).toBe(stops.length - 1);
    // The rule, and the model that records it, agree.
    expect(menuFocusPattern).toBe('roving');
    expect(focusManagementPatterns[menuFocusPattern].tabStops).toBe('one');

    await page.evaluate(() => {
      const menu = document.querySelector('#travel-menu') as unknown as {
        focusItem(index: number): void;
      };
      menu.focusItem(1);
    });
    await page.keyboard.press('Tab');
    await settle(page);
    const after = await focused(page);
    expect(
      after.role,
      'Tab left the keyboard inside the menu. A menu is a roving composite and not a focus trap: ' +
        'the ARIA menu pattern is explicit that Tab moves focus out of it.',
    ).not.toBe('menuitem');
  });

  test('an unavailable row is reachable by arrow key and refuses to be activated', async ({
    page,
  }) => {
    await open(page, anatomyStory);
    await page.evaluate(() => {
      const menu = document.querySelector('mjx-menu') as unknown as {
        focusItem(index: number): void;
      };
      menu.focusItem(1);
    });
    await page.keyboard.press('ArrowDown');
    const landed = await focused(page);
    expect(
      landed.label,
      'the arrow key skipped the unavailable row. That is a toolbar’s behaviour, not a menu’s: a ' +
        'command a person cannot reach is a command whose reason they can never read.',
    ).toBe('Paste Special');
    expect(landed.disabled).toBe('true');

    let activations = 0;
    await page.exposeFunction('mjxCountActivation', () => {
      activations += 1;
    });
    await page.evaluate(() => {
      document.addEventListener('mjx-menu-activate', () => {
        (globalThis as unknown as { mjxCountActivation(): void }).mjxCountActivation();
      });
    });
    await page.keyboard.press('Enter');
    await settle(page);
    await page.evaluate(() => {
      const row = document.querySelector('mjx-menu-item[label="Paste Special"]');
      const box = row?.shadowRoot?.querySelector('.item');
      if (box instanceof HTMLElement) box.click();
    });
    await settle(page);
    expect(
      activations,
      'an unavailable row was activated. It is refused in JavaScript rather than by the platform, ' +
        'which is the price of staying reachable — and therefore the thing that has to be tested.',
    ).toBe(0);
  });

  test('Arrow Right opens a submenu immediately — the keyboard never waits for hover intent', async ({
    page,
  }) => {
    await open(page, travelStory);
    const elapsed = await page.evaluate(async () => {
      const menu = document.querySelector('#travel-menu') as unknown as {
        items: HTMLElement[];
        focusItem(index: number): void;
      };
      menu.focusItem(1);
      const started = performance.now();
      const parent = menu.items[1] as unknown as { submenu?: Element } & HTMLElement;
      parent.dispatchEvent(
        new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, composed: true }),
      );
      const opened = parent.submenu?.hasAttribute('open') ?? false;
      return { elapsed: performance.now() - started, opened };
    });
    expect(
      elapsed.opened,
      'Arrow Right did not open the submenu in the same task. A keyboard path that inherited the ' +
        'pointer’s hover delay is the classic way this component goes wrong.',
    ).toBe(true);
    expect(elapsed.elapsed).toBeLessThan(submenuHoverOpenDelay);

    await settle(page);
    expect((await focused(page)).label, 'Arrow Right did not move into the submenu').toBe(
      'Keep Source Formatting',
    );
    expect((await focused(page)).menuLabel).toBe('Paste Options');
  });

  test('Escape closes exactly one level and gives focus back to the row that opened it', async ({
    page,
  }) => {
    await open(page, travelStory);
    await page.evaluate(() => {
      const menu = document.querySelector('#travel-menu') as unknown as {
        focusItem(index: number): void;
      };
      menu.focusItem(1);
    });
    await page.keyboard.press('ArrowRight');
    await settle(page);
    // Down to the row that owns the second level, and into it.
    await page.keyboard.press('End');
    await page.keyboard.press('ArrowRight');
    await settle(page);
    expect((await focused(page)).menuLabel).toBe('Set Default Paste');

    await page.keyboard.press('Escape');
    await settle(page);
    expect(
      (await focused(page)).label,
      'Escape closed more than one level, or dropped focus entirely',
    ).toBe('Set Default Paste');
    expect(
      await page.evaluate(
        () =>
          document
            .querySelector('mjx-menu-item[label="Set Default Paste"] mjx-menu')
            ?.hasAttribute('open') ?? true,
      ),
    ).toBe(false);

    await page.keyboard.press('Escape');
    await settle(page);
    expect((await focused(page)).label).toBe('Paste Options');
    expect(
      await page.evaluate(
        () =>
          document.querySelector('mjx-menu-item[label="Paste Options"]')?.getAttribute('aria-expanded') ??
          null,
      ),
      'the parent row still announces its submenu as open after it closed',
    ).toBe('false');
  });

  test('under RTL the arrows mirror: Arrow Left opens and Arrow Right closes', async ({ page }) => {
    await open(page, rtlStory);
    await page.locator('#rtl-invoker').click();
    await settle(page);
    const menuLabel = await page.evaluate(
      () => document.querySelector('#rtl-anchor mjx-menu')?.getAttribute('label') ?? null,
    );
    expect(menuLabel).not.toBeNull();

    await page.keyboard.press('ArrowDown');
    await settlePaint(page);
    const onParent = await focused(page);
    expect(onParent.role).toBe('menuitem');
    expect(onParent.label).toBe('إرسال إلى الخلف');

    await page.keyboard.press('ArrowLeft');
    await settlePaint(page);
    const inside = await focused(page);
    expect(
      inside.menuLabel,
      'Arrow Left did not open the submenu under RTL. A submenu opens toward the *end* of the ' +
        'line, and hard-coding Arrow Right makes the component unusable in half the writing ' +
        'systems Office ships in.',
    ).toBe('إرسال إلى الخلف');

    await page.keyboard.press('ArrowRight');
    await settle(page);
    expect((await focused(page)).label).toBe('إرسال إلى الخلف');
  });
});

// ── focus restoration ────────────────────────────────────────────────────────

test.describe('focus goes back to the invoker', () => {
  test('on Escape, on selection, and on an outside click', async ({ page }) => {
    for (const close of ['escape', 'activate', 'outside'] as const) {
      await open(page, placementStory);
      await page.locator('#placement-invoker').click();
      await settle(page);
      expect((await focused(page)).role, `${close}: the menu did not take focus`).toBe('menuitem');

      if (close === 'escape') await page.keyboard.press('Escape');
      if (close === 'activate') await page.keyboard.press('Enter');
      // ⚠ Well away from the anchor: the invoker is absolutely positioned in the stage's own
      // corner, so a click at (5, 5) lands *on the invoker* and is not an outside click at all.
      if (close === 'outside') {
        await page.locator('#placement-stage').click({ position: { x: 600, y: 12 } });
      }
      await settle(page);

      expect(
        (await focused(page)).id,
        `closing by ${close} did not return focus to the invoker. Focus restoration is the thing ` +
          'most often missed and the thing a keyboard user notices first.',
      ).toBe('placement-invoker');
      expect(
        await page.evaluate(
          () => document.querySelector('#placement-anchor mjx-menu')?.hasAttribute('open') ?? true,
        ),
      ).toBe(false);
    }
  });

  test('a split button’s arrow opens a menu, and gets its focus back', async ({ page }) => {
    await open(page, splitStory);
    await page.locator('mjx-split-button').click({ position: { x: 5, y: 5 } });
    await settle(page);
    // The primary region fires `mjx-activate`, never a menu request, so nothing opened.
    expect(
      await page.evaluate(
        () => document.querySelector('#split-stage mjx-menu')?.hasAttribute('open') ?? false,
      ),
      'the primary half of a split button opened a menu; the whole point of the arrow is that it ' +
        'is not the action',
    ).toBe(false);

    await page.evaluate(() => {
      const button = document.querySelector('mjx-split-button');
      const arrow = button?.shadowRoot?.querySelector('.control[data-region="menu"]');
      if (arrow instanceof HTMLElement) arrow.click();
    });
    await settle(page);
    expect((await focused(page)).role).toBe('menuitem');
    await page.keyboard.press('Escape');
    await settle(page);
    const back = await focused(page);
    expect(back.tag, 'Escape did not return focus into the split button').toBe('button');
  });
});

// ── the pointer ──────────────────────────────────────────────────────────────

interface Boxes {
  parent: { x: number; y: number; width: number; height: number };
  sibling: { x: number; y: number; width: number; height: number };
  submenu: { x: number; y: number; width: number; height: number } | null;
}

async function boxes(page: Page): Promise<Boxes> {
  return page.evaluate(() => {
    const rows = [...document.querySelectorAll('#travel-menu > mjx-menu-item')];
    const parent = rows[1];
    const sibling = rows[2];
    if (parent === undefined || sibling === undefined) throw new Error('the travel story changed');
    const submenu = parent.querySelector('mjx-menu')?.shadowRoot?.querySelector('.menu');
    const rect = (element: Element): { x: number; y: number; width: number; height: number } => {
      const box = element.getBoundingClientRect();
      return { x: box.x, y: box.y, width: box.width, height: box.height };
    };
    return {
      parent: rect(parent),
      sibling: rect(sibling),
      submenu: submenu === null || submenu === undefined ? null : rect(submenu),
    };
  });
}

async function submenuIsOpen(page: Page): Promise<boolean> {
  return page.evaluate(
    () =>
      document.querySelector('mjx-menu-item[label="Paste Options"] mjx-menu')?.hasAttribute('open') ??
      false,
  );
}

test.describe('hover intent and the safe triangle', () => {
  test('a submenu waits for the hover delay rather than opening on contact', async ({ page }) => {
    await open(page, travelStory);
    const rects = await boxes(page);
    await page.mouse.move(rects.parent.x + 20, rects.parent.y + rects.parent.height / 2);
    await settle(page);
    expect(
      await submenuIsOpen(page),
      'the submenu opened the instant the pointer touched the row. A menu whose submenus open on ' +
        'contact opens three of them while a person crosses it.',
    ).toBe(false);
    await page.waitForTimeout(submenuHoverOpenDelay + 150);
    expect(await submenuIsOpen(page)).toBe(true);
  });

  /**
   * **The assertion the whole tolerance exists for.**
   *
   * The pointer leaves *Paste Options* travelling down and to the right toward the submenu, and it
   * passes over *Paste Special* on the way. A menu with no tolerance closes the submenu at that
   * moment, and the person is left aiming at nothing — which is the single most-noticed menu defect
   * there is.
   */
  test('a diagonal across a sibling does not close the submenu', async ({ page }) => {
    await open(page, travelStory);
    const rects = await boxes(page);
    const start = {
      x: rects.parent.x + rects.parent.width - 30,
      y: rects.parent.y + rects.parent.height / 2,
    };
    await page.mouse.move(start.x, start.y);
    await page.waitForTimeout(submenuHoverOpenDelay + 150);
    expect(await submenuIsOpen(page)).toBe(true);

    const opened = await boxes(page);
    const submenu = opened.submenu;
    expect(submenu, 'the submenu has no box to travel toward').not.toBeNull();
    if (submenu === null) return;

    /*
     * ⚠ The path is stepped **and sampled at every step**, and that is what makes this failable.
     *
     * Asserting only the end state proves nothing: the pointer finishes inside the submenu, whose
     * own hover takes the highlight, so a menu with no tolerance at all reaches the same final
     * reading by a completely different route. What separates them is the *middle* of the path —
     * the three or four samples taken while the pointer is over the crossed sibling — and this
     * suite watched the whole assertion pass with `travellingToward` hard-wired to `false` before
     * it was written this way.
     */
    const target = { x: submenu.x + 30, y: submenu.y + submenu.height / 2 };
    const steps = 12;
    const trace: { x: number; label: string | null; open: boolean }[] = [];
    for (let step = 1; step <= steps; step += 1) {
      const x = start.x + ((target.x - start.x) * step) / steps;
      const y = start.y + ((target.y - start.y) * step) / steps;
      await page.mouse.move(x, y);
      const active = await focused(page);
      trace.push({ x: Math.round(x), label: active.label, open: await submenuIsOpen(page) });
    }

    const crossing = trace.filter((sample) => sample.x < submenu.x);
    expect(
      crossing.length,
      'the path never crossed the sibling, so nothing was being tested',
    ).toBeGreaterThan(2);
    for (const sample of crossing) {
      expect(
        sample.label,
        `at x=${String(sample.x)} the crossed sibling took the highlight. A diagonal toward a ` +
          'submenu is the classic menu defect, and it is exactly what the safe triangle in ' +
          'src/overlay/floating.ts exists to prevent.',
      ).not.toBe('Paste Special');
      expect(sample.open, `at x=${String(sample.x)} the submenu had already closed`).toBe(true);
    }

    // …and the pointer arrived where it was aiming: inside the submenu, on one of its rows.
    expect((await focused(page)).menuLabel).toBe('Paste Options');
    await page.waitForTimeout(submenuHoverCloseDelay + 200);
    expect(
      await submenuIsOpen(page),
      'the submenu closed after the pointer had reached it — the hover close timer armed on the ' +
        'way past was never cancelled',
    ).toBe(true);
  });

  /**
   * **The control experiment, and the thing that makes the assertion above mean anything.**
   *
   * A component that simply never closed a submenu would pass the test above. Two paths are
   * therefore driven that must *not* be read as travel: straight down onto the sibling, and the
   * same diagonal with a pause in the middle long enough for the grace to expire.
   */
  test('a move straight down onto the sibling does close it, and so does pausing half way', async ({
    page,
  }) => {
    await open(page, travelStory);
    const rects = await boxes(page);
    await page.mouse.move(rects.parent.x + 30, rects.parent.y + rects.parent.height / 2);
    await page.waitForTimeout(submenuHoverOpenDelay + 150);
    expect(await submenuIsOpen(page)).toBe(true);

    await page.mouse.move(rects.parent.x + 30, rects.sibling.y + rects.sibling.height / 2, {
      steps: 6,
    });
    await page.waitForTimeout(submenuHoverCloseDelay + 200);
    expect(
      await submenuIsOpen(page),
      'moving straight down onto a sibling left the submenu open. If this passes while the ' +
        'diagonal test also passes, nothing is being measured: the component never closes a ' +
        'submenu at all.',
    ).toBe(false);

    // …and the sibling did take the highlight.
    expect((await focused(page)).label).toBe('Paste Special');
  });

  test('hovering a row moves the keyboard to it, so the pointer and the keyboard agree', async ({
    page,
  }) => {
    await open(page, travelStory);
    const rects = await boxes(page);
    await page.mouse.move(rects.sibling.x + 30, rects.sibling.y + rects.sibling.height / 2);
    await settle(page);
    expect((await focused(page)).label).toBe('Paste Special');
  });
});

// ── placement ────────────────────────────────────────────────────────────────

interface PlacementReading {
  placement: Placement | undefined;
  anchor: Rect | undefined;
  boundary: Rect | undefined;
  natural: { width: number; height: number } | undefined;
  side: LogicalSide;
  align: Align;
  direction: Direction;
  gap: number;
  rect: { x: number; y: number; width: number; height: number };
  frame: { x: number; y: number; width: number; height: number } | null;
  viewport: { width: number; height: number };
}

async function readPlacement(page: Page, selector: string): Promise<PlacementReading> {
  return page.evaluate((menuSelector) => {
    const menu = document.querySelector(menuSelector) as unknown as Record<string, unknown> & Element;
    const box = menu.shadowRoot?.querySelector('.menu');
    if (!(box instanceof HTMLElement)) throw new Error(`no menu box for ${menuSelector}`);
    const rect = box.getBoundingClientRect();
    const frame = document
      .querySelector('mjx-resizable-container')
      ?.shadowRoot?.querySelector('.frame');
    const frameRect = frame?.getBoundingClientRect();
    return {
      placement: menu['placement'],
      anchor: menu['anchorRect'],
      boundary: menu['boundaryRect'],
      natural: menu['naturalSize'],
      side: menu['side'],
      align: menu['align'],
      direction: menu['direction'],
      gap: Number.parseFloat(getComputedStyle(box).getPropertyValue('--mjx-floating-gap')) || 0,
      rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
      frame:
        frameRect === undefined
          ? null
          : { x: frameRect.x, y: frameRect.y, width: frameRect.width, height: frameRect.height },
      viewport: { width: window.innerWidth, height: window.innerHeight },
    } as never;
  }, selector);
}

/** Put the anchor in one corner of the stage and open the menu there. */
async function openInCorner(
  page: Page,
  corner: 'start-start' | 'start-end' | 'end-start' | 'end-end',
  ids: { anchor: string; invoker: string },
): Promise<void> {
  await page.evaluate(
    ([where, anchorId]) => {
      const anchor = document.querySelector(`#${anchorId}`);
      if (!(anchor instanceof HTMLElement)) throw new Error('no anchor');
      const [block, inline] = where.split('-');
      anchor.style.insetBlockStart = block === 'start' ? '0' : 'auto';
      anchor.style.insetBlockEnd = block === 'end' ? '0' : 'auto';
      anchor.style.insetInlineStart = inline === 'start' ? '0' : 'auto';
      anchor.style.insetInlineEnd = inline === 'end' ? '0' : 'auto';
    },
    [corner, ids.anchor] as const,
  );
  await settle(page);
  await page.locator(`#${ids.invoker}`).click();
  await settlePaint(page);
}

test.describe('placement, flipping and the boundary that clips', () => {
  const corners = ['start-start', 'start-end', 'end-start', 'end-end'] as const;

  for (const corner of corners) {
    test(`at the ${corner} corner the component and placeFloating agree`, async ({ page }) => {
      await open(page, placementStory);
      await openInCorner(page, corner, {
        anchor: 'placement-anchor',
        invoker: 'placement-invoker',
      });
      const reading = await readPlacement(page, '#placement-anchor mjx-menu');
      expect(reading.placement, 'the menu recorded no placement').toBeDefined();
      expect(reading.anchor).toBeDefined();
      expect(reading.boundary).toBeDefined();
      expect(reading.natural).toBeDefined();
      if (
        reading.placement === undefined ||
        reading.anchor === undefined ||
        reading.boundary === undefined ||
        reading.natural === undefined
      ) {
        return;
      }

      // **The correspondence assertion.** Not "the menu moved" — the same arithmetic, re-run in
      // Node over the component's own inputs, must produce the component's own answer.
      const model = placeFloating({
        anchor: reading.anchor,
        floating: reading.natural,
        boundary: reading.boundary,
        side: reading.side,
        align: reading.align,
        direction: reading.direction,
        gap: reading.gap,
      });
      expect(reading.placement, `${corner}: the component did not place where the model says`).toEqual(
        model,
      );

      // …and the box is actually where the placement said, and inside what clips it.
      expect(reading.rect.x).toBeCloseTo(reading.placement.x, 0);
      expect(reading.rect.y).toBeCloseTo(reading.placement.y, 0);
      const boundary = reading.boundary;
      expect(reading.rect.x, `${corner}: left of the boundary`).toBeGreaterThanOrEqual(
        boundary.x - 1,
      );
      expect(reading.rect.y, `${corner}: above the boundary`).toBeGreaterThanOrEqual(boundary.y - 1);
      expect(
        reading.rect.x + reading.rect.width,
        `${corner}: past the boundary's inline end — MJXOFF-183 left exactly this unwatched`,
      ).toBeLessThanOrEqual(boundary.x + boundary.width + 1);
      expect(
        reading.rect.y + reading.rect.height,
        `${corner}: past the boundary's block end`,
      ).toBeLessThanOrEqual(boundary.y + boundary.height + 1);
    });
  }

  test('a menu at the block end flips above its anchor rather than hanging out of the frame', async ({
    page,
  }) => {
    await open(page, placementStory);
    await openInCorner(page, 'start-start', {
      anchor: 'placement-anchor',
      invoker: 'placement-invoker',
    });
    const top = await readPlacement(page, '#placement-anchor mjx-menu');
    expect(top.placement?.side, 'there is room below at the top of the stage').toBe('bottom');
    expect(top.placement?.flipped).toBe(false);

    await page.keyboard.press('Escape');
    await openInCorner(page, 'end-start', {
      anchor: 'placement-anchor',
      invoker: 'placement-invoker',
    });
    const bottom = await readPlacement(page, '#placement-anchor mjx-menu');
    expect(
      bottom.placement?.side,
      'the menu stayed below its anchor at the bottom of the frame, which is where it would be ' +
        'clipped — the frame is a container query container and therefore a containing block, so ' +
        'a fixed box inside it is clipped by its overflow',
    ).toBe('top');
    expect(bottom.placement?.flipped).toBe(true);
  });

  test('the boundary is the harness frame, not the viewport', async ({ page }) => {
    await open(page, placementStory);
    await openInCorner(page, 'start-start', {
      anchor: 'placement-anchor',
      invoker: 'placement-invoker',
    });
    const reading = await readPlacement(page, '#placement-anchor mjx-menu');
    expect(reading.frame).not.toBeNull();
    expect(reading.boundary).toBeDefined();
    if (reading.frame === null || reading.boundary === undefined) return;
    // The frame is 1440 wide inside a 1280 viewport, so the boundary is the *overlap* — and it is
    // strictly smaller than the frame in both axes because of the inset.
    expect(reading.boundary.y).toBeGreaterThanOrEqual(reading.frame.y);
    expect(reading.boundary.y + reading.boundary.height).toBeLessThanOrEqual(
      reading.frame.y + reading.frame.height,
    );
    expect(reading.boundary.width).toBeLessThan(reading.viewport.width);
  });

  test('a submenu under RTL opens toward the end of the line, which is leftward', async ({
    page,
  }) => {
    await open(page, rtlStory);
    await page.locator('#rtl-invoker').click();
    await settle(page);
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowLeft');
    await settlePaint(page);
    const reading = await readPlacement(page, '#rtl-anchor mjx-menu-item mjx-menu');
    expect(reading.direction).toBe('rtl');
    expect(reading.side, 'the submenu declares the logical inline end').toBe('inlineEnd');
    expect(
      reading.placement?.side,
      'inlineEnd resolved to `right` under RTL. One function decides this — physicalSide() — and ' +
        'a component that hard-coded it would need a second code path for half the world.',
    ).toBe('left');
  });
});

// ── the sheet ────────────────────────────────────────────────────────────────

test.describe('the touch presentation is a sheet', () => {
  test('on a phone-width container the menu is pinned to the bottom at full width', async ({
    page,
  }) => {
    await open(page, phoneStory);
    await page.locator('#phone-canvas').click({ button: 'right' });
    await settlePaint(page);

    const reading = await page.evaluate(() => {
      const menu = document.querySelector('#phone-context mjx-menu');
      const box = menu?.shadowRoot?.querySelector('.menu');
      if (!(box instanceof HTMLElement)) throw new Error('no menu box');
      const rect = box.getBoundingClientRect();
      // The *inner* container is the one the story pins to 390; the outer one is the harness.
      const frames = [...document.querySelectorAll('mjx-resizable-container')].map((container) => {
        const frame = container.shadowRoot?.querySelector('.frame');
        const frameRect = frame?.getBoundingClientRect();
        return frameRect === undefined
          ? null
          : { x: frameRect.x, y: frameRect.y, width: frameRect.width, height: frameRect.height };
      });
      return {
        presentation: getComputedStyle(box).getPropertyValue('--mjx-menu-presentation').trim(),
        position: getComputedStyle(box).position,
        rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
        frames: frames.filter((frame) => frame !== null),
        placement: (menu as unknown as Record<string, unknown>)['placement'],
        motion: box.className,
      };
    });

    const inner = reading.frames[reading.frames.length - 1];
    expect(inner, 'the phone story has no pinned container').toBeDefined();
    if (inner === undefined) return;

    // Correspondence: the model says what the container width should produce, and the cascade did.
    expect(menuPresentationAt(Math.round(inner.width), true)).toBe('sheet');
    expect(
      reading.presentation,
      `a ${String(Math.round(inner.width))}px container produced '${reading.presentation}'. Below ` +
        `${String(menuSheetAtOrBelow)}px a floating list must become a sheet: aiming a thumb at a ` +
        'list beside a pointer is a list nobody hits.',
    ).toBe('sheet');
    expect(reading.position).toBe('fixed');
    // A sheet is *pinned* rather than anchored: it sits on one edge of the boundary at that
    // boundary's full width, and it is the boundary — the frame — rather than the window.
    expect(menuPresentations.sheet.anchored).toBe(false);
    expect((reading.placement as { side?: string } | undefined)?.side).toBe('bottom');
    expect(reading.motion).toContain('mjx-motion-sheet-enter');

    expect(reading.rect.width).toBeCloseTo(inner.width, 0);
    expect(reading.rect.x).toBeCloseTo(inner.x, 0);
    expect(
      reading.rect.y + reading.rect.height,
      'the sheet is not sitting on the bottom edge of what clips it',
    ).toBeCloseTo(inner.y + inner.height, 0);
  });

  test('a long press opens it, and a drag does not', async ({ page }) => {
    await open(page, phoneStory);

    const press = async (drift: number): Promise<boolean> =>
      page.evaluate(async (moveBy) => {
        const canvas = document.querySelector('#phone-canvas');
        if (!(canvas instanceof HTMLElement)) throw new Error('no canvas');
        const box = canvas.getBoundingClientRect();
        const at = { clientX: box.x + box.width / 2, clientY: box.y + box.height / 2 };
        const options = { bubbles: true, composed: true, pointerType: 'touch', pointerId: 1 };
        canvas.dispatchEvent(new PointerEvent('pointerdown', { ...options, ...at }));
        if (moveBy !== 0) {
          canvas.dispatchEvent(
            new PointerEvent('pointermove', {
              ...options,
              clientX: at.clientX + moveBy,
              clientY: at.clientY,
            }),
          );
        }
        await new Promise((done) => setTimeout(done, 700));
        const menu = document.querySelector('#phone-context mjx-menu');
        const opened = menu?.hasAttribute('open') ?? false;
        canvas.dispatchEvent(new PointerEvent('pointerup', { ...options, ...at }));
        return opened;
      }, drift);

    // A synthetic PointerEvent goes through the same listener as a real one; only `isTrusted`
    // differs, and Playwright cannot drive a touch long-press any other way.
    expect(await press(0), 'a long press did not open the context menu').toBe(true);
    await page.keyboard.press('Escape');
    await settle(page);
    expect(
      await press(60),
      'a drag opened the context menu. The same gesture starts a scroll, and the only thing that ' +
        'tells them apart is how far the finger moved.',
    ).toBe(false);
  });
});

// ── the context menu ─────────────────────────────────────────────────────────

test.describe('the context menu opens the three ways it has to', () => {
  test('right-click puts it at the pointer', async ({ page }) => {
    await open(page, canvasStory);
    const canvas = await page.locator('#context-canvas').boundingBox();
    expect(canvas).not.toBeNull();
    if (canvas === null) return;
    const at = { x: Math.round(canvas.x + 40), y: Math.round(canvas.y + 40) };
    await page.mouse.click(at.x, at.y, { button: 'right' });
    await settlePaint(page);

    const reading = await readPlacement(page, '#canvas-context mjx-menu');
    expect(reading.anchor, 'a right-click anchors on a zero-sized rectangle at the pointer').toEqual({
      x: at.x,
      y: at.y,
      width: 0,
      height: 0,
    });
    expect(reading.rect.x).toBeGreaterThanOrEqual(at.x - 1);
    expect((await focused(page)).role, 'the menu did not take focus').toBe('menuitem');
  });

  test('the Context Menu key puts it at the focused element, and gives focus back', async ({
    page,
  }) => {
    await open(page, canvasStory);
    await page.locator('#context-canvas').focus();
    await page.keyboard.press('ContextMenu');
    await settlePaint(page);

    const opened = await page.evaluate(
      () => document.querySelector('#canvas-context mjx-menu')?.hasAttribute('open') ?? false,
    );
    expect(opened, 'the Context Menu key did not open the menu').toBe(true);
    const reading = await readPlacement(page, '#canvas-context mjx-menu');
    const canvas = await page.locator('#context-canvas').boundingBox();
    expect(canvas).not.toBeNull();
    if (canvas === null) return;
    expect(
      reading.anchor?.width,
      'the keyboard opened the menu at a point rather than at the focused element. A keyboard ' +
        'user’s context is their focus, not wherever the pointer was left.',
    ).toBeCloseTo(canvas.width, 0);
    expect((await focused(page)).role).toBe('menuitem');

    await page.keyboard.press('Escape');
    await settle(page);
    expect((await focused(page)).id).toBe('context-canvas');
  });

  test('Shift + F10 opens it too, for the keyboards that have no Context Menu key', async ({
    page,
  }) => {
    await open(page, canvasStory);
    await page.locator('#context-canvas').focus();
    await page.keyboard.press('Shift+F10');
    await settle(page);
    expect(
      await page.evaluate(
        () => document.querySelector('#canvas-context mjx-menu')?.hasAttribute('open') ?? false,
      ),
    ).toBe(true);
  });
});

// ── density ──────────────────────────────────────────────────────────────────

test.describe('density', () => {
  test('a compact menu keeps every row above the accessible hit-target floor', async ({ page }) => {
    await open(page, compactStory);
    const rows = await page.evaluate(() => {
      return [...document.querySelectorAll('mjx-menu-item')].map((item) => {
        const row = item.shadowRoot?.querySelector('.item');
        const style = row instanceof HTMLElement ? getComputedStyle(row) : null;
        const density =
          row instanceof HTMLElement
            ? getComputedStyle(row).getPropertyValue('--mjx-density-step').trim()
            : '';
        return {
          label: item.getAttribute('label') ?? '',
          minBlock: style === null ? 0 : Number.parseFloat(style.minBlockSize) || 0,
          height: row instanceof HTMLElement ? row.getBoundingClientRect().height : 0,
          density,
        };
      });
    });
    expect(rows.length).toBeGreaterThan(3);
    for (const row of rows) {
      expect(
        row.minBlock,
        `'${row.label}' is ${String(row.minBlock)}px tall at density step ${row.density}, and ` +
          'WCAG 2.2 Target Size (Minimum) is the floor no density mode may go under.',
      ).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
      expect(row.height).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
    }
    // …and the two halves of the story really are at two densities, or the assertion above is
    // being made twice about the same thing.
    expect(new Set(rows.map((row) => row.density)).size).toBe(2);
  });
});
