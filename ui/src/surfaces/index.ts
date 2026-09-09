/**
 * The surface family: six surfaces, three elements, one model.
 *
 * ```ts
 * import { defineSurfaces, surfaceDocumentCss } from './src/surfaces/index.ts';
 * defineSurfaces();
 * ```
 *
 * ⚠ **`surfaceDocumentCss` must reach the document**, and it is not optional decoration. It carries
 * the three **scheme-keyed** custom properties — the scrim, the modal's edge and the sheet's handle
 * — and `:root` is a document selector, so a shell that only calls `defineSurfaces()` gets a modal
 * with an unpainted scrim. `.storybook/preview.ts` appends it exactly as it appends
 * `galleryDocumentCss` and `inputDocumentCss`, and for the same reason.
 */

export { MjxDialog, defineDialog, dialogBoundaryFractions } from './dialog.ts';
export { MjxPopover, definePopover, popoverExpectedStops, type PopoverKind } from './popover.ts';
export { MjxTaskPane, defineTaskPane } from './task-pane.ts';
export { SurfaceSession, surfaceIdAttribute, type SurfaceSessionHost } from './surface-session.ts';
export * from './surface-model.ts';

import { defineDialog } from './dialog.ts';
import { definePopover } from './popover.ts';
import { defineTaskPane } from './task-pane.ts';

/** Register all three. Idempotent. */
export function defineSurfaces(): void {
  defineDialog();
  definePopover();
  defineTaskPane();
}
