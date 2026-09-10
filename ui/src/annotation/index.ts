/**
 * MJXOFF-193's annotation family: four elements, one packing algorithm, one author palette.
 *
 * ```ts
 * import { defineAnnotation, annotationDocumentCss } from './src/annotation/index.ts';
 * defineAnnotation();
 * ```
 *
 * ⚠ **`annotationDocumentCss` must reach the document**, and it is not optional decoration. It
 * carries the eight **scheme-keyed** author colours, and `:root` is a document selector — so a shell
 * that only calls `defineAnnotation()` gets a review pane whose author bands are all the border
 * colour, which reads as a document with one author rather than as a missing stylesheet.
 */

export {
  MjxAnnotationCard,
  MjxCommentCard,
  actionButton,
  defineCommentCard,
  type CardParts,
  type CardPlacement,
} from './comment-card.ts';
export {
  MjxCommentThread,
  defineCommentThread,
  type CommentReply,
} from './comment-thread.ts';
export { MjxTrackedChangeCard, defineTrackedChangeCard } from './tracked-change-card.ts';
export {
  MjxReviewPane,
  defineReviewPane,
  estimatedCardExtent,
  type ReviewAnnotation,
} from './review-pane.ts';
export {
  annotationDocumentCss,
  annotationMotionClass,
  annotationTypeRoles,
  commentCardCss,
  commentThreadCss,
  connectorInsetLength,
  reviewPaneCss,
  trackedChangeCardCss,
} from './annotation-sheets.ts';
export * from './annotation-model.ts';
export * from './author-colour.ts';

import { defineCommentCard } from './comment-card.ts';
import { defineCommentThread } from './comment-thread.ts';
import { defineReviewPane } from './review-pane.ts';
import { defineTrackedChangeCard } from './tracked-change-card.ts';

/** Register all four. Idempotent, and what `.storybook/preview.ts` calls. */
export function defineAnnotation(): void {
  defineCommentCard();
  defineCommentThread();
  defineTrackedChangeCard();
  defineReviewPane();
}
