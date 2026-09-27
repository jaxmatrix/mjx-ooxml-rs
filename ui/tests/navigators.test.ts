import { describe, expect, it } from 'vitest';

import { ExtentTable, extentOrigin } from '../src/foundations/extent-table.ts';
import {
  anchorAtOffset,
  defaultOverscanRows,
  offsetOfAnchor,
  offsetToReveal,
  rowsInWindow,
  stableOffsetFor,
  virtualWindow,
  windowForOffset,
  windowSpacers,
  type KeyedAnchor,
} from '../src/foundations/virtual-list.ts';
import { cellsInWindow, galleryWindow, galleryOverscanRows } from '../src/gallery/gallery-model.ts';
import { ScrollbarModel } from '../src/furniture/scroll-model.ts';
import { phoneShellAtOrBelow } from '../src/harness/presets.ts';
import {
  applyRailSelection,
  emptyRailSelection,
  countNodes,
  findNode,
  flattenTree,
  indentNode,
  isTreeReorder,
  moveAmongSiblings,
  moveSelectionBy,
  navigatorAriaPatterns,
  navigatorOrder,
  navigatorPhoneSheetAtOrBelow,
  navigatorPresentationAt,
  navigatorPresentationProperty,
  navigatorTags,
  nextTreeIndex,
  noModifiers,
  outdentNode,
  railRowOfSlide,
  railRowPlan,
  railSlideName,
  railThumbnailState,
  renameKeyOutcome,
  sheetColourIsAnEdgeNotAFill,
  sheetNameForbidden,
  sheetNameMaximum,
  sheetNameProblem,
  subtreeIds,
  tabScrollTarget,
  treeExpandOutcome,
  treeKeyAction,
  typeAheadIndex,
  type RailSlide,
  type TreeNode,
} from '../src/navigators/navigator-model.ts';
import {
  sheetTabBarCss,
  thumbnailRailCss,
  treeCss,
  virtualListCss,
} from '../src/navigators/navigator-sheets.ts';
import { largeDeck, largeNavigatorCount, largeOutline } from '../stories/navigators/specimens.ts';

/**
 * MJXOFF-191's Node tier.
 *
 * Four navigators, one virtualiser and three ARIA patterns — and almost everything worth asserting
 * about them is *arithmetic over a sequence*, which is exactly what a browser test cannot sweep. So
 * the shape of this file follows the ticket's two traps:
 *
 * * **Virtualisation is invisible on a short list**, so every count here is taken against
 *   `largeNavigatorCount` — the number the stories use — and the ceiling always has an anti-vacuity
 *   assertion beside it. U06's ceiling was once satisfied by zero and would have stayed green for a
 *   component that never previewed at all.
 * * **Scroll-position stability when items change above the viewport never appears in a still**, so
 *   every stability assertion is made **beside the naive answer** — keep the offset — and requires
 *   the two to differ. U11's rule: without a positive control a stability assertion is a tolerance
 *   nobody tested.
 *
 * And the first section is the one the ticket asks to be reported on: **what was lifted**, asserted
 * as an equivalence rather than left as a claim.
 */

// ── the lift, asserted rather than claimed ───────────────────────────────────

/**
 * `galleryWindow`'s body as MJXOFF-185 committed it, copied here **for one purpose**: to be the
 * thing the binding is compared against.
 *
 * A copy in a test file is not the duplication the ticket forbids; a copy in a second component is.
 * This one exists so that a drift in `virtualWindow` fails with two numbers rather than silently
 * changing what a gallery, a font list and a swatch grid all do.
 */
function galleryWindowAsCommitted(
  rowCount: number,
  firstVisibleRow: number,
  visibleRows: number,
  overscan = 1,
): { firstRow: number; lastRow: number } {
  if (rowCount <= 0) return { firstRow: 0, lastRow: 0 };
  const first = Math.max(0, Math.min(firstVisibleRow, rowCount - 1) - overscan);
  const last = Math.min(rowCount, Math.max(firstVisibleRow, 0) + Math.max(visibleRows, 1) + overscan);
  return { firstRow: first, lastRow: Math.max(last, first) };
}

describe('U06’s virtualisation was lifted, not copied', () => {
  it('galleryWindow is byte-for-byte the same answer over a sweep of 1,120 calls', () => {
    let compared = 0;
    for (const rowCount of [0, 1, 2, 7, 50, 400, 5000]) {
      for (const first of [-3, 0, 1, 5, 49, 399, 4999, 6000]) {
        for (const visible of [0, 1, 3, 8, 40]) {
          for (const overscan of [0, 1, 2, 4]) {
            expect(galleryWindow(rowCount, first, visible, overscan)).toEqual(
              galleryWindowAsCommitted(rowCount, first, visible, overscan),
            );
            compared += 1;
          }
        }
      }
    }
    // Anti-vacuity: a sweep whose loops never ran would pass every assertion inside them.
    expect(compared).toBe(7 * 8 * 5 * 4);
  });

  it('and its default overscan is still one, which is what the callers rely on', () => {
    expect(galleryOverscanRows).toBe(defaultOverscanRows);
    expect(galleryWindow(80, 20, 4)).toEqual(galleryWindowAsCommitted(80, 20, 4));
  });

  it('the two the gallery’s own suite asserts still hold, through the binding', () => {
    expect(galleryWindow(80, 0, 4, 1)).toEqual({ firstRow: 0, lastRow: 5 });
    expect(galleryWindow(80, 20, 4, 1)).toEqual({ firstRow: 19, lastRow: 25 });
    expect(galleryWindow(80, 78, 4, 1)).toEqual({ firstRow: 77, lastRow: 80 });
    expect(galleryWindow(0, 0, 4, 1)).toEqual({ firstRow: 0, lastRow: 0 });
  });

  it('rowsInWindow and cellsInWindow agree over a one-column plan, which is what a rail is', () => {
    const plan = railRowPlan(largeDeck(120).map(({ section: _section, ...rest }) => rest));
    const window_ = virtualWindow(plan.length, 10, 8);
    expect(cellsInWindow(plan, window_)).toBe(rowsInWindow(window_));
  });
});

describe('U11’s extent table was lifted, and the scrollbar still agrees with it', () => {
  /**
   * The same sequence of operations, applied to `ScrollbarModel` and to a bare `ExtentTable`.
   *
   * MJXOFF-190's own rule for a lift, quoted: *if a binding ever drifts, the two callers disagree
   * about what the same operation does, which is exactly the defect two copies would have had.*
   */
  it('over 200 random operation sequences', () => {
    let corrections = 0;
    for (let seed = 1; seed <= 200; seed += 1) {
      const pages = 5 + (seed % 30);
      const height = 100 + (seed % 7) * 10;
      const model = ScrollbarModel.fromExtent({ pages, pageHeight: height, precision: 'estimated' });
      const table = new ExtentTable(pages, height);

      for (let step = 0; step < 12; step += 1) {
        const which = (seed * 7 + step * 13) % 3;
        const page = (seed + step * 3) % (pages + 4);
        if (which === 0) {
          const measured = 40 + ((seed + step) % 400);
          model.recordMeasuredHeight(page, measured);
          table.recordMeasured(page, measured);
          if (page < table.count) corrections += 1;
        } else if (which === 1) {
          const wanted = 1 + ((seed + step * 5) % 40);
          model.extendToAtLeast(wanted);
          table.extendToAtLeast(wanted);
        } else {
          const wanted = 1 + ((seed * 3 + step) % 50);
          model.recordExactPageCount(wanted);
          table.setCount(wanted);
        }

        expect(model.pageCount).toBe(table.count);
        expect(model.totalHeight).toBe(table.total);
        expect(model.measuredPages).toBe(table.measuredCount);
        for (const probe of [0, 1, Math.floor(table.count / 2), table.count - 1, table.count + 3]) {
          expect(model.offsetOf(probe)).toBe(table.offsetOf(probe));
        }
        for (const offset of [-1, 0, 1, table.total / 3, table.total - 1, table.total + 99]) {
          const fromModel = model.anchorAt(offset);
          const fromTable = table.anchorAt(offset);
          expect(fromModel.page).toBe(fromTable.index);
          expect(fromModel.within).toBe(fromTable.within);
          expect(model.offsetOfAnchor(fromModel)).toBe(table.offsetOfAnchor(fromTable));
        }
      }
    }
    // Anti-vacuity: a sweep that never recorded a measurement would compare two untouched tables.
    expect(corrections).toBeGreaterThan(500);
  });

  it('and the scrollbar keeps its own floor of one page, which the table deliberately does not', () => {
    const model = ScrollbarModel.fromExtent({ pages: 0, pageHeight: 500, precision: 'estimated' });
    expect(model.pageCount).toBe(1);
    // A list with no rows is a legitimate state — an empty outline, a deck whose slides have not
    // loaded — and forcing it to have one would draw a row nobody put there.
    expect(new ExtentTable(0, 500).count).toBe(0);
  });
});

// ── the extent table ─────────────────────────────────────────────────────────

describe('the extent table', () => {
  function table(): ExtentTable {
    const built = new ExtentTable(10, 100);
    built.recordMeasured(0, 250);
    built.recordMeasured(1, 50);
    return built;
  }

  it('sums the extents it was given, measured and guessed alike', () => {
    expect(table().total).toBe(250 + 50 + 8 * 100);
    expect(table().offsetOf(0)).toBe(0);
    expect(table().offsetOf(1)).toBe(250);
    expect(table().offsetOf(2)).toBe(300);
  });

  it('answers the document’s own extent past the end, never zero', () => {
    const built = table();
    expect(built.offsetOf(99)).toBe(built.total);
  });

  it('converts an offset into an anchor and back', () => {
    const built = table();
    expect(built.anchorAt(-4)).toEqual(extentOrigin);
    expect(built.anchorAt(0)).toEqual(extentOrigin);
    expect(built.anchorAt(275)).toEqual({ index: 1, within: 25 });
    expect(built.offsetOfAnchor({ index: 1, within: 25 })).toBe(275);
  });

  it('inserts and removes in the middle, which is what a scrollbar never had to do', () => {
    const built = table();
    built.insertAt(1, 3);
    expect(built.count).toBe(13);
    // The three new rows are estimates and they sit between the two measured ones.
    expect(built.measuredCount).toBe(2);
    expect(built.offsetOf(4)).toBe(250 + 3 * 100);
    built.removeAt(1, 3);
    expect(built.count).toBe(10);
    expect(built.offsetOf(1)).toBe(250);
  });

  it('keeps measurements across a resize, which is R13’s reasoning unchanged', () => {
    const built = table();
    built.setCount(4);
    expect(built.measuredCount).toBe(2);
    expect(built.metric(0)?.extent).toBe(250);
    built.setCount(12);
    expect(built.measuredCount).toBe(2);
    expect(built.count).toBe(12);
  });

  it('re-estimates the guessed rows and leaves the measured ones alone', () => {
    const built = table();
    built.reEstimate(400);
    expect(built.metric(0)?.extent).toBe(250);
    expect(built.metric(5)?.extent).toBe(400);
  });
});

// ── the window ───────────────────────────────────────────────────────────────

describe('the window over rows of different heights', () => {
  /** Ten rows: two tall ones, then eight short. */
  function mixed(): ExtentTable {
    const built = new ExtentTable(10, 20);
    built.recordMeasured(0, 200);
    built.recordMeasured(1, 200);
    return built;
  }

  it('finds the first visible row by search rather than by dividing', () => {
    // A divider at the estimate would say row 12 for offset 250; the truth is row 1.
    expect(windowForOffset(mixed(), 250, 100, 0).firstRow).toBe(1);
    expect(Math.floor(250 / 20)).not.toBe(1);
  });

  it('covers the viewport and stops, whatever the row heights are', () => {
    const window_ = windowForOffset(mixed(), 400, 60, 0);
    expect(window_.firstRow).toBe(2);
    // Three twenty-tall rows cover sixty. The walk stops there rather than walking the table.
    expect(rowsInWindow(window_)).toBe(3);
  });

  it('builds at least one row in a viewport of zero, which is a component nobody has laid out', () => {
    // U06's *a ceiling satisfied by zero*, in its most literal form: a window of nothing would be a
    // list that renders empty for ever, and every count assertion would pass over it.
    expect(rowsInWindow(windowForOffset(mixed(), 0, 0, 0))).toBeGreaterThan(0);
  });

  it('is empty for an empty table and only for an empty table', () => {
    expect(windowForOffset(new ExtentTable(0, 20), 0, 500)).toEqual({ firstRow: 0, lastRow: 0 });
  });

  it('spaces out exactly the rows it did not build', () => {
    const built = mixed();
    const spacers = windowSpacers(built, { firstRow: 2, lastRow: 5 });
    expect(spacers.leading).toBe(400);
    expect(spacers.trailing).toBe(built.total - built.offsetOf(5));
    // The two spacers plus the built rows are the whole table: a sizer that measured only what was
    // built is a scrollbar that lies further the further you scroll.
    expect(spacers.leading + 3 * 20 + spacers.trailing).toBe(built.total);
  });

  it('scrolls to a row by the least that reveals it, and not at all when it is already there', () => {
    const built = mixed();
    expect(offsetToReveal(built, 5, 100, 0)).toBe(built.offsetOf(5) + 20 - 100);
    expect(offsetToReveal(built, 0, 100, 300)).toBe(0);
    // Already fully visible: the list must not move at all, or every arrow key jumps the viewport.
    expect(offsetToReveal(built, 3, 100, built.offsetOf(3))).toBe(built.offsetOf(3));
  });

  it('never scrolls past the end', () => {
    const built = mixed();
    expect(offsetToReveal(built, 9, 100, 0)).toBeLessThanOrEqual(built.total - 100);
  });
});

// ── the trap: stability when things change above the viewport ────────────────

describe('scroll position when rows change above the viewport', () => {
  function deck(count: number): { table: ExtentTable; keys: string[] } {
    const keys = Array.from({ length: count }, (_unused, index) => `s${String(index)}`);
    const table = new ExtentTable(count, 60);
    return { table, keys };
  }

  it('a keyed anchor survives an insertion above it and the naive answer does not', () => {
    const before = deck(400);
    const offset = before.table.offsetOf(200) + 17;
    const anchor = anchorAtOffset(before.table, before.keys, offset);
    expect(anchor).toEqual({ key: 's200', within: 17 });

    const after = deck(400);
    after.keys.unshift(...['a', 'b', 'c'].map((letter) => `new-${letter}`));
    after.table.insertAt(0, 3);

    const kept = offsetOfAnchor(after.table, after.keys, anchor as KeyedAnchor);
    expect(kept).toBe(offset + 3 * 60);

    // ⚠ The positive control. Without it *the row did not move* is trivially true of a list that
    // never scrolls, of one whose rows are all the same height, and of a change below the viewport.
    const naive = offset;
    expect(kept).not.toBe(naive);
    expect((kept ?? 0) - naive).toBe(180);
  });

  it('an INDEX anchor would have failed the same case, which is why the anchor is a key', () => {
    const before = deck(400);
    const offset = before.table.offsetOf(200);
    const indexAnchor = before.table.anchorAt(offset);

    const after = deck(400);
    after.table.insertAt(0, 3);
    // The index still resolves — to the wrong row. The reader is moved by three slides and nothing
    // reports an error, which is precisely why this shape of defect ships.
    expect(after.table.offsetOfAnchor(indexAnchor)).toBe(offset);
    expect(after.keys[indexAnchor.index]).toBe('s200');
    after.keys.unshift('new-a', 'new-b', 'new-c');
    expect(after.keys[indexAnchor.index]).toBe('s197');
  });

  it('a removal above the reader moves them back by exactly what was removed', () => {
    const before = deck(400);
    const anchor = anchorAtOffset(before.table, before.keys, before.table.offsetOf(300));
    const after = deck(400);
    after.keys.splice(10, 5);
    after.table.removeAt(10, 5);
    expect(offsetOfAnchor(after.table, after.keys, anchor as KeyedAnchor)).toBe(
      before.table.offsetOf(300) - 5 * 60,
    );
  });

  it('a row that has gone reports that it has gone rather than answering zero', () => {
    const after = deck(10);
    expect(offsetOfAnchor(after.table, after.keys, { key: 'gone', within: 4 })).toBeUndefined();
    // A list that scrolled to the top whenever anything anywhere was deleted is what returning
    // zero here would have produced.
  });

  it('the offset is clamped to what the list can actually scroll', () => {
    const built = deck(10);
    const anchor: KeyedAnchor = { key: 's9', within: 0 };
    // The wanted offset is past the end, and a scrollTop write past the end is silently clamped by
    // the browser to a DIFFERENT number — so the model and the DOM would disagree about where the
    // list is, which is the state every subsequent window is computed from.
    expect(stableOffsetFor(built.table, built.keys, anchor, 400, 0)).toBe(10 * 60 - 400);
    expect(stableOffsetFor(built.table, built.keys, undefined, 400, 99)).toBe(99);
  });

  it('a correction to a row above the reader moves the offset by exactly the correction', () => {
    const built = deck(400);
    const anchor = anchorAtOffset(built.table, built.keys, built.table.offsetOf(200));
    built.table.recordMeasured(5, 260);
    expect(offsetOfAnchor(built.table, built.keys, anchor as KeyedAnchor)).toBe(
      200 * 60 + (260 - 60),
    );
    // Anti-vacuity, R13's own: a stability assertion alone is green for a model that never corrects.
    expect(built.table.measuredCount).toBe(1);
  });
});

// ── the fixtures are large enough for any of this to mean anything ──────────

describe('the fixtures are the size the trap requires', () => {
  it('five thousand of each, which is the number the stories and the gates share', () => {
    expect(largeNavigatorCount).toBeGreaterThanOrEqual(5000);
    expect(largeDeck()).toHaveLength(largeNavigatorCount);
    expect(countNodes(largeOutline())).toBeGreaterThanOrEqual(largeNavigatorCount);
  });

  it('and a window over them builds a screenful, which is the whole claim', () => {
    const table = new ExtentTable(largeNavigatorCount, 40);
    const window_ = windowForOffset(table, 0, 600);
    expect(rowsInWindow(window_)).toBeGreaterThan(0);
    expect(rowsInWindow(window_)).toBeLessThan(largeNavigatorCount / 100);
  });
});

// ── the tree ─────────────────────────────────────────────────────────────────

const outline: readonly TreeNode[] = [
  {
    id: 'a',
    label: 'Alpha',
    children: [
      { id: 'a1', label: 'Alpha one' },
      { id: 'a2', label: 'Alpha two', children: [{ id: 'a2i', label: 'Alpha two i' }] },
    ],
  },
  { id: 'b', label: 'Bravo', children: [{ id: 'b1', label: 'Bravo one' }] },
  { id: 'c', label: 'Charlie' },
];

describe('flattening a tree', () => {
  it('shows only what is open, and counts positions within a branch', () => {
    const rows = flattenTree(outline, new Set(['a']));
    expect(rows.map((row) => row.id)).toEqual(['a', 'a1', 'a2', 'b', 'c']);
    const a2 = rows.find((row) => row.id === 'a2');
    expect(a2?.level).toBe(2);
    expect(a2?.posInSet).toBe(2);
    // ⚠ Over its OWN SIBLINGS, not over the flattened list. A set size taken from the visible rows
    // would announce "3 of 5" for the second of two children.
    expect(a2?.setSize).toBe(2);
    expect(a2?.parentId).toBe('a');
  });

  it('marks a leaf as unexpandable and never as collapsed', () => {
    const rows = flattenTree(outline, new Set());
    const charlie = rows.find((row) => row.id === 'c');
    expect(charlie?.expandable).toBe(false);
    expect(charlie?.expanded).toBe(false);
  });

  it('an id in the expanded set that names a leaf opens nothing', () => {
    expect(flattenTree(outline, new Set(['c']))).toHaveLength(3);
  });
});

describe('a heading moves with its subtree', () => {
  it('subtreeIds is the node and everything beneath it', () => {
    expect(subtreeIds(outline, 'a')).toEqual(['a', 'a1', 'a2', 'a2i']);
    expect(subtreeIds(outline, 'c')).toEqual(['c']);
    expect(subtreeIds(outline, 'nope')).toEqual([]);
  });

  it('moving among siblings carries the children', () => {
    const moved = moveAmongSiblings(outline, 'a', 1);
    expect(moved.map((node) => node.id)).toEqual(['b', 'a', 'c']);
    // ⚠ The assertion the ticket names. A keyboard implementation that swapped two entries in the
    // FLATTENED list would leave these three behind under whichever heading ended up above them,
    // and it would look completely correct until somebody collapsed the branch.
    expect(subtreeIds(moved, 'a')).toEqual(['a', 'a1', 'a2', 'a2i']);
    expect(findNode(moved, 'a')?.children).toHaveLength(2);
  });

  it('and refuses to move off either end of its own branch', () => {
    expect(moveAmongSiblings(outline, 'a', -1).map((node) => node.id)).toEqual(['a', 'b', 'c']);
    expect(moveAmongSiblings(outline, 'c', 1).map((node) => node.id)).toEqual(['a', 'b', 'c']);
    // It does NOT hop into the next branch: a keyboard reorder that silently reparented a heading
    // would be an outline edit nobody asked for.
    expect(moveAmongSiblings(outline, 'a1', -1)).toEqual(outline);
  });

  it('indent makes it the last child of the sibling above, subtree included', () => {
    const indented = indentNode(outline, 'b');
    expect(indented.map((node) => node.id)).toEqual(['a', 'c']);
    expect(findNode(indented, 'a')?.children?.map((child) => child.id)).toEqual(['a1', 'a2', 'b']);
    expect(subtreeIds(indented, 'b')).toEqual(['b', 'b1']);
  });

  it('and refuses on the first of a branch, which has nothing above it', () => {
    expect(indentNode(outline, 'a')).toEqual(outline);
    expect(indentNode(outline, 'a1')).toEqual(outline);
  });

  it('outdent makes it the next sibling of its parent, and leaves its later siblings alone', () => {
    const out = outdentNode(outline, 'a1');
    expect(out.map((node) => node.id)).toEqual(['a', 'a1', 'b', 'c']);
    expect(findNode(out, 'a')?.children?.map((child) => child.id)).toEqual(['a2']);
    // GUESS: Word promotes the trailing siblings into the promoted heading. This does not, and the
    // divergence is deliberate — promoting text a person had not selected is an edit noticed three
    // saves later.
    expect(findNode(out, 'a1')?.children ?? []).toHaveLength(0);
  });

  it('and refuses at the top level', () => {
    expect(outdentNode(outline, 'a')).toEqual(outline);
  });

  it('every reorder leaves the same nodes in the tree, only somewhere else', () => {
    for (const operation of [
      () => moveAmongSiblings(outline, 'a2', -1),
      () => indentNode(outline, 'b'),
      () => outdentNode(outline, 'a2i'),
      () => outdentNode(outline, 'b1'),
    ]) {
      expect(countNodes(operation())).toBe(countNodes(outline));
    }
  });
});

describe('the tree’s keyboard', () => {
  it('maps the ARIA tree keys', () => {
    expect(treeKeyAction('ArrowDown', noModifiers)).toBe('next');
    expect(treeKeyAction('ArrowUp', noModifiers)).toBe('previous');
    expect(treeKeyAction('Home', noModifiers)).toBe('first');
    expect(treeKeyAction('End', noModifiers)).toBe('last');
    expect(treeKeyAction('ArrowRight', noModifiers)).toBe('expand');
    expect(treeKeyAction('ArrowLeft', noModifiers)).toBe('collapse');
    expect(treeKeyAction('Enter', noModifiers)).toBe('activate');
    expect(treeKeyAction(' ', noModifiers)).toBe('activate');
    expect(treeKeyAction('*', noModifiers)).toBe('expandAll');
  });

  it('mirrors the horizontal pair under right-to-left', () => {
    expect(treeKeyAction('ArrowLeft', noModifiers, 'rtl')).toBe('expand');
    expect(treeKeyAction('ArrowRight', noModifiers, 'rtl')).toBe('collapse');
  });

  it('Alt is what separates reorder from navigate, and it mirrors too', () => {
    const alt = { ...noModifiers, altKey: true };
    expect(treeKeyAction('ArrowUp', alt)).toBe('moveUp');
    expect(treeKeyAction('ArrowDown', alt)).toBe('moveDown');
    expect(treeKeyAction('ArrowRight', alt)).toBe('indent');
    expect(treeKeyAction('ArrowLeft', alt)).toBe('outdent');
    expect(treeKeyAction('ArrowRight', alt, 'rtl')).toBe('outdent');
    expect(treeKeyAction('ArrowLeft', alt, 'rtl')).toBe('indent');
    expect(['moveUp', 'moveDown', 'indent', 'outdent'].every((action) =>
      isTreeReorder(action as never),
    )).toBe(true);
    expect(isTreeReorder('next')).toBe(false);
  });

  it('returns nothing for a key that is not ours, so Tab keeps meaning Tab', () => {
    expect(treeKeyAction('Tab', noModifiers)).toBeUndefined();
    expect(treeKeyAction('a', noModifiers)).toBeUndefined();
    expect(treeKeyAction('ArrowDown', { ...noModifiers, ctrlKey: true })).toBeUndefined();
  });

  it('ArrowRight means three different things and ArrowLeft means three more', () => {
    const rows = flattenTree(outline, new Set(['a']));
    const closed = rows.find((row) => row.id === 'a2');
    const open = rows.find((row) => row.id === 'a');
    const leaf = rows.find((row) => row.id === 'c');
    const child = rows.find((row) => row.id === 'a1');

    expect(treeExpandOutcome(closed, 'expand')).toBe('open');
    expect(treeExpandOutcome(open, 'expand')).toBe('toFirstChild');
    expect(treeExpandOutcome(leaf, 'expand')).toBe('nothing');

    expect(treeExpandOutcome(open, 'collapse')).toBe('close');
    expect(treeExpandOutcome(child, 'collapse')).toBe('toParent');
    expect(treeExpandOutcome(leaf, 'collapse')).toBe('nothing');
    expect(treeExpandOutcome(undefined, 'expand')).toBe('nothing');
  });

  it('clamps at both ends rather than wrapping', () => {
    expect(nextTreeIndex('next', 4, 5)).toBe(4);
    expect(nextTreeIndex('previous', 0, 5)).toBe(0);
    expect(nextTreeIndex('first', 3, 5)).toBe(0);
    expect(nextTreeIndex('last', 0, 5)).toBe(4);
    expect(nextTreeIndex('next', 0, 0)).toBe(-1);
  });
});

describe('type-ahead, which all four share', () => {
  const labels = ['Alpha', 'Bravo', 'Alpine', 'Charlie', 'Alps'];

  it('searches from after the cursor and wraps once', () => {
    expect(typeAheadIndex(labels, 'al', -1)).toBe(0);
    expect(typeAheadIndex(labels, 'al', 0)).toBe(2);
    expect(typeAheadIndex(labels, 'al', 2)).toBe(4);
    // Wrapping, which is right for a letter and wrong for an arrow key: an arrow means one further
    // in this direction, a letter means the next thing called that.
    expect(typeAheadIndex(labels, 'al', 4)).toBe(0);
  });

  it('is case-insensitive, and answers nothing for a buffer nothing starts with', () => {
    expect(typeAheadIndex(labels, 'BRA', -1)).toBe(1);
    expect(typeAheadIndex(labels, 'zz', 0)).toBeUndefined();
    expect(typeAheadIndex(labels, '', 0)).toBeUndefined();
    expect(typeAheadIndex([], 'a', 0)).toBeUndefined();
  });
});

// ── the rail ─────────────────────────────────────────────────────────────────

const deckFixture: readonly RailSlide[] = [
  { id: 's1', label: 'Title', section: 'Opening' },
  { id: 's2', label: 'Agenda', section: 'Opening', hidden: true },
  { id: 's3', label: 'Method', section: 'Body', thumbnail: '/plate-3.png' },
  { id: 's4', label: 'Results', section: 'Body' },
];

describe('the rail’s plan', () => {
  it('is U06’s row shape, one column, with a heading per section', () => {
    const plan = railRowPlan(deckFixture);
    expect(plan.map((row) => row.kind)).toEqual([
      'heading',
      'cells',
      'cells',
      'heading',
      'cells',
      'cells',
    ]);
    // Delegating the shape is what lets `cellsInWindow` and the whole of U06's arithmetic be reused
    // rather than restated — U08's precedent, applied to a second plan.
    expect(cellsInWindow(plan, { firstRow: 0, lastRow: plan.length })).toBe(deckFixture.length);
  });

  it('and a deck with no sections is one row per slide and nothing else', () => {
    const plan = railRowPlan(deckFixture.map(({ section: _section, ...rest }) => rest));
    expect(plan.every((row) => row.kind === 'cells')).toBe(true);
    expect(plan).toHaveLength(deckFixture.length);
  });

  it('locates a slide in the plan, headings included', () => {
    const plan = railRowPlan(deckFixture);
    expect(railRowOfSlide(plan, 0)).toBe(1);
    expect(railRowOfSlide(plan, 2)).toBe(4);
    expect(railRowOfSlide(plan, 99)).toBe(0);
  });

  it('is empty for an empty deck', () => {
    expect(railRowPlan([])).toEqual([]);
  });
});

describe('the rail’s selection', () => {
  it('a plain choice replaces the selection and moves the anchor', () => {
    const next = applyRailSelection(deckFixture, emptyRailSelection, 2, {
      toggle: false,
      extend: false,
    });
    expect(next).toEqual({ selected: ['s3'], anchor: 2, cursor: 2 });
  });

  it('Ctrl adds and removes one, keeping the rest', () => {
    let state = applyRailSelection(deckFixture, emptyRailSelection, 0, { toggle: false, extend: false });
    state = applyRailSelection(deckFixture, state, 3, { toggle: true, extend: false });
    expect(state.selected).toEqual(['s1', 's4']);
    state = applyRailSelection(deckFixture, state, 0, { toggle: true, extend: false });
    expect(state.selected).toEqual(['s4']);
  });

  it('Shift takes the range from the anchor, in the deck’s own order, in both directions', () => {
    const start = applyRailSelection(deckFixture, emptyRailSelection, 3, { toggle: false, extend: false });
    const back = applyRailSelection(deckFixture, start, 1, { toggle: false, extend: true });
    expect(back.selected).toEqual(['s2', 's3', 's4']);
    expect(back.anchor).toBe(3);
    const forward = applyRailSelection(deckFixture, back, 3, { toggle: false, extend: true });
    expect(forward.selected).toEqual(['s4']);
  });

  it('Ctrl + Shift ADDS the range rather than replacing the selection', () => {
    // ⚠ The order in `applyRailSelection` is the whole of this decision. Read the other way round,
    // the range replaces what was chosen and the Ctrl is silently dropped.
    let state = applyRailSelection(deckFixture, emptyRailSelection, 0, { toggle: false, extend: false });
    state = applyRailSelection(deckFixture, state, 2, { toggle: true, extend: false });
    state = applyRailSelection(deckFixture, state, 3, { toggle: true, extend: true });
    expect(state.selected).toEqual(['s1', 's3', 's4']);
  });

  it('an empty deck selects nothing rather than throwing', () => {
    expect(applyRailSelection([], emptyRailSelection, 3, { toggle: false, extend: false })).toEqual(
      emptyRailSelection,
    );
  });
});

describe('the rail’s reorder', () => {
  it('moves a contiguous selection as a block', () => {
    const moved = moveSelectionBy(deckFixture, ['s1', 's2'], 1);
    expect(moved.map((slide) => slide.id)).toEqual(['s3', 's1', 's2', 's4']);
  });

  it('moves a SCATTERED selection as a block, which is why it is not a per-slide swap', () => {
    const moved = moveSelectionBy(deckFixture, ['s1', 's3'], 1);
    expect(moved.map((slide) => slide.id)).toEqual(['s2', 's1', 's3', 's4']);
    // A per-slide swap would have produced ['s2','s1','s4','s3'] — the selection quietly becomes
    // something else, and nobody who meant *move my slides down* would call that what they asked for.
    expect(moved.map((slide) => slide.id)).not.toEqual(['s2', 's1', 's4', 's3']);
  });

  it('refuses at both ends rather than moving part of the selection', () => {
    expect(moveSelectionBy(deckFixture, ['s1', 's3'], -1)).toEqual(deckFixture);
    expect(moveSelectionBy(deckFixture, ['s2', 's4'], 1)).toEqual(deckFixture);
  });

  it('is a no-op for an empty selection, a zero delta, or everything selected', () => {
    expect(moveSelectionBy(deckFixture, [], 1)).toEqual(deckFixture);
    expect(moveSelectionBy(deckFixture, ['s1'], 0)).toEqual(deckFixture);
    expect(moveSelectionBy(deckFixture, ['s1', 's2', 's3', 's4'], 1)).toEqual(deckFixture);
  });

  it('never loses or duplicates a slide, over every selection of the fixture', () => {
    const ids = deckFixture.map((slide) => slide.id);
    let swept = 0;
    for (let mask = 1; mask < 1 << ids.length; mask += 1) {
      const chosen = ids.filter((_unused, index) => (mask & (1 << index)) !== 0);
      for (const delta of [-1, 1]) {
        const moved = moveSelectionBy(deckFixture, chosen, delta);
        expect([...moved.map((slide) => slide.id)].sort()).toEqual([...ids].sort());
        swept += 1;
      }
    }
    expect(swept).toBe((2 ** ids.length - 1) * 2);
  });
});

describe('what a rail says about a slide', () => {
  it('puts the position, the label, the section and hidden into one name', () => {
    expect(railSlideName(deckFixture[1] as RailSlide, 2, 4)).toBe('Slide 2 of 4, Agenda, Opening, hidden');
    expect(railSlideName(deckFixture[2] as RailSlide, 3, 4)).toBe('Slide 3 of 4, Method, Body');
  });

  it('tells a plate that has arrived from one that has not', () => {
    expect(railThumbnailState(deckFixture[2] as RailSlide)).toBe('ready');
    expect(railThumbnailState(deckFixture[0] as RailSlide)).toBe('pending');
    expect(railThumbnailState({ id: 'x', label: 'x', thumbnail: '' })).toBe('pending');
  });
});

// ── the sheet tab bar ────────────────────────────────────────────────────────

describe('the sheet tab bar', () => {
  it('the affordances clamp at the ends rather than wrapping', () => {
    expect(tabScrollTarget('first', 4, 10)).toBe(0);
    expect(tabScrollTarget('last', 4, 10)).toBe(9);
    expect(tabScrollTarget('previous', 0, 10)).toBe(0);
    expect(tabScrollTarget('next', 9, 10)).toBe(9);
    expect(tabScrollTarget('next', 0, 0)).toBe(-1);
  });

  it('refuses the names Excel refuses, and says which', () => {
    expect(sheetNameProblem('Summary', [])).toBeUndefined();
    expect(sheetNameProblem('   ', [])).toContain('empty');
    expect(sheetNameProblem('x'.repeat(sheetNameMaximum + 1), [])).toContain(
      String(sheetNameMaximum),
    );
    expect(sheetNameProblem('Q1/Q2', [])).toContain('/');
    expect(sheetNameProblem('summary', ['Summary'])).toContain('already');
    // Every forbidden character is refused, not just the one somebody remembered.
    for (const character of sheetNameForbidden) {
      expect(sheetNameProblem(`Sheet${character}`, [])).toBeDefined();
    }
  });

  it('a name at exactly the maximum is allowed, which is the off-by-one', () => {
    expect(sheetNameProblem('x'.repeat(sheetNameMaximum), [])).toBeUndefined();
  });

  it('Enter commits and Escape cancels, and they are different outcomes', () => {
    expect(renameKeyOutcome('Enter', 'Revenue')).toEqual({ kind: 'commit', label: 'Revenue' });
    expect(renameKeyOutcome('Escape', 'Revenue')).toEqual({ kind: 'cancel' });
    expect(renameKeyOutcome('a', 'Revenu')).toEqual({ kind: 'continue' });
    // ⚠ The half that is usually wrong. A field that committed on Escape has destroyed a name with
    // the key people press to mean *stop*, and the two are the same picture afterwards.
    expect(renameKeyOutcome('Escape', 'Revenue')).not.toEqual(
      renameKeyOutcome('Enter', 'Revenue'),
    );
  });

  it('states that a sheet colour is an edge, because a label may not sit on the user’s colour', () => {
    expect(sheetColourIsAnEdgeNotAFill).toBe(true);
  });
});

// ── three patterns, and they are three ───────────────────────────────────────

describe('the ARIA patterns', () => {
  it('are three different container roles across four navigators', () => {
    const containers = navigatorOrder.map((name) => navigatorAriaPatterns[name].container);
    expect(containers).toEqual(['listbox', 'tree', 'listbox', 'tablist']);
    expect(new Set(containers).size).toBe(3);
  });

  it('pair each container with the item role that belongs inside it', () => {
    expect(navigatorAriaPatterns.tree.item).toBe('treeitem');
    expect(navigatorAriaPatterns.virtualList.item).toBe('option');
    expect(navigatorAriaPatterns.thumbnailRail.item).toBe('option');
    expect(navigatorAriaPatterns.sheetTabBar.item).toBe('tab');
  });

  it('and only the rail is multi-selectable', () => {
    const many = navigatorOrder.filter((name) => navigatorAriaPatterns[name].multiSelectable);
    expect(many).toEqual(['thumbnailRail']);
  });

  it('every one of them says why, in a sentence rather than a word', () => {
    for (const name of navigatorOrder) {
      expect(navigatorAriaPatterns[name].why.length).toBeGreaterThan(60);
    }
  });

  it('the four tags are the four this child registers', () => {
    expect(Object.keys(navigatorTags).sort()).toEqual([...navigatorOrder].sort());
    expect(Object.values(navigatorTags).every((tag) => tag.startsWith('mjx-'))).toBe(true);
  });
});

/** The selector of the rule a declaration at `at` sits inside. */
function selectorBefore(css: string, at: number): string {
  const opened = css.lastIndexOf('{', at);
  return css.slice(0, opened).trimEnd().split('\n').slice(-1)[0]?.trim() ?? '';
}

describe('the phone presentation, and the specificity accident it would otherwise be', () => {
  it('is a pane above the phone width and a sheet at or below it', () => {
    expect(navigatorPresentationAt(1280)).toBe('pane');
    expect(navigatorPresentationAt(navigatorPhoneSheetAtOrBelow + 1)).toBe('pane');
    expect(navigatorPresentationAt(navigatorPhoneSheetAtOrBelow)).toBe('sheet');
    expect(navigatorPresentationAt(320)).toBe('sheet');
  });

  it('emits the container block AFTER the base rule, and both at the same specificity', () => {
    for (const [name, css] of [
      ['tree', treeCss],
      ['rail', thumbnailRailCss],
    ] as const) {
      const base = css.indexOf(`${navigatorPresentationProperty}: pane`);
      const phone = css.indexOf(`${navigatorPresentationProperty}: sheet`);
      expect(base, `${name} declares no resting presentation`).toBeGreaterThan(-1);
      expect(phone, `${name} declares no phone presentation`).toBeGreaterThan(-1);
      // ⚠ A @container block changes no specificity, so source order is the whole arbitration —
      // and it arbitrates nothing between rules that are NOT of equal specificity. MJXOFF-183 found
      // that four times; both halves are asserted here because either alone is true and useless.
      expect(phone, `${name} emits its phone block before its base rule`).toBeGreaterThan(base);
      // Both selectors are :where()-wrapped, so both score (0,0,0) and source order decides.
      expect(selectorBefore(css, base), `${name}'s resting rule`).toBe(':where(:host)');
      expect(selectorBefore(css, phone), `${name}'s phone rule`).toBe(':where(:host)');

      /*
       * ⚠ **And the bare `:host` layout block declares none of the three properties the phone block
       * owns.** U03's rule — *the base rule paints nothing* — applied to a presentation: that block
       * scores (0,1,0) and would out-specify the container rule outright, so a stray `inline-size`
       * in it is a navigator that never becomes a sheet, with the emission-order assertion above
       * still perfectly green.
       */
      const layout = css.slice(css.indexOf(':host {'), css.indexOf('}', css.indexOf(':host {')));
      for (const property of ['inline-size', 'max-inline-size', 'border-radius']) {
        expect(layout, `${name}'s bare :host block declares ${property}`).not.toContain(property);
      }
    }
  });

  it('and the two components with one presentation publish none at all', () => {
    // A property published by a component that never changes it reads as a decision nobody took.
    expect(virtualListCss).not.toContain(navigatorPresentationProperty);
    expect(sheetTabBarCss).not.toContain(navigatorPresentationProperty);
  });
});

describe('the phone-sheet width is an alias and never a fifth definition', () => {
  it('is the same number the ribbon, the menu and the gallery use', () => {
    // U05 left the instruction in menu-model.ts — *when a third surface needs it, hoist it, do not
    // add a second definition* — U06 hoisted it, and these are the fifth and sixth to read it.
    expect(navigatorPhoneSheetAtOrBelow).toBe(phoneShellAtOrBelow);
  });
});
