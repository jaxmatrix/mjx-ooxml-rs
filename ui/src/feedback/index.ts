/**
 * The feedback family: five things a person sees while something is happening, six elements, one
 * model.
 *
 * ```ts
 * import { defineFeedback, feedbackDocumentCss } from './src/feedback/index.ts';
 * defineFeedback();
 * ```
 *
 * ⚠ **`feedbackDocumentCss` must reach the document**, and it is not optional decoration. It
 * carries the `@property` registrations for the six `<time>` spans, and an *unregistered* custom
 * property resolves to the un-substituted text `calc(150ms * 4)` rather than to a time — so a shell
 * that only calls `defineFeedback()` gets a screentip that falls back to its generated default and a
 * host override that reaches nothing. `.storybook/preview.ts` appends it exactly as it appends
 * `surfaceDocumentCss`, and for the same class of reason.
 */

export {
  MjxMiniToolbar,
  defineMiniToolbar,
} from './mini-toolbar.ts';
export { MjxScreentip, defineScreentip } from './screentip.ts';
export { MjxToast, MjxToastRegion, defineToasts } from './toast.ts';
export { MjxProgress, defineProgress } from './progress.ts';
export { MjxEmptyState, defineEmptyState } from './empty-state.ts';
export * from './feedback-model.ts';

import { defineMiniToolbar } from './mini-toolbar.ts';
import { defineScreentip } from './screentip.ts';
import { defineToasts } from './toast.ts';
import { defineProgress } from './progress.ts';
import { defineEmptyState } from './empty-state.ts';

/** Register all six. Idempotent. */
export function defineFeedback(): void {
  defineMiniToolbar();
  defineScreentip();
  defineToasts();
  defineProgress();
  defineEmptyState();
}
