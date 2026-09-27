/**
 * Choosing a token to probe, without naming one.
 *
 * **No token name is written anywhere in this child**, and this file is why the resolution-order
 * gate can keep that promise: the probe, its story and its test all ask for *the first colour
 * token the generator emitted* rather than for a particular one. The palette is about to be
 * re-seeded from an existing product of the user's, and a gate that named `--color-green` would go
 * red on the day the seeding lands — which is exactly the day the resolution order most needs to
 * still be proved.
 *
 * Data only, for the reason `dev/bands.ts` and `src/harness/presets.ts` give: the browser tier's
 * specs run in Node and cannot import a module that defines a custom element.
 */

import { customProperties, tokens } from '../tokens/tokens.ts';
import type { TokenPath } from '../src/tokens/resolver.ts';

/** A token's generated default, by dotted path. */
export function generatedValue(path: TokenPath): string {
  let cursor: unknown = tokens;
  for (const segment of path.split('.')) {
    cursor = (cursor as Record<string, unknown>)[segment];
  }
  return String(cursor);
}

/**
 * The first token whose generated default is a colour.
 *
 * "First" is the generator's own emission order, which is the source file's order — stable across
 * regenerations, and arbitrary in a way that is a feature: a probe that reads an arbitrary token
 * cannot have been tuned to a convenient one.
 */
export function firstColorToken(): TokenPath {
  for (const path of Object.keys(customProperties) as TokenPath[]) {
    if (/^#[0-9a-fA-F]{6,8}$/.test(generatedValue(path))) return path;
  }
  throw new Error('the generated token table contains no colour token.');
}
