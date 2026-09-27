import { expect, test, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { tokens, type ColorScheme } from '../../tokens/tokens.ts';
import { accessibleHitTargetMinimum, densityModes } from '../../src/foundations/density.ts';
import {
  essentialCommandLimit,
  groupPresentationAt,
  groupPresentationProperty,
  groupPresentations,
  groupPriorities,
  groupPriorityNames,
  ribbonStates,
  tabStripPickerAtOrBelow,
  tabStripPresentationAt,
  tabStripPresentationProperty,
  tabTones,
  type GroupPresentation,
  type GroupPriority,
} from '../../src/ribbon/ribbon-model.ts';
import { wordTabHomeControlCount, wordTabHomeGroups } from '../../dev/word-tab-home.ts';

/**
 * The ribbon, measured in a browser.
 *
 * MJXOFF-183 says which assertion matters and why the obvious one does not:
 *
 * > *"The ribbon is responsive"* is satisfied by a layout that merely wraps or scrolls
 * > horizontally, which looks plausible in a screenshot and is unusable in practice. **The gate
 * > must assert the three group presentations are actually reached at declared container widths**,
 * > and that a collapsed group's commands are still *reachable* — a control that disappears at
 * > narrow width has not degraded, it has been lost. **That reachability assertion is the one that
 * > matters.**
 *
 * And MJXOFF-182 says what shape the assertion has to have:
 *
 * > **A distinctness gate proves no two states are the same; it does not prove any of them is
 * > right.**
 *
 * So nothing below asserts that the layout *changed*. Every presentation assertion is a comparison
 * against `groupPresentationAt()` — computed from the container width the browser actually gave the
 * ribbon — and against `groupPresentations`, which says what each one must look like. A rule that
 * collapsed the wrong group at the wrong width would satisfy every "it responds" check and fails
 * here by name.
 *
 * ## Four instruments per presentation, and only one of them is the component's own word
 *
 * `--mjx-group-presentation` is what the component reads to know whether its panel is a popup, so a
 * gate that read only that would be asking the implementation to grade its own homework: a
 * stylesheet that set the token and changed no layout would pass. Each presentation is therefore
 * cross-checked against three facts the token does not control — the trigger's `display`, the
 * panel's `position`, and the density step inside the group.
 */

/**
 * ⚠ **Every selector below is written so that finding nothing fails loudly**, which is the
 * mitigation MJXOFF-182 established after a byte budget measured 342 bytes of a 14 kB payload. The
 * reader throws outright when `.group`, `.trigger` or `.panel` is missing; every other site
 * compares against expected *content* or an expected count, so an empty sweep fails with a
 * mismatch rather than passing with nothing. The two custom-property names are interpolated from
 * the model instead, because those are shared with the component's own read-back and a divergence
 * there would be silent in both directions.
 */
const ladderStory = { title: 'Ribbon/Ribbon', name: 'The Priority Ladder' } as const;
const contextualStory = { title: 'Ribbon/Ribbon', name: 'Contextual Tab Sets' } as const;
const statesStory = { title: 'Ribbon/Ribbon', name: 'Ribbon States' } as const;
const simplifiedStory = { title: 'Ribbon/Ribbon', name: 'The Simplified Form' } as const;
const worstCaseStory = { title: 'Ribbon/Word TabHome', name: 'The Worst Case' } as const;
const wordHomeStory = { title: 'Ribbons/Word', name: 'Home' } as const;

/** Open a story by title and name, failing with the name rather than with `undefined`. */
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
  await page.waitForFunction(() => customElements.get('mjx-ribbon') !== undefined);
  await settle(page);
}

async function settle(page: Page): Promise<void> {
  await page.evaluate(async () => {
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
}

/**
 * Two frames is enough for a layout change and **not** enough for a paint change.
 *
 * Every control in this catalogue wears `.mjx-motion-surface-settle`, so a background read two
 * frames after a selection change is an interpolated colour part-way between the old one and the
 * new — `rgba(251, 239, 216, 0.718)` where the token says `rgb(251, 239, 216)`. That is what this
 * suite measured on its first run. The wait is **derived from the token**, twice over plus a
 * frame's grace, exactly as `controls.spec.ts` derives its own: a fixed number here would be the
 * literal the whole child is written against, and a re-seed that slowed the platform's motion would
 * make the suite flaky instead of failing.
 */
const transitionMilliseconds = Number.parseFloat(tokens.duration.transition);

async function settlePaint(page: Page): Promise<void> {
  await settle(page);
  await page.waitForTimeout(transitionMilliseconds * 2 + 100);
}

/** Drive the **outermost** harness container. The viewport is never touched. */
async function setContainerWidth(page: Page, width: number): Promise<void> {
  await page.evaluate((value) => {
    document.querySelector('mjx-resizable-container')?.setAttribute('width', String(value));
  }, width);
  await settle(page);
}

interface GroupReading {
  label: string;
  priority: string;
  presentation: string;
  triggerDisplay: string;
  panelPosition: string;
  panelDisplay: string;
  densityStep: string;
  triggerMinBlock: number;
  commands: number;
  essential: number;
  renderedCommands: number;
}

interface RibbonReading {
  containerWidth: number;
  frameWidth: number;
  viewport: number;
  stripPresentation: string;
  groups: GroupReading[];
}

/**
 * Everything one ribbon says about itself, in one round trip.
 *
 * Only the groups in the **selected** tab are read: the others are inside a `display: none` panel,
 * where a container query has nothing to resolve against and would report an answer that has
 * nothing to do with the layout.
 */
const readRibbon = `() => {
  const ribbon = document.querySelector('mjx-ribbon');
  if (ribbon === null) throw new Error('no ribbon in the story');
  const frame = document
    .querySelector('mjx-resizable-container')
    ?.shadowRoot?.querySelector('.frame');
  const groups = [...document.querySelectorAll('mjx-ribbon-group')].filter(
    (group) => group.closest('mjx-ribbon-tab[selected]') !== null,
  );
  return {
    containerWidth: ribbon.clientWidth,
    frameWidth: frame instanceof HTMLElement ? frame.clientWidth : -1,
    viewport: window.innerWidth,
    stripPresentation: getComputedStyle(ribbon.shadowRoot.querySelector('.strip'))
      .getPropertyValue('${tabStripPresentationProperty}')
      .trim(),
    groups: groups.map((group) => {
      const box = group.shadowRoot.querySelector('.group');
      const trigger = group.shadowRoot.querySelector('.trigger');
      const panel = group.shadowRoot.querySelector('.panel');
      const boxStyle = getComputedStyle(box);
      const triggerStyle = getComputedStyle(trigger);
      const panelStyle = getComputedStyle(panel);
      const children = [...group.children];
      // Every command the group holds, survivors included. Until unit 2b this counted only the
      // children with no slot, so a survivor lost at narrow width would not have been counted as
      // lost — and demotionRules' rule 4 says this gate counts every command at all three widths.
      const allCommands = children.filter((child) => {
        const slot = child.getAttribute('slot');
        return slot === null || slot === '' || slot === 'essential';
      });
      return {
        label: group.getAttribute('label') ?? '',
        priority: group.getAttribute('priority') ?? 'standard',
        presentation: boxStyle.getPropertyValue('${groupPresentationProperty}').trim(),
        triggerDisplay: triggerStyle.display,
        panelPosition: panelStyle.position,
        panelDisplay: panelStyle.display,
        densityStep: boxStyle.getPropertyValue('--mjx-density-step').trim(),
        triggerMinBlock: Number.parseFloat(triggerStyle.minBlockSize),
        commands: allCommands.length,
        essential: children.filter((child) => child.getAttribute('slot') === 'essential').length,
        renderedCommands: allCommands.filter((command) => {
          const rect = command.getBoundingClientRect();
          return rect.width > 0 && rect.height > 0;
        }).length,
      };
    }),
  };
}`;

async function read(page: Page): Promise<RibbonReading> {
  return page.evaluate(
    (source) => (new Function(`return ${source}`)() as () => unknown)() as never,
    readRibbon,
  );
}

/** The density step a presentation runs at, as `getComputedStyle` reports it. */
function expectedDensityStep(presentation: GroupPresentation): string {
  const units = densityModes[groupPresentations[presentation].density].stepUnits;
  return units === 1 ? '.25rem' : `calc(.25rem * ${String(units)})`;
}

/** `#2e9e63` → `rgb(46, 158, 99)`. */
function expectedColor(member: string, scheme: ColorScheme): string {
  const hex = tokens.theme[scheme][member as keyof (typeof tokens.theme)['light']];
  const number = Number.parseInt(hex.slice(1, 7), 16);
  return `rgb(${String((number >> 16) & 0xff)}, ${String((number >> 8) & 0xff)}, ${String(number & 0xff)})`;
}

/** The deepest focused element, described in terms a test can assert on. */
const activeDescriptor = `() => {
  let element = document.activeElement;
  while (element && element.shadowRoot && element.shadowRoot.activeElement) {
    element = element.shadowRoot.activeElement;
  }
  const root = element ? element.getRootNode() : null;
  const host = root instanceof ShadowRoot ? root.host : null;
  return {
    tag: element ? element.tagName.toLowerCase() : null,
    className: element instanceof HTMLElement ? element.className : '',
    hostTag: host ? host.tagName.toLowerCase() : null,
    hostLabel: host ? host.getAttribute('label') : null,
    groupLabel: host && host.closest ? (host.closest('mjx-ribbon-group')?.getAttribute('label') ?? null) : null,
    text: element ? (element.textContent ?? '').trim() : '',
    tabIndex: element instanceof HTMLElement ? element.tabIndex : null,
    role: element ? element.getAttribute('role') : null,
    tabId: element ? element.getAttribute('data-tab') : null,
  };
}`;

interface ActiveDescriptor {
  tag: string | null;
  className: string;
  hostTag: string | null;
  hostLabel: string | null;
  groupLabel: string | null;
  text: string;
  tabIndex: number | null;
  role: string | null;
  tabId: string | null;
}

async function focused(page: Page): Promise<ActiveDescriptor> {
  return page.evaluate(
    (source) => (new Function(`return ${source}`)() as () => unknown)() as never,
    activeDescriptor,
  );
}

// ─────────────────────────────────────────────────────────────────────────────

test.describe('the three group presentations, at declared container widths', () => {
  /**
   * Every boundary in the ladder, both sides — and the presets between them.
   *
   * Derived from `groupPriorities` rather than written out, so a ladder that moves cannot leave
   * this list asserting the old answer. That is the same reasoning `container.spec.ts` gives for
   * computing its expected bands.
   */
  const widths = [
    ...new Set(
      groupPriorityNames.flatMap((priority) => [
        groupPriorities[priority].reduceAtOrBelow + 1,
        groupPriorities[priority].reduceAtOrBelow,
        groupPriorities[priority].collapseAtOrBelow + 1,
        groupPriorities[priority].collapseAtOrBelow,
      ]),
    ),
    1440,
    1000,
    834,
    390,
  ].sort((left, right) => right - left);

  test('every group reaches the presentation the ladder says, and looks like it', async ({
    page,
  }) => {
    await open(page, ladderStory);
    const viewportBefore = await page.evaluate(() => window.innerWidth);

    for (const width of widths) {
      await setContainerWidth(page, width);
      const reading = await read(page);
      expect(reading.groups, `no groups were found at ${String(width)}px`).toHaveLength(
        groupPriorityNames.length,
      );

      for (const group of reading.groups) {
        // The expectation is computed from the width the browser actually gave the container,
        // which is the honest number: a scrollbar or a stray padding would change it, and the
        // separate content-box assertion below is what catches that.
        const expected = groupPresentationAt(
          group.priority as GroupPriority,
          reading.containerWidth,
        );
        const spec = groupPresentations[expected];
        const where = `${group.label} at ${String(reading.containerWidth)}px`;

        expect(group.presentation, `${where}: presentation`).toBe(expected);

        // The three facts the presentation token does not control.
        expect(
          group.triggerDisplay === 'none',
          `${where}: the collapse trigger should be ${spec.triggerVisible ? 'drawn' : 'gone'}`,
        ).toBe(!spec.triggerVisible);
        expect(
          group.panelPosition,
          `${where}: the panel is ${spec.panelIsPopup ? 'a popup' : 'part of the strip'}`,
        ).toBe(spec.panelIsPopup ? 'absolute' : 'static');
        expect(group.densityStep, `${where}: density is ${spec.density}`).toBe(
          expectedDensityStep(expected),
        );
      }
    }

    expect(
      await page.evaluate(() => window.innerWidth),
      'the container changed every group and the viewport stayed exactly where it was. If this ' +
        'fails, the ribbon has become a viewport mechanism and a group in a narrow task pane ' +
        'inside a wide window would never collapse.',
    ).toBe(viewportBefore);
  });

  test('three groups sit in three presentations at one width', async ({ page }) => {
    await open(page, ladderStory);
    await setContainerWidth(page, 1000);
    const reading = await read(page);
    const presentations = new Set(reading.groups.map((group) => group.presentation));
    expect(
      presentations.size,
      'a single global breakpoint cannot produce this, which is the whole argument for a ' +
        'per-group priority.',
    ).toBe(3);
  });

  test('the query container is the ribbon, and it has no chrome inside it', async ({ page }) => {
    await open(page, ladderStory);
    for (const width of [1440, 834, 390]) {
      await setContainerWidth(page, width);
      const reading = await read(page);
      // The U02 lesson applied to a second container: a container query resolves against the
      // content box, so padding on the ribbon's host would move every group's boundary in the
      // catalogue and nothing would say so.
      expect(reading.containerWidth, `the ribbon is not ${String(width)}px wide`).toBe(width);
      expect(reading.frameWidth).toBe(width);
    }
  });

  test('the tab strip becomes a picker at the width it declares', async ({ page }) => {
    await open(page, ladderStory);
    for (const width of [
      tabStripPickerAtOrBelow + 1,
      tabStripPickerAtOrBelow,
      834,
      390,
    ]) {
      await setContainerWidth(page, width);
      const reading = await read(page);
      expect(reading.stripPresentation, `at ${String(width)}px`).toBe(
        tabStripPresentationAt(reading.containerWidth),
      );
    }
  });

  test('the simplified form refuses full and never prevents a collapse', async ({ page }) => {
    await open(page, simplifiedStory);
    for (const width of [1440, 834, 390]) {
      await setContainerWidth(page, width);
      const reading = await read(page);
      for (const group of reading.groups) {
        expect(group.presentation, `${group.label} at ${String(width)}px, simplified`).toBe(
          groupPresentationAt(group.priority as GroupPriority, reading.containerWidth, {
            simplified: true,
          }),
        );
      }
    }
  });
});

test.describe('reachability — the assertion that matters', () => {
  test('every command is still there, and still drawn, in all three presentations', async ({
    page,
  }) => {
    await open(page, ladderStory);

    // What the groups hold when there is room for all of it.
    await setContainerWidth(page, 1440);
    const full = await read(page);
    const expectedCounts = new Map(full.groups.map((group) => [group.label, group.commands]));
    expect(expectedCounts.size).toBe(groupPriorityNames.length);

    for (const width of [1440, 834, 390]) {
      await setContainerWidth(page, width);
      const reading = await read(page);
      for (const group of reading.groups) {
        expect(
          group.commands,
          `${group.label} holds a different number of commands at ${String(width)}px. A control ` +
            'that disappears at narrow width has not degraded, it has been lost.',
        ).toBe(expectedCounts.get(group.label));
      }
    }

    // …and when a collapsed group is opened, every one of them is drawn.
    await setContainerWidth(page, 390);
    await page.evaluate(() => {
      for (const group of document.querySelectorAll('mjx-ribbon-group')) {
        group.setAttribute('open', '');
      }
    });
    await settle(page);
    const opened = await read(page);
    for (const group of opened.groups) {
      expect(group.presentation).toBe('collapsed');
      expect(
        group.renderedCommands,
        `${group.label} is collapsed and open, and ${String(
          group.commands - group.renderedCommands,
        )} of its commands are not drawn.`,
      ).toBe(group.commands);
    }
  });

  test('the reachability gate rejects a command that has been lost', async ({ page }) => {
    // **Proof that the assertion above can fail.** The ticket asks for it directly: *"Prove it can
    // fail by dropping a control from the collapsed menu."* One command is hidden in the page, and
    // the same reading the gate above makes must now report the loss — which is what says the gate
    // is watching whether a command is *drawn* and not merely whether it is in the DOM.
    await open(page, ladderStory);
    await setContainerWidth(page, 390);
    await page.evaluate(() => {
      for (const group of document.querySelectorAll('mjx-ribbon-group')) {
        group.setAttribute('open', '');
      }
      const victim = document
        .querySelector('mjx-ribbon-tab[selected] mjx-ribbon-group')
        ?.querySelector('mjx-button');
      if (!(victim instanceof HTMLElement)) throw new Error('no command to drop');
      victim.style.display = 'none';
    });
    await settle(page);
    const reading = await read(page);
    const first = reading.groups[0];
    expect(first).toBeDefined();
    if (first === undefined) return;
    expect(
      first.renderedCommands,
      'a command was hidden and the reading still counted it as drawn, so the reachability ' +
        'assertion is not watching anything.',
    ).toBe(first.commands - 1);
  });
});

test.describe('the collapsed group is a popup, and a keyboard can use it', () => {
  test('opens by keyboard, traps focus, and gives it back on Escape', async ({ page }) => {
    await open(page, ladderStory);
    await setContainerWidth(page, 390);

    // Reach the first group's trigger the way a person does.
    await page.evaluate(() => {
      const group = document.querySelector('mjx-ribbon-tab[selected] mjx-ribbon-group');
      const trigger = group?.shadowRoot?.querySelector('.trigger');
      if (trigger instanceof HTMLElement) trigger.focus();
    });
    const onTrigger = await focused(page);
    expect(onTrigger.className).toContain('trigger');
    expect(await page.evaluate(() => {
      const group = document.querySelector('mjx-ribbon-tab[selected] mjx-ribbon-group');
      return group?.shadowRoot?.querySelector('.trigger')?.getAttribute('aria-expanded');
    })).toBe('false');

    // Enter opens it and leaves the keyboard on the trigger — a person who pressed Enter asked to
    // see the group, not to be moved into it.
    await page.keyboard.press('Enter');
    await settle(page);
    expect((await focused(page)).className, 'Enter moved the keyboard off the trigger').toContain(
      'trigger',
    );
    expect(
      await page.evaluate(() =>
        document.querySelector('mjx-ribbon-tab[selected] mjx-ribbon-group')?.hasAttribute('open'),
      ),
    ).toBe(true);
    await page.keyboard.press('Escape');
    await settle(page);

    // Arrow Down opens it *and* moves the keyboard inside, which Enter deliberately does not.
    await page.keyboard.press('ArrowDown');
    await settle(page);
    const inside = await focused(page);
    expect(inside.hostTag, 'Arrow Down did not put focus on a command').toBe('mjx-button');
    expect(inside.groupLabel).toBe('Primary');
    expect(await page.evaluate(() => {
      const group = document.querySelector('mjx-ribbon-tab[selected] mjx-ribbon-group');
      return group?.shadowRoot?.querySelector('.trigger')?.getAttribute('aria-expanded');
    })).toBe('true');

    // The trap: more presses than there are stops, and focus is still in this group.
    for (let step = 0; step < 12; step += 1) {
      await page.keyboard.press('Tab');
      await settle(page);
      const where = await focused(page);
      expect(
        where.groupLabel ?? where.className,
        `Tab number ${String(step + 1)} left the open popup. A collapsed group a keyboard can ` +
          'fall out of is a group a keyboard cannot get back into.',
      ).toContain('Primary');
    }

    // Backwards, too.
    for (let step = 0; step < 4; step += 1) {
      await page.keyboard.press('Shift+Tab');
      await settle(page);
      const where = await focused(page);
      expect(where.groupLabel ?? where.className).toContain('Primary');
    }

    await page.keyboard.press('Escape');
    await settle(page);
    const back = await focused(page);
    expect(back.className, 'Escape did not return focus to the button that opened the popup').toContain(
      'trigger',
    );
    expect(
      await page.evaluate(() =>
        document.querySelector('mjx-ribbon-tab[selected] mjx-ribbon-group')?.hasAttribute('open'),
      ),
    ).toBe(false);
  });

  test('keeps the collapse trigger above the accessible hit-target floor in compact density', async ({
    page,
  }) => {
    await open(page, ladderStory);
    await setContainerWidth(page, 390);
    const reading = await read(page);
    for (const group of reading.groups) {
      // The collapsed presentation runs compact, which is exactly where a hit target quietly
      // shrinks below the floor.
      expect(group.presentation).toBe('collapsed');
      expect(
        group.triggerMinBlock,
        `${group.label}'s collapse trigger is ${String(group.triggerMinBlock)}px tall in compact ` +
          'density, and WCAG 2.2 Target Size (Minimum) is the floor.',
      ).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
    }
  });
});

test.describe('the tab strip follows the ARIA tabs pattern', () => {
  test('the selected tab is the only tab stop, arrows move it, and Tab leaves', async ({ page }) => {
    await open(page, ladderStory);
    await setContainerWidth(page, 1440);

    const tabState = async (): Promise<{ id: string; selected: string; tabIndex: number }[]> =>
      page.evaluate(() => {
        const ribbon = document.querySelector('mjx-ribbon');
        return [...(ribbon?.shadowRoot?.querySelectorAll('.tab') ?? [])].map((tab) => ({
          id: tab.getAttribute('data-tab') ?? '',
          selected: tab.getAttribute('aria-selected') ?? '',
          tabIndex: (tab as HTMLElement).tabIndex,
        }));
      });

    const before = await tabState();
    expect(before.length).toBeGreaterThan(1);
    expect(
      before.filter((tab) => tab.tabIndex === 0),
      'a roving tabindex means exactly one stop in the whole tablist; anything else makes a ' +
        'ribbon with ten tabs cost ten presses to walk past.',
    ).toHaveLength(1);
    expect(before.find((tab) => tab.tabIndex === 0)?.selected).toBe('true');

    // Focus the selected tab and arrow along it.
    await page.evaluate(() => {
      const ribbon = document.querySelector('mjx-ribbon');
      const tab = ribbon?.shadowRoot?.querySelector('.tab[aria-selected="true"]');
      if (tab instanceof HTMLElement) tab.focus();
    });
    await page.keyboard.press('ArrowRight');
    await settle(page);
    const moved = await tabState();
    expect(moved[1]?.selected, 'Arrow Right did not select the next tab').toBe('true');
    expect(moved[0]?.tabIndex).toBe(-1);
    expect(moved[1]?.tabIndex).toBe(0);
    expect((await focused(page)).role).toBe('tab');

    await page.keyboard.press('End');
    await settle(page);
    const atEnd = await tabState();
    expect(atEnd[atEnd.length - 1]?.selected).toBe('true');
    await page.keyboard.press('Home');
    await settle(page);
    const atHome = await tabState();
    expect(atHome[0]?.selected).toBe('true');

    // Tab leaves the tablist rather than walking along it.
    await page.keyboard.press('Tab');
    await settle(page);
    expect(
      (await focused(page)).role,
      'Tab walked to another tab. The tablist is one stop; the arrows are how you move inside it.',
    ).not.toBe('tab');
  });

  test('the narrow-width picker opens by keyboard and gives focus back', async ({ page }) => {
    await open(page, ladderStory);
    await setContainerWidth(page, 390);
    const hiddenList = async (): Promise<string> =>
      page.evaluate(() => {
        const ribbon = document.querySelector('mjx-ribbon');
        const tabs = ribbon?.shadowRoot?.querySelector('.tabs');
        return tabs === null || tabs === undefined ? '' : getComputedStyle(tabs).display;
      });

    expect(
      await hiddenList(),
      'the picker is closed, so the tablist is a popup that is not showing',
    ).toBe('none');

    await page.evaluate(() => {
      const picker = document.querySelector('mjx-ribbon')?.shadowRoot?.querySelector('.picker');
      if (picker instanceof HTMLElement) picker.focus();
    });
    await page.keyboard.press('Enter');
    await settle(page);
    expect(await hiddenList()).not.toBe('none');
    expect((await focused(page)).role, 'opening the picker did not put focus on a tab').toBe('tab');

    await page.keyboard.press('ArrowDown');
    await settle(page);
    const afterArrow = await page.evaluate(
      () =>
        document.querySelector('mjx-ribbon')?.shadowRoot?.querySelector('.tab[aria-selected="true"]')
          ?.getAttribute('data-tab') ?? '',
    );
    expect(afterArrow, 'Arrow Down does not move in a vertical tablist').not.toBe('home');

    await page.keyboard.press('Escape');
    await settle(page);
    expect(await hiddenList()).toBe('none');
    expect(
      (await focused(page)).className,
      'Escape did not return focus to the picker button',
    ).toContain('picker');
  });
});

test.describe('contextual tab sets', () => {
  test('are titled, are told apart by a token pair, and say their set in their own name', async ({
    page,
  }) => {
    await open(page, contextualStory);
    await settlePaint(page);

    const bands = await page.evaluate(() => {
      const ribbon = document.querySelector('mjx-ribbon');
      return [...(ribbon?.shadowRoot?.querySelectorAll('.tab-set-title') ?? [])].map((title) => ({
        text: (title.textContent ?? '').trim(),
        background: getComputedStyle(title).backgroundColor,
        hidden: title.getAttribute('aria-hidden'),
      }));
    });
    expect(bands.map((band) => band.text)).toEqual(['Picture Tools', 'Chart Tools']);

    // Correspondence, not distinctness: the band is the token the model names, and the tone is
    // not merely "different from a core tab".
    const contextualBand = tabTones.contextual.bandBackground;
    expect(contextualBand).toBeDefined();
    if (contextualBand !== undefined) {
      for (const band of bands) {
        expect(band.background).toBe(expectedColor(contextualBand, 'light'));
        expect(band.hidden, 'the band repeats what the tab names already say').toBe('true');
      }
    }

    const names = await page.evaluate(() => {
      const ribbon = document.querySelector('mjx-ribbon');
      return [...(ribbon?.shadowRoot?.querySelectorAll('.tab') ?? [])].map((tab) => ({
        id: tab.getAttribute('data-tab') ?? '',
        tone: tab.getAttribute('data-tone') ?? '',
        name: (tab.textContent ?? '').replace(/\s+/g, ' ').trim(),
      }));
    });
    const pictureFormat = names.find((tab) => tab.id === 'picture-format');
    expect(pictureFormat?.tone).toBe('contextual');
    expect(
      pictureFormat?.name,
      'a contextual tab whose name is only "Format" is indistinguishable from three other ' +
        'tab sets’ Format tabs, and the coloured band that tells them apart is a picture.',
    ).toBe('Format, Picture Tools');
    expect(names.find((tab) => tab.id === 'home')?.tone).toBe('core');
  });

  test('a selected contextual tab computes the contextual paint, and a core one the core paint', async ({
    page,
  }) => {
    await open(page, contextualStory);
    const paintOf = async (id: string): Promise<{ background: string; borderColor: string }> => {
      await page.evaluate((tabId) => {
        const ribbon = document.querySelector('mjx-ribbon');
        ribbon?.setAttribute('selected', tabId);
      }, id);
      await settlePaint(page);
      return page.evaluate((tabId) => {
        const ribbon = document.querySelector('mjx-ribbon');
        const tab = ribbon?.shadowRoot?.querySelector(`.tab[data-tab="${tabId}"]`);
        if (tab === null || tab === undefined) throw new Error(`no tab ${tabId}`);
        const style = getComputedStyle(tab);
        return { background: style.backgroundColor, borderColor: style.borderTopColor };
      }, id);
    };

    const core = await paintOf('home');
    expect(core.background).toBe(expectedColor(tabTones.core.selectedBackground, 'light'));
    expect(core.borderColor).toBe(expectedColor(tabTones.core.selectedBorder, 'light'));

    const contextual = await paintOf('picture-format');
    expect(contextual.background).toBe(
      expectedColor(tabTones.contextual.selectedBackground, 'light'),
    );
    expect(contextual.borderColor).toBe(expectedColor(tabTones.contextual.selectedBorder, 'light'));
    // …and therefore also different, which is the weaker claim the stronger one implies.
    expect(contextual.background).not.toBe(core.background);
  });

  test('the story’s own affordance makes a set appear, and announces it', async ({ page }) => {
    await open(page, contextualStory);
    const selectedBefore = await page.evaluate(
      () => document.querySelector('mjx-ribbon')?.getAttribute('selected') ?? '',
    );

    await page.locator('button[data-action="select-table"]').click({ force: true });
    await settle(page);

    expect(
      await page.evaluate(
        () =>
          document.querySelector('mjx-ribbon')?.shadowRoot?.querySelector('.announcer')
            ?.textContent ?? '',
      ),
    ).toBe('Table Tools tab set available');

    expect(
      await page.evaluate(() => document.querySelector('mjx-ribbon')?.getAttribute('selected')),
      'a tab set appearing changed the selected tab. Nothing about selecting a table means the ' +
        'person wanted to leave the tab they were on.',
    ).toBe(selectedBefore);

    const ids = await page.evaluate(() => {
      const ribbon = document.querySelector('mjx-ribbon');
      return [...(ribbon?.shadowRoot?.querySelectorAll('.tab') ?? [])].map(
        (tab) => tab.getAttribute('data-tab') ?? '',
      );
    });
    expect(ids).toContain('table-design');
    expect(ids).toContain('table-layout');
  });

  test('an appearing set does not move the keyboard', async ({ page }) => {
    // ⚠ The story's buttons cannot prove this, because *clicking one of them* moves focus to the
    // button — the assertion would then be about where the click put the keyboard rather than
    // about what the appearance did with it. So the set is appended the way a selection change
    // would append it, with the keyboard held on a tab the whole time, and the assertion is an
    // identity claim about the focused element rather than a claim about its role.
    await open(page, contextualStory);
    await page.evaluate(() => {
      const ribbon = document.querySelector('mjx-ribbon');
      const tab = ribbon?.shadowRoot?.querySelector('.tab[aria-selected="true"]');
      if (tab instanceof HTMLElement) tab.focus();
    });
    const before = await focused(page);
    expect(before.role).toBe('tab');
    expect(before.tabId).toBe('home');

    await page.evaluate(() => {
      const ribbon = document.querySelector('mjx-ribbon');
      const set = document.createElement('mjx-contextual-tab-set');
      set.setAttribute('label', 'SmartArt Tools');
      const panel = document.createElement('mjx-ribbon-tab');
      panel.setAttribute('tab-id', 'smartart-design');
      panel.setAttribute('label', 'Design');
      const group = document.createElement('mjx-ribbon-group');
      group.setAttribute('label', 'Layouts');
      const command = document.createElement('mjx-button');
      command.setAttribute('label', 'Change Layout');
      command.setAttribute('icon', 'slide-layout');
      group.append(command);
      panel.append(group);
      set.append(panel);
      ribbon?.append(set);
    });
    await settle(page);

    const after = await focused(page);
    expect(
      after.tabId,
      'a tab set appeared and the keyboard moved. Selecting an object must never take the ' +
        'keyboard away from what a person was doing with it.',
    ).toBe(before.tabId);
    expect(after.role).toBe('tab');
    expect(after.tabIndex).toBe(0);
    expect(
      await page.evaluate(
        () =>
          document.querySelector('mjx-ribbon')?.shadowRoot?.querySelector('.announcer')
            ?.textContent ?? '',
      ),
    ).toBe('SmartArt Tools tab set available');
  });

  test('a disappearing set moves selection back to the last core tab and does not lose focus', async ({
    page,
  }) => {
    await open(page, contextualStory);
    await page.locator('button[data-action="select-table"]').click({ force: true });
    await settle(page);

    // Be on a core tab first, so "the last core tab" is a real memory rather than a default.
    await page.evaluate(() => {
      document.querySelector('mjx-ribbon')?.setAttribute('selected', 'review');
    });
    await settle(page);
    await page.evaluate(() => {
      document.querySelector('mjx-ribbon')?.setAttribute('selected', 'table-layout');
    });
    await settle(page);
    await page.evaluate(() => {
      const ribbon = document.querySelector('mjx-ribbon');
      const tab = ribbon?.shadowRoot?.querySelector('.tab[aria-selected="true"]');
      if (tab instanceof HTMLElement) tab.focus();
    });
    expect((await focused(page)).role).toBe('tab');

    await page.evaluate(() => {
      document.querySelector('mjx-contextual-tab-set[label="Table Tools"]')?.remove();
    });
    await settle(page);

    expect(
      await page.evaluate(() => document.querySelector('mjx-ribbon')?.getAttribute('selected')),
      'the selected tab was removed with its set and selection did not go back to the last core ' +
        'tab that was chosen.',
    ).toBe('review');
    expect(
      (await focused(page)).role,
      'the focused tab was removed and focus was left nowhere at all, so the next Tab starts ' +
        'again from the top of the page.',
    ).toBe('tab');
    expect(
      await page.evaluate(
        () =>
          document.querySelector('mjx-ribbon')?.shadowRoot?.querySelector('.announcer')
            ?.textContent ?? '',
      ),
    ).toBe('Table Tools tab set closed');
  });
});

test.describe('the ribbon’s three states', () => {
  test('show exactly what the model says each one shows', async ({ page }) => {
    await open(page, statesStory);
    const readings = await page.evaluate(() =>
      [...document.querySelectorAll('mjx-ribbon')].map((ribbon) => {
        const root = ribbon.shadowRoot;
        const display = (selector: string): string => {
          const element = root?.querySelector(selector);
          return element === null || element === undefined ? 'missing' : getComputedStyle(element).display;
        };
        return {
          state: ribbon.getAttribute('state') ?? 'expanded',
          strip: display('.strip'),
          body: display('.body'),
          restore: display('.restore'),
        };
      }),
    );
    expect(readings).toHaveLength(3);
    for (const reading of readings) {
      const spec = ribbonStates[reading.state as keyof typeof ribbonStates];
      expect(reading.strip === 'none', `${reading.state}: the tab strip`).toBe(!spec.stripVisible);
      expect(reading.body === 'none', `${reading.state}: the commands`).toBe(!spec.bodyVisible);
      expect(reading.restore === 'none', `${reading.state}: the way back`).toBe(
        !spec.restoreVisible,
      );
    }
  });
});

// ── where a survivor draws ───────────────────────────────────────────────────

interface PlacedRect {
  x: number;
  y: number;
  right: number;
  bottom: number;
  drawn: boolean;
}

interface GroupPlacement {
  label: string;
  presentation: string;
  /** Each declared command's label, in declared order. */
  declared: string[];
  /** Declared indices of the commands that declare `slot="essential"`. */
  essential: number[];
  /** Declared indices of what the survivor slot really holds, in its order. */
  survivors: number[];
  /** Declared indices of what the panel slot really holds, in its order. */
  panel: number[];
  rects: PlacedRect[];
  trigger: PlacedRect;
}

/**
 * Where every command of every selected group really is: which slot holds it, in which order, and
 * where it was drawn.
 *
 * ⚠ The slots are read out of the shadow root and the indices out of the light DOM, so a group that
 * assigned a command to no slot at all would show up as a declared index missing from both lists —
 * which the equalities below turn into a failure naming the group.
 */
const readPlacement = `() => {
  const groups = [...document.querySelectorAll('mjx-ribbon-group')].filter(
    (group) => group.closest('mjx-ribbon-tab[selected]') !== null,
  );
  const rectOf = (element) => {
    const rect = element.getBoundingClientRect();
    return { x: rect.left, y: rect.top, right: rect.right, bottom: rect.bottom, drawn: rect.width > 0 && rect.height > 0 };
  };
  return groups.map((group) => {
    const root = group.shadowRoot;
    const survivorSlot = root.querySelector('.essential slot');
    const panelSlot = root.querySelector('.commands slot');
    const trigger = root.querySelector('.trigger');
    const box = root.querySelector('.group');
    if (!survivorSlot || !panelSlot || !trigger || !box) throw new Error('a group lost a slot, its trigger or its box');
    const declared = [...group.children].filter((child) => {
      const slot = child.getAttribute('slot');
      return slot === null || slot === '' || slot === 'essential';
    });
    return {
      label: group.getAttribute('label') ?? '',
      presentation: getComputedStyle(box).getPropertyValue('${groupPresentationProperty}').trim(),
      declared: declared.map((child) => child.getAttribute('label') ?? child.tagName.toLowerCase()),
      essential: declared.flatMap((child, index) => (child.getAttribute('slot') === 'essential' ? [index] : [])),
      survivors: survivorSlot.assignedElements().map((element) => declared.indexOf(element)),
      panel: panelSlot.assignedElements().map((element) => declared.indexOf(element)),
      rects: declared.map(rectOf),
      trigger: rectOf(trigger),
    };
  });
}`;

async function readPlacements(page: Page): Promise<GroupPlacement[]> {
  return page.evaluate(
    (source) => (new Function(`return ${source}`)() as () => unknown)() as never,
    readPlacement,
  );
}

/**
 * Which declared command of the named group holds focus, or -1 when focus is anywhere else —
 * including the group's own trigger and dialog launcher.
 *
 * Walks up from the deepest focused element through every shadow host, because a command's focus
 * stop is inside its own shadow root and a picker's may be two roots deep.
 */
const focusedCommandIndex = `(label) => {
  let element = document.activeElement;
  while (element && element.shadowRoot && element.shadowRoot.activeElement) {
    element = element.shadowRoot.activeElement;
  }
  const group = [...document.querySelectorAll('mjx-ribbon-tab[selected] mjx-ribbon-group')].find(
    (candidate) => candidate.getAttribute('label') === label,
  );
  if (!group) throw new Error('no selected group labelled ' + label);
  const declared = [...group.children].filter((child) => {
    const slot = child.getAttribute('slot');
    return slot === null || slot === '' || slot === 'essential';
  });
  let node = element;
  while (node) {
    if (node.parentElement === group) return declared.indexOf(node);
    if (node.parentElement) {
      node = node.parentElement;
      continue;
    }
    const root = node.getRootNode();
    node = root instanceof ShadowRoot ? root.host : null;
  }
  return -1;
}`;

async function focusedIndex(page: Page, label: string): Promise<number> {
  return page.evaluate(
    ({ source, group }) => (new Function(`return ${source}`)() as (value: string) => number)(group),
    { source: focusedCommandIndex, group: label },
  );
}

/**
 * Press Tab until focus leaves the group's commands, and return the commands it visited, each once.
 *
 * Consecutive repeats are collapsed because a picker or a split button is more than one tab stop and
 * is still one command; a command visited, left and visited again is **not** collapsed, and would
 * fail the equality as it should.
 */
async function tabWalk(page: Page, label: string, limit: number): Promise<number[]> {
  const visited: number[] = [];
  for (let step = 0; step < limit; step += 1) {
    await page.keyboard.press('Tab');
    const index = await focusedIndex(page, label);
    if (index === -1) break;
    if (visited[visited.length - 1] !== index) visited.push(index);
  }
  return visited;
}

/** Whether `later` reads after `earlier` in a column-flow grid: further across, or lower in the same column. */
function readsAfter(earlier: PlacedRect, later: PlacedRect): boolean {
  if (later.x > earlier.x + 1) return true;
  return Math.abs(later.x - earlier.x) <= 1 && later.y > earlier.y;
}

const range = (length: number): number[] => Array.from({ length }, (_, index) => index);

test.describe('where a survivor draws — declared order, not a blanket rule', () => {
  test('the fixture is DISCRIMINATING: a survivor has a command declared on each side of it', async ({
    page,
  }) => {
    // Without this, every order assertion below could pass over groups whose survivors happen to be
    // declared first or last — exactly where a blanket rule and the declared order agree.
    await open(page, wordHomeStory);
    await setContainerWidth(page, 1440);
    const placements = await readPlacements(page);
    const interleaved = placements.filter((group) => {
      const first = group.essential[0];
      const last = group.essential[group.essential.length - 1];
      return first !== undefined && last !== undefined && first > 0 && last < group.declared.length - 1;
    });
    expect(interleaved.map((group) => group.label)).toEqual(
      expect.arrayContaining(['Font', 'Paragraph']),
    );
  });

  for (const story of [wordHomeStory, worstCaseStory]) {
    test(`${story.title} · ${story.name}: expanded, every command draws in declared order and the survivor row is empty`, async ({
      page,
    }) => {
      await open(page, story);
      await setContainerWidth(page, 1440);
      const placements = await readPlacements(page);
      expect(placements.length).toBeGreaterThan(3);
      for (const group of placements) {
        expect(group.presentation, `${group.label} at 1440px`).not.toBe('collapsed');
        expect(group.survivors, `${group.label} has a survivor row while expanded`).toEqual([]);
        expect(
          group.panel,
          `${group.label} presents its commands out of declared order (${group.declared.join(', ')})`,
        ).toEqual(range(group.declared.length));
        // Each drawn command against the previous DRAWN one, not the adjacent one: comparing adjacent
        // pairs and skipping any pair with an undrawn member let a single undrawn command hide an
        // inversion across it. And a group that draws fewer than two of its commands would compare
        // nothing at all, so it fails here rather than passing quietly.
        const drawn = group.rects.flatMap((rect, index) => (rect.drawn ? [{ rect, index }] : []));
        if (group.declared.length >= 2) {
          expect(
            drawn.length,
            `${group.label} draws ${String(drawn.length)} of its ${String(group.declared.length)} ` +
              'commands while expanded, so its order cannot be compared',
          ).toBeGreaterThanOrEqual(2);
        }
        for (let position = 1; position < drawn.length; position += 1) {
          const earlier = drawn[position - 1];
          const later = drawn[position];
          if (earlier === undefined || later === undefined) continue;
          expect(
            readsAfter(earlier.rect, later.rect),
            `${group.label}: ${group.declared[later.index] ?? ''} draws before ` +
              `${group.declared[earlier.index] ?? ''}`,
          ).toBe(true);
        }
      }
    });

    test(`${story.title} · ${story.name}: collapsed, the row holds exactly the declared survivors and the popup the rest`, async ({
      page,
    }) => {
      await open(page, story);
      await setContainerWidth(page, 390);
      const placements = await readPlacements(page);
      expect(placements.some((group) => group.essential.length > 0)).toBe(true);
      for (const group of placements) {
        expect(group.presentation, `${group.label} at 390px`).toBe('collapsed');
        expect(group.survivors, `${group.label}'s survivor row`).toEqual(group.essential);
        expect(group.panel, `${group.label}'s popup`).toEqual(
          range(group.declared.length).filter((index) => !group.essential.includes(index)),
        );
        let previous = group.trigger;
        for (const index of group.survivors) {
          const rect = group.rects[index];
          expect(rect?.drawn, `${group.label}: survivor ${group.declared[index] ?? ''} is not drawn`).toBe(true);
          if (rect === undefined) continue;
          // Beside the trigger, on its row, and after the one before it.
          expect(rect.x, `${group.label}: ${group.declared[index] ?? ''} is not after its predecessor`).toBeGreaterThanOrEqual(previous.right - 1);
          expect(rect.y < group.trigger.bottom && rect.bottom > group.trigger.y).toBe(true);
          previous = rect;
        }
        for (const index of group.panel) {
          expect(
            group.rects[index]?.drawn,
            `${group.label}: ${group.declared[index] ?? ''} is drawn although the popup is closed`,
          ).toBe(false);
        }
      }

      // Opened, the popup draws every other command and the survivors do not move into it.
      await page.evaluate(() => {
        for (const group of document.querySelectorAll('mjx-ribbon-tab[selected] mjx-ribbon-group')) {
          group.setAttribute('open', '');
        }
      });
      await settle(page);
      for (const group of await readPlacements(page)) {
        expect(group.survivors, `${group.label}'s survivors moved when it opened`).toEqual(group.essential);
        for (const index of [...group.survivors, ...group.panel]) {
          expect(group.rects[index]?.drawn, `${group.label}: ${group.declared[index] ?? ''} is not drawn open`).toBe(true);
        }
      }
    });
  }

  test('a tab that was hidden, selected by a person at a phone width, arrives already collapsed and correctly slotted', async ({
    page,
  }) => {
    // The path every later unit's audit walks: thirty-seven more tabs, switched at 390px. File's
    // groups were connected inside a `display: none` tab panel while the ribbon was wide, so the
    // only thing that can put their survivors in the row is the probe reporting when the tab
    // becomes rendered. Selecting by clicking — not by setting `selected` — so the picker, the tab
    // button and `selectTab` are all in the path a person takes.
    await open(page, wordHomeStory);
    await setContainerWidth(page, 390);
    expect(
      (await read(page)).stripPresentation,
      'the test clicks the picker, so it must be what the ribbon presents at 390px',
    ).toBe('picker');

    const selectByClicking = async (tabId: string): Promise<void> => {
      await page.locator('mjx-ribbon .strip > .picker').click();
      await settle(page);
      await page.locator(`mjx-ribbon .tab[data-tab="${tabId}"]`).click();
      await settle(page);
      expect(
        await page.evaluate(() => document.querySelector('mjx-ribbon')?.getAttribute('selected')),
        `clicking the ${tabId} tab did not select it`,
      ).toBe(tabId);
    };

    const expectCollapsedAndSlotted = async (where: string): Promise<GroupPlacement[]> => {
      const placements = await readPlacements(page);
      expect(placements.length, `${where}: no selected groups`).toBeGreaterThan(0);
      for (const group of placements) {
        expect(group.presentation, `${where}: ${group.label}`).toBe('collapsed');
        expect(
          group.survivors,
          `${where}: ${group.label}'s survivor slot is not its declared essentials. A tab selected ` +
            'at a narrow width that keeps the placement it was connected with has lost its survivors.',
        ).toEqual(group.essential);
        expect(group.panel, `${where}: ${group.label}'s popup`).toEqual(
          range(group.declared.length).filter((index) => !group.essential.includes(index)),
        );
      }
      return placements;
    };

    await selectByClicking('file');
    const file = await expectCollapsedAndSlotted('File at 390px');
    const save = file.find((group) => group.label === 'Save');
    expect(save, 'File has no Save group').toBeDefined();
    expect(save?.survivors.map((index) => save.declared[index])).toEqual(['AutoSave']);
    expect(
      file.filter((group) => group.label !== 'Save').every((group) => group.survivors.length === 0),
      'a File group other than Save holds a survivor',
    ).toBe(true);

    await selectByClicking('home');
    const home = await expectCollapsedAndSlotted('Home again at 390px');
    for (const label of ['Font', 'Paragraph']) {
      const group = home.find((entry) => entry.label === label);
      expect(group, `Home has no ${label} group`).toBeDefined();
      expect(group?.essential.length, `${label} declares no survivor, so this checks nothing`).toBeGreaterThan(0);
      expect(group?.survivors, `${label} lost its survivors on the way back`).toEqual(group?.essential);
    }
  });

  test('widening an open collapsed group puts every command back in order and closes the popup', async ({
    page,
  }) => {
    await open(page, wordHomeStory);
    await setContainerWidth(page, 390);
    const fontTrigger = page.locator('mjx-ribbon-tab[selected] mjx-ribbon-group[label="Font"] .trigger');
    await fontTrigger.click();
    await settle(page);
    expect(
      await page.evaluate(() =>
        document.querySelector('mjx-ribbon-tab[selected] mjx-ribbon-group[label="Font"]')?.hasAttribute('open'),
      ),
      'clicking the trigger did not open the popup, so nothing below is about an open group',
    ).toBe(true);

    await setContainerWidth(page, 1440);
    await settle(page);
    const wide = (await readPlacements(page)).find((group) => group.label === 'Font');
    expect(wide, 'no Font group at 1440px').toBeDefined();
    if (wide === undefined) return;
    expect(wide.presentation).not.toBe('collapsed');
    expect(wide.survivors, 'Font kept a survivor row after widening').toEqual([]);
    expect(wide.panel, 'Font’s commands are not back in declared order').toEqual(
      range(wide.declared.length),
    );
    for (const [index, rect] of wide.rects.entries()) {
      expect(rect.drawn, `${wide.declared[index] ?? ''} is not drawn after widening`).toBe(true);
    }

    // **The contract, stated where it is checked:** a group whose presentation stops being a popup
    // closes itself — `open` removed, the trigger reporting collapsed — and it does so without
    // moving focus. The part a person would feel is the trap: an open group traps Tab, so a group
    // left open at a desktop width would pull the keyboard back into Font whenever it tried to
    // leave. Shift+Tab from Font's first command must leave Font.
    const state = await page.evaluate(() => {
      const group = document.querySelector('mjx-ribbon-tab[selected] mjx-ribbon-group[label="Font"]');
      const trigger = group?.shadowRoot?.querySelector('.trigger');
      return {
        open: group?.hasAttribute('open') ?? null,
        expanded: trigger?.getAttribute('aria-expanded') ?? null,
        triggerDisplay: trigger instanceof HTMLElement ? getComputedStyle(trigger).display : null,
      };
    });
    expect(state, 'Font is still open at 1440px, where it has no popup to be open').toEqual({
      open: false,
      expanded: 'false',
      triggerDisplay: 'none',
    });
    await page.evaluate(() => {
      const group = document.querySelector('mjx-ribbon-tab[selected] mjx-ribbon-group[label="Font"]');
      const first = group?.children[0];
      if (!(first instanceof HTMLElement)) throw new Error('Font has no first command');
      first.focus();
    });
    expect(await focusedIndex(page, 'Font'), 'focus did not land on Font’s first command').toBe(0);
    await page.keyboard.press('Shift+Tab');
    await settle(page);
    expect(
      await focusedIndex(page, 'Font'),
      'Shift+Tab from Font’s first command stayed inside Font: the trap outlived the popup.',
    ).toBe(-1);

    // And back: the collapsed invariant holds again.
    await setContainerWidth(page, 390);
    await settle(page);
    const narrow = (await readPlacements(page)).find((group) => group.label === 'Font');
    expect(narrow?.presentation).toBe('collapsed');
    expect(narrow?.survivors).toEqual(narrow?.essential);
    expect(narrow?.panel).toEqual(
      range(narrow?.declared.length ?? 0).filter((index) => !(narrow?.essential ?? []).includes(index)),
    );
  });

  test('focus walks the commands in the order they draw, expanded and collapsed', async ({ page }) => {
    await open(page, wordHomeStory);
    const label = 'Paragraph';

    // Expanded: every command, in declared order — the survivors included, where they stand.
    await setContainerWidth(page, 1440);
    const expanded = (await readPlacements(page)).find((group) => group.label === label);
    expect(expanded, `no ${label} group`).toBeDefined();
    if (expanded === undefined) return;
    await page.evaluate((name) => {
      const group = [...document.querySelectorAll('mjx-ribbon-tab[selected] mjx-ribbon-group')].find(
        (candidate) => candidate.getAttribute('label') === name,
      );
      const first = group?.children[0];
      if (!(first instanceof HTMLElement)) throw new Error('no first command');
      first.focus();
    }, label);
    expect(await focusedIndex(page, label), 'focus did not land on the first command').toBe(0);
    const walked = await tabWalk(page, label, expanded.declared.length * 3);
    expect(
      [0, ...walked],
      `Tab visits ${label}'s commands in a different order from the one they draw in. A keyboard ` +
        'and a pointer must meet the same group — WCAG 2.4.3.',
    ).toEqual(range(expanded.declared.length));

    // Collapsed and closed: the trigger, then the survivors in declared order, then out.
    await setContainerWidth(page, 390);
    const collapsed = (await readPlacements(page)).find((group) => group.label === label);
    expect(collapsed?.essential.length).toBeGreaterThan(0);
    await page.evaluate((name) => {
      const group = [...document.querySelectorAll('mjx-ribbon-tab[selected] mjx-ribbon-group')].find(
        (candidate) => candidate.getAttribute('label') === name,
      );
      const trigger = group?.shadowRoot?.querySelector('.trigger');
      if (!(trigger instanceof HTMLElement)) throw new Error('no trigger');
      trigger.focus();
    }, label);
    expect(await tabWalk(page, label, (collapsed?.declared.length ?? 0) * 3)).toEqual(
      collapsed?.essential ?? [],
    );
  });
});

test.describe('the worst case — Word’s TabHome', () => {
  test('renders every control the census counts, and keeps them all at every width', async ({
    page,
  }) => {
    await open(page, worstCaseStory);
    const count = async (): Promise<number> =>
      page.evaluate(
        () =>
          document.querySelectorAll(
            'mjx-ribbon-tab[selected] mjx-button, mjx-ribbon-tab[selected] mjx-toggle-button',
          ).length,
      );

    await setContainerWidth(page, 1440);
    expect(
      await count(),
      'the specimen no longer renders the number of controls the census records.',
    ).toBe(wordTabHomeControlCount);

    for (const width of [834, 390]) {
      await setContainerWidth(page, width);
      expect(await count(), `at ${String(width)}px`).toBe(wordTabHomeControlCount);
    }
  });

  test('degrades group by group, in the order the priorities declare', async ({ page }) => {
    await open(page, worstCaseStory);
    const expectedFor = async (width: number): Promise<Record<string, string>> => {
      await setContainerWidth(page, width);
      const reading = await read(page);
      const found: Record<string, string> = {};
      for (const group of reading.groups) {
        expect(group.presentation, `${group.label} at ${String(width)}px`).toBe(
          groupPresentationAt(group.priority as GroupPriority, reading.containerWidth),
        );
        found[group.label] = group.presentation;
      }
      return found;
    };

    const desktop = await expectedFor(1440);
    expect(desktop['Font']).toBe('full');
    expect(desktop['Paragraph']).toBe('full');
    expect(desktop['Copilot'], 'an ancillary group is never full below a wide desktop').toBe(
      'reduced',
    );

    const tablet = await expectedFor(834);
    expect(tablet['Font']).toBe('reduced');
    // Clipboard is `secondary` and collapses at 800, so at 834 it is still reduced — the ladder's
    // answer, not a rounder one. `Copilot` is `ancillary` and has been a button since 1000.
    expect(tablet['Clipboard']).toBe('reduced');
    expect(tablet['Copilot']).toBe('collapsed');

    const phone = await expectedFor(390);
    expect(new Set(Object.values(phone))).toEqual(new Set(['collapsed']));
  });

  test('keeps every group inside the demotion ceiling, and demotes nothing that opens a popup', async ({
    page,
  }) => {
    await open(page, worstCaseStory);
    const reading = await read(page);
    expect(reading.groups).toHaveLength(wordTabHomeGroups.length);
    for (const group of reading.groups) {
      expect(
        group.essential,
        `${group.label} keeps ${String(group.essential)} commands through a collapse, and the ` +
          `ceiling is ${String(essentialCommandLimit)}. A longer survivor row is a ribbon again.`,
      ).toBeLessThanOrEqual(essentialCommandLimit);
    }

    const popups = await page.evaluate(() =>
      [...document.querySelectorAll('mjx-ribbon-tab[selected] [slot="essential"]')].map(
        (command) => ({
          tag: command.tagName.toLowerCase(),
          label: command.getAttribute('label') ?? '',
          haspopup:
            command.shadowRoot?.querySelector('[aria-haspopup]') !== null &&
            command.shadowRoot?.querySelector('[aria-haspopup]') !== undefined,
        }),
      ),
    );
    expect(popups.length).toBeGreaterThan(0);
    for (const command of popups) {
      expect(
        command.haspopup,
        `${command.label} survives a collapse and opens a popup. A popup inside a popup is where ` +
          'a phone UI dies — rule 1 of the demotion rules.',
      ).toBe(false);
      expect(command.tag).not.toBe('mjx-split-button');
    }
  });
});
