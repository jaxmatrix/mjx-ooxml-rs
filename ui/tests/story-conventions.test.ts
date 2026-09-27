import { ESLint } from 'eslint';
import tseslint from 'typescript-eslint';
import { describe, expect, it } from 'vitest';

import mjx from '../eslint-rules/story-conventions.js';
import { conventionsParameter, expectsViolations, storyConventions } from '../src/story/conventions.ts';
import { customProperties } from '../tokens/tokens.ts';
import type { TokenPath } from '../src/tokens/resolver.ts';

/**
 * The story conventions, both halves.
 *
 * `BUILD_PLAN_LOOP_1.md` §3 requires every component to ship its states matrix, its token
 * dependencies and its keyboard and screen-reader behaviour. Fifteen further Phase U children
 * inherit whatever is enforced here, so this suite's job is to watch the enforcement **fail** —
 * a lint rule nobody has seen reject anything might be matching nothing at all.
 */

/** A token that certainly exists, chosen from the generated table so no name is written here. */
const realToken = Object.keys(customProperties)[0] as TokenPath;

const complete = {
  statesMatrix: [{ name: 'default', description: 'Resting.' }],
  tokenDependencies: [realToken],
  keyboard: [{ keys: 'Tab', does: 'Focuses the control.' }],
  screenReader: 'Announces its label and its state.',
} as const;

describe('storyConventions', () => {
  it('returns the conventions a meta puts under its own parameter key', () => {
    expect(conventionsParameter).toBe('mjx');
    expect(storyConventions(complete)).toMatchObject({ screenReader: complete.screenReader });
  });

  it('refuses an empty states matrix', () => {
    expect(() => storyConventions({ ...complete, statesMatrix: [] })).toThrow(/statesMatrix is empty/);
  });

  it('refuses an empty token-dependency list', () => {
    expect(() => storyConventions({ ...complete, tokenDependencies: [] })).toThrow(
      /tokenDependencies is empty/,
    );
  });

  it('refuses an empty keyboard list and an empty screen-reader note', () => {
    expect(() => storyConventions({ ...complete, keyboard: [] })).toThrow(/keyboard is empty/);
    expect(() => storyConventions({ ...complete, screenReader: '  ' })).toThrow(/screenReader is empty/);
  });

  it('refuses a token dependency that names no token, by name', () => {
    // This is the one a document cannot enforce. A hand-written dependency list rots the moment a
    // token is renamed, and a stale list is worse than none because a reader trusts it — the very
    // next thing that happens to this palette is a re-seed from another product.
    expect(() =>
      storyConventions({
        ...complete,
        tokenDependencies: [realToken, 'color.notAToken' as TokenPath],
      }),
    ).toThrow(/'color\.notAToken'/);
  });

  it('refuses an expectation with no rules and one with no reason', () => {
    expect(() => expectsViolations([], 'because')).toThrow(/at least one axe rule/);
    expect(() => expectsViolations(['color-contrast'], ' ')).toThrow(/must say why/);
  });
});

// ── the lint rule ────────────────────────────────────────────────────────────

const goodStory = `
import { storyConventions } from '../src/story/conventions.ts';
const meta = {
  title: 'Test/Thing',
  parameters: {
    mjx: storyConventions({
      statesMatrix: [{ name: 'default', description: 'Resting.' }],
      tokenDependencies: ['color.paper'],
      keyboard: [{ keys: 'Tab', does: 'Focuses.' }],
      screenReader: 'Announces its label.',
    }),
  },
};
export default meta;
export const Default = {};
`;

const noConventions = `
export default { title: 'Test/Thing', parameters: { layout: 'centered' } };
export const Default = {};
`;

const notThroughFactory = `
export default {
  title: 'Test/Thing',
  parameters: {
    mjx: {
      statesMatrix: [{ name: 'default', description: 'Resting.' }],
      tokenDependencies: ['color.paper'],
      keyboard: [{ keys: 'Tab', does: 'Focuses.' }],
      screenReader: 'Announces its label.',
    },
  },
};
`;

const missingKeyboard = `
import { storyConventions } from '../src/story/conventions.ts';
export default {
  title: 'T',
  parameters: {
    mjx: storyConventions({
      statesMatrix: [{ name: 'default', description: 'Resting.' }],
      tokenDependencies: ['color.paper'],
      screenReader: 'Announces its label.',
    }),
  },
};
`;

const emptyMatrix = `
import { storyConventions } from '../src/story/conventions.ts';
export default {
  title: 'T',
  parameters: {
    mjx: storyConventions({
      statesMatrix: [],
      tokenDependencies: ['color.paper'],
      keyboard: [{ keys: 'Tab', does: 'Focuses.' }],
      screenReader: 'Announces its label.',
    }),
  },
};
`;

const noDefaultExport = `
export const Default = {};
`;

/** ESLint configured with nothing but the rule under test, so a failure is unambiguous. */
function linter(): ESLint {
  return new ESLint({
    overrideConfigFile: true,
    overrideConfig: [
      {
        files: ['**/*.stories.ts'],
        languageOptions: { parser: tseslint.parser, ecmaVersion: 2023, sourceType: 'module' },
        plugins: { mjx },
        rules: { 'mjx/story-conventions': 'error' },
      },
    ],
  });
}

async function messageIds(source: string): Promise<string[]> {
  const [result] = await linter().lintText(source, { filePath: 'thing.stories.ts' });
  return (result?.messages ?? []).map((message) => message.messageId ?? '(none)');
}

describe('the mjx/story-conventions lint rule', () => {
  it('accepts a story that declares all four through the factory', async () => {
    expect(await messageIds(goodStory)).toEqual([]);
  });

  it('rejects a meta with no conventions at all', async () => {
    // The failure the runtime validator structurally cannot see: nothing was called, so nothing
    // could throw.
    expect(await messageIds(noConventions)).toEqual(['noParameters']);
  });

  it('rejects conventions written as a bare object, which nothing would validate', async () => {
    expect(await messageIds(notThroughFactory)).toEqual(['notThroughFactory']);
  });

  it('rejects a story missing its keyboard behaviour', async () => {
    expect(await messageIds(missingKeyboard)).toEqual(['missing']);
  });

  it('rejects an empty states matrix statically, as well as at runtime', async () => {
    expect(await messageIds(emptyMatrix)).toEqual(['empty']);
  });

  it('rejects a story file with no default export', async () => {
    expect(await messageIds(noDefaultExport)).toEqual(['noDefaultExport']);
  });

  it('accepts conventions hoisted into a named const', async () => {
    // Which is the shape every story in this repository actually uses, because the conventions are
    // long enough that nesting them inside the meta buries the title.
    const source = `
import { storyConventions } from '../src/story/conventions.ts';
const conventions = storyConventions({
  statesMatrix: [{ name: 'default', description: 'Resting.' }],
  tokenDependencies: ['color.paper'],
  keyboard: [{ keys: 'Tab', does: 'Focuses.' }],
  screenReader: 'Announces its label.',
});
const meta = { title: 'T', parameters: { mjx: conventions } };
export default meta;
`;
    expect(await messageIds(source)).toEqual([]);
  });
});
