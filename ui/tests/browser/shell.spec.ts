import { expect, test, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { containerPresets } from '../../src/harness/presets.ts';
import {
  absentFromShells,
  minimumDistinctComponents,
  shellSizePreset,
  shellStories,
  shellSurfaceMinimumExtent,
  tagsRequiredInShells,
} from '../../stories/shell/shell-model.ts';
import { catalogueByTag } from '../../src/mobile/touch-audit.ts';

/**
 * **The assembly gate** — MJXOFF-274's conditions three to five, over the rendered nine.
 *
 * `tests/shell.test.ts` holds the *source* half: no ad-hoc substitute for a catalogued component
 * appears in the assembly's own markup. This file holds the three halves a source scan cannot see:
 *
 * | Condition | Test |
 * |---|---|
 * | the nine stories exist and each renders at the width it claims | `renders at its own width` |
 * | no horizontal overflow, no clipped surface, both schemes | the same test |
 * | every catalogued component appears in a shell, or is named absent — **and the absent list is not a lie** | `the catalogue is accounted for` |
 *
 * ## Why the coverage union is one test rather than nine
 *
 * Playwright starts a fresh worker after a failing test, so a module-level accumulator resets at
 * every failure and an aggregate assertion is only complete on the runs where nothing was wrong —
 * *the worst possible property for an assertion about coverage*. MJXOFF-194 met that twice in one
 * afternoon and answered it with a ledger on disk. The cheaper answer, where the whole sweep fits
 * inside one test, is to put it inside one test: nine `openStory` calls in a loop accumulate into a
 * local, and a crash halfway is a failed test rather than a partial answer reported as a whole one.
 *
 * ## Why the width is asserted rather than driven
 *
 * Each shell story declares `globals: { containerPreset }` in its own CSF, so the size is a property
 * of the story rather than of whoever opened it. `openStory` is therefore called **without** a
 * preset here, and the first assertion is that the frame came up at the width the story asked for —
 * which is the only thing that proves the mechanism worked. A test that passed the preset in would
 * have been testing its own URL.
 */

const stories = builtStories();
const storyIds = new Set(stories.map((entry) => entry.id));

/** The shell's own root, and the harness frame its width is measured on. */
const shellSelector = '[data-mjx-shell]';
const frameSelector = 'mjx-resizable-container';

interface Overflow {
  readonly where: string;
  readonly scrollWidth: number;
  readonly clientWidth: number;
  readonly overflowX: string;
}

interface SurfaceBox {
  readonly name: string;
  readonly width: number;
  readonly height: number;
  readonly outsideBy: number;
}

interface ShellReport {
  readonly found: boolean;
  readonly frameWidth: number;
  readonly shellWidth: number;
  readonly overflowing: Overflow[];
  readonly surfaces: SurfaceBox[];
  readonly drawnWhileHidden: string[];
  readonly tags: string[];
}

/**
 * Everything the gate needs from one rendered shell, in one round trip.
 *
 * ⚠ **Two different walks, on purpose.**
 *
 * The **overflow** walk is the shell's own light DOM only. What is inside a component's shadow root
 * is that component's business and has its own suite; what an assembly is answerable for is whether
 * the boxes *it* laid out hold the components it put in them. A walk that descended would have
 * reported a ribbon that deliberately clips its own strip as an assembly defect.
 *
 * The **coverage** walk descends every shadow root, because half this catalogue builds its children
 * rather than slotting them: a review pane creates the three card elements, a ribbon creates its tab
 * buttons, and a button creates its icon. A light-DOM-only coverage scan would have reported four
 * components as never assembled while they were on the screen.
 */
async function inspect(page: Page): Promise<ShellReport> {
  return page.evaluate(
    ({ shellQuery, frameQuery, minimumExtent }) => {
      const shell = document.querySelector(shellQuery);
      const frame = document.querySelector(frameQuery)?.shadowRoot?.querySelector('[part="frame"]');
      if (shell === null) {
        return {
          found: false,
          frameWidth: 0,
          shellWidth: 0,
          overflowing: [],
          surfaces: [],
          drawnWhileHidden: [],
          tags: [],
        };
      }

      const name = (element: Element): string => {
        const id = element.getAttribute('id');
        const surfaceName = element.getAttribute('data-mjx-shell-surface');
        const suffix = id !== null ? `#${id}` : surfaceName !== null ? `[${surfaceName}]` : '';
        return `${element.localName}${suffix}`;
      };

      // ── overflow: the shell's own boxes, and the hosts it put in them ──────
      const overflowing: {
        where: string;
        scrollWidth: number;
        clientWidth: number;
        overflowX: string;
      }[] = [];
      for (const element of [shell, ...Array.from(shell.querySelectorAll('*'))]) {
        const overflowX = getComputedStyle(element).overflowX;
        if (overflowX === 'auto' || overflowX === 'scroll') continue;
        if (element.scrollWidth - element.clientWidth > 1) {
          overflowing.push({
            where: name(element),
            scrollWidth: element.scrollWidth,
            clientWidth: element.clientWidth,
            overflowX,
          });
        }
      }

      // ── the named surfaces: present, sized, and inside the shell ──────────
      const shellBox = shell.getBoundingClientRect();
      const surfaces = Array.from(shell.querySelectorAll('[data-mjx-shell-surface]')).map(
        (element) => {
          const box = element.getBoundingClientRect();
          const outsideBy = Math.max(
            shellBox.left - box.left,
            box.right - shellBox.right,
            shellBox.top - box.top,
            box.bottom - shellBox.bottom,
            0,
          );
          return {
            name: element.getAttribute('data-mjx-shell-surface') ?? '(unnamed)',
            width: box.width,
            height: box.height,
            outsideBy,
          };
        },
      );

      // ── coverage, and anything hidden that is still drawn ──────────────────
      const tags = new Set<string>();
      const drawnWhileHidden = new Set<string>();
      const walk = (root: ParentNode, owner: string): void => {
        for (const element of Array.from(root.querySelectorAll('[hidden]'))) {
          const style = getComputedStyle(element);
          if (style.display === 'none') continue;
          if (element.getBoundingClientRect().width <= 0) continue;
          drawnWhileHidden.add(
            `${owner} → ${element.localName}${element.className === '' ? '' : `.${String(element.className).split(' ')[0] ?? ''}`}` +
              ` is [hidden] and computes display: ${style.display}`,
          );
        }
        for (const element of Array.from(root.querySelectorAll('*'))) {
          tags.add(element.localName);
          const shadow = (element as HTMLElement).shadowRoot;
          if (shadow !== null) walk(shadow, element.localName);
        }
      };
      walk(shell, 'the shell');

      void minimumExtent;
      return {
        found: true,
        frameWidth: frame === null || frame === undefined ? 0 : frame.getBoundingClientRect().width,
        shellWidth: shellBox.width,
        overflowing,
        surfaces,
        drawnWhileHidden: [...drawnWhileHidden].sort(),
        tags: [...tags].sort(),
      };
    },
    {
      shellQuery: shellSelector,
      frameQuery: frameSelector,
      minimumExtent: shellSurfaceMinimumExtent,
    },
  );
}

// ── the nine, in both schemes ────────────────────────────────────────────────

for (const shell of shellStories) {
  for (const theme of ['light', 'dark'] as const) {
    test(`${shell.application} · ${shell.size} · ${theme} renders at its own width, without overflow or a clipped surface`, async ({
      page,
    }) => {
      expect(
        storyIds.has(shell.id),
        `the built catalogue has no story '${shell.id}'. shell-model.ts derives that id from the ` +
          'title and the export name; one of the two has drifted, and every assertion in this file ' +
          'would otherwise have been made about a page that is not the shell.',
      ).toBe(true);

      // ⚠ No `containerPreset` here. The story declares its own, and this is what proves it.
      await openStory(page, shell.id, { theme });
      const report = await inspect(page);

      expect(report.found, `no [data-mjx-shell] root in ${shell.id}`).toBe(true);

      const expected = containerPresets[shellSizePreset[shell.size]];
      expect(
        Math.abs(report.frameWidth - expected),
        `${shell.id} came up ${String(report.frameWidth)} px wide, not ${String(expected)}. The ` +
          'story declares its size with a story-level global; if that mechanism has stopped ' +
          'working, every one of these tests has been measuring the desktop.',
      ).toBeLessThanOrEqual(1);

      expect(
        report.overflowing.map(
          (entry) =>
            `${entry.where} is ${String(entry.scrollWidth)} px of content in a ` +
            `${String(entry.clientWidth)} px box (overflow-x: ${entry.overflowX})`,
        ),
        'horizontal overflow in the assembly. A box that is not a declared scroller and holds ' +
          'more than it is wide is either a component the shell gave too little room or a shell ' +
          'that forgot a min-inline-size: 0.',
      ).toEqual([]);

      // Anti-vacuity: a shell with no named surfaces would pass the two assertions below by
      // having nothing to check, which is this programme's oldest failure mode.
      expect(report.surfaces.length, `${shell.id} declares no named surfaces`).toBeGreaterThan(1);

      expect(
        report.surfaces
          .filter(
            (entry) =>
              entry.width < shellSurfaceMinimumExtent || entry.height < shellSurfaceMinimumExtent,
          )
          .map((entry) => `${entry.name} is ${entry.width.toFixed(1)} x ${entry.height.toFixed(1)}`),
        `a named surface is under the ${String(shellSurfaceMinimumExtent)} px floor on an axis, ` +
          'which means the layout squeezed it out whatever the DOM says.',
      ).toEqual([]);

      expect(
        report.surfaces
          .filter((entry) => entry.outsideBy > 1)
          .map((entry) => `${entry.name} sits ${entry.outsideBy.toFixed(1)} px outside the shell`),
        'a named surface is outside the shell’s own box — clipped. The stage is a fixed height ' +
          'precisely so that this is a failure rather than a scroll: a status bar pushed past the ' +
          'foot of the window is gone, not below the fold.',
      ).toEqual([]);

      /*
       * ⚠ **The assertion this child did not set out to write, and the one that earned its keep.**
       *
       * `ui/README.md` has recorded since MJXOFF-189 that the user agent's `[hidden]` rule loses to
       * any author rule at all — and three sheets written *before* that discovery never restated
       * it. So a measure input drew its invalid warning glyph and its message permanently, a font
       * picker drew its substitution warning permanently, a label drew its hint, and a slider drew
       * an empty tick rail: six elements with `hidden` set, a correct accessibility tree, and every
       * attribute assertion in four component suites passing.
       *
       * None of it was visible in a catalogue, because a small triangle in each of four fields on a
       * page of fields reads as part of the design. It is obvious the moment one field sits in a
       * task pane beside a document, which is what an assembly is for. The fix is one line per
       * sheet; this is what keeps it fixed.
       */
      expect(
        report.drawnWhileHidden,
        'something in the assembly carries the `hidden` attribute and is still on the screen. ' +
          'The user agent’s [hidden] rule is in the user-agent origin, so any author `display` ' +
          'rule beats it: restate `[hidden] { display: none !important; }` last in the component’s ' +
          'sheet. A state indicator that is always on is the same as no indicator at all.',
      ).toEqual([]);
    });
  }
}

// ── coverage, in both directions ─────────────────────────────────────────────

test('the catalogue is accounted for: every component is in a shell, or is named absent', async ({
  page,
}) => {
  const seen = new Set<string>();
  for (const shell of shellStories) {
    await openStory(page, shell.id);
    const report = await inspect(page);
    expect(report.found, `no [data-mjx-shell] root in ${shell.id}`).toBe(true);
    for (const tag of report.tags) seen.add(tag);
  }

  const catalogued = [...seen].filter((tag) => catalogueByTag.has(tag));

  // Anti-vacuity first: everything below is an assertion about a set, and a broken walk would
  // otherwise report an empty one as a clean answer.
  expect(
    catalogued.length,
    'the nine shells between them rendered almost no catalogued components, which cannot be ' +
      'right — the walk is broken rather than the assembly.',
  ).toBeGreaterThanOrEqual(minimumDistinctComponents);

  expect(
    tagsRequiredInShells.filter((tag) => !seen.has(tag)),
    'these catalogued components appear in none of the nine shells. A component that appears in ' +
      'no assembly has never been seen in context: put it in one, or name it in ' +
      '`absentFromShells` with a reason.',
  ).toEqual([]);

  expect(
    Object.keys(absentFromShells).filter((tag) => seen.has(tag)),
    'these components are named as deliberately absent and are in fact present. The reason ' +
      'attached to each is now a lie, which is worse than no list at all — either remove the ' +
      'entry or remove the component from the shell.',
  ).toEqual([]);

  expect(
    [...seen].filter((tag) => tag.startsWith('mjx-') && !catalogueByTag.has(tag)),
    'the assembly rendered an `mjx-` element the catalogue does not know about. Either it is a ' +
      'new component — in which case `catalogueComponents` in src/mobile/touch-audit.ts is where ' +
      'it is declared — or the shell invented one, which is the mock-up this gate exists against.',
  ).toEqual([]);
});
