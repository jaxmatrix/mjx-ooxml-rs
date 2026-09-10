import { expect, test, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { accessibleHitTargetMinimum } from '../../src/foundations/density.ts';
import { alignedTextProperties } from '../../src/formula/formula-sheets.ts';
import {
  formulaModes,
  formulaRowBounds,
  formulaStoryTitles,
  referenceColourSlots,
  referenceSlotProperty,
} from '../../src/formula/formula-model.ts';

/**
 * MJXOFF-192's Excel chrome, driven in a real browser.
 *
 * The unit tier owns the caret-position table, because the answer is a pure function and a table of
 * fourteen offsets belongs where it can be read. **What only this tier can prove** is that the
 * component actually asks that function, at the caret the *browser* reports, after real keystrokes —
 * and four further things that have no meaning without a rendering engine:
 *
 * * **the two text layers lay text out identically**, compared property by property through
 *   `getComputedStyle` from the one list both the stylesheet and this file read. A drift in any of
 *   them is a caret sitting beside its own glyph, which is the classic defect of this technique and
 *   is invisible until a line wraps;
 * * **the reference ring resolves to four different colours**, in the scheme the story is in,
 *   through the scheme-keyed properties on `:root` — because a ring whose four slots resolved to
 *   one colour would satisfy every contrast assertion perfectly and colour nothing;
 * * **the autocomplete is operable by the keyboard alone**, filtered, completed and dismissed
 *   without a pointer ever being used;
 * * **the hit-target floor**, which is a promise about pixels.
 */

const tooltipStory = { title: formulaStoryTitles.bar, name: 'The Argument Tooltip Tracks The Caret' } as const;
const colourStory = { title: formulaStoryTitles.bar, name: 'Coloured References, And The Contract Under Them' } as const;
const completeStory = { title: formulaStoryTitles.bar, name: 'The Function Autocomplete' } as const;
const bracketStory = { title: formulaStoryTitles.bar, name: 'Bracket Matching As The Caret Moves' } as const;
const modeStory = { title: formulaStoryTitles.bar, name: 'The Four Modes, Announced' } as const;
const rowsStory = { title: formulaStoryTitles.bar, name: 'Expanding To Multiple Lines' } as const;
const denseBarStory = { title: formulaStoryTitles.bar, name: 'In Compact Density' } as const;
const navStory = { title: formulaStoryTitles.nameBox, name: 'Typing An Address Navigates' } as const;
const namesStory = { title: formulaStoryTitles.nameBox, name: 'The List Of Names And Tables' } as const;

async function open(
  page: Page,
  story: { readonly title: string; readonly name: string },
  options: { containerPreset?: string; theme?: string } = {},
): Promise<void> {
  const entry = builtStories().find(
    (candidate) => candidate.title === story.title && candidate.name === story.name,
  );
  expect(entry, `${story.title} · ${story.name} is missing from the catalogue`).toBeDefined();
  if (entry === undefined) return;
  await openStory(page, entry.id, options);
  for (const tag of ['mjx-formula-bar', 'mjx-name-box']) {
    await page.waitForFunction((name) => customElements.get(name) !== undefined, tag);
  }
  await settle(page);
}

async function settle(page: Page): Promise<void> {
  await page.evaluate(async () => {
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
}

/** Put the caret at an offset and let the component see it, the way a person's click would. */
async function caretTo(page: Page, barId: string, offset: number): Promise<void> {
  await page.evaluate(
    ({ id, at }) => {
      const bar = document.getElementById(id);
      const entry = bar?.shadowRoot?.querySelector('.entry');
      if (!(entry instanceof HTMLTextAreaElement)) return;
      entry.focus();
      entry.setSelectionRange(at, at);
      entry.dispatchEvent(new Event('select', { bubbles: true }));
    },
    { id: barId, at: offset },
  );
  await settle(page);
}

async function tooltipText(page: Page, barId: string): Promise<{ text: string; emphasis: string }> {
  return page.evaluate((id) => {
    const tip = document.getElementById(id)?.shadowRoot?.querySelector('.tooltip');
    if (!(tip instanceof HTMLElement) || tip.hidden) return { text: '', emphasis: '' };
    const emphasis = tip.querySelector('.emphasis');
    return { text: tip.textContent ?? '', emphasis: emphasis?.textContent ?? '' };
  }, barId);
}

// ── the caret, as the BROWSER reports it ─────────────────────────────────────

test.describe('the argument tooltip, driven by a real caret', () => {
  test('emphasises the argument the caret is in, and follows it across a nested call', async ({
    page,
  }) => {
    await open(page, tooltipStory);
    const formula = await page.locator('#tooltip-bar').evaluate((element) => (element as HTMLElement & { value: string }).value);
    expect(formula).toContain('COUNTIF(');

    await caretTo(page, 'tooltip-bar', formula.indexOf('IF(') + 3);
    expect((await tooltipText(page, 'tooltip-bar')).emphasis).toBe('logical_test');

    const insideCountif = formula.indexOf('A1:A9,') + 'A1:A9,'.length;
    await caretTo(page, 'tooltip-bar', insideCountif);
    const nested = await tooltipText(page, 'tooltip-bar');
    expect(nested.text).toContain('COUNTIF(');
    expect(nested.emphasis).toBe('criteria');
  });

  test('KEEPS the argument across a comma inside a quoted string — the child’s whole trap', async ({
    page,
  }) => {
    await open(page, tooltipStory);
    const formula = await page.locator('#tooltip-bar').evaluate((element) => (element as HTMLElement & { value: string }).value);

    // Immediately after the comma INSIDE "#,##0". A comma count reports argument 3 of TEXT, which
    // does not exist; the tokeniser reports argument 2, which is the one being typed.
    const insideFormat = formula.indexOf('"#,') + 3;
    await caretTo(page, 'tooltip-bar', insideFormat);
    const inside = await tooltipText(page, 'tooltip-bar');
    expect(inside.text).toContain('TEXT(');
    expect(inside.emphasis).toBe('format_text');

    // And the component's own answer, read off the property the story's readout reads.
    const reported = await page.locator('#tooltip-bar').evaluate((element) => {
      const argument = (element as HTMLElement & {
        argument?: { functionName: string; argumentIndex: number };
      }).argument;
      return argument === undefined ? undefined : `${argument.functionName}#${String(argument.argumentIndex)}`;
    });
    expect(reported).toBe('TEXT#1');
  });

  test('reports the tooltip as the editor’s description, so it is announced', async ({ page }) => {
    await open(page, tooltipStory);
    const linked = await page.locator('#tooltip-bar').evaluate((element) => {
      const root = element.shadowRoot;
      const entry = root?.querySelector('.entry');
      const described = entry?.getAttribute('aria-describedby') ?? '';
      return root?.getElementById(described)?.getAttribute('role') ?? '';
    });
    expect(linked).toBe('tooltip');
  });

  test('shows no tooltip outside a call, so it is not permanently on screen', async ({ page }) => {
    await open(page, tooltipStory);
    await caretTo(page, 'tooltip-bar', 0);
    expect((await tooltipText(page, 'tooltip-bar')).text).toBe('');
  });
});

// ── the two layers, which is what makes the caret land on its own glyph ──────

test.describe('the transparent editor over the coloured layer', () => {
  test('lays text out identically in both, property by property', async ({ page }) => {
    await open(page, colourStory);
    const compared = await page.locator('#colour-bar').evaluate((element, properties) => {
      const root = element.shadowRoot;
      const entry = root?.querySelector('.entry');
      const backdrop = root?.querySelector('.backdrop');
      if (!(entry instanceof HTMLElement) || !(backdrop instanceof HTMLElement)) return null;
      const one = getComputedStyle(entry);
      const two = getComputedStyle(backdrop);
      return properties.map((property) => ({
        property,
        entry: one.getPropertyValue(property),
        backdrop: two.getPropertyValue(property),
      }));
    }, [...alignedTextProperties]);
    expect(compared).not.toBeNull();
    for (const row of compared ?? []) {
      expect(row.entry, `${row.property} differs between the two layers`).toBe(row.backdrop);
      // A property that resolved to nothing in BOTH would compare equal and mean nothing.
      expect(row.entry, `${row.property} resolved to nothing`).not.toBe('');
    }
  });

  test('draws the editor’s own glyphs transparent and its caret visible', async ({ page }) => {
    await open(page, colourStory);
    const painted = await page.locator('#colour-bar').evaluate((element) => {
      const entry = element.shadowRoot?.querySelector('.entry');
      if (!(entry instanceof HTMLElement)) return null;
      const style = getComputedStyle(entry);
      return { color: style.color, caret: style.caretColor };
    });
    expect(painted?.color).toBe('rgba(0, 0, 0, 0)');
    expect(painted?.caret).not.toBe('rgba(0, 0, 0, 0)');
  });
});

// ── the reference contract, as pixels and as an event ────────────────────────

test.describe('coloured references', () => {
  test('draws four DIFFERENT colours, resolved through the scheme-keyed properties', async ({
    page,
  }) => {
    await open(page, colourStory);
    const resolved = await page.evaluate((properties) => {
      const style = getComputedStyle(document.documentElement);
      return properties.map((property) => style.getPropertyValue(property).trim());
    }, referenceColourSlots.map((_, index) => referenceSlotProperty(index)));
    expect(resolved.every((value) => value !== '')).toBe(true);
    expect(new Set(resolved).size).toBe(referenceColourSlots.length);
  });

  test('paints each reference in its slot’s colour, and the repeat in the SAME one', async ({
    page,
  }) => {
    await open(page, colourStory);
    const painted = await page.locator('#colour-bar').evaluate((element) => {
      const spans = element.shadowRoot?.querySelectorAll('.token-reference') ?? [];
      return [...spans].map((span) => ({
        text: span.textContent ?? '',
        colour: getComputedStyle(span).color,
      }));
    });
    expect(painted.map((row) => row.text)).toEqual(['A1:A9', 'B2', 'Sheet2!C3', '$A$1', 'Sheet2!C3']);
    expect(new Set(painted.map((row) => row.colour)).size).toBe(4);
    expect(painted[2]?.colour).toBe(painted[4]?.colour);
    expect(painted[0]?.colour).not.toBe(painted[1]?.colour);
  });

  test('emits the contract with offsets and ordered bounds, on a keystroke', async ({ page }) => {
    await open(page, colourStory);
    const emitted = await page.evaluate(async () => {
      const bar = document.getElementById('colour-bar');
      if (bar === null) return null;
      const seen = new Promise<unknown>((resolve) => {
        bar.addEventListener('mjx-formula-references', (event) => {
          resolve((event as CustomEvent<{ references: unknown }>).detail.references);
        }, { once: true });
      });
      const entry = bar.shadowRoot?.querySelector('.entry');
      if (!(entry instanceof HTMLTextAreaElement)) return null;
      entry.focus();
      entry.value = `${entry.value}+D4`;
      entry.dispatchEvent(new Event('input', { bubbles: true }));
      return seen;
    });
    const references = emitted as { text: string; from: number; to: number; slot: number; bounds: unknown }[];
    expect(references.map((reference) => reference.text)).toContain('D4');
    const last = references.at(-1);
    expect(last?.bounds).toEqual({ firstColumn: 3, lastColumn: 3, firstRow: 3, lastRow: 3 });
    expect(last?.to).toBeGreaterThan(last?.from ?? 0);
  });

  test('resolves the ring to different colours in the DARK scheme too', async ({ page }) => {
    // The slot is an identity and the token is per scheme; a ring that only worked in light would
    // pass every assertion above.
    await open(page, colourStory, { theme: 'dark' });
    const resolved = await page.evaluate((properties) => {
      const style = getComputedStyle(document.documentElement);
      return properties.map((property) => style.getPropertyValue(property).trim());
    }, referenceColourSlots.map((_, index) => referenceSlotProperty(index)));
    expect(new Set(resolved).size).toBe(referenceColourSlots.length);
  });
});

// ── brackets ─────────────────────────────────────────────────────────────────

test('a bracket pair lights only while the caret is on it', async ({ page }) => {
  await open(page, bracketStory);
  const formula = await page.locator('#bracket-bar').evaluate((element) => (element as HTMLElement & { value: string }).value);

  await caretTo(page, 'bracket-bar', formula.indexOf('SUM(') + 4);
  expect(await page.locator('#bracket-bar').evaluate((element) => element.shadowRoot?.querySelectorAll('.bracket-match').length ?? 0)).toBe(2);

  await caretTo(page, 'bracket-bar', formula.indexOf('A1:A9') + 2);
  expect(await page.locator('#bracket-bar').evaluate((element) => element.shadowRoot?.querySelectorAll('.bracket-match').length ?? 0)).toBe(0);
});

// ── the autocomplete, by keyboard alone ──────────────────────────────────────

test.describe('the function autocomplete', () => {
  test('filters, completes and dismisses with no pointer at all', async ({ page }) => {
    await open(page, completeStory);
    const bar = page.locator('#complete-bar');
    await bar.evaluate((element) => {
      const entry = element.shadowRoot?.querySelector('.entry');
      if (entry instanceof HTMLTextAreaElement) entry.focus();
    });

    // Filtering: the story starts at `=SU`, so a keystroke that narrows it to SUMIF should.
    await page.keyboard.type('M');
    await settle(page);
    const offered = await bar.evaluate((element) =>
      ((element as HTMLElement & { completions: { name: string }[] }).completions ?? []).map((entry) => entry.name),
    );
    expect(offered).toEqual(['SUM', 'SUMIF', 'SUMIFS']);

    // Moving: the arrow keys move a cursor that is announced through aria-activedescendant.
    await page.keyboard.press('ArrowDown');
    await settle(page);
    const active = await bar.evaluate((element) => {
      const entry = element.shadowRoot?.querySelector('.entry');
      const id = entry?.getAttribute('aria-activedescendant') ?? '';
      return {
        id,
        text: element.shadowRoot?.getElementById(id)?.textContent ?? '',
        name: (element as HTMLElement & { activeCompletion?: { name: string } }).activeCompletion?.name ?? '',
      };
    });
    expect(active.id).not.toBe('');
    expect(active.name).toBe('SUMIF');
    expect(active.text).toContain('SUMIF');

    // Completing: Tab takes the active one, and the caret lands inside the parentheses.
    await page.keyboard.press('Tab');
    await settle(page);
    const completed = await bar.evaluate((element) => {
      const entry = element.shadowRoot?.querySelector('.entry');
      return {
        value: (element as HTMLElement & { value: string }).value,
        caret: entry instanceof HTMLTextAreaElement ? entry.selectionStart : -1,
        open: (element as HTMLElement & { completionsOpen: boolean }).completionsOpen,
      };
    });
    expect(completed.value).toBe('=SUMIF(');
    expect(completed.caret).toBe('=SUMIF('.length);
    expect(completed.open).toBe(false);
  });

  test('Escape dismisses the list, and a SECOND Escape cancels the edit', async ({ page }) => {
    await open(page, completeStory);
    const bar = page.locator('#complete-bar');
    await bar.evaluate((element) => {
      const entry = element.shadowRoot?.querySelector('.entry');
      if (entry instanceof HTMLTextAreaElement) entry.focus();
    });
    await page.keyboard.type('M');
    await settle(page);
    expect(await bar.evaluate((element) => (element as HTMLElement & { completionsOpen: boolean }).completionsOpen)).toBe(true);

    await page.keyboard.press('Escape');
    await settle(page);
    expect(await bar.evaluate((element) => (element as HTMLElement & { completionsOpen: boolean }).completionsOpen)).toBe(false);
    // The edit is still alive: one Escape must never throw away what was typed.
    expect(await bar.evaluate((element) => (element as HTMLElement & { mode: string }).mode)).not.toBe('ready');

    await page.keyboard.press('Escape');
    await settle(page);
    expect(await bar.evaluate((element) => (element as HTMLElement & { mode: string }).mode)).toBe('ready');
  });

  test('says how many matched, because aria-expanded is not legal on a textarea', async ({ page }) => {
    await open(page, completeStory);
    const bar = page.locator('#complete-bar');
    await bar.evaluate((element) => {
      const entry = element.shadowRoot?.querySelector('.entry');
      if (entry instanceof HTMLTextAreaElement) entry.focus();
    });
    await page.keyboard.type('M');
    await settle(page);
    const spoken = await bar.evaluate((element) => element.shadowRoot?.querySelector('.live')?.textContent ?? '');
    expect(spoken).toContain('3 functions');
    expect(spoken).toContain('SUM');
    // And the attribute really is absent, rather than absent by accident.
    const expanded = await bar.evaluate((element) => element.shadowRoot?.querySelector('.entry')?.hasAttribute('aria-expanded'));
    expect(expanded).toBe(false);
  });
});

// ── the four modes ───────────────────────────────────────────────────────────

test.describe('the four modes', () => {
  test('are distinguishable, and each transition is ANNOUNCED', async ({ page }) => {
    await open(page, modeStory);
    const bar = page.locator('#mode-bar');
    const readState = async (): Promise<{ mode: string; chip: string; spoken: string }> =>
      bar.evaluate((element) => ({
        mode: (element as HTMLElement & { mode: string }).mode,
        chip: element.shadowRoot?.querySelector('.mode')?.getAttribute('data-mode') ?? '',
        spoken: element.shadowRoot?.querySelector('.live')?.textContent ?? '',
      }));

    expect((await readState()).mode).toBe('ready');

    await bar.evaluate((element) => {
      const entry = element.shadowRoot?.querySelector('.entry');
      if (entry instanceof HTMLTextAreaElement) entry.focus();
    });
    await settle(page);
    const edit = await readState();
    expect(edit.mode).toBe('edit');
    expect(edit.chip).toBe('edit');
    expect(edit.spoken).toBe(formulaModes.edit.announcement);

    // `=` puts the caret where an arrow key would name a range.
    await page.keyboard.type('=');
    await settle(page);
    const point = await readState();
    expect(point.mode).toBe('point');
    expect(point.spoken).toBe(formulaModes.point.announcement);

    // …and a reference takes it back out again, to the mode it came from.
    await page.keyboard.type('A1');
    await settle(page);
    expect((await readState()).mode).toBe('edit');

    await page.keyboard.press('Escape');
    await settle(page);
    const ready = await readState();
    expect(ready.mode).toBe('ready');
    expect(ready.spoken).toBe(formulaModes.ready.announcement);
  });

  test('reaches ENTER mode — the fourth — and announces it too', async ({ page }) => {
    // ⚠ The mode a component alone cannot enter: Excel enters it when a person types into a CELL,
    // and there is no grid in this loop. The story's button is what a shell would do, and without
    // this assertion "all four modes are distinguishable" would cover three.
    await open(page, modeStory);
    await page.locator('#mode-begin-entry').click();
    await settle(page);
    const entered = await page.locator('#mode-bar').evaluate((element) => ({
      mode: (element as HTMLElement & { mode: string }).mode,
      chip: element.shadowRoot?.querySelector('.mode')?.getAttribute('data-mode') ?? '',
      spoken: element.shadowRoot?.querySelector('.live')?.textContent ?? '',
    }));
    expect(entered.mode).toBe('enter');
    expect(entered.chip).toBe('enter');
    expect(entered.spoken).toBe(formulaModes.enter.announcement);

    // …and Point returns to ENTER rather than to Edit, which is the whole reason the mode is a
    // state with a memory rather than a value.
    await page.keyboard.type('=');
    await settle(page);
    expect(await page.locator('#mode-bar').evaluate((element) => (element as HTMLElement & { mode: string }).mode)).toBe('point');
    await page.keyboard.type('A1');
    await settle(page);
    expect(await page.locator('#mode-bar').evaluate((element) => (element as HTMLElement & { mode: string }).mode)).toBe('enter');
  });

  test('draws all FOUR modes differently, and no two of them alike', async ({ page }) => {
    await open(page, modeStory);
    const bar = page.locator('#mode-bar');
    const drawing = async (): Promise<string> =>
      bar.evaluate((element) => {
        const chip = element.shadowRoot?.querySelector('.mode');
        const dot = element.shadowRoot?.querySelector('.mode-dot');
        if (!(chip instanceof HTMLElement) || !(dot instanceof HTMLElement)) return '';
        const mark = getComputedStyle(dot);
        return [
          getComputedStyle(chip).backgroundColor,
          mark.borderRadius,
          mark.rotate,
          mark.height,
          mark.backgroundColor,
          mark.borderTopWidth,
        ].join('|');
      });
    const seen = new Set<string>();
    seen.add(await drawing());
    await page.locator('#mode-begin-entry').click();
    await settle(page);
    seen.add(await drawing());
    await page.keyboard.type('=');
    await settle(page);
    seen.add(await drawing());
    await page.keyboard.press('Escape');
    await settle(page);
    await page.locator('#mode-begin-edit').click();
    await settle(page);
    seen.add(await drawing());
    // Four modes, four drawings. A shape repeated between two of them is two states a reader
    // cannot tell apart, whether or not the fill differs.
    expect(seen.size).toBe(4);
  });

  test('draws each mode as a different SHAPE as well as a different fill', async ({ page }) => {
    // A state told only in colour is a state some readers cannot read.
    await open(page, modeStory);
    const bar = page.locator('#mode-bar');
    const shapeOf = async (): Promise<string> =>
      bar.evaluate((element) => {
        const dot = element.shadowRoot?.querySelector('.mode-dot');
        if (!(dot instanceof HTMLElement)) return '';
        const style = getComputedStyle(dot);
        return [style.borderRadius, style.rotate, style.height, style.backgroundColor].join('|');
      });
    const ready = await shapeOf();
    await bar.evaluate((element) => {
      (element as HTMLElement & { beginEdit: () => void }).beginEdit();
    });
    await settle(page);
    const editing = await shapeOf();
    expect(editing).not.toBe(ready);
  });

  test('makes confirm and cancel unavailable until there is something to confirm', async ({ page }) => {
    await open(page, modeStory);
    const bar = page.locator('#mode-bar');
    const disabled = async (): Promise<boolean[]> =>
      bar.evaluate((element) =>
        ['cancel', 'confirm'].map(
          (key) => element.shadowRoot?.querySelector(`.affordance.${key}`)?.hasAttribute('disabled') ?? false,
        ),
      );
    expect(await disabled()).toEqual([true, true]);
    await bar.evaluate((element) => {
      (element as HTMLElement & { beginEdit: () => void }).beginEdit();
    });
    await settle(page);
    expect(await disabled()).toEqual([false, false]);
  });
});

// ── the height handle ────────────────────────────────────────────────────────

test.describe('the expansion handle', () => {
  test('is a separator with a value, and grows the editor by keyboard alone', async ({ page }) => {
    await open(page, rowsStory);
    const bar = page.locator('#rows-bar');
    const handle = page.locator('#rows-bar').locator('.handle');
    await handle.focus();

    const before = await bar.evaluate((element) => (element as HTMLElement & { rows: number }).rows);
    expect(before).toBe(formulaRowBounds.min);
    expect(await handle.getAttribute('role')).toBe('separator');
    expect(await handle.getAttribute('aria-valuemin')).toBe(String(formulaRowBounds.min));
    expect(await handle.getAttribute('aria-valuemax')).toBe(String(formulaRowBounds.max));

    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
    await settle(page);
    expect(await bar.evaluate((element) => (element as HTMLElement & { rows: number }).rows)).toBe(3);
    expect(await handle.getAttribute('aria-valuenow')).toBe('3');

    await page.keyboard.press('End');
    await settle(page);
    expect(await bar.evaluate((element) => (element as HTMLElement & { rows: number }).rows)).toBe(formulaRowBounds.max);

    await page.keyboard.press('Home');
    await settle(page);
    expect(await bar.evaluate((element) => (element as HTMLElement & { rows: number }).rows)).toBe(formulaRowBounds.min);
  });

  test('really makes the editor taller, and says so with aria-multiline', async ({ page }) => {
    await open(page, rowsStory);
    const bar = page.locator('#rows-bar');
    const heightOf = async (): Promise<number> =>
      bar.evaluate((element) => element.shadowRoot?.querySelector('.entry')?.getBoundingClientRect().height ?? 0);
    const one = await heightOf();
    await page.locator('#rows-bar').locator('.handle').focus();
    await page.keyboard.press('End');
    await settle(page);
    expect(await heightOf()).toBeGreaterThan(one);
    expect(
      await bar.evaluate((element) => element.shadowRoot?.querySelector('.entry')?.getAttribute('aria-multiline')),
    ).toBe('true');
  });

  test('does not claim Tab, so the handle is not a trap', async ({ page }) => {
    await open(page, rowsStory);
    const handle = page.locator('#rows-bar').locator('.handle');
    await handle.focus();
    const rows = await page.locator('#rows-bar').evaluate((element) => (element as HTMLElement & { rows: number }).rows);
    await page.keyboard.press('Tab');
    await settle(page);
    expect(await page.locator('#rows-bar').evaluate((element) => (element as HTMLElement & { rows: number }).rows)).toBe(rows);
    const stillOnHandle = await page.evaluate(() => {
      const active = document.activeElement;
      const inner = active?.shadowRoot?.activeElement;
      return inner?.classList.contains('handle') ?? false;
    });
    expect(stillOnHandle).toBe(false);
  });
});

// ── the name box ─────────────────────────────────────────────────────────────

test.describe('the name box', () => {
  test('emits a navigation request for an address, with the bounds already parsed', async ({
    page,
  }) => {
    await open(page, navStory);
    const request = await page.evaluate(async () => {
      const box = document.getElementById('nav-box');
      if (box === null) return null;
      const seen = new Promise<unknown>((resolve) => {
        box.addEventListener('mjx-name-navigate', (event) => {
          resolve((event as CustomEvent<unknown>).detail);
        }, { once: true });
      });
      const entry = box.shadowRoot?.querySelector('.entry');
      if (!(entry instanceof HTMLInputElement)) return null;
      entry.focus();
      entry.value = 'c9';
      entry.dispatchEvent(new Event('input', { bubbles: true }));
      entry.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
      return seen;
    });
    expect(request).toMatchObject({
      kind: 'address',
      text: 'C9',
      bounds: { firstColumn: 2, lastColumn: 2, firstRow: 8, lastRow: 8 },
    });
  });

  test('emits one for a defined name too, with what the name points at', async ({ page }) => {
    await open(page, namesStory);
    const request = await page.evaluate(async () => {
      const box = document.getElementById('list-box');
      if (box === null) return null;
      const seen = new Promise<unknown>((resolve) => {
        box.addEventListener('mjx-name-navigate', (event) => {
          resolve((event as CustomEvent<unknown>).detail);
        }, { once: true });
      });
      const entry = box.shadowRoot?.querySelector('.entry');
      if (!(entry instanceof HTMLInputElement)) return null;
      entry.focus();
      entry.value = 'revenue';
      entry.dispatchEvent(new Event('input', { bubbles: true }));
      entry.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
      return seen;
    });
    expect(request).toMatchObject({ kind: 'name', entry: { name: 'Revenue', definition: 'Summary!$B$1' } });
  });

  test('keeps U07’s invariant: the text it shows is the value it reports', async ({ page }) => {
    await open(page, navStory);
    const agreed = await page.evaluate(async () => {
      const box = document.getElementById('nav-box');
      const entry = box?.shadowRoot?.querySelector('.entry');
      if (box === null || !(entry instanceof HTMLInputElement)) return null;
      entry.focus();
      entry.value = 'b7';
      entry.dispatchEvent(new Event('input', { bubbles: true }));
      entry.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
      await new Promise((done) => requestAnimationFrame(done));
      return { value: (box as HTMLElement & { value: string }).value, text: entry.value };
    });
    expect(agreed?.value).toBe('B7');
    expect(agreed?.text).toBe('B7');
  });

  test('refuses a reference outside the grid and reverts rather than navigating', async ({ page }) => {
    await open(page, navStory);
    const outcome = await page.evaluate(async () => {
      const box = document.getElementById('nav-box');
      const entry = box?.shadowRoot?.querySelector('.entry');
      if (box === null || !(entry instanceof HTMLInputElement)) return null;
      let navigated = false;
      box.addEventListener('mjx-name-navigate', () => {
        navigated = true;
      });
      const refused = new Promise<string>((resolve) => {
        box.addEventListener('mjx-input-invalid', (event) => {
          resolve((event as CustomEvent<{ text: string }>).detail.text);
        }, { once: true });
      });
      entry.focus();
      entry.value = 'XFE1';
      entry.dispatchEvent(new Event('input', { bubbles: true }));
      entry.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
      const text = await refused;
      return { text, navigated, value: (box as HTMLElement & { value: string }).value };
    });
    expect(outcome?.text).toBe('XFE1');
    expect(outcome?.navigated).toBe(false);
  });
});

// ── the floor ────────────────────────────────────────────────────────────────

test('every affordance clears the 24-pixel hit-target floor, in compact density', async ({ page }) => {
  await open(page, denseBarStory);
  const boxes = await page.locator('#dense-bar').evaluate((element) => {
    const parts = element.shadowRoot?.querySelectorAll('.affordance, .handle') ?? [];
    return [...parts].map((part) => {
      const box = part.getBoundingClientRect();
      return { width: box.width, height: box.height };
    });
  });
  // Anti-vacuity: a selector that matched nothing would satisfy every ceiling below.
  expect(boxes.length).toBe(4);
  for (const box of boxes) {
    expect(box.width).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
    expect(box.height).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
  }
});
