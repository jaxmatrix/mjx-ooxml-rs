/**
 * **The one gate between a document's palette and a static picture**, shared by every gallery whose art is drawn in
 * the document's colours: the Table Styles galleries in `stories/ribbons/table-tools-menus.ts` and the WordArt Quick
 * Styles gallery in `stories/ribbons/wordart-styles-menus.ts`.
 *
 * Both build their pictures as one markup string made static with `unsafeStatic`, because `<mjx-gallery-item>` clones
 * its children into the gallery's shadow root and so its art must carry no lit binding. A value that reaches such a
 * string is not escaped by lit, so it is checked here, once, rather than by two copies of a regular expression that
 * could drift apart.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

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
