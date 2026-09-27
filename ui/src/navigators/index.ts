/**
 * MJXOFF-191's four navigators, and the one call that registers them.
 *
 * How a user moves through a document that does not fit on a screen: Word's navigation pane,
 * PowerPoint's slide sorter, Excel's sheet tabs, and the windowed list all three are built on.
 *
 * **One virtualisation**, in `src/foundations/virtual-list.ts` and `src/foundations/extent-table.ts`
 * — U06's `galleryWindow` and U11's prefix sums, lifted down a level and given a binary search so
 * the rows need not all be one row tall. **Three ARIA patterns**, in `navigatorAriaPatterns`,
 * because a tree, a listbox and a tablist are three different things and using the wrong one is
 * worse than using none.
 */

import { defineSheetTabBar } from './sheet-tab-bar.ts';
import { defineThumbnailRail } from './thumbnail-rail.ts';
import { defineTree } from './tree.ts';
import { defineVirtualList } from './virtual-list.ts';

export { MjxVirtualList, defineVirtualList, virtualListPattern, virtualListSheet } from './virtual-list.ts';
export type { VirtualItem } from './virtual-list.ts';
export { MjxTree, defineTree, treePattern, treeSheet } from './tree.ts';
export {
  MjxThumbnailRail,
  defineThumbnailRail,
  thumbnailRailPattern,
  thumbnailRailSheet,
} from './thumbnail-rail.ts';
export {
  MjxSheetTabBar,
  defineSheetTabBar,
  sheetTabBarPattern,
  sheetTabBarSheet,
  tabScrollLabels,
} from './sheet-tab-bar.ts';
export { VirtualScroller, defaultEstimatedRowExtent } from './virtual-scroller.ts';
export type { ScrollerParts, VirtualRowSource } from './virtual-scroller.ts';
export * from './navigator-model.ts';
export {
  sheetTabBarCss,
  thumbnailRailCss,
  treeCss,
  virtualListCss,
} from './navigator-sheets.ts';

/** Register all four. Idempotent, and what `.storybook/preview.ts` calls. */
export function defineNavigators(): void {
  defineVirtualList();
  defineTree();
  defineThumbnailRail();
  defineSheetTabBar();
}
