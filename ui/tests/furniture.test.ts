import { describe, expect, it } from 'vitest';

import {
  clampToBounds,
  dragFraction,
  keyFraction,
  splitterDefaultBounds,
  splitterDefaultFraction,
  splitterLabel,
  splitterStep,
  splitterValueNow,
} from '../src/foundations/splitter.ts';
import {
  clampFraction,
  fractionFromDrag,
  fractionFromKey,
  taskPaneDefaultFraction,
  taskPaneFractionBounds,
  taskPaneResizeStep,
} from '../src/surfaces/surface-model.ts';
import {
  clampZoom,
  collapsedFraction,
  defaultZoomPercent,
  fitToPagePercent,
  furnitureColor,
  fitToWidthPercent,
  formatZoom,
  letterPagePixels,
  nonTextMinimum,
  parseZoomPercent,
  scrollMarkSpecs,
  scrollbarContrastPairs,
  scrollbarPaints,
  statusBarCss,
  statusOverflowLabel,
  statusPresentationAt,
  statusPresentationProperty,
  statusPriorities,
  statusPriorityNames,
  worstScrollbarContrast,
  zoomBounds,
  zoomFitGutter,
  zoomStepFrom,
  zoomSteps,
} from '../src/furniture/furniture-model.ts';
import {
  ScrollbarModel,
  gripPoint,
  markFraction,
  marksSummary,
  minimumThumbFraction,
  offsetAtThumbStart,
  offsetUnderGrip,
  scrollMarkKindNames,
  scrollable,
  scrollableExtent,
  thumbFraction,
  thumbStart,
  type ScrollMark,
} from '../src/furniture/scroll-model.ts';
import { containerPresets } from '../src/harness/presets.ts';
import { readTypedNumber } from '../src/inputs/measure.ts';
import { contrastRatioOrWorst } from '../src/tokens/contrast.ts';
import type { ColorScheme } from '../tokens/tokens.ts';

/**
 * MJXOFF-190's model, in Node.
 *
 * Everything here has a correct answer, which is the whole reason this child's gates are arithmetic
 * rather than screenshots. **Furniture is where *"it looks fine"* hides best**, because it is chrome
 * nobody stares at: a scrollbar over content that fits behaves perfectly under any implementation
 * at all, a fit-to-width that is off by two per cent looks exactly like one that is right, and a
 * status bar that dropped the wrong segment looks like a status bar.
 *
 * Two assertions in this file are **positive controls** rather than assertions about the component,
 * and both are marked at their site: the naive thumb recomputation, which is asserted to *differ*
 * from the real answer so that the stability assertion beside it is known not to be vacuous; and
 * the lifted-primitive equivalence, which is asserted over a sweep rather than at one value.
 */

const schemes: readonly ColorScheme[] = ['light', 'dark'];

// ── the lifted splitter primitive ────────────────────────────────────────────

describe('the splitter primitive, and the task pane that now binds to it', () => {
  /**
   * ⚠ **The whole justification for the lift, asserted rather than claimed.**
   *
   * MJXOFF-188 wrote this arithmetic under task-pane names; MJXOFF-190 needed it with different
   * bounds and moved it to `src/foundations/splitter.ts`. If the binding drifts from the primitive
   * by so much as a rounding, the task pane and the splitter disagree about what an arrow key does
   * — which is precisely the defect two implementations would have had.
   */
  it('the task pane’s three exports are the primitive, over a sweep', () => {
    for (let value = -0.5; value <= 1.5; value += 0.01) {
      expect(clampFraction(value)).toBe(
        clampToBounds(value, taskPaneFractionBounds, taskPaneDefaultFraction),
      );
    }
    expect(clampFraction(Number.NaN)).toBe(taskPaneDefaultFraction);

    const boundary = { start: 0, size: 1000 };
    for (const side of ['left', 'right'] as const) {
      for (let at = -100; at <= 1100; at += 7) {
        expect(fractionFromDrag(at, boundary, side)).toBe(
          dragFraction(at, boundary, side, taskPaneFractionBounds, taskPaneDefaultFraction),
        );
      }
      for (const key of ['ArrowLeft', 'ArrowRight', 'Home', 'End', 'Escape', 'Tab']) {
        expect(fractionFromKey(key, 0.4, side)).toBe(
          keyFraction(
            key,
            0.4,
            side,
            taskPaneFractionBounds,
            taskPaneResizeStep,
            taskPaneDefaultFraction,
          ),
        );
      }
    }
  });

  it('the step is one number, aliased rather than written twice', () => {
    expect(taskPaneResizeStep).toBe(splitterStep);
  });

  it('the growing arrow depends on the side, and Home and End are the bounds', () => {
    const bounds = splitterDefaultBounds;
    const grown = keyFraction('ArrowRight', 0.4, 'left', bounds, splitterStep, 0.3);
    const shrunk = keyFraction('ArrowLeft', 0.4, 'left', bounds, splitterStep, 0.3);
    expect(grown).toBeCloseTo(0.42, 10);
    expect(shrunk).toBeCloseTo(0.38, 10);
    // Mirrored for a region on the other side, which is the same call and not a second path.
    expect(keyFraction('ArrowLeft', 0.4, 'right', bounds, splitterStep, 0.3)).toBe(grown);
    expect(keyFraction('ArrowRight', 0.4, 'right', bounds, splitterStep, 0.3)).toBe(shrunk);

    expect(keyFraction('Home', 0.4, 'left', bounds, splitterStep, 0.3)).toBe(bounds.min);
    expect(keyFraction('End', 0.4, 'left', bounds, splitterStep, 0.3)).toBe(bounds.max);
    // A key the splitter does not own must not be consumed, or it would eat Tab.
    expect(keyFraction('Tab', 0.4, 'left', bounds, splitterStep, 0.3)).toBeUndefined();
  });

  it('a drag with no boundary falls back rather than producing a NaN fraction', () => {
    // A NaN written into a custom property makes `calc()` fail to parse, so the region takes its
    // CSS width while announcing a number nobody can read.
    expect(dragFraction(500, { start: 0, size: 0 }, 'left', splitterDefaultBounds, 0.25)).toBe(0.25);
    expect(clampToBounds(Number.NaN, splitterDefaultBounds, 0.25)).toBe(0.25);
  });

  it('a splitter announces what it resizes, and its value is a whole percentage', () => {
    expect(splitterLabel('the navigator')).toBe('Resize the navigator');
    expect(splitterLabel('')).toBe('Resize pane');
    expect(splitterValueNow(0.325)).toBe(33);
    expect(splitterDefaultFraction).toBeGreaterThan(splitterDefaultBounds.min);
    expect(splitterDefaultFraction).toBeLessThan(splitterDefaultBounds.max);
    // Collapse is a command and not a fraction: the bounds forbid zero on purpose, so a drag can
    // never lose a region by accident.
    expect(collapsedFraction).toBe(0);
    expect(collapsedFraction).toBeLessThan(splitterDefaultBounds.min);
  });
});

// ── the scroll model, which mirrors R13 ──────────────────────────────────────

/** The specimen document: twenty pages guessed at a thousand each. */
function estimated(): ScrollbarModel {
  return ScrollbarModel.fromExtent({ pages: 20, pageHeight: 1000, precision: 'estimated' });
}

describe('the scroll model — R13’s three assertions, in the chrome’s language', () => {
  /**
   * `crates/mjx-view/tests/scroll_stability.rs` asserts all three at once, and R13 says why: *"the
   * third alone would be green for a model that never corrected anything — which is why the first
   * two are asserted beside it."*
   */
  it('a correction lengthens the document, moves later pages, and leaves the anchor naming the same place', () => {
    const model = estimated();
    const anchor = { page: 12, within: 300 };

    expect(model.totalHeight).toBe(20_000);
    expect(model.offsetOf(12)).toBe(12_000);
    expect(model.offsetOfAnchor(anchor)).toBe(12_300);

    for (let page = 0; page < 10; page += 1) model.recordMeasuredHeight(page, 1400);

    // 1 — the document is longer, so the scrollbar's length is now more nearly right.
    expect(model.totalHeight).toBe(24_000);
    // 2 — pages after the correction really did move.
    expect(model.offsetOf(12)).toBe(16_000);
    // 3 — and the anchor still names the same page, the same distance into it.
    expect(model.anchorAt(model.offsetOfAnchor(anchor))).toEqual(anchor);
    // The anti-vacuity figure R13 exposes for exactly this reason.
    expect(model.measuredPages).toBe(10);
  });

  it('a correction that agrees with the guess still counts as measured', () => {
    const model = estimated();
    model.recordMeasuredHeight(3, 1000);
    expect(model.measuredPages).toBe(1);
    expect(model.totalHeight).toBe(20_000);
  });

  it('an exact page count shrinks or grows the table, and an edit makes the count a guess again', () => {
    const model = estimated();
    model.recordExactPageCount(12);
    expect(model.pageCount).toBe(12);
    expect(model.countPrecision).toBe('exact');
    expect(model.totalHeight).toBe(12_000);

    // R13: an edit that inserts content makes the count a guess again, and a model that kept the
    // old figure would clamp the scrollbar short of content that exists.
    model.reEstimate({ pages: 30, pageHeight: 1000, precision: 'estimated' });
    expect(model.pageCount).toBe(30);
    expect(model.countPrecision).toBe('estimated');

    model.extendToAtLeast(10);
    expect(model.pageCount).toBe(30);
    model.extendToAtLeast(44);
    expect(model.pageCount).toBe(44);
  });

  it('measured heights survive a re-estimate, because a page that did not move did not move', () => {
    const model = estimated();
    model.recordMeasuredHeight(0, 1400);
    model.reEstimate({ pages: 25, pageHeight: 1000, precision: 'estimated' });
    expect(model.measuredPages).toBe(1);
    expect(model.metric(0)?.height).toBe(1400);
  });

  it('an offset past the end and one before the start both land somewhere real', () => {
    const model = estimated();
    expect(model.anchorAt(-5)).toEqual({ page: 0, within: 0 });
    expect(model.anchorAt(0)).toEqual({ page: 0, within: 0 });
    expect(model.anchorAt(19_999)).toEqual({ page: 19, within: 999 });
    expect(model.offsetOf(99)).toBe(model.totalHeight);
  });
});

// ── the thumb, at several ratios ─────────────────────────────────────────────

describe('the thumb’s size, which is the identity-value trap', () => {
  /**
   * ⚠ **A scrollbar over content that fits exercises nothing.** So this sweeps ratios rather than
   * measuring one, including the two ends the ticket names: content that barely overflows, and
   * content that massively does.
   */
  it('is the visible share of the document, at four ratios', () => {
    expect(thumbFraction(800, 800)).toBe(1);
    expect(thumbFraction(800, 900)).toBeCloseTo(8 / 9, 10);
    expect(thumbFraction(800, 4000)).toBeCloseTo(0.2, 10);
    // Sized honestly this would be 0.002 — under a pixel on any real track, and a control nobody
    // can grab.
    expect(thumbFraction(800, 400_000)).toBe(minimumThumbFraction);
  });

  it('the floor starts biting at exactly the ratio the floor names', () => {
    const justAbove = 800 / (minimumThumbFraction + 0.0001);
    const justBelow = 800 / (minimumThumbFraction - 0.0001);
    expect(thumbFraction(800, justAbove)).toBeGreaterThan(minimumThumbFraction);
    expect(thumbFraction(800, justBelow)).toBe(minimumThumbFraction);
  });

  it('content that fits is not scrollable, and its travel is zero', () => {
    expect(scrollable(800, 800)).toBe(false);
    expect(scrollable(800, 801)).toBe(true);
    expect(scrollableExtent(800, 800)).toBe(0);
    expect(thumbStart(500, 800, 800)).toBe(0);
    expect(offsetAtThumbStart(0.5, 800, 800)).toBe(0);
  });

  it('the thumb reaches the end of the track exactly at the end of the document', () => {
    const total = 4000;
    const viewport = 800;
    const size = thumbFraction(viewport, total);
    expect(thumbStart(0, viewport, total)).toBe(0);
    expect(thumbStart(scrollableExtent(viewport, total), viewport, total)).toBeCloseTo(1 - size, 10);
  });

  it('thumbStart and offsetAtThumbStart are each other’s inverse', () => {
    for (const offset of [0, 100, 1600, 2400, 3200]) {
      const start = thumbStart(offset, 800, 4000);
      expect(offsetAtThumbStart(start, 800, 4000)).toBeCloseTo(offset, 6);
    }
  });
});

// ── the trap ─────────────────────────────────────────────────────────────────

describe('the thumb does not jump when the extent changes under the grip', () => {
  const viewport = 4000;
  const pointer = 0.5;
  const grip = 0.5;

  it('the point under the finger is invariant, and the naive answer is not', () => {
    const model = estimated();
    const before = model.totalHeight;
    expect(before).toBe(20_000);

    const offsetBefore = offsetUnderGrip(pointer, grip, viewport, before);
    expect(offsetBefore).toBe(8000);
    const gripBefore = gripPoint(
      thumbStart(offsetBefore, viewport, before),
      grip,
      thumbFraction(viewport, before),
    );
    expect(gripBefore).toBeCloseTo(pointer, 10);

    // The extent is revised mid-drag, which is exactly what R13 does as pages are laid out.
    for (let page = 0; page < 10; page += 1) model.recordMeasuredHeight(page, 1400);
    const after = model.totalHeight;
    expect(after).toBe(24_000);
    // The thumb genuinely changed size — without this the invariant below is about nothing.
    expect(thumbFraction(viewport, after)).not.toBeCloseTo(thumbFraction(viewport, before), 6);

    const offsetAfter = offsetUnderGrip(pointer, grip, viewport, after);
    expect(offsetAfter).toBe(10_000);
    const gripAfter = gripPoint(
      thumbStart(offsetAfter, viewport, after),
      grip,
      thumbFraction(viewport, after),
    );
    expect(gripAfter).toBeCloseTo(pointer, 10);

    // ⚠ THE POSITIVE CONTROL. The naive recomputation — keep the offset, recompute the thumb from
    // it — is what the ticket asks to be proved failable, and this is the proof: had the component
    // done that, the assertion above would have been off by a twelfth of the track. A stability
    // assertion with nothing to be stable *against* is a tolerance nobody has tested.
    const naiveGrip = gripPoint(
      thumbStart(offsetBefore, viewport, after),
      grip,
      thumbFraction(viewport, after),
    );
    expect(Math.abs(naiveGrip - pointer)).toBeGreaterThan(0.05);
    expect(naiveGrip).toBeCloseTo(0.416_666, 4);
  });

  it('the grip is held wherever within the thumb the finger landed', () => {
    const total = 20_000;
    for (const where of [0, 0.25, 0.5, 0.75, 1]) {
      const offset = offsetUnderGrip(0.6, where, viewport, total);
      const point = gripPoint(
        thumbStart(offset, viewport, total),
        where,
        thumbFraction(viewport, total),
      );
      expect(point).toBeCloseTo(0.6, 6);
    }
  });

  it('a grip near an end clamps the thumb inside the track rather than off it', () => {
    const total = 20_000;
    const size = thumbFraction(viewport, total);
    const atStart = offsetUnderGrip(0, 1, viewport, total);
    const atEnd = offsetUnderGrip(1, 0, viewport, total);
    expect(atStart).toBe(0);
    expect(thumbStart(atEnd, viewport, total)).toBeCloseTo(1 - size, 10);
  });
});

// ── the marks ────────────────────────────────────────────────────────────────

describe('the channel of marks', () => {
  const marks: ScrollMark[] = [
    { kind: 'search', page: 4, within: 0.5, label: 'pipeline' },
    { kind: 'comment', page: 0, within: 0, label: 'Ask legal' },
    { kind: 'change', page: 19, within: 1, label: 'Inserted' },
  ];

  it('sits at the document fraction, over the whole track', () => {
    const model = estimated();
    // Page 4 at half its height is 4500 of 20000.
    expect(markFraction(model, marks[0] as ScrollMark)).toBeCloseTo(0.225, 10);
    expect(markFraction(model, marks[1] as ScrollMark)).toBe(0);
    expect(markFraction(model, marks[2] as ScrollMark)).toBe(1);
  });

  it('moves with a correction, because a mark is a place in a document and not a place on a track', () => {
    const model = estimated();
    const before = markFraction(model, marks[0] as ScrollMark);
    for (let page = 0; page < 10; page += 1) model.recordMeasuredHeight(page, 1400);
    // 4 × 1400 + 700 = 6300 of 24000.
    expect(markFraction(model, marks[0] as ScrollMark)).toBeCloseTo(6300 / 24_000, 10);
    expect(markFraction(model, marks[0] as ScrollMark)).not.toBeCloseTo(before, 4);
  });

  /**
   * ⚠ **Never told apart by colour alone.** This palette has two colour families, so three kinds
   * distinguished only by hue would be two a person can tell apart and a third they cannot.
   */
  it('every kind has its own lane as well as its own colour', () => {
    const lanes = new Set(scrollMarkKindNames.map((kind) => scrollMarkSpecs[kind].lane));
    const paints = new Set(scrollMarkKindNames.map((kind) => scrollMarkSpecs[kind].paint));
    expect(lanes.size).toBe(scrollMarkKindNames.length);
    expect(paints.size).toBe(scrollMarkKindNames.length);
  });

  it('is summarised in words a person would use, and counts in the right number', () => {
    expect(marksSummary([])).toBe('No marks');
    expect(marksSummary(marks)).toBe('1 search result, 1 comment, 1 tracked change');
    expect(
      marksSummary([marks[0] as ScrollMark, { ...(marks[0] as ScrollMark), page: 6 }]),
    ).toBe('2 search results');
  });
});

// ── every colour the scrollbar puts on screen ────────────────────────────────

describe('the scrollbar’s contrast, measured rather than remembered', () => {
  /**
   * ⚠ **Measured, not written down.** The palette re-seed that cost MJXOFF-279 a whole child moved
   * figures ten children had recorded in their models. Nothing here records a ratio: the sweep
   * computes them and asserts the floor, and the caption in the catalogue calls the same function.
   */
  it('every pair clears the non-text minimum in both schemes', () => {
    for (const scheme of schemes) {
      for (const pair of scrollbarContrastPairs()) {
        const ratio = contrastRatioOrWorst(
          furnitureColor(scheme, pair.foreground),
          furnitureColor(scheme, pair.background),
        );
        expect(ratio, `${pair.what} in ${scheme}`).toBeGreaterThanOrEqual(nonTextMinimum);
      }
      expect(worstScrollbarContrast(scheme).ratio).toBeGreaterThanOrEqual(nonTextMinimum);
      expect(worstScrollbarContrast(scheme).what).not.toBe('');
    }
  });

  it('the sweep is over something — a track, a thumb and three marks', () => {
    // A helper whose failure mode is "found nothing" makes every ceiling assertion pass, which is
    // U07's finding and the reason this assertion sits beside the one above.
    expect(scrollbarContrastPairs().length).toBe(1 + scrollMarkKindNames.length);
    expect(scrollbarPaints.thumb).not.toBe(scrollbarPaints.track);
  });
});

// ── the zoom control ─────────────────────────────────────────────────────────

describe('the zoom control’s arithmetic', () => {
  it('clamps, rounds and falls back', () => {
    expect(clampZoom(100)).toBe(100);
    expect(clampZoom(7)).toBe(zoomBounds.min);
    expect(clampZoom(700)).toBe(zoomBounds.max);
    expect(clampZoom(100.4)).toBe(100);
    expect(clampZoom(Number.NaN)).toBe(defaultZoomPercent);
  });

  it('the stepping commands move between stops, not by one per cent', () => {
    expect(zoomStepFrom(100, 1)).toBe(125);
    expect(zoomStepFrom(100, -1)).toBe(75);
    // A value between two stops moves to the nearer side, in each direction.
    expect(zoomStepFrom(110, 1)).toBe(125);
    expect(zoomStepFrom(110, -1)).toBe(100);
    // At the ends it answers the bound; the component disables the command, which a person sees.
    expect(zoomStepFrom(zoomBounds.min, -1)).toBe(zoomBounds.min);
    expect(zoomStepFrom(zoomBounds.max, 1)).toBe(zoomBounds.max);
  });

  it('the stops are inside the bounds and in order', () => {
    expect(zoomSteps[0]).toBe(zoomBounds.min);
    expect(zoomSteps[zoomSteps.length - 1]).toBe(zoomBounds.max);
    for (let index = 1; index < zoomSteps.length; index += 1) {
      expect(zoomSteps[index]).toBeGreaterThan(zoomSteps[index - 1] ?? 0);
    }
  });

  it('fit to width is a number, and fit to page is the smaller of the two', () => {
    const viewport = { width: 1000, height: 700 };
    expect(fitToWidthPercent(viewport)).toBe(116);
    expect(fitToPagePercent(viewport)).toBe(61);
    expect(fitToPagePercent(viewport)).toBeLessThanOrEqual(fitToWidthPercent(viewport));
  });

  /**
   * ⚠ **Floored, and the assertion is about the consequence rather than about `Math.floor`.**
   *
   * A fit rounded up overflows the viewport by a fraction of a pixel and produces the horizontal
   * scrollbar the command exists to avoid, so what is asserted is that the page *fits* at the
   * answer and does *not* fit one per cent above it.
   */
  it('a fit never overflows the room it was given, and one per cent more would', () => {
    const viewport = { width: 1000, height: 700 };
    const available = viewport.width - zoomFitGutter * 2;
    const fitted = fitToWidthPercent(viewport);
    expect((fitted / 100) * letterPagePixels.width).toBeLessThanOrEqual(available);
    expect(((fitted + 1) / 100) * letterPagePixels.width).toBeGreaterThan(available);
  });

  it('both fits clamp at both bounds', () => {
    expect(fitToWidthPercent({ width: 100, height: 100 })).toBe(zoomBounds.min);
    expect(fitToPagePercent({ width: 100, height: 100 })).toBe(zoomBounds.min);
    expect(fitToWidthPercent({ width: 10_000, height: 10_000 })).toBe(zoomBounds.max);
    expect(fitToPagePercent({ width: 10_000, height: 10_000 })).toBe(zoomBounds.max);
  });

  it('a page with no size falls back rather than dividing by zero', () => {
    expect(fitToWidthPercent({ width: 1000, height: 700 }, { width: 0, height: 0 })).toBe(
      defaultZoomPercent,
    );
    expect(fitToPagePercent({ width: 1000, height: 700 }, { width: 816, height: 0 })).toBe(
      defaultZoomPercent,
    );
  });

  it('a page is US Letter at 96 dpi, written as arithmetic', () => {
    expect(letterPagePixels.width).toBe(816);
    expect(letterPagePixels.height).toBe(1056);
  });

  it('is written and announced as a whole percentage', () => {
    expect(formatZoom(100)).toBe('100%');
    expect(formatZoom(99.6)).toBe('100%');
  });
});

describe('what the readout accepts, refuses and clamps', () => {
  it('reads a percentage with or without its sign', () => {
    expect(parseZoomPercent('125')).toEqual({ ok: true, percent: 125, clamped: false });
    expect(parseZoomPercent('125%')).toEqual({ ok: true, percent: 125, clamped: false });
    expect(parseZoomPercent(' 125 % ')).toEqual({ ok: true, percent: 125, clamped: false });
  });

  it('accepts both decimal separators, because the grammar has no thousands separator', () => {
    expect(parseZoomPercent('112.5')).toMatchObject({ ok: true, percent: 113 });
    expect(parseZoomPercent('112,5')).toMatchObject({ ok: true, percent: 113 });
    // The one grammar, shared with the measure input rather than written a second time.
    expect(readTypedNumber('112,5')).toEqual({ ok: true, value: { magnitude: 112.5, tail: '' } });
  });

  /** ⚠ U07's rule inherited whole: what it cannot read, it refuses — it does not guess. */
  it('refuses what it cannot read, and says which fragment defeated it', () => {
    expect(parseZoomPercent('')).toEqual({
      ok: false,
      error: { failure: 'empty', offending: '' },
    });
    expect(parseZoomPercent('banana')).toEqual({
      ok: false,
      error: { failure: 'notANumber', offending: 'banana' },
    });
    expect(parseZoomPercent('-')).toMatchObject({ ok: false, error: { failure: 'notANumber' } });
    expect(parseZoomPercent('100pt')).toEqual({
      ok: false,
      error: { failure: 'unknownUnit', offending: 'pt' },
    });
    // Each of these is a distinct way a person mistypes, and none of them is read as something
    // adjacent.
    expect(parseZoomPercent('1.2.3').ok).toBe(false);
    expect(parseZoomPercent('1 000').ok).toBe(false);
  });

  /** Out of range is not a failure. It is a number a person meant. */
  it('clamps rather than refusing, and says that it clamped', () => {
    expect(parseZoomPercent('700')).toEqual({ ok: true, percent: 500, clamped: true });
    expect(parseZoomPercent('5')).toEqual({ ok: true, percent: 10, clamped: true });
    expect(parseZoomPercent('100')).toMatchObject({ clamped: false });
  });
});

// ── the status bar's ladder ──────────────────────────────────────────────────

describe('the status bar drops segments in a declared order', () => {
  const widths = [containerPresets.desktop, containerPresets.tablet, containerPresets.phone];

  it('at three widths, and the phone keeps exactly the ones that never drop', () => {
    const shownAt = (width: number): string[] =>
      statusPriorityNames.filter((priority) => statusPresentationAt(priority, width) === 'shown');

    expect(shownAt(containerPresets.desktop)).toEqual([...statusPriorityNames]);
    expect(shownAt(containerPresets.tablet)).toEqual(['standard', 'essential']);
    expect(shownAt(containerPresets.phone)).toEqual(['essential']);
  });

  /** ⚠ Nothing is lost: whatever leaves the bar is somewhere, and the two sets are the whole set. */
  it('nothing is lost at any width — shown and overflowed partition the ladder', () => {
    for (const width of widths) {
      const shown = statusPriorityNames.filter(
        (priority) => statusPresentationAt(priority, width) === 'shown',
      );
      const overflow = statusPriorityNames.filter(
        (priority) => statusPresentationAt(priority, width) === 'overflow',
      );
      expect(shown.length + overflow.length).toBe(statusPriorityNames.length);
      expect(new Set([...shown, ...overflow]).size).toBe(statusPriorityNames.length);
    }
  });

  it('the ladder is monotonic: a narrower bar never shows more', () => {
    for (const priority of statusPriorityNames) {
      let previous = true;
      for (let width = 1600; width >= 280; width -= 10) {
        const shown = statusPresentationAt(priority, width) === 'shown';
        expect(previous || !shown).toBe(true);
        previous = shown;
      }
    }
  });

  it('the ladder is ordered — each priority gives way before the one above it', () => {
    const drops = statusPriorityNames.map(
      (priority) => statusPriorities[priority].dropAtOrBelow ?? 0,
    );
    for (let index = 1; index < drops.length; index += 1) {
      expect(drops[index]).toBeLessThan(drops[index - 1] ?? Number.POSITIVE_INFINITY);
    }
    // The top of the ladder never drops at all, which is what "a phone has room for two of them"
    // means as a declaration rather than as a hope.
    expect(statusPriorities.essential.dropAtOrBelow).toBeUndefined();
  });

  it('the overflow disclosure counts in the right number', () => {
    expect(statusOverflowLabel(0)).toBe('No hidden status items');
    expect(statusOverflowLabel(1)).toBe('1 more status item');
    expect(statusOverflowLabel(3)).toBe('3 more status items');
  });
});

describe('the status bar’s stylesheet, and the specificity accident it is arranged around', () => {
  /**
   * ⚠ **MJXOFF-183's accident, for the fourth time.** A `@container` block changes no specificity,
   * so a base rule written as a bare class would be (0,1,0), would beat every generated block at
   * every width, and the probe would report `shown` forever — with the gate comparing the component
   * against a model it silently never followed.
   *
   * MJXOFF-183 also records that the *order* assertion alone was true and useless. Both are here.
   */
  it('every presentation rule is wrapped in :where(), so nothing out-specifies anything', () => {
    const presentationRules = statusBarCss
      .split('\n')
      .filter((line) => line.includes(statusPresentationProperty))
      .length;
    // The base declaration plus one per priority that drops.
    const dropping = statusPriorityNames.filter(
      (priority) => statusPriorities[priority].dropAtOrBelow !== undefined,
    ).length;
    expect(presentationRules).toBe(1 + dropping);
    expect(statusBarCss).toContain(`:where(.probe) {\n    ${statusPresentationProperty}: shown;`);
    for (const priority of statusPriorityNames) {
      if (statusPriorities[priority].dropAtOrBelow === undefined) continue;
      expect(statusBarCss).toContain(`:where(.probe[data-priority='${priority}'])`);
    }
  });

  it('the container blocks are emitted after the base rules they override', () => {
    const base = statusBarCss.indexOf(`:where(.probe) {`);
    const first = statusBarCss.indexOf('@container');
    expect(base).toBeGreaterThan(-1);
    expect(first).toBeGreaterThan(base);
  });

  it('[hidden] is restated last, because the UA’s rule loses to any author rule at all', () => {
    // MJXOFF-189's finding: `[hidden] { display: none }` is user-agent origin, so a component's own
    // `display:` beats it — with the attribute set, the accessibility tree correct, and the element
    // on screen.
    const hidden = statusBarCss.lastIndexOf('[hidden]');
    const lastDisplay = statusBarCss.lastIndexOf('display:', hidden);
    expect(hidden).toBeGreaterThan(-1);
    expect(lastDisplay).toBeLessThan(hidden);
  });
});
