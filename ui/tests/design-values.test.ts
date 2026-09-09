import { ESLint } from 'eslint';
import tseslint from 'typescript-eslint';
import { describe, expect, it } from 'vitest';

import mjx from '../eslint-rules/design-values.js';

/**
 * The literal-value lint, watched failing in every direction it is supposed to fail in.
 *
 * MJXOFF-181's *"Done when"*: **"A literal-value lint fails on a hard-coded hex, radius or duration
 * in a component, proved on a throwaway."** The throwaways are the sources below — deliberately bad
 * modules that are never imported and never built, fed to a linter configured with nothing but the
 * rule under test, so a failure is unambiguous.
 *
 * A lint rule nobody has watched reject anything is a lint rule that might be matching nothing.
 * That is U01's doctrine for `mjx/story-conventions`, and it matters more here, because this rule
 * is the only thing standing between the palette re-seed and one control that stayed the old green.
 *
 * ## Both directions, and the second is the harder one
 *
 * Rejecting `#2e9e63` is easy. What a rule like this gets wrong is the *false positive* — flagging
 * a value in a comment, or a `1px` hairline for which no token exists — because a rule that cries
 * wolf gets disabled, and a disabled rule is indistinguishable from a clean codebase. So there are
 * as many acceptance cases below as rejection ones, and the real `src/` tree is linted as a whole
 * in `npm run lint`, which is the largest acceptance case there is.
 */

/** ESLint with the rule under test and nothing else. */
function linter(): ESLint {
  return new ESLint({
    overrideConfigFile: true,
    overrideConfig: [
      {
        files: ['**/*.ts'],
        languageOptions: { parser: tseslint.parser, ecmaVersion: 2023, sourceType: 'module' },
        plugins: { mjx },
        rules: { 'mjx/no-literal-design-values': 'error' },
      },
    ],
  });
}

async function messageIds(source: string): Promise<string[]> {
  const [result] = await linter().lintText(source, { filePath: 'component.ts' });
  return (result?.messages ?? []).map((message) => message.messageId ?? '(none)');
}

describe('what mjx/no-literal-design-values rejects', () => {
  it('a hex colour in a shadow-root stylesheet', async () => {
    const source = 'const styles = `:host { background: #2e9e63; }`;\nexport default styles;';
    expect(await messageIds(source)).toEqual(['literalColor']);
  });

  it('a hex colour in a plain string, not only in a template', async () => {
    expect(await messageIds("export const brand = '#223b33';")).toEqual(['literalColor']);
  });

  it('the same colour spelled as a function', async () => {
    const source = 'const styles = `:host { color: rgba(34, 59, 51, 0.11); }`;\nexport default styles;';
    expect(await messageIds(source)).toEqual(['literalColor']);
  });

  it('a hard-coded corner radius', async () => {
    // The specific failure §4 warns about: reverting towards Office's 2–4px corners. Two messages,
    // because a 4px corner is both a literal length and a radius that reads no token, and both
    // sentences are worth saying.
    const source = 'const styles = `.card { border-radius: 4px; }`;\nexport default styles;';
    expect(await messageIds(source)).toEqual(
      expect.arrayContaining(['literalLength', 'literalRadius']),
    );
  });

  it('a radius that is a literal even when it is a large one', async () => {
    // 16px happens to equal --radius-card today. That is exactly why it must be rejected: it is
    // invisible until the token moves, and then it is one card with the old corner.
    const source = 'const styles = `.card { border-radius: 16px; }`;\nexport default styles;';
    expect(await messageIds(source)).toEqual(
      expect.arrayContaining(['literalLength', 'literalRadius']),
    );
  });

  it('a literal duration and a literal easing', async () => {
    const source =
      'const styles = `.panel { transition: transform 150ms cubic-bezier(0.34, 1.56, 0.64, 1); }`;\n' +
      'export default styles;';
    expect(await messageIds(source)).toEqual(
      expect.arrayContaining(['literalDuration', 'literalEasing']),
    );
  });

  it('a spacing written in pixels rather than in --spacing', async () => {
    const source = 'const styles = `.row { gap: 8px; padding: 12px; }`;\nexport default styles;';
    expect(await messageIds(source)).toEqual(['literalLength', 'literalLength']);
  });

  it('a type size written in pixels', async () => {
    const source = 'const styles = `.label { font-size: 12px; }`;\nexport default styles;';
    expect(await messageIds(source)).toEqual(['literalLength']);
  });
});

describe('what it accepts, which is the half that keeps it switched on', () => {
  it('a stylesheet written entirely in tokens', async () => {
    const source =
      'const styles = `\n' +
      '  :host {\n' +
      '    background: var(--theme-surface);\n' +
      '    border-radius: var(--radius-card);\n' +
      '    padding: calc(var(--spacing) * 3);\n' +
      '    transition: transform var(--duration-transition) var(--ease-ink);\n' +
      '  }\n' +
      '`;\nexport default styles;';
    expect(await messageIds(source)).toEqual([]);
  });

  it('a 1px hairline, for which no token exists', async () => {
    const source = 'const styles = `:host { border: 1px solid var(--theme-border); }`;\nexport default styles;';
    expect(await messageIds(source)).toEqual([]);
  });

  it('zero, in any unit', async () => {
    const source =
      'const styles = `:host { margin: 0; outline: 0px; transition-duration: 0s; }`;\nexport default styles;';
    expect(await messageIds(source)).toEqual([]);
  });

  it('a relative unit, which is a layout relationship rather than a measurement', async () => {
    const source = 'const styles = `output { min-inline-size: 5ch; inline-size: 50%; }`;\nexport default styles;';
    expect(await messageIds(source)).toEqual([]);
  });

  it('a value inside a CSS comment, which is where an explanation belongs', async () => {
    // The false positive that would have got the rule disabled. `src/harness/resizable-container.ts`
    // explains its container-query defect in terms of 700px, 666px and 34px, and the explanation is
    // the most valuable text in the file.
    const source =
      'const styles = `\n' +
      '  /* A 16px padding would make a 700px frame report 666px — every breakpoint 34px late. */\n' +
      '  .frame { container-type: inline-size; }\n' +
      '`;\nexport default styles;';
    expect(await messageIds(source)).toEqual([]);
  });

  it('a radius composed by interpolation, whose value lives in another module', async () => {
    const source =
      'import { radiusVariable } from "./surfaces.ts";\n' +
      'const styles = `.card { border-radius: ${radiusVariable("card")}; }`;\n' +
      'export default styles;';
    expect(await messageIds(source)).toEqual([]);
  });

  it('a URL, which contains a // that is not a comment', async () => {
    const source = "export const ns = 'http://www.w3.org/2000/svg';";
    expect(await messageIds(source)).toEqual([]);
  });
});
