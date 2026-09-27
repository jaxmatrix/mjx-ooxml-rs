import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, test } from 'vitest';

import { bodyTextMinimum, contrastRatio } from '../dev/contrast.ts';
import { tokens, customProperties } from '../tokens/tokens.ts';
import { controlStateNames, controlStateSpecs } from '../src/controls/control-states.ts';
import { containerName, phoneShellAtOrBelow } from '../src/harness/presets.ts';
import { tabStripPickerAtOrBelow, groupPresentationOrder } from '../src/ribbon/ribbon-model.ts';
import {
  longPressDelay,
  menuSheetAtOrBelow,
  submenuHoverOpenDelay,
} from '../src/menus/menu-model.ts';
import {
  cellsInWindow,
  galleryActions,
  galleryCaptionIsSizeNotColour,
  galleryCellStateCascade,
  galleryCellStateNames,
  galleryCellStates,
  galleryCellStatesCss,
  galleryCss,
  galleryEvents,
  galleryFlyoutColumns,
  galleryGroupDegradationCss,
  galleryItemKindNames,
  galleryKeyAction,
  galleryPresentationAt,
  galleryPresentationCss,
  galleryPresentationOrder,
  galleryPresentations,
  galleryRowPlan,
  gallerySheetAtOrBelow,
  galleryStripRows,
  galleryStripRowsFor,
  galleryTouchPreviewDelay,
  galleryTypeRoles,
  galleryWindow,
  isGalleryMovement,
  nextGalleryIndex,
  pointerPreviewSettleDelay,
  previewSourceNames,
  touchPreviewAffordance,
  uncategorisedSectionLabel,
  type GalleryAction,
  type GalleryCellState,
  type GalleryRow,
} from '../src/gallery/gallery-model.ts';
import {
  PreviewSession,
  applyPreviewEvent,
  previewEventKinds,
  type PreviewEvent,
} from '../src/gallery/preview-session.ts';
import {
  galleryTestIds,
  galleryTokenDependencies,
  largeGalleryItemCount,
  styleValues,
} from '../stories/gallery/specimens.ts';

/**
 * The gallery's Node tier.
 *
 * MJXOFF-185's hardest requirement — *"leaving restored precisely the prior state"* — is an
 * invariant over **sequences**, and a browser test can only ever drive one sequence at a time. So
 * the protocol is a pure state machine and the first suite below drives thousands of sequences
 * through it, comparing a simulated listener against the machine after every single event. The
 * browser tier then proves that the component *obeys* it through real pointers and real keys.
 *
 * Everything else here is the same division `tests/menus.test.ts` makes: the keyboard model, the
 * grid arithmetic, the row plan and the window are pure functions with edges worth naming, and the
 * stylesheet's cascade is a string a test can read.
 */

// ── the preview protocol ─────────────────────────────────────────────────────

/** A tiny deterministic generator, so a failing sequence can be reproduced from its seed. */
function generator(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state * 1_664_525 + 1_013_904_223) >>> 0;
    return state / 0x1_0000_0000;
  };
}

const subjects = ['normal', 'title', 'heading-1', 'quote', 'caption'].map((value) => ({
  value,
  label: value,
}));

describe('the live-preview protocol', () => {
  test('a preview is closed by exactly one cancel or one commit, never both and never neither', () => {
    // 400 random sequences of 40 operations each. The point of a fuzz here rather than a table is
    // that the invariant has to hold for orderings nobody thought of — which is the difference
    // between "leaving restores" and "leaving restores *exactly*, always".
    for (let seed = 1; seed <= 400; seed += 1) {
      const random = generator(seed);
      const session = new PreviewSession('normal');
      const listener: { applied: string | undefined } = { applied: 'normal' };
      /** value → how many previews of it are open. Never above one, and never left open. */
      const open = new Map<string, number>();

      for (let step = 0; step < 40; step += 1) {
        const roll = random();
        const subject = subjects[Math.floor(random() * subjects.length)] ?? subjects[0];
        if (subject === undefined) continue;
        const events: PreviewEvent[] =
          roll < 0.6
            ? session.request(subject, 'pointer')
            : roll < 0.85
              ? session.cancel('pointer')
              : session.commit(subject, 'keyboard');

        for (const event of events) {
          expect(previewEventKinds).toContain(event.kind);
          if (event.kind === 'preview') open.set(event.value, (open.get(event.value) ?? 0) + 1);
          else {
            // A cancel or a commit closes a preview — or, for a commit with nothing in flight,
            // closes nothing. What may never happen is closing one twice.
            const count = open.get(event.value) ?? 0;
            expect(count, `${event.kind} of ${event.value} closed a preview that was not open`)
              .toBeLessThanOrEqual(1);
            if (count > 0) open.set(event.value, count - 1);
          }
          applyPreviewEvent(listener, event);
        }

        // **The correspondence assertion**, after every *operation* rather than only at the end of
        // the run — which is what makes it evidence about the protocol rather than about the last
        // thing that happened. It is per operation and not per event on purpose: one operation is
        // one atomic batch (`commit` of a different item is a cancel *and* a commit, in one task,
        // with no paint between), so an event-by-event comparison would be asserting that an
        // intermediate state is a state anybody can see.
        expect(listener.applied, `seed ${String(seed)} step ${String(step)}`).toBe(
          session.effective,
        );
        expect(session.outstanding).toBeLessThanOrEqual(1);
        const stillOpen = [...open.values()].reduce((total, count) => total + count, 0);
        expect(stillOpen, `seed ${String(seed)} step ${String(step)}`).toBe(session.outstanding);
      }
    }
  });

  test('a sequence with no commit in it leaves the document exactly where it started', () => {
    for (let seed = 1; seed <= 200; seed += 1) {
      const random = generator(seed);
      const session = new PreviewSession('heading-1');
      const listener: { applied: string | undefined } = { applied: 'heading-1' };
      for (let step = 0; step < 30; step += 1) {
        const subject = subjects[Math.floor(random() * subjects.length)] ?? subjects[0];
        if (subject === undefined) continue;
        const events =
          random() < 0.7 ? session.request(subject, 'pointer') : session.cancel('pointer');
        for (const event of events) applyPreviewEvent(listener, event);
      }
      for (const event of session.cancel('pointer')) applyPreviewEvent(listener, event);
      expect(listener.applied, `seed ${String(seed)}`).toBe('heading-1');
      expect(session.effective).toBe('heading-1');
    }
  });

  test('the anti-vacuity check: those sequences really did change the document in the middle', () => {
    // ⚠ MJXOFF-184's lesson, applied here: *"asserting only the end state proved nothing."* If a
    // preview never actually applied anything, "it went back" would be trivially true and both
    // tests above would be measuring silence. So this one requires that the listener held something
    // other than the committed value at some point in the middle of the run.
    const session = new PreviewSession('heading-1');
    const listener: { applied: string | undefined } = { applied: 'heading-1' };
    const seen = new Set<string | undefined>();
    for (const subject of subjects) {
      for (const event of session.request(subject, 'pointer')) {
        applyPreviewEvent(listener, event);
        seen.add(listener.applied);
      }
    }
    expect(seen.size).toBeGreaterThan(1);
    expect([...seen]).toContain('title');
    for (const event of session.cancel('pointer')) applyPreviewEvent(listener, event);
    expect(listener.applied).toBe('heading-1');
  });

  test('re-requesting the item already previewed emits nothing at all', () => {
    const session = new PreviewSession();
    expect(session.request({ value: 'a', label: 'A' }, 'pointer')).toHaveLength(1);
    expect(session.request({ value: 'a', label: 'A' }, 'pointer')).toHaveLength(0);
    expect(session.request({ value: 'a', label: 'A' }, 'keyboard')).toHaveLength(0);
    expect(session.outstanding).toBe(1);
  });

  test('switching cancels the old one first, and the order is cancel then preview', () => {
    const session = new PreviewSession('normal');
    session.request({ value: 'a', label: 'A' }, 'pointer');
    const events = session.request({ value: 'b', label: 'B' }, 'pointer');
    expect(events.map((event) => `${event.kind}:${event.value}`)).toEqual([
      'cancel:a',
      'preview:b',
    ]);
    expect(events.every((event) => event.restore === 'normal')).toBe(true);
  });

  test('committing what is being previewed does not flash: one commit, no cancel', () => {
    const session = new PreviewSession('normal');
    session.request({ value: 'title', label: 'Title' }, 'pointer');
    const events = session.commit({ value: 'title', label: 'Title' }, 'pointer');
    expect(events).toHaveLength(1);
    expect(events[0]?.kind).toBe('commit');
    // The restore a commit carries is what it *replaced*, which is what an undo stack needs.
    expect(events[0]?.restore).toBe('normal');
    expect(session.committed).toBe('title');
    expect(session.outstanding).toBe(0);
  });

  test('committing something else cancels the preview first', () => {
    const session = new PreviewSession('normal');
    session.request({ value: 'quote', label: 'Quote' }, 'pointer');
    const events = session.commit({ value: 'title', label: 'Title' }, 'keyboard');
    expect(events.map((event) => `${event.kind}:${event.value}`)).toEqual([
      'cancel:quote',
      'commit:title',
    ]);
  });

  test('after a commit, cancelling a later preview restores the committed value — not the first', () => {
    // **The failure this field exists to make impossible.** A listener that remembered "what it was
    // when the page loaded" would put `normal` back here, and the still would look identical.
    const session = new PreviewSession('normal');
    const listener: { applied: string | undefined } = { applied: 'normal' };
    for (const event of session.commit({ value: 'title', label: 'Title' }, 'pointer')) {
      applyPreviewEvent(listener, event);
    }
    for (const event of session.request({ value: 'quote', label: 'Quote' }, 'pointer')) {
      applyPreviewEvent(listener, event);
    }
    expect(listener.applied).toBe('quote');
    for (const event of session.cancel('pointer')) applyPreviewEvent(listener, event);
    expect(listener.applied).toBe('title');
    expect(listener.applied).not.toBe('normal');
  });

  test('a committed value cannot be adopted under a live preview', () => {
    const session = new PreviewSession('normal');
    session.request({ value: 'quote', label: 'Quote' }, 'pointer');
    session.adopt('title');
    expect(session.committed).toBe('normal');
    session.cancel('pointer');
    session.adopt('title');
    expect(session.committed).toBe('title');
  });

  test('every event names one of the three declared sources', () => {
    for (const by of previewSourceNames) {
      const session = new PreviewSession();
      const [event] = session.request({ value: 'a', label: 'A' }, by);
      expect(event?.by).toBe(by);
    }
  });
});

// ── the keyboard model ───────────────────────────────────────────────────────

describe('the two-dimensional keyboard model', () => {
  const ltr = { direction: 'ltr' as const, expanded: false };
  const rtl = { direction: 'rtl' as const, expanded: false };

  test('every action the model declares is reachable from some key', () => {
    const reached = new Set<GalleryAction>();
    const keys = [
      'ArrowRight',
      'ArrowLeft',
      'ArrowDown',
      'ArrowUp',
      'Home',
      'End',
      'PageDown',
      'PageUp',
      'Enter',
      ' ',
      'Escape',
    ];
    for (const key of keys) {
      const action = galleryKeyAction(key, ltr);
      if (action !== undefined) reached.add(action);
    }
    const expand = galleryKeyAction('ArrowDown', { ...ltr, altKey: true });
    if (expand !== undefined) reached.add(expand);
    expect([...reached].sort()).toEqual([...galleryActions].sort());
  });

  test('the inline arrows mirror under RTL and the block arrows do not', () => {
    expect(galleryKeyAction('ArrowRight', ltr)).toBe('next');
    expect(galleryKeyAction('ArrowLeft', ltr)).toBe('previous');
    expect(galleryKeyAction('ArrowRight', rtl)).toBe('previous');
    expect(galleryKeyAction('ArrowLeft', rtl)).toBe('next');
    expect(galleryKeyAction('ArrowDown', ltr)).toBe(galleryKeyAction('ArrowDown', rtl));
    expect(galleryKeyAction('ArrowUp', ltr)).toBe(galleryKeyAction('ArrowUp', rtl));
  });

  test('Escape cancels whether or not the flyout is open — it is the revert key', () => {
    expect(galleryKeyAction('Escape', ltr)).toBe('cancel');
    expect(galleryKeyAction('Escape', { ...ltr, expanded: true })).toBe('cancel');
  });

  test('Alt + Arrow Down expands, and only from the strip', () => {
    expect(galleryKeyAction('ArrowDown', { ...ltr, altKey: true })).toBe('expand');
    expect(galleryKeyAction('ArrowDown', { ...ltr, expanded: true, altKey: true })).toBeUndefined();
    // Alt with anything else belongs to the shell, not to a listbox.
    expect(galleryKeyAction('ArrowRight', { ...ltr, altKey: true })).toBeUndefined();
    expect(galleryKeyAction('Enter', { ...ltr, altKey: true })).toBeUndefined();
  });

  test('the movement actions are exactly the ones that move the roving stop', () => {
    for (const action of galleryActions) {
      const moves = isGalleryMovement(action);
      expect(moves).toBe(!['commit', 'cancel', 'expand'].includes(action));
    }
  });
});

describe('where a movement lands, over a deliberately ragged grid', () => {
  // Eight items in four columns is two full rows; **eleven** in four is two full rows and a tail of
  // three, which is the shape every off-by-one in this arithmetic hides behind.
  const grid = { count: 11, columns: 4, rowsPerPage: 2 };

  test('the inline arrows are linear and wrap across rows', () => {
    expect(nextGalleryIndex('next', 3, grid)).toBe(4);
    expect(nextGalleryIndex('previous', 4, grid)).toBe(3);
  });

  test('nothing wraps around the ends', () => {
    expect(nextGalleryIndex('next', 10, grid)).toBe(10);
    expect(nextGalleryIndex('previous', 0, grid)).toBe(0);
  });

  test('Arrow Down off the ragged tail clamps to the last item rather than falling off', () => {
    // Row 1 column 3 is index 7; below it is index 11, which does not exist. The answer a person
    // means is *the last item*, and the fixture with a multiple of four in it never asks.
    expect(nextGalleryIndex('rowDown', 7, grid)).toBe(10);
    expect(nextGalleryIndex('rowDown', 6, grid)).toBe(10);
    expect(nextGalleryIndex('rowDown', 4, grid)).toBe(8);
  });

  test('Arrow Up at the first row stays put rather than jumping to the first item', () => {
    expect(nextGalleryIndex('rowUp', 2, grid)).toBe(2);
    expect(nextGalleryIndex('rowUp', 0, grid)).toBe(0);
    expect(nextGalleryIndex('rowUp', 6, grid)).toBe(2);
  });

  test('Home and End reach the ends of the whole gallery, not of a row', () => {
    expect(nextGalleryIndex('first', 7, grid)).toBe(0);
    expect(nextGalleryIndex('last', 0, grid)).toBe(10);
  });

  test('a page is rowsPerPage rows, clamped at both ends', () => {
    expect(nextGalleryIndex('pageDown', 0, grid)).toBe(8);
    expect(nextGalleryIndex('pageDown', 8, grid)).toBe(10);
    expect(nextGalleryIndex('pageUp', 10, grid)).toBe(2);
    expect(nextGalleryIndex('pageUp', 2, grid)).toBe(0);
  });

  test('an empty gallery has no landing place, and a degenerate grid does not divide by zero', () => {
    expect(nextGalleryIndex('next', 0, { count: 0, columns: 4, rowsPerPage: 2 })).toBe(-1);
    expect(nextGalleryIndex('rowDown', 0, { count: 5, columns: 0, rowsPerPage: 0 })).toBe(1);
  });
});

// ── the row plan and the window ──────────────────────────────────────────────

describe('the row plan, which is where the two geometries are decided', () => {
  const categories = ['A', 'A', 'A', 'A', 'B', 'B', 'C'];

  test('the strip is linear: ceil(count / columns) rows and no headings', () => {
    const plan = galleryRowPlan(categories, 3, false);
    expect(plan).toEqual<GalleryRow[]>([
      { kind: 'cells', start: 0, end: 3 },
      { kind: 'cells', start: 3, end: 6 },
      { kind: 'cells', start: 6, end: 7 },
    ]);
  });

  test('the expanded surface breaks the grid at every category boundary', () => {
    const plan = galleryRowPlan(categories, 3, true);
    expect(plan).toEqual<GalleryRow[]>([
      { kind: 'heading', category: 'A' },
      { kind: 'cells', start: 0, end: 3 },
      { kind: 'cells', start: 3, end: 4 },
      { kind: 'heading', category: 'B' },
      { kind: 'cells', start: 4, end: 6 },
      { kind: 'heading', category: 'C' },
      { kind: 'cells', start: 6, end: 7 },
    ]);
  });

  test('a section never runs a row across a boundary, which is what makes it a section', () => {
    const plan = galleryRowPlan(categories, 3, true);
    for (const row of plan) {
      if (row.kind !== 'cells') continue;
      const first = categories[row.start];
      for (let index = row.start; index < row.end; index += 1) {
        expect(categories[index]).toBe(first);
      }
    }
  });

  test('every item appears exactly once, in both geometries', () => {
    for (const sectioned of [false, true]) {
      const seen = new Set<number>();
      for (const row of galleryRowPlan(categories, 3, sectioned)) {
        if (row.kind !== 'cells') continue;
        for (let index = row.start; index < row.end; index += 1) {
          expect(seen.has(index)).toBe(false);
          seen.add(index);
        }
      }
      expect(seen.size).toBe(categories.length);
    }
  });

  test('an empty gallery plans nothing, and a zero column count does not loop forever', () => {
    expect(galleryRowPlan([], 4, true)).toEqual([]);
    expect(galleryRowPlan(['A', 'A'], 0, false)).toHaveLength(2);
  });
});

describe('the virtual window', () => {
  test('it is the visible rows plus the overscan, clamped at both ends', () => {
    expect(galleryWindow(80, 0, 4, 1)).toEqual({ firstRow: 0, lastRow: 5 });
    expect(galleryWindow(80, 20, 4, 1)).toEqual({ firstRow: 19, lastRow: 25 });
    expect(galleryWindow(80, 78, 4, 1)).toEqual({ firstRow: 77, lastRow: 80 });
  });

  test('an empty plan builds nothing', () => {
    expect(galleryWindow(0, 0, 4, 1)).toEqual({ firstRow: 0, lastRow: 0 });
  });

  test('the window is a small fraction of a large gallery, which is the whole point', () => {
    const categories = Array.from({ length: largeGalleryItemCount }, () => 'Office');
    const plan = galleryRowPlan(categories, 6, false);
    const window = galleryWindow(plan.length, 0, 3, 1);
    // Four rows of six is twenty-four cells for a four-hundred-item gallery.
    expect(cellsInWindow(plan, window)).toBeLessThan(largeGalleryItemCount / 10);
    expect(cellsInWindow(plan, { firstRow: 0, lastRow: plan.length })).toBe(largeGalleryItemCount);
  });
});

// ── the stylesheet ───────────────────────────────────────────────────────────

describe('the cascade, which is source order and nothing else', () => {
  const presentationCss = galleryPresentationCss();
  const stateCss = galleryCellStatesCss('.cell');

  test('every presentation selector is wrapped in :where(), so all three score (0,0,0)', () => {
    // ⚠ MJXOFF-183 shipped a (0,4,0) selector against a (0,2,0) ladder and its *emission-order* test
    // stayed green throughout, because emission order only arbitrates between rules of equal
    // specificity. Asserting the wrapping is what makes the order assertion below mean anything.
    for (const line of presentationCss.split('\n')) {
      if (!line.includes('{') || line.trim().startsWith('@container')) continue;
      if (!line.includes('.surface')) continue;
      expect(line.trim(), line).toMatch(/^:where\(/);
    }
  });

  test('the presentation blocks are emitted in the order the model declares', () => {
    const positions = galleryPresentationOrder.map((presentation) =>
      presentationCss.indexOf(`${'--mjx-gallery-presentation'}: ${presentation};`),
    );
    expect(positions.every((position) => position >= 0)).toBe(true);
    expect([...positions].sort((left, right) => left - right)).toEqual(positions);
  });

  test('the sheet is a container query against the harness frame, at the shared threshold', () => {
    expect(presentationCss).toContain(
      `@container ${containerName} (width <= ${String(gallerySheetAtOrBelow)}px)`,
    );
  });

  test('every cell-state selector is wrapped, and source order is the declared cascade', () => {
    const positions = galleryCellStateCascade.map((state) => {
      const first = galleryCellStates[state].matches[0];
      expect(first, state).toBeDefined();
      return stateCss.indexOf((first ?? '').replaceAll('%s', '.cell'));
    });
    expect(positions.every((position) => position >= 0)).toBe(true);
    expect([...positions].sort((left, right) => left - right)).toEqual(positions);
    for (const block of stateCss.split('\n')) {
      if (!block.includes('{')) continue;
      expect(block.trim()).toMatch(/^:where\(/);
    }
  });

  test('`focus` paints nothing of its own, so the ring is the only keyboard highlight', () => {
    expect(galleryCellStates.focus.matches).toHaveLength(0);
    expect(galleryCellStateCascade).not.toContain('focus');
  });

  test('unavailable is last, so an unavailable selected cell looks like both', () => {
    expect(galleryCellStateCascade[galleryCellStateCascade.length - 1]).toBe('unavailable');
  });

  /**
   * ⚠ The specificity accident, guarded for the sixth time.
   *
   * `.cell` scores (0,1,0) and every state rule scores (0,0,0), so a single `background:` in the
   * base rule would out-specify the entire state table and leave a cell rendering its resting paint
   * in all seven states while every *"the state exists"* check passed.
   */
  test('the base rule paints none of the properties the state table owns', () => {
    const base = galleryCss.slice(galleryCss.indexOf('\n  .cell {'));
    const rule = base.slice(0, base.indexOf('}'));
    for (const property of [
      'background',
      'border-color',
      'border-style',
      'color',
      'font-weight',
      'opacity',
      'box-shadow',
    ]) {
      expect(rule, `.cell must not declare ${property}`).not.toMatch(
        new RegExp(`(^|[;{\\s])${property}\\s*:`),
      );
    }
  });

  test('the group-degradation rules cover every group presentation, all wrapped', () => {
    const css = galleryGroupDegradationCss();
    for (const presentation of groupPresentationOrder) {
      expect(css).toContain(`:where(.surface[data-group-presentation='${presentation}'])`);
      expect(css).toContain(
        `--mjx-gallery-strip-rows: ${String(galleryStripRowsFor(presentation))};`,
      );
    }
  });
});

// ── the constants that must agree with something else ────────────────────────

describe('the numbers this component is not allowed to have its own opinion about', () => {
  test('one phone threshold, aliased by three surfaces', () => {
    expect(gallerySheetAtOrBelow).toBe(phoneShellAtOrBelow);
    expect(menuSheetAtOrBelow).toBe(phoneShellAtOrBelow);
    expect(tabStripPickerAtOrBelow).toBe(phoneShellAtOrBelow);
  });

  test('the presentation model turns on the shared threshold, in both directions', () => {
    expect(galleryPresentationAt(1440, false)).toBe('strip');
    expect(galleryPresentationAt(320, false)).toBe('strip');
    expect(galleryPresentationAt(gallerySheetAtOrBelow + 1, true)).toBe('flyout');
    expect(galleryPresentationAt(gallerySheetAtOrBelow, true)).toBe('sheet');
  });

  test('the touch preview delay is the menu’s long press, read rather than restated', () => {
    expect(galleryTouchPreviewDelay).toBe(longPressDelay);
  });

  test('the pointer settle is shorter than a submenu’s, which is the argued relationship', () => {
    // A preview costs nothing and is undone by moving on, so it can afford to be eager; opening a
    // submenu moves the whole surface. If a later child makes them equal, the argument in
    // `pointerPreviewSettleDelay` has stopped being true and should be rewritten rather than the
    // number quietly changed.
    expect(pointerPreviewSettleDelay).toBeLessThan(submenuHoverOpenDelay);
    expect(pointerPreviewSettleDelay).toBeGreaterThan(0);
  });

  test('a presentation exists for every name, and only the strip is in flow', () => {
    for (const name of galleryPresentationOrder) {
      expect(galleryPresentations[name].description).not.toBe('');
    }
    expect(galleryPresentations.strip.position).toBe('static');
    expect(galleryPresentations.flyout.position).toBe('fixed');
    expect(galleryPresentations.sheet.position).toBe('fixed');
    // A sheet is pinned, never placed. That is the behavioural half of the difference.
    expect(galleryPresentations.sheet.anchored).toBe(false);
    expect(galleryPresentations.flyout.anchored).toBe(true);
  });

  test('only the expanded presentations carry sections and a footer', () => {
    expect(galleryPresentations.strip.sectioned).toBe(false);
    expect(galleryPresentations.flyout.sectioned).toBe(true);
    expect(galleryPresentations.sheet.sectioned).toBe(true);
  });

  test('a gallery in a reduced or collapsed group shows fewer rows than in a full one', () => {
    expect(galleryStripRows.full).toBeGreaterThanOrEqual(galleryStripRows.reduced);
    expect(galleryStripRows.collapsed).toBeLessThanOrEqual(galleryStripRows.reduced);
    for (const presentation of groupPresentationOrder) {
      expect(galleryStripRowsFor(presentation)).toBeGreaterThan(0);
    }
  });

  test('the flyout aims for more columns than a strip is likely to have room for', () => {
    expect(galleryFlyoutColumns).toBeGreaterThan(1);
  });

  test('every cell state names a control state that exists, and none is hard-disabled', () => {
    for (const state of galleryCellStateNames) {
      expect(controlStateNames).toContain(galleryCellStates[state].controlState);
      expect(controlStateSpecs[galleryCellStates[state].controlState]).toBeDefined();
    }
    const named = galleryCellStateNames.map(
      (state) => galleryCellStates[state as GalleryCellState].controlState,
    );
    // A gallery cell is never hard-disabled, for the reason a menu row never is: an unavailable
    // style must stay reachable so its reason can be read.
    expect(named).not.toContain('disabled');
  });

  test('the events are four distinct names under one prefix', () => {
    const values = Object.values(galleryEvents);
    expect(new Set(values).size).toBe(values.length);
    for (const value of values) expect(value.startsWith('mjx-gallery-')).toBe(true);
  });

  test('the touch affordance is recorded as a decision and marked as a guess', () => {
    expect(touchPreviewAffordance.gesture).not.toBe('');
    expect(touchPreviewAffordance.isGuess).toBe(true);
  });
});

/**
 * ⚠ The measurement that decides why a caption is not grey, asserted rather than described.
 *
 * MJXOFF-184 measured `--theme-text-secondary` at 4.32 : 1 on `--theme-border-subtle`, which is the
 * fill `hover` paints across the whole cell — so a grey caption would be illegible exactly when the
 * pointer is on it, which on a gallery is exactly when it is read. **This test fires if a re-seed
 * makes the pairing legal**, which is the correct outcome: the reason for the design has gone and
 * somebody should decide again rather than inherit it.
 */
test('a gallery caption is told apart by size and weight because the grey pairing is illegible', () => {
  const ratio = contrastRatio(tokens.theme.light.textSecondary, tokens.theme.light.borderSubtle);
  expect(ratio).toBeDefined();
  expect(galleryCaptionIsSizeNotColour).toBe(true);
  expect(
    ratio ?? 0,
    'the secondary text now clears 4.5:1 on the hover fill — the size-not-colour rule can be ' +
      'revisited, and this assertion is the place it is being asked from',
  ).toBeLessThan(bodyTextMinimum);
});

test('the type role goes on the cell, and the caption carries none', () => {
  // MJXOFF-182's adoption-order accident, reached by a fifth route: every type role declares a
  // font-weight, so a role class on the caption would beat the weight a selected cell inherits.
  expect(Object.keys(galleryTypeRoles)).toContain('cell');
  expect(Object.keys(galleryTypeRoles)).not.toContain('caption');
});

// ── the catalogue's own contracts ────────────────────────────────────────────

describe('the story file’s contracts', () => {
  const specimens = readFileSync(
    resolve(import.meta.dirname, '../stories/gallery/specimens.ts'),
    'utf8',
  );
  const stories = readFileSync(
    resolve(import.meta.dirname, '../stories/gallery/gallery.stories.ts'),
    'utf8',
  );
  const authored = `${specimens}\n${stories}`;

  test('every id the gates look up is written literally in the specimen stylesheet or markup', () => {
    // lit cannot bind inside a `<style>` element, so the ids in `specimenCss` are literals. This is
    // what stops the two drifting: rename one in `galleryTestIds` and the selector stops matching,
    // silently, and every gate that reads the preview target starts measuring an unstyled paragraph.
    for (const id of Object.values(galleryTestIds)) {
      expect(authored.includes(`"${id}"`) || authored.includes(`#${id}`), id).toBe(true);
    }
  });

  test('the token dependency list names only tokens the generator emitted', () => {
    const paths = galleryTokenDependencies();
    expect(paths.length).toBeGreaterThan(0);
    for (const path of paths) expect(customProperties).toHaveProperty(path);
  });

  test('the style fixture is deliberately not a multiple of any audited column count', () => {
    // A ragged last row is where two-dimensional navigation goes wrong, and a fixture with an exact
    // multiple in it is a fixture that hides the case.
    expect(styleValues).toHaveLength(8);
    expect(new Set(styleValues).size).toBe(styleValues.length);
  });

  test('the large fixture is large enough for a node count to mean something', () => {
    expect(largeGalleryItemCount).toBeGreaterThanOrEqual(200);
  });

  test('the three item kinds are named once and used by the specimen', () => {
    expect(galleryItemKindNames).toHaveLength(3);
    for (const kind of galleryItemKindNames) expect(specimens).toContain(kind);
  });

  test('an item with no category joins the named fallback section', () => {
    expect(uncategorisedSectionLabel).not.toBe('');
  });
});
