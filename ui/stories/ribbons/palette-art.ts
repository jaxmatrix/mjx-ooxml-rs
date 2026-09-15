/**
 * **The one gate between a document's palette and a static picture**, shared by every gallery whose art is drawn in
 * the document's colours: the Table Styles galleries in `stories/ribbons/table-tools-menus.ts`, the WordArt Quick
 * Styles gallery in `stories/ribbons/wordart-styles-menus.ts` and the picture Quick Styles gallery in
 * `stories/ribbons/picture-tools-menus.ts`.
 *
 * Each builds its pictures as one markup string made static with `unsafeStatic`, because `<mjx-gallery-item>` clones
 * its children into the gallery's shadow root and so its art must carry no lit binding. A value that reaches such a
 * string is not escaped by lit, so it is checked here, once, rather than by two copies of a regular expression that
 * could drift apart.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import type { ThemeColorPalette, ThemeColorSlot } from '../../src/pickers/picker-model.ts';

/**
 * A palette value **only if it is a hex colour**, and otherwise `undefined`.
 *
 * `ThemeColorPalette` is `string`-typed, so a value such as `red;background:url(…)` or `"><script>` is representable;
 * a style attribute built from it would be an injection. A document colour is `#rrggbb` in every palette this
 * catalogue has, so nothing else is let through, and a caller falls back to a token.
 */
export function hexColour(value: string | undefined): string | undefined {
  return value !== undefined && /^#[0-9a-fA-F]{3,8}$/.test(value) ? value : undefined;
}

/**
 * **A palette slot's colour, checked**, or the token a document that has not said falls back to: `--document-page`
 * for Background 1, `--theme-text-primary` for every other slot. Moved here from the WordArt gallery when the picture
 * styles gallery needed the same rule, so the two cannot drift.
 */
export function paletteSlotColour(palette: ThemeColorPalette, slot: ThemeColorSlot): string {
  const fallback = slot === 'background1' ? 'var(--document-page)' : 'var(--theme-text-primary)';
  return hexColour(palette[slot]) ?? fallback;
}

/** **A step of a picture's own scale**, so every length a static picture draws is a multiple of the spacing token. */
export function spacingStep(multiple: number): string {
  return `calc(var(--spacing) * ${String(multiple)})`;
}
