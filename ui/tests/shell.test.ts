import { readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

import {
  absentFromShells,
  adHocAriaAttributes,
  adHocSubstituteTags,
  describeAdHocFinding,
  findAdHocSubstitutes,
  minimumDistinctComponents,
  shellApplicationNames,
  shellSizeNames,
  shellStories,
  shellStoryId,
  shellStoryTitle,
  tagsRequiredInShells,
} from '../stories/shell/shell-model.ts';
import { catalogueByTag, catalogueComponents } from '../src/mobile/touch-audit.ts';

/**
 * **The no-mock-up gate, before a browser starts.**
 *
 * MJXOFF-274's third condition, in its own words:
 *
 * > **Every shell is built from the real custom elements** — assert that the assembly contains no
 * > ad-hoc substitute for a catalogued component. A grep-style gate over the assembly's own source
 * > is enough, and it is the assertion that keeps this honest.
 *
 * The rendered half is `tests/browser/shell.spec.ts`. This is the half that catches the *cheap*
 * failure — a `<button>` written into a shell because wiring the real one was one more step — and
 * it catches it in a second rather than in a browser run.
 *
 * ## Proved able to fail, four ways
 *
 * A gate nobody has watched reject anything is a gate that might be matching nothing, which is this
 * catalogue's oldest lesson. So the rule is run over hand-written specimens as well as over the real
 * assembly: a raw control element, a `<div>` wearing a role, an interactive ARIA state, and — the
 * one that matters most — **prose that names a forbidden element and must not be flagged**, because
 * a rule that cannot tell a comment from markup is a rule nobody can describe in words and everyone
 * eventually disables.
 */

const shellDirectory = resolve(import.meta.dirname, '../stories/shell');

/** Every source file the assembly is written in. Read from the directory, never from a list. */
function assemblySources(): { name: string; source: string }[] {
  const names = readdirSync(shellDirectory).filter((name) => name.endsWith('.ts'));
  expect(
    names.length,
    'no sources found under stories/shell/. Every assertion below would pass over an empty set, ' +
      'which is the failure mode this suite exists to avoid.',
  ).toBeGreaterThanOrEqual(4);
  return names.map((name) => ({
    name,
    source: readFileSync(resolve(shellDirectory, name), 'utf8'),
  }));
}

describe('the assembly is built from the catalogue and not from markup', () => {
  it('names no ad-hoc substitute for a catalogued component', () => {
    const findings = assemblySources().flatMap(({ name, source }) =>
      findAdHocSubstitutes(source).map((finding) => describeAdHocFinding(name, finding)),
    );
    expect(findings).toEqual([]);
  });

  it('is not vacuous: every shell story composes a large number of the catalogue’s elements', () => {
    for (const application of shellApplicationNames) {
      const source = readFileSync(resolve(shellDirectory, `${application}.stories.ts`), 'utf8');
      const tags = new Set(
        [...source.matchAll(/<(mjx-[a-z-]+)[\s/>]/g)].map((match) => match[1] ?? ''),
      );
      const catalogued = [...tags].filter((tag) => catalogueByTag.has(tag));
      expect(
        catalogued.length,
        `${application}.stories.ts names only ${String(catalogued.length)} catalogued elements. ` +
          'A shell that composes almost nothing would satisfy the no-substitute rule perfectly by ' +
          'having nothing in it.',
      ).toBeGreaterThanOrEqual(20);
    }
  });

  it('names no `mjx-` element the catalogue does not ship', () => {
    const invented = new Set<string>();
    for (const { source } of assemblySources()) {
      for (const match of source.matchAll(/<(mjx-[a-z-]+)[\s/>]/g)) {
        const tag = match[1] ?? '';
        if (!catalogueByTag.has(tag)) invented.add(tag);
      }
    }
    expect(
      [...invented],
      'the assembly opens an `mjx-` element the catalogue does not know about — either a new ' +
        'component that belongs in `catalogueComponents`, or an invented one, which is the ' +
        'mock-up this gate exists against.',
    ).toEqual([]);
  });
});

describe('the rule can reject', () => {
  it('catches a raw control element and says which component it should have been', () => {
    const findings = findAdHocSubstitutes('const t = html`<button type="button">Bold</button>`;');
    expect(findings).toHaveLength(1);
    expect(findings[0]?.kind).toBe('tag');
    expect(findings[0]?.name).toBe('button');
    expect(findings[0]?.advice).toContain('mjx-button');
  });

  it('catches every tag it declares, one at a time', () => {
    for (const tag of Object.keys(adHocSubstituteTags)) {
      const findings = findAdHocSubstitutes(`html\`<${tag} />\``);
      expect(findings.map((finding) => finding.name), `<${tag}> was not caught`).toEqual([tag]);
    }
  });

  it('catches a div wearing a role, which is the form the temptation actually takes', () => {
    const findings = findAdHocSubstitutes('html`<div role="tablist" class="tabs"></div>`');
    expect(findings.map((finding) => finding.name)).toEqual(['role']);
  });

  it('catches every interactive ARIA state it declares', () => {
    for (const attribute of adHocAriaAttributes) {
      const findings = findAdHocSubstitutes(`html\`<div ${attribute}="true"></div>\``);
      expect(findings.map((finding) => finding.name), `${attribute} was not caught`).toEqual([
        attribute,
      ]);
    }
  });

  it('permits aria-label, because naming a region is the host’s job', () => {
    expect(findAdHocSubstitutes('html`<div aria-label="Workspace"></div>`')).toEqual([]);
  });

  /**
   * ⚠ **The instrument test, and the important one.**
   *
   * This very file, and `shell-model.ts` itself, name every forbidden element in prose. A scanner
   * that could not tell a comment from markup would flag its own documentation, somebody would add
   * an exemption, and the exemption would eventually be the thing that let a real substitute
   * through. The same argument `eslint-rules/design-values.js` makes about explaining a value.
   */
  it('does not flag a forbidden name that appears only in prose', () => {
    const source = [
      '/**',
      ' * A shell never opens a raw <button>, a <select> or a <dialog>, and never writes role=.',
      ' */',
      '// Nor a <label role="x"> in a line comment.',
      'const t = html`<mjx-button label="Bold"></mjx-button>`;',
    ].join('\n');
    expect(findAdHocSubstitutes(source)).toEqual([]);
  });

  it('reports the line a breach is actually on', () => {
    const source = ['/*', ' * two', ' * lines', ' */', '', 'html`<button></button>`'].join('\n');
    const findings = findAdHocSubstitutes(source);
    expect(findings).toHaveLength(1);
    expect(
      findings[0]?.line,
      'a block comment must be replaced by the newlines it spanned, or every line number after ' +
        'the first doc block points a reader at an innocent line.',
    ).toBe(6);
  });

  it('never flags the catalogue’s own element names', () => {
    const source = catalogueComponents.map((component) => `<${component.tag}></${component.tag}>`).join('\n');
    expect(findAdHocSubstitutes(source)).toEqual([]);
  });
});

describe('coverage is declared in both directions', () => {
  it('every deliberately-absent entry names a component the catalogue actually ships', () => {
    const unknown = Object.keys(absentFromShells).filter((tag) => !catalogueByTag.has(tag));
    expect(
      unknown,
      'a reason is attached to a component that does not exist, which is prose nothing checks.',
    ).toEqual([]);
  });

  it('every deliberately-absent entry carries a reason worth reading', () => {
    for (const [tag, reason] of Object.entries(absentFromShells)) {
      expect(reason.trim().length, `${tag}'s reason is too short to be one`).toBeGreaterThan(60);
    }
  });

  it('the required set is the catalogue minus the absent set, and is most of the catalogue', () => {
    expect(tagsRequiredInShells.length).toBe(
      catalogueComponents.length - Object.keys(absentFromShells).length,
    );
    expect(
      tagsRequiredInShells.length,
      'the required set has to be large, or the browser sweep asserts almost nothing.',
    ).toBeGreaterThanOrEqual(minimumDistinctComponents);
  });
});

describe('the nine', () => {
  it('are three applications at three sizes, with distinct ids', () => {
    expect(shellStories).toHaveLength(9);
    expect(new Set(shellStories.map((entry) => entry.id)).size).toBe(9);
    expect(shellApplicationNames).toHaveLength(3);
    expect(shellSizeNames).toHaveLength(3);
  });

  it('derive their ids the way Storybook does, from the titles the story files declare', () => {
    for (const application of shellApplicationNames) {
      const source = readFileSync(resolve(shellDirectory, `${application}.stories.ts`), 'utf8');
      expect(
        source,
        `${application}.stories.ts must declare title: '${shellStoryTitle[application]}' as a ` +
          'literal — CSF is indexed statically, and the browser gate looks every story up by the ' +
          'id derived from it.',
      ).toContain(`title: '${shellStoryTitle[application]}'`);
      for (const size of shellSizeNames) {
        const exportName = size.charAt(0).toUpperCase() + size.slice(1);
        expect(source).toContain(`export const ${exportName}: Story`);
        expect(shellStoryId(application, size)).toBe(
          `shell-${application}--${size}`,
        );
      }
    }
  });

  it('each declare their own size with a story-level global rather than trusting the toolbar', () => {
    for (const application of shellApplicationNames) {
      const source = readFileSync(resolve(shellDirectory, `${application}.stories.ts`), 'utf8');
      for (const preset of ['desktop', 'tablet', 'phone']) {
        expect(
          source,
          `${application}.stories.ts must pin its ${preset} story with ` +
            `globals: { containerPreset: '${preset}' }. Without it the story renders at whatever ` +
            'the toolbar was last left on, and the whole size axis is an accident.',
        ).toContain(`globals: { containerPreset: '${preset}' }`);
      }
    }
  });
});
