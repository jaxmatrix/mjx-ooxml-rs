/**
 * The story conventions every Phase U child follows.
 *
 * `BUILD_PLAN_LOOP_1.md` §3:
 *
 * > **Every component ships three things** with it: its states matrix, its token dependencies (so
 * > a token change's blast radius is visible), and its keyboard and screen-reader behaviour. A
 * > component without those is not ready for audit.
 *
 * There are two halves to making that true rather than aspirational, and both are here:
 *
 * * **`storyConventions` validates at runtime** — an empty states matrix throws, and a token
 *   dependency naming a token the generator did not emit throws with the name in the message. The
 *   second is the one that earns its keep: a hand-written dependency list rots the moment a token
 *   is renamed, and a list nobody checks is worse than no list, because a reader trusts it.
 * * **`eslint-rules/story-conventions.js` validates statically** — so a story file that never
 *   calls `storyConventions` at all cannot slip past by exporting a bare meta. The lint rule is
 *   what a fifteen-child catalogue actually needs; the runtime check is what makes the *contents*
 *   true rather than merely present.
 *
 * `tests/story-conventions.test.ts` runs ESLint over deliberately bad story sources and asserts
 * both directions, because a lint rule nobody has watched fail is a lint rule that might be
 * matching nothing.
 */

import { customProperties } from '../../tokens/tokens.ts';
import type { TokenPath } from '../tokens/resolver.ts';

/** One row of the states matrix: a state the auditor must be able to see. */
export interface StoryState {
  /** `default`, `hover`, `disabled`, `loading`, `error`, … */
  readonly name: string;
  /** What puts the component into it, and what the auditor should look for. */
  readonly description: string;
}

/** One keyboard behaviour the component promises. */
export interface KeyboardBehaviour {
  /** The keys, written as a person presses them: `Tab`, `Shift + Tab`, `Arrow Down`, … */
  readonly keys: string;
  /** What happens. */
  readonly does: string;
}

/**
 * The declaration a gate story makes about its own accessibility violations.
 *
 * Only the throwaway probes under `dev/` carry this. It exists so the a11y sweep can assert that a
 * story which is *supposed* to fail actually does — see `tests/browser/a11y.spec.ts`.
 */
export interface ExpectedViolations {
  /** axe rule ids this story must violate, e.g. `color-contrast`. */
  readonly rules: readonly string[];
  /** Why a deliberately broken story exists in the catalogue at all. */
  readonly because: string;
}

/** Everything a component must ship beside its stories. */
export interface StoryConventions {
  readonly statesMatrix: readonly StoryState[];
  readonly tokenDependencies: readonly TokenPath[];
  readonly keyboard: readonly KeyboardBehaviour[];
  /** What a screen reader announces, in the words it announces them. */
  readonly screenReader: string;
  readonly expectViolations?: ExpectedViolations;
}

/**
 * Validate a story's conventions and return them, for a meta's `parameters.mjx`.
 *
 * ## Why this is not a `defineStoryMeta(meta, …)` wrapper
 *
 * It was, until Storybook's own indexer refused it. **CSF is statically analysed**: the index that
 * names every story in the built catalogue is produced by reading the source rather than by
 * running it, and it requires the default export to be an object literal — a call expression is
 * rejected with *"CSF: default export must be an object"*. A wrapper would therefore have made
 * every story file unindexable, which is a far worse outcome than a slightly longer declaration.
 *
 * So the conventions go where Storybook already expects arbitrary metadata — inside `parameters` —
 * and this function is what makes them *checked* rather than merely present. The static half of
 * the guarantee lives in `eslint-rules/story-conventions.js`, which reads the same shape out of
 * the source and can therefore also catch the story that never calls this at all.
 *
 * @throws {Error} when a required list is empty or a token dependency names no token.
 */
export function storyConventions(conventions: StoryConventions): StoryConventions {
  const { statesMatrix, tokenDependencies, keyboard, screenReader, expectViolations } = conventions;

  if (statesMatrix.length === 0) {
    throw new Error('statesMatrix is empty. A component with one state still has one row.');
  }
  if (tokenDependencies.length === 0) {
    throw new Error(
      'tokenDependencies is empty. If the component genuinely reads no token it is not themed, ' +
        'which is itself the thing to declare.',
    );
  }
  if (keyboard.length === 0) {
    throw new Error(
      'keyboard is empty. A component that cannot be reached by keyboard says so with a row that ' +
        "says so — 'not focusable' is a keyboard behaviour and an audit finding.",
    );
  }
  if (screenReader.trim() === '') {
    throw new Error('screenReader is empty.');
  }

  const unknown = tokenDependencies.filter((path) => !(path in customProperties));
  if (unknown.length > 0) {
    throw new Error(
      `tokenDependencies names ${unknown.map((path) => `'${path}'`).join(', ')}, which the ` +
        "generated token table does not contain. The list is how a token change's blast radius is " +
        'found, so a stale name in it is a component that silently drops off the list.',
    );
  }

  return {
    statesMatrix,
    tokenDependencies,
    keyboard,
    screenReader,
    ...(expectViolations === undefined ? {} : { expectViolations }),
  };
}

/** The parameter key the conventions live under, named once so a test cannot mistype it. */
export const conventionsParameter = 'mjx' as const;

/**
 * A story's declaration that it is *supposed* to fail an accessibility rule.
 *
 * Written per story rather than per component, because a component's whole point is usually that
 * three of its stories pass and one deliberately does not. Storybook merges story parameters over
 * meta parameters, so this lands beside the conventions the meta already declared.
 *
 * Only the throwaway probes under `dev/` may use it. There is no mechanism stopping a real
 * component from doing so, and there does not need to be one: a reviewer reading
 * `expectsViolations` in a component's story file is reading an admission.
 */
export function expectsViolations(
  rules: readonly string[],
  because: string,
): { mjx: { expectViolations: ExpectedViolations } } {
  if (rules.length === 0) {
    throw new Error('expectsViolations needs at least one axe rule id, or it declares nothing.');
  }
  if (because.trim() === '') {
    throw new Error('expectsViolations needs a reason: a deliberately broken story must say why.');
  }
  return { [conventionsParameter]: { expectViolations: { rules, because } } };
}
