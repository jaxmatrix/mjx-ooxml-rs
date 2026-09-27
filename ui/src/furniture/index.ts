/**
 * MJXOFF-190's document furniture, and the one call that registers it.
 *
 * The chrome that frames the document surface: a status bar with a declared drop order, a zoom
 * control that is a slider and a numeric field composed, a scrollbar over a viewport it does not
 * own, and a splitter between two regions.
 *
 * Two of the four are **the boundary where web chrome meets the Rust canvas**, which is why the
 * contracts they expose are the interesting part of this child rather than the drawings. The
 * scrollbar mirrors R13's `ScrollModel` — an estimated extent corrected page by page, with the
 * anchor rather than the offset as the state — and the splitter shares its arithmetic with
 * `<mjx-task-pane>` through `src/foundations/splitter.ts` rather than restating it.
 */

import { defineScrollbar } from './scrollbar.ts';
import { defineSplitter } from './splitter.ts';
import { defineStatusBar } from './status-bar.ts';
import { defineStatusSegment } from './status-segment.ts';
import { defineZoomControl } from './zoom-control.ts';

export { MjxStatusBar, defineStatusBar, overflowSlotName, statusBarSheet, statusOverflowIcon } from './status-bar.ts';
export { MjxStatusSegment, defineStatusSegment, statusSegmentSheet } from './status-segment.ts';
export {
  MjxZoomControl,
  defineZoomControl,
  zoomControlSheet,
  zoomInvalidIcon,
  zoomStepIcons,
} from './zoom-control.ts';
export {
  MjxScrollMark,
  MjxScrollbar,
  defineScrollbar,
  marksChangedEvent,
  scrollLineFraction,
  scrollMarksIn,
  scrollbarSheet,
} from './scrollbar.ts';
export { MjxSplitter, defineSplitter, splitterSheet } from './splitter.ts';
export * from './furniture-model.ts';
export * from './scroll-model.ts';

/** Register all six. Idempotent, and what `.storybook/preview.ts` calls. */
export function defineFurniture(): void {
  defineStatusSegment();
  defineStatusBar();
  defineZoomControl();
  defineScrollbar();
  defineSplitter();
}
