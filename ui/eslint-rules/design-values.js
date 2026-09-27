/**
 * `mjx/no-literal-design-values` — a hard-coded colour, radius, spacing or duration in a shipped
 * component is a defect.
 *
 * MJXOFF-181 states it as a requirement and as a gate in the same breath:
 *
 * > R01's generated tokens are the only source of a value. **A literal colour, radius or duration
 * > anywhere in a component is a defect**, and a lint rule should say so.
 *
 * > *"Icons render and typography is applied"* is satisfied by […] components carrying hard-coded
 * > values that merely happen to match the tokens today. **So the gates are a bundle-size
 * > assertion and a literal-value lint.**
 *
 * The second sentence is the whole reason this rule is worth writing rather than trusting review.
 * `#2e9e63` in a component is *invisible* while the token is also `#2e9e63`; it becomes visible on
 * the day the palette is re-seeded, in a diff nobody is reading, as one control that stayed the old
 * green. A lint rule is the only instrument that can see it beforehand.
 *
 * ## What it looks at
 *
 * String literals and template literals in `src/**` — which is where a framework-free custom
 * element's CSS lives, because a shadow root's stylesheet is a string. Comments are stripped first
 * (both JavaScript's and CSS's), because the *explanation* of a value is exactly where a value
 * should be written down: `src/harness/resizable-container.ts` explains its container-query defect
 * in terms of 700px and 666px, and a rule that flagged that would be a rule people disable.
 *
 * ## What it flags, and the three things it deliberately allows
 *
 * | Flagged | Because |
 * |---|---|
 * | `#rgb`, `#rrggbb`, `#rrggbbaa` | a colour belongs to `tokens.json` |
 * | `rgb()`, `rgba()`, `hsl()`, `oklch()`, `color()` | the same colour, spelled differently |
 * | `cubic-bezier(…)` | an easing belongs to `--ease-*` |
 * | `150ms`, `0.3s` | a duration belongs to `--duration-*` |
 * | any px length | a spacing, a radius or a size belongs to `--spacing`, `--radius-*` or `--text-*` |
 * | `border-radius:` with no `var(--radius` | the specific failure §4 warns about — reverting to Office's 2–4px corners |
 *
 * The allowances are `0` in any unit (zero is not a design decision), `1px` (there is no
 * border-width token and a hairline rule is not a design value — if a token for it ever appears,
 * delete this allowance), and every unit that is not `px`: `em`, `ch`, `%` and `fr` are layout
 * relationships rather than measurements, and `rem` never appears because `--spacing` is already
 * expressed in it.
 *
 * ## Proved able to fail
 *
 * `tests/design-values.test.ts` runs this rule over deliberately bad sources and asserts each
 * message id, and over the real `src/` tree and asserts silence. A lint rule nobody has watched
 * reject anything is a lint rule that might be matching nothing — the same doctrine
 * `story-conventions.js` is written under.
 */

/** Units whose values are layout relationships rather than design measurements. */
const allowedUnits = ['em', 'ch', 'ex', 'vw', 'vh', 'vmin', 'vmax', 'fr', '%', 'deg'];

/**
 * A hairline. There is no border-width token; when one appears, this allowance goes.
 *
 * Anything else in px is a measurement that a token already has a name for.
 */
const allowedPixelLengths = ['0px', '1px'];

/** Strip JavaScript and CSS comments so an explanation of a value is never mistaken for one. */
function withoutComments(text) {
  return text
    .replaceAll(/\/\*[\s\S]*?\*\//g, ' ')
    .replaceAll(/(^|\s)\/\/[^\n]*/g, '$1 ');
}

/**
 * @param {string} text
 * @returns {{ messageId: string, value: string }[]}
 */
function findings(text) {
  const source = withoutComments(text);
  /** @type {{ messageId: string, value: string }[]} */
  const found = [];

  for (const match of source.matchAll(/#[0-9a-fA-F]{3,8}\b/g)) {
    // A three-, four-, six- or eight-digit hex is a colour. Anything else is an id or a fragment.
    if ([4, 5, 7, 9].includes(match[0].length)) {
      found.push({ messageId: 'literalColor', value: match[0] });
    }
  }

  for (const match of source.matchAll(/\b(?:rgba?|hsla?|oklch|oklab|lab|lch|color)\(/g)) {
    found.push({ messageId: 'literalColor', value: match[0] });
  }

  for (const match of source.matchAll(/\bcubic-bezier\s*\(/g)) {
    found.push({ messageId: 'literalEasing', value: match[0] });
  }

  for (const match of source.matchAll(/(?<![\w.])\d+(?:\.\d+)?m?s\b/g)) {
    if (/^0(?:\.0+)?m?s$/.test(match[0])) continue;
    found.push({ messageId: 'literalDuration', value: match[0] });
  }

  for (const match of source.matchAll(/(?<![\w.$])\d+(?:\.\d+)?px\b/g)) {
    if (allowedPixelLengths.includes(match[0])) continue;
    found.push({ messageId: 'literalLength', value: match[0] });
  }

  for (const match of source.matchAll(/border-radius\s*:\s*([^;}\n]*)/g)) {
    const value = (match[1] ?? '').trim();
    // Empty means the declaration ran off the end of this template chunk, i.e. the value is a
    // `${…}` interpolation. Same reasoning as the explicit `${` case below: the value is not in
    // this file, and the module it came from is subject to the same rule.
    if (value === '') continue;
    if (value.includes('var(--radius')) continue;
    if (value.includes('inherit') || value === '0') continue;
    if (allowedUnits.some((unit) => value.endsWith(unit))) continue;
    // A `${…}` interpolation is a computed radius: the value is not in this file, so this rule
    // cannot judge it and the module it came from is subject to the same rule.
    if (value.includes('${')) continue;
    found.push({ messageId: 'literalRadius', value });
  }

  return found;
}

/** @type {import('eslint').Rule.RuleModule} */
const rule = {
  meta: {
    type: 'problem',
    docs: {
      description:
        'A shipped component reads every colour, radius, spacing and duration from a generated ' +
        'token. A literal is invisible until the palette changes, and then it is one control that ' +
        'stayed the old colour.',
    },
    schema: [],
    messages: {
      literalColor:
        "'{{value}}' is a literal colour. Colours come from the generated tokens — " +
        'var(--color-*), var(--theme-*) or var(--document-*). MJXOFF-181: a literal colour in a ' +
        'component is a defect.',
      literalEasing:
        "'{{value}}' is a literal easing curve. Use var(--ease-ink), var(--ease-out-soft) or " +
        'var(--ease-spring) — and see src/foundations/motion.ts for which one this is.',
      literalDuration:
        "'{{value}}' is a literal duration. Use var(--duration-transition).",
      literalLength:
        "'{{value}}' is a literal length. Spacing is calc(var(--spacing) * n), a corner is " +
        'var(--radius-*), a type size is var(--text-*). Only 0px and a 1px hairline are allowed, ' +
        'because no token names a border width.',
      literalRadius:
        "border-radius: '{{value}}' does not read a radius token. DESIGN_TOKENS.md §4: the large " +
        'radii are the most recognisable part of this design language, and reverting to Office’s ' +
        '2–4px corners discards it.',
    },
  },

  create(context) {
    /** @param {any} node @param {string} text */
    function check(node, text) {
      for (const finding of findings(text)) {
        context.report({ node, messageId: finding.messageId, data: { value: finding.value } });
      }
    }

    return {
      Literal(node) {
        if (typeof node.value === 'string') check(node, node.value);
      },
      TemplateElement(node) {
        check(node, node.value.raw);
      },
    };
  },
};

export default { rules: { 'no-literal-design-values': rule } };
