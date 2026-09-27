/**
 * MJXOFF-185's gallery, registered together.
 *
 * The same reasoning `src/controls/index.ts`, `src/ribbon/index.ts` and `src/menus/index.ts` give:
 * a component that is only defined by the story that happened to import it is a component whose
 * absence looks like a rendering bug in a *different* story. A gallery is the worst case of it —
 * an unregistered `<mjx-gallery-item>` is an ordinary element with its art still in the light DOM,
 * so a gallery that had not been defined would render as a wall of unstyled miniatures rather than
 * as nothing, which is a failure that looks like a styling bug.
 */

export { MjxGallery, defineGallery } from './gallery.ts';
export { MjxGalleryItem, defineGalleryItem, itemChangedEvent } from './gallery-item.ts';
export type { GalleryItemDescriptor } from './gallery-item.ts';
export { PreviewSession, applyPreviewEvent } from './preview-session.ts';
export type { PreviewEvent, PreviewEventKind, PreviewSubject } from './preview-session.ts';
export { galleryEvents, galleryTags, galleryDocumentCss } from './gallery-model.ts';

import { defineGallery } from './gallery.ts';
import { defineGalleryItem } from './gallery-item.ts';

/** Register both. Idempotent, because a story file and a test may both ask. */
export function defineGalleryElements(): void {
  defineGalleryItem();
  defineGallery();
}
